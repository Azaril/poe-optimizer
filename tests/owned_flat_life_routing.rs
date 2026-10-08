//! Ordinary Player Life uses the same injected Talisman applicability as levels.
#[path = "support/owned_flat_life_routing.rs"]
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
fn life_delivery_reuses_the_guard_preserving_other_writers_and_coverage() {
    family::check_authored();
}

#[test]
#[ignore = "requires FLAT_LIFE_ROUTING_PRIOR and a fresh FLAT_LIFE_ROUTING_OUTPUT"]
fn publish_life_routing_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_FLAT_LIFE_ROUTING_PRIOR")
                .expect("checked predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_FLAT_LIFE_ROUTING_OUTPUT").expect("fresh output"),
        ),
        &family::data(),
        &[],
        &["authoring.json", "owners.json"],
        family::stage,
        json!({"new_definitions":0,"new_template_programs":4,"guarded_modifier_programs":1,
            "closed_existing_owners":0,"expected_retired_selected_input_issues":0,"whole_build_parity":false}),
        [107, 117, 109, 123, 4],
    );
}
