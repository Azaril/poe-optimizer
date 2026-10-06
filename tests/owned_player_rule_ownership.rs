//! One shared Player owner; intrinsic component coverage is not final Life.
use reference_native::empty_support;
#[allow(dead_code)]
#[path = "support/owned_player_intrinsic_life.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[path = "support/owned_player_rule_ownership.rs"]
mod ownership;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_support_delivery_publication.rs"]
mod publication;
#[allow(dead_code)]
#[path = "support/owned_player_intrinsic_life_native.rs"]
mod reference_native;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use serde_json::json;
use std::path::PathBuf;

#[test]
fn authored_player_ownership_moves_one_body_without_completing_game_rules() {
    ownership::check_authored();
}

#[test]
#[ignore = "requires PLAYER_RULE_OWNERSHIP_PRIOR and fresh PLAYER_RULE_OWNERSHIP_OUTPUT"]
fn publish_player_rule_ownership_preserving_all_five_originals() {
    publication::run_with_scope(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_PLAYER_RULE_OWNERSHIP_PRIOR")
                .expect("checked predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_PLAYER_RULE_OWNERSHIP_OUTPUT")
                .expect("fresh output"),
        ),
        &ownership::data(),
        &[],
        &[
            "ownership.json",
            "migration.json",
            "bindings.json",
            "source-vectors.json",
            "authoring.json",
        ],
        ownership::stage,
        publication::PublicationScope {
            closed_existing_rule_owners: 0,
            passive_refinement: false,
            extra: json!({"new_definitions":1,"removed_class_programs":8,"new_actor_programs":1,"new_actor_applications":1,"new_receivers":0,"class_partial_preserved":true,"shared_player_initialization_complete":false,"final_life_claim":false,"whole_build_claim":false}),
        },
    );
}
