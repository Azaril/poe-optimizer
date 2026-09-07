//! Native throughput accounting works without a PoB checkout or worker process.
use serde_json::Value;
use std::{fs, path::Path, process::Command};

const FIXTURE: &str = include_str!("fixtures/calibration/spark-mapping.xml");

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
}

fn run(temp: &Path, input: &Path, mode: &str, jobs: usize, evaluations: usize) -> Value {
    let output = temp.join(format!("{mode}-{jobs}.json"));
    let result = cli()
        .current_dir(temp)
        .arg("benchmark-native")
        .arg(input)
        .args([
            "--mode",
            mode,
            "--jobs",
            &jobs.to_string(),
            "--evaluations",
            &evaluations.to_string(),
            "--timeout-seconds",
            "30",
            "--output",
        ])
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(result.stdout.is_empty());
    serde_json::from_slice(&fs::read(output).unwrap()).unwrap()
}

#[test]
fn prepared_and_document_modes_preserve_finite_results_across_worker_counts() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("build.xml");
    fs::write(&input, FIXTURE).unwrap();
    let mut first: Option<Value> = None;
    for mode in ["prepared", "document"] {
        for jobs in [1, 4] {
            let report = run(temp.path(), &input, mode, jobs, 64);
            assert_eq!(report["schema_version"], 1);
            assert_eq!(report["mode"], mode);
            assert_eq!(report["status"], "completed");
            assert_eq!(report["termination"], "evaluation_limit");
            assert_eq!(report["budget"]["requested_jobs"], jobs);
            assert_eq!(report["budget"]["resolved_jobs"], jobs);
            assert_eq!(report["iterations"]["attempts"], 64);
            assert_eq!(report["iterations"]["completed"], 64);
            assert_eq!(report["iterations"]["failures"], 0);
            assert_eq!(report["iterations"]["discarded_late"], 0);
            assert!(
                report["iterations"]["completed_per_second"]
                    .as_f64()
                    .unwrap()
                    > 0.0
            );
            assert_eq!(report["backend"]["id"], "native-poe2");
            for field in [
                "implementation_version",
                "rules_revision",
                "source_fingerprint",
                "adapter_fingerprint",
            ] {
                assert!(
                    !report["backend"][field].as_str().unwrap().is_empty(),
                    "{field}"
                );
            }
            assert_eq!(report["preparation"]["profile_preparations"], 1);
            assert_eq!(report["preparation"]["calculation_warmups"], 0);
            assert_eq!(report["calculation_results_cached"], false);
            assert_eq!(report["backend_identity_changed"], false);
            assert_eq!(report["non_finite_or_unavailable_completed_results"], 0);
            assert_eq!(report["metric_checksum"]["finite_results"], 64);
            assert_eq!(report["metric_checksum"]["all_results_identical"], true);
            assert_eq!(
                report["metric_checksum"]["per_result_sha256"]
                    .as_str()
                    .unwrap()
                    .len(),
                64
            );
            let measurements = report["sample_measurements"].as_array().unwrap();
            assert_eq!(measurements.len(), 9);
            assert!(
                measurements
                    .iter()
                    .all(|value| value["value"]["status"] == "finite"
                        && value["schema_version"].as_u64().unwrap() > 0
                        && value["unit"].is_string())
            );
            let life = measurements
                .iter()
                .find(|value| value["query"]["id"] == "life")
                .unwrap();
            assert_eq!(life["value"]["value"], 809.0);
            let scope = report["measurement_scope"].as_str().unwrap();
            assert!(scope.contains(if mode == "prepared" {
                "immutable profile parsing is reused"
            } else {
                "reparses the complete XML request"
            }));
            if let Some(first) = &first {
                assert_eq!(report["metric_checksum"], first["metric_checksum"]);
                assert_eq!(report["sample_measurements"], first["sample_measurements"]);
                assert_eq!(report["backend"], first["backend"]);
                assert_eq!(report["input_xml_sha256"], first["input_xml_sha256"]);
            } else {
                first = Some(report);
            }
        }
    }
}

