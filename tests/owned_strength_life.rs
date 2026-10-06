//! Data-defined inherent Life amount from final Strength and resolved controls.
#[path = "support/owned_strength_life.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[path = "support/owned_strength_life_native.rs"]
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
fn authored_strength_life_is_one_derived_receiver_with_explicit_inputs() {
    family::check_authored();
}
#[test]
#[ignore = "requires STRENGTH_LIFE_PRIOR and fresh OUTPUT"]
fn publish_strength_life_preserving_all_five_originals() {
    publication::run_with_scope(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_STRENGTH_LIFE_PRIOR")
                .expect("checked predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_STRENGTH_LIFE_OUTPUT").expect("fresh output"),
        ),
        &family::data(),
        &[],
        &["source-vectors.json"],
        family::stage,
        publication::PublicationScope {
            closed_existing_rule_owners: 0,
            passive_refinement: false,
            extra: json!({"new_definitions":6,"new_programs":1,"new_receivers":1,"changed_schema_descriptors":0,"attribute_and_flag_producers_remain_open":true,"final_life":false,"incoming_contributors_closed":false}),
        },
    );
}
