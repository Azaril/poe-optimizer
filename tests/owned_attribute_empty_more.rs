//! Guarded empty-MORE queries; no nonempty law or whole-build coverage claim.
#[allow(dead_code)]
#[path = "support/owned_attribute_count_native.rs"]
mod count_native;
#[path = "support/owned_attribute_empty_more.rs"]
mod empty_more;
#[path = "support/owned_empty_support_domain.rs"]
mod empty_support;
#[allow(dead_code)]
#[path = "support/owned_attribute_count_cutover.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[path = "support/owned_attribute_empty_more_native.rs"]
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
#[allow(dead_code)]
#[path = "support/owned_attribute_step_consumers.rs"]
mod step;
use serde_json::json;
use std::path::PathBuf;

#[test]
fn authored_empty_more_is_six_guarded_product_reads_without_literal_producers() {
    empty_more::check_authored();
}
#[test]
#[ignore = "requires ATTRIBUTE_EMPTY_MORE_PRIOR and fresh ATTRIBUTE_EMPTY_MORE_OUTPUT"]
fn publish_empty_more_preserving_all_five_originals() {
    publication::run_with_scope(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_ATTRIBUTE_EMPTY_MORE_PRIOR")
                .expect("checked predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_ATTRIBUTE_EMPTY_MORE_OUTPUT")
                .expect("fresh output"),
        ),
        &empty_more::data(),
        &[],
        &[
            "producers.json",
            "bindings.json",
            "source-vectors.json",
            "authoring.json",
        ],
        empty_more::stage,
        publication::PublicationScope {
            closed_existing_rule_owners: 6,
            passive_refinement: false,
            extra: json!({
                "new_definitions":0,"new_receivers":6,"new_ordered_queries":6,
                "empty_incoming_domain_checked":true,"nonempty_domain_rejected_before_activation":true,
                "production_query_registry_complete":false,"production_base_increase_membership_complete":false,
                "nonempty_more_grouping_claim":false,"whole_build_claim":false
            }),
        },
    );
}
