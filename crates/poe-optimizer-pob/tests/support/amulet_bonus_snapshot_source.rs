//! Actual pre-copy Amulet query, with immutable source reports and finite controls.
use super::magnified_area_support::set_attr;
use super::*;

const TEST_NAME: &str =
    "amulet_bonus_snapshot::amulet_snapshot_observes_original_sum_and_copy_boundary";
const OBSERVER: &str = include_str!("amulet_bonus_snapshot_source.lua");
const STAT: &str = "EffectOfBonusesFromAmulet";

#[test]
#[ignore = "requires pinned PoB; actual pre-Amulet query in both JIT modes"]
fn amulet_snapshot_observes_original_sum_and_copy_boundary() {
    super::physical_support::run_modes(
        super::physical_support::Witness {
            name: TEST_NAME,
            child_env: "POE_AMULET_BONUS_SNAPSHOT_SOURCE_CHILD",
            output_env: "POE_AMULET_BONUS_SNAPSHOT_SOURCE_OUT",
            default_output: "runs/owned-amulet-bonus-snapshot-source-01",
            label: "Amulet snapshot",
        },
        child,
    );
}

fn child(root: &Path, out: &Path, enabled: bool) {
    assert_eq!(
        pinned::manifest_sha256(),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let dir = root.join("tests/fixtures/builds/breadth-20260908");
    let index: Json = serde_json::from_slice(&fs::read(dir.join("index.json")).unwrap()).unwrap();
    let originals: Vec<_> = (1..=5)
        .map(|i| fs::read(dir.join(format!("build-{i:02}.xml"))).unwrap())
        .collect();
    let mut cases = vec![];
    for (i, bytes) in originals.iter().enumerate() {
        assert_eq!(digest(bytes), index["builds"][i]["xml_sha256"]);
        cases.push(observe(
            root,
            &format!("original-{:02}", i + 1),
            std::str::from_utf8(bytes).unwrap(),
            enabled,
            true,
            None,
        ));
    }
    let original = std::str::from_utf8(&originals[4]).unwrap();
    let controls = controls(original);
    for (name, xml, control) in &controls {
        cases.push(observe(
            root,
            name,
            xml,
            enabled,
            true,
            Some(control.clone()),
        ));
    }
    for (i, bytes) in originals.iter().enumerate() {
        cases.push(observe(
            root,
            &format!("repeat-original-{:02}", i + 1),
            std::str::from_utf8(bytes).unwrap(),
            enabled,
            true,
            None,
        ));
    }
    cases.push(observe(
        root,
        "unhooked-original-05",
        original,
        enabled,
        false,
        None,
    ));
    let amulet = controls
        .iter()
        .find(|r| r.0 == "amulet-percent-100")
        .unwrap();
    cases.push(observe(
        root,
        "unhooked-amulet-percent-100",
        &amulet.1,
        enabled,
        false,
        Some(amulet.2.clone()),
    ));
    assert_eq!(cases.len(), 19);
    let mut files = FILES.to_vec();
    files.extend([
        "src/Classes/ModStore.lua",
        "src/Classes/ModDB.lua",
        "src/Classes/ModList.lua",
        "src/Classes/Item.lua",
        "src/Classes/ConfigTab.lua",
        "src/Classes/TreeTab.lua",
        "src/Modules/ModParser.lua",
        "src/Data/ModCache.lua",
        "src/TreeData/0_5/tree.lua",
        "src/Classes/PassiveTree.lua",
    ]);
    files.sort_unstable();
    files.dedup();
    let report = json!({"schema_version":1,"source_revision":pinned::UPSTREAM_REVISION,"manifest_sha256":pinned::manifest_sha256(),
        "observer_sha256":digest(OBSERVER.as_bytes()),"lifecycle_sha256":digest(LIFECYCLE.as_bytes()),
        "files":files.into_iter().map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})).collect::<Vec<_>>(),
        "original_sources":originals.iter().enumerate().map(|(i,b)|json!({"path":format!("tests/fixtures/builds/breadth-20260908/build-{:02}.xml",i+1),"sha256":digest(b)})).collect::<Vec<_>>(),
        "lifecycle_stages":STAGES,"numeric_tolerance":0,"business_wrappers":false,"source_tables_mutated":false,
        "scope":{"current_capture_bucket":true,"incoming_domain":"untagged INC, flags0, keywordFlags0, finite numeric values",
          "arbitrary_supplier_domain":false,"whole_build_closure":false,"game_legality":false,"native_inventory_closure":false,
          "capture":"CalcSetup.lua1662 original Sum before Amulet ScaleAddMod1667; post-copy bucket is diagnostic"},
        "cases":cases});
    let bytes = serde_json::to_vec(&report).unwrap();
    let suffix = if enabled { "on" } else { "off" };
    let raw = out.join(format!("source-jit-{suffix}.raw.json"));
    fs::write(&raw, &bytes).unwrap();
    eprintln!(
        "Amulet snapshot evidence: {} bytes at {}",
        bytes.len(),
        raw.display()
    );
    assert!(bytes.len() <= 64 * 1024 * 1024, "bounded source report");
    check(&report);
    fs::write(out.join(format!("source-jit-{suffix}.json")), bytes).unwrap();
    for (i, bytes) in originals.iter().enumerate() {
        assert!(
            fs::read(dir.join(format!("build-{:02}.xml", i + 1))).unwrap() == *bytes,
            "original fixture changed"
        );
    }
}

