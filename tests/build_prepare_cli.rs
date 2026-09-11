//! Preparation consumes caller inputs without calculating metrics or admitting
//! complete builds whose general producers have not been implemented.
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
}
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_owned()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn assert_preparation_only(report: &Value) {
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["scope"], "native_preparation");
    assert_eq!(report["calculation"], "not_run");
    assert_eq!(report["whole_build_parity"], "not_established");
    assert_eq!(report["backend"]["id"], "native-poe2");
    assert!(report["requested"]["options"].is_object());
    assert_eq!(report["requested"]["metric_queries"], serde_json::json!([]));
    for key in ["metrics", "results", "statistics", "native_metrics"] {
        assert!(report.get(key).is_none(), "unexpected computed field {key}");
    }
}
fn ready_input(directory: &Path) -> (PathBuf, Vec<u8>) {
    let bytes = fs::read(root().join("tests/fixtures/calibration/spark-mapping.xml")).unwrap();
    let input = directory.join("caller-selected.xml");
    fs::write(&input, &bytes).unwrap();
    (input, bytes)
}

#[test]
fn supported_native_fixture_prepares_from_an_unrelated_directory_without_calculation() {
    let temp = tempfile::tempdir().unwrap();
    let (input, original) = ready_input(temp.path());
    let report = success(
        cli()
            .current_dir(temp.path())
            .arg("prepare-build")
            .arg(&input)
            .output()
            .unwrap(),
    );
    assert_preparation_only(&report);
    assert_eq!(report["status"], "ready_for_supported_native_metrics");
    assert!(report.get("preparation").is_none());
    assert_eq!(report["source_sha256"], hash(&original));
    assert_eq!(report["selected_view"]["source_sha256"], hash(&original));
    assert_eq!(
        report["selected_view"]["skills"]["selected"]["members"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        report["selected_view"]["lineage"].as_str().unwrap().len(),
        32
    );
    assert_eq!(fs::read(&input).unwrap(), original);
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
}

#[test]
fn real_caller_share_code_returns_source_linked_prerequisites_and_no_fake_metrics() {
    let temp = tempfile::tempdir().unwrap();
    let corpus = root().join("tests/fixtures/builds/breadth-20260908");
    let imports = fs::read(corpus.join("imports.txt")).unwrap();
    let code = std::str::from_utf8(&imports)
        .unwrap()
        .lines()
        .nth(1)
        .unwrap();
    let input = temp.path().join("supplied-code.txt");
    fs::write(&input, code).unwrap();
    let options = temp.path().join("caller-options.json");
    let requested_options = serde_json::json!({"selection":{"socket_group":2,"active_skill":3,"minion_skill":1},"encounter":{"name":"Caller incomplete diagnostics","enemy_level":84,"boss":"pinnacle","incoming_hit":{"physical":1234.5,"fire":67.25,"cold":0.0,"lightning":0.0,"chaos":0.0}}});
    let option_bytes = serde_json::to_vec(&requested_options).unwrap();
    fs::write(&options, &option_bytes).unwrap();
    let expected_hash = hash(&fs::read(corpus.join("build-02.xml")).unwrap());
    let report = success(
        cli()
            .current_dir(temp.path())
            .arg("prepare-build")
            .arg(&input)
            .arg("--options")
            .arg(&options)
            .output()
            .unwrap(),
    );
    assert_preparation_only(&report);
    assert_eq!(report["status"], "incomplete");
    assert_eq!(report["requested"]["options"], requested_options);
    assert_eq!(report["preparation"]["requested"], report["requested"]);
    assert_eq!(report["source_sha256"], expected_hash);
    assert_eq!(
        report["preparation"]["view"]["source_sha256"],
        expected_hash
    );
    assert!(
        !report["preparation"]["legacy_adapter_error"]
            .as_str()
            .unwrap()
            .is_empty()
    );
    let issues = report["preparation"]["issues"].as_array().unwrap();
    for stage in [
        "skill_group_producers",
        "item_registration",
        "equipment_assignment",
        "passive_allocation",
        "configuration_effects",
    ] {
        assert!(
            issues.iter().any(|issue| issue["stage"] == stage),
            "missing independently discoverable stage {stage}"
        );
    }
    let located: Vec<_> = issues
        .iter()
        .filter(|issue| !issue["instance"].is_null())
        .collect();
    assert!(!located.is_empty());
    for issue in located {
        assert_eq!(issue["source"]["source_sha256"], expected_hash);
        assert!(issue["source"]["ordinal"].as_u64().is_some());
        assert_ne!(issue["instance"]["kind"], "item_record");
    }
    assert_eq!(fs::read_to_string(&input).unwrap(), code);
    assert_eq!(fs::read(corpus.join("imports.txt")).unwrap(), imports);
    assert_eq!(fs::read(&options).unwrap(), option_bytes);
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 2);
}

