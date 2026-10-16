//! Current selected source inventory and physical endpoints before conversion.
#[allow(dead_code)]
#[path = "support/owned_minion_attack_selection.rs"]
mod attack_selection_family;
#[allow(dead_code)]
#[path = "support/owned_combined_added_attack_damage.rs"]
mod combined_added_damage_family;
#[allow(dead_code)]
#[path = "support/owned_selected_physical_base.rs"]
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
use std::{fs, path::PathBuf};

fn native_fixture() -> (Value, Value, Value) {
    let d: Value = family::read("dependencies.json");
    let mut definitions = d["selected_native"]["definitions"]
        .as_array()
        .unwrap()
        .clone();
    for row in d["definitions"].as_array().unwrap() {
        if !definitions.contains(row) {
            definitions.push(row.clone());
        }
    }
    let schema = json!({"definitions":definitions,"slots":d["selected_native"]["slots"]});
    let rules = json!({"owners":d["selected_native"]["owners"],"effect_applications":d["effect_applications"],"existing_actor_rules":d["existing_actor_rules"],"contribution_queries":{"closure":d["query_registry_closure"]},"receivers":{"closure":d["receiver_registry_closure"]}});
    (schema, rules, json!([d["routes"]]))
}
#[test]
fn selected_physical_packet_uses_current_exact_action_queries_and_keeps_partial_owners() {
    family::check_authored();
    let (s, r, t) = native_fixture();
    family::check_native(&s, &r, &t, false);
}
#[test]
fn every_potential_writer_is_counted_even_unselected_inactive_zero_or_application_owned() {
    let (s, original, t) = native_fixture();
    for edit in 0..6 {
        let mut r = original.clone();
        let mut p = json!(family::consumer().owners[0].programs.members[0]);
        p["id"] = json!("unselected-zero-writer");
        p["reads"] = json!([]);
        p["nodes"] = json!([
            {"id":"zero","expression":{"kind":"literal","value":{"kind":"quantity","value":{"value":0.0,"unit":family::read::<Value>("bindings.json")["units"]["damage"]}}}},
            {"id":"inactive","expression":{"kind":"literal","value":{"kind":"boolean","value":false}}}
        ]);
        let stat = if edit == 5 {
            "def.0000000000003212"
        } else if edit == 2 {
            "def.000000000000339c"
        } else {
            "def.000000000000339a"
        };
        let mut effect = p["effects"][0].clone();
        effect["effect"]["stat"]["key"] = json!(stat);
        effect["effect"]["value"] = json!("zero");
        effect["when"] = if edit == 1 {
            json!("inactive")
        } else {
            Value::Null
        };
        if edit != 2 && edit != 5 {
            effect["effect"]["kind"] = json!("contribute");
            effect["effect"]["contribution"] = json!("add");
        }
        p["effects"] = json!([effect]);
        let _: poe_optimizer_core::owned_rules::RuleProgram =
            serde_json::from_value(p.clone()).unwrap();
        if edit == 3 {
            r["effect_applications"]["members"][0]["program"] = p;
        } else if edit == 4 {
            let literal = r["effect_applications"]["members"][0]["program"]["nodes"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|n| n["id"] == "one")
                .unwrap();
            literal["expression"]["value"]["value"]["value"] = json!(2.0);
        } else {
            let owner = r["owners"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|o| {
                    o["owner"]["kind"] == "slot"
                        && o["owner"]["value"]["kind"] == "action_output"
                        && o["owner"]["value"]["value"]["slot"]["key"] == "def.0000000000000025"
                })
                .unwrap();
            owner["programs"]["members"].as_array_mut().unwrap().push(p);
        }
        assert!(
            std::panic::catch_unwind(|| family::check_native(&s, &r, &t, false)).is_err(),
            "edit {edit}"
        );
    }
}
#[test]
fn selected_declaration_routes_and_prior_partial_boundaries_are_exact() {
    let (schema, rules, routes) = native_fixture();
    for edit in 0..4 {
        let (mut s, mut r, mut t) = (schema.clone(), rules.clone(), routes.clone());
        match edit {
            0 => {
                s["slots"].as_array_mut().unwrap().pop();
            }
            1 => {
                r["owners"][0]["programs"]["closure"] = json!({"kind":"unknown"});
            }
            2 => t[0]["routes"]["members"] = json!([]),
            _ => r["existing_actor_rules"]["members"] = json!([]),
        }
        assert!(std::panic::catch_unwind(|| family::check_native(&s, &r, &t, false)).is_err());
    }
}
#[test]
fn source_authority_rejects_missing_pins_changed_identity_and_broadened_scope() {
    let original: Value = family::read("source-evidence.json");
    family::authenticate_source(&original);
    for edit in 0..4 {
        let mut s = original.clone();
        match edit {
            0 => {
                s["artifacts"].as_array_mut().unwrap().pop();
            }
            1 => s["artifacts"][0]["sha256"] = json!("wrong"),
            2 => s["scope"]["all_future_basic_actions"] = json!(true),
            _ => s["late_extra_aura"]["future_reused_pass_absence"] = json!(true),
        }
        assert!(std::panic::catch_unwind(|| family::authenticate_source(&s)).is_err());
    }
}

#[test]
#[ignore = "explicit SELECTED_PHYSICAL_BASE_PRIOR/OUTPUT; preserves whole-build gaps"]
fn publish_selected_physical_base_preserving_all_five_originals() {
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_SELECTED_PHYSICAL_BASE_OUTPUT").unwrap(),
    );
    publication::run_with_expected_selected_counts(
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_SELECTED_PHYSICAL_BASE_PRIOR").unwrap()),
        out.clone(),
        &family::data(),
        &[],
        &[
            "authoring.json",
            "dependencies.json",
            "bindings.json",
            "source-evidence.json",
        ],
        family::stage,
        json!({"new_definitions":4,"new_programs":1,"new_queries":2,"new_receivers":0,"selected_native_flat_groups":2,"complete_damage":false}),
        [106, 117, 109, 123, 4],
    );
    let xml = fs::read(family::root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
        .unwrap();
    for name in ["prior-original-05", "original-05"] {
        let directory = out.join(name);
        let selection = selected::selection(&xml, &directory);
        let draft: Value =
            serde_json::from_slice(&fs::read(directory.join("draft.json")).unwrap()).unwrap();
        family::check_selected_import(&draft, &selection);
        let mut changed = selection.clone();
        changed["build"]["skills"]["local"] = json!(1);
        assert!(
            std::panic::catch_unwind(|| family::check_selected_import(&draft, &changed)).is_err()
        );
    }
}