fn observe(
    root: &Path,
    name: &str,
    xml: &str,
    enabled: bool,
    instrumented: bool,
    control: Option<Json>,
) -> Json {
    eprintln!(
        "Amulet snapshot case {name}, JIT {}",
        if enabled { "on" } else { "off" }
    );
    let before = |lua: &Lua| -> Result<(), RuntimeError> {
        lua.globals()
            .set("amuletSnapshotInstrumented", instrumented)?;
        lua.globals().set("amuletSnapshotJit", enabled)?;
        lua.load("if amuletSnapshotJit then jit.on() else jit.off();jit.flush() end")
            .exec()?;
        Ok(())
    };
    let before_build = |lua: &Lua| -> Result<Function, RuntimeError> {
        lua.globals().set("djinnPhase", "before")?;
        let lifecycle: Function = lua
            .load(LIFECYCLE)
            .set_name("@amulet-snapshot-original-lifecycle")
            .eval()?;
        let api: mlua::Table = lua
            .load(OBSERVER)
            .set_name("@amulet-snapshot-observer")
            .eval()?;
        let install: Function = api.get("install")?;
        let cleanup: Function = install.call(())?;
        lua.globals().set("amuletSnapshotApi", api)?;
        let combine:Function=lua.load("return function(observer,auth) return function() local ok,err=pcall(observer);auth();if not ok then error(err,0)end end end").eval()?;
        Ok(combine.call((cleanup, lifecycle))?)
    };
    let observer = |lua: &Lua| -> Result<Json, RuntimeError> {
        let api: mlua::Table = lua.globals().get("amuletSnapshotApi")?;
        let observe: Function = api.get("observe")?;
        let rebuild: Function = api.get("rebuild")?;
        let mut states = serde_json::Map::new();
        for (i, stage) in STAGES.iter().enumerate() {
            if i > 0 {
                rebuild.call::<()>(())?;
            }
            let a: Value = observe.call(())?;
            let a: Json = lua.from_value(a)?;
            let b: Value = observe.call(())?;
            let b: Json = lua.from_value(b)?;
            same(&a, &b, "observer noninterference");
            states.insert((*stage).into(), a);
        }
        Ok(Json::Object(states))
    };
    let scratch = tempfile::tempdir().unwrap();
    let observed = source::observe_with_build_hook_unwrapped(
        &root.join("vendor/path-of-building-poe2"),
        scratch.path(),
        xml,
        None,
        false,
        Some(&before),
        Some(&before_build),
        Some(&observer),
    )
    .unwrap_or_else(|e| panic!("{name}: source failed: {e}"));
    assert_eq!(observed["configuration_method_wrappers"], false);
    assert_eq!(observed["original_build_output_available"], true);
    json!({"name":name,"xml_sha256":digest(xml.as_bytes()),"instrumented":instrumented,"control":control,"states":observed["additional_observation"]})
}

