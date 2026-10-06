//! Checked pre-Amulet snapshot endpoint; source and readiness payloads stay offline.
#[path = "support/owned_amulet_bonus_snapshot.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[path = "support/owned_amulet_bonus_snapshot_native.rs"]
mod native;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_support_delivery_publication.rs"]
mod publication;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use serde_json::json;
use std::path::PathBuf;

#[test]
fn authored_amulet_snapshot_distinguishes_incoming_contributions_from_scalar() {
    family::check_authored();
}

#[test]
fn snapshot_publication_excludes_nested_authoring_payloads_but_keeps_commitments() {
    let payload = json!({"reference_only":true,"stages":[{"id":"snapshot"}]});
    assert!(publication::carries_authoring(
        &json!({"nested":[payload.clone()]}),
        std::slice::from_ref(&payload),
        &[]
    ));
    assert!(!publication::carries_authoring(
        &json!({"provenance":[{"kind":family::KIND,
        "authoring_input":"evidence-content-digest"}]}),
        &[payload],
        &[]
    ));
}

#[test]
#[ignore = "requires passed snapshot evidence and explicit AMULET_SNAPSHOT_PRIOR/OUTPUT paths"]
fn publish_amulet_bonus_snapshot_preserving_five_originals() {
    publication::run_with_payloads(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_AMULET_SNAPSHOT_PRIOR")
                .expect("checked Offering final-inputs predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_AMULET_SNAPSHOT_OUTPUT")
                .expect("new immutable snapshot output directory"),
        ),
        &family::data(""),
        &[],
        &["readiness.json", "source-vectors.json"],
        family::stage,
        json!({"new_definitions":0,"new_slots":0,"new_programs":1,"new_receivers":1,
            "new_snapshot_program_inventory_complete":true,
            "incoming_contributor_inventory_closed":false,"readiness_reference_only":true}),
    );
}
