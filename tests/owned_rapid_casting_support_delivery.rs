//! One cast-speed contribution channel; all prior original-build gaps remain.
#[path = "support/owned_rapid_casting_support_delivery.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[path = "support/owned_rapid_casting_native.rs"]
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
fn carries_authoring(value: &Value, payloads: &[Value]) -> bool {
    if payloads.contains(value) {
        return true;
    }
    match value {
        Value::String(text) => {
            text.contains("skills[\"SupportRapidCastingPlayer\"]")
                || text.contains("skills[\"SupportRapidCastingPlayerTwo\"]")
        }
        Value::Array(rows) => rows.iter().any(|r| carries_authoring(r, payloads)),
        Value::Object(fields) => fields.values().any(|r| carries_authoring(r, payloads)),
        _ => false,
    }
}

#[test]
fn authored_rapid_casting_preserves_prepared_inputs_and_partial_owners() {
    family::check_authored();
}

#[test]
fn rapid_publication_boundary_excludes_authoring_payload_but_keeps_commitments() {
    let payloads: Vec<Value> = ["receiving.json", "preparation.json", "source-vectors.json"]
        .into_iter()
        .map(family::read)
        .collect();
    for payload in &payloads {
        assert!(carries_authoring(&json!({"nested":[payload]}), &payloads));
    }
    assert!(carries_authoring(
        &json!({"source":"skills[\"SupportRapidCastingPlayer\"] = { support = true }"}),
        &payloads
    ));
    assert!(carries_authoring(
        &json!(["skills[\"SupportRapidCastingPlayerTwo\"] = { support = true }"]),
        &payloads
    ));
    assert!(!carries_authoring(
        &json!({"kind":family::KIND,"authoring_input":family::authoring_digest()}),
        &payloads
    ));
}

#[test]
#[ignore = "requires passed source witness, POE_OPTIMIZER_TEST_RAPID_PRIOR and fresh POE_OPTIMIZER_TEST_RAPID_OUTPUT"]
fn publish_rapid_casting_preserving_five_originals() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_RAPID_PRIOR").expect("checked Area predecessor"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_RAPID_OUTPUT").expect("new publication directory"),
    );
    assert!(!out.exists(), "evidence is immutable");
    let prior_files = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    // This gate includes exact source report authentication. An awaiting-source
    // packet must fail before the output directory or package is created.
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
    let payloads: Vec<Value> = ["receiving.json", "preparation.json", "source-vectors.json"]
        .into_iter()
        .map(family::read)
        .collect();
    for name in files.keys() {
        assert!(
            ![
                "receiving.json",
                "preparation.json",
                "source-vectors.json",
                "authoring.json",
                "bindings.json",
                "dependencies.json",
                "migration.json"
            ]
            .contains(&name.as_str())
        );
        let document: Value =
            serde_json::from_slice(&fs::read(package.join(name)).unwrap()).unwrap();
        assert!(
            !carries_authoring(&document, &payloads),
            "authoring payload embedded in {name}"
        );
    }
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
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
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
            "new_definitions":1,"new_programs":4,"closed_rule_owners":0,"retired_input_issues":0,
            "complete_original_builds":0,"evaluation_bundle_added":false,"receiving_fragment_only":true,
            "authoring_payloads_excluded":true,"provenance_preserved":true,
            "final_cast_time_claimed":false,"cost_or_reservation_contributions_added":false,
        }),
    );
}
