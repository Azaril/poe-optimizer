//! Owned rule CLI integration from explicit files with no game snapshot or PoB.
#[path = "../crates/poe-optimizer-engine/tests/support/owned_rule_fixture.rs"]
mod fixture;
use poe_optimizer_core::{owned_definitions::OwnedDefinitionKey, owned_rules::*, owned_schema::*};
use poe_optimizer_data::owned_schema::{OwnedSchemaLimits, encode_schema_package};
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap()
}
fn args() -> Vec<&'static str> {
    vec![
        "check-owned-rules",
        "rules.json",
        "--definitions",
        "definitions.json",
        "--probe",
        "probe.json",
        "--canonical-output",
        "canonical.json",
    ]
}
fn save(dir: &Path, fixture: &fixture::Fixture, index: usize) {
    fs::write(
        dir.join("definitions.json"),
        encode_schema_package(&fixture.schema, OwnedSchemaLimits::default()).unwrap(),
    )
    .unwrap();
    fs::write(
        dir.join("rules.json"),
        serde_json::to_vec(&fixture.rules).unwrap(),
    )
    .unwrap();
    let case = &fixture.cases[index];
    fs::write(dir.join("probe.json"),serde_json::to_vec(&json!({"owner":case.owner,"program":case.program,"facts":case.facts.iter().map(|(read,value)|json!({"read":read,"value":value})).collect::<Vec<_>>()})).unwrap()).unwrap();
}
fn success(out: Output) -> Value {
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}
#[test]
fn explicit_packages_probe_all_effect_families_from_an_empty_directory() {
    let fixture = fixture::fixture();
    for (index, case) in fixture.cases.iter().enumerate() {
        let temp = tempfile::tempdir().unwrap();
        save(temp.path(), &fixture, index);
        let report = success(run(temp.path(), &args()));
        assert_eq!(report["verification"]["operations"], "compiled");
        assert_eq!(report["verification"]["provider_resolution"], "not_run");
        assert_eq!(
            report["verification"]["whole_build_parity"],
            "not_established"
        );
        assert_eq!(
            report["verification"]["calculation"],
            "explicit_fact_component"
        );
        for (actual, expected) in report["probe"]["effects"]
            .as_array()
            .unwrap()
            .iter()
            .zip(&case.expected)
        {
            assert_eq!(
                actual["id"],
                serde_json::to_value(&expected.id).unwrap(),
                "{}",
                case.name
            );
            if let Some(value) = &expected.value {
                assert_eq!(actual["disposition"]["kind"], "applied", "{}", case.name);
                assert_eq!(
                    actual["disposition"]["value"],
                    serde_json::to_value(value).unwrap(),
                    "{}",
                    case.name
                );
            } else {
                assert_eq!(actual["disposition"]["kind"], "inactive", "{}", case.name);
            }
        }
        assert_eq!(
            report["probe"]["effects"].as_array().unwrap().len(),
            case.expected.len()
        );
        let before = fs::read(temp.path().join("canonical.json")).unwrap();
        assert!(!run(temp.path(), &args()).status.success());
        assert_eq!(
            before,
            fs::read(temp.path().join("canonical.json")).unwrap()
        );
    }
}
#[test]
fn hidden_cycle_and_unknown_operations_fail_before_writing_canonical_data() {
    let mut fixture = fixture::fixture();
    fixture.rules.owners[0].programs.members[0]
        .nodes
        .push(RuleNode {
            id: OwnedDefinitionKey::new("unreachable-cycle").unwrap(),
            expression: RuleExpression::Not {
                value: OwnedDefinitionKey::new("unreachable-cycle").unwrap(),
            },
        });
    let temp = tempfile::tempdir().unwrap();
    save(temp.path(), &fixture, 0);
    assert!(!run(temp.path(), &args()).status.success());
    assert!(!temp.path().join("canonical.json").exists());
    fixture.rules = fixture::fixture().rules;
    for version in [
        "owned-domain-operations-v1",
        "owned-domain-operations-v2",
        "future-unknown-operation-set",
    ] {
        fixture.rules.operations_version = OwnedDefinitionKey::new(version).unwrap();
        save(temp.path(), &fixture, 0);
        assert!(!run(temp.path(), &args()).status.success(), "{version}");
        assert!(!temp.path().join("canonical.json").exists());
    }
}
#[test]
fn pending_owner_membership_remains_visible_after_known_program_execution() {
    let mut fixture = fixture::fixture();
    let owner = &mut fixture.rules.owners[0];
    owner.programs.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: owner.owner.clone(),
            facet: SchemaFacet::GameRules,
            code: OwnedDefinitionKey::new("unconverted-effects").unwrap(),
        }],
    };
    let temp = tempfile::tempdir().unwrap();
    save(temp.path(), &fixture, 0);
    let report = success(run(temp.path(), &args()));
    assert_eq!(report["partial_owners"], 1);
    assert_eq!(report["probe"]["owner_programs_closure"]["kind"], "partial");
}
#[test]
fn stale_artifacts_duplicate_facts_and_oversized_probes_do_not_publish() {
    let fixture = fixture::fixture();
    for bad in ["binding", "duplicate-fact", "unknown-field", "oversized"] {
        let temp = tempfile::tempdir().unwrap();
        save(temp.path(), &fixture, 0);
        if bad == "binding" {
            let mut data = serde_json::to_value(&fixture.rules).unwrap();
            data["definitions"]["content_sha256"] = json!("ab".repeat(32));
            fs::write(
                temp.path().join("rules.json"),
                serde_json::to_vec(&data).unwrap(),
            )
            .unwrap();
        } else if bad == "oversized" {
            fs::File::create(temp.path().join("probe.json"))
                .unwrap()
                .set_len(8 * 1024 * 1024 + 1)
                .unwrap();
        } else {
            let path = temp.path().join("probe.json");
            let mut p: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            if bad == "unknown-field" {
                p["unexpected"] = json!(true);
            } else {
                let row = p["facts"][0].clone();
                p["facts"].as_array_mut().unwrap().push(row);
            }
            fs::write(path, serde_json::to_vec(&p).unwrap()).unwrap();
        }
        assert!(!run(temp.path(), &args()).status.success(), "{bad}");
        assert!(!temp.path().join("canonical.json").exists(), "{bad}");
    }
}
#[test]
fn help_and_compilation_only_need_explicit_artifacts() {
    let temp = tempfile::tempdir().unwrap();
    let help = run(temp.path(), &["check-owned-rules", "--help"]);
    assert!(help.status.success());
    assert!(
        String::from_utf8(help.stdout)
            .unwrap()
            .contains("--definitions")
    );
    let fixture = fixture::fixture();
    save(temp.path(), &fixture, 0);
    let report = success(run(
        temp.path(),
        &[
            "check-owned-rules",
            "rules.json",
            "--definitions",
            "definitions.json",
        ],
    ));
    assert!(report["probe"].is_null());
    assert_eq!(report["verification"]["calculation"], "not_run");
    assert!(!temp.path().join("canonical.json").exists());
}
