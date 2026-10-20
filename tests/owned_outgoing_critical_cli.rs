//! Partial critical-component publication, with independent unchanged-build gates.
#[path = "support/owned_outgoing_critical_publication.rs"]
mod critical_publication;
#[allow(dead_code)]
#[path = "support/owned_outgoing_critical.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[allow(dead_code)]
#[path = "support/owned_outgoing_critical_domain.rs"]
mod ordinary;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_support_delivery_publication.rs"]
mod publication;
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_outgoing_critical_rounding.rs"]
mod rounding;
#[path = "support/owned_selected_request.rs"]
mod selected;
use serde_json::{Value, json};

fn predecessor_fixture() -> (Value, Value, Value) {
    let d = family::read("dependencies.json");
    let mut owners = d["owners_before"].as_array().unwrap().clone();
    owners.push(d["baseline_owner_before"].clone());
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
    // Preserve the authentic writer order, independently of unrelated fixture owners.
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
fn authored_component_retains_partial_queries_and_publication_boundaries() {
    critical_publication::check_authored();
    let (schema, rules, routes) = predecessor_fixture();
    critical_publication::review_predecessor(&schema, &rules, &routes).unwrap();
}

#[test]
fn publication_refuses_missing_pins_and_any_coverage_claim() {
    let original = family::read("authoring.json");
    for field in [
        "complete_request",
        "candidate_source_closure",
        "query_closure_changed",
        "existing_owner_closure_changed",
        "ready_for_native_admission",
    ] {
        let mut changed = original.clone();
        changed["scope"][field] = json!(true);
        assert!(
            critical_publication::review_authoring(&changed).is_err(),
            "{field}"
        );
    }
    let mut changed = original.clone();
    changed["artifacts"].as_array_mut().unwrap().pop();
    assert!(critical_publication::review_authoring(&changed).is_err());
    let mut changed = original;
    changed["artifacts"][0]["sha256"] = json!("0".repeat(64));
    assert!(critical_publication::review_authoring(&changed).is_err());
}

#[test]
fn dependency_review_refuses_changed_owner_route_and_zero_inactive_application_writer() {
    let (schema, rules, routes) = predecessor_fixture();
    let mut changed = rules.clone();
    changed["owners"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|o| o["owner"] == family::read("dependencies.json")["owners_before"][0]["owner"])
        .unwrap()["programs"]["closure"] = json!({"kind":"complete"});
    assert!(critical_publication::review_predecessor(&schema, &changed, &routes).is_err());
    let mut changed_routes = routes.clone();
    changed_routes["outputs"][0]["routes"]["members"]
        .as_array_mut()
        .unwrap()
        .pop();
    assert!(critical_publication::review_predecessor(&schema, &rules, &changed_routes).is_err());
    // The census inspects potential effects without evaluating their condition/value.
    let mut p = family::read("consumer.json")["owners"][1]["programs"]["members"][0].clone();
    p["effects"][0]["when"] = json!("inactive-control");
    p["effects"][0]["effect"]["value"] = json!("zero-control");
    let mut changed = rules;
    changed["effect_applications"]["members"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"unlisted-zero-inactive","program":p}));
    assert!(critical_publication::review_predecessor(&schema, &changed, &routes).is_err());
}

#[test]
#[ignore = "explicit immutable publication paths; preserves all five original imports"]
fn publish_partial_critical_component_and_preserve_all_five_originals() {
    publication::run_with_expected_selected_counts(
        std::env::var_os("POE_OPTIMIZER_TEST_OUTGOING_CRITICAL_PRIOR")
            .unwrap()
            .into(),
        std::env::var_os("POE_OPTIMIZER_TEST_OUTGOING_CRITICAL_OUTPUT")
            .unwrap()
            .into(),
        &family::data(),
        &[],
        &[
            "authoring.json",
            "dependencies.json",
            "bindings.json",
            "source-evidence.json",
            "source-domain.json",
            "source-vectors.json",
            "ordinary-source-domain.json",
            "rounding-domain.json",
        ],
        critical_publication::stage,
        json!({"new_definitions":7,"new_owner_programs":2,"replaced_programs":1,
            "new_queries":16,"complete_request":false,"candidate_source_closure":false,
            "native_admission":false,"exact_component_inverse":true}),
        [106, 117, 109, 123, 4],
    );
}
