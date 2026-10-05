//! Optional finite consumer-time evidence for unchanged Original05. No Import
//! disposition, gameplay producer, broad parser proof or native closure is added.
use super::*;
use std::collections::{BTreeMap, BTreeSet};
#[path = "json_evidence.rs"]
mod json_evidence;

const TEST: &str =
    "generated_extra_stat_consumption::unchanged_original_extra_stats_reach_the_actual_consumer";
const CHILD: &str = "POE_EXTRA_STAT_CONSUMPTION_CHILD";
const OUTPUT: &str = "POE_EXTRA_STAT_CONSUMPTION_OUT";
const OBSERVER: &str = include_str!("generated_extra_stat_consumption.lua");
const EFFECTS: [&str; 5] = [
    "SummonSandDjinnPlayer",
    "CommandSandDjinnKnifeThrowPlayer",
    "SummonWaterDjinnPlayer",
    "CommandWaterDjinnBubblePlayer",
    "FireboltPlayer",
];

#[test]
#[ignore = "requires original pinned PoB; finite unchanged-source evidence only"]
fn unchanged_original_extra_stats_reach_the_actual_consumer() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = std::env::var_os(OUTPUT)
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("runs/owned-extra-stat-consumption-source-01"));
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        run(&root, &out, mode == "on");
        return;
    }
    assert!(
        !out.exists(),
        "fresh evidence directory required: {}",
        out.display()
    );
    fs::create_dir_all(&out).unwrap();
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--ignored", "--nocapture"])
            .env(CHILD, mode)
            .env(OUTPUT, &out)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let started = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(
                    status.success(),
                    "consumer source child failed: {}\n{}",
                    path.display(),
                    tail(&path)
                );
                break;
            }
            if started.elapsed() > Duration::from_secs(240) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!(
                    "consumer source child deadline: {}\n{}",
                    path.display(),
                    tail(&path)
                );
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    json_evidence::assert_files_equal(
        &out.join("source-jit-off.json"),
        &out.join("source-jit-on.json"),
        "exact consumer evidence across JIT modes",
    );
}

fn run(root: &Path, out: &Path, enabled: bool) {
    let input = root.join("tests/fixtures/builds/breadth-20260908/build-05.xml");
    let xml = fs::read_to_string(&input).unwrap();
    let index: Json =
        serde_json::from_slice(&fs::read(input.parent().unwrap().join("index.json")).unwrap())
            .unwrap();
    assert_eq!(digest(xml.as_bytes()), index["builds"][4]["xml_sha256"]);
    let frame = source_frame(&xml);
    let mut cases = Vec::new();
    for (name, instrumented) in [
        ("original-05", true),
        ("repeat-original-05", true),
        ("uninstrumented-original-05", false),
    ] {
        eprintln!(
            "Extra-stat consumer case {name}, JIT {}",
            if enabled { "on" } else { "off" }
        );
        cases.push(observe(root, name, &xml, enabled, instrumented));
    }
    let files = [
        "src/Modules/CalcActiveSkill.lua",
        "src/Modules/CalcSetup.lua",
        "src/Modules/CalcPerform.lua",
        "src/Classes/ModStore.lua",
        "src/Classes/ModList.lua",
        "src/Classes/ModDB.lua",
        "src/Classes/Item.lua",
        "src/Classes/PartyTab.lua",
        "src/Classes/ConfigTab.lua",
        "src/Classes/SkillsTab.lua",
        "src/Classes/PassiveSpec.lua",
        "src/Modules/Data.lua",
        "src/Data/SkillStatMap.lua",
        "src/Modules/ModTools.lua",
        "src/Modules/ModParser.lua",
        "src/Data/ModCache.lua",
        "src/Modules/Main.lua",
        "src/HeadlessWrapper.lua",
        "src/Modules/Build.lua",
    ];
    let report = json!({"schema_version":2,"source_revision":pinned::UPSTREAM_REVISION,
        "evidence_view":"raw_source_observation",
        "deterministic_comparison":{"view":"bidding_distinct_channel_projection_v1",
            "scope":"exact unchanged manual Djinn Bidding II pair at local sequence positions 1 and 2",
            "source_iteration":"CalcActiveSkill.lua:87 pairs(stats)",
            "recipient_transfer":"CalcPerform.lua:1161-1166; ModDB.lua:31-37 separate named channels",
            "same_channel_order":"preserved","all_other_records_and_numerical_outputs":"exact"},
        "manifest_sha256":pinned::manifest_sha256(),"observer_sha256":digest(OBSERVER.as_bytes()),
        "original_path":"tests/fixtures/builds/breadth-20260908/build-05.xml","original_sha256":digest(xml.as_bytes()),
        "source_frame":frame,"native_inventory_authority":false,"native_build_parity":false,
        "field_non_applicability_certificate":false,"all_suppliers_or_transforms_proved":false,
        "dormant_presets_covered":false,"business_wrappers":false,"numeric_tolerance":0,
        "observer_requeries_extra_stats":false,"effect_ids":EFFECTS,
        "files":files.map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})),
        "cases":cases});
    let bytes = serde_json::to_vec(&report).unwrap();
    let mode = if enabled { "on" } else { "off" };
    let raw = out.join(format!("source-jit-{mode}.raw.json"));
    fs::write(&raw, &bytes).unwrap();
    eprintln!(
        "Saved raw consumer evidence: {} bytes at {}",
        bytes.len(),
        raw.display()
    );
    assert!(
        bytes.len() <= 64 * 1024 * 1024,
        "bounded diagnostic artifact"
    );
    assert_eq!(fs::read_to_string(input).unwrap(), xml);
    validate(&report, &xml);
    let semantic = semantic_report(report);
    let bytes = serde_json::to_vec(&semantic).unwrap();
    fs::write(out.join(format!("source-jit-{mode}.json")), bytes).unwrap();
    assert_json_equal(
        &semantic["cases"][0]["states"],
        &semantic["cases"][1]["states"],
        "independent fresh source replay",
    );
}

