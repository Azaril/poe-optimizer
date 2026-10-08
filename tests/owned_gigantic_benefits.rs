//! Historical Gigantic benefit packet authoring and publication provenance.
//! Current paired contributions are exercised by the joined Sniper component.
#[path = "support/owned_gigantic_benefits.rs"]
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
use serde_json::json;
use std::path::PathBuf;

#[test]
fn gigantic_benefits_are_one_conditional_actor_program_with_two_general_channels() {
    family::check_authored();
}

#[test]
#[ignore = "requires immutable source reports and the checked Gigantic status predecessor"]
fn publish_gigantic_benefits_preserving_all_five_originals() {
    publication::run_with_scope(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_GIGANTIC_BENEFITS_PRIOR")
                .expect("checked status predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_GIGANTIC_BENEFITS_OUTPUT").expect("fresh output"),
        ),
        &family::data(),
        &[],
        &["source-vectors.json"],
        family::stage,
        publication::PublicationScope {
            closed_existing_rule_owners: 0,
            passive_refinement: false,
            extra: json!({"new_definitions":2,"new_programs":1,"new_receivers":0,
                "actor_owner_remains_partial":true,"full_life_pool":false,"full_damage_metric":false,
                "complete_more_bucket":false,"reservation_delivery_claimed":false}),
        },
    );
}
