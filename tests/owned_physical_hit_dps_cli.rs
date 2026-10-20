//! Publish an internal physical component while preserving whole-build refusal.
#[allow(dead_code)]
#[path = "support/owned_physical_hit_dps.rs"]
mod dps;
#[path = "support/owned_physical_hit_dps_publication.rs"]
mod physical;
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
#[allow(dead_code)]
#[path = "support/owned_physical_hit_dps_source.rs"]
mod source;
use poe_optimizer_core::owned_rules::DefinitionRules;
use serde_json::{Value, json};
use source::migration_preservation;

fn predecessor_fixture() -> (Value, Value, Value) {
    let dependencies: Value = dps::read("dependencies.json");
    let mut owners = dependencies["owners_before"].as_array().unwrap().clone();
    for writer in dependencies["potential_writers"].as_array().unwrap() {
        let index = if let Some(index) = owners.iter().position(|o| o["owner"] == writer["owner"]) {
            index
        } else {
            owners.push(json!({"owner":writer["owner"],"programs":{"members":[],"closure":{"kind":"complete"}}}));
            owners.len() - 1
        };
        let programs = owners[index]["programs"]["members"].as_array_mut().unwrap();
        if !programs.iter().any(|p| p["id"] == writer["program"]["id"]) {
            programs.push(writer["program"].clone());
        }
    }
    owners.sort_by_key(|owner| {
        dependencies["potential_writers"]
            .as_array()
            .unwrap()
            .iter()
            .position(|writer| writer["owner"] == owner["owner"])
            .unwrap_or(usize::MAX)
    });
    (
        json!({"definitions":dependencies["definitions"]}),
        json!({"owners":owners,"effect_applications":{"members":[],"closure":{"kind":"partial","value":{"gaps":[]}}}}),
        json!({"outputs":dependencies["input_routes"]}),
    )
}

#[test]
fn physical_component_seals_actual_predecessor_without_final_metric_binding() {
    dps::check();
    source::check();
    physical::review_authoring(&dps::read::<Value>("authoring.json")).unwrap();
    let (schema, rules, routing) = predecessor_fixture();
    physical::review_predecessor(&schema, &rules, &routing).unwrap();
    let mut staged_rules = rules.clone();
    for owner in staged_rules["owners"].as_array_mut().unwrap() {
        let typed: DefinitionRules = serde_json::from_value(owner.clone()).unwrap();
        *owner = json!(typed);
    }
    physical::review_predecessor(&schema, &staged_rules, &routing).unwrap();
}

#[test]
fn publication_rejects_coverage_claims_and_missing_or_changed_artifacts() {
    let authoring: Value = dps::read("authoring.json");
    for field in [
        "complete_request",
        "candidate_source_closure",
        "query_closure_changed",
        "existing_owner_closure_changed",
        "new_source_execution",
        "ready_for_native_admission",
    ] {
        let mut changed = authoring.clone();
        changed["scope"][field] = json!(true);
        assert!(physical::review_authoring(&changed).is_err(), "{field}");
    }
    let mut changed = authoring.clone();
    changed["artifacts"].as_array_mut().unwrap().pop();
    assert!(physical::review_authoring(&changed).is_err());
    let mut changed = authoring;
    changed["artifacts"][0]["sha256"] = json!("0".repeat(64));
    assert!(physical::review_authoring(&changed).is_err());
}

#[test]
fn dependency_review_rejects_closure_route_and_inactive_zero_writer_changes() {
    let (schema, rules, routing) = predecessor_fixture();
    let dependencies: Value = dps::read("dependencies.json");
    let mut changed = rules.clone();
    changed["owners"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|owner| owner["owner"] == dependencies["owners_before"][0]["owner"])
        .unwrap()["programs"]["closure"] = json!({"kind":"complete"});
    assert!(physical::review_predecessor(&schema, &changed, &routing).is_err());
    let mut changed_routes = routing.clone();
    changed_routes["outputs"][0]["routes"]["members"]
        .as_array_mut()
        .unwrap()
        .pop();
    assert!(physical::review_predecessor(&schema, &rules, &changed_routes).is_err());
    let component: Value = dps::read("consumer.json");
    let mut program = component["owners"][0]["programs"]["members"][0].clone();
    program["effects"][0]["when"] = json!("inactive-control");
    program["effects"][0]["effect"]["value"] = json!("zero-control");
    let mut changed = rules;
    changed["effect_applications"]["members"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"unlisted-zero-inactive","program":program}));
    assert!(physical::review_predecessor(&schema, &changed, &routing).is_err());
}

#[test]
#[ignore = "explicit immutable paths; preserves all five unchanged imports"]
fn publish_physical_component_preserving_original_queries_and_partial_coverage() {
    publication::run_with_expected_selected_counts(
        std::env::var_os("POE_OPTIMIZER_TEST_PHYSICAL_HIT_DPS_PRIOR")
            .unwrap()
            .into(),
        std::env::var_os("POE_OPTIMIZER_TEST_PHYSICAL_HIT_DPS_OUTPUT")
            .unwrap()
            .into(),
        &dps::data(),
        &[],
        &[
            "authoring.json",
            "dependencies.json",
            "bindings.json",
            "source-evidence.json",
            "source-domain.json",
            "source-vectors.json",
        ],
        physical::stage,
        json!({"new_definitions":4,"new_owner_programs":1,"new_queries":0,
            "complete_request":false,"candidate_source_closure":false,
            "native_admission":false,"exact_component_inverse":true}),
        [107, 118, 110, 124, 5],
    );
}
