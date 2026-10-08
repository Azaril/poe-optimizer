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
const FOCUS_TEST: &str = "generated_extra_stat_consumption::manual_djinn_global_two_reaches_original_extra_stat_admission";
const FOCUS_CHILD: &str = "POE_MANUAL_DJINN_ADMISSION_CHILD";
const FOCUS_OUTPUT: &str = "POE_MANUAL_DJINN_ADMISSION_OUT";
const DJINN: [&str; 2] = ["SummonSandDjinnPlayer", "SummonWaterDjinnPlayer"];

#[test]
#[ignore = "requires original pinned PoB; compact manual Djinn consumer accounting only"]
fn manual_djinn_global_two_reaches_original_extra_stat_admission() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = std::env::var_os(FOCUS_OUTPUT)
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("runs/owned-manual-djinn-admission-source-01"));
    if let Some(mode) = std::env::var_os(FOCUS_CHILD) {
        assert!(mode == "off" || mode == "on");
        run_focused(&root, &out, mode == "on");
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
            .args(["--exact", FOCUS_TEST, "--ignored", "--nocapture"])
            .env(FOCUS_CHILD, mode)
            .env(FOCUS_OUTPUT, &out)
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
                    "compact source child failed: {}\n{}",
                    path.display(),
                    tail(&path)
                );
                break;
            }
            if started.elapsed() > Duration::from_secs(600) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!(
                    "compact source child deadline: {}\n{}",
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
        "exact compact original admission evidence across JIT modes",
    );
}

fn manual_change(xml: &str, skill: &str, field: &str, value: &str) -> String {
    assert!(DJINN.contains(&skill));
    assert!(matches!(field, "enableGlobal2" | "enabled"));
    let doc = roxmltree::Document::parse(xml).unwrap();
    let skills = doc
        .descendants()
        .find(|n| n.has_tag_name("Skills"))
        .unwrap();
    let set = selected_child(
        skills,
        "SkillSet",
        skills.attribute("activeSkillSet").unwrap(),
    );
    let found: Vec<_> = set
        .children()
        .filter(|n| n.has_tag_name("Skill") && n.attribute("source").is_none())
        .flat_map(|n| n.children())
        .filter(|n| n.has_tag_name("Gem") && n.attribute("skillId") == Some(skill))
        .collect();
    assert_eq!(found.len(), 1, "exact selected manual occurrence");
    assert_eq!(found[0].attribute(field), Some("true"));
    change_attributes(xml, found[0], &[(field, Some(value))])
}

fn focused_inputs(xml: &str) -> Vec<(&'static str, String)> {
    let sand = manual_change(xml, DJINN[0], "enableGlobal2", "false");
    vec![
        ("original-05", xml.to_owned()),
        ("manual-sand-global2-false", sand.clone()),
        (
            "manual-water-global2-false",
            manual_change(xml, DJINN[1], "enableGlobal2", "false"),
        ),
        (
            "both-manual-global2-false",
            manual_change(&sand, DJINN[1], "enableGlobal2", "false"),
        ),
        (
            "manual-sand-disabled",
            manual_change(xml, DJINN[0], "enabled", "false"),
        ),
        (
            "manual-water-disabled",
            manual_change(xml, DJINN[1], "enabled", "false"),
        ),
        ("repeat-original-05", xml.to_owned()),
    ]
}

