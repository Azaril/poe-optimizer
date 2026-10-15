//! Offline data and retained original-source proof; no Lua execution.
#[allow(dead_code)]
#[path = "support/owned_configuration_default_sources.rs"]
mod configuration_sources;
#[allow(dead_code)]
#[path = "support/owned_configuration_default_stage.rs"]
mod configuration_stage;
#[allow(dead_code)]
#[path = "support/owned_configuration_distance_inputs.rs"]
mod distance;
use serde_json::json;

#[test]
fn two_distance_inputs_preserve_distinct_source_meanings_and_complete_receipts() {
    distance::check_authored();
}

#[test]
fn distance_source_identity_and_body_tampering_are_rejected() {
    let original = distance::read::<serde_json::Value>("source-evidence.json");
    for mutation in 0..5 {
        let mut e = original.clone();
        match mutation {
            0 => e["source_revision"] = json!("wrong"),
            1 => e["spans"][0]["body"] = json!("wrong"),
            2 => e["archives"][0]["path"] = json!("src/Modules/CalcSetup.lua"),
            3 => e["witness"]["jit_on_sha256"] = json!("wrong"),
            _ => e["scope"]["contribution_emission"] = json!(true),
        }
        assert!(std::panic::catch_unwind(|| distance::authenticate(&e)).is_err());
    }
}

#[test]
fn distance_observation_roles_zero_records_and_combat_are_not_interchangeable() {
    let original = distance::witness();
    for mutation in 0..7 {
        let mut w = original.clone();
        let cases = w["cases"].as_array_mut().unwrap();
        let name = match mutation {
            0 | 1 => "distance-input-zero-placeholder-80",
            2 => "distance-placeholder-zero",
            3 | 4 => "distance-calcs-combat",
            _ => "distance-placeholder-80",
        };
        let at = cases.iter().position(|c| c["name"] == name).unwrap();
        match mutation {
            0 => {
                cases[at]["state"]["main"]["player"]["passes"][0]["accuracy"]["skill_distance"] =
                    json!(80.)
            }
            1 => cases[at]["state"]["config"]["distance_records"][0]["mod"]["tags"] = json!([]),
            2 => cases[at]["state"]["config"]["distance_records"] = json!([{"value":0}]),
            3 => {
                cases[at]["state"]["calcs"]["player"]["passes"][0]["accuracy"]["distance_sum"] =
                    json!(80.)
            }
            4 => cases[at]["state"]["config"]["enemy_distance"]["placeholder"] = json!(0.),
            5 => {
                cases.remove(at);
            }
            _ => {
                let duplicate = cases[at].clone();
                cases.push(duplicate);
            }
        }
        assert!(std::panic::catch_unwind(|| distance::validate_witness(&w)).is_err());
    }
}
