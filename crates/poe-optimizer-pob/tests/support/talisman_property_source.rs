//! Original Amulet diversion and independent early copy; finite source evidence.
use super::magnified_area_support::set_attr;
use super::*;

const TEST_NAME: &str =
    "talisman_property::talisman_routes_ordinary_properties_independently_of_early_copies";
const OBSERVER: &str = include_str!("talisman_property_source.lua");
const COPY_LINE: &str = "\n100% increased bonuses gained from equipped rings and amulets\n";

#[test]
#[ignore = "requires pinned PoB; original Talisman routing and early copy calls"]
fn talisman_routes_ordinary_properties_independently_of_early_copies() {
    super::physical_support::run_modes(
        super::physical_support::Witness {
            name: TEST_NAME,
            child_env: "POE_TALISMAN_PROPERTY_SOURCE_CHILD",
            output_env: "POE_TALISMAN_PROPERTY_SOURCE_OUT",
            default_output: "runs/owned-talisman-property-source-01",
            label: "Talisman properties",
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
    for name in ["allocated-talisman", "talisman-boots-copy-100"] {
        let case = controls.iter().find(|c| c.0 == name).unwrap();
        cases.push(observe(
            root,
            &format!("unhooked-{name}"),
            &case.1,
            enabled,
            false,
            Some(case.2.clone()),
        ));
    }
    assert_eq!(cases.len(), 21);
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
        "src/Classes/PassiveTree.lua",
    ]);
    files.sort_unstable();
    files.dedup();
    let report = json!({"schema_version":1,"source_revision":pinned::UPSTREAM_REVISION,"manifest_sha256":pinned::manifest_sha256(),
        "observer_sha256":digest(OBSERVER.as_bytes()),"lifecycle_sha256":digest(LIFECYCLE.as_bytes()),
        "files":files.into_iter().map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})).collect::<Vec<_>>(),
        "original_sources":originals.iter().enumerate().map(|(i,b)|json!({"path":format!("tests/fixtures/builds/breadth-20260908/build-{:02}.xml",i+1),"sha256":digest(b)})).collect::<Vec<_>>(),
        "lifecycle_stages":STAGES,"numeric_tolerance":0,"business_wrappers":false,
        "scope":{"actual_diversion_transport":true,"independent_early_player_copy":true,"actual_player_property_consumers":true,"actual_minion_store_receipt":true,
            "connected_control_path":[61042,44344,39935],"roll_legality_authority":false,"complete_native_owner":false,"intended_gameplay_law":false,
            "minion_gem_level_consumer_claim":false,"whole_build_closure":false},"cases":cases});
    let bytes = serde_json::to_vec(&report).unwrap();
    let suffix = if enabled { "on" } else { "off" };
    let raw = out.join(format!("source-jit-{suffix}.raw.json"));
    fs::write(&raw, &bytes).unwrap();
    eprintln!(
        "Talisman evidence: {} bytes at {}",
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
        "Talisman case {name}, JIT {}",
        if enabled { "on" } else { "off" }
    );
    let before = |lua: &Lua| -> Result<(), RuntimeError> {
        lua.globals().set("talismanInstrumented", instrumented)?;
        lua.globals().set("talismanJit", enabled)?;
        lua.globals().set("talismanXml", xml)?;
        lua.load("if talismanJit then jit.on() else jit.off();jit.flush() end")
            .exec()?;
        Ok(())
    };
    let before_build = |lua: &Lua| -> Result<Function, RuntimeError> {
        lua.globals().set("djinnPhase", "before")?;
        let lifecycle: Function = lua
            .load(LIFECYCLE)
            .set_name("@talisman-original-lifecycle")
            .eval()?;
        let api: mlua::Table = lua
            .load(OBSERVER)
            .set_name("@talisman-property-observer")
            .eval()?;
        let install: Function = api.get("install")?;
        let cleanup: Function = install.call(())?;
        lua.globals().set("talismanApi", api)?;
        let combine:Function=lua.load("return function(observer,auth) return function() local ok,err=pcall(observer);auth();if not ok then error(err,0)end end end").eval()?;
        Ok(combine.call((cleanup, lifecycle))?)
    };
    let observer = |lua: &Lua| -> Result<Json, RuntimeError> {
        let api: mlua::Table = lua.globals().get("talismanApi")?;
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
        BuildLineage::from_bytes([118; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    let states = &observed["additional_observation"];
    for stage in STAGES {
        for env in rows(&states[stage]["environments"]) {
            for binding in rows(&env["offerings"]).iter().map(|s| &s["source"]).chain(
                rows(&env["receipts"])
                    .iter()
                    .filter(|r| r["source_present"] == true)
                    .map(|r| &r["source"]),
            ) {
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

fn allocate(xml: &str, talisman: bool) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let tree = doc.descendants().find(|n| n.has_tag_name("Tree")).unwrap();
    let active: usize = tree.attribute("activeSpec").unwrap().parse().unwrap();
    let spec = tree
        .children()
        .filter(|n| n.has_tag_name("Spec"))
        .nth(active - 1)
        .unwrap();
    assert_eq!(spec.attribute("treeVersion"), Some("0_5"));
    let nodes = spec.attribute("nodes").unwrap();
    assert!(nodes.split(',').any(|n| n == "61042"));
    assert!(!nodes.split(',').any(|n| n == "44344" || n == "39935"));
    let suffix = if talisman { ",44344,39935" } else { ",44344" };
    let replacement = set_attr(&xml[spec.range()], "nodes", &format!("{nodes}{suffix}"));
    let mut out = xml.to_owned();
    out.replace_range(spec.range(), &replacement);
    out
}
fn item_control(xml: &str, slot: &str, remove_item: bool) -> String {
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
    let mut out = xml.to_owned();
    if remove_item {
        out.replace_range(
            selected.range(),
            &set_attr(&xml[selected.range()], "itemId", "0"),
        );
    } else {
        let item = items
            .children()
            .find(|n| n.has_tag_name("Item") && n.attribute("id") == selected.attribute("itemId"))
            .unwrap();
        let text = item.children().find(|n| n.is_text()).unwrap();
        let raw = &xml[text.range()];
        out.replace_range(text.range(), &format!("{raw}{COPY_LINE}"));
    }
    out
}
fn controls(original: &str) -> Vec<(String, String, Json)> {
    let baseline = allocate(original, false);
    let allocated = allocate(original, true);
    assert_eq!(allocated.matches(",39935\"").count(), 1);
    assert_eq!(
        allocated.replacen(",39935\"", "\"", 1),
        baseline,
        "notable removal restores connected baseline"
    );
    let boots = item_control(&allocated, "Boots", false);
    assert_eq!(boots.matches(COPY_LINE).count(), 1);
    let removed = boots.replacen(COPY_LINE, "", 1);
    assert_eq!(
        removed, allocated,
        "supplier removal restores exact allocated control"
    );
    [
        ("connected-baseline",baseline.clone(),false,false,0,22),
        ("allocated-talisman",allocated.clone(),true,false,0,21),
        ("remove-talisman",baseline.clone(),false,false,0,22),
        ("baseline-boots-copy-100",item_control(&baseline,"Boots",false),false,false,100,23),
        ("talisman-boots-copy-100",boots,true,false,100,22),
        ("remove-copy-supplier",removed,true,false,0,21),
        ("talisman-amulet-copy-100",item_control(&allocated,"Amulet",false),true,false,0,21),
        ("talisman-absent-amulet",item_control(&allocated,"Amulet",true),true,true,0,21),
    ].into_iter().map(|(name,xml,talisman,absent,percent,level)|(name.into(),xml,json!({"talisman_allocated":talisman,"absent_amulet":absent,
        "expected_pre_copy_percent":percent,"expected_offering_level":level,"connected_baseline_xml_sha256":digest(baseline.as_bytes()),
        "allocated_baseline_xml_sha256":digest(allocated.as_bytes()),"roll_legality_authority":false,"native_owner_closure":false}))).collect()
}
fn same(a: &Json, b: &Json, label: &str) {
    assert_eq!(json_evidence::first_difference(a, b, label), None);
}
fn named<'a>(report: &'a Json, name: &str) -> &'a Json {
    let r: Vec<_> = rows(&report["cases"])
        .iter()
        .filter(|r| r["name"] == name)
        .collect();
    assert_eq!(r.len(), 1);
    r[0]
}
fn property_rows(rows_value: &Json) -> Vec<&Json> {
    rows(rows_value)
        .iter()
        .filter(|r| r["record"]["name"] == "GemProperty")
        .collect()
}
fn check_transport(transport: &Json, line: u64) {
    for row in rows(transport) {
        assert_eq!(row["caller_line"].as_u64(), Some(line));
        for flag in [
            "exact_source_object",
            "exact_destination",
            "actual_item_return",
            "return_observed",
        ] {
            assert_eq!(row[flag], true);
        }
        assert_eq!(rows(&row["insertions"]).len(), 1);
        assert_eq!(row["insertions"][0]["exact_inserted_object"], true);
    }
}
fn check(report: &Json) {
    assert_eq!(rows(&report["cases"]).len(), 21);
    for case in rows(&report["cases"]) {
        for stage in STAGES {
            let state = &case["states"][stage];
            assert_eq!(rows(&state["environments"]).len(), 2);
            for flag in ["original_methods_preserved", "hook_removed"] {
                assert_eq!(state[flag], true);
            }
            for flag in [
                "business_wrappers",
                "source_tables_mutated",
                "diagnostic_requery",
                "roll_legality_authority",
                "native_owner_closure",
                "minion_gem_level_consumer_claim",
            ] {
                assert_eq!(state[flag], false);
            }
            for env in rows(&state["environments"]) {
                assert_eq!(env["exact_selected_environment"], true);
                for offering in rows(&env["offerings"]) {
                    assert_eq!(offering["exact_physical_source"], true);
                    if case["instrumented"] == false {
                        continue;
                    }
                    same(
                        &offering["assembly"]["after"],
                        &offering["final"],
                        "prepared Offering retained after perform",
                    );
                    same(
                        &offering["assembly"]["lookup"],
                        &offering["lookup"],
                        "prepared lookup retained",
                    );
                    assert_eq!(rows(&offering["ordinary"]).len(), 1);
                    for ordinary in rows(&offering["ordinary"]) {
                        for flag in [
                            "original_query",
                            "exact_actor_store",
                            "no_diverted_object_consumed",
                            "before_perform",
                        ] {
                            assert_eq!(ordinary[flag], true);
                        }
                    }
                }
                if case["instrumented"] == false {
                    continue;
                }
                check_transport(&env["diverted"], 1402);
                check_transport(&env["copies"], 1667);
                for receipt in rows(&env["receipts"]) {
                    for flag in [
                        "exact_talisman_source",
                        "exact_minion_destination",
                        "original_return",
                        "skill_actor_is_player",
                        "receiver_is_skill_minion",
                        "selected_minion_identity",
                    ] {
                        assert_eq!(receipt[flag], true);
                    }
                    same(
                        &receipt["records"],
                        &env["talisman_records"],
                        "exact delivered Talisman records",
                    );
                    assert_eq!(
                        rows(&receipt["joins"]).len(),
                        rows(&receipt["records"]).len()
                    );
                    for (i, join) in rows(&receipt["joins"]).iter().enumerate() {
                        assert_eq!(join["source_index"].as_u64(), Some(i as u64 + 1));
                        assert!(!rows(&join["destination_indices"]).is_empty());
                    }
                }
                let control = &case["control"];
                if case["name"] == "original-05" || case["name"] == "repeat-original-05" {
                    assert_eq!(env["path"][2]["allocated"], false);
                    assert_eq!(env["query"]["result"], 0);
                    assert!(rows(&env["diverted"]).is_empty());
                    assert!(rows(&env["receipts"]).is_empty());
                    assert_eq!(rows(&env["offerings"]).len(), 1);
                    assert_eq!(env["offerings"][0]["raw"]["level"], 20);
                    assert_eq!(env["offerings"][0]["final"]["level"], 22);
                    assert_eq!(env["offerings"][0]["final"]["quality"], 0);
                }
                if !control.is_null() {
                    assert_eq!(env["class"]["id"], 7);
                    assert_eq!(env["class"]["ascendancy_id"], 3);
                    assert_eq!(rows(&env["path"]).len(), 3);
                    for (i, id) in [61042, 44344, 39935].into_iter().enumerate() {
                        let node = &env["path"][i];
                        assert_eq!(node["id"], id);
                        assert_eq!(node["exact_allocated_object"], true);
                        assert_eq!(
                            node["allocated"],
                            if i < 2 {
                                json!(true)
                            } else {
                                control["talisman_allocated"].clone()
                            }
                        );
                    }
                    assert_eq!(env["path"][2]["name"], "Necromantic Talisman");
                    for (from, to) in [(0, 44344), (1, 61042), (1, 39935), (2, 44344)] {
                        assert!(
                            rows(&env["path"][from]["connections"]["positions"])
                                .iter()
                                .any(|p| p["value"] == to)
                        );
                    }
                    let absent = control["absent_amulet"] == true;
                    let diverted = control["talisman_allocated"] == true && !absent;
                    if absent {
                        assert_eq!(env["amulet"]["absent"], true);
                        assert!(env["query"].is_null());
                        assert!(rows(&env["copies"]).is_empty());
                    } else {
                        assert_eq!(env["amulet"]["type"], "Amulet");
                        assert_eq!(env["amulet"]["exact_registered"], true);
                        assert_eq!(env["query"]["result"], control["expected_pre_copy_percent"]);
                        assert_eq!(env["query"]["original_call"], true);
                        assert_eq!(env["query"]["return_observed"], true);
                        assert_eq!(env["query"]["exact_player_store"], true);
                    }
                    assert_eq!(env["final_amulet"]["absent"] == true, diverted || absent);
                    let talents = property_rows(&env["talisman_records"]);
                    assert_eq!(talents.len(), usize::from(diverted));
                    if diverted {
                        assert_eq!(
                            talents[0]["record"]["value"],
                            json!({"keyword":"minion","key":"level","value":1,"keyOfScaledMod":"value"})
                        );
                        assert_eq!(
                            rows(&env["diverted"]).len(),
                            rows(&env["talisman_records"]).len()
                        );
                        let selected: Vec<_> = rows(&env["receipts"])
                            .iter()
                            .filter(|r| r["selected"] == true)
                            .collect();
                        assert_eq!(selected.len(), 1);
                        assert_eq!(selected[0]["source_present"], true);
                        for receipt in rows(&env["receipts"]) {
                            assert_eq!(property_rows(&receipt["records"]).len(), 1);
                        }
                    } else {
                        assert!(rows(&env["diverted"]).is_empty());
                        assert!(rows(&env["receipts"]).is_empty());
                    }
                    assert_eq!(rows(&env["offerings"]).len(), 1);
                    let offering = &env["offerings"][0];
                    assert_eq!(
                        offering["final"]["level"],
                        control["expected_offering_level"]
                    );
                    assert_eq!(offering["final"]["quality"], 0);
                    let ordinary = &offering["ordinary"][0];
                    let candidates = rows(&ordinary["candidates"]);
                    let originals: Vec<_> = candidates
                        .iter()
                        .filter(|c| {
                            c["record"]["sourceSlot"] == "Amulet"
                                && c["record"]["source"].as_str().unwrap().starts_with("Item:")
                        })
                        .collect();
                    assert_eq!(originals.len(), usize::from(!diverted && !absent));
                    let copies: Vec<_> = rows(&env["copies"])
                        .iter()
                        .enumerate()
                        .filter(|(_, c)| c["source_record"]["name"] == "GemProperty")
                        .collect();
                    assert_eq!(copies.len(), usize::from(!absent));
                    for (index, copy) in copies {
                        let value = control["expected_pre_copy_percent"].as_i64().unwrap() / 100;
                        assert_eq!(copy["source_record"]["value"]["value"], 1);
                        assert_eq!(copy["insertions"][0]["record"]["value"]["value"], value);
                        let joined: Vec<_> = rows(&ordinary["joins"])
                            .iter()
                            .filter(|j| rows(&j["early_copy_indices"]).contains(&json!(index + 1)))
                            .collect();
                        assert_eq!(joined.len(), 1);
                        assert!(rows(&ordinary["matched"]).contains(&joined[0]["candidate_index"]));
                    }
                }
            }
        }
    }
    for i in 1..=5 {
        same(
            &named(report, &format!("original-{i:02}"))["states"],
            &named(report, &format!("repeat-original-{i:02}"))["states"],
            "independent unchanged replay",
        );
    }
    for (a, b) in [
        ("connected-baseline", "remove-talisman"),
        ("allocated-talisman", "remove-copy-supplier"),
    ] {
        let a = named(report, a);
        let b = named(report, b);
        assert_eq!(a["xml_sha256"], b["xml_sha256"]);
        same(&a["states"], &b["states"], "exact removal inverse replay");
    }
    for name in [
        "original-05",
        "allocated-talisman",
        "talisman-boots-copy-100",
    ] {
        let a = named(report, name);
        let b = named(report, &format!("unhooked-{name}"));
        for stage in STAGES {
            same(
                &a["states"][stage]["outputs"],
                &b["states"][stage]["outputs"],
                "unhooked output equality",
            );
            for (a, b) in rows(&a["states"][stage]["environments"])
                .iter()
                .zip(rows(&b["states"][stage]["environments"]))
            {
                assert_eq!(rows(&a["offerings"]).len(), rows(&b["offerings"]).len());
                for (a, b) in rows(&a["offerings"]).iter().zip(rows(&b["offerings"])) {
                    for key in ["source", "raw", "final", "lookup"] {
                        same(&a[key], &b[key], "unhooked physical input equality");
                    }
                }
            }
        }
    }
}

// Synthetic-input diagnostic: PoB routes by a presentation-title substring even
// on a manually renamed rare Amulet. This does not establish an obtainable item,
// corruption outcome, valid-game defect, or native/gameplay authority.
const TITLE_TEST_NAME: &str =
    "talisman_property::rare_item_title_changes_reference_routing_without_game_identity";

#[test]
#[ignore = "requires pinned PoB; diagnostic for presentation-name routing"]
fn rare_item_title_changes_reference_routing_without_game_identity() {
    super::physical_support::run_modes(
        super::physical_support::Witness {
            name: TITLE_TEST_NAME,
            child_env: "POE_RARE_TITLE_ROUTING_SOURCE_CHILD",
            output_env: "POE_RARE_TITLE_ROUTING_SOURCE_OUT",
            default_output: "runs/owned-rare-title-routing-source-01",
            label: "Rare title routing diagnostic",
        },
        rare_title_child,
    );
}

fn rename_solar_title(xml: &str) -> String {
    let original = "Rarity: RARE\nNew Item\nSolar Amulet";
    let replacement = "Rarity: RARE\nKalandra's Touch\nSolar Amulet";
    assert_eq!(xml.matches(original).count(), 1);
    let renamed = xml.replacen(original, replacement, 1);
    assert_eq!(renamed.replacen(replacement, original, 1), xml);
    let doc = roxmltree::Document::parse(&renamed).unwrap();
    let item = doc
        .descendants()
        .find(|node| node.has_tag_name("Item") && node.attribute("id") == Some("23"))
        .unwrap();
    assert!(item.text().unwrap().contains(replacement));
    renamed
}

fn rare_title_child(root: &Path, out: &Path, enabled: bool) {
    let relative = "tests/fixtures/builds/breadth-20260908/build-05.xml";
    let bytes = fs::read(root.join(relative)).unwrap();
    let index: Json = serde_json::from_slice(
        &fs::read(root.join("tests/fixtures/builds/breadth-20260908/index.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(digest(&bytes), index["builds"][4]["xml_sha256"]);
    let original = std::str::from_utf8(&bytes).unwrap();
    let renamed = rename_solar_title(original);
    let controls = [
        ("original-05", original.to_owned(), false, 0, 22),
        ("renamed-amulet", renamed.clone(), true, 0, 21),
        (
            "original-boots-copy-100",
            item_control(original, "Boots", false),
            false,
            100,
            23,
        ),
        (
            "renamed-boots-copy-100",
            item_control(&renamed, "Boots", false),
            true,
            100,
            22,
        ),
    ];
    let mut cases = Vec::new();
    for (prefix, instrumented) in [("", true), ("repeat-", true), ("unhooked-", false)] {
        for (name, xml, renamed, percent, level) in &controls {
            cases.push(observe(
                root,
                &format!("{prefix}{name}"),
                xml,
                enabled,
                instrumented,
                Some(json!({"renamed_rare_amulet":renamed,"copy_percent":percent,
                    "offering_level":level,"game_identity_changed":false})),
            ));
        }
    }
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
        "src/Classes/PassiveTree.lua",
    ]);
    files.sort_unstable();
    files.dedup();
    let report = json!({"schema_version":1,"source_revision":pinned::UPSTREAM_REVISION,
        "manifest_sha256":pinned::manifest_sha256(),"observer_sha256":digest(OBSERVER.as_bytes()),
        "lifecycle_sha256":digest(LIFECYCLE.as_bytes()),"lifecycle_stages":STAGES,
        "files":files.into_iter().map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})).collect::<Vec<_>>(),
        "original_source":{"path":relative,"sha256":digest(&bytes)},"numeric_tolerance":0,
        "scope":{"synthetic_name_routing_control":true,"obtainable_item_claimed":false,
            "corruption_outcome_claimed":false,
            "game_identity_changed":false,
            "native_owner_closure":false,"intended_gameplay_law":false,
            "ordinary_amplitude_and_independent_copy":true,"whole_build_parity":false},"cases":cases});
    let suffix = if enabled { "on" } else { "off" };
    let encoded = serde_json::to_vec(&report).unwrap();
    fs::write(out.join(format!("source-jit-{suffix}.raw.json")), &encoded).unwrap();
    assert!(encoded.len() <= 32 * 1024 * 1024);
    check_rare_title(&report);
    fs::write(out.join(format!("source-jit-{suffix}.json")), encoded).unwrap();
    assert_eq!(fs::read(root.join(relative)).unwrap(), bytes);
}

fn check_rare_title(report: &Json) {
    assert_eq!(rows(&report["cases"]).len(), 12);
    for case in rows(&report["cases"]) {
        let renamed = case["control"]["renamed_rare_amulet"].as_bool().unwrap();
        let percent = case["control"]["copy_percent"].as_i64().unwrap();
        let source = if renamed {
            "Item:23:Kalandra's Touch, Solar Amulet"
        } else {
            "Item:23:New Item, Solar Amulet"
        };
        for stage in STAGES {
            let state = &case["states"][stage];
            assert_eq!(rows(&state["environments"]).len(), 2);
            for flag in ["original_methods_preserved", "hook_removed"] {
                assert_eq!(state[flag], true);
            }
            for flag in [
                "business_wrappers",
                "source_tables_mutated",
                "diagnostic_requery",
                "native_owner_closure",
                "roll_legality_authority",
            ] {
                assert_eq!(state[flag], false);
            }
            for env in rows(&state["environments"]) {
                assert_eq!(env["exact_selected_environment"], true);
                assert_eq!(rows(&env["offerings"]).len(), 1);
                let offering = &env["offerings"][0];
                assert_eq!(offering["exact_physical_source"], true);
                assert_eq!(offering["raw"]["level"], 20);
                assert_eq!(
                    offering["final"]["level"],
                    case["control"]["offering_level"]
                );
                assert_eq!(offering["final"]["quality"], 0);
                if case["instrumented"] == false {
                    continue;
                }
                assert_eq!(env["amulet"]["id"], 23);
                assert_eq!(env["amulet"]["type"], "Amulet");
                assert_eq!(env["amulet"]["source"], source);
                assert_eq!(env["amulet"]["exact_registered"], true);
                same(
                    &env["amulet"],
                    &env["final_amulet"],
                    "Amulet remains equipped",
                );
                assert_eq!(env["path"][2]["allocated"], false);
                assert!(rows(&env["diverted"]).is_empty());
                assert!(rows(&env["receipts"]).is_empty());
                assert_eq!(env["query"]["result"], percent);
                for flag in ["original_call", "return_observed", "exact_player_store"] {
                    assert_eq!(env["query"][flag], true);
                }
                check_transport(&env["copies"], 1667);
                let copies: Vec<_> = rows(&env["copies"])
                    .iter()
                    .enumerate()
                    .filter(|(_, row)| row["source_record"]["name"] == "GemProperty")
                    .collect();
                assert_eq!(copies.len(), 1);
                let (copy_index, copy) = copies[0];
                assert_eq!(copy["source_record"]["source"], source);
                assert_eq!(copy["source_record"]["value"]["value"], 1);
                assert_eq!(
                    copy["insertions"][0]["record"]["value"]["value"],
                    percent / 100
                );
                assert_eq!(rows(&offering["ordinary"]).len(), 1);
                let ordinary = &offering["ordinary"][0];
                for flag in ["original_query", "exact_actor_store", "before_perform"] {
                    assert_eq!(ordinary[flag], true);
                }
                same(
                    &ordinary["after"],
                    &offering["final"],
                    "ordinary consumer final input",
                );
                same(
                    &offering["assembly"]["after"],
                    &offering["final"],
                    "assembled final input",
                );
                let candidates = rows(&ordinary["candidates"]);
                let direct = candidates
                    .iter()
                    .filter(|row| row["record"]["source"] == source)
                    .count();
                assert_eq!(direct, usize::from(!renamed));
                let crown: Vec<_> = candidates
                    .iter()
                    .filter(|row| row["record"]["source"] == "Item:21:New Item, Iron Crown")
                    .collect();
                assert_eq!(crown.len(), 1);
                assert_eq!(crown[0]["value"]["value"], 1);
                assert!(rows(&ordinary["matched"]).contains(&crown[0]["index"]));
                let joins: Vec<_> = rows(&ordinary["joins"])
                    .iter()
                    .filter(|row| rows(&row["early_copy_indices"]).contains(&json!(copy_index + 1)))
                    .collect();
                assert_eq!(joins.len(), 1);
                assert!(rows(&ordinary["matched"]).contains(&joins[0]["candidate_index"]));
            }
        }
    }
    for name in [
        "original-05",
        "renamed-amulet",
        "original-boots-copy-100",
        "renamed-boots-copy-100",
    ] {
        let original = named(report, name);
        let repeat = named(report, &format!("repeat-{name}"));
        let unhooked = named(report, &format!("unhooked-{name}"));
        same(
            &original["states"],
            &repeat["states"],
            "independent title-control replay",
        );
        for stage in STAGES {
            same(
                &original["states"][stage]["outputs"],
                &unhooked["states"][stage]["outputs"],
                "unhooked title-control outputs",
            );
            for (a, b) in rows(&original["states"][stage]["environments"])
                .iter()
                .zip(rows(&unhooked["states"][stage]["environments"]))
            {
                for field in ["source", "raw", "final", "lookup"] {
                    same(
                        &a["offerings"][0][field],
                        &b["offerings"][0][field],
                        "unhooked title-control physical inputs",
                    );
                }
            }
        }
    }
}