// Each runtime join is backed independently by the imported XML occurrence and
// every saved attribute. The observer's runtime position is kept separately.
fn focused_saved_sources(xml: &str) -> Vec<Json> {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([109; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    let skills = doc
        .descendants()
        .find(|n| n.has_tag_name("Skills"))
        .unwrap();
    let set = selected_child(
        skills,
        "SkillSet",
        skills.attribute("activeSkillSet").unwrap(),
    );
    let mut result = Vec::new();
    for (gi, group) in set
        .children()
        .filter(|n| n.has_tag_name("Skill"))
        .enumerate()
    {
        for (position, gem) in group
            .children()
            .filter(|n| n.has_tag_name("Gem"))
            .enumerate()
        {
            if !gem
                .attribute("skillId")
                .is_some_and(|id| DJINN.contains(&id))
            {
                continue;
            }
            let mut ordinals = Vec::new();
            for node in [group, gem] {
                let ordinal = doc
                    .descendants()
                    .filter(|n| n.is_element())
                    .position(|n| n == node)
                    .unwrap();
                let source = &evidence.rows()[ordinal];
                assert_eq!(source.occurrence().id().ordinal() as usize, ordinal);
                assert_eq!(source.occurrence().name(), node.tag_name().name());
                let attrs: BTreeMap<_, _> = source
                    .attributes()
                    .iter()
                    .map(|a| (a.origin().name.clone(), a.decoded().unwrap().to_owned()))
                    .collect();
                assert_eq!(attrs, attributes(node));
                ordinals.push(ordinal);
            }
            result.push(json!({"preset":set.attribute("id"),"saved_group":gi+1,"saved_position":position+1,
                "group_ordinal":ordinals[0],"gem_ordinal":ordinals[1],
                "group_range":[group.range().start,group.range().end],"gem_range":[gem.range().start,gem.range().end],
                "group_attributes":attributes(group),"gem_attributes":attributes(gem),
                "group_xml_sha256":digest(xml[group.range()].as_bytes()),
                "gem_xml_sha256":digest(xml[gem.range()].as_bytes())}));
        }
    }
    assert_eq!(
        result.len(),
        4,
        "two manual and two exact tree-supplied saved instances"
    );
    result
}

fn run_focused(root: &Path, out: &Path, enabled: bool) {
    let input = root.join("tests/fixtures/builds/breadth-20260908/build-05.xml");
    let xml = fs::read_to_string(&input).unwrap();
    let index: Json =
        serde_json::from_slice(&fs::read(input.parent().unwrap().join("index.json")).unwrap())
            .unwrap();
    assert_eq!(digest(xml.as_bytes()), index["builds"][4]["xml_sha256"]);
    let inputs = focused_inputs(&xml);
    let mut cases = Vec::new();
    for (name, changed) in &inputs {
        eprintln!(
            "Compact manual Djinn admission {name}, JIT {}",
            if enabled { "on" } else { "off" }
        );
        let mut case = observe_mode(root, name, changed, enabled, true, true);
        case["saved_sources"] = json!(focused_saved_sources(changed));
        if *name != "repeat-original-05" {
            let uninstrumented = observe_mode(root, name, changed, enabled, false, true);
            case["uninstrumented_states"] = uninstrumented["states"].clone();
        }
        cases.push(case);
    }
    let files = [
        "src/Modules/CalcActiveSkill.lua",
        "src/Modules/CalcSetup.lua",
        "src/Modules/CalcPerform.lua",
        "src/Modules/CalcTools.lua",
        "src/Classes/ModStore.lua",
        "src/Classes/ModList.lua",
        "src/Classes/ModDB.lua",
        "src/Classes/SkillsTab.lua",
        "src/Modules/Data.lua",
        "src/Data/Gems.lua",
        "src/Data/Skills/other.lua",
        "src/Data/SkillStatMap.lua",
        "src/Modules/ModTools.lua",
        "src/HeadlessWrapper.lua",
        "src/Modules/Main.lua",
    ];
    let report = json!({"schema_version":1,"evidence_view":"manual_djinn_global2_original_admission_v1",
        "source_revision":pinned::UPSTREAM_REVISION,"manifest_sha256":pinned::manifest_sha256(),
        "observer_sha256":digest(OBSERVER.as_bytes()),
        "test_sha256":digest(include_str!("generated_extra_stat_consumption.rs").as_bytes()),
        "original_path":"tests/fixtures/builds/breadth-20260908/build-05.xml","original_sha256":digest(xml.as_bytes()),
        "source_frame":source_frame(&xml),"lifecycle":STAGES,
        "execution_counts_per_jit":{"fresh_vm_loads":13,"normal_rebuilds":26,"lifecycle_state_captures":39},
        "selection_scope":"exact selected manual Sand/Water and separate Tree:13289/32705 copies",
        "admission_scope":"original List at CalcActiveSkill:795, original per-ancestor filtering, identical returned table at original merge call and return",
        "environment_scope":"every original call retained in call order, including ancillary MAIN/CALCULATOR environments; absence census covers only identity-checked final MAIN/CALCS environments",
        "not_called_scope":"explicit selected-source/effect census; absence is not a queried empty result",
        "native_inventory_authority":false,"native_build_parity":false,"field_non_applicability_certificate":false,
        "all_suppliers_or_transforms_proved":false,"dormant_presets_covered":false,"business_wrappers":false,
        "observer_requeries_extra_stats":false,"numeric_tolerance":0,"projection_or_deduplication":false,
        "files":files.map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})),"cases":cases});
    let bytes = serde_json::to_vec(&report).unwrap();
    let mode = if enabled { "on" } else { "off" };
    fs::write(out.join(format!("source-jit-{mode}.raw.json")), &bytes).unwrap();
    assert!(
        // First complete capture was 9,327,788 bytes: 168 original calls per
        // enabled state, including ancillary calculator environments. Retain
        // every call rather than deduplicate it against the final view census.
        bytes.len() <= 16 * 1024 * 1024,
        "compact source evidence bound (16 MiB, all original calls retained)"
    );
    validate_focused(&report);
    assert_json_equal(
        &report["cases"][0]["states"],
        &report["cases"][6]["states"],
        "independent compact fresh replay",
    );
    assert_eq!(fs::read_to_string(input).unwrap(), xml);
    fs::write(out.join(format!("source-jit-{mode}.json")), bytes).unwrap();
}

