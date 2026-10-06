//! Published local Armour/ES composition with explicit intermediate scope.
#[path = "support/owned_local_defence_composition.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[path = "support/owned_local_defence_composition_native.rs"]
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
fn authored_local_defence_preserves_grouping_and_intermediate_scope() {
    family::check_authored();
}
#[test]
#[ignore = "requires checked LOCAL_DEFENCE_COMPOSITION_PRIOR and fresh OUTPUT"]
fn publish_local_defence_composition_preserving_all_five_originals() {
    publication::run_with_scope(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_LOCAL_DEFENCE_COMPOSITION_PRIOR")
                .expect("checked predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_LOCAL_DEFENCE_COMPOSITION_OUTPUT")
                .expect("fresh output"),
        ),
        &family::data(),
        &[],
        &["source-vectors.json"],
        family::stage,
        publication::PublicationScope {
            closed_existing_rule_owners: 0,
            passive_refinement: false,
            extra: json!({"new_definitions":8,"new_programs":2,"new_receivers":2,"template_owners_unchanged_partial":true,"incoming_contributors_closed":false,"final_item_defences":false,"actor_defences":false,"crafted_quality_preparation_implemented":false,"intermediate_preoverride_values_only":true}),
        },
    );
}
