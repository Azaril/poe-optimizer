#![cfg(feature = "pob")]

use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn evaluate(path: &Path) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(root())
        .arg("evaluate")
        .arg(path)
        .arg("--timeout-seconds")
        .arg("60")
        .arg("--raw")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut report: Value =
        serde_json::from_slice(&output.stdout).expect("CLI emits one JSON document");
    let raw = report["evaluation"]["attachments"][0]["content"]
        .as_str()
        .unwrap();
    let snapshot: Value = serde_json::from_str(raw).unwrap();
    report["evaluation"] = snapshot;
    report
}

fn assert_actor_parity(left: &Value, right: &Value) {
    for field in ["skill_name", "skill_id", "non_finite_metrics"] {
        assert_eq!(left[field], right[field], "actor field {field}");
    }
    let left_metrics = left["metrics"].as_object().unwrap();
    let right_metrics = right["metrics"].as_object().unwrap();
    assert_eq!(left_metrics.len(), right_metrics.len());
    for (key, expected) in left_metrics {
        let expected = expected.as_f64().unwrap();
        let actual = right_metrics[key].as_f64().unwrap();
        let tolerance = 1e-8 * expected.abs().max(1.0);
        assert!(
            (actual - expected).abs() <= tolerance,
            "{key}: {expected} vs {actual}"
        );
    }
}

#[test]
fn fresh_workers_preserve_fixture_outputs_across_level_change_and_export() {
    let original = root().join("tests/fixtures/builds/pobarchives-Dfz36mCq.xml");
    let first = evaluate(&original);
    assert_eq!(first["status"], "experimental_evaluation");
    assert_eq!(first["evaluation"]["build"]["level"], 96);
    assert_eq!(
        first["evaluation"]["build"]["allocated_nodes"]
            .as_array()
            .unwrap()
            .len(),
        130
    );
    assert_eq!(
        first["evaluation"]["player"]["skill_id"],
        "SummonSandDjinnPlayer"
    );
    assert_eq!(
        first["evaluation"]["minion"]["skill_id"],
        "ExplosiveTeleportSandDjinn"
    );
    assert!(
        first["evaluation"]["player"]["non_finite_metrics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|name| name == "ChaosMaximumHitTaken")
    );
    let warnings = first["evaluation"]["warnings"].as_array().unwrap();
    assert_eq!(
        warnings
            .iter()
            .filter(|s| s.as_str().unwrap().starts_with("Unresolved skill"))
            .count(),
        3
    );
    assert!(
        !first["evaluation"]["diagnostics"]
            .as_str()
            .unwrap()
            .is_empty()
    );

    let scratch = tempfile::tempdir().unwrap();
    let changed = scratch.path().join("level95.xml");
    let source = fs::read_to_string(&original).unwrap();
    assert!(source.contains("level=\"96\""));
    fs::write(&changed, source.replacen("level=\"96\"", "level=\"95\"", 1)).unwrap();
    assert_eq!(evaluate(&changed)["evaluation"]["build"]["level"], 95);
    let again = evaluate(&original);
    assert_eq!(first["evaluation"]["build"], again["evaluation"]["build"]);
    assert_actor_parity(
        &first["evaluation"]["player"],
        &again["evaluation"]["player"],
    );
    assert_actor_parity(
        &first["evaluation"]["minion"],
        &again["evaluation"]["minion"],
    );

    let exported = scratch.path().join("export.xml");
    fs::write(
        &exported,
        first["evaluation"]["export_xml"].as_str().unwrap(),
    )
    .unwrap();
    let reloaded = evaluate(&exported);
    assert_eq!(
        first["evaluation"]["build"],
        reloaded["evaluation"]["build"]
    );
    assert_actor_parity(
        &first["evaluation"]["player"],
        &reloaded["evaluation"]["player"],
    );
    assert_actor_parity(
        &first["evaluation"]["minion"],
        &reloaded["evaluation"]["minion"],
    );
}

#[test]
fn incompatible_target_version_never_returns_startup_metrics() {
    let scratch = tempfile::tempdir().unwrap();
    let source =
        fs::read_to_string(root().join("tests/fixtures/builds/pobarchives-Dfz36mCq.xml")).unwrap();
    assert!(source.contains("targetVersion=\"0_1\""));
    let input = scratch.path().join("incompatible.xml");
    fs::write(
        &input,
        source.replacen("targetVersion=\"0_1\"", "targetVersion=\"unsupported\"", 1),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(root())
        .arg("evaluate")
        .arg(input)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        error.contains("targetVersion") || error.contains("interactive import"),
        "{error}"
    );
}

#[test]
fn import_preserves_bytes_and_output_aliases_fail_before_evaluation() {
    let scratch = tempfile::tempdir().unwrap();
    let imported = scratch.path().join("imported.xml");
    let output = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg("import")
        .arg(root().join("example.import.txt"))
        .arg("--output")
        .arg(&imported)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        fs::read(imported).unwrap(),
        fs::read(root().join("tests/fixtures/builds/pobarchives-Dfz36mCq.xml")).unwrap()
    );
    let output = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(scratch.path())
        .arg("evaluate")
        .arg("missing-input.xml")
        .args(["--output", "result", "--export", "./result"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("different paths"));
    assert!(!scratch.path().join("result").exists());
}

#[test]
fn incomplete_container_imports_but_never_evaluates_a_default_character() {
    let scratch = tempfile::tempdir().unwrap();
    let input = scratch.path().join("empty.xml");
    fs::write(&input, "<PathOfBuilding2/>").unwrap();
    let imported = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg("import")
        .arg(&input)
        .output()
        .unwrap();
    assert!(imported.status.success());
    let evaluated = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(root())
        .arg("evaluate")
        .arg(input)
        .output()
        .unwrap();
    assert!(!evaluated.status.success());
    assert!(evaluated.stdout.is_empty());
    assert!(String::from_utf8_lossy(&evaluated.stderr).contains("Build"));
}
