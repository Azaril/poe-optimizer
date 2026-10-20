//! Partial armour-response publication; complete-build gates remain independent.
#[path = "support/owned_enemy_physical_mitigation_publication.rs"]
mod armour;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_support_delivery_publication.rs"]
mod publication;
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use serde_json::{Value, json};

fn predecessor_fixture() -> (Value, Value, Value) {
    let d = armour::read("dependencies.json");
    let mut owners = d["owners_before"].as_array().unwrap().clone();
    for writer in d["potential_writers"].as_array().unwrap() {
        let n = if let Some(n) = owners.iter().position(|o| o["owner"] == writer["owner"]) {
            n
        } else {
            owners.push(json!({"owner":writer["owner"],"programs":{"members":[],"closure":{"kind":"complete"}}}));
            owners.len() - 1
        };
        let programs = owners[n]["programs"]["members"].as_array_mut().unwrap();
        if !programs.iter().any(|p| p["id"] == writer["program"]["id"]) {
            programs.push(writer["program"].clone());
        }
    }
    owners.sort_by_key(|o| {
        d["potential_writers"]
            .as_array()
            .unwrap()
            .iter()
            .position(|w| w["owner"] == o["owner"])
            .unwrap_or(usize::MAX)
    });
    (
        json!({"definitions":d["definitions"]}),
        json!({"owners":owners,"effect_applications":{"members":[],"closure":{"kind":"partial","value":{"gaps":[]}}}}),
        json!({"outputs":d["input_routes"]}),
    )
}

#[test]
fn authored_armour_component_preserves_partial_admission_and_actual_dependencies() {
    armour::check_authored();
    let (schema, rules, routes) = predecessor_fixture();
    armour::review_predecessor(&schema, &rules, &routes).unwrap();
}

#[test]
fn armour_publication_refuses_missing_pins_and_any_complete_scope_claim() {
    let original = armour::read("authoring.json");
    for field in [
        "complete_request",
        "complete_mitigation",
        "candidate_source_closure",
        "query_closure_changed",
        "existing_owner_closure_changed",
        "ready_for_native_admission",
    ] {
        let mut changed = original.clone();
        changed["scope"][field] = json!(true);
        assert!(armour::review_authoring(&changed).is_err(), "{field}");
    }
    let mut changed = original.clone();
    changed["artifacts"].as_array_mut().unwrap().pop();
    assert!(armour::review_authoring(&changed).is_err());
    let mut changed = original;
    changed["artifacts"][0]["sha256"] = json!("0".repeat(64));
    assert!(armour::review_authoring(&changed).is_err());
}

#[test]
fn armour_dependency_review_rejects_owner_route_and_zero_inactive_application_changes() {
    let (schema, rules, routes) = predecessor_fixture();
    let mut changed = rules.clone();
    changed["owners"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|o| o["owner"] == armour::read("dependencies.json")["owners_before"][0]["owner"])
        .unwrap()["programs"]["closure"] = json!({"kind":"complete"});
    assert!(armour::review_predecessor(&schema, &changed, &routes).is_err());
    let mut changed_routes = routes.clone();
    changed_routes["outputs"][0]["routes"]["members"]
        .as_array_mut()
        .unwrap()
        .pop();
    assert!(armour::review_predecessor(&schema, &rules, &changed_routes).is_err());
    let mut p = armour::read("dependencies.json")["configured_armour"]["program"].clone();
    p["effects"][0]["when"] = json!("inactive-control");
    p["effects"][0]["effect"]["value"] = json!("zero-control");
    let mut changed = rules;
    changed["effect_applications"]["members"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"unlisted-zero-inactive","program":p}));
    assert!(armour::review_predecessor(&schema, &changed, &routes).is_err());
}

#[test]
#[ignore = "explicit immutable publication paths; preserves all five original imports"]
fn publish_partial_armour_response_and_preserve_all_five_originals() {
    publication::run_with_expected_selected_counts(
        std::env::var_os("POE_OPTIMIZER_TEST_ENEMY_PHYSICAL_MITIGATION_PRIOR")
            .unwrap()
            .into(),
        std::env::var_os("POE_OPTIMIZER_TEST_ENEMY_PHYSICAL_MITIGATION_OUTPUT")
            .unwrap()
            .into(),
        &armour::data(),
        &[],
        &[
            "authoring.json",
            "dependencies.json",
            "bindings.json",
            "source-evidence.json",
            "source-vectors.json",
        ],
        armour::stage,
        json!({"new_definitions":2,"new_owner_programs":1,"new_queries":4,
            "complete_request":false,"complete_mitigation":false,"candidate_source_closure":false,
            "native_admission":false,"exact_component_inverse":true}),
        [106, 117, 109, 123, 4],
    );
}
