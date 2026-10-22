//! The CLI shares the portable metric API; no PoB checkout or snapshot is read.
#[allow(dead_code, unused_imports)]
#[path = "../crates/poe-optimizer-engine/tests/support/owned_fixtures.rs"]
mod engine_fixtures;

#[allow(dead_code)]
use crate::engine_fixtures::metrics as support;
use poe_optimizer_core::{
    owned_binding::*, owned_build::*, owned_routing::*, owned_rules::*, owned_schema::*,
};
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
        support_discovery: Some(SupportDiscoveryInput {
            providers: f
                .owners
                .iter()
                .map(|row| SupportSourceDomainDeclaration {
                    owner: row.owner.clone(),
                    domain: SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly),
                })
                .collect(),
        }),
        existing_actor_rules: None,
        contribution_queries: None,
        effect_applications: None,
        receivers: DeclaredSet::complete(vec![]),
        tables: f.tables.clone(),
        schema_version: OWNED_RULE_PACKAGE_VERSION,
        namespace: ns(),
        release: key("rules"),
        semantics_version: key("test-v1"),
        operations_version: key(OWNED_RULE_OPERATIONS_V14),
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
        assert_eq!(report["schema_version"], 2);
        assert_eq!(
            report["binding_report"],
            serde_json::to_value(
                bind_owned_request(
                    plan.effect_plan().definitions(),
                    &f.request(),
                    BindingLimits::default()
                )
                .unwrap()
            )
            .unwrap()
        );
        assert_eq!(report["document_kind"], "owned_metric_report");
        assert_eq!(
            report["verification"]["requirement_scope"],
            "injected_requirements"
        );
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

fn output_owned_measurements(mut f: Fixture) -> Fixture {
    // Keep the same measurements while isolating Actor requirements from this
    // fixture's V14 rule for unqueried Skill-owned Action contexts.
    let owner = subject(def::<poe_optimizer_core::owned_definitions::SkillDefinition>("skill"));
    let programs = &mut f.owner_mut(&owner).programs.members;
    let index = programs
        .iter()
        .position(|program| program.id == key("quantity"))
        .unwrap();
    let quantity = programs.remove(index);
    f.owner_mut(&SchemaSubject::Slot(SlotAddress::ActionOutput(output())))
        .programs
        .members
        .push(quantity);
    f
}

fn with_requirements() -> Fixture {
    let mut f = output_owned_measurements(fixture());
    f.schema
        .definitions
        .push(DefinitionDescriptor::Stat(DefinitionEntry {
            id: def("unknown-requirement"),
            schema: SchemaState::Known(StatSchema {
                value: ComputedValueType::Boolean,
                targets: vec![RuleEntityKind::Actor],
            }),
        }));
    f.owner_mut(&class_owner())
        .programs
        .members
        .push(RuleProgram {
            id: key("requirements"),
            context: RuleEntityKind::Actor,
            reads: vec![RuleRead {
                id: key("unknown"),
                value_type: ComputedValueType::Boolean,
                source: RuleReadSource::Stat {
                    entity: RuleEntity::Current,
                    stat: def("unknown-requirement"),
                },
            }],
            nodes: vec![
                node(
                    "true",
                    RuleExpression::Literal {
                        value: ParameterValue::Boolean(true),
                    },
                ),
                node(
                    "false",
                    RuleExpression::Literal {
                        value: ParameterValue::Boolean(false),
                    },
                ),
                read_node("unknown", "unknown"),
            ],
            effects: [
                ("satisfied", "true", None),
                ("violated", "false", None),
                ("unavailable", "unknown", None),
                ("inapplicable", "false", Some("false")),
            ]
            .into_iter()
            .map(|(id, value, when)| RuleEffect {
                id: key(id),
                when: when.map(key),
                effect: RuleEffectKind::Requirement {
                    satisfied: key(value),
                    code: key(id),
                },
            })
            .collect(),
        });
    f
}

