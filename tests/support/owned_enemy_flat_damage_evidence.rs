//! Pure offline admission checker for the pinned enemy flat-physical domain.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::PathBuf};
const SELECTION: &str = "data/owned/poe2/3887ae68/minion-attack-selection/source-evidence.json";
const ADDED: &str = "data/owned/poe2/3887ae68/combined-added-attack-damage/source-evidence.json";
// Exact reviewed factories and transports; changed code requires a new review.
const REVIEWED_LEDGER: &str = "83bdd2316a727f2d35bd416c7df832f0729fc68e9aae761ea9e3cbe33c833495";
const REVIEWED_EXPANDED: &str = "a7d333d7ca6e499d21044461b191494d9a3a3b36e1fb9f5f933eb654a2506804";
const REVIEWED_BOSSES: &str = "a409503dd65854d4a9b32e6dc58ae764cbd3c5695c67dbf6c5e5271ae9a2972f";
fn root() -> PathBuf {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if p.join("data/owned").is_dir() {
        p
    } else {
        p.join("../..")
    }
}
fn digest(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn fingerprint(v: &Value) -> String {
    digest(&serde_json::to_vec(v).unwrap())
}
pub fn dependencies() -> Value {
    json!([
        {"path":SELECTION,"sha256":"a2daf1e5c6c058ae2ce32f434feee325d06344a96a5ccbf29ed7320df2c63ec3"},
        {"path":ADDED,"sha256":"6c33fe128a0f6622785e70ef26843d47b8da33cc652f29ccf11246180e34fd45"}
    ])
}
fn dependency(path: &str) -> Value {
    let b = std::fs::read(root().join(path)).unwrap();
    let deps = dependencies();
    let row = deps
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["path"] == path)
        .unwrap();
    assert_eq!(digest(&b), row["sha256"].as_str().unwrap());
    serde_json::from_slice(&b).unwrap()
}
pub fn source_files_digest() -> String {
    fingerprint(&dependency(ADDED)["files"])
}
pub fn check_semantics(v: &Value) {
    assert_eq!(v["schema_version"], 1);
    assert_eq!(v["kind"], "pinned-constructed-enemy-flat-physical-domain");
    assert_eq!(
        v["upstream_revision"],
        "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4"
    );
    assert_eq!(
        v["scope"],
        json!({"channels":["SelfPhysicalMin","SelfPhysicalMax"],"exact_intrinsic_basic_source":true,"hostile_minions_admitted":false,"imported_party_admitted":false,"requires_accounted_inputs":true,"native_runtime_or_coverage_change":false})
    );
    // Raw zero, inactive, nested and unselected candidates are not absence.
    assert_eq!(
        v["target_modifiers"],
        json!([]),
        "actual constructed target modifier exists"
    );
    for row in v["boss_additional_stats"].as_array().unwrap() {
        assert!(matches!(row["mode"].as_str().unwrap(), "base" | "uber"));
        for (name, value) in row["stats"].as_object().unwrap() {
            match name.as_str() {
                "CannotBeDodged" | "CannotBeEvaded" | "CannotBeSuppressed" | "CannotBeBlocked" => {
                    assert_eq!(value, "flag")
                }
                "PhysicalDamageSkillConvertToFire"
                | "PhysicalDamageSkillConvertToLightning"
                | "PhysicalDamageSkillConvertToChaos"
                | "PhysicalDamageSkillConvertToCold" => {
                    assert!(value.as_f64().is_some_and(|v| v == 25.0 || v == 100.0))
                }
                _ => panic!("unreviewed actual BossSkills additional modifier {name}"),
            }
        }
    }
    let names = v["expanded_census"]["self_modifier_names"]
        .as_object()
        .unwrap();
    assert!(!names.contains_key("SelfPhysicalMin") && !names.contains_key("SelfPhysicalMax"));
    assert_eq!(fingerprint(&v["expanded_census"]), REVIEWED_EXPANDED);
    assert_eq!(fingerprint(&v["boss_additional_stats"]), REVIEWED_BOSSES);
    let prior = dependency(ADDED);
    assert_eq!(v["dependencies"], dependencies());
    for name in ["raw_tables", "raw_rows"] {
        assert_eq!(v["catalog"][name], prior["catalog"][name]);
        assert_eq!(v["cache_census"][name], prior["cache_census"][name]);
        assert!(
            v["expanded_census"][name].as_u64().unwrap()
                > v["cache_census"][name].as_u64().unwrap()
        );
    }
    for name in ["aliases", "callbacks", "metatables"] {
        assert_eq!(
            v["catalog"][format!("{name}_sha256")],
            fingerprint(&prior["catalog"][name])
        );
    }
    assert_eq!(
        v["cache_census"]["aliases_sha256"],
        fingerprint(&prior["cache_census"]["aliases"])
    );
    assert_eq!(
        v["cache_census"]["modifiers"],
        prior["cache_census"]["modifiers"]
    );
    assert_eq!(v["module_order"], prior["module_order"]);
    assert_eq!(v["selected"], prior["catalog"]["selected"]);
    assert_eq!(v["full_profile"], prior["catalog"]["full_profile"]);
    assert!(
        v["full_profile"].get("hostile").is_none(),
        "selected Actor must use environment Enemy, not Player"
    );
    assert_eq!(v["corroboration"], prior["selected_pass_corroboration"]);
    assert_eq!(v["name_ledger_sha256"], fingerprint(&v["name_ledger"]));
    assert_eq!(
        v["name_ledger_sha256"], REVIEWED_LEDGER,
        "changed factory/transport/name assignment ledger requires review"
    );
}
pub fn check(v: &Value, full: bool) {
    check_semantics(v);
    assert_eq!(v["source_files_sha256"], source_files_digest());
    let prior = dependency(ADDED);
    let manifest =
        std::fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json"))
            .unwrap();
    assert_eq!(v["source_manifest_sha256"], digest(&manifest));
    if full {
        let files: BTreeMap<String, String> =
            serde_json::from_value(prior["files"].clone()).unwrap();
        for (path, expected) in files {
            let text =
                std::fs::read_to_string(root().join("vendor/path-of-building-poe2").join(path))
                    .unwrap()
                    .replace("\r\n", "\n");
            assert_eq!(digest(text.as_bytes()), expected);
        }
    }
    let c = &v["corroboration"];
    let b = std::fs::read(root().join(c["path"].as_str().unwrap())).unwrap();
    assert_eq!(c["bytes"], b.len());
    assert_eq!(c["sha256"], digest(&b));
    let report: Value = serde_json::from_slice(&b).unwrap();
    let mut observed = 0;
    for case in report["cases"].as_array().unwrap() {
        for mode in ["main", "calcs"] {
            for actor in case["state"][mode]["actors"].as_array().unwrap() {
                if actor["summon_effect_id"] != "SummonSkeletalSnipersPlayer" {
                    continue;
                }
                assert_eq!(actor["actor_profile"], "RaisedSkeletonSniper");
                assert_eq!(actor["hostile"], false);
                for child in actor["children"].as_array().unwrap() {
                    if child["effect_id"] != "MinionMeleeBow" {
                        continue;
                    }
                    // Unexecuted CALCS carries no original base call; it never
                    // proves an empty result or contributes an observed count.
                    let Some(calls) = child["base_calls"].as_array() else {
                        assert_eq!(mode, "calcs");
                        assert!(child.get("base_calls").is_none());
                        assert!(child.get("passes").is_none());
                        assert_eq!(child["output"], json!({}));
                        continue;
                    };
                    for call in calls {
                        if call["damage_type"] != "Physical" {
                            continue;
                        }
                        let p = &call["preconversion"];
                        assert_eq!(call["query_state_preserved"], true);
                        for key in [
                            "exact_actor_store",
                            "exact_enemy_store",
                            "exact_parent",
                            "exact_pass_cfg",
                            "exact_pass_source",
                            "exact_summoner",
                        ] {
                            assert_eq!(p[key], true);
                        }
                        assert_eq!(p["raw"]["enemy"], json!({}));
                        for (key, name) in [
                            ("enemy_minimum", "SelfPhysicalMin"),
                            ("enemy_maximum", "SelfPhysicalMax"),
                        ] {
                            assert_eq!(call[key], json!({"names":[name],"records":{},"value":0}));
                        }
                        observed += 1;
                    }
                }
            }
        }
    }
    assert_eq!(
        observed, 9,
        "eight MAIN and one effective CALCS observations"
    );
}
#[test]
fn retained_enemy_proof_refuses_fabricated_absence_and_scope_widening() {
    let b = std::fs::read(
        root().join("data/owned/poe2/3887ae68/enemy-flat-attack-damage/source-evidence.json"),
    )
    .unwrap();
    let original: Value = serde_json::from_slice(&b).unwrap();
    check(&original, false);
    for edit in 0..7 {
        let mut changed = original.clone();
        match edit {
            0 => {
                changed["target_modifiers"] = json!([{"path":"real-test-candidate","record":{"name":"SelfPhysicalMin","type":"BASE","value":0}}])
            }
            1 => changed["scope"]["imported_party_admitted"] = json!(true),
            2 => changed["full_profile"]["hostile"] = json!(false),
            3 => changed["boss_additional_stats"][0]["stats"]["SelfPhysicalMax"] = json!(0),
            4 => {
                changed["name_ledger"]["dynamic_factories"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
                changed["name_ledger_sha256"] = json!(fingerprint(&changed["name_ledger"]));
            }
            5 => {
                changed["expanded_census"]["callbacks"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            6 => {
                changed["boss_additional_stats"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            _ => unreachable!(),
        }
        assert!(std::panic::catch_unwind(|| check_semantics(&changed)).is_err());
    }
}
