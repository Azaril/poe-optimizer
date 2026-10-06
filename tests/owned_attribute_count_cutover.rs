//! Checked retirement of Integer attribute contributions in the current release.
//! Raw contribution checks do not establish final attributes or full-build parity.
#[allow(dead_code)]
#[path = "support/owned_attribute_count_cutover.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[path = "support/owned_attribute_count_native.rs"]
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
use serde_json::json;
use std::path::PathBuf;

#[test]
#[ignore = "requires ATTRIBUTE_COUNT_PRIOR and fresh ATTRIBUTE_COUNT_OUTPUT"]
fn publish_count_contributions_preserving_all_five_originals() {
    publication::run_with_scope(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_ATTRIBUTE_COUNT_PRIOR")
                .expect("checked predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_ATTRIBUTE_COUNT_OUTPUT").expect("fresh output"),
        ),
        &family::data(),
        &[],
        &["count-programs.json"],
        family::stage,
        publication::PublicationScope {
            closed_existing_rule_owners: 0,
            passive_refinement: false,
            extra: json!({
                "new_count_channels": 6,
                "changed_programs": 368,
                "old_attribute_contributions_removed": true,
                "final_attribute_and_control_producers_remain_open": true,
                "incoming_contributors_closed": false,
                "numerical_grouping_policy_selected": false
            }),
        },
    );
}
