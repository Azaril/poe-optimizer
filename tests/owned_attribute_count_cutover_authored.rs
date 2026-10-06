//! Exact authored Count cutover checks; publication and actual occurrence tests are separate.
#[allow(dead_code)]
#[path = "support/owned_attribute_count_cutover.rs"]
mod family;
#[allow(dead_code)]
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[test]
fn authored_attribute_count_cutover_preserves_each_existing_occurrence() {
    family::check_authored();
}
