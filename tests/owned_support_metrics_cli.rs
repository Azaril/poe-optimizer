//! CLI support evaluation uses the portable metric plan and injected owned artifacts.
#[allow(dead_code)]
#[path = "../crates/poe-optimizer-engine/tests/support/owned_support_delivery_fixture.rs"]
mod support;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_metrics::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::{owned_metrics::*, owned_rules::*, owned_schema::*};
use poe_optimizer_engine::{owned_plan::*, owned_supports::SupportPreparationLimits};
use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
    sync::Arc,
};
use support::{child_actor, def, key, occurrence};

fn fixture(level: u16) -> (support::Inputs, Arc<OwnedMetricMapping>, OwnedRulePackage) {
    let mut f = support::source_fixture();
    f.build
        .gems
        .iter_mut()
        .find(|g| g.id == occurrence(51))
        .unwrap()
        .level = level;
    for definition in &mut f.schema.definitions {
        if let DefinitionDescriptor::Metric(row) = definition {
            row.schema = SchemaState::Known(MetricSchema {
                targets: vec![MetricTargetKind::Actor, MetricTargetKind::Action],
                unit: def("count"),
                actor_roles: vec![MetricActorRole::Owned],
                provider_roles: vec![ProviderRole::SkillUse],
            });
        }
    }
    f.schema
        .definitions
        .push(DefinitionDescriptor::Stat(DefinitionEntry {
            id: def("support-measurement"),
            schema: SchemaState::Known(StatSchema {
                value: ComputedValueType::Quantity { unit: def("count") },
                targets: vec![RuleEntityKind::Actor, RuleEntityKind::Action],
            }),
        }));
    for owner in &mut f.owners {
        for program in &mut owner.programs.members {
            if [key("actor-final"), key("action-final")].contains(&program.id) {
                program.nodes.extend([
                    RuleNode {
                        id: key("one-unit"),
                        expression: RuleExpression::Literal {
                            value: ParameterValue::Quantity(
                                FiniteQuantity::new(1.0, def("count")).unwrap(),
                            ),
                        },
                    },
                    RuleNode {
                        id: key("measurement"),
                        expression: RuleExpression::ScaleInteger {
                            value: key("one-unit"),
                            count: key("value"),
                        },
                    },
                ]);
                program.effects.push(support::fixture::derive(
                    "measure",
                    RuleEntity::Current,
                    "support-measurement",
                    "measurement",
                ));
            }
        }
    }
    let SkillTarget::Generated(generated) = support::target(30, "first") else {
        unreachable!()
    };
    let mut provider = generated.provider;
    provider.grant_path.push(DeclaredSlot {
        declaration: SlotOwnerDefId::Actor(def("family")),
        slot: def("first"),
    });
    let action = ActionSelection {
        action: ActionKey {
            actor: child_actor(30),
            provider,
            output: support::output(),
        },
        part: def("part"),
        mode: def("mode"),
        stat_set: def("set"),
    };
    f.queries.requests = [
        ("actor-a", MetricTarget::Actor(child_actor(30))),
        ("action-a", MetricTarget::Action(Box::new(action))),
        ("actor-b", MetricTarget::Actor(child_actor(31))),
        ("actor-a-again", MetricTarget::Actor(child_actor(30))),
    ]
    .into_iter()
    .map(|(id, target)| MetricRequest {
        id: QueryId::new(id).unwrap(),
        metric: def("requested"),
        target,
    })
    .collect();
    let mut source = None;
    let input = support::inputs_with_rules(&f, |rules| source = Some(rules.clone()), |_| {});
    let stored = OwnedRulePackage::new(
        source.unwrap(),
        input.definitions.as_ref(),
        RuleStorageLimits::default(),
    )
    .unwrap();
    assert_eq!(input.rules.source_identity(), Some(*stored.identity()));
    let mapping = Arc::new(
        OwnedMetricMapping::new(
            MetricMappingInput {
                schema_version: OWNED_METRIC_MAPPING_VERSION,
                namespace: support::fixture::ns(),
                release: key("support-metrics"),
                definitions: input.definitions.identity().clone(),
                bindings: [MetricBindingRole::OwnedActor, MetricBindingRole::Action]
                    .into_iter()
                    .map(|role| MetricStatBinding {
                        metric: def("requested"),
                        role,
                        stat: def("support-measurement"),
                    })
                    .collect(),
            },
            input.definitions.as_ref(),
            MetricMappingLimits::default(),
        )
        .unwrap(),
    );
    (input, mapping, stored)
}
fn save(
    dir: &Path,
    input: &support::Inputs,
    mapping: &OwnedMetricMapping,
    rules: &OwnedRulePackage,
) {
    fs::write(
        dir.join("request.json"),
        encode_owned(
            &OwnedDocument::Request(Box::new((*input.request).clone())),
            OwnedInputLimits::default(),
        )
        .unwrap(),
    )
    .unwrap();
    fs::write(
        dir.join("schema.json"),
        encode_schema_package(&input.definitions, OwnedSchemaLimits::default()).unwrap(),
    )
    .unwrap();
    for (name, value) in [
        ("rules.json", serde_json::to_value(rules.input()).unwrap()),
        (
            "routing.json",
            serde_json::to_value(input.routing.input()).unwrap(),
        ),
        (
            "stages.json",
            serde_json::to_value(input.stages.input()).unwrap(),
        ),
        (
            "preparation.json",
            serde_json::to_value(input.preparation.input()).unwrap(),
        ),
        (
            "inputs.json",
            serde_json::to_value(input.inputs.input()).unwrap(),
        ),
        (
            "receiving.json",
            serde_json::to_value(input.receiving.input()).unwrap(),
        ),
        (
            "metrics.json",
            serde_json::to_value(mapping.input()).unwrap(),
        ),
    ] {
        fs::write(dir.join(name), serde_json::to_vec(&value).unwrap()).unwrap();
    }
}

