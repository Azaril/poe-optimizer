//! One retained original report; source archives authenticate acquisition only.
use super::*;

const PACKET: &str = "data/owned/poe2/3887ae68/minion-preconversion-source";

fn packet_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(PACKET)
}

fn packet() -> (Json, Json) {
    let root = packet_root();
    let metadata: Json =
        serde_json::from_slice(&fs::read(root.join("source-vectors.json")).unwrap()).unwrap();
    let bytes = fs::read(root.join("report.json")).unwrap();
    assert_eq!(metadata["report"]["path"], "report.json");
    assert_eq!(metadata["report"]["bytes"], bytes.len());
    assert_eq!(metadata["report"]["sha256"], digest(&bytes));
    (metadata, serde_json::from_slice(&bytes).unwrap())
}

fn authenticate(metadata: &Json, report: &Json) {
    assert_eq!(metadata["schema_version"], 1);
    assert_eq!(report["schema_version"], 1);
    assert_eq!(metadata["kind"], "original-minion-preconversion-census");
    assert_eq!(metadata["native_coverage"], false);
    assert_eq!(metadata["copied_formula_as_evidence"], false);
    for field in [
        "native_coverage",
        "whole_build_parity",
        "conditional_game_obtainability",
        "later_damage_pipeline",
    ] {
        assert_eq!(report["scope"][field], false, "{field}");
    }
    assert_eq!(report["source_revision"], pinned::UPSTREAM_REVISION);
    assert_eq!(report["manifest_sha256"], pinned::manifest_sha256());
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    assert_eq!(
        report["original_xml_sha256"],
        digest(
            &fs::read(root.join("tests/fixtures/builds/breadth-20260908/build-05.xml")).unwrap()
        )
    );
    assert_eq!(rows(&report["files"]).len(), FILES.len());
    let mut paths = std::collections::BTreeSet::new();
    for pin in rows(&report["files"]) {
        let path = pin["path"].as_str().unwrap();
        assert!(FILES.contains(&path));
        assert!(paths.insert(path));
        assert_eq!(pin["sha256"], pinned::expected_file_sha256(path).unwrap());
    }
    let archives = rows(&metadata["source_archives"]);
    assert_eq!(archives.len(), 4);
    for (pin, (name, source, field)) in archives.iter().zip([
        (
            "observer.lua",
            "crates/poe-optimizer-pob/tests/support/owned_minion_physical_damage_source.lua",
            "observer_sha256",
        ),
        (
            "driver.rs",
            "crates/poe-optimizer-pob/tests/owned_minion_physical_damage_source.rs",
            "driver_sha256",
        ),
        (
            "source-helper.rs",
            "crates/poe-optimizer-pob/tests/support/minion_preconversion_source.rs",
            "source_helper_sha256",
        ),
        (
            "bootstrap.rs",
            "crates/poe-optimizer-pob/tests/support/configuration_preparation_source.rs",
            "bootstrap_sha256",
        ),
    ]) {
        assert_eq!(pin["path"], format!("evidence/{name}"));
        assert_eq!(pin["source_path"], source);
        assert_eq!(pin["executable"], false);
        let bytes = fs::read(packet_root().join("evidence").join(name)).unwrap();
        assert_eq!(pin["bytes"], bytes.len());
        assert_eq!(pin["sha256"], digest(&bytes));
        assert_eq!(pin["sha256"], report[field]);
    }
    let reports = rows(&metadata["reports"]);
    assert_eq!(reports.len(), 2);
    for (pin, mode) in reports.iter().zip(["off", "on"]) {
        assert_eq!(pin["jit_mode"], mode);
        assert_eq!(pin["bytes"], metadata["report"]["bytes"]);
        assert_eq!(pin["sha256"], metadata["report"]["sha256"]);
        assert!(
            pin["path"]
                .as_str()
                .unwrap()
                .ends_with(&format!("/source-jit-{mode}.json"))
        );
    }
    for field in [
        "original_base_locals",
        "raw_zero_and_conditional_records",
        "original_pass_assembly",
        "unhooked_jit_mode_from_filename",
        "unhooked_controls",
    ] {
        assert_eq!(report["capture"][field], true, "{field}");
    }
    let original =
        fs::read_to_string(root.join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap();
    let expected = super::cases(&original);
    assert_eq!(rows(&report["cases"]).len(), expected.len());
    for (case, expected) in rows(&report["cases"]).iter().zip(expected) {
        assert_eq!(case["name"], expected.name);
        assert_eq!(case["xml_sha256"], digest(expected.xml.as_bytes()));
        assert_eq!(
            case["warm_xml_sha256"],
            json!(expected.warm.map(|xml| digest(xml.as_bytes())))
        );
        for field in [
            "original_functions_preserved",
            "loaded_state_preserved",
            "cached_outputs_preserved",
            "saved_specs_preserved",
            "fresh_actor_construction",
            "query_state_preserved",
        ] {
            assert_eq!(case["state"][field], true, "{field}");
        }
        assert_eq!(case["state"]["source_actor_level_mutated"], false);
        assert_eq!(case["state"]["business_method_wrappers"], false);
        for mode in ["main", "calcs"] {
            let actor = rows(&case["state"][mode]["actors"])
                .iter()
                .find(|a| a["actor_profile"] == "RaisedSkeletonSniper")
                .unwrap();
            assert_eq!(
                actor["source_occurrence"],
                json!({"source_present":true,"matches":[{
                    "enable_global_1":true,"enable_global_2":true,"gem_id":"Metadata/Items/Gems/SkillGemSkeletalSniper",
                    "gem_index":1,"group_enabled":true,"group_index":3,"group_is_active_socket_group":true,
                    "group_label":"","physical_gem_id":"Metadata/Items/Gems/SkillGemSkeletalSniper",
                    "raw_level":20,"raw_quality":0,"source_enabled":true
                }]})
            );
            let child = rows(&actor["children"])
                .iter()
                .find(|s| s["effect_id"] == "MinionMeleeBow")
                .unwrap();
            if mode == "calcs" && case["name"] != "sniper-calcs-effective" {
                assert!(
                    child.get("base_calls").is_none(),
                    "unexecuted is not an observed zero"
                );
                continue;
            }
            for call in rows(&child["base_calls"])
                .iter()
                .filter(|c| c["damage_type"] == "Physical")
            {
                let selection = &call["preconversion"]["selection"];
                assert_eq!(selection["weapon"], selection["source"]);
                assert_eq!(selection["weapon"], actor["weapon1"]);
                assert_eq!(selection["source_is_actor_skill_data"], false);
                let raw = rows(&call["preconversion"]["raw"]["skill"]);
                let more: Vec<_> = raw
                    .iter()
                    .filter(|r| r["mod"]["name"] == "AddedDamage")
                    .collect();
                assert_eq!(more.len(), 1);
                assert_eq!(
                    more[0],
                    &json!({"ancestor_depth":0,"mod":{
                        "flags":1,"keyword_flags":0,"name":"AddedDamage","source":"Skeletal Sniper Damage Multiplier",
                        "tags":[{"neg":true,"partialMatch":true,"skillNameList":["Spectre","Companion"],"summonSkill":true,"type":"SkillName"}],
                        "type":"MORE","value":14.999999999999991
                    }})
                );
                for (name, value) in [("PhysicalMin", 3), ("PhysicalMax", 7)] {
                    for record in raw.iter().filter(|r| r["mod"]["name"] == name) {
                        let conditional =
                            case["name"].as_str().unwrap().starts_with("conditional-");
                        let tags = if conditional {
                            json!([{"type":"Condition","var":"FullLife"}])
                        } else {
                            json!({})
                        };
                        let value = if case["name"] == "zero-flat-physical" {
                            0
                        } else {
                            value
                        };
                        assert_eq!(
                            record,
                            &json!({"ancestor_depth":1,"mod":{
                                "flags":0,"keyword_flags":0,"name":name,"source":"Custom:Physical damage source control",
                                "tags":tags,"type":"BASE","value":value
                            }})
                        );
                    }
                }
            }
        }
    }
    super::check(report);
}

#[test]
fn retained_preconversion_census_authenticates_original_report_and_sources() {
    let (metadata, report) = packet();
    authenticate(&metadata, &report);
}

#[test]
fn retained_preconversion_census_rejects_substituted_source_and_build() {
    let (metadata, report) = packet();
    for field in [
        "source_revision",
        "manifest_sha256",
        "original_xml_sha256",
        "observer_sha256",
        "driver_sha256",
        "source_helper_sha256",
        "bootstrap_sha256",
    ] {
        let mut changed = report.clone();
        changed[field] = json!("substituted");
        assert!(
            std::panic::catch_unwind(|| authenticate(&metadata, &changed)).is_err(),
            "{field}"
        );
    }
    let mut changed = report.clone();
    changed["files"].as_array_mut().unwrap().pop();
    assert!(std::panic::catch_unwind(|| authenticate(&metadata, &changed)).is_err());
}

#[test]
fn retained_preconversion_census_rejects_acquisition_or_native_coverage_claims() {
    let (metadata, report) = packet();
    let mut changed = report.clone();
    changed["schema_version"] = json!(2);
    assert!(std::panic::catch_unwind(|| authenticate(&metadata, &changed)).is_err());
    for field in [
        "native_coverage",
        "whole_build_parity",
        "conditional_game_obtainability",
        "later_damage_pipeline",
    ] {
        let mut changed = report.clone();
        changed["scope"][field] = json!(true);
        assert!(
            std::panic::catch_unwind(|| authenticate(&metadata, &changed)).is_err(),
            "{field}"
        );
    }
    for field in ["native_coverage", "copied_formula_as_evidence"] {
        let mut changed = metadata.clone();
        changed[field] = json!(true);
        assert!(
            std::panic::catch_unwind(|| authenticate(&changed, &report)).is_err(),
            "{field}"
        );
    }
    let mut changed = metadata.clone();
    changed["reports"][1]["sha256"] = json!("different-mode");
    assert!(std::panic::catch_unwind(|| authenticate(&changed, &report)).is_err());
    let mut changed = metadata.clone();
    changed["source_archives"][0]["executable"] = json!(true);
    assert!(std::panic::catch_unwind(|| authenticate(&changed, &report)).is_err());
}

#[test]
fn retained_preconversion_census_requires_exact_controls_and_preservation() {
    let (metadata, report) = packet();
    for field in ["name", "xml_sha256", "warm_xml_sha256"] {
        let mut changed = report.clone();
        changed["cases"][4][field] = json!("substituted");
        assert!(std::panic::catch_unwind(|| authenticate(&metadata, &changed)).is_err());
    }
    for field in [
        "original_base_locals",
        "raw_zero_and_conditional_records",
        "original_pass_assembly",
        "unhooked_controls",
    ] {
        let mut changed = report.clone();
        changed["capture"][field] = json!(false);
        assert!(std::panic::catch_unwind(|| authenticate(&metadata, &changed)).is_err());
    }
    for field in [
        "original_functions_preserved",
        "cached_outputs_preserved",
        "query_state_preserved",
    ] {
        let mut changed = report.clone();
        changed["cases"][4]["state"][field] = json!(false);
        assert!(std::panic::catch_unwind(|| authenticate(&metadata, &changed)).is_err());
    }
}

fn census_mut(report: &mut Json, case: usize) -> &mut Json {
    let actor = report["cases"][case]["state"]["main"]["actors"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|a| a["actor_profile"] == "RaisedSkeletonSniper")
        .unwrap();
    let child = actor["children"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|s| s["effect_id"] == "MinionMeleeBow")
        .unwrap();
    let call = child["base_calls"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["damage_type"] == "Physical")
        .unwrap();
    &mut call["preconversion"]
}

#[test]
fn retained_preconversion_census_rejects_changed_predicates_ancestry_and_source() {
    let (metadata, report) = packet();
    for field in ["flags", "keyword_flags"] {
        let mut changed = report.clone();
        let census = census_mut(&mut changed, 4);
        let record = census["raw"]["skill"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r["mod"]["name"] == "AddedDamage")
            .unwrap();
        record["mod"][field] = json!(128);
        assert!(std::panic::catch_unwind(|| authenticate(&metadata, &changed)).is_err());
    }
    let mut changed = report.clone();
    let census = census_mut(&mut changed, 6);
    let record = census["raw"]["skill"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["mod"]["name"] == "PhysicalMin")
        .unwrap();
    record["mod"]["tags"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type":"Condition","var":"ExtraInactivePredicate"}));
    assert!(std::panic::catch_unwind(|| authenticate(&metadata, &changed)).is_err());
    let mut changed = report.clone();
    let census = census_mut(&mut changed, 5);
    let record = census["raw"]["skill"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["mod"]["name"] == "PhysicalMin")
        .unwrap();
    record["ancestor_depth"] = json!(0);
    assert!(std::panic::catch_unwind(|| authenticate(&metadata, &changed)).is_err());
    for field in ["weapon", "source_is_actor_skill_data"] {
        let mut changed = report.clone();
        let selection = &mut census_mut(&mut changed, 0)["selection"];
        if field == "weapon" {
            selection[field]["PhysicalMin"] = json!(209);
        } else {
            selection[field] = json!(true);
        }
        assert!(std::panic::catch_unwind(|| authenticate(&metadata, &changed)).is_err());
    }
}

#[test]
#[ignore = "requires the two complete local preconversion acquisition reports"]
fn retained_preconversion_census_matches_both_complete_local_reports() {
    let (metadata, report) = packet();
    authenticate(&metadata, &report);
    let retained = fs::read(packet_root().join("report.json")).unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for pin in rows(&metadata["reports"]) {
        let bytes = fs::read(root.join(pin["path"].as_str().unwrap())).unwrap();
        assert!(bytes == retained, "full report differs: {}", pin["path"]);
    }
}
