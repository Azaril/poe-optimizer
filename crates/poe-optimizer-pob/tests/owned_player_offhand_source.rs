//! Original off-hand branch, loaded item type and selected-use correspondence.
//! Structural saved facts are distinct from prepared equipment and GetCondition.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;
use mlua::{Function, Lua, LuaSerdeExt, Table, Value};
use poe_optimizer_core::build_identity::BuildLineage;
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};
use poe_optimizer_pob::{runtime::RuntimeError, source as pinned};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    fs,
    ops::Range,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    rc::Rc,
    time::{Duration, Instant},
};
const OBSERVE: &str = include_str!("support/player_offhand_source.lua");
const TEST: &str = "actual_player_offhand_branch_preserves_selected_and_prepared_boundaries";
const CHILD: &str = "POE_PLAYER_OFFHAND_SOURCE_CHILD";
const OUTPUT: &str = "POE_OPTIMIZER_TEST_PLAYER_OFFHAND_SOURCE_OUT";
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn rows(v: &Json) -> &[Json] {
    if let Some(a) = v.as_array() {
        a
    } else {
        assert!(v.as_object().is_some_and(|o| o.is_empty()));
        &[]
    }
}
fn observed(root: &Path, xml: &str, jit: bool, hooked: bool) -> Json {
    let module = Rc::new(RefCell::new(None::<Table>));
    let before_source = |lua: &Lua| {
        lua.load(if jit {
            "jit.on()"
        } else {
            "jit.off();jit.flush()"
        })
        .exec()?;
        Ok(())
    };
    let before_build = |lua: &Lua| -> Result<Function, RuntimeError> {
        let observer: Table = lua
            .load(OBSERVE)
            .set_name("@player_offhand_source.lua")
            .eval()?;
        let cleanup = observer.raw_get::<Function>("begin")?.call((hooked, jit))?;
        *module.borrow_mut() = Some(observer);
        Ok(cleanup)
    };
    let after = |lua: &Lua| -> Result<Json, RuntimeError> {
        let result: Table = module
            .borrow()
            .as_ref()
            .unwrap()
            .raw_get::<Function>("observe")?
            .call(jit)?;
        Ok(lua.from_value(Value::Table(result))?)
    };
    let scratch = tempfile::tempdir().unwrap();
    let report = source::observe_with_build_hook_unwrapped(
        &root.join("vendor/path-of-building-poe2"),
        scratch.path(),
        xml,
        None,
        false,
        Some(&before_source),
        Some(&before_build),
        Some(&after),
    )
    .unwrap();
    assert_eq!(report["configuration_method_wrappers"], false);
    assert_eq!(report["original_build_output_available"], true);
    json!({"source_hash":report["source_hash"],"selected":report["selected"],"state":report["additional_observation"]})
}
fn without_capture(mut host: Json) -> Json {
    let s = host["state"].as_object_mut().unwrap();
    s.remove("hooked");
    s.remove("invocations");
    for mode in ["MAIN", "CALCS"] {
        s.get_mut("modes").unwrap()[mode]
            .as_object_mut()
            .unwrap()
            .remove("provenance");
    }
    host
}
fn structural(item: &Json) -> Json {
    let occupied = item["present"].as_bool().unwrap();
    json!({"empty":!occupied,"shield":occupied && item["type"]=="Shield","focus":occupied && item["type"]=="Focus"})
}
fn branch(before: &Json, after: &Json, item: &Json) {
    let selected = if item["present"] == false {
        Some("OffHandIsEmpty")
    } else {
        match item["type"].as_str().unwrap() {
            "Shield" => Some("UsingShield"),
            "Focus" => Some("UsingFocus"),
            _ => None,
        }
    };
    for name in ["UsingShield", "UsingFocus", "OffHandIsEmpty"] {
        if Some(name) == selected {
            assert_eq!(
                after[name],
                json!({"present":true,"kind":"boolean","value":true})
            );
        } else {
            assert_eq!(
                before[name], after[name],
                "branch does not write false into other conditions"
            );
        }
    }
}
fn source_frame(xml: &str) -> Json {
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([97; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let e = SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    let items = e
        .rows()
        .iter()
        .find(|r| r.occurrence().name() == "Items")
        .unwrap();
    let set_id = items.attribute("activeItemSet").unwrap().decoded().unwrap();
    let set = e
        .rows()
        .iter()
        .find(|r| {
            r.occurrence().name() == "ItemSet"
                && r.occurrence().parent() == Some(items.occurrence().id())
                && r.attribute("id")
                    .is_some_and(|a| a.decoded().unwrap() == set_id)
        })
        .unwrap();
    let swap = set
        .attribute("useSecondWeaponSet")
        .unwrap()
        .decoded()
        .unwrap();
    assert!(matches!(swap, "true" | "false" | "nil"));
    let slot = |hand: &str| {
        let name = format!("{hand}{}", if swap == "true" { " Swap" } else { "" });
        let row = e
            .rows()
            .iter()
            .find(|r| {
                r.occurrence().name() == "Slot"
                    && r.occurrence().parent() == Some(set.occurrence().id())
                    && r.attribute("name")
                        .is_some_and(|a| a.decoded().unwrap() == name)
            })
            .unwrap();
        let id = row.attribute("itemId").unwrap().decoded().unwrap();
        let item = e.rows().iter().find(|r| {
            r.occurrence().name() == "Item"
                && r.occurrence().parent() == Some(items.occurrence().id())
                && r.attribute("id")
                    .is_some_and(|a| a.decoded().unwrap() == id)
        });
        assert_eq!(item.is_none(), id == "0");
        json!({"slot_ordinal":row.occurrence().id().ordinal(),"slot_name":name,"item_id":id.parse::<u64>().unwrap(),"item_ordinal":item.map(|r|r.occurrence().id().ordinal())})
    };
    json!({"xml_sha256":hash(xml.as_bytes()),"selected_item_set":set_id.parse::<u64>().unwrap(),"item_set_ordinal":set.occurrence().id().ordinal(),"raw_swap":swap,"weapon_loadout":if swap=="true" {2} else {1},"weapon_one":slot("Weapon 1"),"weapon_two":slot("Weapon 2")})
}
fn check(host: &Json, source: &Json, hooked: bool, filtering: bool) {
    let s = &host["state"];
    assert_eq!(s["hooked"], hooked);
    assert_eq!(s["selection"]["items"], source["selected_item_set"]);
    assert_eq!(
        s["selection"]["use_second_weapon_set"],
        source["weapon_loadout"] == 2
    );
    assert_eq!(s["methods"]["actor"]["first"], 264);
    assert_eq!(s["methods"]["parse_raw"]["first"], 468);
    let catalogue = rows(&s["catalogue"]);
    assert!(!catalogue.is_empty());
    assert!(
        catalogue
            .windows(2)
            .all(|x| x[0]["name"].as_str().unwrap() < x[1]["name"].as_str().unwrap())
    );
    for mode in ["MAIN", "CALCS"] {
        let m = &s["modes"][mode];
        for (field, src) in [
            ("saved_main_hand", "weapon_one"),
            ("saved_slot", "weapon_two"),
        ] {
            assert_eq!(m[field]["name"], source[src]["slot_name"]);
            assert_eq!(m[field]["selected_item_id"], source[src]["item_id"]);
            let item = &m[field]["item"];
            if item["present"] == true {
                assert_eq!(item["source_item_id"]["value"], source[src]["item_id"]);
                assert_eq!(item["source_item_id"]["present"], true);
                assert_eq!(item["exact_catalogue_base"], true);
                assert_eq!(item["type"], item["base_type"]);
                let matches: Vec<_> = catalogue
                    .iter()
                    .filter(|r| r["name"] == item["base_name"] && r["type"] == item["type"])
                    .collect();
                assert_eq!(matches.len(), 1);
            } else {
                assert_eq!(source[src]["item_id"], 0);
            }
        }
        assert_eq!(m["selected_to_prepared_same_object"], !filtering);
        if filtering {
            assert_eq!(m["saved_occupied"], true);
            assert_eq!(m["prepared_occupied"], false);
            assert_eq!(m["saved_slot"]["item"]["type"], "Focus");
            assert!(
                rows(&m["saved_main_hand"]["item"]["disables_item"])
                    .iter()
                    .any(|r| rows(&r["tags"])
                        .iter()
                        .any(|t| t["type"] == "DisablesItem" && t["slotName"] == "Weapon 2")),
                "actual parsed main-hand disable tag"
            );
        } else {
            assert_eq!(m["saved_slot"]["item"], m["prepared_item"]);
        }
        if hooked {
            let p = &m["provenance"];
            for flag in ["exact_actor", "exact_store", "exact_prepared_item"] {
                assert_eq!(p[flag], true);
            }
            branch(&p["before"], &p["after"], &m["prepared_item"]);
            let call = &rows(&s["invocations"])[p["invocation"].as_u64().unwrap() as usize - 1];
            assert_eq!(call["before"], p["before"]);
            assert_eq!(call["after"], p["after"]);
            assert_eq!(call["item"], m["prepared_item"]);
            assert_eq!(
                call[if mode == "MAIN" {
                    "exact_final_main"
                } else {
                    "exact_final_calcs"
                }],
                true
            );
            if m["saved_occupied"] == true {
                assert!(!rows(&p["selected_item_parses"]).is_empty());
            }
        } else {
            assert!(m.get("provenance").is_none());
        }
    }
    assert_eq!(!rows(&s["invocations"]).is_empty(), hooked);
    for call in rows(&s["invocations"]) {
        branch(&call["before"], &call["after"], &call["item"]);
    }
    for field in [
        "original_branch",
        "original_item_type_assignment",
        "scalar_outputs_preserved",
        "original_methods_preserved",
        "saved_items_preserved",
        "saved_selection_preserved",
        "condition_read_set_preserved",
    ] {
        assert_eq!(s["evidence"][field], true);
    }
    for field in [
        "business_method_wrappers",
        "full_output_graph",
        "effective_condition_closure",
        "item_filtering_or_substitution_native_admission",
        "unarmed_or_unencumbered",
    ] {
        assert_eq!(s["evidence"][field], false);
    }
}
fn replace(xml: &str, range: Range<usize>, value: &str) -> String {
    let mut changed = xml.to_owned();
    changed.replace_range(range.clone(), value);
    let mut inverse = changed.clone();
    inverse.replace_range(range.start..range.start + value.len(), &xml[range]);
    assert_eq!(inverse, xml);
    changed
}
fn selected_set<'a, 'input>(doc: &'a roxmltree::Document<'input>) -> roxmltree::Node<'a, 'input> {
    let items = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Items"))
        .unwrap();
    items
        .children()
        .find(|n| {
            n.has_tag_name("ItemSet") && n.attribute("id") == items.attribute("activeItemSet")
        })
        .unwrap()
}
fn slot_item(xml: &str, name: &str, id: u64) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let set = selected_set(&doc);
    let node = set
        .children()
        .find(|n| n.has_tag_name("Slot") && n.attribute("name") == Some(name))
        .unwrap();
    let a = node.attributes().find(|a| a.name() == "itemId").unwrap();
    replace(xml, a.range(), &format!("itemId=\"{id}\""))
}
fn swap(xml: &str, enabled: bool) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let set = selected_set(&doc);
    let a = set
        .attributes()
        .find(|a| a.name() == "useSecondWeaponSet")
        .unwrap();
    replace(xml, a.range(), &format!("useSecondWeaponSet=\"{enabled}\""))
}
fn item_text(xml: &str, id: &str, value: Option<&str>) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let items = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Items"))
        .unwrap();
    let item = items
        .children()
        .find(|n| n.has_tag_name("Item") && n.attribute("id") == Some(id))
        .unwrap();
    // Original ItemsTab:Load parses the non-whitespace text child and separately
    // applies nested ModRange metadata. Edit only that text, retaining every
    // metadata child and all surrounding source bytes.
    let nodes: Vec<_> = item
        .children()
        .filter(|n| n.is_text() && !n.text().unwrap().trim().is_empty())
        .collect();
    assert_eq!(nodes.len(), 1);
    assert!(nodes[0].text().unwrap().trim_start().starts_with("Rarity:"));
    let raw = value
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{}\nUses both hand slots\n", &xml[nodes[0].range()]));
    replace(xml, nodes[0].range(), &raw)
}
struct Case {
    name: String,
    original: usize,
    xml: String,
    synthetic: bool,
    filtering: bool,
}
fn child(root: &Path, out: &Path, jit: bool) {
    // Authenticate every pin before any expensive original build loads.
    let manifest: Json =
        serde_json::from_slice(include_bytes!("../data/pob-source-manifest.json")).unwrap();
    let paths = [
        "src/Modules/CalcPerform.lua",
        "src/Modules/CalcSetup.lua",
        "src/Classes/Item.lua",
        "src/Classes/ItemsTab.lua",
        "src/Classes/ModStore.lua",
        "src/Modules/ModParser.lua",
    ];
    let mut files: Vec<_> = paths
        .iter()
        .map(|path| json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()}))
        .collect();
    for row in rows(&manifest["files"]) {
        if row["path"].as_str().unwrap().starts_with("src/Data/Bases/") {
            files.push(row.clone());
        }
    }
    let fixtures = root.join("tests/fixtures/builds/breadth-20260908");
    let index: Json =
        serde_json::from_slice(&fs::read(fixtures.join("index.json")).unwrap()).unwrap();
    let originals: Vec<_> = (1..=5)
        .map(|i| fs::read_to_string(fixtures.join(format!("build-{i:02}.xml"))).unwrap())
        .collect();
    for (i, xml) in originals.iter().enumerate() {
        let name = format!("build-{:02}.xml", i + 1);
        let pin = rows(&index["builds"])
            .iter()
            .find(|r| r["xml"] == name)
            .unwrap();
        assert_eq!(pin["xml_sha256"], hash(xml.as_bytes()));
    }
    let mut cases: Vec<_> = originals
        .iter()
        .enumerate()
        .map(|(i, xml)| Case {
            name: format!("original-{:02}", i + 1),
            original: i + 1,
            xml: xml.clone(),
            synthetic: false,
            filtering: false,
        })
        .collect();
    let swapped = swap(
        &slot_item(&slot_item(&originals[0], "Weapon 2", 0), "Weapon 2 Swap", 1),
        true,
    );
    cases.extend([
        Case {
            name: "original-01-empty".into(),
            original: 1,
            xml: slot_item(&originals[0], "Weapon 2", 0),
            synthetic: false,
            filtering: false,
        },
        Case {
            name: "original-01-shield".into(),
            original: 1,
            xml: item_text(
                &originals[0],
                "1",
                Some("\nRarity: NORMAL\nSplintered Tower Shield\n"),
            ),
            synthetic: true,
            filtering: false,
        },
        Case {
            name: "original-02-first-loadout".into(),
            original: 2,
            xml: swap(&originals[1], false),
            synthetic: false,
            filtering: false,
        },
        Case {
            name: "original-01-focus-swapped".into(),
            original: 1,
            xml: swapped,
            synthetic: false,
            filtering: false,
        },
        Case {
            name: "original-01-disables-offhand".into(),
            original: 1,
            xml: item_text(&originals[0], "9", None),
            synthetic: true,
            filtering: true,
        },
    ]);
    let mode = if jit { "on" } else { "off" };
    let mut captures = vec![];
    let mut native = vec![];
    for (i, c) in cases.iter().enumerate() {
        eprintln!(
            "Player off-hand {}/{} {} JIT {mode}",
            i + 1,
            cases.len(),
            c.name
        );
        let frame = source_frame(&c.xml);
        let original = observed(root, &c.xml, jit, true);
        let repeat = observed(root, &c.xml, jit, true);
        let unhooked = observed(root, &c.xml, jit, false);
        let capture = json!({"name":c.name,"original_number":c.original,"source":frame,"synthetic_item_edit":c.synthetic,"filtering_control":c.filtering,"original":original,"repeat":repeat,"unhooked":unhooked});
        fs::write(
            out.join(format!("source-jit-{mode}-case-{:02}.raw.json", i + 1)),
            serde_json::to_vec_pretty(&capture).unwrap(),
        )
        .unwrap();
        for (host, hooked) in [(&original, true), (&repeat, true), (&unhooked, false)] {
            check(host, &frame, hooked, c.filtering);
        }
        assert!(
            original == repeat,
            "independent fresh replay differs; retained raw case {i}"
        );
        assert!(
            without_capture(original.clone()) == without_capture(unhooked.clone()),
            "hook changes source state or scalar outputs; retained raw case {i}"
        );
        let s = &original["state"];
        let main = &s["modes"]["MAIN"];
        let expected = [
            Some("Focus"),
            Some("Sceptre"),
            None,
            None,
            None,
            None,
            Some("Shield"),
            None,
            Some("Focus"),
            Some("Focus"),
        ][i];
        for m in ["MAIN", "CALCS"] {
            let item = &s["modes"][m]["saved_slot"]["item"];
            assert_eq!(
                item["present"],
                expected.is_some(),
                "bounded control must establish its intended saved occupancy"
            );
            if let Some(kind) = expected {
                assert_eq!(
                    item["type"], kind,
                    "bounded control must reach the real loaded item type"
                );
            }
        }
        let prepared:[Json;2]=["MAIN","CALCS"].map(|m|{let a=&s["modes"][m];json!({"mode":m,"item":a["prepared_item"],"same_selected_item":a["selected_to_prepared_same_object"],"before":a["provenance"]["before"],"after":a["provenance"]["after"],"facts":structural(&a["prepared_item"])})});
        native.push(json!({"case_index":i,"name":c.name,"original":c.original,"source":frame,"selected":{"weapon_one":main["saved_main_hand"]["item"],"weapon_two":main["saved_slot"]["item"],"facts":structural(&main["saved_slot"]["item"])},"prepared":prepared,"synthetic_item_edit":c.synthetic,"filtering_control":c.filtering}));
        captures.push(capture);
    }
    for (i, xml) in originals.iter().enumerate() {
        assert_eq!(
            *xml,
            fs::read_to_string(fixtures.join(format!("build-{:02}.xml", i + 1))).unwrap()
        );
    }
    for c in &captures {
        assert_eq!(
            c["original"]["state"]["catalogue"],
            captures[0]["original"]["state"]["catalogue"]
        );
    }
    let report = json!({"schema_version":1,"source_revision":pinned::UPSTREAM_REVISION,"manifest_sha256":pinned::manifest_sha256(),"observer_sha256":hash(OBSERVE.as_bytes()),"bootstrap_sha256":hash(include_bytes!("support/configuration_preparation_source.rs")),"files":files,
        "case_count":cases.len(),"complete_loads_per_jit":cases.len()*3,"catalogue":captures[0]["original"]["state"]["catalogue"],"native_cases":native,"cases":captures,
        "scope":{"original_offhand_branch":true,"original_item_type_assignment":true,"fresh_repeat":true,"fresh_unhooked_comparison":true,"selected_structural_projection":true,"scalar_output_comparison":true,"full_output_graph":false,"no_retry_or_settling":true,"effective_condition_closure":false,"item_filtering_or_substitution_native_admission":false,"synthetic_items_obtainable":false,"full_native_build_parity":false}});
    fs::write(
        out.join(format!("source-jit-{mode}.json")),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "requires complete pinned PoB source; bounded original off-hand observer"]
fn actual_player_offhand_branch_preserves_selected_and_prepared_boundaries() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = std::env::var_os(OUTPUT)
        .map(PathBuf::from)
        .map(|p| if p.is_absolute() { p } else { root.join(p) })
        .unwrap_or_else(|| root.join("runs/owned-player-offhand-source-01"));
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        child(&root, &out, mode == "on");
        return;
    }
    assert!(!out.exists(), "fresh immutable source output required");
    fs::create_dir_all(&out).unwrap();
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut process = Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", TEST, "--nocapture"])
            .env(CHILD, mode)
            .env(OUTPUT, &out)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = process.try_wait().unwrap() {
                assert!(status.success(), "source child failed: {}", path.display());
                break;
            }
            if start.elapsed() > Duration::from_secs(600) {
                process.kill().unwrap();
                process.wait().unwrap();
                panic!("source deadline: {}", path.display());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    assert!(
        fs::read(out.join("source-jit-off.json")).unwrap()
            == fs::read(out.join("source-jit-on.json")).unwrap(),
        "both JIT modes must produce byte-identical evidence"
    );
}
