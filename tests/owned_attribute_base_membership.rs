//! Supported class/passive BASE membership; other donor domains fail closed.
#[path = "support/owned_attribute_base_membership.rs"]
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
fn authored_base_domain_has_bounded_exact_donors_and_explicit_exclusions() {
    family::check_authored();
}
#[test]
#[ignore = "requires ATTRIBUTE_BASE_PRIOR and fresh ATTRIBUTE_BASE_OUTPUT"]
fn publish_bounded_base_membership_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_ATTRIBUTE_BASE_PRIOR")
                .expect("checked predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_ATTRIBUTE_BASE_OUTPUT").expect("fresh output"),
        ),
        &family::data(),
        &[],
        &["source-vectors.json", "authoring.json"],
        family::stage,
        json!({"complete_bounded_base_groups":6,"declared_effect_members":1972,"excluded_item_effects":12,"new_definitions":0,"new_programs":0,"new_receivers":0,"numeric_program_changes":0,"global_registry_closure_changed":false,"schema_identity_changed":false,"expected_retired_item_text_diagnostics":0,"expected_retired_selected_input_issues":0,"whole_build_parity":false}),
        [107, 117, 109, 123, 5],
    );
}
