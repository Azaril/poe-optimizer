//! Injected final critical arithmetic; no importer or whole-build coverage change.
#[allow(dead_code)]
#[path = "support/owned_player_critical_receivers.rs"]
mod family;
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
fn final_results_have_stat_owners_and_keep_supplier_coverage_open() {
    family::check_authored();
}

#[test]
fn configured_inputs_are_reused_without_an_import_adapter() {
    let c = family::consumer();
    let input_reads: Vec<_> = c
        .owners
        .iter()
        .flat_map(|o| &o.programs.members)
        .flat_map(|p| &p.reads)
        .filter_map(|r| match &r.source {
            poe_optimizer_core::owned_rules::RuleReadSource::External { input, .. } => {
                Some(input.key().as_str())
            }
            _ => None,
        })
        .collect();
    assert!(input_reads.contains(&"def.0000000000003371"));
    assert!(input_reads.contains(&"def.0000000000003372"));
    assert!(input_reads.iter().all(|key| matches!(
        *key,
        "def.0000000000003251" | "def.0000000000003371" | "def.0000000000003372"
    )));
    let e = family::extension();
    assert!(e.schema.iter().all(|r| matches!(
        r,
        poe_optimizer_import::owned_recipe_extension::SchemaExtensionEntry::Definition(
            poe_optimizer_core::owned_schema::DefinitionDescriptor::Stat(_)
        )
    )));
    let bindings: Value = family::read("bindings.json");
    assert_eq!(bindings["outputs"].as_array().unwrap().len(), 3);
}

#[test]
#[ignore = "requires PLAYER_CRITICAL_PRIOR/OUTPUT; preserves all five exact imports"]
fn publish_player_critical_receivers_preserving_all_five() {
    publication::run_with_expected_selected_counts(
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_PLAYER_CRITICAL_PRIOR").unwrap()),
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_PLAYER_CRITICAL_OUTPUT").unwrap()),
        &family::data(),
        &[],
        &["authoring.json", "dependencies.json", "bindings.json"],
        family::stage,
        json!({"new_definitions":13,"new_programs":3,"new_receivers":3,"new_queries":4,
            "supplier_admission":false,"complete_critical_effect":false,"whole_build_parity":false}),
        [106, 117, 109, 123, 4],
    );
}
