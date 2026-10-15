//! Reporting preferences do not authorize aggregate evaluation or gameplay effects.
#[allow(dead_code)]
#[path = "support/owned_reporting_inclusion.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_reporting_inclusion_publication.rs"]
mod publication;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use serde_json::json;
use std::path::PathBuf;

#[test]
fn reporting_intent_authenticates_source_and_exact_eligibility_appends() {
    family::check_authored();
}
#[test]
fn reporting_source_proof_rejects_value_scope_body_and_identity_mutations() {
    let proof = family::source();
    for i in 0..5 {
        let mut changed = proof.clone();
        match i {
            0 => changed["source_revision"] = json!("other"),
            1 => changed["semantics"]["nil"] = json!(true),
            2 => changed["semantics"]["aggregate_execution"] = json!(true),
            3 => changed["archives"][0]["source"]["sha256"] = json!("other"),
            _ => {
                changed["archives"].as_array_mut().unwrap().pop();
            }
        }
        assert!(std::panic::catch_unwind(|| family::authenticate_source(&changed)).is_err());
    }
}
#[test]
#[ignore = "explicit REPORTING_INCLUSION_PRIOR/OUTPUT publication; no aggregate execution"]
fn publish_reporting_inclusion_preserving_all_five_originals() {
    publication::run(
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_REPORTING_INCLUSION_PRIOR").unwrap()),
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_REPORTING_INCLUSION_OUTPUT").unwrap()),
    );
}
