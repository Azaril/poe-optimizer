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
const HANDS_OBSERVE: &str = include_str!("support/player_prepared_hands_source.lua");
const HANDS_TEST: &str =
    "actual_player_prepared_hands_distinguish_item_profile_and_condition_writes";
const HANDS_CHILD: &str = "POE_PLAYER_PREPARED_HANDS_SOURCE_CHILD";
const HANDS_OUTPUT: &str = "POE_OPTIMIZER_TEST_PLAYER_PREPARED_HANDS_SOURCE_OUT";
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
    observed_with_hands(root, xml, jit, hooked, false)
}
fn observed_with_hands(root: &Path, xml: &str, jit: bool, hooked: bool, hands: bool) -> Json {
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
        let mut observer: Table = lua
            .load(OBSERVE)
            .set_name("@player_offhand_source.lua")
            .eval()?;
        if hands {
            observer = lua
                .load(HANDS_OBSERVE)
                .set_name("@player_prepared_hands_source.lua")
                .eval::<Function>()?
                .call(observer)?;
        }
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
    run_source_test(
        TEST,
        CHILD,
        OUTPUT,
        "runs/owned-player-offhand-source-01",
        child,
    );
}
fn run_source_test(
    test: &str,
    child_var: &str,
    output_var: &str,
    default_out: &str,
    run_child: fn(&Path, &Path, bool),
) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = std::env::var_os(output_var)
        .map(PathBuf::from)
        .map(|p| if p.is_absolute() { p } else { root.join(p) })
        .unwrap_or_else(|| root.join(default_out));
    if let Some(mode) = std::env::var_os(child_var) {
        assert!(mode == "off" || mode == "on");
        run_child(&root, &out, mode == "on");
        return;
    }
    assert!(!out.exists(), "fresh immutable source output required");
    fs::create_dir_all(&out).unwrap();
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut process = Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", test, "--nocapture"])
            .env(child_var, mode)
            .env(output_var, &out)
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

