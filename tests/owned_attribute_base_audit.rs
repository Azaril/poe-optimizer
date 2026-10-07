//! Audit real BASE lanes before assigning native contributor order.
//! This publishes no rule closure and does not certify a complete build.
#[path = "support/owned_attribute_base_numeric_audit.rs"]
mod numeric;
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_attribute_base_source_audit.rs"]
mod source_audit;

#[test]
fn authored_base_source_inventory_retains_every_observed_lane() {
    source_audit::verify(false);
}

#[test]
#[ignore = "requires authenticated retained attribute pipeline reports and pinned PoB"]
fn retained_base_source_reports_match_exact_projection_in_both_jit_modes() {
    source_audit::verify(true);
}
