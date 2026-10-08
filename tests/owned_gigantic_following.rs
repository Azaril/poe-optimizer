//! Historical Gigantic packet authoring and publication provenance.
//! Current Boolean execution is covered by the joined Sniper component tests.
#[path = "support/owned_gigantic_following.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[path = "support/owned_passive_refinement_publication.rs"]
mod passive_publication;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_support_delivery_publication.rs"]
mod publication;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use serde_json::{Value, json};
use std::path::PathBuf;

#[test]
fn gigantic_authors_one_complete_default_with_separate_status_and_efficiency() {
    family::check_authored();
}

#[test]
#[ignore = "requires immutable source reports and checked Gigantic predecessor"]
fn publish_gigantic_preserving_all_five_originals() {
    let output = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_GIGANTIC_FOLLOWING_OUTPUT").expect("fresh output"),
    );
    publication::run_with_scope(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_GIGANTIC_FOLLOWING_PRIOR")
                .expect("checked Minion Life predecessor"),
        ),
        output.clone(),
        &family::data(),
        &[],
        &["source-vectors.json"],
        family::stage,
        publication::PublicationScope {
            closed_existing_rule_owners: 1,
            passive_refinement: true,
            extra: json!({"new_definitions":3,"new_programs":3,"new_receivers":1,
                "new_complete_stat_owners":1,"final_damage_claimed":false,
                "final_life_claimed":false,"reservation_delivery_claimed":false,
                "effective_transforms_proved_absent":false}),
        },
    );
    let bytes = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/builds/breadth-20260908/build-05.xml"),
    )
    .unwrap();
    let dir = output.join("original-05");
    let selection = selected::selection(&bytes, &dir);
    let draft: Value =
        serde_json::from_slice(&std::fs::read(dir.join("draft.json")).unwrap()).unwrap();
    let preset = draft["draft"]["allocation_presets"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == selection["build"]["allocations"])
        .unwrap();
    assert_eq!(preset["allocations"]["completion"]["kind"], "complete");
    let selected_ids = preset["allocations"]["members"].as_array().unwrap();
    assert_eq!(
        draft["draft"]["allocations"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|a| selected_ids.contains(&a["id"])
                && a["node"]["kind"] == "known"
                && a["node"]["value"]["key"] == "def.0000000000001532")
            .count(),
        1
    );
}
