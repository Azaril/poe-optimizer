//! Skill inspection is source/identity evidence, independent of native effect support.
#[path = "support/skill_preparation_edits.rs"]
mod preparation_edits;
use serde_json::Value;
use std::{fs, path::Path, process::Command};
fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
}
fn success(output: std::process::Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn count_role(value: &Value, role: &str) -> usize {
    match value {
        Value::Object(fields) => {
            usize::from(fields.get("source_use").and_then(Value::as_str) == Some(role))
                + fields.values().map(|v| count_role(v, role)).sum::<usize>()
        }
        Value::Array(values) => values.iter().map(|v| count_role(v, role)).sum(),
        _ => 0,
    }
}
#[test]
fn caller_build_inspection_preserves_all_saved_skill_occurrences_without_data_or_runtime() {
    let corpus =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/builds/breadth-20260908");
    let temp = tempfile::tempdir().unwrap();
    let mut sets = 0;
    let mut groups = 0;
    let mut instances = 0;
    for i in 1..=5 {
        let input = corpus.join(format!("build-{i:02}.xml"));
        let before = fs::read(&input).unwrap();
        let report = success(
            cli()
                .current_dir(temp.path())
                .arg("inspect-build")
                .arg(&input)
                .output()
                .unwrap(),
        );
        assert_eq!(report["skills"]["status"], "source_projected");
        assert!(report.get("definition_lookup").is_none());
        assert_eq!(report["verification"]["native_admission"], "not_checked");
        assert_eq!(report["verification"]["reference_calculation"], "not_run");
        sets += count_role(&report["skills"]["projection"], "saved_set");
        groups += count_role(&report["skills"]["projection"], "group");
        instances += count_role(&report["skills"]["projection"], "gem_instance");
        assert_eq!(fs::read(&input).unwrap(), before);
    }
    // Independent complete-source inventory, including inactive saved sets.
    assert_eq!((sets, groups, instances), (15, 200, 541));
}
#[test]
fn malformed_configuration_does_not_erase_authored_skills_or_trigger_a_calculation() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("caller.xml");
    let xml = "<PathOfBuilding2><Skills activeSkillSet='9'><SkillSet id='9' title='Caller'><Skill enabled='true'><Gem nameSpec='Future skill' skillId='CallerEffect' note='raw\n\tvalue'/></Skill></SkillSet></Skills><Config><Input name='bad' boolean='maybe'/></Config></PathOfBuilding2>";
    fs::write(&input, xml).unwrap();
    let report = success(
        cli()
            .current_dir(temp.path())
            .arg("inspect-build")
            .arg(&input)
            .arg("--with-definitions")
            .output()
            .unwrap(),
    );
    assert_eq!(report["configuration"]["status"], "not_projected");
    assert_eq!(report["skills"]["status"], "source_projected");
    assert_eq!(
        count_role(&report["skills"]["projection"], "gem_instance"),
        1
    );
    assert_eq!(
        report["definition_lookup"]["configuration"]["status"],
        "not_looked_up"
    );
    assert_eq!(
        report["definition_lookup"]["game_mechanics"],
        "not_evaluated"
    );
    assert_eq!(report["verification"]["native_admission"], "not_checked");
    assert_eq!(report["verification"]["reference_calculation"], "not_run");
    assert_eq!(fs::read_to_string(&input).unwrap(), xml);
}

#[test]
fn selected_catalog_changes_reference_resolution_without_loading_a_build_evaluator() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("caller.xml");
    let data_path = temp.path().join("caller-data.json");
    let mut package = poe_optimizer_data::game_data::bundled_snapshot()
        .unwrap()
        .package()
        .clone();
    let gem = &mut package.skill_identities.gems[0];
    let expected_key = gem.key.clone();
    let expected_effect = gem.primary_effect_id.clone();
    gem.game_id = "Caller/Injected".into();
    gem.variant_id = "caller-variant".into();
    let declaration =
        &mut package.skill_identities.gem_declarations[gem.winning_declaration as usize - 1];
    declaration.identity.game_id = gem.game_id.clone();
    declaration.identity.variant_id = gem.variant_id.clone();
    preparation_edits::refresh_lookups(&mut package);
    package.refresh_section_digests().unwrap();
    let bytes = package.canonical_bytes().unwrap();
    fs::write(&data_path, &bytes).unwrap();
    let xml = "<PathOfBuilding2><Skills activeSkillSet='3'><SkillSet id='3'><Skill><Gem gemId='Caller/Injected' variantId='caller-variant' skillId='Caller/Wrong' nameSpec='Caller supplied name'/></Skill></SkillSet></Skills></PathOfBuilding2>";
    fs::write(&input, xml).unwrap();
    let reviewed = success(
        cli()
            .current_dir(temp.path())
            .arg("inspect-build")
            .arg(&input)
            .arg("--with-definitions")
            .output()
            .unwrap(),
    );
    let before =
        &reviewed["definition_lookup"]["skills"]["lookup"]["records"][0]["record"]["lookup"];
    assert_eq!(before["resolution"]["kind"], "external_gem");
    assert_eq!(before["resolution"]["status"], "missing");
    let selected = success(
        cli()
            .current_dir(temp.path())
            .arg("inspect-build")
            .arg(&input)
            .arg("--data")
            .arg(&data_path)
            .output()
            .unwrap(),
    );
    assert_eq!(
        selected["definition_lookup"]["data_trust"]["status"],
        "custom_unreviewed"
    );
    let lookup = &selected["definition_lookup"]["skills"]["lookup"];
    assert_eq!(
        lookup["interpretation"],
        "authored_identity_before_socket_group_processing"
    );
    assert_eq!(lookup["socket_group_processing"], "not_run");
    assert_eq!(lookup["actor_resolution"], "not_resolved");
    let resolved = &lookup["records"][0]["record"]["lookup"];
    assert_eq!(resolved["skill_id"]["decoded"], "Caller/Wrong");
    assert_eq!(resolved["resolution"]["status"], "exact");
    assert_eq!(resolved["resolution"]["candidates"][0]["key"], expected_key);
    assert_eq!(
        resolved["resolution"]["candidates"][0]["primary_effect_id"],
        expected_effect
    );
    assert_eq!(selected["verification"]["native_admission"], "not_checked");
    assert_eq!(selected["verification"]["reference_calculation"], "not_run");
    assert_eq!(fs::read_to_string(input).unwrap(), xml);
    assert_eq!(fs::read(data_path).unwrap(), bytes);
}
#[test]
fn rejected_dataset_trust_never_publishes_an_identity_report_or_overwrites_input() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("caller.xml");
    let data_path = temp.path().join("data.json");
    let output = temp.path().join("output.json");
    let xml = "<PathOfBuilding2><Skills><SkillSet id='8'><Skill><Gem nameSpec='Caller'/></Skill></SkillSet></Skills></PathOfBuilding2>";
    fs::write(&input, xml).unwrap();
    fs::write(
        &data_path,
        poe_optimizer_data::game_data::bundled_package_bytes(),
    )
    .unwrap();
    let rejected = cli()
        .arg("inspect-build")
        .arg(&input)
        .arg("--data")
        .arg(&data_path)
        .arg("--data-sha256")
        .arg("0".repeat(64))
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert!(!rejected.status.success());
    assert!(!output.exists());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("SHA-256"));
    let collision = cli()
        .arg("inspect-build")
        .arg(&input)
        .arg("--with-definitions")
        .arg("--output")
        .arg(&input)
        .output()
        .unwrap();
    assert!(!collision.status.success());
    assert_eq!(fs::read_to_string(&input).unwrap(), xml);
}