const SUPPORT_FLAGS: [(&str, &str); 4] = [
    ("--stages", "stages.json"),
    ("--support-preparation", "preparation.json"),
    ("--support-inputs", "inputs.json"),
    ("--support-receiving", "receiving.json"),
];
fn run(dir: &Path, support_mask: usize, extra: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    command.current_dir(dir).args([
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
    ]);
    for (index, (flag, path)) in SUPPORT_FLAGS.iter().enumerate() {
        if support_mask & (1 << index) != 0 {
            command.args([*flag, *path]);
        }
    }
    command.args(extra).output().unwrap()
}

#[test]
fn every_incomplete_support_artifact_set_is_rejected_before_loading() {
    let directory = tempfile::tempdir().unwrap();
    for mask in 1..15 {
        let result = run(directory.path(), mask, &["--output", "report.json"]);
        assert!(!result.status.success(), "subset {mask} was accepted");
        assert!(
            String::from_utf8_lossy(&result.stderr).contains("required arguments"),
            "subset {mask}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(result.stdout.is_empty());
        assert!(!directory.path().join("report.json").exists());
    }
}

#[test]
fn support_cli_matches_native_ordered_metrics_and_preserves_output() {
    for level in [2, 7] {
        let directory = tempfile::tempdir().unwrap();
        let (input, mapping, rules) = fixture(level);
        save(directory.path(), &input, &mapping, &rules);
        let effect_plan = Arc::new(
            OwnedSupportEffectPlan::compile(
                input,
                PlanLimits::default(),
                SupportPreparationLimits::default(),
            )
            .unwrap(),
        );
        let plan = OwnedSupportMetricPlan::compile(effect_plan, mapping).unwrap();
        let expected = plan.evaluate(&mut plan.new_scratch()).unwrap();
        let result = run(directory.path(), 15, &["--output", "report.json"]);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let report: Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(report["schema_version"], 3);
        assert_eq!(report["document_kind"], "owned_metric_report");
        assert_eq!(
            report["evaluation"],
            serde_json::to_value(&expected.evaluation).unwrap()
        );
        assert_eq!(
            report["support_preparation"],
            serde_json::to_value(&expected.support).unwrap()
        );
        assert_eq!(report["support_preparation"]["status"], "evaluated");
        assert_eq!(
            report["bindings"],
            serde_json::to_value(plan.bindings()).unwrap()
        );
        assert_eq!(
            report["binding_report"],
            serde_json::to_value(plan.binding_report()).unwrap()
        );
        assert_eq!(
            report["verification"]["whole_build_parity"],
            "not_established"
        );
        let numbers: Vec<_> = expected
            .evaluation
            .results
            .iter()
            .map(|row| {
                let EffectValue::Known {
                    value: ParameterValue::Quantity(q),
                } = &row.value
                else {
                    panic!("expected quantity: {row:?}")
                };
                assert_eq!(q.unit(), &def("count"));
                q.value()
            })
            .collect();
        let action = 12.0 + 2.0 * f64::from(level);
        assert_eq!(numbers, vec![action + 16.0, action, 25.0, action + 16.0]);
        assert_eq!(
            fs::read(directory.path().join("report.json")).unwrap(),
            result.stdout
        );
        assert!(
            !run(directory.path(), 15, &["--output", "report.json"])
                .status
                .success()
        );
        assert_eq!(
            fs::read(directory.path().join("report.json")).unwrap(),
            result.stdout
        );
    }
}

#[test]
fn stale_support_bindings_fail_before_report_publication() {
    let directory = tempfile::tempdir().unwrap();
    let (input, mapping, rules) = fixture(2);
    for (_, file) in SUPPORT_FLAGS {
        save(directory.path(), &input, &mapping, &rules);
        let path = directory.path().join(file);
        let mut artifact: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        artifact["definitions"]["content_sha256"] = Value::String("0".repeat(64));
        fs::write(&path, serde_json::to_vec(&artifact).unwrap()).unwrap();
        let result = run(directory.path(), 15, &["--output", "report.json"]);
        assert!(!result.status.success(), "accepted stale {file}");
        assert!(result.stdout.is_empty());
        assert!(!directory.path().join("report.json").exists());
    }
    // Changing only the authored rule release still invalidates exact package
    // bindings even though the executable arithmetic has not changed.
    save(directory.path(), &input, &mapping, &rules);
    let path = directory.path().join("rules.json");
    let mut artifact: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    artifact["release"] = Value::String("other-release".into());
    fs::write(&path, serde_json::to_vec(&artifact).unwrap()).unwrap();
    let result = run(directory.path(), 15, &["--output", "report.json"]);
    assert!(!result.status.success());
    assert!(result.stdout.is_empty());
    assert!(!directory.path().join("report.json").exists());
}

#[test]
fn missing_source_order_is_reported_without_numeric_fallback() {
    let directory = tempfile::tempdir().unwrap();
    let (mut input, mapping, rules) = fixture(2);
    let mut build = input.request.build().input().clone();
    build.authored_support_order = None;
    let limits = OwnedInputLimits::default();
    input.request = Arc::new(
        OwnedEvaluationRequest::new(
            BuildSpec::new(build, limits).unwrap(),
            input.request.scenario().clone(),
            input.request.queries().clone(),
            limits,
        )
        .unwrap(),
    );
    save(directory.path(), &input, &mapping, &rules);
    let effect_plan = Arc::new(
        OwnedSupportEffectPlan::compile(
            input,
            PlanLimits::default(),
            SupportPreparationLimits::default(),
        )
        .unwrap(),
    );
    let plan = OwnedSupportMetricPlan::compile(effect_plan, mapping).unwrap();
    let expected = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let result = run(directory.path(), 15, &[]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(
        report["support_preparation"],
        serde_json::to_value(&expected.support).unwrap()
    );
    assert_eq!(
        report["support_preparation"]["status"],
        "preparation_unresolved"
    );
    assert_eq!(
        report["evaluation"],
        serde_json::to_value(&expected.evaluation).unwrap()
    );
    assert_eq!(expected.evaluation.results.len(), 4);
    assert!(
        expected
            .evaluation
            .results
            .iter()
            .all(|r| !matches!(r.value, EffectValue::Known { .. }))
    );
}