fn validate_focused(report: &Json) {
    let cases = report["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 7);
    for (ci, case) in cases.iter().enumerate() {
        assert_eq!(
            case["selected"],
            json!({"skills":4,"items":2,"passives":3,"config":1})
        );
        for stage in STAGES {
            if ci != 6 {
                assert_json_equal(
                    &case["states"][stage]["outputs"],
                    &case["uninstrumented_states"][stage]["outputs"],
                    &format!("compact hook noninterference {} {stage}", case["name"]),
                );
                assert!(
                    case["uninstrumented_states"][stage]
                        .get("consumer")
                        .is_none()
                );
            }
            let state = &case["states"][stage]["consumer"];
            for flag in ["original_functions_preserved", "hook_removed"] {
                assert_eq!(state[flag], true);
            }
            for flag in [
                "observer_requeried_list",
                "native_field_disposition",
                "whole_supplier_domain_complete",
            ] {
                assert_eq!(state[flag], false);
            }
            let calls = rows(&state["calls"]);
            let environments = rows(&state["environments"]);
            let final_environments = focused_final_environments(environments);
            let mut final_call_indices = BTreeSet::new();
            for (i, call) in calls.iter().enumerate() {
                let environment = call["environment"].as_u64().unwrap() as usize;
                assert!((1..=environments.len()).contains(&environment));
                assert_eq!(call["mode"], environments[environment - 1]["mode"]);
                focused_saved_source(case, &call["source"]);
                assert!(
                    djinn_effects(call["source"]["instance"]["skillId"].as_str().unwrap())
                        .contains(&call["effect"].as_str().unwrap())
                );
                validate_focused_call(call);
                if final_environments.contains(&environment) {
                    final_call_indices.insert(i + 1);
                }
            }
            let census = rows(&state["census"]);
            assert_eq!(census.len(), 2);
            let mut used = BTreeSet::new();
            let mut census_environments = BTreeSet::new();
            for mode in census {
                assert!(matches!(mode["mode"].as_str(), Some("MAIN" | "CALCS")));
                let environment = mode["environment"].as_u64().unwrap() as usize;
                assert!(final_environments.contains(&environment));
                assert!(census_environments.insert(environment));
                assert_eq!(mode["mode"], environments[environment - 1]["mode"]);
                let groups = rows(&mode["groups"]);
                assert_eq!(groups.len(), 4);
                let mut identities = BTreeSet::new();
                for group in groups {
                    let src = &group["source"];
                    let saved = focused_saved_source(case, src);
                    assert!(identities.insert(saved["gem_ordinal"].as_u64().unwrap()));
                    assert_eq!(
                        src["source_present"],
                        saved["group_attributes"].get("source").is_some()
                    );
                    let enabled = saved["gem_attributes"]["enabled"] == "true";
                    assert_eq!(src["instance"]["enabled"], enabled);
                    assert_eq!(rows(&group["effects"]).len(), 2);
                    for (ei, effect) in rows(&group["effects"]).iter().enumerate() {
                        assert_eq!(effect["catalogue_identity"], true);
                        assert_eq!(effect["index"], ei + 1);
                        let field = format!("enableGlobal{}", ei + 1);
                        assert_eq!(effect["global_field"], field);
                        assert_eq!(
                            effect["global_value"],
                            saved["gem_attributes"][&field] == "true"
                        );
                        assert_eq!(
                            effect["has_global_effect"],
                            json!({"kind":"absent"}),
                            "bounded pinned catalogue fact only"
                        );
                        validate_global_lookup(&effect["global_effect_lookup"]);
                        let expected_effect =
                            djinn_effects(src["instance"]["skillId"].as_str().unwrap())[ei];
                        assert_eq!(effect["id"], expected_effect);
                        assert_eq!(effect["active_skill_present"], enabled);
                        assert_eq!(rows(&effect["active_skills"]).len(), usize::from(enabled));
                        let indices = rows(&effect["builder_call_indices"]);
                        if !enabled {
                            assert!(indices.is_empty());
                            assert_eq!(effect["consumer_observation"], "not_called");
                            continue;
                        }
                        assert_eq!(effect["consumer_observation"], "inspect_original_call_rows");
                        assert!(!indices.is_empty(), "active original consumer was observed");
                        for index in indices {
                            let index = index.as_u64().unwrap() as usize;
                            assert!((1..=calls.len()).contains(&index));
                            assert!(used.insert(index));
                            let call = &calls[index - 1];
                            assert_eq!(call["source"]["runtime"], src["runtime"]);
                            assert_eq!(call["effect"], effect["id"]);
                            assert_eq!(call["mode"], mode["mode"]);
                            assert_eq!(call["environment"], mode["environment"]);
                        }
                    }
                }
            }
            assert_eq!(census_environments, final_environments);
            assert_eq!(
                used, final_call_indices,
                "every final-view call has one exact census join; ancillary calls remain independently validated"
            );
        }
    }
}

fn djinn_effects(skill: &str) -> [&'static str; 2] {
    match skill {
        "SummonSandDjinnPlayer" => [DJINN[0], "CommandSandDjinnKnifeThrowPlayer"],
        "SummonWaterDjinnPlayer" => [DJINN[1], "CommandWaterDjinnBubblePlayer"],
        _ => panic!("unexpected selected source {skill}"),
    }
}

fn focused_saved_source<'a>(case: &'a Json, src: &Json) -> &'a Json {
    assert_eq!(src["runtime"]["preset"], 4);
    assert_eq!(rows(&src["runtime"]["positions"]).len(), 1);
    let saved: Vec<_> = rows(&case["saved_sources"])
        .iter()
        .filter(|s| {
            s["gem_attributes"]["skillId"] == src["instance"]["skillId"]
                && s["group_attributes"]["source"].as_str() == src["source"].as_str()
        })
        .collect();
    assert_eq!(saved.len(), 1, "exact saved supplier join");
    assert_eq!(
        src["source_present"],
        saved[0]["group_attributes"].get("source").is_some()
    );
    assert_eq!(
        src["instance"]["enabled"],
        saved[0]["gem_attributes"]["enabled"] == "true"
    );
    saved[0]
}

