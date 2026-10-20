#[path = "support/owned_gem_direct_inventories.rs"]
mod family;

#[test]
fn twelve_direct_gem_closures_preserve_all_other_schema_and_rule_coverage() {
    assert_eq!(family::checked_migration().schema.len(), 4);
    assert_eq!(family::prior_descriptors().len(), 4);
}
#[test]
fn every_required_closure_and_exact_twelve_field_inverse_are_checked() {
    family::delta_controls();
}
#[test]
fn complete_primary_chains_and_partial_descendant_obligations_are_required() {
    family::ownership_controls();
}
#[test]
fn inherited_catalog_constructor_missing_reference_and_stat_set_proofs_hold() {
    family::inherited_proof_controls();
}
#[test]
fn external_generated_groups_and_skill_owned_factories_keep_their_owners() {
    family::source_ownership_controls();
}
#[test]
fn unreviewed_barbs_thorns_and_additional_factories_are_rejected() {
    family::factory_controls();
}
#[test]
fn orb_ground_and_offering_spike_obligations_cannot_be_erased() {
    family::descendant_controls();
}
