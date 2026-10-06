//! Six real Count consumers; production contributor/factor coverage stays open.
#[allow(dead_code)]
#[path = "support/owned_attribute_count_native.rs"]
mod count_native;
#[path = "support/owned_empty_support_domain.rs"]
mod empty_support;
#[allow(dead_code)]
#[path = "support/owned_attribute_count_cutover.rs"]
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
#[path = "support/owned_attribute_step_consumers.rs"]
mod step;
use serde_json::json;
use std::path::PathBuf;

#[test]
fn authored_consumer_contract_has_no_production_factor_default_or_member_order() {
    step::check_authored();
}
#[test]
#[ignore = "requires ATTRIBUTE_STEP_PRIOR and fresh ATTRIBUTE_STEP_OUTPUT"]
fn publish_attribute_consumers_preserving_all_five_originals() {
    publication::run_with_scope(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_ATTRIBUTE_STEP_PRIOR")
                .expect("checked predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_ATTRIBUTE_STEP_OUTPUT").expect("fresh output"),
        ),
        &step::data(),
        &[],
        // queries.json is the native RulePackageInput registry itself, so its
        // exact value must ship in rules.json. Only offline authoring envelopes
        // and source evidence are forbidden runtime payloads.
        &[
            "consumers.json",
            "source-vectors.json",
            "schema-migration.json",
            "bindings.json",
            "authoring.json",
        ],
        step::stage,
        publication::PublicationScope {
            closed_existing_rule_owners: 0,
            passive_refinement: false,
            extra: json!({
                "new_definitions":9,"new_receivers":6,"new_ordered_queries":12,
                "production_query_membership_complete":false,"production_effective_more_default":false,
                "production_member_ranks":false,"conditional_snapshots_claim":false,"whole_build_claim":false
            }),
        },
    );
}
