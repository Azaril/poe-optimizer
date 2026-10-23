//! Offline source proof; no new PoB execution or native effect implementation.
use super::withered;
use serde_json::json;

#[test]
fn configured_player_withered_is_a_raw_actor_input_with_both_lane_receipts() {
    withered::check_authored();
}
#[test]
fn withered_source_identity_and_effect_claim_mutations_are_rejected() {
    let original = withered::read::<serde_json::Value>("source-evidence.json");
    for mutation in 0..5 {
        let mut e = original.clone();
        match mutation {
            0 => e["spans"][0]["body"] = json!("wrong"),
            1 => e["archives"][0]["path"] = json!("src/Modules/CalcSetup.lua"),
            2 => e["constructor_dependencies"][0]["sha256"] = json!("wrong"),
            3 => e["scope"]["target"] = json!("enemy"),
            _ => e["scope"]["native_effect"] = json!(true),
        }
        assert!(std::panic::catch_unwind(|| withered::authenticate(&e)).is_err());
    }
}
#[test]
fn withered_constructor_callback_and_recipient_cannot_be_conflated() {
    let original = withered::observation();
    withered::validate_observation(&original);
    for mutation in 0..8 {
        let mut o = original.clone();
        match mutation {
            0 => o["constructors"][0]["default_state"] = json!(15),
            1 => o["constructors"][0]["placeholder"] = json!(0),
            2 => o["calls"][0]["body"]["first_line"] = json!(1740),
            3 => o["calls"][0]["argument_fields"]["val"] = json!(10),
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
                let record = o["calls"][0]["after"]["player_records"]
                    .as_array_mut()
                    .unwrap()
                    .pop()
                    .unwrap();
                o["calls"][0]["after"]["enemy_records"] = json!([record]);
            }
            _ => {
                let record = o["calls"][0]["after"]["player_records"]
                    .as_array_mut()
                    .unwrap()
                    .last_mut()
                    .unwrap();
                record["body"]["_positions"] = json!([]);
            }
        }
        assert!(std::panic::catch_unwind(|| withered::validate_observation(&o)).is_err());
    }
}
