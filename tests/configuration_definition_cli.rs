//! Definition inspection uses caller source and selected data without a calculation host.
use poe_optimizer_core::options::Scalar;
use poe_optimizer_data::{
    configuration::ConfigScalarKind,
    game_data::{GameDataLoader, LoadLimits, TrustPolicy, bundled_snapshot},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
}
fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn xml(fields: &str) -> String {
    format!(
        "<PathOfBuilding2><Config activeConfigSet='1'><ConfigSet id='1'>{fields}</ConfigSet></Config></PathOfBuilding2>"
    )
}
fn write_source(folder: &Path, fields: &str) -> (std::path::PathBuf, String) {
    let input = folder.join("caller.xml");
    let text = xml(fields);
    fs::write(&input, &text).unwrap();
    (input, text)
}
fn scalar_record<'a>(report: &'a Value, name: &str) -> &'a Value {
    &report["definition_lookup"]["sets"][0]["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|record| record["lookup"]["name"] == name)
        .unwrap()["lookup"]
}
fn source_only_claims(report: &Value) {
    assert_eq!(
        report["verification"]["effective_configuration"],
        "not_evaluated"
    );
    assert_eq!(report["verification"]["game_mechanics"], "not_evaluated");
    assert_eq!(report["verification"]["build_legality"], "not_checked");
    assert_eq!(report["verification"]["reference_calculation"], "not_run");
    let lookup = &report["definition_lookup"];
    assert_eq!(lookup["effective_configuration"], "not_evaluated");
    assert_eq!(lookup["game_mechanics"], "not_evaluated");
    assert_eq!(lookup["callbacks"], "not_executed");
    assert_eq!(lookup["option_membership_is_import_whitelist"], false);
}

#[test]
fn reviewed_definitions_work_outside_repository_without_build_or_reference_runtime() {
    let temp = tempfile::tempdir().unwrap();
    let (input, text) = write_source(
        temp.path(),
        "<Input name='resistancePenalty' number='-60'/><Placeholder name='callerUnknown' string='raw\n\tvalue'/>",
    );
    assert!(!temp.path().join("vendor").exists());
    let report = success(
        cli()
            .current_dir(temp.path())
            .arg("inspect-configuration")
            .arg(&input)
            .arg("--with-definitions")
            .output()
            .unwrap(),
    );
    assert_eq!(report["scope"], "configuration_source_projection_v1");
    assert_eq!(report["status"], "source_projected");
    let snapshot = bundled_snapshot().unwrap();
    assert_eq!(
        report["definition_lookup"]["data"],
        serde_json::to_value(snapshot.identity()).unwrap()
    );
    assert_eq!(
        report["definition_lookup"]["data_trust"],
        serde_json::to_value(snapshot.trust()).unwrap()
    );
    assert_eq!(report["definition_lookup"]["catalog_definition_count"], 564);
    assert_eq!(
        report["definition_lookup"]["source_xml_sha256"],
        format!("{:x}", Sha256::digest(text.as_bytes()))
    );
    let matched = scalar_record(&report, "resistancePenalty");
    assert_eq!(matched["definitions"].as_array().unwrap().len(), 1);
    assert_eq!(matched["definitions"][0]["option_indices"], json!([7]));
    assert_eq!(matched["definitions"][0]["scalar_kind_matches"], true);
    assert_eq!(
        scalar_record(&report, "callerUnknown")["definitions"],
        json!([])
    );
    source_only_claims(&report);
    assert_eq!(fs::read(&input).unwrap(), text.as_bytes());
}

