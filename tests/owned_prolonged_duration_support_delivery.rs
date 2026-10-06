//! Real player-action topology and duration/cost contributions; no whole-build closure.
#[path = "support/owned_prolonged_duration_support_delivery.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[path = "support/owned_prolonged_duration_native.rs"]
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
use serde_json::json;
use std::path::PathBuf;
const EFFECTS: &[&str] = &[
    "ProlongedDurationSupportPlayer",
    "ProlongedDurationSupportPlayerTwo",
];

#[test]
fn authored_prolonged_duration_keeps_real_action_and_partial_owners() {
    family::check_authored();
}

#[test]
#[ignore = "requires passed source evidence and explicit PROLONGED_PRIOR/OUTPUT paths"]
fn publish_prolonged_duration_preserving_five_originals() {
    publication::run(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_PROLONGED_PRIOR")
                .expect("checked Encroaching predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_PROLONGED_OUTPUT").expect("new output directory"),
        ),
        &family::data(""),
        EFFECTS,
        family::stage,
        json!({"new_definitions":4,"new_slots":1,"new_programs":6,"final_resource_cost_claimed":false,
            "final_duration_claimed":false,"application_lifetime_transfer_claimed":false,"reservation_contributions_added":false}),
    );
}
