//! One ordinary-cost contribution; all prior original-build gaps remain.
#[path = "support/owned_encroaching_ground_support_delivery.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[path = "support/owned_encroaching_ground_native.rs"]
mod native;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;

#[path = "support/owned_support_delivery_publication.rs"]
mod publication;
use serde_json::{Value, json};
use std::path::PathBuf;
const EFFECTS: &[&str] = &["SupportEncroachingGroundPlayer"];
fn carries_authoring(value: &Value, payloads: &[Value]) -> bool {
    publication::carries_authoring(value, payloads, EFFECTS)
}

#[test]
fn authored_encroaching_ground_preserves_prepared_inputs_and_partial_owners() {
    family::check_authored();
}

#[test]
fn encroaching_publication_boundary_excludes_authoring_payload_but_keeps_commitments() {
    let payloads: Vec<Value> = ["receiving.json", "preparation.json", "source-vectors.json"]
        .into_iter()
        .map(family::read)
        .collect();
    for payload in &payloads {
        assert!(carries_authoring(&json!({"nested":[payload]}), &payloads));
    }
    assert!(carries_authoring(
        &json!({"source":"skills[\"SupportEncroachingGroundPlayer\"] = { support = true }"}),
        &payloads
    ));
    assert!(carries_authoring(
        &json!(["skills[\"SupportEncroachingGroundPlayer\"] = { support = true }"]),
        &payloads
    ));
    assert!(!carries_authoring(
        &json!({"kind":family::KIND,"authoring_input":family::authoring_digest()}),
        &payloads
    ));
}

#[test]
#[ignore = "requires passed source witness, POE_OPTIMIZER_TEST_ENCROACHING_PRIOR and fresh POE_OPTIMIZER_TEST_ENCROACHING_OUTPUT"]
fn publish_encroaching_ground_preserving_five_originals() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_ENCROACHING_PRIOR")
            .expect("checked Rapid predecessor"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_ENCROACHING_OUTPUT")
            .expect("new publication directory"),
    );
    publication::run(
        prior_path,
        out,
        &family::data(""),
        EFFECTS,
        family::stage,
        json!({"new_definitions":0,"new_programs":2,"final_resource_cost_claimed":false,"ground_growth_claimed":false,"reservation_contributions_added":false}),
    );
}