fn focused_final_environments(environments: &[Json]) -> BTreeSet<usize> {
    let mut found = BTreeMap::new();
    for (i, env) in environments.iter().enumerate() {
        assert_eq!(env["index"], i + 1);
        assert!(matches!(
            env["mode"].as_str(),
            Some("MAIN" | "CALCS" | "CALCULATOR")
        ));
        for (flag, mode) in [("final_main", "MAIN"), ("final_calcs", "CALCS")] {
            if env[flag].as_bool().unwrap() {
                assert_eq!(env["mode"], mode);
                assert!(
                    found.insert(mode, i + 1).is_none(),
                    "one exact final environment per view"
                );
            }
        }
    }
    assert_eq!(found.len(), 2);
    found.into_values().collect()
}

fn validate_global_lookup(lookup: &Json) {
    assert_eq!(lookup["has_metatable"], false);
    assert_eq!(lookup["lookup_exact"], true);
    assert_eq!(lookup["raw"], json!({"kind":"absent"}));
    assert_eq!(
        lookup["effective"], lookup["raw"],
        "raw absence is not effective absence without this proof"
    );
}

fn validate_focused_call(call: &Json) {
    validate_global_lookup(&call["global_effect_lookup"]);
    for flag in [
        "builder_called",
        "builder_returned",
        "query_called",
        "query_returned",
        "merge_called",
        "merge_returned",
        "exact_filter_identity",
        "exact_returned_payload",
        "payload_unchanged",
    ] {
        assert_eq!(call[flag], true, "missing {flag}");
    }
    assert_eq!(call["observer_requeried_list"], false);
    assert_eq!(call["query_caller_line"], 795);
    assert_eq!(call["merge_caller_line"], 795);
    assert_eq!(call["cfg_effect_id"], call["effect"]);
    assert_eq!(call["returned_count"], 0);
    assert_eq!(call["admitted_count"], 0);
    assert_json_equal(
        &call["returned_stats"],
        &call["admitted_stats"],
        "original result table handed to consumer",
    );
    assert_eq!(call["returned_stats"]["kind"], "raw_table");
    assert_eq!(call["returned_stats"]["has_metatable"], false);
    assert!(rows(&call["returned_stats"]["fields"]).is_empty());
    let chain = rows(&call["candidates"]);
    assert!(!chain.is_empty());
    let internal = rows(&call["internal_calls"]);
    assert_eq!(chain.len(), internal.len());
    for (depth, (raw, actual)) in chain.iter().zip(internal).enumerate() {
        assert_eq!(raw["depth"], depth);
        assert!(
            rows(&raw["records"]).is_empty(),
            "bounded empty ExtraSkillStat bucket; no general filter law"
        );
        assert_eq!(actual["depth"], depth);
        assert_eq!(actual["name"], "ExtraSkillStat");
        for flag in ["exact_context", "exact_filter", "returned"] {
            assert_eq!(actual[flag], true);
        }
        assert_eq!(actual["result_count_before"], 0);
        assert_eq!(actual["result_count_after"], 0);
    }
}

