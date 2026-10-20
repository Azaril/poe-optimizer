//! Publish real saved stage intent without claiming complete applicability.
#[allow(dead_code)]
#[path = "support/owned_scenario_stage_inputs.rs"]
mod family;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
#[path = "support/owned_scenario_stage_preservation.rs"]
mod stages;

use poe_optimizer_import::owned_release::StagedOwnedRelease;
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::Path, process::Command};

fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
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
fn canonical_lineage(value: &mut Value, expected: &Value) {
    match value {
        Value::Object(fields) => {
            if let Some(lineage) = fields.get_mut("lineage") {
                assert_eq!(lineage, expected, "one authenticated project identity");
                *lineage = json!("canonical-test-lineage");
            }
            for child in fields.values_mut() {
                canonical_lineage(child, expected);
            }
        }
        Value::Array(children) => {
            for child in children {
                canonical_lineage(child, expected);
            }
        }
        _ => {}
    }
}
fn authenticated_outputs(
    directory: &Path,
    package: &Path,
    case: usize,
    release: &StagedOwnedRelease,
) -> [Value; 3] {
    let mut draft = read(directory.join("draft.json"));
    let bytes = fs::read(directory.join("sidecar.json")).unwrap();
    let mut sidecar: Value = serde_json::from_slice(&bytes).unwrap();
    let mut report = read(directory.join("report.json"));
    preservation::authenticate(&sidecar, directory, package, case, release);
    let lineage = sidecar["allocator_before"]["lineage"].clone();
    assert!(lineage.is_string());
    assert_eq!(report["allocator_before"], sidecar["allocator_before"]);
    assert_eq!(report["allocator_after"], sidecar["allocator_after"]);
    assert_eq!(report["draft_digest"], sidecar["draft"]);
    assert_eq!(
        report["sidecar_sha256"],
        json!(format!("{:x}", Sha256::digest(&bytes)))
    );
    assert_eq!(report["sidecar_bytes"], json!(bytes.len()));
    assert_eq!(
        report["output"]["directory"],
        json!(fs::canonicalize(directory).unwrap())
    );
    // The CLI intentionally creates a fresh project. Only those authenticated
    // identities, derived commitments and actual output paths may differ.
    sidecar["draft"] = Value::Null;
    report["draft_digest"] = Value::Null;
    report["sidecar_sha256"] = Value::Null;
    report["sidecar_bytes"] = Value::Null;
    report["output"]["directory"] = Value::Null;
    for value in [&mut draft, &mut sidecar, &mut report] {
        canonical_lineage(value, &lineage);
    }
    [draft, sidecar, report]
}

#[test]
#[ignore = "explicit immutable paths; publishes and preserves all five unchanged originals"]
fn publish_stage_intents_with_exact_sources_and_incomplete_applicability() {
    let prior_path = std::path::PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_SCENARIO_STAGES_PRIOR").unwrap(),
    );
    let out = std::path::PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_SCENARIO_STAGES_OUTPUT").unwrap(),
    );
    assert!(!out.exists(), "immutable checkpoint output");
    family::check_authored().unwrap();
    let prior_files = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = family::stage(&prior);
    assert_eq!(prior.receipt().input.to_string(), family::PREDECESSOR);
    assert_ne!(next.receipt().definitions, prior.receipt().definitions);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    fs::create_dir_all(&out).unwrap();
    write(out.join("endpoint.json"), next.input());
    let package = out.join("package");
    assert_eq!(
        publish(&out.join("endpoint.json"), &package),
        json!(next.receipt())
    );
    assert_eq!(
        publish(&package, &out.join("rebuilt")),
        json!(next.receipt())
    );
    assert_eq!(
        release::inventory(&package),
        release::inventory(&out.join("rebuilt"))
    );
    assert_eq!(
        release::inventory(&package).keys().collect::<Vec<_>>(),
        prior_files.keys().collect::<Vec<_>>()
    );
    for query in prior.query_sets() {
        let file = format!("queries-{}.json", query.name.as_str());
        assert_eq!(
            fs::read(prior_path.join(&file)).unwrap(),
            fs::read(package.join(&file)).unwrap()
        );
    }
    let mut originals = Vec::new();
    for case in 1..=5 {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let bytes = fs::read(&source).unwrap();
        let before = out.join(format!("prior-original-{case:02}"));
        let after = out.join(format!("original-{case:02}"));
        let replay = out.join(format!("replayed-original-{case:02}"));
        release::normalize(&prior_path, &source, case, &before);
        release::normalize(&package, &source, case, &after);
        originals.push(stages::compare(
            case,
            &bytes,
            &out,
            &prior_path,
            &package,
            &prior,
            &next,
        ));
        release::normalize(&package, &source, case, &replay);
        assert_eq!(
            authenticated_outputs(&after, &package, case, &next),
            authenticated_outputs(&replay, &package, case, &next),
            "fresh imports preserve complete content and diagnostics"
        );
        assert_eq!(fs::read(source).unwrap(), bytes);
    }
    assert_eq!(release::inventory(&prior_path), prior_files);
    write(
        out.join("validation.json"),
        &json!({
            "before":prior.receipt().input,"after":next.receipt().input,
            "originals":originals,"queries":110,
            "rebuild_byte_identical":true,"fresh_imports_identical":true,
            "prior_unchanged":true,"complete_original_builds":0,
            "evaluation_bundle_added":false,"retired_input_issues":0,
            "exact_import_inverse":true,"complete_skill_applicability":false
        }),
    );
}
