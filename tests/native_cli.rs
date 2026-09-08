use std::{fs, process::Command};
fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
}
fn fixture() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/calibration/spark-mapping.xml")
}
#[test]
fn native_cli_evaluates_and_exports_without_a_pob_checkout_in_the_working_directory() {
    let temp = tempfile::tempdir().unwrap();
    let report = temp.path().join("native.json");
    let export = temp.path().join("native.xml");
    let output = cli()
        .current_dir(temp.path())
        .arg("evaluate")
        .arg(fixture())
        .args(["--backend", "native", "--raw", "--output"])
        .arg(&report)
        .arg("--export")
        .arg(&export)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&fs::read(&report).unwrap()).unwrap();
    assert_eq!(value["schema_version"], 3);
    assert_eq!(value["evaluation"]["backend"]["id"], "native-poe2");
    assert_eq!(
        value["evaluation"]["measurements"]
            .as_array()
            .unwrap()
            .len(),
        13
    );
    assert_eq!(fs::read(&export).unwrap(), fs::read(fixture()).unwrap());
    assert_eq!(value["evaluation"]["diagnostic_only"], true);
    let retry = cli()
        .arg("evaluate")
        .arg(fixture())
        .args(["--backend", "native", "--output"])
        .arg(&report)
        .output()
        .unwrap();
    assert!(!retry.status.success());
    assert!(String::from_utf8_lossy(&retry.stderr).contains("exists"));
}
#[test]
fn native_catalog_and_unsupported_metrics_do_not_fall_back_to_pob() {
    let output = cli()
        .args(["metrics", "--backend", "native"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let catalog: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(catalog.as_array().unwrap().len(), 13);
    let output = cli()
        .arg("evaluate")
        .arg(fixture())
        .args(["--backend", "native", "--metric", "player.pob_total_ehp"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("metric"));
    let output = cli()
        .arg("evaluate")
        .arg(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("example.import.txt"))
        .args(["--backend", "native"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("native")
            || String::from_utf8_lossy(&output.stderr).contains("Native")
    );
}
#[test]
fn native_results_share_objective_and_saved_assessment_contracts() {
    let temp = tempfile::tempdir().unwrap();
    let objective = temp.path().join("objective.json");
    fs::write(&objective,serde_json::to_vec(&serde_json::json!({"schema_version":1,"objective":{"kind":"scalar","direction":"maximize","metric":{"actor":"player","id":"selected_hit_dps"},"unit":"damage_per_second"},"constraints":[]})).unwrap()).unwrap();
    let report = temp.path().join("run.json");
    let output = cli()
        .arg("evaluate")
        .arg(fixture())
        .args(["--backend", "native", "--objective"])
        .arg(&objective)
        .arg("--output")
        .arg(&report)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let direct: serde_json::Value = serde_json::from_slice(&fs::read(&report).unwrap()).unwrap();
    let output = cli()
        .arg("assess")
        .arg(&report)
        .arg("--objective")
        .arg(&objective)
        .output()
        .unwrap();
    assert!(output.status.success());
    let assessed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        direct["objective_assessment"],
        assessed["objective_assessment"]
    );
}
#[cfg(not(feature = "pob"))]
#[test]
fn native_only_cli_has_no_pob_backend_or_worker_commands() {
    let temp = tempfile::tempdir().unwrap();
    let result = cli()
        .current_dir(temp.path())
        .arg("evaluate")
        .arg(fixture())
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(value["evaluation"]["backend"]["id"], "native-poe2");
    assert!(
        !cli()
            .args(["metrics", "--backend", "pob"])
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(!cli().arg("__worker").output().unwrap().status.success());
}

#[test]
fn armour_rating_objective_reassesses_offline_and_rejects_minion_scope() {
    let temp = tempfile::tempdir().unwrap();
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/builds/mace-local-armour.xml");
    let spec = serde_json::json!({"schema_version":1,"objective":{"kind":"scalar","metric":{"actor":"player","id":"armour"},"unit":"rating_points","direction":"maximize"},"constraints":[{"id":"evasion-floor","metric":{"actor":"player","id":"evasion"},"unit":"rating_points","operator":">=","threshold":150,"violation_scale":150}]});
    let objective = temp.path().join("rating-objective.json");
    fs::write(&objective, serde_json::to_vec(&spec).unwrap()).unwrap();
    let report = temp.path().join("rating-run.json");
    let output = cli()
        .arg("evaluate")
        .arg(&fixture)
        .args([
            "--backend",
            "native",
            "--metric",
            "player.armour",
            "--objective",
        ])
        .arg(&objective)
        .arg("--output")
        .arg(&report)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let direct: serde_json::Value = serde_json::from_slice(&fs::read(&report).unwrap()).unwrap();
    let output = cli()
        .arg("assess")
        .arg(&report)
        .arg("--objective")
        .arg(&objective)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let assessed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        direct["objective_assessment"],
        assessed["objective_assessment"]
    );
    assert_eq!(
        direct["evaluation"]["measurements"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    for metric in direct["evaluation"]["measurements"].as_array().unwrap() {
        assert_eq!(metric["unit"], "rating_points");
        assert_eq!(metric["schema_version"], 1);
        assert!(metric["value"]["value"].as_f64().unwrap() >= 150.0);
    }
    let output = cli()
        .arg("evaluate")
        .arg(&fixture)
        .args(["--backend", "native", "--metric", "selected_minion.armour"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("metric"));
}

#[test]
fn movement_objective_records_baseline_percentage_and_reassesses_without_pob() {
    let temp = tempfile::tempdir().unwrap();
    let input = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/builds/mace-body-armour.xml");
    let spec = serde_json::json!({"schema_version":1,"objective":{"kind":"scalar","direction":"maximize","metric":{"actor":"player","id":"movement_speed_pct"},"unit":"percent"},"constraints":[{"id":"baseline","metric":{"actor":"player","id":"movement_speed_pct"},"unit":"percent","operator":">=","threshold":100,"violation_scale":100}]});
    let objective = temp.path().join("movement-objective.json");
    let report = temp.path().join("movement.json");
    fs::write(&objective, serde_json::to_vec(&spec).unwrap()).unwrap();
    let output = cli()
        .current_dir(temp.path())
        .arg("evaluate")
        .arg(&input)
        .args(["--backend", "native", "--objective"])
        .arg(&objective)
        .arg("--output")
        .arg(&report)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let direct: serde_json::Value = serde_json::from_slice(&fs::read(&report).unwrap()).unwrap();
    assert_eq!(
        direct["objective_assessment"]["status"],
        "constraints_satisfied"
    );
    let movement = direct["evaluation"]["measurements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["query"]["id"] == "movement_speed_pct")
        .unwrap();
    assert_eq!(movement["schema_version"], 1);
    assert_eq!(movement["unit"], "percent");
    assert!((movement["value"]["value"].as_f64().unwrap() - 118.8).abs() < 1e-9);
    let output = cli()
        .current_dir(temp.path())
        .arg("assess")
        .arg(&report)
        .arg("--objective")
        .arg(&objective)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let reassessed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        direct["objective_assessment"],
        reassessed["objective_assessment"]
    );
    let output = cli()
        .current_dir(temp.path())
        .arg("evaluate")
        .arg(&input)
        .args([
            "--backend",
            "native",
            "--metric",
            "selected_minion.movement_speed_pct",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("metric"));
}
