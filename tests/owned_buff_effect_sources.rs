//! Publish the source-Skill reducers without changing any imported build facts.
#[allow(dead_code)]
#[path = "support/owned_buff_effect_sources.rs"]
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
#[allow(dead_code)]
#[path = "support/owned_buff_effect_recipient_evidence.rs"]
mod source_evidence;
use serde_json::json;
use std::path::PathBuf;

#[test]
#[ignore = "requires BUFF_SOURCES_PRIOR and fresh BUFF_SOURCES_OUTPUT"]
fn publish_source_queries_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_BUFF_SOURCES_PRIOR").expect("checked predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_BUFF_SOURCES_OUTPUT").expect("fresh output"),
        ),
        &family::data(),
        &[],
        &["authoring.json"],
        family::stage,
        json!({"new_programs":3,"new_queries":4,"complete_empty_groups":4,"nonempty_source_admission":false,
            "whole_build_parity":false,"expected_retired_item_text_diagnostics":0,"expected_retired_selected_input_issues":0}),
        [107, 117, 109, 123, 5],
    );
}