#[test]
fn requirements_are_reported_with_empty_or_unrelated_queries_without_hiding_measurements() {
    let mut assessment = None;
    for requests in 0..3 {
        let mut baseline = output_owned_measurements(fixture());
        let mut f = with_requirements();
        match requests {
            0 => {
                baseline.queries.requests.clear();
                f.queries.requests.clear();
            }
            1 => {
                baseline
                    .queries
                    .requests
                    .retain(|q| q.id.as_str() == "child-a");
                f.queries.requests.retain(|q| q.id.as_str() == "child-a");
            }
            _ => {}
        }
        let baseline_plan = compile(&baseline);
        let baseline = baseline_plan
            .evaluate(&mut baseline_plan.new_scratch())
            .unwrap();
        let plan = compile(&f);
        let expected = plan.evaluate(&mut plan.new_scratch()).unwrap();
        // Requirements describe feasibility, not the availability of otherwise
        // independent measurements, including an unrelated supplied Actor.
        assert_eq!(expected.results, baseline.results);
        let dir = tempfile::tempdir().unwrap();
        save(dir.path(), &f);
        let output = run(dir.path(), &[]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            report["evaluation"],
            serde_json::to_value(&expected).unwrap()
        );
        assert_eq!(
            report["verification"]["requirement_scope"],
            "injected_requirements"
        );
        assert_eq!(report["verification"]["game_legality"], "not_checked");
        let actual = &report["evaluation"]["requirements"];
        let rows = actual["results"].as_array().unwrap();
        assert_eq!(rows.len(), 4);
        for state in ["satisfied", "violated", "unavailable", "inapplicable"] {
            let row = rows.iter().find(|row| row["code"] == state).unwrap();
            assert_eq!(row["outcome"]["status"], state);
            assert!(!row["key"]["invocation"]["origin"].is_null());
            assert!(!row["key"]["invocation"]["entity"].is_null());
        }
        assert!(expected.requirements.has_violations());
        assert!(!expected.requirements.is_satisfied());
        if let Some(previous) = &assessment {
            assert_eq!(actual, previous);
        }
        assessment = Some(actual.clone());
    }
}

#[test]
fn incomplete_requirement_inventory_keeps_known_violations_and_does_not_claim_legality() {
    let mut f = with_requirements();
    f.queries.requests.clear();
    f.owner_mut(&class_owner()).programs.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: class_owner(),
            facet: SchemaFacet::GameRules,
            code: key("unconverted-requirements"),
        }],
    };
    let plan = compile(&f);
    let expected = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    save(dir.path(), &f);
    let output = run(dir.path(), &[]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["evaluation"],
        serde_json::to_value(&expected).unwrap()
    );
    assert_eq!(
        report["evaluation"]["requirements"]["coverage"]["status"],
        "incomplete"
    );
    assert!(
        report["evaluation"]["results"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(expected.requirements.has_violations());
    assert!(!expected.requirements.is_satisfied());
    assert_eq!(report["verification"]["game_legality"], "not_checked");
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

#[test]
fn binding_diagnostics_preserve_exact_modifier_sites_and_unavailable_query_rows() {
    let dir = tempfile::tempdir().unwrap();
    let mut f = fixture();
    let owner = modifier_owner();
    let modifier = f
        .schema
        .definitions
        .iter_mut()
        .find_map(|row| match row {
            DefinitionDescriptor::Modifier(row) if row.id == def("modifier") => Some(row),
            _ => None,
        })
        .unwrap();
    let SchemaState::Known(modifier) = &mut modifier.schema else {
        panic!("known fixture modifier");
    };
    modifier.declarations.parameters.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: owner.clone(),
            facet: SchemaFacet::InputSchema,
            code: key("unconverted-modifier-inputs"),
        }],
    };
    f.build
        .skills
        .iter_mut()
        .find(|skill| skill.id == occurrence(30))
        .unwrap()
        .enabled = false;
    let plan = compile(&f);
    let request = f.request();
    let expected = bind_owned_request(
        plan.effect_plan().definitions(),
        &request,
        BindingLimits::default(),
    )
    .unwrap();
    assert_eq!(expected.schema(), SchemaBindingStatus::Unresolved);
    assert_eq!(plan.effect_plan().binding_report(), &expected);
    assert!(expected.issues().contains(&BindingIssue {
        site: BindingSite {
            location: BindingLocation::Modifier {
                item: f.build.items[0].id,
                modifier: f.build.items[0].modifiers[0].id,
            },
            facet: BindingFacet::RequiredValues,
        },
        class: IssueClass::Unresolved,
        code: BindingIssueCode::PartialMembership,
        subject: Some(owner),
    }));
    for id in ["child-a", "child-a-again"] {
        let query = expected
            .queries()
            .iter()
            .find(|query| query.id.as_str() == id)
            .unwrap();
        assert_eq!(query.selector, SelectorBindingStatus::Unavailable);
        assert!(expected.issues().iter().any(|issue| {
            issue.site.location == BindingLocation::Query(query.id.clone())
                && issue.class == IssueClass::Unavailable
                && issue.code == BindingIssueCode::DisabledProvider
        }));
    }
    save(dir.path(), &f);
    let output = run(dir.path(), &[]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["schema_version"], 2);
    assert_eq!(
        report["binding_report"],
        serde_json::to_value(&expected).unwrap()
    );
    assert_eq!(
        report["bindings"],
        serde_json::to_value(plan.bindings()).unwrap()
    );
    let evaluation = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert_eq!(
        report["evaluation"],
        serde_json::to_value(&evaluation).unwrap()
    );
    assert_eq!(evaluation.results.len(), f.queries.requests.len());
    assert_eq!(
        report["evaluation"]["results"][0]["value"]["status"],
        "inactive"
    );
    assert_eq!(
        report["evaluation"]["results"][4]["value"]["status"],
        "inactive"
    );
}
