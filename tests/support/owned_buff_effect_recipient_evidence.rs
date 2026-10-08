//! Retained recipient observations, never evaluator inputs. Empty-domain support
//! is narrower than these source controls: nonempty custom sources are recorded
//! solely to require refusal until their origins and composition are authored.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

const REPORT_HASH: &str = "030f14d71a01a2c54862eb858249b2abca1ba1caa9337dbffb3458118777e935";
const OBSERVER_HASH: &str = "73ff0ee76552730818460ab93f5b3940bc612165c54e6bac6ab02faa7c54dcd8";
const CASES: [(&str, &str); 22] = [
    ("original-05", "main"),
    ("original-05", "calcs"),
    ("repeat-original-05", "main"),
    ("warm-calcs-to-original", "main"),
    ("sniper-calcs-effective", "calcs"),
    ("offering-disabled", "main"),
    ("sniper-calcs-unbuffed", "calcs"),
    ("offering-level-1", "main"),
    ("offering-duplicate-equal", "main"),
    ("offering-higher-first", "main"),
    ("offering-higher-last", "main"),
    ("offering-source-buff-effect", "main"),
    ("offering-recipient-buff-effect", "main"),
    ("offering-both-buff-effects", "main"),
    ("offering-positive-half-tie", "main"),
    ("offering-negative-half-tie", "main"),
    ("offering-combined-more", "main"),
    ("offering-magnitude", "main"),
    ("two-recipients-original-main-clone-calcs", "main"),
    ("two-recipients-original-main-clone-calcs", "calcs"),
    ("two-recipients-clone-main-original-calcs", "main"),
    ("two-recipients-clone-main-original-calcs", "calcs"),
];
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn read() -> Value {
    serde_json::from_slice(
        &fs::read(
            root().join("data/owned/poe2/3887ae68/buff-effect-recipients/source-vectors.json"),
        )
        .unwrap(),
    )
    .unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn rows(value: &Value) -> &[Value] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(value.as_object().is_some_and(|object| object.is_empty()));
        &[]
    }
}
fn one<'a>(rows: &'a [Value], field: &str, value: &Value) -> &'a Value {
    let found: Vec<_> = rows.iter().filter(|row| row[field] == *value).collect();
    assert_eq!(found.len(), 1, "unique {field}={value}");
    found[0]
}
fn pick(value: &Value, fields: &[&str]) -> Value {
    Value::Object(
        fields
            .iter()
            .map(|&key| (key.into(), value[key].clone()))
            .collect(),
    )
}
fn projection(case: &Value, mode: &str) -> Value {
    let state = &case["state"];
    assert_eq!(case["available"], true);
    for field in [
        "cached_outputs_preserved",
        "loaded_state_preserved",
        "original_functions_preserved",
        "query_state_preserved",
        "saved_specs_preserved",
        "fresh_actor_construction",
    ] {
        assert_eq!(state[field], true);
    }
    assert_eq!(state["source_actor_level_mutated"], false);
    assert_eq!(state["business_method_wrappers"], false);
    let env = &state[mode];
    let invocations: Vec<_> = rows(&env["offering_merge_events"]).iter().map(|event| {
        let skill = one(rows(&env["skills"]), "source_occurrence", &event["source_occurrence"]);
        assert_eq!(skill["effect_id"], "PainOfferingPlayer");
        let buffs: Vec<_> = rows(&skill["buffs"]).iter().filter(|buff| buff["scaling"]["recipient_occurrence"] == event["recipient_occurrence"]).collect();
        assert_eq!(buffs.len(), 1);
        let buff = buffs[0];
        let scaling = &buff["scaling"];
        json!({
            "source_effect":skill["effect_id"], "source_occurrence":skill["source_occurrence"],
            "physical_level":skill["physical_level"], "effective_level":skill["effective_level"],
            "buff_fields":buff["fields"], "raw_modifiers":buff["modifiers"],
            "source_store_is_skill":scaling["source_store_is_skill"],
            "recipient_profile":scaling["recipient_profile"], "recipient_hostile":scaling["recipient_hostile"],
            "recipient_occurrence":scaling["recipient_occurrence"],
            "recipient_increased":scaling["recipient_increased"], "recipient_more":scaling["recipient_more"],
            "source_scaling":pick(scaling,&["source_buff_increased","source_buff_more","source_magnitude_increased","source_magnitude_more"]),
            "source_cfg":pick(&scaling["source_cfg"],&["present","fields","skill_gem_is_source","granted_effect_is_source","granted_effect","skill_gem_tags"]),
            "merge_event":event
        })
    }).collect();
    let kind = if invocations.is_empty() {
        "no-invocation"
    } else if invocations[0]["recipient_profile"] != "RaisedSkeletonSniper" {
        "other-recipient-diagnostic"
    } else if invocations.iter().any(|i| {
        !rows(&i["recipient_increased"]["records"]).is_empty()
            || !rows(&i["recipient_more"]["records"]).is_empty()
    }) {
        "recipient-nonempty-unsupported"
    } else {
        "sniper-empty-domain"
    };
    let recipients: Vec<_> = rows(&env["actors"])
        .iter()
        .filter(|actor| {
            actor["summon_effect_id"] == "SummonSkeletalSnipersPlayer"
                || actor["is_environment_minion"] == true
        })
        .map(|actor| {
            pick(
                actor,
                &[
                    "ordinal",
                    "actor_profile",
                    "summon_effect_id",
                    "physical_level",
                    "effective_level",
                    "quality",
                    "source_occurrence",
                    "is_environment_minion",
                ],
            )
        })
        .collect();
    json!({"case":case["name"],"mode":mode,"xml_sha256":case["xml_sha256"],"warm_xml_sha256":case["warm_xml_sha256"],"observation_kind":kind,"main_group":env["main_group"],"recipients":recipients,"selected_recipient":env["selected_minion"],"invocations":invocations})
}