fn assert_json_equal(left: &Json, right: &Json, label: &str) {
    if let Some(difference) = json_evidence::first_difference(left, right, "$") {
        panic!("{label}: {difference}; complete raw evidence remains on disk");
    }
}

fn raw_table(fields: Vec<(Json, Json)>) -> Json {
    json!({"kind":"raw_table","has_metatable":false,"fields":fields.into_iter().map(|(key,value)|
        json!({"key_type":if key.is_number(){"number"}else{"string"},"key":key,"value":value})).collect::<Vec<_>>()})
}

fn expected_bidding_record(name: &str, kind: &str) -> Json {
    let source = json!("Skill:SupportBiddingPlayerTwo");
    let inner = raw_table(vec![
        (
            json!(1),
            raw_table(vec![
                (json!("type"), json!("Condition")),
                (json!("var"), json!("CommandableSkill")),
            ]),
        ),
        (json!("flags"), json!(0)),
        (json!("keywordFlags"), json!(0)),
        (json!("name"), json!(name)),
        (json!("source"), source.clone()),
        (json!("type"), json!(kind)),
        (json!("value"), json!(30)),
    ]);
    raw_table(vec![
        (json!("flags"), json!(0)),
        (json!("keywordFlags"), json!(0)),
        (json!("name"), json!("MinionModifier")),
        (json!("source"), source),
        (json!("type"), json!("LIST")),
        (json!("value"), raw_table(vec![(json!("mod"), inner)])),
    ])
}

fn bidding_source(row: &Json) -> bool {
    row["record"]["fields"].as_array().is_some_and(|fields| {
        fields.iter().any(|f| {
            f["key_type"] == "string"
                && f["key"] == "source"
                && f["value"] == "Skill:SupportBiddingPlayerTwo"
        })
    })
}

