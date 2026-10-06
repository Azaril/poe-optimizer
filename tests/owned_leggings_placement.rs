//! A single placement inventory; numerical ownership and ItemSets stay exact.
#[path = "support/owned_leggings_placement.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[path = "support/owned_leggings_placement_native.rs"]
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
fn leggings_placement_preserves_every_other_domain() {
    family::check_authored();
}

#[test]
#[ignore = "requires checked LEGGINGS_PLACEMENT_PRIOR and RELEASE"]
fn bind_leggings_placement_with_actual_original05() {
    native::check();
}

#[test]
#[ignore = "requires checked LEGGINGS_PLACEMENT_PRIOR and fresh OUTPUT"]
fn publish_leggings_placement_preserving_all_five_originals() {
    publication::run_with_scope(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_LEGGINGS_PLACEMENT_PRIOR")
                .expect("checked predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_LEGGINGS_PLACEMENT_OUTPUT").expect("fresh output"),
        ),
        &family::data(),
        &[],
        &["source-vectors.json"],
        family::stage,
        publication::PublicationScope {
            closed_existing_rule_owners: 0,
            passive_refinement: false,
            extra: json!({"new_definitions":0,"new_programs":0,"closed_equipment_slot_inventories":1,"other_descriptor_changes":0,"numerical_owner_changes":0,"item_set_membership_changed":false,"stock_modeled":false,"expected_retired_item_text_diagnostics":0,"expected_retired_selected_input_issues":0}),
        },
    );
}
