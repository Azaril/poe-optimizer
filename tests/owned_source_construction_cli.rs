//! Explicit offline migration proof; no old-format runtime or numerical evaluation.
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_source_construction_preservation.rs"]
mod proof;
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;

use serde_json::{Value, json};
use std::{fs, path::PathBuf, process::Command};

fn publish(input: &std::path::Path, output: &std::path::Path) -> Value {
    let result = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg("assemble-owned-release")
        .arg(input)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout).unwrap()
}

#[test]
#[ignore = "explicit candidate and fresh output paths; preserves five authenticated originals"]
fn source_construction_cutover_preserves_all_five_imports_and_pending_requests() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidate =
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_CONSTRUCTION_CANDIDATE").unwrap());
    let out =
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_CONSTRUCTION_VALIDATION_OUTPUT").unwrap());
    assert!(
        !out.exists(),
        "validation evidence must use a fresh directory"
    );
    let prior = root.join("runs/owned-scenario-stage-inputs-publication-01");
    let package = candidate.join("package");
    let old_files = release::inventory(&prior.join("package"));
    let new_files = release::inventory(&package);
    let staged = release::load(&package);
    let old_receipt = proof::authenticate_cutover(&prior, &candidate, &staged);
    fs::create_dir_all(&out).unwrap();
    assert_eq!(
        publish(&package, &out.join("rebuilt")),
        json!(staged.receipt())
    );
    assert_eq!(release::inventory(&out.join("rebuilt")), new_files);
    let mut cases = Vec::new();
    for case in 1..=5 {
        let source = root.join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let bytes = proof::read_bytes(&source);
        let first = out.join(format!("original-{case:02}"));
        let replay = out.join(format!("replayed-original-{case:02}"));
        release::normalize(&package, &source, case, &first);
        release::normalize(&package, &source, case, &replay);
        cases.push(proof::compare(
            case,
            &bytes,
            &prior,
            &candidate,
            &out,
            &old_receipt,
            &staged,
        ));
        assert_eq!(proof::read_bytes(&source), bytes);
    }
    assert_eq!(release::inventory(&prior.join("package")), old_files);
    assert_eq!(release::inventory(&package), new_files);
    proof::write(
        out.join("validation.json"),
        &json!({
            "before":proof::PRIOR_INPUT,"after":staged.receipt().input,
            "originals":cases,"query_rows":110,"rebuild_byte_identical":true,
            "native_recipe_unchanged":true,"fresh_imports_identical":true,
            "complete_original_builds":0,"retired_input_issues":0,
            "calculation":"not_run","whole_build_parity":"not_established"
        }),
    );
}
