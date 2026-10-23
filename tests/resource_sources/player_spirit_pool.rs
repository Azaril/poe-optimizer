//! Source-backed component publication, without complete-build authority.
#[allow(unused_imports)]
use super::{migration_preservation, preservation, publication, release, selected};
#[allow(dead_code)]
#[path = "../support/owned_player_spirit_pool.rs"]
mod family;
use serde_json::json;
use std::path::PathBuf;
#[test]
fn player_spirit_authors_current_sources_and_retains_open_coverage() {
    family::check_authored();
}
#[test]
fn spirit_retained_source_proof_rejects_metadata_values_and_lost_zero_copy() {
    let proof = family::source_vectors();
    for i in 0..7 {
        let mut p = proof.clone();
        match i {
            0 => p["vectors"][0]["final_spirit"] = json!(114),
            1 => {
                p["vectors"][0]["base_records"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            2 => p["source_metadata"]["source_revision"] = json!("other"),
            3 => p["scope"]["native_supplier_proof"] = json!(true),
            4 => p["archives"][0]["raw"]["sha256"] = json!("other"),
            5 => p["formula_source"]["body"] = json!("other"),
            _ => {
                p["vectors"].as_array_mut().unwrap().pop();
            }
        }
        assert!(std::panic::catch_unwind(|| family::authenticate_source_vectors(&p)).is_err());
    }
}
#[test]
#[ignore = "explicit PLAYER_SPIRIT_POOL_PRIOR/OUTPUT publication; no supplier-completeness authority"]
fn publish_player_spirit_pool_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_PLAYER_SPIRIT_POOL_PRIOR").unwrap()),
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_PLAYER_SPIRIT_POOL_OUTPUT").unwrap()),
        &family::data(),
        &[],
        &[
            "authoring.json",
            "dependencies.json",
            "bindings.json",
            "source-vectors.json",
        ],
        family::stage,
        json!({"new_definitions":5,"new_programs":2,"new_receivers":1,"new_queries":9,"supplier_admission":false,"complete_spirit":false,"whole_build_parity":false}),
        [106, 117, 109, 123, 4],
    );
}
