//! Published intrinsic Life contributions, with finite input boundaries explicit.
#[path = "support/owned_minion_life_source.rs"]
mod family;
#[path = "../crates/poe-optimizer-engine/tests/support/minion_attack_source_fixture.rs"]
mod intrinsic;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[path = "support/owned_minion_life_source_native.rs"]
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
fn intrinsic_life_reuses_existing_life_channel_and_preserves_partial_coverage() {
    family::check_authored();
}

#[test]
#[ignore = "requires immutable source evidence and checked Gigantic benefits predecessor"]
fn publish_intrinsic_life_preserving_all_five_originals() {
    publication::run_with_scope(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_MINION_LIFE_SOURCE_PRIOR")
                .expect("checked predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_MINION_LIFE_SOURCE_OUTPUT").expect("fresh output"),
        ),
        &family::data(),
        &[],
        &["source-vectors.json"],
        family::stage,
        publication::PublicationScope {
            closed_existing_rule_owners: 0,
            passive_refinement: false,
            extra: json!({"new_definitions":0,"new_programs":1,"new_tables":1,
                "new_receivers":0,"actor_owner_remains_partial":true,"full_life_pool":false,
                "gigantic_life_channel_replaced":true,"retired_life_channel_has_no_live_consumers":true}),
        },
    );
}
