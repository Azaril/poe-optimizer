//! Explicit intrinsic declaration closure without numerical owner promotion.
#[path = "support/owned_solar_declarations.rs"]
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
fn solar_declarations_preserve_optional_inputs_and_partial_mechanics() {
    family::check_authored();
}
#[test]
#[ignore = "requires checked SOLAR_DECLARATIONS_PRIOR and fresh OUTPUT"]
fn publish_solar_declarations_preserving_all_five_originals() {
    let bindings: serde_json::Value = family::read("bindings.json");
    publication::run_with_item_parameter_completions(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_SOLAR_DECLARATIONS_PRIOR")
                .expect("checked predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_SOLAR_DECLARATIONS_OUTPUT").expect("fresh output"),
        ),
        &family::data(),
        &[],
        &["source-vectors.json"],
        family::stage,
        publication::PublicationScope {
            closed_existing_rule_owners: 0,
            passive_refinement: false,
            extra: json!({"new_definitions":0,"new_programs":0,"closed_template_declaration_inventories":7,"closed_template_rule_owners":0,"quality_membership_unchanged":true,"modifier_membership_unchanged":true,"optional_requirement_preserved":true}),
        },
        &[preservation::ItemParameterCompletion {
            original: 5,
            template: serde_json::from_value(bindings["template"].clone()).unwrap(),
            source_ordinal: 580,
            content_entry: 0,
        }],
    );
}