/// Reuse the reviewed Bidding witness's distinct-channel comparison contract.
/// This representation accepts only the exact original two-record pair. It does
/// not sort lists, move other records or normalize arithmetic within a channel.
/// Original positions and every raw field remain in the immutable raw report.
fn project_bidding_pair(local: &mut Json) -> bool {
    let records = rows(&local["records"]);
    let indices: Vec<_> = records
        .iter()
        .enumerate()
        .filter(|(_, r)| bidding_source(r))
        .map(|(i, _)| i)
        .collect();
    if indices.is_empty() {
        return false;
    }
    assert_eq!(
        indices,
        [0, 1],
        "only the reviewed adjacent Bidding pair may be projected"
    );
    let mut channels = serde_json::Map::new();
    for (index, row) in records[..2].iter().enumerate() {
        let channel = if row["record"] == expected_bidding_record("Damage", "MORE") {
            "Damage/MORE"
        } else if row["record"] == expected_bidding_record("CooldownRecovery", "INC") {
            "CooldownRecovery/INC"
        } else {
            panic!("unreviewed Bidding record, tag, source, value or raw shape");
        };
        let expected = json!({"bucket":"sequence","index":index+1,"name":"MinionModifier","record":row["record"]});
        assert_json_equal(row, &expected, "exact Bidding outer record");
        let mut entry = row.clone();
        entry.as_object_mut().unwrap().remove("index").unwrap();
        assert!(
            channels.insert(channel.into(), entry).is_none(),
            "duplicate Bidding channel"
        );
    }
    assert_eq!(channels.len(), 2);
    drop(local["records"].as_array_mut().unwrap().drain(..2));
    local["bidding_parent_positions"] = json!([1, 2]);
    local["bidding_parent_channels"] = Json::Object(channels);
    true
}