struct HandCase {
    name: &'static str,
    xml: String,
    main_present: bool,
    gloves_present: bool,
    profile_type: &'static str,
    unencumbered: bool,
    filtered_main: bool,
    facebreaker_lines: bool,
}
fn hand_cases(original: &str) -> Vec<HandCase> {
    let empty = slot_item(original, "Weapon 1", 0);
    let club = item_text(original, "28", Some("\nRarity: NORMAL\nWooden Club\n"));
    let facebreaker = item_text(
        &empty,
        "20",
        Some(
            "\nRarity: NORMAL\nStocky Mitts\nImplicits: 0\nCan Attack as though using a One Handed Mace while both of your hand slots are empty\nUnarmed Attacks that would use an Equipped One Hand Mace's damage use this Item's damage\n",
        ),
    );
    let disabled = slot_item(
        &item_text(
            &club,
            "24",
            Some("\nRarity: NORMAL\nWooden Club\nImplicits: 0\nUses both hand slots\n"),
        ),
        "Weapon 2",
        24,
    );
    [
        (
            "original-05",
            original.to_owned(),
            true,
            true,
            "None",
            false,
            false,
            false,
        ),
        (
            "caster-without-gloves",
            slot_item(original, "Gloves", 0),
            true,
            false,
            "None",
            true,
            false,
            false,
        ),
        (
            "empty-with-gloves",
            empty.clone(),
            false,
            true,
            "None",
            false,
            false,
            false,
        ),
        (
            "empty-without-gloves",
            slot_item(&empty, "Gloves", 0),
            false,
            false,
            "None",
            true,
            false,
            false,
        ),
        (
            "ordinary-weapon",
            club.clone(),
            true,
            true,
            "One Hand Mace",
            false,
            false,
            false,
        ),
        (
            "ordinary-weapon-without-gloves",
            slot_item(&club, "Gloves", 0),
            true,
            false,
            "One Hand Mace",
            false,
            false,
            false,
        ),
        (
            "empty-facebreaker-lines",
            facebreaker,
            false,
            true,
            "None",
            false,
            false,
            true,
        ),
        (
            "offhand-filters-main",
            disabled,
            false,
            true,
            "None",
            false,
            true,
            false,
        ),
    ]
    .into_iter()
    .map(
        |(
            name,
            xml,
            main_present,
            gloves_present,
            profile_type,
            unencumbered,
            filtered_main,
            facebreaker_lines,
        )| HandCase {
            name,
            xml,
            main_present,
            gloves_present,
            profile_type,
            unencumbered,
            filtered_main,
            facebreaker_lines,
        },
    )
    .collect()
}
fn hand_source_frame(xml: &str) -> Json {
    let mut frame = source_frame(xml);
    let doc = roxmltree::Document::parse(xml).unwrap();
    let set = selected_set(&doc);
    let gloves = set
        .children()
        .find(|n| n.has_tag_name("Slot") && n.attribute("name") == Some("Gloves"))
        .unwrap();
    let id = gloves.attribute("itemId").unwrap().parse::<u64>().unwrap();
    frame["gloves"] = json!({"slot_name":"Gloves","item_id":id});
    frame
}
fn without_hand_capture(mut host: Json) -> Json {
    let hands = host["state"]["prepared_hands"].as_object_mut().unwrap();
    hands.remove("hooked");
    hands.remove("invocations");
    for mode in ["MAIN", "CALCS"] {
        hands.get_mut("modes").unwrap()[mode]
            .as_object_mut()
            .unwrap()
            .remove("provenance");
    }
    host
}
fn hand_invocation<'a>(state: &'a Json, mode: &str, kind: &str) -> &'a Json {
    let receipt = &state["modes"][mode]["provenance"][kind];
    assert_eq!(receipt["exact_actor"], true);
    assert_eq!(receipt["exact_store"], true);
    let i = receipt["invocation"].as_u64().unwrap() as usize;
    let invocation = &rows(&state["invocations"])[i - 1];
    assert_eq!(invocation["kind"], kind);
    assert_eq!(invocation["mode"], mode);
    assert!(invocation["source_invocation"].as_u64().unwrap() > 0);
    let entries = rows(&invocation["entry_events"]);
    assert!(!entries.is_empty());
    assert_eq!(entries[0]["state"], invocation["before"]);
    for entry in entries {
        assert_eq!(entry["line"], invocation["first"]);
    }
    assert_eq!(
        invocation[if mode == "MAIN" {
            "exact_final_main"
        } else {
            "exact_final_calcs"
        }],
        true
    );
    invocation
}
fn check_hand_capture(host: &Json, c: &HandCase, hooked: bool) {
    let s = &host["state"];
    let hands = &s["prepared_hands"];
    assert_eq!(hands["hooked"], hooked);
    assert_eq!(
        s["hooked"], false,
        "the historical observer remains unmodified and unhooked"
    );
    assert_eq!(hands["methods"]["initializer"]["first"], 717);
    assert_eq!(hands["methods"]["actor"]["first"], 264);
    assert_eq!(hands["methods"]["perform"]["first"], 1193);
    let source = hand_source_frame(&c.xml);
    for mode in ["MAIN", "CALCS"] {
        let final_state = &hands["modes"][mode]["final"];
        let ordinary = &s["modes"][mode];
        assert_eq!(
            ordinary["saved_main_hand"]["selected_item_id"],
            source["weapon_one"]["item_id"]
        );
        assert_eq!(
            ordinary["saved_slot"]["selected_item_id"],
            source["weapon_two"]["item_id"]
        );
        assert_eq!(final_state["prepared_main"]["present"], c.main_present);
        assert_eq!(final_state["prepared_gloves"]["present"], c.gloves_present);
        if c.main_present {
            assert_eq!(
                final_state["prepared_main"]["source_item_id"]["value"],
                source["weapon_one"]["item_id"]
            );
        }
        if c.gloves_present {
            assert_eq!(
                final_state["prepared_gloves"]["source_item_id"]["value"],
                source["gloves"]["item_id"]
            );
        }
        assert_eq!(final_state["primary"]["value"]["type"], c.profile_type);
        assert_eq!(
            final_state["primary_is_catalogue_object"], false,
            "copied intrinsic profiles are not catalogue aliases"
        );
        assert_eq!(
            final_state["primary_is_item_profile"],
            c.profile_type == "One Hand Mace"
        );
        assert_eq!(
            final_state["conditions"]["Unarmed"]["present"],
            c.profile_type == "None"
        );
        assert_eq!(
            final_state["conditions"]["Unencumbered"]["present"],
            c.unencumbered
        );
        assert_eq!(
            final_state["legacy_player_gloves"]["present"], false,
            "capture the different late-branch field; do not substitute itemList.Gloves"
        );
        for channel in rows(&final_state["ancestry"]) {
            assert!(rows(&channel["modifiers"]["DisableWeapons"]).is_empty());
        }
        if !hooked {
            assert!(hands["modes"][mode]["provenance"].is_null());
            continue;
        }
        let init = hand_invocation(hands, mode, "initialization");
        assert_eq!(
            (init["first"].as_u64(), init["last"].as_u64()),
            (Some(1853), Some(1889))
        );
        assert_eq!(init["after"]["primary"]["value"]["type"], c.profile_type);
        assert_eq!(init["after"]["prepared_main"]["present"], c.main_present);
        assert_eq!(
            init["after"]["primary_equals_catalogue"],
            c.profile_type == "None" && !c.facebreaker_lines
        );
        let init_events = rows(&init["events"]);
        assert_eq!(init_events.iter().filter(|e| e["line"] == 1854).count(), 1);
        for line in [1873, 1877] {
            assert_eq!(
                init_events.iter().filter(|e| e["line"] == line).count(),
                usize::from(c.facebreaker_lines)
            );
        }
        if c.facebreaker_lines {
            assert_eq!(
                init["after"]["primary"]["value"]["asThoughUsing"]["One Hand Mace"],
                true
            );
            assert_eq!(
                init["after"]["primary"]["value"]["FacebreakerItemDamage"],
                true
            );
        }
        let condition = hand_invocation(hands, mode, "conditions");
        assert_eq!(
            (condition["first"].as_u64(), condition["last"].as_u64()),
            (Some(280), Some(319))
        );
        let events = rows(&condition["events"]);
        assert_eq!(
            events.iter().filter(|e| e["line"] == 281).count(),
            usize::from(c.profile_type == "None")
        );
        assert_eq!(
            events.iter().filter(|e| e["line"] == 283).count(),
            usize::from(c.unencumbered)
        );
        for (name, wrote) in [
            ("Unarmed", c.profile_type == "None"),
            ("Unencumbered", c.unencumbered),
        ] {
            if wrote {
                assert_eq!(
                    condition["after"]["conditions"][name],
                    json!({"present":true,"kind":"boolean","value":true})
                );
            } else {
                assert_eq!(
                    condition["after"]["conditions"][name],
                    condition["before"]["conditions"][name]
                );
            }
        }
        let late = hand_invocation(hands, mode, "late_disable");
        assert_eq!(
            (late["first"].as_u64(), late["last"].as_u64()),
            (Some(3229), Some(3240))
        );
        assert!(
            rows(&late["events"]).is_empty(),
            "no actual DisableWeapons producer in these controls"
        );
        for field in [
            "primary",
            "conditions",
            "prepared_main",
            "prepared_gloves",
            "legacy_player_gloves",
        ] {
            assert_eq!(late["before"][field], late["after"][field]);
        }
        if c.filtered_main {
            assert_eq!(ordinary["saved_main_hand"]["item"]["present"], true);
            assert_eq!(final_state["prepared_main"]["present"], false);
            assert_eq!(ordinary["prepared_item"]["source_item_id"]["value"], 24);
            assert!(
                rows(&ordinary["prepared_item"]["disables_item"])
                    .iter()
                    .any(|r| rows(&r["tags"])
                        .iter()
                        .any(|t| t["type"] == "DisablesItem" && t["slotName"] == "Weapon 1"))
            );
        }
    }
    assert_eq!(hands["evidence"]["late_disable_activated"], false);
    assert_eq!(hands["evidence"]["full_native_build_parity"], false);
    assert_eq!(hands["evidence"]["effective_condition_closure"], false);
    assert_eq!(hands["evidence"]["business_method_wrappers"], false);
}

