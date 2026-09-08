use poe_optimizer_data::game_data::{bundled_package_bytes, bundled_snapshot};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};
fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
}
fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/calibration/spark-mapping.xml")
}
fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn evaluate(data: Option<&Path>) -> Value {
    let mut command = cli();
    command
        .arg("evaluate")
        .arg(fixture())
        .args(["--backend", "native"]);
    if let Some(data) = data {
        command.arg("--data").arg(data);
    }
    success(command.output().unwrap())
}
fn dps(report: &Value) -> f64 {
    report["evaluation"]["measurements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|metric| metric["query"]["id"] == "selected_hit_dps")
        .unwrap()["value"]["value"]
        .as_f64()
        .unwrap()
}
fn custom(path: &Path) {
    let mut package = bundled_snapshot().unwrap().package().clone();
    package.spark.lightning_minimum *= 2.0;
    package.spark.lightning_maximum *= 2.0;
    package.refresh_section_digests().unwrap();
    fs::write(path, package.canonical_bytes().unwrap()).unwrap();
}
#[test]
fn one_executable_loads_reviewed_and_custom_packages_and_exports_dataset_metadata() {
    let temp = tempfile::tempdir().unwrap();
    let reviewed = temp.path().join("reviewed.json");
    let changed = temp.path().join("changed.json");
    fs::write(&reviewed, bundled_package_bytes()).unwrap();
    custom(&changed);
    let baseline = evaluate(None);
    let external = evaluate(Some(&reviewed));
    assert_eq!(
        external["evaluation"]["backend"],
        baseline["evaluation"]["backend"]
    );
    assert_eq!(
        external["evaluation"]["measurements"],
        baseline["evaluation"]["measurements"]
    );
    let export = temp.path().join("result.xml");
    let changed_result = success(
        cli()
            .arg("evaluate")
            .arg(fixture())
            .args(["--backend", "native", "--data"])
            .arg(&changed)
            .arg("--export")
            .arg(&export)
            .output()
            .unwrap(),
    );
    assert_eq!(changed_result["schema_version"], 3);
    assert!((dps(&changed_result) - 2.0 * dps(&baseline)).abs() < 1e-12);
    assert_ne!(
        changed_result["evaluation"]["backend"]["data"],
        baseline["evaluation"]["backend"]["data"]
    );
    assert_eq!(
        changed_result["evaluation"]["backend"]["adapter_fingerprint"],
        baseline["evaluation"]["backend"]["adapter_fingerprint"]
    );
    assert_eq!(changed_result["evaluation"]["attachments"], json!([]));
    assert!(
        changed_result["evaluation"]["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning.as_str().unwrap().contains("CustomUnreviewed"))
    );
    let metadata: Value =
        serde_json::from_slice(&fs::read(temp.path().join("result.xml.data.json")).unwrap())
            .unwrap();
    assert_eq!(metadata["backend"], changed_result["evaluation"]["backend"]);
    assert_eq!(metadata["uses_packaged_default"], false);
    assert_eq!(fs::read(&export).unwrap(), fs::read(fixture()).unwrap());
    let reimport = success(
        cli()
            .arg("evaluate")
            .arg(&export)
            .args(["--backend", "native", "--data"])
            .arg(&changed)
            .output()
            .unwrap(),
    );
    assert_eq!(
        reimport["evaluation"]["measurements"],
        changed_result["evaluation"]["measurements"]
    );
}
#[test]
fn invalid_selected_packages_and_export_collisions_fail_without_fallback_or_overwrite() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("data.json");
    fs::write(&data, bundled_package_bytes()).unwrap();
    let output = temp.path().join("result.json");
    let export = temp.path().join("result.xml");
    let fail = cli()
        .arg("evaluate")
        .arg(fixture())
        .args(["--backend", "native", "--data"])
        .arg(&data)
        .arg("--data-sha256")
        .arg("0".repeat(64))
        .arg("--output")
        .arg(&output)
        .arg("--export")
        .arg(&export)
        .output()
        .unwrap();
    assert!(!fail.status.success());
    assert!(String::from_utf8_lossy(&fail.stderr).contains("SHA-256"));
    assert!(
        !output.exists() && !export.exists() && !temp.path().join("result.xml.data.json").exists()
    );
    fs::write(&data, b"{}").unwrap();
    let fail = cli()
        .args(["metrics", "--backend", "native", "--data"])
        .arg(&data)
        .output()
        .unwrap();
    assert!(!fail.status.success());
    let metadata = temp.path().join("result.xml.data.json");
    fs::write(&metadata, b"preserve").unwrap();
    let fail = cli()
        .arg("evaluate")
        .arg(fixture())
        .args(["--backend", "native", "--export"])
        .arg(&export)
        .output()
        .unwrap();
    assert!(!fail.status.success());
    assert_eq!(fs::read(&metadata).unwrap(), b"preserve");
    assert!(!export.exists());
    let fail = cli()
        .args(["metrics", "--backend", "native", "--data-sha256"])
        .arg("0".repeat(64))
        .output()
        .unwrap();
    assert!(!fail.status.success());
    #[cfg(feature = "pob")]
    assert!(
        !cli()
            .args(["metrics", "--backend", "pob", "--data"])
            .arg(&data)
            .output()
            .unwrap()
            .status
            .success()
    );
}
#[test]
fn external_package_benchmark_keeps_one_dataset_across_worker_counts() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("custom.json");
    custom(&data);
    let mut first = None;
    for jobs in [1, 4] {
        let report = success(
            cli()
                .arg("benchmark-native")
                .arg(fixture())
                .arg("--data")
                .arg(&data)
                .args(["--evaluations", "32", "--jobs"])
                .arg(jobs.to_string())
                .output()
                .unwrap(),
        );
        assert_eq!(report["schema_version"], 2);
        assert_eq!(report["data_trust"]["status"], "custom_unreviewed");
        assert_eq!(report["iterations"]["completed"], 32);
        assert_eq!(report["iterations"]["failures"], 0);
        assert_eq!(report["backend_identity_changed"], false);
        assert!(
            report["initialization"]["backend_and_data_ms"]
                .as_f64()
                .unwrap()
                >= 0.0
        );
        if let Some(identity) = &first {
            assert_eq!(identity, &report["backend"]);
        }
        first = Some(report["backend"].clone());
    }
}
#[test]
fn saved_schema_three_requires_valid_native_data_identity() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("saved.json");
    let objective = temp.path().join("objective.json");
    fs::write(&objective,serde_json::to_vec(&json!({"schema_version":1,"objective":{"kind":"scalar","metric":{"actor":"player","id":"selected_hit_dps"},"unit":"damage_per_second","direction":"maximize"},"constraints":[]})).unwrap()).unwrap();
    let baseline = evaluate(None);
    let mut invalid = baseline.clone();
    invalid["evaluation"]["backend"]["data"] = Value::Null;
    for value in [invalid, {
        let mut invalid = baseline;
        invalid["evaluation"]["backend"]["data"]["content_sha256"] = json!("wrong");
        invalid
    }] {
        fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(
            !cli()
                .arg("assess")
                .arg(&path)
                .arg("--objective")
                .arg(&objective)
                .output()
                .unwrap()
                .status
                .success()
        );
    }
}
