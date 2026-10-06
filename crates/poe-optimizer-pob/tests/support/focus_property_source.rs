//! Actual Focus branch transport and its physical Sniper property consumer.
use super::magnified_area_support::set_attr;
use super::*;
use std::collections::BTreeSet;

const TEST_NAME: &str = "focus_property::focus_scaling_drops_only_the_duplicate_list_property";
const OBSERVER: &str = include_str!("focus_property_source.lua");
const PROPERTY_LINE: &str = "{desecrated}+2 to Level of all Minion Skills";
const SNIPER: &str = "SummonSkeletalSnipersPlayer";

#[test]
#[ignore = "requires pinned PoB; actual Focus branch and physical property consumer"]
fn focus_scaling_drops_only_the_duplicate_list_property() {
    super::physical_support::run_modes(
        super::physical_support::Witness {
            name: TEST_NAME,
            child_env: "POE_FOCUS_PROPERTY_SOURCE_CHILD",
            output_env: "POE_FOCUS_PROPERTY_SOURCE_OUT",
            default_output: "runs/owned-focus-property-source-01",
            label: "Focus properties",
        },
        child,
    );
}

fn child(root: &Path, out: &Path, enabled: bool) {
    let dir = root.join("tests/fixtures/builds/breadth-20260908");
    let index: Json = serde_json::from_slice(&fs::read(dir.join("index.json")).unwrap()).unwrap();
    let original = fs::read(dir.join("build-01.xml")).unwrap();
    assert_eq!(digest(&original), index["builds"][0]["xml_sha256"]);
    let xml = std::str::from_utf8(&original).unwrap();
    let controls = controls(xml);
    let mut cases = vec![observe(root, "original-01", xml, enabled, true, None)];
    for (name, value, control) in &controls {
        cases.push(observe(
            root,
            name,
            value,
            enabled,
            true,
            Some(control.clone()),
        ));
    }
    cases.push(observe(
        root,
        "repeat-original-01",
        xml,
        enabled,
        true,
        None,
    ));
    let positive = controls.iter().find(|c| c.0 == "focus-supplier").unwrap();
    cases.push(observe(
        root,
        "repeat-focus-supplier",
        &positive.1,
        enabled,
        true,
        Some(positive.2.clone()),
    ));
    cases.push(observe(
        root,
        "unhooked-original-01",
        xml,
        enabled,
        false,
        None,
    ));
    cases.push(observe(
        root,
        "unhooked-focus-supplier",
        &positive.1,
        enabled,
        false,
        Some(positive.2.clone()),
    ));
    assert_eq!(cases.len(), 10);
    let mut files = FILES.to_vec();
    files.extend([
        "src/Classes/ModStore.lua",
        "src/Classes/ModDB.lua",
        "src/Classes/ModList.lua",
        "src/Classes/Item.lua",
        "src/Classes/ItemsTab.lua",
        "src/Modules/ModParser.lua",
        "src/Data/ModCache.lua",
        "src/Data/Skills/act_int.lua",
        "src/TreeData/0_5/tree.lua",
    ]);
    files.sort_unstable();
    files.dedup();
    let report = json!({"schema_version":1,"source_revision":pinned::UPSTREAM_REVISION,"manifest_sha256":pinned::manifest_sha256(),
      "observer_sha256":digest(OBSERVER.as_bytes()),"lifecycle_sha256":digest(LIFECYCLE.as_bytes()),
      "files":files.into_iter().map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})).collect::<Vec<_>>(),
      "original_source":{"path":"tests/fixtures/builds/breadth-20260908/build-01.xml","sha256":digest(&original)},
      "lifecycle_stages":STAGES,"numeric_tolerance":0,"business_wrappers":false,
      "scope":{"actual_focus_transport":true,"actual_physical_sniper_consumer":true,"complete_effective_allocation_deltas":true,
        "whole_build_closure":false,"native_owner_closure":false,"intended_gameplay_law":false},"cases":cases});
    let bytes = serde_json::to_vec(&report).unwrap();
    let suffix = if enabled { "on" } else { "off" };
    let raw = out.join(format!("source-jit-{suffix}.raw.json"));
    fs::write(&raw, &bytes).unwrap();
    eprintln!("Focus evidence: {} bytes at {}", bytes.len(), raw.display());
    assert!(bytes.len() <= 32 * 1024 * 1024, "bounded source report");
    check(&report);
    fs::write(out.join(format!("source-jit-{suffix}.json")), bytes).unwrap();
    assert_eq!(fs::read(dir.join("build-01.xml")).unwrap(), original);
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
        "Focus case {name}, JIT {}",
        if enabled { "on" } else { "off" }
    );
    let before = |lua: &Lua| -> Result<(), RuntimeError> {
        lua.globals().set("focusInstrumented", instrumented)?;
        lua.globals().set("focusJit", enabled)?;
        lua.globals().set("focusXml", xml)?;
        lua.load("if focusJit then jit.on() else jit.off();jit.flush() end")
            .exec()?;
        Ok(())
    };
    let before_build = |lua: &Lua| -> Result<Function, RuntimeError> {
        lua.globals().set("djinnPhase", "before")?;
        let lifecycle: Function = lua
            .load(LIFECYCLE)
            .set_name("@focus-original-lifecycle")
            .eval()?;
        let api: mlua::Table = lua
            .load(OBSERVER)
            .set_name("@focus-property-observer")
            .eval()?;
        let install: Function = api.get("install")?;
        let cleanup: Function = install.call(())?;
        lua.globals().set("focusApi", api)?;
        let combine: Function = lua.load("return function(observer,auth) return function() local ok,err=pcall(observer);auth();if not ok then error(err,0)end end end").eval()?;
        Ok(combine.call((cleanup, lifecycle))?)
    };
    let observer = |lua: &Lua| -> Result<Json, RuntimeError> {
        let api: mlua::Table = lua.globals().get("focusApi")?;
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
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([117; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    let states = &observed["additional_observation"];
    for stage in STAGES {
        for env in rows(&states[stage]["environments"]) {
            for skill in rows(&env["skills"]) {
                let binding = &skill["source"];
                let row = evidence
                    .rows()
                    .get(usize::try_from(binding["source_ordinal"].as_u64().unwrap()).unwrap())
                    .unwrap();
                assert_eq!(row.occurrence().name(), "Gem");
                assert_eq!(
                    row.attributes().len(),
                    binding["attributes"].as_object().unwrap().len()
                );
                for a in row.attributes() {
                    assert_eq!(
                        binding["attributes"][&a.origin().name],
                        a.decoded().unwrap()
                    );
                }
                let group = evidence.row(row.occurrence().parent().unwrap()).unwrap();
                assert_eq!(group.occurrence().name(), "Skill");
                assert!(group.attribute("source").is_none());
                assert_eq!(
                    binding["group_source_ordinal"].as_u64(),
                    Some(u64::from(group.occurrence().id().ordinal()))
                );
                let set = evidence.row(group.occurrence().parent().unwrap()).unwrap();
                assert_eq!(set.occurrence().name(), "SkillSet");
                assert_eq!(
                    binding["preset"].as_u64(),
                    Some(
                        set.attribute("id")
                            .unwrap()
                            .decoded()
                            .unwrap()
                            .parse::<u64>()
                            .unwrap()
                    )
                );
            }
        }
    }
    json!({"name":name,"xml_sha256":digest(xml.as_bytes()),"source_identity":evidence.identity(),"independent_source_bindings_verified":true,"instrumented":instrumented,"control":control,"states":states})
}

fn selected_item<'a, 'input>(
    doc: &'a roxmltree::Document<'input>,
    slot: &str,
) -> roxmltree::Node<'a, 'input> {
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
    items
        .children()
        .find(|n| n.has_tag_name("Item") && n.attribute("id") == selected.attribute("itemId"))
        .unwrap()
}
fn line(xml: &str, slot: &str, text: &str, remove: bool) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let item = selected_item(&doc, slot);
    let node = item.children().find(|n| n.is_text()).unwrap();
    let raw = &xml[node.range()];
    let modified = if remove {
        assert_eq!(raw.matches(text).count(), 1);
        raw.replacen(text, "", 1)
    } else {
        format!("{raw}{text}")
    };
    let mut out = xml.to_owned();
    out.replace_range(node.range(), &modified);
    out
}
fn same(a: &Json, b: &Json, label: &str) {
    assert_eq!(json_evidence::first_difference(a, b, label), None);
}
fn named<'a>(report: &'a Json, name: &str) -> &'a Json {
    let v: Vec<_> = rows(&report["cases"])
        .iter()
        .filter(|c| c["name"] == name)
        .collect();
    assert_eq!(v.len(), 1);
    v[0]
}
fn nodes(xml: &str, remove: &[u32], add: &[u32]) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let tree = doc.descendants().find(|n| n.has_tag_name("Tree")).unwrap();
    assert_eq!(tree.attribute("activeSpec"), Some("1"));
    let specs: Vec<_> = tree.children().filter(|n| n.has_tag_name("Spec")).collect();
    assert_eq!(specs.len(), 1);
    let spec = specs[0];
    let mut ids: Vec<u32> = spec
        .attribute("nodes")
        .unwrap()
        .split(',')
        .map(|v| v.parse().unwrap())
        .collect();
    assert_eq!(ids.iter().collect::<BTreeSet<_>>().len(), ids.len());
    for id in remove {
        let position = ids.iter().position(|i| i == id).unwrap();
        ids.remove(position);
    }
    for id in add {
        assert!(!ids.contains(id));
        ids.push(*id);
    }
    assert!(
        ids.contains(&36891),
        "preserve the existing Time-Lost radius path"
    );
    let replacement = set_attr(
        &xml[spec.range()],
        "nodes",
        &ids.iter().map(u32::to_string).collect::<Vec<_>>().join(","),
    );
    let mut result = xml.to_owned();
    result.replace_range(spec.range(), &replacement);
    result
}
fn controls(original: &str) -> Vec<(String, String, Json)> {
    let baseline = nodes(original, &[30265, 23265], &[35880]);
    let positive = nodes(&baseline, &[], &[20701]);
    let removed = nodes(&positive, &[20701], &[]);
    let restored = nodes(&removed, &[], &[20701]);
    assert_eq!(removed, baseline);
    assert_eq!(restored, positive);
    let property_removed = line(&positive, "Weapon 2", PROPERTY_LINE, true);
    [
        ("focus-matched-baseline", baseline, false, false),
        ("focus-supplier", positive, true, false),
        ("remove-focus-supplier", removed, false, false),
        ("restore-focus-supplier", restored, true, false),
        ("remove-focus-property", property_removed, true, true),
    ]
    .into_iter()
    .map(|(name, xml, supplier, removed)| {
        (
            name.into(),
            xml,
            json!({
      "supplier":supplier,"focus_property_removed":removed,"removed_original_nodes":[30265,23265],
      "added_original_nodes":if supplier {vec![35880,20701]}else{vec![35880]},
      "physical_slot":"Weapon 2","focus_item_id":1,"property_line":PROPERTY_LINE,
      "fixture_roll_legality_authority":false,"native_owner_closure":false}),
        )
    })
    .collect()
}
#[test]
fn focus_controls_preserve_unrelated_saved_inputs() {
    let original = include_str!("../../../../tests/fixtures/builds/breadth-20260908/build-01.xml");
    let controls = controls(original);
    assert_eq!(controls.len(), 5);
    for (_, xml, control) in controls {
        let original_doc = roxmltree::Document::parse(original).unwrap();
        let changed = roxmltree::Document::parse(&xml).unwrap();
        for name in ["Build", "Skills", "Config", "Notes", "TreeView"] {
            let before = original_doc.descendants().find(|n| n.has_tag_name(name));
            let after = changed.descendants().find(|n| n.has_tag_name(name));
            assert_eq!(
                before.map(|n| &original[n.range()]),
                after.map(|n| &xml[n.range()])
            );
        }
        let item = selected_item(&changed, "Weapon 2");
        assert_eq!(item.attribute("id"), Some("1"));
        assert_eq!(
            xml[item.range()].matches(PROPERTY_LINE).count(),
            usize::from(control["focus_property_removed"] == false)
        );
    }
}
fn physical_inputs(state: &Json) -> Json {
    json!(rows(&state["environments"]).iter().map(|env|json!({"mode":env["mode"],"allocations":env["allocations"],
      "skills":rows(&env["skills"]).iter().map(|s|json!({"effect":s["effect"],"source":s["source"],"raw":s["raw"],"final":s["final"],
        "final_lookup":s["final_lookup"],"stat_set":s["stat_set"],"exact_physical_source":s["exact_physical_source"]})).collect::<Vec<_>>()
    })).collect::<Vec<_>>())
}
fn allocation_ids(env: &Json) -> BTreeSet<u64> {
    let rows = rows(&env["allocations"]);
    let ids: BTreeSet<_> = rows.iter().map(|r| r["id"].as_u64().unwrap()).collect();
    assert_eq!(ids.len(), rows.len());
    ids
}
fn focus_rows(value: &Json) -> Vec<&Json> {
    rows(value)
        .iter()
        .filter(|r| r["record"]["name"] == "GemProperty" && r["record"]["sourceSlot"] == "Focus")
        .collect()
}
fn check(report: &Json) {
    assert_eq!(rows(&report["cases"]).len(), 10);
    for case in rows(&report["cases"]) {
        assert_eq!(case["independent_source_bindings_verified"], true);
        let control = &case["control"];
        let supplier = control["supplier"] == true;
        let removed = control["focus_property_removed"] == true;
        for stage in STAGES {
            let state = &case["states"][stage];
            for key in ["original_methods_preserved", "hook_removed"] {
                assert_eq!(state[key], true);
            }
            for key in [
                "business_wrappers",
                "source_tables_mutated",
                "diagnostic_requery",
                "native_owner_closure",
                "intended_gameplay_law",
            ] {
                assert_eq!(state[key], false);
            }
            assert_eq!(rows(&state["environments"]).len(), 2);
            for (index, env) in rows(&state["environments"]).iter().enumerate() {
                let original_env =
                    &named(report, "original-01")["states"][stage]["environments"][index];
                let original_ids = allocation_ids(original_env);
                let actual = allocation_ids(env);
                let mut expected = original_ids.clone();
                if !control.is_null() {
                    for id in [30265, 23265] {
                        assert!(expected.remove(&id));
                    }
                    assert!(expected.insert(35880));
                    if supplier {
                        assert!(expected.insert(20701));
                    }
                }
                assert_eq!(
                    actual, expected,
                    "complete effective allocation delta: {} {stage}",
                    case["name"]
                );
                assert!(actual.contains(&36891));
                // Every retained node keeps its complete recorded identity; no hidden effective replacement.
                for row in rows(&env["allocations"]) {
                    if let Some(prior) = rows(&original_env["allocations"])
                        .iter()
                        .find(|p| p["id"] == row["id"])
                    {
                        same(row, prior, "unchanged effective node identity");
                    }
                }
                let path = rows(&env["path"]);
                for (from, to) in [(8305, 35880), (35880, 20701)] {
                    let node = path.iter().find(|n| n["id"] == from).unwrap();
                    assert!(
                        rows(&node["connections"]["positions"])
                            .iter()
                            .any(|n| n["value"] == to)
                    );
                }
                let skills = rows(&env["skills"]);
                assert_eq!(skills.len(), 1);
                let skill = &skills[0];
                assert_eq!(skill["effect"], SNIPER);
                assert_eq!(skill["exact_physical_source"], true);
                assert_eq!(skill["raw"], original_env["skills"][0]["raw"]);
                assert_eq!(
                    skill["final"]["quality"],
                    original_env["skills"][0]["final"]["quality"]
                );
                assert_eq!(
                    skill["final"]["level"].as_f64().unwrap(),
                    original_env["skills"][0]["final"]["level"]
                        .as_f64()
                        .unwrap()
                        - if removed { 2.0 } else { 0.0 }
                );
                if case["instrumented"] == false {
                    continue;
                }
                let ret = &env["item_return"];
                assert_eq!(ret["original_return"], true);
                assert_eq!(ret["item"]["id"], 1);
                assert_eq!(ret["item"]["type"], "Focus");
                assert_eq!(ret["item"]["slot"], "Weapon 2");
                assert_eq!(ret["item"]["exact_registered"], true);
                let props = focus_rows(&ret["records"]);
                assert_eq!(props.len(), usize::from(!removed));
                if let Some(prop) = props.first() {
                    assert_eq!(prop["record"]["type"], "LIST");
                    assert_eq!(prop["record"]["flags"], 0);
                    assert_eq!(prop["record"]["keywordFlags"], 0);
                    assert_eq!(
                        prop["record"]["source"],
                        "Item:1:Plague Chant, Sacred Focus"
                    );
                    assert_eq!(
                        prop["record"]["value"],
                        json!({"key":"level","keyOfScaledMod":"value","keyword":"minion","value":2})
                    );
                    assert!(prop["record"].get("positions").is_none());
                }
                let merges = rows(&env["merges"]);
                let scaled = rows(&env["scaled"]);
                let deliveries = rows(&env["deliveries"]);
                assert_eq!(merges.len(), if supplier && !removed { 2 } else { 0 });
                assert_eq!(scaled.len(), usize::from(supplier && !removed));
                assert_eq!(deliveries.len(), usize::from(supplier));
                if supplier && !removed {
                    assert_eq!(merges[0]["caller_line"], 1477);
                    assert_eq!(merges[0]["argument_occurrences_after"], 1);
                    assert_eq!(merges[1]["caller_line"], 1482);
                    assert_eq!(merges[1]["skip_non_additive"], true);
                    assert_eq!(merges[1]["argument_occurrences_after"], 0);
                    same(
                        &merges[1]["before"],
                        &merges[1]["after"],
                        "rejected LIST candidate leaves combined list unchanged",
                    );
                    assert_eq!(merges[1]["record"]["value"]["value"], -1);
                    assert_eq!(scaled[0]["factor"].as_f64(), Some(-0.5));
                    assert_eq!(scaled[0]["record"]["value"]["value"], 2);
                    assert_eq!(scaled[0]["insertions"][0]["record"]["value"]["value"], -1);
                    for row in merges {
                        for key in [
                            "original_return",
                            "exact_source_object",
                            "exact_combined_destination",
                        ] {
                            assert_eq!(row[key], true);
                        }
                    }
                    assert_eq!(scaled[0]["original_return"], true);
                    assert_eq!(scaled[0]["insertions"][0]["exact_inserted_object"], true);
                    assert_eq!(scaled[0]["insertions"][0]["new_object"], true);
                }
                if supplier {
                    let contributors: Vec<_> = rows(&env["focus_effect_contributors"])
                        .iter()
                        .flat_map(|store| rows(&store["rows"]))
                        .collect();
                    assert_eq!(contributors.len(), 1);
                    assert_eq!(
                        contributors[0]["record"]["name"],
                        "EffectOfBonusesFromFocus"
                    );
                    assert_eq!(contributors[0]["record"]["type"], "INC");
                    assert_eq!(contributors[0]["record"]["value"], -50);
                    assert_eq!(contributors[0]["record"]["source"], "Tree:20701");
                    assert_eq!(deliveries[0]["original_return"], true);
                    assert_eq!(deliveries[0]["exact_destination"], true);
                    assert_eq!(rows(&deliveries[0]["joins"]).len(), usize::from(!removed));
                    if !removed {
                        assert_eq!(deliveries[0]["joins"][0]["record"]["value"]["value"], 2);
                    }
                }
                let ordinary = rows(&skill["ordinary"]);
                assert_eq!(ordinary.len(), 1);
                assert_eq!(ordinary[0]["original_query"], true);
                assert_eq!(ordinary[0]["exact_player_store"], true);
                let candidates = focus_rows(&ordinary[0]["candidates"]);
                assert_eq!(candidates.len(), usize::from(!removed));
                for candidate in candidates {
                    assert_eq!(candidate["record"]["value"]["value"], 2);
                    assert!(rows(&ordinary[0]["matched"]).contains(&candidate["index"]));
                    assert!(
                        rows(&ordinary[0]["item_object_joins"])
                            .iter()
                            .any(|j| j["candidate_index"] == candidate["index"])
                    );
                }
                assert_eq!(skill["assembly"]["original_call"], true);
                same(
                    &skill["assembly"]["after"],
                    &skill["final"],
                    "physical input stable after original assembly",
                );
            }
        }
    }
    for (a, b) in [
        ("original-01", "repeat-original-01"),
        ("focus-supplier", "repeat-focus-supplier"),
        ("focus-matched-baseline", "remove-focus-supplier"),
        ("focus-supplier", "restore-focus-supplier"),
    ] {
        assert_eq!(
            named(report, a)["xml_sha256"],
            named(report, b)["xml_sha256"]
        );
        same(
            &named(report, a)["states"],
            &named(report, b)["states"],
            "independent fresh and exact removal/restoration replay",
        );
    }
    for name in ["original-01", "focus-supplier"] {
        let a = named(report, name);
        let b = named(report, &format!("unhooked-{name}"));
        for stage in STAGES {
            same(
                &a["states"][stage]["outputs"],
                &b["states"][stage]["outputs"],
                "unhooked complete scalar outputs",
            );
            same(
                &physical_inputs(&a["states"][stage]),
                &physical_inputs(&b["states"][stage]),
                "unhooked physical inputs and full allocations",
            );
        }
    }
}
