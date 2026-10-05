//! Magnified Area publication preserves every original request and open obligation.
#[path = "support/owned_magnified_area_support_delivery.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[path = "support/owned_magnified_area_native.rs"]
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

fn carries_authoring_payload(
    value: &Value,
    receiving: &Value,
    vectors: &Value,
    declarations: &[String],
) -> bool {
    if value == receiving || value == vectors {
        return true;
    }
    match value {
        Value::String(text) => declarations
            .iter()
            .any(|source| text.contains(source) || text.contains(source.lines().next().unwrap())),
        Value::Array(rows) => rows
            .iter()
            .any(|row| carries_authoring_payload(row, receiving, vectors, declarations)),
        Value::Object(fields) => fields
            .values()
            .any(|row| carries_authoring_payload(row, receiving, vectors, declarations)),
        _ => false,
    }
}

#[test]
fn publication_boundary_rejects_embedded_authoring_but_keeps_commitments() {
    let receiving: Value = family::read("receiving.json");
    let vectors: Value = family::read("source-vectors.json");
    let declarations: Vec<_> = vectors["source_declarations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["declaration"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(declarations.len(), 2);
    for payload in [
        receiving.clone(),
        vectors.clone(),
        json!(declarations[0]),
        json!({"nested":[{"source":declarations[1]}]}),
    ] {
        assert!(carries_authoring_payload(
            &payload,
            &receiving,
            &vectors,
            &declarations
        ));
    }
    let a: Value = family::read("authoring.json");
    assert!(!carries_authoring_payload(
        &json!({"kind":"magnified-area-support-delivery-v1","prior_input":a["before"],
            "authoring_input":a["artifact_sha256"]["source-vectors"]}),
        &receiving,
        &vectors,
        &declarations,
    ));
}

#[test]
fn authored_magnified_area_preserves_costs_conditions_and_partial_coverage() {
    family::check_authored();
}

#[test]
#[ignore = "requires passed complete-source evidence, POE_OPTIMIZER_TEST_MAGNIFIED_PRIOR and a new POE_OPTIMIZER_TEST_MAGNIFIED_OUTPUT directory"]
fn publish_magnified_area_preserving_all_five_originals() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_MAGNIFIED_PRIOR").expect("exact predecessor package"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_MAGNIFIED_OUTPUT").expect("new output directory"),
    );
    assert!(!out.exists(), "publication must not overwrite evidence");
    let old_files = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = family::stage(&prior);
    assert_eq!(next.receipt().query_rows, 110);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert!(prior.evaluation().is_none() && next.evaluation().is_none());
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
        old_files.keys().collect::<Vec<_>>(),
        "authoring evidence must not add or replace a runtime/import artifact"
    );
    assert_eq!(files, release::inventory(&out.join("rebuilt")));
    let receiving: Value = family::read("receiving.json");
    let vectors: Value = family::read("source-vectors.json");
    let declarations: Vec<_> = vectors["source_declarations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["declaration"].as_str().unwrap().to_owned())
        .collect();
    for name in files.keys() {
        assert!(
            ![
                "source-vectors.json",
                "receiving.json",
                "authoring.json",
                "dependencies.json",
                "bindings.json",
                "migration.json"
            ]
            .contains(&name.as_str())
        );
        let document: Value =
            serde_json::from_slice(&fs::read(package.join(name)).unwrap()).unwrap();
        assert!(
            !carries_authoring_payload(&document, &receiving, &vectors, &declarations),
            "authoring-only evidence leaked into {name}"
        );
    }
    // Evidence is committed by the exact checked provenance digest, not copied
    // into an evaluator artifact. Preserve every historical receipt as well.
    let published: Value =
        serde_json::from_slice(&fs::read(package.join("release.json")).unwrap()).unwrap();
    assert_eq!(published["provenance"], json!(next.receipt().provenance));
    assert_eq!(
        next.receipt().provenance.len(),
        prior.receipt().provenance.len() + 1
    );
    assert_eq!(
        next.receipt().provenance[..prior.receipt().provenance.len()],
        prior.receipt().provenance
    );
    assert_eq!(
        published["provenance"].as_array().unwrap().last().unwrap()["kind"],
        "magnified-area-support-delivery-v1"
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
        let result = preservation::compare_original(case, &bytes, &comparison);
        assert_eq!(result["physical_lists_completed"], 0);
        assert_eq!(fs::read(&source).unwrap(), bytes);
        originals.push(result);
    }
    assert_eq!(old_files, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({
            "before":prior.receipt().input,"after":next.receipt().input,"originals":originals,
            "queries":110,"artifacts":18,"rebuild_byte_identical":true,"prior_unchanged":true,
            "new_definitions":3,"new_programs":5,"closed_rule_owners":0,"retired_input_issues":0,
            "complete_original_builds":0,"evaluation_bundle_added":false,
            "receiving_fragment_only":true,"final_resource_cost_or_radius_claimed":false,
            "authoring_payloads_excluded":true,"provenance_preserved":true
        }),
    );
}
