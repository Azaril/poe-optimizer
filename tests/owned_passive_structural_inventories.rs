#[path = "support/owned_passive_structural_inventories.rs"]
mod family;

#[test]
fn six_passive_owners_close_eighteen_direct_inventories_preserving_positive_chains() {
    let migration = family::checked_migration();
    assert_eq!(migration.schema.len(), family::prior_descriptors().len());
}
#[test]
fn unrelated_closures_missing_grants_and_crossed_barya_chains_are_rejected() {
    family::delta_controls();
}
#[test]
fn wrong_default_view_node_mapping_and_skill_identity_are_rejected() {
    family::identity_controls();
}
#[test]
fn parser_remainders_cannot_replace_exact_recipient_semantics() {
    family::semantic_controls();
}
#[test]
fn unreviewed_intrinsic_fields_and_additional_grants_are_rejected() {
    family::structural_controls();
}
#[test]
fn every_partial_candidate_is_reviewed_or_explicitly_unresolved() {
    family::census_controls();
}
