//! Default root inventory closure is distinct from Class mechanics and whole-build coverage.
#[path = "support/owned_class_start_root.rs"]
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
fn default_start_root_has_complete_source_inventory_and_no_class_promotion() {
    family::check_authored();
}

#[test]
fn root_closure_cannot_hide_members_topology_or_class_coverage_changes() {
    let dependencies: Value = family::read("dependencies.json");
    let closure: Value = family::read("closure.json");
    for edit in 0..4 {
        let mut d = dependencies.clone();
        let mut c = closure.clone();
        match edit {
            0 => c["definitions"][0]["value"]["schema"]["value"]["adjacent"]["members"] = json!([]),
            1 => {
                c["definitions"][0]["value"]["schema"]["value"]["declarations"]["grants"]["members"] =
                    json!(["unaccounted-input"])
            }
            2 => c["owners"][0]["programs"]["members"] = json!(["unaccounted-program"]),
            3 => d["action_owners"][1]["programs"]["closure"] = json!({"kind":"complete"}),
            _ => unreachable!(),
        }
        assert!(std::panic::catch_unwind(|| family::check_rows(&d, &c)).is_err());
    }
}

#[test]
fn empty_default_proof_rejects_extra_effects_or_declarations() {
    let a: Value = family::read("authoring.json");
    let v: Value = family::read("source-vectors.json");
    for edit in 0..3 {
        let mut changed = v.clone();
        match edit {
            0 => {
                changed["projection"]["constructed"]["default_modifiers"]["records"] =
                    json!([{"name":"unaccounted"}])
            }
            1 => {
                changed["projection"]["constructed"]["declaration_fields"][0]["present"] =
                    json!(true)
            }
            2 => changed["projection"]["constructed"]["excluded"]
                .as_array_mut()
                .unwrap()
                .push(json!({"key":"grantedSkills","reason":"render_geometry"})),
            _ => unreachable!(),
        }
        assert!(std::panic::catch_unwind(|| family::check_vectors(&a, &changed, false)).is_err());
    }
}

#[test]
#[ignore = "requires CLASS_START_ROOT_PRIOR, fresh OUTPUT and passed original-constructor reports"]
fn publish_default_start_root_preserving_all_five_originals() {
    publication::run_with_scope(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_CLASS_START_ROOT_PRIOR")
                .expect("checked predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_CLASS_START_ROOT_OUTPUT").expect("fresh output"),
        ),
        &family::data(),
        &[],
        &["source-vectors.json"],
        family::stage,
        publication::PublicationScope {
            closed_existing_rule_owners: 1,
            passive_refinement: true,
            extra: json!({
                "new_definitions":0,"new_programs":0,"closed_intrinsic_declaration_inventories":7,
                "class_rule_coverage_preserved":true,"default_root_only":true,
                "universal_player_initialization_closed":false,"external_transformations_closed":false
            }),
        },
    );
}