fn channel(channel: &Value, kind: &str, expected: f64, raw: Option<f64>) {
    assert_eq!(channel["names"], json!(["BuffEffectOnSelf"]));
    assert_eq!(channel["value"].as_f64(), Some(expected));
    let records = rows(&channel["records"]);
    if let Some(raw) = raw {
        assert_eq!(records.len(), 1);
        let record = &records[0];
        assert_eq!(record["value"].as_f64(), Some(raw));
        let modifier = &record["mod"];
        assert_eq!(modifier["value"].as_f64(), Some(raw));
        assert_eq!(modifier["name"], "BuffEffectOnSelf");
        assert_eq!(modifier["type"], kind);
        assert_eq!(modifier["source"], "Custom:Physical damage source control");
        assert_eq!(modifier["flags"], 0);
        assert_eq!(modifier["keyword_flags"], 0);
        assert!(rows(&modifier["tags"]).is_empty());
    } else {
        assert!(
            records.is_empty(),
            "neutral support requires an empty domain, not cancellation"
        );
    }
}

/// Ordinary checks authenticate the committed compact observations and source
/// manifest. Full publication also compares the projection to both retained
/// byte-identical reports. The historical observer Git identity is provenance;
/// neither mode invokes Git, Lua, or a current observer against old report bytes.
pub fn check(proof: &Value, full: bool) {
    assert_eq!(proof["schema_version"], 1);
    assert_eq!(proof["status"], "passed");
    assert_eq!(
        proof["scope"],
        json!({
            "supported_domain":"complete-empty-recipient-increase-and-multiply-inventories",
            "recipients":"exact-current-Sniper-Actor-occurrences",
            "publication_actor_owner_complete":false,"publication_global_query_registry_complete":false,
            "nonempty_composition_supported":false,"game_source_admission":false,
            "whole_build_parity":false,"source_observations_are_native_inputs":false,
            "other_recipient_rows":"diagnostics-only"
        })
    );
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(hash(&manifest_bytes), proof["source_manifest_sha256"]);
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(manifest["upstream_revision"], proof["source_revision"]);
    let mut files = BTreeSet::new();
    for pin in rows(&proof["source_files"]) {
        assert!(files.insert(pin["path"].as_str().unwrap()));
        assert_eq!(one(rows(&manifest["files"]), "path", &pin["path"]), pin);
    }
    assert_eq!(
        files,
        [
            "src/Modules/CalcPerform.lua",
            "src/Classes/ModStore.lua",
            "src/Modules/Common.lua",
            "src/Data/Skills/act_int.lua",
            "src/Modules/CalcActiveSkill.lua"
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(proof["observer"]["sha256"], OBSERVER_HASH);
    assert_eq!(proof["observer"]["bytes"], 28314);
    assert_eq!(
        proof["observer"]["git_blob"],
        "e44717d81fb401b13a3b6b60445f1df098c82f65"
    );
    assert_eq!(
        proof["observer"]["git_commit"],
        "b99c7f1a7692c7eaaae5ccd076d0beaaa16d08ca"
    );
    assert_eq!(
        proof["observer"]["path"],
        "crates/poe-optimizer-pob/tests/support/owned_minion_physical_damage_source.lua"
    );
    assert_eq!(
        proof["source_test"]["git_commit"],
        proof["observer"]["git_commit"]
    );
    assert_eq!(
        proof["source_test"]["path"],
        "crates/poe-optimizer-pob/tests/owned_minion_physical_damage_source.rs"
    );
    assert_eq!(proof["source_test"]["bytes"], 61336);
    assert_eq!(
        proof["source_test"]["git_blob"],
        "86bdb5e3cc77d3e4ee217fbcf1b1f1db81a4fbe8"
    );
    assert_eq!(
        proof["source_test"]["sha256"],
        "8c928f2051fb994a975c62e904b046bc2f5fcb8ed9c1d7799b23c9258f9f93e9"
    );
    assert_eq!(proof["vector_count"], CASES.len());
    let vectors = rows(&proof["vectors"]);
    assert_eq!(vectors.len(), CASES.len());
    let mut total_invocations = 0;
    for (vector, (case, mode)) in vectors.iter().zip(CASES) {
        assert_eq!(vector["case"], case);
        assert_eq!(vector["mode"], mode);
        assert_eq!(vector["xml_sha256"].as_str().unwrap().len(), 64);
        let invocations = rows(&vector["invocations"]);
        total_invocations += invocations.len();
        let duplicate_sources = matches!(
            case,
            "offering-duplicate-equal" | "offering-higher-first" | "offering-higher-last"
        );
        let disabled = matches!(case, "offering-disabled" | "sniper-calcs-unbuffed");
        assert_eq!(
            invocations.len(),
            if disabled {
                0
            } else if duplicate_sources {
                2
            } else {
                1
            }
        );
        let selected = rows(&vector["selected_recipient"]["matches"]);
        assert_eq!(selected.len(), 1);
        let actor = one(
            rows(&vector["recipients"]),
            "ordinal",
            &selected[0]["ordinal"],
        );
        assert_eq!(actor["is_environment_minion"], true);
        assert_eq!(actor["source_occurrence"], selected[0]["source"]);
        let other = case == "original-05" && mode == "calcs";
        assert_eq!(
            actor["actor_profile"],
            if other {
                "RaisedSkeletonArsonist"
            } else {
                "RaisedSkeletonSniper"
            }
        );
        let (inc, inc_raw, more, more_raw) = match case {
            "offering-recipient-buff-effect" | "offering-both-buff-effects" => {
                (25.0, Some(25.0), 1.0, None)
            }
            "offering-positive-half-tie" | "offering-negative-half-tie" => {
                (0.0, None, 0.43, Some(-57.0))
            }
            "offering-combined-more" => (0.0, None, 1.5, Some(50.0)),
            _ => (0.0, None, 1.0, None),
        };
        let nonempty = inc_raw.is_some() || more_raw.is_some();
        let kind = if disabled {
            "no-invocation"
        } else if other {
            "other-recipient-diagnostic"
        } else if nonempty {
            "recipient-nonempty-unsupported"
        } else {
            "sniper-empty-domain"
        };
        assert_eq!(vector["observation_kind"], kind);
        let mut source_occurrences = BTreeSet::new();
        for invocation in invocations {
            assert!(source_occurrences.insert(invocation["source_occurrence"].to_string()));
            assert_eq!(invocation["source_effect"], "PainOfferingPlayer");
            assert_eq!(invocation["source_store_is_skill"], true);
            assert_eq!(invocation["recipient_hostile"], false);
            assert_eq!(invocation["recipient_profile"], actor["actor_profile"]);
            assert_eq!(
                invocation["recipient_occurrence"],
                vector["selected_recipient"]
            );
            for field in ["activeSkillBuff", "applyMinions", "applyNotPlayer"] {
                assert_eq!(invocation["buff_fields"][field], true);
            }
            assert_eq!(invocation["source_cfg"]["present"], true);
            assert_eq!(invocation["source_cfg"]["skill_gem_is_source"], true);
            assert_eq!(invocation["source_cfg"]["granted_effect_is_source"], true);
            assert_eq!(
                invocation["source_cfg"]["granted_effect"]["id"],
                "PainOfferingPlayer"
            );
            channel(&invocation["recipient_increased"], "INC", inc, inc_raw);
            channel(&invocation["recipient_more"], "MORE", more, more_raw);
            let event = &invocation["merge_event"];
            assert_eq!(event["minion_destination"], true);
            assert_eq!(event["source_effect"], invocation["source_effect"]);
            assert_eq!(event["source_level"], invocation["effective_level"]);
            assert_eq!(event["source_occurrence"], invocation["source_occurrence"]);
            assert_eq!(
                event["recipient_occurrence"],
                invocation["recipient_occurrence"]
            );
            assert_eq!(event["recipient_profile"], invocation["recipient_profile"]);
        }
        if case.starts_with("two-recipients-") {
            let actors = rows(&vector["recipients"]);
            assert_eq!(actors.len(), 2);
            let qualities: BTreeSet<_> = actors
                .iter()
                .map(|a| a["quality"].as_u64().unwrap())
                .collect();
            assert_eq!(qualities, [0, 20].into_iter().collect());
            let sources: BTreeSet<_> = actors
                .iter()
                .map(|a| a["source_occurrence"].to_string())
                .collect();
            assert_eq!(sources.len(), 2);
            let clone = (case == "two-recipients-clone-main-original-calcs") == (mode == "main");
            assert_eq!(actor["quality"], if clone { 20 } else { 0 });
        }
    }
    assert_eq!(total_invocations, 23);
    assert_eq!(rows(&proof["reports"]).len(), 2);
    let mut first_report = None;
    for (pin, mode) in rows(&proof["reports"]).iter().zip(["off", "on"]) {
        assert_eq!(
            pin["path"],
            format!("runs/owned-minion-physical-damage-source-02/source-jit-{mode}.json")
        );
        assert_eq!(pin["bytes"], 19_612_949);
        assert_eq!(pin["sha256"], REPORT_HASH);
        if full {
            let bytes = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
            assert_eq!(bytes.len(), 19_612_949);
            assert_eq!(hash(&bytes), REPORT_HASH);
            if let Some(first) = &first_report {
                assert_eq!(&bytes, first);
            } else {
                first_report = Some(bytes.clone());
            }
            let report: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(report["source_revision"], proof["source_revision"]);
            assert_eq!(report["source_hash"], proof["source_manifest_sha256"]);
            assert_eq!(
                report["evidence"]["observer_sha256"],
                proof["observer"]["sha256"]
            );
            assert_eq!(rows(&report["cases"]).len(), 37);
            assert_eq!(report["evidence"]["complete_load_attempts_per_jit"], 38);
            for pin in rows(&proof["source_files"]) {
                assert_eq!(
                    one(rows(&report["evidence"]["files"]), "path", &pin["path"])["sha256"],
                    pin["sha256"]
                );
            }
            let projected: Vec<_> = CASES
                .iter()
                .map(|(case, mode)| {
                    projection(one(rows(&report["cases"]), "name", &json!(case)), mode)
                })
                .collect();
            assert_eq!(
                projected, vectors,
                "exact current packet projection of retained report"
            );
        }
    }
}

/// Two actual Sniper environments are checked. Original05 CALCS selected an
/// Arsonist; it is deliberately excluded from this narrowly named assertion.
pub fn assert_empty_baseline() {
    let proof = read();
    check(&proof, false);
    for (case, mode) in [("original-05", "main"), ("sniper-calcs-effective", "calcs")] {
        let found: Vec<_> = rows(&proof["vectors"])
            .iter()
            .filter(|v| v["case"] == case && v["mode"] == mode)
            .collect();
        assert_eq!(found.len(), 1);
        let row = found[0];
        assert_eq!(row["observation_kind"], "sniper-empty-domain");
        let invocations = rows(&row["invocations"]);
        assert_eq!(invocations.len(), 1);
        assert_eq!(invocations[0]["recipient_profile"], "RaisedSkeletonSniper");
        channel(&invocations[0]["recipient_increased"], "INC", 0.0, None);
        channel(&invocations[0]["recipient_more"], "MORE", 1.0, None);
    }
}