#[test]
fn selected_data_implies_lookup_and_preserves_custom_key_options_defaults_and_trust() {
    let temp = tempfile::tempdir().unwrap();
    let data_path = temp.path().join("caller-data.json");
    let mut package = bundled_snapshot().unwrap().package().clone();
    let definition = package
        .configuration
        .definitions
        .iter_mut()
        .find(|row| row.key == "resistancePenalty")
        .unwrap();
    definition.key = "callerRenamedOption".into();
    definition.options.truncate(2);
    definition.options[0].value = Scalar::Text("custom\n\toption".into());
    definition.options[0].label = Some("Caller label".into());
    definition.options[1].value = Scalar::Number(7.25);
    definition.scalar_kinds = vec![ConfigScalarKind::Text, ConfigScalarKind::Number];
    definition.defaults.input = Some(Scalar::Boolean(false));
    definition.defaults.placeholder = Some(Scalar::Number(3.5));
    definition.defaults.option_index = Some(2);
    package.refresh_section_digests().unwrap();
    let bytes = package.canonical_bytes().unwrap();
    fs::write(&data_path, &bytes).unwrap();
    let expected =
        GameDataLoader::from_bytes(&bytes, &TrustPolicy::AllowCustom, &LoadLimits::default())
            .unwrap();
    let (input, text) = write_source(
        temp.path(),
        "<Input name='callerRenamedOption' string='custom\n\toption'/><Placeholder name='callerRenamedOption' number='7.25'/><Input name='resistancePenalty' number='-60'/>",
    );
    let output = temp.path().join("report.json");
    let result = cli()
        .current_dir(temp.path())
        .arg("inspect-configuration")
        .arg(&input)
        .arg("--data")
        .arg(&data_path)
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(result.stdout.is_empty());
    let report: Value = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
    assert_eq!(
        report["definition_lookup"]["data"],
        serde_json::to_value(expected.identity()).unwrap()
    );
    assert_eq!(
        report["definition_lookup"]["data_trust"]["status"],
        "custom_unreviewed"
    );
    assert_eq!(
        scalar_record(&report, "callerRenamedOption")["definitions"][0]["option_indices"],
        json!([1])
    );
    assert_eq!(
        scalar_record(&report, "resistancePenalty")["definitions"],
        json!([])
    );
    let records = report["definition_lookup"]["sets"][0]["records"]
        .as_array()
        .unwrap();
    assert_eq!(records[1]["kind"], "placeholder");
    assert_eq!(
        records[1]["lookup"]["definitions"][0]["option_indices"],
        json!([2])
    );
    assert_eq!(
        report["definition_lookup"]["initial_defaults_before_load"]["inputs"]["callerRenamedOption"],
        serde_json::to_value(Scalar::Number(7.25)).unwrap()
    );
    assert_eq!(
        report["definition_lookup"]["initial_defaults_before_load"]["placeholders"]["callerRenamedOption"],
        serde_json::to_value(Scalar::Number(3.5)).unwrap()
    );
    source_only_claims(&report);
    assert_eq!(fs::read(&input).unwrap(), text.as_bytes());
    assert_eq!(fs::read(&data_path).unwrap(), bytes);
}

#[test]
fn invalid_catalog_or_review_digest_cannot_publish_a_report_or_fallback() {
    let temp = tempfile::tempdir().unwrap();
    let (input, text) = write_source(
        temp.path(),
        "<Input name='resistancePenalty' number='-60'/>",
    );
    let data_path = temp.path().join("selected.json");
    let output = temp.path().join("report.json");
    let mut invalid = bundled_snapshot().unwrap().package().clone();
    invalid.configuration.definitions[1].id = invalid.configuration.definitions[0].id.clone();
    invalid.refresh_section_digests().unwrap();
    let invalid_bytes = invalid.canonical_bytes().unwrap();
    fs::write(&data_path, &invalid_bytes).unwrap();
    let result = cli()
        .arg("inspect-configuration")
        .arg(&input)
        .arg("--data")
        .arg(&data_path)
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(result.stdout.is_empty());
    assert!(!output.exists());
    assert!(String::from_utf8_lossy(&result.stderr).contains("duplicate definition occurrence ID"));
    assert_eq!(fs::read(&data_path).unwrap(), invalid_bytes);
    let valid = bundled_snapshot()
        .unwrap()
        .package()
        .canonical_bytes()
        .unwrap();
    fs::write(&data_path, &valid).unwrap();
    let result = cli()
        .arg("inspect-configuration")
        .arg(&input)
        .arg("--data")
        .arg(&data_path)
        .arg("--data-sha256")
        .arg("0".repeat(64))
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(result.stdout.is_empty());
    assert!(!output.exists());
    assert!(String::from_utf8_lossy(&result.stderr).contains("SHA-256"));
    assert_eq!(fs::read(&input).unwrap(), text.as_bytes());
    assert_eq!(fs::read(&data_path).unwrap(), valid);
}

#[test]
fn lookup_output_collision_preserves_exact_caller_source() {
    let temp = tempfile::tempdir().unwrap();
    let (input, text) = write_source(
        temp.path(),
        "<Input name='resistancePenalty' number='-60'/>",
    );
    let result = cli()
        .current_dir(temp.path())
        .arg("inspect-configuration")
        .arg(&input)
        .arg("--with-definitions")
        .arg("--output")
        .arg(&input)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(result.stdout.is_empty());
    assert_eq!(fs::read(&input).unwrap(), text.as_bytes());
}

#[test]
fn default_inspection_remains_source_only_without_catalog_enrichment() {
    let temp = tempfile::tempdir().unwrap();
    let (input, text) = write_source(
        temp.path(),
        "<Input name='callerOnly' string='literal\n\tvalue'/>",
    );
    let report = success(
        cli()
            .current_dir(temp.path())
            .arg("inspect-configuration")
            .arg(&input)
            .output()
            .unwrap(),
    );
    assert!(report.get("definition_lookup").is_none());
    assert!(report.get("definition_implementation_sha256").is_none());
    assert_eq!(report["verification"]["reference_calculation"], "not_run");
    assert_eq!(
        report["verification"]["effective_configuration"],
        "not_evaluated"
    );
    assert_eq!(fs::read(&input).unwrap(), text.as_bytes());
}
