//! Caller-owned source inspection remains independent of build mechanic admission.
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, process::Command};
fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
}
fn contains_string(value: &Value, expected: &str) -> bool {
    match value {
        Value::String(text) => text == expected,
        Value::Array(values) => values.iter().any(|v| contains_string(v, expected)),
        Value::Object(values) => values.values().any(|v| contains_string(v, expected)),
        _ => false,
    }
}
#[test]
fn arbitrary_caller_configuration_retains_literal_values_without_reference_runtime() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("caller.xml");
    let xml = "<PathOfBuilding2><Build className='Caller'/><Config activeConfigSet='2'><ConfigSet id='1' title='Inactive'><Input name='otherUserOption' boolean='false'/></ConfigSet><ConfigSet id='2' title='Selected'><Input name='arbitraryUserOption' string='first\r\n\tsecond &amp; third'/></ConfigSet></Config></PathOfBuilding2>";
    fs::write(&path, xml).unwrap();
    let result = cli()
        .current_dir(temp.path())
        .arg("inspect-configuration")
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["status"], "source_projected");
    assert_eq!(report["scope"], "configuration_source_projection_v1");
    assert_eq!(
        report["input"]["xml_sha256"],
        format!("{:x}", Sha256::digest(xml.as_bytes()))
    );
    assert!(contains_string(
        &report["configuration"],
        "first\r\n\tsecond & third"
    ));
    assert!(contains_string(
        &report["configuration"],
        "arbitraryUserOption"
    ));
    assert_eq!(report["verification"]["game_mechanics"], "not_evaluated");
    assert_eq!(
        report["verification"]["effective_configuration"],
        "not_evaluated"
    );
    assert_eq!(report["verification"]["build_legality"], "not_checked");
    assert_eq!(report["verification"]["reference_calculation"], "not_run");
    assert_eq!(fs::read(&path).unwrap(), xml.as_bytes());
}
#[test]
fn all_five_corpus_inputs_project_without_modifying_source_or_claiming_build_support() {
    let folder =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/builds/breadth-20260908");
    let index: Value =
        serde_json::from_slice(&fs::read(folder.join("index.json")).unwrap()).unwrap();
    for entry in index["builds"].as_array().unwrap() {
        let path = folder.join(entry["xml"].as_str().unwrap());
        let before = fs::read(&path).unwrap();
        let result = cli()
            .arg("inspect-configuration")
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}: {}",
            entry["id"],
            String::from_utf8_lossy(&result.stderr)
        );
        let report: Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(report["input"]["xml_sha256"], entry["xml_sha256"]);
        assert_eq!(report["status"], "source_projected");
        assert_eq!(report["verification"]["game_mechanics"], "not_evaluated");
        assert_eq!(fs::read(path).unwrap(), before);
    }
}
#[test]
fn malformed_configurations_cannot_publish_and_reports_never_overwrite_input() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("input.xml");
    let report = temp.path().join("report.json");
    let invalid = "<PathOfBuilding2><Config activeConfigSet='1'><ConfigSet id='1'><Input name='callerKey' string='a'/><Input name='callerKey' string='b'/></ConfigSet></Config></PathOfBuilding2>";
    fs::write(&input, invalid).unwrap();
    let result = cli()
        .arg("inspect-configuration")
        .arg(&input)
        .arg("--output")
        .arg(&report)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(!report.exists());
    let valid = "<PathOfBuilding2><Config activeConfigSet='1'><ConfigSet id='1'><Input name='callerKey' string='value'/></ConfigSet></Config></PathOfBuilding2>";
    fs::write(&input, valid).unwrap();
    let result = cli()
        .arg("inspect-configuration")
        .arg(&input)
        .arg("--output")
        .arg(&input)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert_eq!(fs::read_to_string(&input).unwrap(), valid);
    let result = cli()
        .arg("inspect-configuration")
        .arg(&input)
        .arg("--output")
        .arg(&report)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report: Value = serde_json::from_slice(&fs::read(report).unwrap()).unwrap();
    assert_eq!(report["status"], "source_projected");
}
#[test]
fn input_path_is_required_and_has_no_fixture_fallback() {
    let result = cli().arg("inspect-configuration").output().unwrap();
    assert!(!result.status.success());
    assert!(result.stdout.is_empty());
    assert!(String::from_utf8_lossy(&result.stderr).contains("INPUT"));
}