fn allocate(xml: &str, include_mystic: bool) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let tree = doc.descendants().find(|n| n.has_tag_name("Tree")).unwrap();
    let active: usize = tree.attribute("activeSpec").unwrap().parse().unwrap();
    let spec = tree
        .children()
        .filter(|n| n.has_tag_name("Spec"))
        .nth(active - 1)
        .unwrap();
    let nodes = spec.attribute("nodes").unwrap();
    assert!(!nodes.split(',').any(|n| n == "7068"));
    // Ordinary source loading prunes an orphan notable. Select its real
    // Huntress/Ritualist context and connected ascendancy path in this control.
    let path = if include_mystic {
        "36365,58574,34785,3223,7068"
    } else {
        "36365,58574,34785,3223"
    };
    let mut selected = set_attr(&xml[spec.range()], "nodes", &format!("{nodes},{path}"));
    for (key, value) in [
        ("classId", "8"),
        ("classInternalId", "8"),
        ("ascendClassId", "3"),
        ("ascendancyInternalId", "Huntress3"),
    ] {
        selected = set_attr(&selected, key, value);
    }
    let mut out = xml.to_owned();
    out.replace_range(spec.range(), &selected);
    out
}
fn item_control(xml: &str, slot: &str, remove: bool) -> (String, String) {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let items = doc.descendants().find(|n| n.has_tag_name("Items")).unwrap();
    let set = items
        .children()
        .find(|n| {
            n.has_tag_name("ItemSet") && n.attribute("id") == items.attribute("activeItemSet")
        })
        .unwrap();
    let selected = set
        .children()
        .find(|n| n.has_tag_name("Slot") && n.attribute("name") == Some(slot))
        .unwrap();
    let id = selected.attribute("itemId").unwrap().to_owned();
    let mut out = xml.to_owned();
    if remove {
        out.replace_range(
            selected.range(),
            &set_attr(&xml[selected.range()], "itemId", "0"),
        );
    } else {
        let item = items
            .children()
            .find(|n| n.has_tag_name("Item") && n.attribute("id") == Some(id.as_str()))
            .unwrap();
        let text = item.children().find(|n| n.is_text()).unwrap();
        let raw = &xml[text.range()];
        let extra = "\n100% increased bonuses gained from equipped rings and amulets\n";
        out.replace_range(text.range(), &format!("{raw}{extra}"));
    }
    roxmltree::Document::parse(&out).unwrap();
    (out, id)
}
fn controls(xml: &str) -> Vec<(String, String, Json)> {
    let allocated = allocate(xml, true);
    let removed = allocate(xml, false);
    assert_eq!(allocated.matches(",7068\"").count(), 1);
    assert_eq!(
        allocated.replacen(",7068\"", "\"", 1),
        removed,
        "removing only the notable restores the connected diagnostic baseline"
    );
    let baseline_sha = digest(removed.as_bytes());
    let (boots, boot_id) = item_control(xml, "Boots", false);
    let (amulet, amulet_id) = item_control(xml, "Amulet", false);
    let (both, _) = item_control(&allocated, "Boots", false);
    let added_line = "\n100% increased bonuses gained from equipped rings and amulets\n";
    assert_eq!(both.matches(added_line).count(), 1);
    assert_eq!(
        both.replacen(added_line, "", 1),
        allocated,
        "removing only the injected item line restores the allocated diagnostic baseline"
    );
    let (absent, _) = item_control(xml, "Amulet", true);
    [
        ("allocated-mystic-attunement",allocated.clone(),25,Some(7068),None,false),
        ("remove-mystic-attunement",removed,0,None,None,false),
        ("boots-percent-100",boots,100,None,Some(boot_id.clone()),false),
        ("amulet-percent-100",amulet,100,None,Some(amulet_id.clone()),false),
        ("boots-and-passive-125",both,125,Some(7068),Some(boot_id),false),
        ("remove-boots-percent",allocated,25,Some(7068),None,false),
        ("absent-amulet",absent,0,None,Some(amulet_id),true),
    ].into_iter().map(|(name,xml,sum,node,item,absent)| (name.into(),xml,json!({"expected_pre_copy_percent":sum,"allocated_node":node,"item_id":item,"absent_amulet":absent,"allocation_path_legality_authority":false,"roll_legality_authority":false,"ritualist_control":matches!(name,"allocated-mystic-attunement"|"remove-mystic-attunement"|"boots-and-passive-125"|"remove-boots-percent"),"node_removal_restores_connected_baseline":name=="remove-mystic-attunement","connected_baseline_xml_sha256":if matches!(name,"allocated-mystic-attunement"|"remove-mystic-attunement"){Some(&baseline_sha)}else{None}}))).collect()
}
fn same(a: &Json, b: &Json, label: &str) {
    assert_eq!(json_evidence::first_difference(a, b, label), None);
}
fn named<'a>(report: &'a Json, name: &str) -> &'a Json {
    let found: Vec<_> = rows(&report["cases"])
        .iter()
        .filter(|c| c["name"] == name)
        .collect();
    assert_eq!(found.len(), 1);
    found[0]
}
fn total(chain: &Json) -> f64 {
    rows(chain)
        .iter()
        .map(|s| {
            rows(&s["rows"])
                .iter()
                .map(|r| r["record"]["value"].as_f64().unwrap())
                .sum::<f64>()
        })
        .sum()
}
fn check(report: &Json) {
    assert_eq!(rows(&report["cases"]).len(), 19);
    for case in rows(&report["cases"]) {
        for stage in STAGES {
            let state = &case["states"][stage];
            let snapshot = &state["snapshot"];
            for k in [
                "complete_frames",
                "original_methods_preserved",
                "hook_removed",
            ] {
                assert_eq!(snapshot[k], true);
            }
            for k in [
                "business_wrappers",
                "diagnostic_query_substitution",
                "whole_supplier_domain_complete",
                "native_inventory_authority",
            ] {
                assert_eq!(snapshot[k], false);
            }
            assert!(snapshot["work"].as_u64().unwrap() <= 2_000_000);
            for env in rows(&state["environments"]) {
                assert_eq!(env["actual_selected_env_observed"], true);
            }
            if case["instrumented"] == false {
                assert!(rows(&snapshot["queries"]).is_empty());
                continue;
            }
            let absent = case["control"]["absent_amulet"] == true;
            if absent {
                assert!(rows(&snapshot["queries"]).is_empty());
            }
            for q in rows(&snapshot["queries"]) {
                for k in [
                    "cfg_absent",
                    "store_is_player",
                    "bucket_complete",
                    "untagged_domain",
                    "query_return_observed",
                ] {
                    assert_eq!(q[k], true);
                }
                assert_eq!(q["query_name"], STAT);
                assert_eq!(q["query_type"], "INC");
                assert_eq!(q["caller_line"], 1662);
                let expected = if case["control"].is_object() {
                    case["control"]["expected_pre_copy_percent"]
                        .as_f64()
                        .unwrap()
                } else {
                    total(&q["before"])
                };
                assert_eq!(q["result"].as_f64().unwrap(), expected);
                assert_eq!(total(&q["before"]), expected);
                assert_eq!(rows(&q["internal"]).len(), rows(&q["before"]).len());
                for depth in 0..rows(&q["before"]).len() {
                    let r = rows(&q["internal"])
                        .iter()
                        .find(|r| r["depth"] == depth)
                        .unwrap();
                    assert_eq!(r["original_return"], true);
                    assert_eq!(
                        r["result"].as_f64().unwrap(),
                        rows(&q["before"])[depth..]
                            .iter()
                            .map(|s| rows(&s["rows"])
                                .iter()
                                .map(|r| r["record"]["value"].as_f64().unwrap())
                                .sum::<f64>())
                            .sum::<f64>()
                    );
                }
                for layer in rows(&q["before"]) {
                    for r in rows(&layer["rows"]) {
                        let m = &r["record"];
                        assert_eq!(m["name"], STAT);
                        assert_eq!(m["type"], "INC");
                        assert_eq!(m["flags"], 0);
                        assert_eq!(m["keywordFlags"], 0);
                        assert!(m.get("positions").is_none());
                    }
                }
                for item in rows(&q["source_candidates"]["items"]) {
                    assert_eq!(item["exact_registered"], true);
                }
                if case["control"]["ritualist_control"] == true {
                    assert_eq!(
                        q["source_candidates"]["class"],
                        json!({"id":8,"name":"Huntress","ascendancy_id":3,"ascendancy_name":"Ritualist"})
                    );
                    for id in [36365, 58574, 34785, 3223] {
                        assert!(
                            rows(&q["source_candidates"]["nodes"])
                                .iter()
                                .any(|n| n["id"] == id),
                            "connected Ritualist path node {id}"
                        );
                    }
                    if case["name"] == "remove-mystic-attunement" {
                        assert!(
                            rows(&q["source_candidates"]["nodes"])
                                .iter()
                                .all(|n| n["id"] != 7068)
                        );
                    }
                }
                let selected = case["control"]["allocated_node"].as_u64();
                if let Some(id) = selected {
                    let n = rows(&q["source_candidates"]["nodes"])
                        .iter()
                        .find(|n| n["id"] == id)
                        .unwrap();
                    assert_eq!(n["name"], "Mystic Attunement");
                    assert!(!rows(&n["original_returns"]).is_empty());
                    assert!(rows(&n["original_returns"]).iter().any(|r| {
                        rows(&r["returned"]["rows"])
                            .iter()
                            .any(|m| m["record"]["name"] == STAT && m["record"]["value"] == 25)
                    }));
                    assert!(rows(&q["before"]).iter().any(|s| {
                        rows(&s["rows"]).iter().any(|r| {
                            r["record"]["source"] == "Tree:7068" && r["record"]["value"] == 25
                        })
                    }));
                }
                assert!(!rows(&q["copies"]).is_empty());
                for copy in rows(&q["copies"]) {
                    for k in [
                        "exact_source_object",
                        "exact_copy_argument",
                        "exact_destination",
                        "return_observed",
                    ] {
                        assert_eq!(copy[k], true);
                    }
                    assert_eq!(copy["factor"].as_f64().unwrap(), expected / 100.0);
                    assert_eq!(rows(&copy["insertions"]).len(), 1);
                    assert_eq!(copy["insertions"][0]["exact_inserted_object"], true);
                }
                if case["name"] == "amulet-percent-100" {
                    let copies = rows(&q["copies"]);
                    let percent: Vec<_> = copies
                        .iter()
                        .filter(|c| c["original_record"]["name"] == STAT)
                        .collect();
                    assert_eq!(percent.len(), 1);
                    assert_eq!(percent[0]["original_record"]["value"], 100);
                    assert_eq!(percent[0]["insertions"][0]["record"]["value"], 100);
                    assert_eq!(total(&copies.last().unwrap()["after_bucket"]), 200.0);
                }
            }
            if case["name"] == "original-05" {
                assert!(!rows(&snapshot["queries"]).is_empty());
                for q in rows(&snapshot["queries"]) {
                    assert_eq!(q["result"], 0);
                    assert_eq!(total(&q["before"]), 0.0);
                }
            }
        }
    }
    for i in 1..=5 {
        same(
            &named(report, &format!("original-{i:02}"))["states"],
            &named(report, &format!("repeat-original-{i:02}"))["states"],
            "independent original replay",
        );
    }
    {
        let (a, b) = ("allocated-mystic-attunement", "remove-boots-percent");
        assert_eq!(
            named(report, a)["xml_sha256"],
            named(report, b)["xml_sha256"]
        );
        same(
            &named(report, a)["states"],
            &named(report, b)["states"],
            "exact supplier removal restores input and observation",
        );
    }
    let removed = named(report, "remove-mystic-attunement");
    for name in ["allocated-mystic-attunement", "remove-mystic-attunement"] {
        assert_eq!(
            named(report, name)["control"]["connected_baseline_xml_sha256"],
            removed["xml_sha256"]
        );
    }
    assert_eq!(
        removed["control"]["node_removal_restores_connected_baseline"],
        true
    );
    assert_ne!(
        removed["xml_sha256"],
        named(report, "original-05")["xml_sha256"]
    );
    for base in ["original-05", "amulet-percent-100"] {
        for stage in STAGES {
            same(
                &named(report, base)["states"][stage]["outputs"],
                &named(report, &format!("unhooked-{base}"))["states"][stage]["outputs"],
                "original output with and without hook",
            );
        }
    }
}
