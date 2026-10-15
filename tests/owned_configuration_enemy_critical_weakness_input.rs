//! Source-bound raw Enemy input, with no activation or critical-effect admission.
#[allow(dead_code)]
#[path = "support/owned_configuration_default_sources.rs"]
mod configuration_sources;
#[allow(dead_code)]
#[path = "support/owned_configuration_default_stage.rs"]
mod configuration_stage;
#[path = "support/owned_configuration_enemy_critical_weakness_publication.rs"]
mod critical_publication;
#[allow(dead_code)]
#[path = "support/owned_configuration_enemy_critical_weakness_input.rs"]
mod critical_weakness;
#[path = "support/owned_configuration_default_publication.rs"]
mod publication;
#[allow(dead_code)]
#[path = "support/owned_selected_request.rs"]
mod selected;
use serde_json::json;

#[test]
fn critical_weakness_is_an_enemy_raw_count_with_both_lane_receipts() {
    critical_weakness::check_authored();
}
#[test]
fn source_identity_and_effect_claim_mutations_are_rejected() {
    let original = critical_weakness::read::<serde_json::Value>("source-evidence.json");
    for mutation in 0..6 {
        let mut e = original.clone();
        match mutation {
            0 => e["spans"][0]["body"] = json!("wrong"),
            1 => e["archives"][0]["path"] = json!("src/Modules/CalcSetup.lua"),
            2 => e["constructor_dependencies"][0]["sha256"] = json!("wrong"),
            3 => e["scope"]["target"] = json!("player"),
            4 => e["scope"]["native_effect"] = json!(true),
            _ => e["scope"]["zero_candidate_is_emission"] = json!(true),
        }
        assert!(std::panic::catch_unwind(|| critical_weakness::authenticate(&e)).is_err());
    }
}
#[test]
fn constructor_callback_and_enemy_record_are_independently_checked() {
    let original = critical_weakness::observation();
    critical_weakness::validate_observation(&original);
    for mutation in 0..9 {
        let mut o = original.clone();
        match mutation {
            0 => o["constructors"][0]["default_state"] = json!(20),
            1 => o["constructors"][0]["placeholder"] = json!(0),
            2 => o["calls"][0]["body"]["first_line"] = json!(1908),
            3 => o["calls"][0]["argument_fields"]["val"] = json!(10),
            4 => {
                o["calls"].as_array_mut().unwrap().pop();
            }
            5 => {
                o["calls"][0]["after"]["enemy_records"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            6 => {
                let record = o["calls"][0]["after"]["enemy_records"]
                    .as_array_mut()
                    .unwrap()
                    .pop()
                    .unwrap();
                o["calls"][0]["after"]["player_records"] = json!([record]);
            }
            7 => {
                o["calls"][0]["after"]["enemy_records"]
                    .as_array_mut()
                    .unwrap()
                    .last_mut()
                    .unwrap()["body"]["value"] = json!(20)
            }
            _ => {
                o["calls"][0]["after"]["enemy_records"]
                    .as_array_mut()
                    .unwrap()
                    .last_mut()
                    .unwrap()["body"]["_positions"] = json!([])
            }
        }
        assert!(std::panic::catch_unwind(|| critical_weakness::validate_observation(&o)).is_err());
    }
}
#[test]
#[ignore = "requires authenticated prior and fresh output; no source VM"]
fn publish_critical_weakness_input_preserving_all_five_originals() {
    critical_publication::run();
}