#[test]
fn malformed_and_unsupported_input_do_not_create_reports() {
    let temp = tempfile::tempdir().unwrap();
    for (name, contents) in [
        ("malformed", "<PathOfBuilding2><Build>".to_owned()),
        (
            "unsupported",
            FIXTURE.replace("className=\"Sorceress\"", "className=\"Warrior\""),
        ),
    ] {
        let input = temp.path().join(format!("{name}.xml"));
        let output = temp.path().join(format!("{name}.json"));
        fs::write(&input, contents).unwrap();
        let result = cli()
            .current_dir(temp.path())
            .arg("benchmark-native")
            .arg(input)
            .args(["--evaluations", "2", "--output"])
            .arg(&output)
            .output()
            .unwrap();
        assert!(!result.status.success());
        assert!(!output.exists());
        assert!(!result.stderr.is_empty());
    }
}

#[test]
fn output_collision_is_rejected_before_reading_input() {
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("existing.json");
    fs::write(&output, b"preserve existing result").unwrap();
    let result = cli()
        .current_dir(temp.path())
        .arg("benchmark-native")
        .arg(temp.path().join("missing.xml"))
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("Output already exists"));
    assert_eq!(fs::read(output).unwrap(), b"preserve existing result");
}

#[test]
fn benchmark_bounds_are_validated_before_reading_input() {
    let temp = tempfile::tempdir().unwrap();
    for (flag, value) in [
        ("--jobs", "0"),
        ("--jobs", "65"),
        ("--evaluations", "0"),
        ("--evaluations", "1000001"),
        ("--timeout-seconds", "0"),
    ] {
        let result = cli()
            .current_dir(temp.path())
            .arg("benchmark-native")
            .arg(temp.path().join("missing.xml"))
            .args([flag, value])
            .output()
            .unwrap();
        assert!(!result.status.success());
        assert!(
            String::from_utf8_lossy(&result.stderr).contains("Benchmark requires"),
            "{flag} {value}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
}

#[test]
fn document_deadline_reports_partial_work_with_a_balanced_ledger() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("long-notes.xml");
    fs::write(
        &input,
        FIXTURE.replace("<Notes>", &format!("<Notes>{}", "n".repeat(256 * 1024))),
    )
    .unwrap();
    let result = cli()
        .current_dir(temp.path())
        .arg("benchmark-native")
        .arg(input)
        .args([
            "--mode",
            "document",
            "--jobs",
            "1",
            "--evaluations",
            "1000000",
            "--timeout-seconds",
            "1",
        ])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["status"], "partial");
    assert_eq!(report["termination"], "time_budget");
    let iterations = &report["iterations"];
    let attempts = iterations["attempts"].as_u64().unwrap();
    assert!(attempts < 1_000_000);
    assert_eq!(
        attempts,
        iterations["completed"].as_u64().unwrap()
            + iterations["failures"].as_u64().unwrap()
            + iterations["discarded_late"].as_u64().unwrap()
    );
}

#[test]
fn mace_unavailable_average_keeps_the_complete_result_checksum_absent() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("mace.xml");
    fs::write(&input, include_str!("fixtures/calibration/mace-wooden.xml")).unwrap();
    for mode in ["prepared", "document"] {
        let report = run(temp.path(), &input, mode, 4, 8);
        assert_eq!(report["status"], "completed");
        assert_eq!(report["iterations"]["completed"], 8);
        assert_eq!(report["iterations"]["failures"], 0);
        assert_eq!(report["non_finite_or_unavailable_completed_results"], 8);
        assert!(report["metric_checksum"].is_null());
        let measurements = report["sample_measurements"].as_array().unwrap();
        assert_eq!(
            measurements
                .iter()
                .filter(|value| value["value"]["status"] == "finite")
                .count(),
            8
        );
        let average = measurements
            .iter()
            .find(|value| value["query"]["id"] == "selected_average_hit")
            .unwrap();
        assert_eq!(average["value"]["status"], "unavailable");
    }
}