fn semantic_report(mut report: Json) -> Json {
    assert_eq!(report["evidence_view"], "raw_source_observation");
    for case in report["cases"].as_array_mut().unwrap() {
        if case["instrumented"] != true {
            continue;
        }
        for stage in STAGES {
            for call in case["states"][stage]["consumer"]["calls"]
                .as_array_mut()
                .unwrap()
            {
                let source = &call["source"];
                let source_ok = source["source_present"] == false
                    && source["source"] == json!({"kind":"absent"})
                    && source["runtime"]["preset"] == 4
                    && call["stat_set_index"] == 1
                    && match (
                        source["instance"]["skillId"].as_str(),
                        call["effect"].as_str(),
                    ) {
                        (
                            Some("SummonSandDjinnPlayer"),
                            Some("SummonSandDjinnPlayer" | "CommandSandDjinnKnifeThrowPlayer"),
                        ) => source["runtime"]["group"] == 5,
                        (
                            Some("SummonWaterDjinnPlayer"),
                            Some("SummonWaterDjinnPlayer" | "CommandWaterDjinnBubblePlayer"),
                        ) => source["runtime"]["group"] == 9,
                        _ => false,
                    };
                let retained: Vec<_> = rows(&call["effect_list"])
                    .iter()
                    .filter(|r| r["effect"] == "SupportBiddingPlayerTwo")
                    .collect();
                let retained_ok = retained.len() == 1
                    && retained[0]["is_support"] == true
                    && retained[0]["level"] == 1
                    && retained[0]["quality"] == 0;
                for (depth, ancestor) in call["ancestry"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .enumerate()
                {
                    if project_bidding_pair(&mut ancestor["local_records"]) {
                        assert!(
                            source_ok && retained_ok && depth == 0,
                            "Bidding projection requires the exact reviewed manual source and retained support"
                        );
                    }
                }
            }
        }
    }
    report["evidence_view"] = json!("bidding_distinct_channel_projection_v1");
    report
}

fn observe(root: &Path, name: &str, xml: &str, enabled: bool, instrumented: bool) -> Json {
    let before = |lua: &Lua| -> Result<(), RuntimeError> {
        lua.globals().set("consumerJit", enabled)?;
        lua.globals()
            .set("extraConsumptionEffects", lua.to_value(&EFFECTS)?)?;
        lua.load("if consumerJit then jit.on() else jit.off();jit.flush() end")
            .exec()?;
        Ok(())
    };
    let install = |lua: &Lua| -> Result<Function, RuntimeError> {
        let api: mlua::Table = lua
            .load(OBSERVER)
            .set_name("@original-extra-stat-consumer-observer")
            .eval()?;
        lua.globals().set("extraConsumption", api.clone())?;
        if instrumented {
            Ok(api.get::<Function>("install")?.call(())?)
        } else {
            Ok(lua
                .load("return function() assert(debug.gethook()==nil) end")
                .eval()?)
        }
    };
    let stage = |lua: &Lua| -> Result<Json, RuntimeError> {
        let api: mlua::Table = lua.globals().get("extraConsumption")?;
        let capture: Function = api.get("observe")?;
        let rebuild: Function = api.get("rebuild")?;
        let mut states = serde_json::Map::new();
        for (i, stage) in STAGES.iter().enumerate() {
            if i > 0 {
                rebuild.call::<()>(instrumented)?;
            }
            let one: Json = lua.from_value(capture.call::<Value>(())?)?;
            assert_json_equal(
                &one,
                &lua.from_value::<Json>(capture.call::<Value>(())?)?,
                "read-only stage capture",
            );
            states.insert((*stage).into(), one);
        }
        Ok(Json::Object(states))
    };
    let scratch = tempfile::tempdir().unwrap();
    let result = source::observe_with_build_hook_unwrapped(
        &root.join("vendor/path-of-building-poe2"),
        scratch.path(),
        xml,
        None,
        false,
        Some(&before),
        Some(&install),
        Some(&stage),
    )
    .unwrap_or_else(|e| panic!("{name}: original consumer evidence failed: {e}"));
    assert_eq!(result["configuration_method_wrappers"], false);
    assert_eq!(result["original_build_output_available"], true);
    json!({"name":name,"xml_sha256":digest(xml.as_bytes()),"instrumented":instrumented,
        "selected":result["selected"],"states":result["additional_observation"]})
}

fn attributes(node: roxmltree::Node<'_, '_>) -> BTreeMap<String, String> {
    node.attributes()
        .map(|a| (a.name().into(), a.value().into()))
        .collect()
}
fn selected_child<'a, 'input>(
    parent: roxmltree::Node<'a, 'input>,
    name: &str,
    id: &str,
) -> roxmltree::Node<'a, 'input> {
    parent
        .children()
        .find(|n| n.has_tag_name(name) && n.attribute("id") == Some(id))
        .unwrap()
}
fn source_frame(xml: &str) -> Json {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let section = |name| {
        doc.root_element()
            .children()
            .find(|n| n.has_tag_name(name))
            .unwrap()
    };
    let skills = section("Skills");
    let items = section("Items");
    let tree = section("Tree");
    let config = section("Config");
    let item_set = selected_child(items, "ItemSet", items.attribute("activeItemSet").unwrap());
    let skill_set = selected_child(
        skills,
        "SkillSet",
        skills.attribute("activeSkillSet").unwrap(),
    );
    let config_set = selected_child(
        config,
        "ConfigSet",
        config.attribute("activeConfigSet").unwrap(),
    );
    let spec = tree
        .children()
        .filter(|n| n.has_tag_name("Spec"))
        .nth(
            tree.attribute("activeSpec")
                .unwrap()
                .parse::<usize>()
                .unwrap()
                - 1,
        )
        .unwrap();
    let equipment: Vec<_> = item_set
        .children()
        .filter(|n| n.has_tag_name("Slot") && n.attribute("itemId") != Some("0"))
        .map(|n| json!(attributes(n)))
        .collect();
    assert_eq!(equipment.len(), 9);
    let ids: BTreeSet<_> = equipment
        .iter()
        .map(|s| s["itemId"].as_str().unwrap())
        .collect();
    assert_eq!(ids.len(), 8);
    let party = section("Party");
    assert!(
        !party
            .children()
            .any(|n| n.is_element() || n.text().is_some_and(|t| !t.trim().is_empty()))
    );
    json!({"axes":{"skills":skills.attribute("activeSkillSet"),"items":items.attribute("activeItemSet"),
        "passives":tree.attribute("activeSpec"),"config":config.attribute("activeConfigSet")},
        "equipment_uses":equipment,"skill_set_xml":&xml[skill_set.range()],"spec_xml":&xml[spec.range()],
        "config_xml":&xml[config_set.range()],"item_set_xml":&xml[item_set.range()],"party_xml":&xml[party.range()]})
}

fn validate(report: &Json, xml: &str) {
    let cases = report["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 3);
    let doc = roxmltree::Document::parse(xml).unwrap();
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([109; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    let expected_axes = json!({"skills":4,"items":2,"passives":3,"config":1});
    let equipment = report["source_frame"]["equipment_uses"].as_array().unwrap();
    for case in cases {
        assert_eq!(case["selected"], expected_axes);
        for stage in STAGES {
            assert_json_equal(
                &case["states"][stage]["outputs"],
                &cases[2]["states"][stage]["outputs"],
                &format!(
                    "observer cannot change numerical output: {} {stage}",
                    case["name"]
                ),
            );
        }
    }
    for case in &cases[..2] {
        for stage in STAGES {
            let state = &case["states"][stage]["consumer"];
            assert_eq!(state["hook_removed"], true);
            assert_eq!(state["original_functions_preserved"], true);
            assert_eq!(state["native_field_disposition"], false);
            assert_eq!(state["whole_supplier_domain_complete"], false);
            let environments = rows(&state["environments"]);
            assert!(!environments.is_empty());
            for env in environments {
                assert_eq!(env["axes"], expected_axes);
                let uses = rows(&env["items"]);
                assert_eq!(uses.len(), 9);
                let actual: BTreeSet<_> = uses
                    .iter()
                    .map(|r| (r["slot"].as_str().unwrap(), r["id"].as_u64().unwrap()))
                    .collect();
                let expected: BTreeSet<_> = equipment
                    .iter()
                    .map(|r| {
                        (
                            r["name"].as_str().unwrap(),
                            r["itemId"].as_str().unwrap().parse::<u64>().unwrap(),
                        )
                    })
                    .collect();
                assert_eq!(actual, expected, "distinct equipment-use identities");
                for item in uses {
                    assert_eq!(item["saved_object_exact"], true);
                    assert_eq!(item["id"], item["selected_item_id"]);
                }
                assert_eq!(env["config"]["input_exact"], true);
                assert_eq!(env["config"]["placeholder_exact"], true);
            }
            let calls = rows(&state["calls"]);
            assert!(!calls.is_empty());
            let mut seen = BTreeSet::new();
            for call in calls {
                assert_eq!(call["caller_line"], 795);
                assert_eq!(call["actor_is_player"], true);
                for flag in [
                    "exact_effect_cfg",
                    "exact_caller_objects",
                    "observer_noninterference",
                    "original_return_observed",
                ] {
                    assert_eq!(call[flag], true, "{} {stage} {flag}", case["name"]);
                }
                assert_eq!(call["observer_requeried_list"], false);
                let env_index = call["environment"].as_u64().unwrap() as usize;
                assert!((1..=environments.len()).contains(&env_index));
                assert_eq!(call["mode"], environments[env_index - 1]["mode"]);
                let source = &call["source"];
                assert_eq!(source["runtime"]["preset"], 4);
                let skill_id = source["instance"]["skillId"].as_str().unwrap();
                let source_value = source["source"].as_str();
                let source_present = source["source_present"].as_bool().unwrap();
                assert_eq!(source_present, source_value.is_some());
                let groups: Vec<_> = doc
                    .descendants()
                    .filter(|n| {
                        n.has_tag_name("Skill")
                            && n.parent().is_some_and(|p| {
                                p.has_tag_name("SkillSet") && p.attribute("id") == Some("4")
                            })
                            && n.attribute("source") == source_value
                            && n.children().any(|g| {
                                g.has_tag_name("Gem") && g.attribute("skillId") == Some(skill_id)
                            })
                    })
                    .collect();
                assert_eq!(
                    groups.len(),
                    1,
                    "exact original source frame {skill_id} {source_value:?}"
                );
                let ordinal = doc
                    .descendants()
                    .filter(|n| n.is_element())
                    .position(|n| n == groups[0])
                    .unwrap();
                let source_row = &evidence.rows()[ordinal];
                assert_eq!(source_row.occurrence().id().ordinal() as usize, ordinal);
                assert_eq!(source_row.occurrence().name(), "Skill");
                let actual: BTreeMap<_, _> = source_row
                    .attributes()
                    .iter()
                    .map(|a| (a.origin().name.clone(), a.decoded().unwrap().to_owned()))
                    .collect();
                assert_eq!(
                    actual,
                    attributes(groups[0]),
                    "independent complete source attribute correspondence"
                );
                seen.insert((
                    call["mode"].as_str().unwrap(),
                    call["effect"].as_str().unwrap(),
                    source_value,
                ));
                // Payloads are retained, not assumed empty. Any key/opaque/nested
                // supplier requires a separately reviewed map/transport disposition.
                assert_eq!(call["extra_stats"]["kind"], "raw_table");
                assert!(!rows(&call["ancestry"]).is_empty());
            }
            for mode in ["MAIN", "CALCS"] {
                for (effect, source) in [
                    ("SummonSandDjinnPlayer", Some("Tree:13289")),
                    ("CommandSandDjinnKnifeThrowPlayer", Some("Tree:13289")),
                    ("SummonWaterDjinnPlayer", Some("Tree:32705")),
                    ("CommandWaterDjinnBubblePlayer", Some("Tree:32705")),
                    ("FireboltPlayer", Some(STAFF_SOURCE)),
                ] {
                    assert!(
                        seen.contains(&(mode, effect, source)),
                        "missing exact consumer {mode} {effect} {source:?}"
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod projection_tests {
    use super::*;

    fn local(reverse: bool) -> Json {
        let mut pair = [
            expected_bidding_record("Damage", "MORE"),
            expected_bidding_record("CooldownRecovery", "INC"),
        ];
        if reverse {
            pair.reverse();
        }
        json!({"present":true,"count":4,"selection":"extra_stats_and_nested_supplier_records","records":[
            {"bucket":"sequence","index":1,"name":"MinionModifier","record":pair[0]},
            {"bucket":"sequence","index":2,"name":"MinionModifier","record":pair[1]},
            {"bucket":"sequence","index":7,"name":"MinionModifier","record":{"source":"unreviewed-a","name":"Damage","type":"MORE","value":2}},
            {"bucket":"sequence","index":8,"name":"MinionModifier","record":{"source":"unreviewed-b","name":"Damage","type":"MORE","value":3}}
        ]})
    }

    fn project(mut value: Json) -> Json {
        assert!(project_bidding_pair(&mut value));
        value
    }

    #[test]
    fn only_reviewed_distinct_channels_have_incidental_order() {
        let original = local(false);
        let projected = project(original.clone());
        assert_eq!(projected, project(local(true)));
        assert_eq!(
            projected["records"],
            json!(&original["records"].as_array().unwrap()[2..])
        );
        assert_eq!(projected["bidding_parent_positions"], json!([1, 2]));
        assert_eq!(projected["count"], original["count"]);
        let mut same_channel = original.clone();
        same_channel["records"].as_array_mut().unwrap().swap(2, 3);
        assert_ne!(
            projected,
            project(same_channel),
            "same-channel or unreviewed order must remain exact"
        );
        let mut numerical = original;
        numerical["records"][2]["record"]["value"] = json!(4);
        assert_ne!(
            projected,
            project(numerical),
            "unrelated numerical values remain exact"
        );
    }

    #[test]
    fn duplicate_unknown_or_moved_bidding_records_are_rejected() {
        let initial = local(false);
        let mut duplicate = initial.clone();
        duplicate["records"][1]["record"] = duplicate["records"][0]["record"].clone();
        let mut unknown = initial.clone();
        unknown["records"][0]["record"]["extra"] = json!(true);
        let mut changed_value = initial.clone();
        changed_value["records"][0]["record"]["fields"][5]["value"]["fields"][0]["value"]["fields"]
            [6]["value"] = json!(31);
        let mut moved = initial.clone();
        moved["records"][0]["index"] = json!(3);
        let mut third = initial;
        let extra = third["records"][0].clone();
        third["records"].as_array_mut().unwrap().push(extra);
        for invalid in [duplicate, unknown, changed_value, moved, third] {
            assert!(std::panic::catch_unwind(|| project(invalid)).is_err());
        }
    }

    #[test]
    fn unreviewed_sources_are_never_rewritten() {
        let mut value = local(false);
        for row in &mut value["records"].as_array_mut().unwrap()[..2] {
            row["record"]["fields"][3]["value"] = json!("Skill:DifferentSupport");
        }
        let before = value.clone();
        assert!(!project_bidding_pair(&mut value));
        assert_eq!(value, before);
    }
}