#[test]
fn prepared_hand_controls_change_only_selected_item_inputs() {
    let original = include_str!("../../../tests/fixtures/builds/breadth-20260908/build-05.xml");
    let cases = hand_cases(original);
    assert_eq!(cases.len(), 8);
    let hashes: std::collections::BTreeSet<_> =
        cases.iter().map(|c| hash(c.xml.as_bytes())).collect();
    assert_eq!(hashes.len(), cases.len());
    let doc = roxmltree::Document::parse(original).unwrap();
    for c in &cases {
        let changed = roxmltree::Document::parse(&c.xml).unwrap();
        for name in ["Build", "Tree", "Skills", "Config"] {
            let before = doc
                .root_element()
                .children()
                .find(|n| n.has_tag_name(name))
                .unwrap();
            let after = changed
                .root_element()
                .children()
                .find(|n| n.has_tag_name(name))
                .unwrap();
            assert_eq!(
                &original[before.range()],
                &c.xml[after.range()],
                "{} {name}",
                c.name
            );
        }
        assert_eq!(hand_source_frame(&c.xml)["selected_item_set"], 2);
    }
    assert_eq!(
        hash(original.as_bytes()),
        "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089"
    );
}

fn hand_child(root: &Path, out: &Path, jit: bool) {
    let xml = fs::read_to_string(root.join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
        .unwrap();
    assert_eq!(
        hash(xml.as_bytes()),
        "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089"
    );
    let paths = [
        "src/Modules/CalcSetup.lua",
        "src/Modules/CalcPerform.lua",
        "src/Modules/Data.lua",
        "src/Classes/Item.lua",
        "src/Classes/ItemsTab.lua",
        "src/Classes/ModStore.lua",
        "src/Modules/ModParser.lua",
        "src/Data/Bases/mace.lua",
        "src/Data/Bases/staff.lua",
        "src/Data/Bases/gloves.lua",
        "src/Data/Uniques/gloves.lua",
    ];
    let files: Vec<_> = paths
        .iter()
        .map(|path| {
            let expected = pinned::expected_file_sha256(path).unwrap();
            let verified =
                pinned::read_verified_text(&root.join("vendor/path-of-building-poe2"), path)
                    .unwrap();
            assert_eq!(
                hash(verified.as_bytes()),
                expected,
                "normalized source pin: {path}"
            );
            json!({"path":path,"sha256":expected})
        })
        .collect();
    let cases = hand_cases(&xml);
    let mode = if jit { "on" } else { "off" };
    // A current independent unhooked load must also retain every historical
    // off-hand field for Original05. Do not repin that report to this extension.
    let prior_path = format!("runs/owned-player-offhand-source-02/source-jit-{mode}.json");
    let prior_bytes = fs::read(root.join(&prior_path)).unwrap();
    let prior_sha256 = hash(&prior_bytes);
    assert_eq!(
        prior_sha256,
        "d4bbaebc058287d511cc9bdd1497feb8a9f8ba7eea8d65c40d26a01f54bf27b7"
    );
    let prior: Json = serde_json::from_slice(&prior_bytes).unwrap();
    assert_eq!(prior["observer_sha256"], hash(OBSERVE.as_bytes()));
    let prior_original = rows(&prior["cases"])
        .iter()
        .find(|c| c["name"] == "original-05")
        .unwrap();
    let mut captures = Vec::new();
    for (i, c) in cases.iter().enumerate() {
        eprintln!(
            "Player prepared hands {}/{} {} JIT {mode}",
            i + 1,
            cases.len(),
            c.name
        );
        let original = observed_with_hands(root, &c.xml, jit, true, true);
        let repeat = observed_with_hands(root, &c.xml, jit, true, true);
        let unhooked = observed_with_hands(root, &c.xml, jit, false, true);
        let capture = json!({"name":c.name,"source":hand_source_frame(&c.xml),
            "synthetic_item_lines":c.profile_type == "One Hand Mace" || c.filtered_main || c.facebreaker_lines,
            "original":original,"repeat":repeat,"unhooked":unhooked});
        fs::write(
            out.join(format!("source-jit-{mode}-case-{:02}.raw.json", i + 1)),
            serde_json::to_vec_pretty(&capture).unwrap(),
        )
        .unwrap();
        for (observed, hooked) in [(&original, true), (&repeat, true), (&unhooked, false)] {
            check_hand_capture(observed, c, hooked);
        }
        assert!(
            original == repeat,
            "fresh deterministic replay {} (raw reports retained)",
            c.name
        );
        assert!(
            without_hand_capture(original.clone()) == without_hand_capture(unhooked.clone()),
            "read-only hooks changed source state or scalar output {} (raw reports retained)",
            c.name
        );
        if c.name == "original-05" {
            let mut base_snapshot = unhooked;
            base_snapshot["state"]
                .as_object_mut()
                .unwrap()
                .remove("prepared_hands");
            assert!(
                base_snapshot == prior_original["unhooked"],
                "original off-hand default fields differ from the authenticated earlier unhooked load"
            );
        }
        captures.push(capture);
    }
    let report = json!({"schema_version":1,"source_revision":pinned::UPSTREAM_REVISION,
        "manifest_sha256":pinned::manifest_sha256(),"observer_sha256":hash(HANDS_OBSERVE.as_bytes()),
        "base_observer_sha256":hash(OBSERVE.as_bytes()),
        "bootstrap_sha256":hash(include_bytes!("support/configuration_preparation_source.rs")),
        "harness_sha256":hash(include_bytes!("owned_player_offhand_source.rs")),"files":files,
        "historical_default_regression":{"directory":"runs/owned-player-offhand-source-02","sha256":prior_sha256,
            "matching_jit_report_checked":true,"case":"original-05","all_default_fields_equal":true},
        "case_count":cases.len(),"complete_loads_per_jit":cases.len()*3,"cases":captures,
        "scope":{"class_dependency":"shared Player prepared hand and Unarmed/Unencumbered state remains unconverted; selected slot occupancy alone is insufficient",
            "original_profile_assignment":true,"original_condition_write_branches":true,"fresh_repeat":true,
            "fresh_unhooked_comparison":true,"scalar_output_comparison":true,"no_retry_or_settling":true,
            "separate_late_disable_branch":true,"late_disable_activated":false,
            "late_disable_positive_producer":"none identified in pinned source; no injected flag",
            "full_effective_condition_closure":false,"native_prepared_profile_law":false,
            "class_owner_closed":false,"actor_owner_closed":false,"full_native_build_parity":false,
            "synthetic_items_obtainable":false,"full_output_graph":false}});
    fs::write(
        out.join(format!("source-jit-{mode}.json")),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "requires complete pinned PoB source; bounded original prepared-hand observer"]
fn actual_player_prepared_hands_distinguish_item_profile_and_condition_writes() {
    run_source_test(
        HANDS_TEST,
        HANDS_CHILD,
        HANDS_OUTPUT,
        "runs/owned-player-prepared-hands-source-01",
        hand_child,
    );
}
