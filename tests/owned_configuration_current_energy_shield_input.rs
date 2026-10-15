//! Raw Player resource percentage preserves both source-local callback outputs.
#[allow(dead_code)]
#[path = "support/owned_configuration_default_sources.rs"]
mod configuration_sources;
#[allow(dead_code)]
#[path = "support/owned_configuration_default_stage.rs"]
mod configuration_stage;
#[allow(dead_code)]
#[path = "support/owned_configuration_current_energy_shield_input.rs"]
mod current_energy_shield;
#[path = "support/owned_configuration_current_energy_shield_publication.rs"]
mod current_energy_shield_publication;
#[path = "support/owned_configuration_default_publication.rs"]
mod publication;
#[allow(dead_code)]
#[path = "support/owned_selected_request.rs"]
mod selected;
use serde_json::json;

#[test]
fn current_energy_shield_is_player_percentage_with_both_lane_receipts() {
    current_energy_shield::check_authored();
}

#[test]
fn source_identity_and_both_responsibilities_are_sealed() {
    let original = current_energy_shield::read::<serde_json::Value>("source-evidence.json");
    for mutation in 0..10 {
        let mut e = original.clone();
        match mutation {
            0 => e["spans"][0]["body"] = json!("only one output"),
            1 => e["archives"][0]["path"] = json!("src/Modules/CalcSetup.lua"),
            2 => e["constructor_dependencies"][0]["sha256"] = json!("wrong"),
            3 => e["scope"]["target"] = json!("enemy"),
            4 => e["scope"]["native_flag"] = json!(true),
            5 => e["scope"]["global_condition_absence"] = json!(true),
            6 => {
                e["derivation"]["nonzero"].as_array_mut().unwrap().remove(0);
            }
            7 => {
                e["derivation"]["zero"] =
                    json!([{"name":"Condition:UseCurrentEnergyShield","value":false}])
            }
            8 => e["derivation"]["nonzero"][1]["value"] = json!("clamp-to-150"),
            _ => e["derivation"]["source_scope"] = json!("global-condition"),
        }
        assert!(std::panic::catch_unwind(|| current_energy_shield::authenticate(&e)).is_err());
    }
}

#[test]
fn both_ordered_outputs_and_entire_original_callback_state_are_checked() {
    let original = current_energy_shield::observation();
    current_energy_shield::validate_observation(&original);
    for mutation in 0..13 {
        let mut o = original.clone();
        match mutation {
            0 => o["constructors"][0]["default_state"] = json!(100),
            1 => o["constructors"][0]["placeholder"] = json!(0),
            2 => o["calls"][0]["body"]["first_line"] = json!(158),
            3 => o["calls"][0]["argument_fields"]["val"] = json!(0),
            4 => {
                o["calls"].as_array_mut().unwrap().pop();
            }
            5 => {
                o["calls"][0]["after"]["player_records"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            6 => {
                let records = o["calls"][0]["after"]["player_records"]
                    .as_array_mut()
                    .unwrap();
                records.remove(records.len() - 2);
            }
            7 => {
                let records = o["calls"][0]["after"]["player_records"]
                    .as_array_mut()
                    .unwrap();
                let n = records.len();
                records.swap(n - 1, n - 2);
            }
            8 => {
                let records = o["calls"][0]["after"]["player_records"]
                    .as_array_mut()
                    .unwrap();
                let n = records.len();
                records[n - 2]["body"]["value"] = json!(false);
            }
            9 => {
                o["calls"][0]["after"]["player_records"]
                    .as_array_mut()
                    .unwrap()
                    .last_mut()
                    .unwrap()["body"]["value"] = json!(150);
            }
            10 => o["calls"][0]["after"]["enemy_records"] = json!([{"unexpected":true}]),
            11 => {
                let records = o["calls"][0]["after"]["player_records"]
                    .as_array_mut()
                    .unwrap();
                let n = records.len();
                records[n - 2]["body"]["_positions"] =
                    json!([{"index":1,"value":{"type":"Condition","var":"Effective"}}]);
            }
            _ => o["calls"][0]["body"]["path"] = json!("@unreviewed.lua"),
        }
        assert!(
            std::panic::catch_unwind(|| current_energy_shield::validate_observation(&o)).is_err()
        );
    }
}

#[test]
fn one_validated_candidate_retains_the_complete_source_local_pair() {
    current_energy_shield::authenticate(&current_energy_shield::read("source-evidence.json"));
    for candidate in [-1_000_000., -12.5, 0.5, 100., 175., 1_000_000.] {
        let records = current_energy_shield::source_outputs(Some(candidate)).unwrap();
        let records = records.as_array().unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(
            records[0],
            json!({"name":"Condition:UseCurrentEnergyShield","type":"FLAG","value":true,"source":"Config","flags":0,"keywordFlags":0})
        );
        assert_eq!(
            records[1],
            json!({"name":"Multiplier:CurrentEnergyShield","type":"BASE","value":candidate,"source":"Config","flags":0,"keywordFlags":0})
        );
    }
    // No callback emission is different from supplying a false Boolean, and
    // neither establishes the global value in the presence of other sources.
    for zero in [0., -0.] {
        assert_eq!(
            current_energy_shield::source_outputs(Some(zero)),
            Some(json!([]))
        );
    }
    assert_eq!(current_energy_shield::source_outputs(None), None);
    for invalid in [f64::NAN, f64::INFINITY, -1_000_001., 1_000_001.] {
        assert_eq!(current_energy_shield::source_outputs(Some(invalid)), None);
    }
}

#[test]
#[ignore = "requires authenticated prior and fresh output; no source VM"]
fn publish_current_energy_shield_input_preserving_all_five_originals() {
    current_energy_shield_publication::run();
}
