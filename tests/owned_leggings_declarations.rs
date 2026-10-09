//! Five intrinsic Leggings declaration inventories; socket and rule coverage stay open.
#[path = "support/owned_leggings_declarations.rs"]
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
fn leggings_declarations_change_only_five_empty_intrinsic_inventories() {
    family::check_authored();
}

#[test]
fn leggings_evidence_refuses_missing_sockets_wrong_identity_and_nonempty_grants() {
    family::check_evidence_negatives();
}

#[test]
fn leggings_refinement_refuses_nonempty_unknown_or_unrelated_declarations() {
    family::check_descriptor_negatives();
}

#[test]
#[ignore = "one-time exact packet authoring from retained reports and checked predecessor"]
fn author_leggings_declarations_from_retained_reports() {
    family::author();
}

#[test]
#[ignore = "requires checked LEGGINGS_DECLARATIONS_PRIOR, retained source reports and fresh OUTPUT"]
fn publish_leggings_declarations_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_LEGGINGS_DECLARATIONS_PRIOR")
                .expect("checked predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_LEGGINGS_DECLARATIONS_OUTPUT")
                .expect("fresh output"),
        ),
        &family::data(),
        &[],
        &["source-vectors.json"],
        family::stage,
        json!({
            "new_definitions":0,
            "new_programs":0,
            "closed_template_declaration_inventories":5,
            "closed_template_rule_owners":0,
            "socket_declaration_unchanged":true,
            "parameter_declaration_unchanged":true,
            "quality_membership_unchanged":true,
            "modifier_membership_unchanged":true,
            "modifier_supplied_grants_not_certified":true
        }),
        [106, 117, 109, 123, 4],
    );
}
