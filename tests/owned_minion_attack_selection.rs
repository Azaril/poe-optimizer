#[path = "support/owned_minion_attack_selection.rs"]
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
fn fixed_intrinsic_source_changes_only_four_existing_routes() {
    family::check_authored();
    assert_eq!(family::before_routes().len(), 1);
    assert_eq!(family::routes().len(), 1);
}

#[test]
fn source_scope_rejects_inheritance_replacements_and_new_suppliers() {
    family::check_source_refusals();
}

#[test]
fn route_refinement_rejects_scope_defaults_bindings_and_unrelated_changes() {
    let original: Value = family::read("routes.json");
    for edit in 0..10 {
        let mut value = original.clone();
        match edit {
            0 => {
                value[0]["source_selectors"]["members"][0]["sources"][0]["origin"] =
                    json!({"kind":"current_action"})
            }
            1 => value[0]["source_selectors"]["members"][0]["selection"] = json!({"kind":"all"}),
            2 => {
                value[0]["source_selectors"]["members"][0]["policy"]["value"]["source"] =
                    json!("foreign")
            }
            3 => {
                value[0]["routes"]["members"][2]["source"]["value"]["stats"][0]["stat"]["key"] =
                    json!("def.000000000000320f")
            }
            4 => value[0]["routes"]["members"][2]["target"]["key"] = json!("def.0000000000003212"),
            5 => value[0]["routes"]["closure"] = json!({"kind":"complete"}),
            6 => value[0]["source_selectors"]["members"][0]["sources"]
                .as_array_mut()
                .unwrap()
                .push(json!({"id":"foreign","origin":{"kind":"current_action"}})),
            7 => {
                value[0]["routes"]["members"][0]["source"]["value"]["stat"]["key"] =
                    json!("def.000000000000320f")
            }
            8 => value[0]["source_selectors"]["members"]
                .as_array_mut()
                .unwrap()
                .clear(),
            9 => value[0]["output"]["slot"]["key"] = json!("def.0000000000000028"),
            _ => unreachable!(),
        }
        assert!(
            std::panic::catch_unwind(|| family::check_routes(&value)).is_err(),
            "edit {edit}"
        );
    }
}

#[test]
fn selector_proof_rejects_new_suppliers_and_direct_or_foreign_actor_admission() {
    let d: Value = family::read("dependencies.json");
    let original = &d["supply_inventory"];
    for edit in 0..4 {
        let mut changed = original.clone();
        match edit {
            0 => {
                let row = changed["definitions"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| {
                        r["kind"] == "skill" && r["value"]["id"]["key"] == "def.0000000000000021"
                    })
                    .unwrap();
                row["value"]["schema"]["value"]["directly_selectable"] = json!(true);
            }
            1 => {
                let mut row = changed["slots"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| {
                        r["kind"] == "skill_grant"
                            && r["value"]["schema"]["value"]["skill"]["key"]
                                == "def.0000000000000021"
                    })
                    .unwrap()
                    .clone();
                row["value"]["id"]["declaration"]["definition"]["key"] =
                    json!("def.000000000000000a");
                changed["slots"].as_array_mut().unwrap().push(row);
            }
            2 => {
                let row = changed["slots"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| r["kind"] == "actor")
                    .unwrap();
                row["value"]["schema"]["value"]["provider_definition"]["key"] =
                    json!("def.000000000000000a");
            }
            3 => {
                let row = changed["slots"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| r["kind"] == "action_output")
                    .unwrap();
                row["value"]["schema"]["value"]["actor_role"] = json!({"kind":"player"});
            }
            _ => unreachable!(),
        }
        assert!(
            std::panic::catch_unwind(|| family::check_supply_inventory(
                &changed,
                &changed["owners"]
            ))
            .is_err(),
            "edit {edit}"
        );
    }
}

#[test]
#[ignore = "authenticates retained source census and original source files; no source VM"]
fn retained_source_inventory_authenticates_fixed_intrinsic_selection() {
    family::check_source(true);
}

#[test]
#[ignore = "requires MINION_ATTACK_SELECTION_PRIOR, fresh OUTPUT and retained source evidence"]
fn publish_intrinsic_selection_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_MINION_ATTACK_SELECTION_PRIOR")
                .expect("checked prior"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_MINION_ATTACK_SELECTION_OUTPUT")
                .expect("fresh output"),
        ),
        &family::data(),
        &[],
        &[
            "authoring.json",
            "source-evidence.json",
            "dependencies.json",
        ],
        family::stage,
        json!({"new_definitions":0,"new_programs":0,"replaced_routes":4,"new_selectors":1,"closed_selector_inventories":1,"closed_route_inventories":0,"whole_build_parity":false}),
        [106, 117, 109, 123, 4],
    );
}
