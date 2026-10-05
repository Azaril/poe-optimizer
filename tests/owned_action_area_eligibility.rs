//! Publish an owned Action predicate without completing any original build.
#[path = "support/owned_action_area_eligibility.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[path = "support/owned_action_area_eligibility_native.rs"]
mod native;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;

use serde::Serialize;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn write(path: impl AsRef<Path>, value: &impl Serialize) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}
fn publish(input: &Path, output: &Path) -> Value {
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
fn authored_area_eligibility_preserves_exact_owner_bodies_and_unknown_domain() {
    family::check_authored();
}

#[test]
#[ignore = "requires passed source census, POE_OPTIMIZER_TEST_AREA_PRIOR and fresh POE_OPTIMIZER_TEST_AREA_OUTPUT"]
fn publish_action_area_eligibility_preserving_five_originals() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_AREA_PRIOR").expect("checked predecessor"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_AREA_OUTPUT").expect("new publication directory"),
    );
    assert!(!out.exists(), "evidence is immutable");
    let prior_files = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = family::stage(&prior);
    fs::create_dir_all(&out).unwrap();
    write(out.join("endpoint.json"), next.input());
    write(out.join("receipt.json"), next.receipt());
    let package = out.join("package");
    assert_eq!(
        publish(&out.join("endpoint.json"), &package),
        json!(next.receipt())
    );
    assert_eq!(
        publish(&package, &out.join("rebuilt")),
        json!(next.receipt())
    );
    let files = release::inventory(&package);
    assert_eq!(files.len(), 18);
    assert_eq!(
        files.keys().collect::<Vec<_>>(),
        prior_files.keys().collect::<Vec<_>>()
    );
    assert_eq!(files, release::inventory(&out.join("rebuilt")));
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert_eq!(
        next.receipt().provenance.len(),
        prior.receipt().provenance.len() + 1
    );
    assert_eq!(
        next.receipt().provenance[..prior.receipt().provenance.len()],
        prior.receipt().provenance
    );
    let comparison = preservation::Comparison {
        prior: &prior,
        next: &next,
        prior_path: &prior_path,
        package: &package,
        out: &out,
        families: &[],
        selected_before: [106, 117, 109, 122, 5],
        selected_after: [106, 117, 109, 122, 5],
        rebind_definitions: true,
    };
    let mut originals = vec![];
    for case in 1..=5 {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let bytes = fs::read(&source).unwrap();
        release::normalize(
            &prior_path,
            &source,
            case,
            &out.join(format!("prior-original-{case:02}")),
        );
        release::normalize(
            &package,
            &source,
            case,
            &out.join(format!("original-{case:02}")),
        );
        let row = preservation::compare_original(case, &bytes, &comparison);
        assert_eq!(row["physical_lists_completed"], 0);
        assert_eq!(fs::read(&source).unwrap(), bytes);
        originals.push(row);
    }
    assert_eq!(prior_files, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({
            "before":prior.receipt().input,"after":next.receipt().input,"originals":originals,
            "queries":110,"artifacts":18,"rebuild_byte_identical":true,"prior_unchanged":true,
            "new_definitions":0,"new_programs":11,"closed_rule_owners":0,"retired_input_issues":0,
            "complete_original_builds":0,"evaluation_bundle_added":false,
            "raw_source_or_lua_runtime_embedded":false,"final_radius_claimed":false,
        }),
    );
}
