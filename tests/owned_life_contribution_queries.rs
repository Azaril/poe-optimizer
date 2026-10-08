//! Publish canonical Life query membership through the current owned release path.
#[allow(dead_code)]
#[path = "support/owned_life_contribution_queries.rs"]
mod family;
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
#[ignore = "requires LIFE_QUERIES_PRIOR and fresh LIFE_QUERIES_OUTPUT"]
fn publish_life_queries_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_LIFE_QUERIES_PRIOR").expect("checked predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_LIFE_QUERIES_OUTPUT").expect("fresh output"),
        ),
        &family::data(),
        &[],
        &["authoring.json"],
        family::stage,
        json!({"new_queries":3,"groups":7,"exact_members":8,"complete_groups":6,"final_life":false,"whole_build_parity":false,"expected_retired_item_text_diagnostics":0,"expected_retired_selected_input_issues":0}),
        [107, 117, 109, 123, 5],
    );
}
