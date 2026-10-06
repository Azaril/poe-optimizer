//! Original late slot-copy transport versus already prepared physical skills.
use super::magnified_area_support::{focus, set_attr};
use super::*;

const TEST_NAME: &str = "late_slot_property::late_slot_copies_do_not_reassemble_prepared_gems";
const OBSERVER: &str = include_str!("late_slot_property_source.lua");
const PROPERTY: &str = "+1 to Level of all Minion Skills";
const PAIN: &str = "PainOfferingPlayer";

#[test]
#[ignore = "requires pinned PoB; actual late slot-copy and property consumers"]
fn late_slot_copies_do_not_reassemble_prepared_gems() {
    super::physical_support::run_modes(
        super::physical_support::Witness {
            name: TEST_NAME,
            child_env: "POE_LATE_SLOT_PROPERTY_SOURCE_CHILD",
            output_env: "POE_LATE_SLOT_PROPERTY_SOURCE_OUT",
            default_output: "runs/owned-late-slot-property-source-01",
            label: "Late slot properties",
        },
        child,
    );
}

fn child(root: &Path, out: &Path, enabled: bool) {
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
    let positive = controls.iter().find(|c| c.0 == "ring-copy-100").unwrap();
    cases.push(observe(
        root,
        "unhooked-ring-copy-100",
        &positive.1,
        enabled,
        false,
        Some(positive.2.clone()),
    ));
    assert_eq!(cases.len(), 19);
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
    ]);
    files.sort_unstable();
    files.dedup();
    let report = json!({"schema_version":1,"source_revision":pinned::UPSTREAM_REVISION,"manifest_sha256":pinned::manifest_sha256(),
        "observer_sha256":digest(OBSERVER.as_bytes()),"lifecycle_sha256":digest(LIFECYCLE.as_bytes()),
        "files":files.into_iter().map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})).collect::<Vec<_>>(),
        "original_sources":originals.iter().enumerate().map(|(i,b)|json!({"path":format!("tests/fixtures/builds/breadth-20260908/build-{:02}.xml",i+1),"sha256":digest(b)})).collect::<Vec<_>>(),
        "lifecycle_stages":STAGES,"numeric_tolerance":0,"business_wrappers":false,
        "scope":{"actual_late_copy_transport":true,"actual_prepared_physical_inputs":true,"diagnostic_ring_outside_canonical_modifier_domain":true,
          "roll_legality_authority":false,"whole_build_closure":false,"native_owner_closure":false,"intended_gameplay_law":false,
          "late_query":"CalcPerform.lua1491","list_copy":"CalcPerform.lua1526","display_consumers":"CalcPerform.lua3630-3633"},"cases":cases});
    let bytes = serde_json::to_vec(&report).unwrap();
    let suffix = if enabled { "on" } else { "off" };
    let raw = out.join(format!("source-jit-{suffix}.raw.json"));
    fs::write(&raw, &bytes).unwrap();
    eprintln!(
        "Late slot evidence: {} bytes at {}",
        bytes.len(),
        raw.display()
    );
    assert!(bytes.len() <= 32 * 1024 * 1024, "bounded source report");
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
        "Late slot case {name}, JIT {}",
        if enabled { "on" } else { "off" }
    );
    let before = |lua: &Lua| -> Result<(), RuntimeError> {
        lua.globals().set("lateSlotInstrumented", instrumented)?;
        lua.globals().set("lateSlotJit", enabled)?;
        lua.globals().set("lateSlotXml", xml)?;
        lua.load("if lateSlotJit then jit.on() else jit.off();jit.flush() end")
            .exec()?;
        Ok(())
    };
    let before_build = |lua: &Lua| -> Result<Function, RuntimeError> {
        lua.globals().set("djinnPhase", "before")?;
        let lifecycle: Function = lua
            .load(LIFECYCLE)
            .set_name("@late-slot-original-lifecycle")
            .eval()?;
        let api: mlua::Table = lua
            .load(OBSERVER)
            .set_name("@late-slot-property-observer")
            .eval()?;
        let install: Function = api.get("install")?;
        let cleanup: Function = install.call(())?;
        lua.globals().set("lateSlotApi", api)?;
        let combine: Function = lua.load("return function(observer,auth) return function() local ok,err=pcall(observer);auth();if not ok then error(err,0)end end end").eval()?;
        Ok(combine.call((cleanup, lifecycle))?)
    };
    let observer = |lua: &Lua| -> Result<Json, RuntimeError> {
        let api: mlua::Table = lua.globals().get("lateSlotApi")?;
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
fn ring_clone(xml: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let items = doc.descendants().find(|n| n.has_tag_name("Items")).unwrap();
    let id = items
        .children()
        .filter(|n| n.has_tag_name("Item"))
        .map(|n| n.attribute("id").unwrap().parse::<u32>().unwrap())
        .max()
        .unwrap()
        + 1;
    let ring = selected_item(&doc, "Ring 1");
    let set = items
        .children()
        .find(|n| {
            n.has_tag_name("ItemSet") && n.attribute("id") == items.attribute("activeItemSet")
        })
        .unwrap();
    let slot = set
        .children()
        .find(|n| n.has_tag_name("Slot") && n.attribute("name") == Some("Ring 1"))
        .unwrap();
    let replacement = set_attr(&xml[slot.range()], "itemId", &id.to_string());
    let cloned = set_attr(&xml[ring.range()], "id", &id.to_string());
    let mut out = xml.to_owned();
    // The original record precedes its selected slot in saved Items XML.
    assert!(ring.range().end < slot.range().start);
    out.replace_range(slot.range(), &replacement);
    out.insert_str(ring.range().end, &format!("\n{cloned}"));
    line(&out, "Ring 1", &format!("\n{PROPERTY}\n"), false)
}
fn controls(original: &str) -> Vec<(String, String, Json)> {
    let focused = focus(original, PAIN, None, 1, 1);
    let ring = ring_clone(&focused);
    let percent = |n| format!("\n{n}% increased bonuses gained from left equipped ring\n");
    let positive = line(&ring, "Boots", &percent(100), false);
    let removed = line(&positive, "Boots", &percent(100), true);
    assert_eq!(
        removed, ring,
        "removing only the percentage restores exact property-only control"
    );
    let property_removed = line(&positive, "Ring 1", &format!("\n{PROPERTY}\n"), true);
    let helmet_removed = line(&focused, "Helmet", PROPERTY, true);
    let doc = roxmltree::Document::parse(&ring).unwrap();
    let ring_id = selected_item(&doc, "Ring 1")
        .attribute("id")
        .unwrap()
        .to_owned();
    let other_id = selected_item(&doc, "Ring 2")
        .attribute("id")
        .unwrap()
        .to_owned();
    assert_ne!(ring_id, other_id);
    [
        ("ring-property-only",ring.clone(),23,0,0,true),
        ("ring-copy-100",positive,23,100,1,true),
        ("ring-copy-150",line(&ring,"Boots",&percent(150),false),23,150,1,true),
        ("ring-copy-zero",line(&ring,"Boots",&percent(0),false),23,0,0,true),
        ("remove-ring-copy-supplier",removed,23,0,0,true),
        ("remove-ring-property",property_removed,22,100,0,true),
        ("remove-helmet-property",helmet_removed,21,0,0,false),
    ].into_iter().map(|(name,xml,level,percent,copies,cloned)| (name.into(),xml,json!({"expected_offering_level":level,"expected_ring_percent":percent,"expected_ring_property_copies":copies,
        "ring_id":if cloned {Some(&ring_id)}else{None},"unchanged_ring2_id":if cloned {Some(&other_id)}else{None},"ring_clone":cloned,
        "property_only_xml_sha256":digest(ring.as_bytes()),"roll_legality_authority":false,"native_owner_closure":false}))).collect()
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
fn query<'a>(env: &'a Json, slot: &str) -> &'a Json {
    let v: Vec<_> = rows(&env["queries"])
        .iter()
        .filter(|q| q["slot"] == slot)
        .collect();
    assert_eq!(v.len(), 1);
    v[0]
}
fn physical_inputs(state: &Json) -> Json {
    json!(
        rows(&state["environments"])
            .iter()
            .map(|env| {
                json!({
                    "mode": env["mode"],
                    "skills": rows(&env["skills"])
                        .iter()
                        .map(|skill| json!({
                            "effect": skill["effect"],
                            "source": skill["source"],
                            "raw": skill["raw"],
                            "final": skill["final"],
                            "final_lookup": skill["final_lookup"],
                            "stat_set": skill["stat_set"],
                            "selected_main": skill["selected_main"],
                            "exact_physical_source": skill["exact_physical_source"],
                            "root_lookup_present": skill["root_lookup_present"],
                        }))
                        .collect::<Vec<_>>(),
                })
            })
            .collect::<Vec<_>>()
    )
}
fn check(report: &Json) {
    assert_eq!(rows(&report["cases"]).len(), 19);
    for case in rows(&report["cases"]) {
        assert_eq!(case["independent_source_bindings_verified"], true);
        for stage in STAGES {
            let state = &case["states"][stage];
            for flag in ["original_methods_preserved", "hook_removed"] {
                assert_eq!(state[flag], true);
            }
            for flag in [
                "business_wrappers",
                "source_tables_mutated",
                "diagnostic_requery",
                "roll_legality_authority",
                "native_owner_closure",
            ] {
                assert_eq!(state[flag], false);
            }
            assert_eq!(rows(&state["environments"]).len(), 2);
            for env in rows(&state["environments"]) {
                assert_eq!(env["exact_selected_environment"], true);
                for skill in rows(&env["skills"]) {
                    assert_eq!(skill["exact_physical_source"], true);
                    assert_eq!(skill["root_lookup_present"], true);
                    if case["instrumented"] == false {
                        continue;
                    }
                    for key in ["prepared", "after_perform"] {
                        same(
                            &skill[key],
                            &skill["final"],
                            "same physical input before and after perform",
                        );
                    }
                    for key in ["prepared_lookup", "after_lookup"] {
                        same(
                            &skill[key],
                            &skill["final_lookup"],
                            "prepared level row remains exact",
                        );
                    }
                    assert_eq!(skill["assembly"]["original_call"], true);
                    assert_eq!(skill["assembly"]["before_perform"], true);
                    assert_eq!(rows(&skill["ordinary"]).len(), 1);
                    for ordinary in rows(&skill["ordinary"]) {
                        for key in [
                            "original_query",
                            "exact_actor_store",
                            "before_perform",
                            "no_late_copy_object_consumed",
                        ] {
                            assert_eq!(ordinary[key], true);
                        }
                    }
                }
                if case["instrumented"] == false {
                    continue;
                }
                for q in rows(&env["queries"]) {
                    assert_eq!(q["original_return"], true);
                    assert_eq!(q["item"]["exact_registered"], true);
                }
                for copy in rows(&env["copies"]) {
                    for key in [
                        "original_query",
                        "exact_source_list",
                        "exact_copy_argument",
                        "exact_player_destination",
                        "return_observed",
                        "after_preparation",
                    ] {
                        assert_eq!(copy[key], true);
                    }
                    assert_eq!(copy["source_record"]["name"], "GemProperty");
                    assert_eq!(copy["source_record"]["type"], "LIST");
                    assert_eq!(copy["item"]["exact_registered"], true);
                    assert_eq!(rows(&copy["insertions"]).len(), 1);
                    assert_eq!(copy["insertions"][0]["exact_inserted_object"], true);
                }
                if case["name"] == "original-05"
                    || case["name"] == "repeat-original-05"
                    || !case["control"].is_null()
                {
                    assert_eq!(query(env, "Helmet")["result"], 0);
                    assert!(rows(&env["copies"]).iter().all(|c| c["slot"] != "Helmet"));
                    let pain: Vec<_> = rows(&env["skills"])
                        .iter()
                        .filter(|s| s["effect"] == PAIN)
                        .collect();
                    assert_eq!(pain.len(), 1);
                    let level = case["control"]["expected_offering_level"]
                        .as_i64()
                        .unwrap_or(22);
                    assert_eq!(pain[0]["final"]["level"].as_f64(), Some(level as f64));
                    assert_eq!(pain[0]["final"]["quality"].as_f64(), Some(0.0));
                    let ordinary = &pain[0]["ordinary"][0];
                    let helmet: Vec<_> = rows(&ordinary["candidates"])
                        .iter()
                        .filter(|c| c["record"]["sourceSlot"] == "Helmet")
                        .collect();
                    let removed = case["name"] == "remove-helmet-property";
                    assert_eq!(helmet.len(), usize::from(!removed));
                    if let Some(helmet) = helmet.first() {
                        assert_eq!(
                            helmet["record"]["value"],
                            json!({"keyword":"minion","key":"level","value":1,"keyOfScaledMod":"value"})
                        );
                        let prefix = format!("Item:{}:", query(env, "Helmet")["item"]["id"]);
                        assert!(
                            helmet["record"]["source"]
                                .as_str()
                                .unwrap()
                                .starts_with(&prefix)
                        );
                        assert!(rows(&ordinary["matched"]).contains(&helmet["index"]));
                    }
                    if !case["control"].is_null() {
                        let control = &case["control"];
                        assert_eq!(
                            query(env, "Ring 1")["result"],
                            control["expected_ring_percent"]
                        );
                        let copies: Vec<_> = rows(&env["copies"])
                            .iter()
                            .filter(|c| c["slot"] == "Ring 1")
                            .collect();
                        assert_eq!(
                            copies.len() as u64,
                            control["expected_ring_property_copies"].as_u64().unwrap()
                        );
                        assert_eq!(
                            rows(&env["copies"]).len(),
                            copies.len(),
                            "no additional late property copies in this control"
                        );
                        if control["ring_clone"] == true {
                            assert_eq!(
                                query(env, "Ring 1")["item"]["id"].as_u64(),
                                Some(control["ring_id"].as_str().unwrap().parse::<u64>().unwrap())
                            );
                            assert_eq!(
                                query(env, "Ring 2")["item"]["id"].as_u64(),
                                Some(
                                    control["unchanged_ring2_id"]
                                        .as_str()
                                        .unwrap()
                                        .parse::<u64>()
                                        .unwrap()
                                )
                            );
                        }
                        for copy in copies {
                            let source = &copy["source_record"];
                            let inserted = &copy["insertions"][0]["record"];
                            assert_eq!(
                                source["value"],
                                json!({"keyword":"minion","key":"level","value":1,"keyOfScaledMod":"value"})
                            );
                            assert_eq!(source["flags"], 0);
                            assert_eq!(source["keywordFlags"], 0);
                            assert!(source.get("positions").is_none());
                            assert_eq!(inserted["value"], source["value"]);
                            assert_eq!(inserted["flags"], 0);
                            assert_eq!(inserted["keywordFlags"], 0);
                            assert!(inserted.get("positions").is_none());
                            assert!(
                                inserted["source"]
                                    .as_str()
                                    .unwrap()
                                    .starts_with("Many Sources:")
                            );
                            assert_eq!(
                                copy["factor"].as_f64(),
                                Some(control["expected_ring_percent"].as_f64().unwrap() / 100.0)
                            );
                        }
                        assert_eq!(pain[0]["selected_main"], true);
                        let display = rows(&env["display"]);
                        assert_eq!(display.len(), 4);
                        let expected = [
                            ("GemLevel", 20),
                            ("GemItemLevel", level - 20),
                            ("GemSupportLevel", 0),
                            ("GemCorruptionLevel", 0),
                        ];
                        for (actual, (name, value)) in display.iter().zip(expected) {
                            assert_eq!(actual["name"], name);
                            assert_eq!(actual["result"].as_f64(), Some(value as f64));
                            assert_eq!(actual["original_return"], true);
                        }
                    }
                }
            }
        }
    }
    for i in 1..=5 {
        let a = named(report, &format!("original-{i:02}"));
        let b = named(report, &format!("repeat-original-{i:02}"));
        same(&a["states"], &b["states"], "independent unchanged replay");
    }
    for name in ["original-05", "ring-copy-100"] {
        let a = named(report, name);
        let b = named(report, &format!("unhooked-{name}"));
        for stage in STAGES {
            same(
                &a["states"][stage]["outputs"],
                &b["states"][stage]["outputs"],
                "unhooked output equality",
            );
            same(
                &physical_inputs(&a["states"][stage]),
                &physical_inputs(&b["states"][stage]),
                "unhooked physical input and lookup equality",
            );
        }
    }
    let a = named(report, "ring-property-only");
    let b = named(report, "remove-ring-copy-supplier");
    assert_eq!(a["xml_sha256"], b["xml_sha256"]);
    same(&a["states"], &b["states"], "exact supplier removal replay");
}
