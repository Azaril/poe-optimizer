//! Publish selected Player off-hand facts without completing effective conditions.
#[allow(dead_code)]
#[path = "support/owned_empty_support_domain.rs"]
mod empty_support;
#[path = "support/owned_player_offhand_facts.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[path = "support/owned_player_offhand_native.rs"]
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
fn authored_offhand_facts_preserve_selected_structural_scope() {
    family::check_authored();
}

#[test]
#[ignore = "requires authenticated PLAYER_OFFHAND_PRIOR and fresh OUTPUT"]
fn publish_player_offhand_facts_preserving_all_five_originals() {
    publication::run_with_scope(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_PLAYER_OFFHAND_PRIOR")
                .expect("checked predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_PLAYER_OFFHAND_OUTPUT").expect("fresh output"),
        ),
        &family::data(),
        &[],
        &["source-vectors.json"],
        family::stage,
        publication::PublicationScope {
            closed_existing_rule_owners: 0,
            passive_refinement: false,
            extra: json!({"new_definitions":5,"new_programs":1757,"new_receivers":0,"classified_templates":1756,"all_prior_closures_preserved":true,"effective_conditions_complete":false,"selected_structural_facts_only":true}),
        },
    );
}
