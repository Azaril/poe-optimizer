//! Data publication and retained independent oracle observations.
#[path = "support/owned_mixed_minion_damage.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[allow(dead_code)]
#[path = "support/owned_plain_minion_damage.rs"]
mod plain_damage_source;
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
fn authored_mixed_damage_consumes_exact_stacked_group_without_build_literals() {
    family::check_authored();
}
#[test]
#[ignore = "requires MIXED_MINION_DAMAGE_PRIOR and fresh MIXED_MINION_DAMAGE_OUTPUT; retained source reports"]
fn publish_mixed_damage_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_MIXED_MINION_DAMAGE_PRIOR")
                .expect("checked predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_MIXED_MINION_DAMAGE_OUTPUT")
                .expect("fresh output"),
        ),
        &family::data(),
        &[],
        &["authoring.json", "source-vectors.json", "dependencies.json"],
        family::stage,
        serde_json::json!({"new_definitions":2,"new_queries":1,"whole_build_parity":false}),
        [107, 117, 109, 123, 4],
    );
}
