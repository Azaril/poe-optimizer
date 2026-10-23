//! Authenticated final Life arithmetic; whole-build admission stays open.
#[allow(unused_imports)]
use super::{migration_preservation, preservation, publication, release, selected};
#[allow(dead_code)]
#[path = "../support/owned_player_life_pool.rs"]
mod family;
use serde_json::json;
use std::path::PathBuf;

#[test]
fn player_life_formula_has_exact_player_receivers_and_open_supplier_domains() {
    family::check_authored();
}

#[test]
fn retained_source_vectors_reject_changed_values_scope_and_missing_cases() {
    let original = family::source_vectors();
    for index in 0..9 {
        let mut altered = original.clone();
        match index {
            0 => altered["vectors"][0]["inputs"]["base"] = json!(1258),
            1 => altered["vectors"][8]["inputs"]["chaos_inoculation"] = json!(false),
            2 => altered["vectors"][0]["final_life"] = json!(1321),
            3 => {
                altered["vectors"].as_array_mut().unwrap().pop();
            }
            4 => altered["scope"]["native_supplier_proof"] = json!(true),
            5 => altered["archives"][0]["raw"]["sha256"] = json!("wrong"),
            6 => altered["source_metadata"]["source_revision"] = json!("wrong"),
            7 => altered["source_manifest"]["sha256"] = json!("wrong"),
            _ => altered["formula_source"]["body"] = json!("wrong"),
        }
        assert!(
            std::panic::catch_unwind(|| family::authenticate_source_vectors(&altered)).is_err()
        );
    }
}

#[test]
#[ignore = "explicit PLAYER_LIFE_POOL_PRIOR/OUTPUT publication; no source-completeness authority"]
fn publish_player_life_pool_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_PLAYER_LIFE_POOL_PRIOR").unwrap()),
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_PLAYER_LIFE_POOL_OUTPUT").unwrap()),
        &family::data(),
        &[],
        &[
            "authoring.json",
            "dependencies.json",
            "bindings.json",
            "source-vectors.json",
        ],
        family::stage,
        json!({"new_definitions":6,"new_programs":1,"new_receivers":1,"new_queries":7,
            "supplier_admission":false,"complete_life":false,"whole_build_parity":false}),
        [106, 117, 109, 123, 4],
    );
}
