//! Shared Player contribution bridge, with the pure derived receiver unchanged.
#[allow(dead_code)]
#[path = "support/owned_attribute_flag_membership.rs"]
mod attribute_flag_family;
#[path = "support/owned_inherent_life_contribution.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
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
#[test]
fn authored_shared_player_bridge_preserves_the_pure_receiver_and_emission_law() {
    family::check_authored();
}
#[test]
#[ignore = "requires INHERENT_LIFE_PRIOR and fresh INHERENT_LIFE_OUTPUT"]
fn publish_inherent_life_contribution_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_INHERENT_LIFE_PRIOR")
                .expect("checked predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_INHERENT_LIFE_OUTPUT").expect("fresh output"),
        ),
        &family::data(),
        &[],
        &["source-vectors.json", "authoring.json"],
        family::stage,
        json!({"new_programs":1,"new_definitions":0,"new_receivers":0,"changed_schema_descriptors":0,"query_membership_changed":false,"global_registry_closure_changed":false,"contribution_emission":true,"final_life":false,"whole_build_parity":false,"expected_retired_item_text_diagnostics":0,"expected_retired_selected_input_issues":0}),
        [107, 117, 109, 123, 5],
    );
}
