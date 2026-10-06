//! Data-defined intrinsic Player Life, with original input and publication checks.
#[path = "support/owned_player_intrinsic_life.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[path = "support/owned_player_intrinsic_life_native.rs"]
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
fn authored_intrinsic_player_life_preserves_partial_class_owners() {
    family::check_authored();
}
#[test]
#[ignore = "requires PLAYER_INTRINSIC_LIFE_PRIOR and fresh OUTPUT"]
fn publish_player_intrinsic_life_preserving_all_five_originals() {
    publication::run_with_scope(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_PLAYER_INTRINSIC_LIFE_PRIOR")
                .expect("checked predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_PLAYER_INTRINSIC_LIFE_OUTPUT")
                .expect("fresh output"),
        ),
        &family::data(),
        &[],
        &["source-vectors.json"],
        family::stage,
        publication::PublicationScope {
            closed_existing_rule_owners: 0,
            passive_refinement: false,
            extra: json!({"new_definitions":0,"new_programs":8,"new_receivers":0,"changed_schema_descriptors":0,"class_owners_remain_partial":true,"final_life":false,"incoming_contributors_closed":false}),
        },
    );
}
