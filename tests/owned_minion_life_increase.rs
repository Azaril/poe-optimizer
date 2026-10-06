//! Actual physical Sniper preparation joined to intrinsic and received Life.
#[path = "support/owned_minion_life_increase.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[path = "support/owned_minion_life_increase_native.rs"]
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
#[path = "support/owned_sniper_final_inputs.rs"]
mod sniper_family;
use serde_json::json;
use std::path::PathBuf;

#[test]
fn authored_life_increase_preserves_the_existing_contribution_contract() {
    family::check_authored();
}

#[test]
#[ignore = "requires checked MINION_LIFE_INCREASE_PRIOR and fresh OUTPUT"]
fn publish_life_increase_preserving_all_five_originals() {
    publication::run_with_scope(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_MINION_LIFE_INCREASE_PRIOR")
                .expect("checked predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_MINION_LIFE_INCREASE_OUTPUT")
                .expect("fresh output"),
        ),
        &family::data(),
        &[],
        &["source-vectors.json"],
        family::stage,
        publication::PublicationScope {
            closed_existing_rule_owners: 0,
            passive_refinement: false,
            extra: json!({"new_definitions":0,"new_programs":1,"new_receivers":0,
                "actor_owner_remains_partial":true,"incoming_contributors_closed":false,
                "whole_life_result":false,"final_level_fixture_literal":false}),
        },
    );
}
