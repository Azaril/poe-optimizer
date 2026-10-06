//! Published finite ordinary cold contribution; no final resistance claim.
#[path = "support/owned_cold_item_delivery.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[path = "support/owned_cold_item_delivery_native.rs"]
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
fn authored_cold_delivery_keeps_exact_ordinary_unscaled_scope() {
    family::check_authored();
}

#[test]
#[ignore = "requires checked COLD_ITEM_DELIVERY_PRIOR and fresh OUTPUT"]
fn publish_cold_item_delivery_preserving_all_five_originals() {
    publication::run_with_scope(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_COLD_ITEM_DELIVERY_PRIOR")
                .expect("checked predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_COLD_ITEM_DELIVERY_OUTPUT").expect("fresh output"),
        ),
        &family::data(),
        &[],
        &["source-vectors.json", "coverage.json"],
        family::stage,
        publication::PublicationScope {
            closed_existing_rule_owners: 0,
            passive_refinement: false,
            extra: json!({"new_definitions":1,"new_programs":2,"new_receivers":0,"modifier_and_template_remain_partial":true,"additional_residual_owner_obligations":3,"incoming_contributors_closed":false,"final_resistance":false}),
        },
    );
}
