//! Physical Sniper raw inputs through actual item preparation and final assembly.
#[path = "support/owned_sniper_final_inputs.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[path = "support/owned_sniper_final_inputs_native.rs"]
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
fn authored_sniper_final_inputs_preserve_existing_contracts() {
    family::check_authored();
}

#[test]
#[ignore = "requires checked SNIPER_INPUTS_PRIOR and a fresh SNIPER_INPUTS_OUTPUT"]
fn publish_sniper_final_inputs_preserving_five_originals() {
    publication::run_with_payloads(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_SNIPER_INPUTS_PRIOR")
                .expect("checked predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_SNIPER_INPUTS_OUTPUT").expect("fresh output"),
        ),
        &family::data(),
        &["SummonSkeletalSnipersPlayer"],
        &[
            "source-properties.json",
            "readiness.json",
            "source-vectors.json",
        ],
        family::stage,
        json!({"new_definitions":0,"new_programs":1,"closed_existing_rule_owners":0,
            "real_support_origin_discovery_completed":false,"source_relation_reference_only":true}),
    );
}
