//! Three exact parameter-inventory closures; all numerical evidence is unchanged.
#[path = "support/owned_selected_item_parameters.rs"]
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
use serde_json::{Value, json};
use std::path::PathBuf;

#[test]
fn selected_item_parameters_preserve_every_other_inventory() {
    family::check_authored();
}

#[test]
#[ignore = "requires checked SELECTED_ITEM_PARAMETERS_PRIOR and fresh OUTPUT"]
fn publish_selected_item_parameters_preserving_all_five_originals() {
    let bindings: Value = family::read("bindings.json");
    let completions: Vec<_> = bindings["templates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| preservation::ItemParameterCompletion {
            original: row["original"].as_u64().unwrap() as usize,
            template: serde_json::from_value(row["template"].clone()).unwrap(),
            source_ordinal: row["source_ordinal"].as_u64().unwrap() as u32,
            content_entry: row["content_entry"].as_u64().unwrap() as usize,
        })
        .collect();
    publication::run_with_item_parameter_completions(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_SELECTED_ITEM_PARAMETERS_PRIOR")
                .expect("checked predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_SELECTED_ITEM_PARAMETERS_OUTPUT")
                .expect("fresh output"),
        ),
        &family::data(),
        &[],
        &["source-vectors.json"],
        family::stage,
        publication::PublicationScope {
            closed_existing_rule_owners: 0,
            passive_refinement: false,
            extra: json!({"new_definitions":0,"new_programs":0,"closed_parameter_inventories":3,"other_declaration_changes":0,"numerical_owner_changes":0,"expected_retired_item_text_diagnostics":3,"expected_retired_selected_input_issues":0}),
        },
        &completions,
    );
}
