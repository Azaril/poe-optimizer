//! Source evidence can close direct Gem inventories without closing gameplay coverage.
#[path = "support/owned_support_structural_inventories.rs"]
mod family;

#[test]
fn closes_only_forty_four_reviewed_empty_support_inventories() {
    let migration = family::checked_migration();
    assert_eq!(migration.schema.len(), family::prior_descriptors().len());
}

#[test]
fn unrelated_closure_changes_removed_and_duplicate_families_are_rejected() {
    family::check_delta_controls();
}

#[test]
fn missing_identity_references_and_additional_effects_are_rejected() {
    family::check_identity_controls();
}

#[test]
fn unreviewed_direct_and_support_child_factories_are_rejected() {
    family::check_factory_controls();
}

#[test]
fn missing_mapping_rows_callbacks_aliases_and_structural_modifiers_are_rejected() {
    family::check_mapping_controls();
}

#[test]
fn null_mapping_absence_requires_exact_scoped_recipient_semantics() {
    family::check_description_controls();
}
