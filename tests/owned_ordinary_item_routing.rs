//! Checked ordinary-item routing publication; contributor closure stays separate.
#[path = "support/owned_ordinary_item_routing.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[path = "support/owned_ordinary_item_routing_native.rs"]
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
fn ordinary_item_routing_preserves_copy_and_partial_owner_inventories() {
    family::check_authored();
}

#[test]
#[ignore = "requires passed source evidence and explicit ORDINARY_ITEM_ROUTING_PRIOR/OUTPUT paths"]
fn publish_ordinary_item_routing_preserving_all_five_originals() {
    publication::run_with_payloads(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_ORDINARY_ITEM_ROUTING_PRIOR")
                .expect("checked Sniper population readiness predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_ORDINARY_ITEM_ROUTING_OUTPUT")
                .expect("fresh immutable routing output directory"),
        ),
        &family::data(""),
        &[],
        &[
            "readiness.json",
            "source-vectors.json",
            "routing-authoring.json",
        ],
        family::stage,
        json!({"new_definitions":2,"new_slots":0,"new_programs":8,
            "replaced_programs":1,"new_receivers":1,"new_complete_stat_owners":1,
            "incoming_contributor_inventory_closed":false,"early_copy_unchanged":true,
            "minion_property_consumer_claimed":false,"readiness_reference_only":true}),
    );
}