#[test]
fn manual_djinn_controls_change_only_the_exact_saved_manual_fields() {
    let xml = include_str!("../../../../tests/fixtures/builds/breadth-20260908/build-05.xml");
    let before = focused_saved_sources(xml);
    for (name, changed) in focused_inputs(xml) {
        let sources = focused_saved_sources(&changed);
        for (old, new) in before.iter().zip(&sources) {
            assert_eq!(old["gem_ordinal"], new["gem_ordinal"]);
            assert_eq!(old["group_ordinal"], new["group_ordinal"]);
            let mut old_attrs = old["gem_attributes"].clone();
            let mut new_attrs = new["gem_attributes"].clone();
            if old["group_attributes"].get("source").is_none() {
                old_attrs.as_object_mut().unwrap().remove("enableGlobal2");
                new_attrs.as_object_mut().unwrap().remove("enableGlobal2");
                old_attrs.as_object_mut().unwrap().remove("enabled");
                new_attrs.as_object_mut().unwrap().remove("enabled");
            }
            assert_eq!(
                old_attrs, new_attrs,
                "{name}: no unrelated gem field mutation"
            );
        }
        let old_frame = source_frame(xml);
        let new_frame = source_frame(&changed);
        for field in [
            "axes",
            "equipment_uses",
            "spec_xml",
            "config_xml",
            "item_set_xml",
            "party_xml",
        ] {
            assert_eq!(
                old_frame[field], new_frame[field],
                "{name}: unchanged {field}"
            );
        }
    }
}

#[test]
fn compact_admission_rejects_unqueried_or_missing_transport_as_empty() {
    let empty = json!({"kind":"raw_table","has_metatable":false,"fields":[]});
    let valid = json!({"builder_called":true,"builder_returned":true,"query_called":true,"query_returned":true,
        "global_effect_lookup":{"raw":{"kind":"absent"},"effective":{"kind":"absent"},"has_metatable":false,"lookup_exact":true},
        "merge_called":true,"merge_returned":true,"exact_filter_identity":true,"exact_returned_payload":true,
        "payload_unchanged":true,"observer_requeried_list":false,"query_caller_line":795,"merge_caller_line":795,
        "effect":"bounded-test-effect","cfg_effect_id":"bounded-test-effect","returned_count":0,"admitted_count":0,
        "returned_stats":empty,"admitted_stats":empty,"candidates":[{"depth":0,"records":[]}],
        "internal_calls":[{"depth":0,"name":"ExtraSkillStat","exact_context":true,"exact_filter":true,
            "returned":true,"result_count_before":0,"result_count_after":0}]});
    validate_focused_call(&valid);
    let mut unqueried = valid.clone();
    unqueried["query_called"] = json!(false);
    let mut missing = valid.clone();
    missing
        .as_object_mut()
        .unwrap()
        .remove("exact_returned_payload");
    let mut no_consumer = valid.clone();
    no_consumer["merge_called"] = json!(false);
    let mut candidate = valid.clone();
    candidate["candidates"][0]["records"] = json!([{"unreviewed":true}]);
    let mut changed = valid;
    changed["admitted_stats"]["fields"] = json!([{"key":"unreviewed"}]);
    for invalid in [unqueried, missing, no_consumer, candidate, changed] {
        assert!(std::panic::catch_unwind(|| validate_focused_call(&invalid)).is_err());
    }
}

