#[allow(dead_code)]
#[path = "support/owned_sniper_activation_readiness.rs"]
mod activation_family;
#[path = "support/owned_action_minion_damage.rs"]
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
use std::path::PathBuf;

#[test]
fn action_damage_uses_exact_command_sources_and_existing_shared_damage() {
    family::check_authored();
}

#[test]
#[ignore = "requires ACTION_MINION_DAMAGE_PRIOR and fresh ACTION_MINION_DAMAGE_OUTPUT; retained source reports"]
fn publish_action_damage_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_ACTION_MINION_DAMAGE_PRIOR")
                .expect("checked prior"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_ACTION_MINION_DAMAGE_OUTPUT")
                .expect("fresh output"),
        ),
        &family::data(),
        &[],
        &[
            "authoring.json",
            "source-vectors.json",
            "dependencies.json",
            "gas-partition.json",
        ],
        family::stage,
        serde_json::json!({"new_definitions":2,"new_queries":1,"whole_build_parity":false}),
        [107, 117, 109, 123, 4],
    );
}