#[test]
fn new_report_file_accepts_existing_options_and_data_schema_then_refuses_overwrite() {
    let temp = tempfile::tempdir().unwrap();
    let (input, original) = ready_input(temp.path());
    let options = temp.path().join("caller-options.json");
    let option_bytes = br#"{"selection":{"socket_group":1,"active_skill":1},"encounter":{"name":"Caller mapping","enemy_level":60,"boss":"normal"}}"#;
    fs::write(&options, option_bytes).unwrap();
    let output = temp.path().join("new-report.json");
    let data = root().join("crates/poe-optimizer-data/data/game-data.json");
    let before_data = hash(&fs::read(&data).unwrap());
    let command = cli()
        .current_dir(temp.path())
        .arg("prepare-build")
        .arg(&input)
        .arg("--options")
        .arg(&options)
        .arg("--data")
        .arg(&data)
        .arg("--data-sha256")
        .arg(&before_data)
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        command.status.success(),
        "{}",
        String::from_utf8_lossy(&command.stderr)
    );
    assert!(command.stdout.is_empty());
    let saved = fs::read(&output).unwrap();
    let report: Value = serde_json::from_slice(&saved).unwrap();
    assert_preparation_only(&report);
    let expected_options: poe_optimizer_core::options::EvaluationOptions =
        serde_json::from_slice(option_bytes).unwrap();
    assert_eq!(
        report["requested"]["options"],
        serde_json::to_value(expected_options).unwrap()
    );
    assert_eq!(report["status"], "ready_for_supported_native_metrics");
    for destination in [&output, &input, &options] {
        let refused = cli()
            .arg("prepare-build")
            .arg(&input)
            .arg("--output")
            .arg(destination)
            .output()
            .unwrap();
        assert!(!refused.status.success());
        assert!(String::from_utf8_lossy(&refused.stderr).contains("Output already exists"));
        assert!(refused.stdout.is_empty());
    }
    assert_eq!(fs::read(&output).unwrap(), saved);
    assert_eq!(fs::read(&input).unwrap(), original);
    assert_eq!(fs::read(&options).unwrap(), option_bytes);
    assert_eq!(hash(&fs::read(&data).unwrap()), before_data);
}

#[test]
fn missing_input_malformed_source_and_invalid_options_do_not_publish_reports() {
    assert!(
        !cli()
            .arg("prepare-build")
            .output()
            .unwrap()
            .status
            .success()
    );
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("malformed.xml");
    let original = "<PathOfBuilding2><Skills>";
    fs::write(&input, original).unwrap();
    let destination = temp.path().join("report.json");
    let failure = cli()
        .arg("prepare-build")
        .arg(&input)
        .arg("--output")
        .arg(&destination)
        .output()
        .unwrap();
    assert!(!failure.status.success());
    assert!(!destination.exists());
    assert_eq!(fs::read_to_string(&input).unwrap(), original);
    let (valid, valid_bytes) = ready_input(temp.path());
    let options = temp.path().join("invalid-options.json");
    let bad_options = br#"{"selection":{"socket_group":0}}"#;
    fs::write(&options, bad_options).unwrap();
    let failure = cli()
        .arg("prepare-build")
        .arg(&valid)
        .arg("--options")
        .arg(&options)
        .arg("--output")
        .arg(&destination)
        .output()
        .unwrap();
    assert!(!failure.status.success());
    assert!(String::from_utf8_lossy(&failure.stderr).contains("positive and one-based"));
    assert!(failure.stdout.is_empty());
    assert!(!destination.exists());
    assert_eq!(fs::read(&valid).unwrap(), valid_bytes);
    assert_eq!(fs::read(&options).unwrap(), bad_options);
}
