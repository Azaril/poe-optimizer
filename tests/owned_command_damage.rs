//! Source-gated publication and finite conditional Minion Damage component.
#[path = "support/owned_command_damage.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[path = "support/owned_command_damage_native.rs"]
mod native;
#[path = "support/owned_passive_refinement_publication.rs"]
mod passive_publication;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_support_delivery_publication.rs"]
mod publication;
#[allow(dead_code)]
#[path = "support/owned_sniper_population_readiness.rs"]
mod readiness_family;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use serde_json::{Value, json};
use std::{fs, path::PathBuf};

#[test]
fn command_damage_authors_only_three_default_bodies_and_conditional_receiving() {
    family::check_authored();
}
#[test]
#[ignore = "requires passed source evidence and explicit COMMAND_DAMAGE_PRIOR/OUTPUT paths"]
fn publish_command_damage_preserving_all_five_originals() {
    let output = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_COMMAND_DAMAGE_OUTPUT")
            .expect("fresh immutable output"),
    );
    publication::run_with_scope(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_COMMAND_DAMAGE_PRIOR")
                .expect("checked Amulet snapshot predecessor"),
        ),
        output.clone(),
        &family::data(),
        &[],
        &["source-vectors.json"],
        family::stage,
        publication::PublicationScope {
            closed_existing_rule_owners: 3,
            passive_refinement: true,
            extra: json!({"new_definitions":3,"new_slots":0,"new_programs":7,"new_receivers":2,
                "new_complete_stat_owners":2,"final_damage_claimed":false,"effective_transforms_proved_absent":false}),
        },
    );
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/builds/breadth-20260908/build-05.xml");
    let bytes = fs::read(source).unwrap();
    let dir = output.join("original-05");
    let selection = selected::selection(&bytes, &dir);
    let draft: Value = serde_json::from_slice(&fs::read(dir.join("draft.json")).unwrap()).unwrap();
    let preset = draft["draft"]["allocation_presets"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == selection["build"]["allocations"])
        .unwrap();
    assert_eq!(preset["allocations"]["completion"]["kind"], "complete");
    let selected_ids = preset["allocations"]["members"].as_array().unwrap();
    let bindings: Value = family::read("bindings.json");
    for node in bindings["nodes"].as_array().unwrap() {
        assert_eq!(
            draft["draft"]["allocations"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|a| selected_ids.contains(&a["id"])
                    && a["node"] == json!({"kind":"known","value":node["definition"]}))
                .count(),
            1,
            "exact selected Original05 allocation remains once"
        );
    }
}
