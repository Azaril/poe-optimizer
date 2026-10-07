//! Complete current increased-attribute memberships; unrelated coverage stays Partial.
#[path = "support/owned_empty_support_domain.rs"]
mod empty_support;
#[path = "support/owned_attribute_increase_membership.rs"]
mod family;
#[path = "support/owned_attribute_increase_membership_native.rs"]
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
#[path = "support/owned_attribute_increase_source.rs"]
mod source;
use serde_json::json;
use std::path::PathBuf;

#[test]
fn authored_increase_membership_is_exhaustive_and_exact() {
    family::check_authored();
}

#[test]
#[ignore = "requires ATTRIBUTE_INCREASE_PRIOR and fresh ATTRIBUTE_INCREASE_OUTPUT"]
fn publish_increase_membership_preserving_all_five_originals() {
    publication::run_with_scope(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_ATTRIBUTE_INCREASE_PRIOR")
                .expect("checked predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_ATTRIBUTE_INCREASE_OUTPUT").expect("fresh output"),
        ),
        &family::data(),
        &[],
        &["source-vectors.json", "authoring.json"],
        family::stage,
        publication::PublicationScope {
            closed_existing_rule_owners: 0,
            passive_refinement: false,
            extra: json!({"complete_increase_groups":6,"declared_effect_members":34,"new_definitions":0,"new_programs":0,"new_receivers":0,"numeric_program_changes":0,"base_membership_changed":false,"global_registry_closure_changed":false,"schema_identity_changed":false,"expected_retired_item_text_diagnostics":0,"expected_retired_selected_input_issues":0}),
        },
    );
}
