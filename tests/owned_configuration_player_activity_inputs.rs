//! Portable source proof and opt-in real CLI publication for four Player inputs.
#[allow(dead_code)]
#[path = "support/owned_configuration_player_activity_inputs.rs"]
mod activity;
#[path = "support/owned_configuration_player_activity_publication.rs"]
mod activity_publication;
#[allow(dead_code)]
#[path = "support/owned_configuration_default_sources.rs"]
mod configuration_sources;
#[allow(dead_code)]
#[path = "support/owned_configuration_default_stage.rs"]
mod configuration_stage;
#[allow(dead_code)]
#[path = "support/owned_configuration_default_publication.rs"]
mod publication;
#[allow(dead_code)]
#[path = "support/owned_selected_request.rs"]
mod selected;
use serde_json::json;

#[test]
fn four_distinct_player_inputs_have_exact_selectors_and_both_lane_receipts() {
    activity::check_authored();
}
#[test]
fn publication_expectations_cannot_omit_alias_or_misclassify_a_source() {
    let inputs: Vec<serde_json::Value> = activity::read("input.json");
    assert_eq!(
        publication::checked_group_ranges(&inputs, &activity_publication::source_expectations()),
        vec![0..1, 1..2, 2..3, 3..4]
    );
    for mutation in 0..8 {
        let mut inputs = inputs.clone();
        let mut groups = activity_publication::source_expectations();
        match mutation {
            0 => {
                groups.pop();
            }
            1 => groups[1].source_name = groups[0].source_name.clone(),
            2 => groups[1].expected.outputs.clear(),
            3 => inputs[1]["value_input"] = inputs[0]["value_input"].clone(),
            4 => groups[0].expected.outputs[0].selection = Some("input_then_placeholder"),
            5 => groups[0].expected.outputs[0].policy_target = "enemy",
            6 => groups.swap(0, 1),
            _ => groups[0].expected.default = 20.,
        }
        assert!(
            std::panic::catch_unwind(|| publication::checked_group_ranges(&inputs, &groups))
                .is_err()
        );
    }
}
#[test]
fn source_identity_and_effect_claim_mutations_are_rejected() {
    let original = activity::read::<serde_json::Value>("source-evidence.json");
    for mutation in 0..7 {
        let mut e = original.clone();
        match mutation {
            0 => e["spans"][0]["body"] = json!("wrong"),
            1 => e["archives"][0]["path"] = json!("src/Modules/CalcSetup.lua"),
            2 => e["constructor_dependencies"][0]["sha256"] = json!("wrong"),
            3 => e["scope"]["target"] = json!("enemy"),
            4 => e["scope"]["native_modifier_delivery"] = json!(true),
            5 => e["scope"]["candidate_is_activated_effect"] = json!(true),
            _ => e["scope"]["skip_input_zero"] = json!(false),
        }
        assert!(std::panic::catch_unwind(|| activity::authenticate(&e)).is_err());
    }
}
#[test]
fn constructors_callbacks_and_exact_player_record_ownership_are_checked() {
    let original = activity::observation();
    activity::validate_observation(&original);
    for index in 0..4 {
        for mutation in 0..10 {
            let mut o = original.clone();
            let row = &mut o[index];
            match mutation {
                0 => row["constructors"][0]["default_state"] = json!(1),
                1 => row["constructors"][0]["placeholder"] = json!(0),
                2 => row["calls"][0]["body"]["first_line"] = json!(1908),
                3 => row["calls"][0]["argument_fields"]["val"] = json!(0),
                4 => {
                    row["calls"].as_array_mut().unwrap().pop();
                }
                5 => {
                    row["calls"][0]["after"]["player_records"]
                        .as_array_mut()
                        .unwrap()
                        .pop();
                }
                6 => {
                    let record = row["calls"][0]["after"]["player_records"]
                        .as_array_mut()
                        .unwrap()
                        .pop()
                        .unwrap();
                    row["calls"][0]["after"]["enemy_records"] = json!([record]);
                }
                7 => {
                    row["calls"][0]["after"]["player_records"]
                        .as_array_mut()
                        .unwrap()
                        .last_mut()
                        .unwrap()["body"]["value"] = json!(20)
                }
                8 => {
                    row["calls"][0]["after"]["player_records"]
                        .as_array_mut()
                        .unwrap()
                        .last_mut()
                        .unwrap()["body"]["_positions"] = json!([])
                }
                _ => row["catalogue"][0]["fields"]["type"] = json!("integer"),
            }
            assert!(std::panic::catch_unwind(|| activity::validate_observation(&o)).is_err());
        }
    }
    for mutation in 0..2 {
        let mut o = original.clone();
        if mutation == 0 {
            o.as_array_mut().unwrap().pop();
        } else {
            o[1] = o[0].clone();
        }
        assert!(std::panic::catch_unwind(|| activity::validate_observation(&o)).is_err());
    }
}
#[test]
#[ignore = "requires authenticated prior and fresh output; no source VM"]
fn publish_player_activity_inputs_preserving_all_five_originals() {
    activity_publication::run();
}
