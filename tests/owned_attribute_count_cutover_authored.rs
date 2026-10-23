//! Archived authored Count laws; native occurrence tests remain separate.
#[allow(dead_code)]
#[path = "support/owned_attribute_count_cutover.rs"]
mod family;
#[test]
fn authored_attribute_count_cutover_preserves_each_existing_occurrence() {
    family::check_authored();
}