#[test]
fn compact_admission_keeps_ancillary_environments_and_rejects_raw_only_metadata() {
    let environments = json!([
        {"index":1,"mode":"MAIN","final_main":true,"final_calcs":false},
        {"index":2,"mode":"MAIN","final_main":false,"final_calcs":false},
        {"index":3,"mode":"CALCULATOR","final_main":false,"final_calcs":false},
        {"index":4,"mode":"CALCS","final_main":false,"final_calcs":true}
    ]);
    assert_eq!(
        focused_final_environments(rows(&environments)),
        BTreeSet::from([1, 4])
    );
    let mut duplicate = environments.clone();
    duplicate[1]["final_main"] = json!(true);
    let mut wrong = environments;
    wrong[2]["final_main"] = json!(true);
    for invalid in [duplicate, wrong] {
        assert!(std::panic::catch_unwind(|| focused_final_environments(rows(&invalid))).is_err());
    }
    let valid = json!({"raw":{"kind":"absent"},"effective":{"kind":"absent"},"has_metatable":false,"lookup_exact":true});
    validate_global_lookup(&valid);
    let mut inherited = valid.clone();
    inherited["has_metatable"] = json!(true);
    inherited["effective"] = json!(true);
    let mut missing = valid;
    missing.as_object_mut().unwrap().remove("effective");
    for invalid in [inherited, missing] {
        assert!(std::panic::catch_unwind(|| validate_global_lookup(&invalid)).is_err());
    }
}

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
        "src/Modules/Common.lua",
        "src/Classes/ModStore.lua",
        "src/Classes/ModList.lua",
        "src/Classes/ModDB.lua",
        "src/Classes/Item.lua",
        "src/Classes/PartyTab.lua",
        "src/Classes/ConfigTab.lua",
        "src/Classes/SkillsTab.lua",
        "src/Classes/PassiveSpec.lua",
        "src/Classes/PassiveTree.lua",
        "src/Modules/Data.lua",
        "src/Data/SkillStatMap.lua",
        "src/Modules/ModTools.lua",
        "src/Modules/ModParser.lua",
        "src/Data/ModCache.lua",
        "src/Data/Global.lua",
        "src/Modules/Main.lua",
        "src/HeadlessWrapper.lua",
        "src/Modules/Build.lua",
    ];
    let report = json!({"schema_version":4,"source_revision":pinned::UPSTREAM_REVISION,
        "node_supplier_scope":"local and authenticated effective fields plus actual per-node original returns; cross-node invocation order not captured",
        "amulet_supplier_scope":"exact unchanged Item23 original ScaleAddMod/AddMod calls and returns, both source records at factor zero; no general transformation law",
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
        // Source03 already retained 62.74 MB before effective node inputs and
        // original return records. Keep all evidence under an explicit bound.
        bytes.len() <= 128 * 1024 * 1024,
        "bounded diagnostic artifact (128 MiB, including effective node evidence)"
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
    observe_mode(root, name, xml, enabled, instrumented, false)
}

fn observe_mode(
    root: &Path,
    name: &str,
    xml: &str,
    enabled: bool,
    instrumented: bool,
    focused: bool,
) -> Json {
    let before = |lua: &Lua| -> Result<(), RuntimeError> {
        lua.globals().set("consumerJit", enabled)?;
        lua.globals().set("extraConsumptionFocus", focused)?;
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
    let spec = doc
        .descendants()
        .find(|n| n.has_tag_name("Tree"))
        .unwrap()
        .children()
        .filter(|n| n.has_tag_name("Spec"))
        .nth(2)
        .unwrap();
    let source_nodes: BTreeSet<u64> = spec
        .attribute("nodes")
        .unwrap()
        .split(',')
        .map(|id| id.parse().unwrap())
        .collect();
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
                let amulet: Vec<_> = uses
                    .iter()
                    .filter(|item| item["slot"] == "Amulet")
                    .collect();
                assert_eq!(amulet.len(), 1);
                validate_amulet(
                    &env["amulet_transport"],
                    amulet[0],
                    env["mode"].as_str().unwrap(),
                );
                assert_eq!(env["config"]["input_exact"], true);
                assert_eq!(env["config"]["placeholder_exact"], true);
                let nodes = rows(&env["nodes"]);
                let actual_nodes: BTreeSet<_> =
                    nodes.iter().map(|n| n["id"].as_u64().unwrap()).collect();
                assert_eq!(
                    actual_nodes.len(),
                    nodes.len(),
                    "unique allocated node identities"
                );
                assert_eq!(
                    actual_nodes, source_nodes,
                    "exact unchanged selected node membership"
                );
                for node in nodes {
                    validate_node(node, env["mode"].as_str().unwrap());
                }
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

fn expected_amulet_record(index: usize, copied: bool, scaled: bool) -> Json {
    let source = if copied {
        "Many Sources:^x88FFFF0% Amulet Bonus Effect"
    } else {
        "Item:23:New Item, Solar Amulet"
    };
    let (name, kind, value) = match index {
        0 => ("Spirit", "BASE", json!(if scaled { 0 } else { 13 })),
        1 => (
            "GemProperty",
            "LIST",
            raw_table(vec![
                (json!("key"), json!("level")),
                (json!("keyOfScaledMod"), json!("value")),
                (json!("keyword"), json!("minion")),
                (json!("value"), json!(if scaled { 0 } else { 1 })),
            ]),
        ),
        _ => panic!("unreviewed original Amulet record"),
    };
    raw_table(vec![
        (json!("flags"), json!(0)),
        (json!("keywordFlags"), json!(0)),
        (json!("name"), json!(name)),
        (json!("source"), json!(source)),
        (json!("sourceSlot"), json!("Amulet")),
        (json!("type"), json!(kind)),
        (json!("value"), value),
    ])
}

fn validate_amulet(transport: &Json, item: &Json, mode: &str) {
    assert_eq!(item["id"], 23);
    let source_records = rows(&item["active"]["records"]);
    assert_eq!(source_records.len(), 2);
    let transport = rows(transport);
    assert_eq!(
        transport.len(),
        2,
        "zero factor must retain both original AddMod deliveries"
    );
    for (index, row) in transport.iter().enumerate() {
        assert_eq!(row["item_id"], 23);
        assert_eq!(row["slot"], "Amulet");
        assert_eq!(row["mode"], mode);
        assert_eq!(row["source_index"], index + 1);
        assert_eq!(row["source_count"], 2);
        assert_eq!(row["caller_line"], 1667);
        assert_eq!(row["add_caller_line"], 117);
        assert_eq!(row["factor"], 0);
        assert_eq!(row["round_to_nearest"], json!({"kind":"absent"}));
        for flag in [
            "exact_saved_item",
            "exact_selected_slot",
            "exact_active_list",
            "exact_receiver",
            "copy_is_distinct",
            "original_accessor_preserved",
            "observer_noninterference",
            "inserted_object_exact",
            "prior_bucket_objects_preserved",
            "add_return_observed",
            "scale_return_observed",
            "source_record_unchanged",
        ] {
            assert_eq!(row[flag], true, "Amulet transport requires {flag}");
        }
        assert_eq!(row["observer_requeried_item"], false);
        assert_json_equal(
            &row["source_record"],
            &expected_amulet_record(index, false, false),
            "complete original Amulet record",
        );
        assert_json_equal(
            &row["source_record"],
            &source_records[index]["record"],
            "exact contemporaneous Amulet supplier",
        );
        assert_json_equal(
            &row["scale_argument"],
            &expected_amulet_record(index, true, false),
            "complete rewritten Amulet copy",
        );
        assert_json_equal(
            &row["delivered_record"],
            &expected_amulet_record(index, true, true),
            "complete zero-scaled Amulet delivery",
        );
        assert_eq!(
            row["bucket"],
            if index == 0 { "Spirit" } else { "GemProperty" }
        );
        let before = row["bucket_count_before"].as_u64().unwrap();
        assert!(before < 16384);
        assert_eq!(row["bucket_count_after"], before + 1);
        assert_eq!(row["inserted_index"], before + 1);
    }
}

fn validate_node(node: &Json, mode: &str) {
    let id = node["id"].as_u64().unwrap();
    assert_eq!(node["node_id"], id);
    assert_eq!(node["from_effective_spec"], true);
    let effective = &node["effective_inputs"];
    assert_eq!(effective["node_id"], id);
    assert_eq!(effective["tree_node_id"], id);
    assert_eq!(effective["spec_node_exact"], true);
    assert_eq!(effective["tree_metatable_exact"], true);
    for (lookup, present) in [
        (
            &effective["modifier_lookup"],
            effective["modifiers"]["present"].as_bool().unwrap(),
        ),
        (
            &effective["keystone_lookup"],
            effective["keystone_mod"] != json!({"kind":"absent"}),
        ),
    ] {
        assert_eq!(lookup["lookup_exact"], true);
        assert_eq!(lookup["effective_present"], present);
        match lookup["origin"].as_str().unwrap() {
            "local" => {
                assert_eq!(lookup["local_present"], true);
                assert!(present);
            }
            "tree_inherited" => {
                assert_eq!(lookup["local_present"], false);
                assert!(present);
            }
            "absent" => {
                assert_eq!(lookup["local_present"], false);
                assert!(!present);
            }
            _ => panic!("unreviewed effective node field origin"),
        }
    }
    assert_eq!(
        node["modifiers"]["present"],
        effective["modifier_lookup"]["local_present"]
    );
    if effective["modifier_lookup"]["local_present"] == true {
        assert_json_equal(
            &node["modifiers"],
            &effective["modifiers"],
            "exact locally overridden node modifiers",
        );
    }
    assert_eq!(
        effective["modifiers"]["count"].as_u64().unwrap() as usize,
        rows(&effective["modifiers"]["records"]).len()
    );
    let returns = rows(&node["build_returns"]);
    assert!((2..=8).contains(&returns.len()));
    assert!(returns.iter().any(|r| r["include_keystone_mods"] == true));
    assert!(
        returns
            .iter()
            .any(|r| r["include_keystone_mods"] == json!({"kind":"absent"}))
    );
    for result in returns {
        assert_eq!(result["node_id"], id);
        assert_eq!(result["caller_line"], 435);
        assert_eq!(result["return_line"], 411);
        assert_eq!(result["exact_allocated_input"], true);
        assert_eq!(result["original_function_return"], true);
        assert_eq!(result["observer_noninterference"], true);
        assert_eq!(result["scratch_supplied"], mode != "MAIN");
        assert!(result["returned_reuses_scratch"].is_boolean());
        assert_json_equal(
            &result["effective_inputs"],
            effective,
            "exact effective node inputs at original return",
        );
        let output = &result["returned_modifiers"];
        assert_eq!(output["present"], true);
        assert_eq!(output["selection"], "all_local_records");
        assert_eq!(
            output["count"].as_u64().unwrap() as usize,
            rows(&output["records"]).len()
        );
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

    fn inherited_node() -> Json {
        let records = json!({"present":true,"count":1,"selection":"all_local_records","records":[
            {"bucket":"sequence","index":1,"name":"MinionModifier","record":{"opaque_test_value":"retained"}}
        ]});
        let effective = json!({"node_id":42,"tree_node_id":42,"node_type":"Normal",
            "spec_node_exact":true,"tree_metatable_exact":true,
            "modifiers":records,"modifier_lookup":{"origin":"tree_inherited","local_present":false,"effective_present":true,"lookup_exact":true},
            "keystone_mod":{"kind":"absent"},"keystone_lookup":{"origin":"absent","local_present":false,"effective_present":false,"lookup_exact":true}});
        let result = |include: Json| {
            json!({"node_id":42,"caller_line":435,"return_line":411,
            "exact_allocated_input":true,"original_function_return":true,"observer_noninterference":true,
            "include_keystone_mods":include,"scratch_supplied":true,"returned_reuses_scratch":true,
            "effective_inputs":effective,"returned_modifiers":records})
        };
        json!({"id":42,"node_id":42,"from_effective_spec":true,
            "modifiers":{"present":false,"count":0,"records":{}},"effective_inputs":effective,
            "build_returns":[result(json!(true)),result(json!({"kind":"absent"}))]})
    }

    #[test]
    fn inherited_node_inputs_remain_present_despite_local_absence() {
        let node = inherited_node();
        validate_node(&node, "CALCS");
        assert_eq!(node["modifiers"]["present"], false);
        assert_eq!(node["effective_inputs"]["modifiers"]["count"], 1);
        assert_eq!(node["build_returns"][1]["returned_modifiers"]["count"], 1);
    }

    #[test]
    fn missing_inheritance_identity_or_original_return_proof_is_rejected() {
        let original = inherited_node();
        let mut missing = original.clone();
        missing["effective_inputs"]["modifier_lookup"]["origin"] = json!("absent");
        let mut identity = original.clone();
        identity["effective_inputs"]["tree_node_id"] = json!(43);
        let mut returned = original.clone();
        returned["build_returns"][0]["node_id"] = json!(43);
        let mut local = original.clone();
        local["modifiers"]["present"] = json!(true);
        let mut absent_pass = original;
        absent_pass["build_returns"].as_array_mut().unwrap().pop();
        for invalid in [missing, identity, returned, local, absent_pass] {
            assert!(std::panic::catch_unwind(|| validate_node(&invalid, "CALCS")).is_err());
        }
    }

    fn amulet_transport() -> (Json, Json) {
        let source: Vec<_> = (0..2)
            .map(|i| json!({"record":expected_amulet_record(i,false,false)}))
            .collect();
        let item = json!({"id":23,"active":{"records":source}});
        let rows: Vec<_> = (0..2).map(|i|json!({"item_id":23,"slot":"Amulet","mode":"MAIN",
            "source_index":i+1,"source_count":2,"caller_line":1667,"add_caller_line":117,
            "factor":0,"round_to_nearest":{"kind":"absent"},"exact_saved_item":true,
            "exact_selected_slot":true,"exact_active_list":true,"exact_receiver":true,"copy_is_distinct":true,
            "original_accessor_preserved":true,"observer_noninterference":true,"inserted_object_exact":true,
            "prior_bucket_objects_preserved":true,"add_return_observed":true,"scale_return_observed":true,
            "source_record_unchanged":true,"observer_requeried_item":false,
            "source_record":expected_amulet_record(i,false,false),"scale_argument":expected_amulet_record(i,true,false),
            "delivered_record":expected_amulet_record(i,true,true),"bucket":if i==0 {"Spirit"}else{"GemProperty"},
            "bucket_count_before":1,"bucket_count_after":2,"inserted_index":2})).collect();
        (Json::Array(rows), item)
    }

    #[test]
    fn zero_factor_retains_both_amulet_records_with_exact_source_identity() {
        let (transport, item) = amulet_transport();
        validate_amulet(&transport, &item, "MAIN");
        assert_eq!(transport.as_array().unwrap().len(), 2);
        assert_ne!(
            transport[1]["source_record"],
            transport[1]["delivered_record"]
        );
    }

    #[test]
    fn missing_zero_delivery_or_changed_amulet_record_and_receiver_are_rejected() {
        let (original, item) = amulet_transport();
        let mut dropped = original.clone();
        dropped.as_array_mut().unwrap().pop();
        let mut factor = original.clone();
        factor[0]["factor"] = json!(1);
        let mut slot = original.clone();
        slot[1]["delivered_record"]["fields"][4]["value"] = json!("Ring 1");
        let mut value = original.clone();
        value[0]["delivered_record"]["fields"][6]["value"] = json!(13);
        let mut receiver = original.clone();
        receiver[0]["exact_receiver"] = json!(false);
        let mut inserted = original;
        inserted[1]["inserted_index"] = json!(1);
        for invalid in [dropped, factor, slot, value, receiver, inserted] {
            assert!(std::panic::catch_unwind(|| validate_amulet(&invalid, &item, "MAIN")).is_err());
        }
    }
}
