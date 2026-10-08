//! Bounded inherent flag membership with unchanged whole-build coverage.
#[path = "support/owned_attribute_flag_membership.rs"]
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
fn authored_memberships_admit_only_two_exact_passive_flag_bodies() {
    family::check_authored();
    assert_eq!(
        family::checked_source()["native_cases"][4]["inherent_life"],
        54
    );
}
#[test]
#[ignore = "requires ATTRIBUTE_FLAGS_PRIOR and fresh ATTRIBUTE_FLAGS_OUTPUT"]
fn publish_bounded_flag_membership_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_ATTRIBUTE_FLAGS_PRIOR")
                .expect("checked predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_ATTRIBUTE_FLAGS_OUTPUT").expect("fresh output"),
        ),
        &family::data(),
        &[],
        &["source-vectors.json", "authoring.json"],
        family::stage,
        json!({"complete_bounded_flag_groups":5,"declared_passive_effect_members":2,"guarded_empty_groups":3,"new_definitions":0,"new_programs":0,"new_receivers":0,"numeric_program_changes":0,"global_registry_closure_changed":false,"schema_identity_changed":false,"expected_retired_item_text_diagnostics":0,"expected_retired_selected_input_issues":0,"whole_build_parity":false}),
        [107, 117, 109, 123, 5],
    );
}
