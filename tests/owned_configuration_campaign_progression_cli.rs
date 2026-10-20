//! Actual campaign progression publication; no build-completion shortcut.
#[path = "support/owned_configuration_campaign_progression_publication.rs"]
mod campaign;
#[path = "support/owned_selected_request.rs"]
mod selected;
use serde_json::json;

#[test]
fn campaign_publication_authenticates_every_packet_and_partial_scope() {
    campaign::review_authoring(&campaign::read("authoring.json")).unwrap();
}

#[test]
fn campaign_publication_refuses_changed_pins_and_completion_claims() {
    let original = campaign::read("authoring.json");
    for field in [
        "complete_request",
        "candidate_source_closure",
        "query_closure_changed",
        "existing_owner_closure_changed",
        "ready_for_native_admission",
    ] {
        let mut changed = original.clone();
        changed["scope"][field] = json!(true);
        assert!(campaign::review_authoring(&changed).is_err(), "{field}");
    }
    let mut changed = original.clone();
    changed["artifacts"].as_array_mut().unwrap().pop();
    assert!(campaign::review_authoring(&changed).is_err());
    let mut changed = original;
    changed["artifacts"][0]["sha256"] = json!("0".repeat(64));
    assert!(campaign::review_authoring(&changed).is_err());
}

#[test]
#[ignore = "explicit immutable publication paths; preserves all five original imports"]
fn publish_campaign_progression_and_preserve_all_five_originals() {
    campaign::run(
        std::env::var_os("POE_OPTIMIZER_TEST_CAMPAIGN_PROGRESSION_PRIOR")
            .unwrap()
            .into(),
        std::env::var_os("POE_OPTIMIZER_TEST_CAMPAIGN_PROGRESSION_OUTPUT")
            .unwrap()
            .into(),
    );
}
