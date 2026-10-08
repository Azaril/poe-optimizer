//! Scalability metadata reconciliation, without widening item admission.
#[path = "support/owned_flat_life_scalability.rs"]
mod family;
#[allow(dead_code)]
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
fn flat_life_scalability_retires_only_one_proved_obligation() {
    family::check_authored();
    for after in [false, true] {
        let owner = family::expected_owner(after);
        assert!(!owner.programs.is_complete());
        assert_eq!(owner.programs.members.len(), 5);
    }
}

#[test]
fn scalability_certificate_refuses_unreviewed_emitters_and_guard_changes() {
    let d: Value = family::read("dependencies.json");
    let rules = json!([d["item_rule"]]);
    let guards = json!([d["source_condition"]]);
    family::check_admission(&rules, &guards);
    let mut additional = rules.clone();
    let mut unknown = d["item_rule"].clone();
    unknown["id"] = json!("unreviewed-flat-life");
    additional.as_array_mut().unwrap().push(unknown);
    assert!(std::panic::catch_unwind(|| family::check_admission(&additional, &guards)).is_err());
    let mut changed = guards.clone();
    changed[0]["all"].as_array_mut().unwrap().remove(0);
    assert!(std::panic::catch_unwind(|| family::check_admission(&rules, &changed)).is_err());
    let mut component = rules.clone();
    component[0]["emissions"][0]["value"]["rolls"][0]["value"]["value"]["source"]["value"] =
        json!("different-component");
    assert!(std::panic::catch_unwind(|| family::check_admission(&component, &guards)).is_err());
}

#[test]
#[ignore = "requires FLAT_LIFE_SCALABILITY_PRIOR and fresh FLAT_LIFE_SCALABILITY_OUTPUT"]
fn publish_flat_life_scalability_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_FLAT_LIFE_SCALABILITY_PRIOR")
                .expect("checked predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_FLAT_LIFE_SCALABILITY_OUTPUT")
                .expect("fresh output"),
        ),
        &family::data(),
        &[],
        &["coverage.json", "source-vectors.json"],
        family::stage,
        json!({"retired_obligations":1,"retained_obligations":6,"new_definitions":0,"new_programs":0,"numerical_program_changes":0,"import_guard_changes":0,"schema_identity_changed":false,"whole_build_parity":false,"final_life":false,"expected_retired_item_text_diagnostics":0,"expected_retired_selected_input_issues":0}),
        [107, 117, 109, 123, 5],
    );
}
