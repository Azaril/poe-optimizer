#[path = "support/owned_gem_skill_inventories.rs"]
mod family;

#[test]
fn four_potential_skill_inventories_close_without_other_schema_changes() {
    assert_eq!(family::checked_migration().schema.len(), 4);
    assert_eq!(family::prior_descriptors().len(), 4);
}
#[test]
fn direct_descendant_and_positive_membership_changes_are_rejected() {
    family::delta_controls();
}
#[test]
fn exact_catalog_winners_roles_and_owned_mappings_are_required() {
    family::identity_controls();
}
#[test]
fn declared_constructed_and_real_missing_references_remain_blocking() {
    family::reference_controls();
}
#[test]
fn unreviewed_source_factories_cannot_silently_change_membership() {
    family::source_controls();
}
#[test]
fn ice_nova_aliases_and_constructed_stat_sets_remain_distinct() {
    family::alias_controls();
}
