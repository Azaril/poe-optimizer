//! Actual Offering input assembly using the accepted source-property contract.
use native::fixture::component as family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[path = "support/owned_offering_final_inputs_native.rs"]
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
fn authored_offering_projects_real_final_inputs_without_intermediate_writers() {
    family::check_authored();
}

#[test]
#[ignore = "requires passed source evidence and explicit OFFERING_INPUTS_PRIOR/OUTPUT paths"]
fn publish_offering_final_inputs_preserving_five_originals() {
    publication::run_with_payloads(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_OFFERING_INPUTS_PRIOR")
                .expect("checked Prolonged predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_OFFERING_INPUTS_OUTPUT")
                .expect("new output directory"),
        ),
        &family::data(""),
        &[
            "PainOfferingPlayer",
            "ProlongedDurationSupportPlayer",
            "ProlongedDurationSupportPlayerTwo",
        ],
        &[
            "source-properties.json",
            "preparation.json",
            "readiness.json",
            "source-vectors.json",
        ],
        family::stage,
        json!({"new_definitions":0,"new_slots":0,"new_programs":1,
            "replaced_programs":1,"source_relation_reference_only":true,
            "physical_final_scalar_writers_added":false,"final_duration_claimed":false}),
    );
}
