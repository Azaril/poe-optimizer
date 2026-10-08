//! Publish five source-bound placement inventories and exercise actual binding authority.
#[path = "support/owned_selected_equipment_placement.rs"]
mod family;
#[allow(dead_code)]
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[path = "support/owned_selected_equipment_placement_native.rs"]
mod native;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_support_delivery_publication.rs"]
mod publication;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use serde_json::{Value, json};
use std::path::PathBuf;
#[test]
fn five_placements_change_only_the_proved_inventories() {
    family::check_authored();
}
#[test]
fn placement_proof_refuses_added_acceptance_missing_contexts_and_weapon_tags() {
    let original: Value = family::read("source-vectors.json");
    for change in 0..6 {
        let mut v = original.clone();
        match change {
            0 => {
                v["projection"][0]["slot_outcomes"][0]["outcomes"][0]["outcome"] =
                    json!({"kind":"boolean","return_count":1,"value":true})
            }
            1 => v["projection"][0]["slot_outcomes"][0]["outcomes"][0]["count"] = json!(62),
            2 => v["projection"][0]["state"]["item"]["base"]["tags"]["onehand"] = json!(true),
            3 => v["projection"][4]["state"]["item"]["base"]["tags"]["twohand"] = json!(false),
            4 => v["projection"][2]["state"]["selected_uses"][0]["slot"] = json!("Ring 3"),
            _ => {
                let slot = v["projection"][4]["slot_outcomes"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|row| row["slot"] == "Weapon 1 Swap")
                    .unwrap();
                slot["outcomes"][0]["outcome"]["value"] = json!(false);
            }
        }
        assert!(std::panic::catch_unwind(|| family::check_vectors(&v)).is_err());
    }
}
#[test]
#[ignore = "requires selected placement prior and published release"]
fn actual_selected_item_uses_bind_only_to_their_proved_slots() {
    native::check();
}
#[test]
#[ignore = "requires selected placement prior and fresh output"]
fn publish_selected_equipment_placement_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_SELECTED_EQUIPMENT_PLACEMENT_PRIOR")
                .expect("checked predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_SELECTED_EQUIPMENT_PLACEMENT_OUTPUT")
                .expect("fresh output"),
        ),
        &family::data(),
        &[],
        &["source-vectors.json"],
        family::stage,
        json!({"equipment_slot_inventories_closed":5,"other_declaration_changes":0,"new_definitions":0,"new_programs":0,"closed_existing_rule_owners":0,"numerical_parity":false,"whole_build_parity":false,"expected_retired_item_text_diagnostics":0,"expected_retired_selected_input_issues":0}),
        [107, 117, 109, 123, 5],
    );
}
