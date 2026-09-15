//! The CLI shares the portable metric API; no PoB checkout or snapshot is read.
#[path = "../crates/poe-optimizer-engine/tests/support/owned_metric_fixture.rs"]
#[allow(dead_code)]
mod support;
use poe_optimizer_core::{owned_build::*, owned_routing::*, owned_rules::*};
use poe_optimizer_data::owned_schema::*;
use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
use support::*;
fn save(dir: &Path, f: &Fixture) {
    let schema =
        OwnedDefinitionSchemaPackage::new(f.schema.clone(), OwnedSchemaLimits::default()).unwrap();
    fs::write(
        dir.join("request.json"),
        encode_owned(
            &OwnedDocument::Request(Box::new(f.request())),
            OwnedInputLimits::default(),
        )
        .unwrap(),
    )
    .unwrap();
    fs::write(
        dir.join("schema.json"),
        encode_schema_package(&schema, OwnedSchemaLimits::default()).unwrap(),
    )
    .unwrap();
    let rules = RulePackageInput {
        tables: f.tables.clone(),
        schema_version: OWNED_RULE_PACKAGE_VERSION,
        namespace: ns(),
        release: key("rules"),
        semantics_version: key("test-v1"),
        operations_version: key(OWNED_RULE_OPERATIONS_VERSION),
        definitions: schema.identity().clone(),
        owners: f.owners.clone(),
    };
    let routing = ActionRoutingInput {
        schema_version: OWNED_ACTION_ROUTING_VERSION,
        namespace: ns(),
        release: key("routing"),
        definitions: schema.identity().clone(),
        outputs: f.routes.clone(),
    };
    fs::write(dir.join("rules.json"), serde_json::to_vec(&rules).unwrap()).unwrap();
    fs::write(
        dir.join("routing.json"),
        serde_json::to_vec(&routing).unwrap(),
    )
    .unwrap();
    fs::write(
        dir.join("metrics.json"),
        serde_json::to_vec(&mapping_input(&schema)).unwrap(),
    )
    .unwrap();
}
fn run(dir: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(dir)
        .args([
            "evaluate-owned",
            "--input",
            "request.json",
            "--schema",
            "schema.json",
            "--rules",
            "rules.json",
            "--routing",
            "routing.json",
            "--metrics",
            "metrics.json",
        ])
        .args(extra)
        .output()
        .unwrap()
}
#[test]
fn ordered_cli_results_equal_native_api_and_changed_input_without_legacy_files() {
    for level in [11, 37] {
        let dir = tempfile::tempdir().unwrap();
        let mut f = fixture();
        f.build
            .gems
            .iter_mut()
            .find(|g| g.id == occurrence(28))
            .unwrap()
            .level = level;
        save(dir.path(), &f);
        let output = run(dir.path(), &["--output", "report.json"]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            fs::read(dir.path().join("report.json")).unwrap(),
            output.stdout
        );
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        let plan = compile(&f);
        assert_eq!(
            report["evaluation"],
            serde_json::to_value(plan.evaluate(&mut plan.new_scratch()).unwrap()).unwrap()
        );
        assert_eq!(
            report["bindings"],
            serde_json::to_value(plan.bindings()).unwrap()
        );
        assert_eq!(report["document_kind"], "owned_metric_report");
        assert_eq!(report["verification"]["game_legality"], "not_checked");
        assert_eq!(
            report["verification"]["whole_build_parity"],
            "not_established"
        );
        assert_eq!(report["verification"]["contributor_closure"], "whole_plan");
        let prior = output.stdout;
        assert!(
            !run(dir.path(), &["--output", "report.json"])
                .status
                .success()
        );
        assert_eq!(fs::read(dir.path().join("report.json")).unwrap(), prior);
    }
}
#[test]
fn stale_or_unknown_metric_artifacts_fail_before_output_publication() {
    let dir = tempfile::tempdir().unwrap();
    save(dir.path(), &fixture());
    let file = dir.path().join("metrics.json");
    let original = fs::read(&file).unwrap();
    let mut value: Value = serde_json::from_slice(&original).unwrap();
    value["source_callback"] = Value::String("forbidden".into());
    fs::write(&file, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(
        !run(dir.path(), &["--output", "report.json"])
            .status
            .success()
    );
    assert!(!dir.path().join("report.json").exists());
    let mut value: Value = serde_json::from_slice(&original).unwrap();
    value["definitions"]["content_sha256"] = Value::String("0".repeat(64));
    fs::write(&file, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(
        !run(dir.path(), &["--output", "report.json"])
            .status
            .success()
    );
    assert!(!dir.path().join("report.json").exists());
}
