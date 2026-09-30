//! The CLI consumes explicit prepared-output artifacts through the native library.
// This CLI target uses only a subset of the shared engine fixture's exports.
#[allow(dead_code, unused_imports)]
#[path = "../crates/poe-optimizer-engine/tests/support/owned_support_output_fixture.rs"]
mod support;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_metrics::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::{owned_metrics::*, owned_schema::*};
use poe_optimizer_engine::{owned_plan::*, owned_supports::*};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
    sync::Arc,
};
use support::{def, key};

fn save(dir: &Path) -> OwnedSupportMetricReport {
    let mut f = support::source_fixture();
    f.schema
        .definitions
        .push(DefinitionDescriptor::Stat(DefinitionEntry {
            id: def("prepared-quantity"),
            schema: SchemaState::Known(StatSchema {
                value: ComputedValueType::Quantity { unit: def("count") },
                targets: vec![RuleEntityKind::Action],
            }),
        }));
    let program = &mut f
        .owner_mut(&SchemaSubject::Slot(SlotAddress::ActionOutput(
            support::delivery::output(),
        )))
        .programs
        .members[0];
    program.nodes.extend([
        support::fixture::node(
            "one-unit",
            RuleExpression::Literal {
                value: ParameterValue::Quantity(FiniteQuantity::new(1.0, def("count")).unwrap()),
            },
        ),
        support::fixture::node(
            "measurement",
            RuleExpression::ScaleInteger {
                value: key("one-unit"),
                count: key("prepared-result"),
            },
        ),
    ]);
    program.effects.push(support::fixture::derive(
        "measurement",
        RuleEntity::Current,
        "prepared-quantity",
        "measurement",
    ));
    f.queries.requests = vec![MetricRequest {
        id: QueryId::new("prepared-action").unwrap(),
        metric: def("requested"),
        target: MetricTarget::Action(Box::new(support::action(30, "first"))),
    }];
    let (args, rules, outputs) = support::authored_inputs(&f);
    let mapping = Arc::new(
        OwnedMetricMapping::new(
            MetricMappingInput {
                schema_version: OWNED_METRIC_MAPPING_VERSION,
                namespace: support::fixture::ns(),
                release: key("mapping"),
                definitions: args.definitions.identity().clone(),
                bindings: vec![MetricStatBinding {
                    metric: def("requested"),
                    role: MetricBindingRole::Action,
                    stat: def("prepared-quantity"),
                }],
            },
            args.definitions.as_ref(),
            Default::default(),
        )
        .unwrap(),
    );
    fs::write(
        dir.join("request.json"),
        encode_owned(
            &OwnedDocument::Request(Box::new((*args.request).clone())),
            Default::default(),
        )
        .unwrap(),
    )
    .unwrap();
    fs::write(
        dir.join("schema.json"),
        encode_schema_package(&args.definitions, Default::default()).unwrap(),
    )
    .unwrap();
    for (name, value) in [
        ("rules", serde_json::to_value(rules.input()).unwrap()),
        (
            "routing",
            serde_json::to_value(args.routing.input()).unwrap(),
        ),
        ("stages", serde_json::to_value(args.stages.input()).unwrap()),
        (
            "preparation",
            serde_json::to_value(args.preparation.input()).unwrap(),
        ),
        ("inputs", serde_json::to_value(args.inputs.input()).unwrap()),
        (
            "receiving",
            serde_json::to_value(args.receiving.input()).unwrap(),
        ),
        ("outputs", serde_json::to_value(outputs.input()).unwrap()),
        ("metrics", serde_json::to_value(mapping.input()).unwrap()),
    ] {
        fs::write(
            dir.join(format!("{name}.json")),
            serde_json::to_vec(&value).unwrap(),
        )
        .unwrap();
    }
    let effects = Arc::new(
        OwnedSupportEffectPlan::compile_with_outputs(
            args,
            outputs,
            PlanLimits::default(),
            SupportPreparationLimits::default(),
        )
        .unwrap(),
    );
    let plan = OwnedSupportMetricPlan::compile(effects, mapping).unwrap();
    plan.evaluate(&mut plan.new_scratch()).unwrap()
}

fn run(dir: &Path, outputs: bool, support_packages: bool) -> Output {
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
    if support_packages {
        command.args([
            "--stages",
            "stages.json",
            "--support-preparation",
            "preparation.json",
            "--support-inputs",
            "inputs.json",
            "--support-receiving",
            "receiving.json",
        ]);
    }
    if outputs {
        command.args(["--support-outputs", "outputs.json"]);
    }
    command.output().unwrap()
}

#[test]
fn explicit_prepared_type_artifact_roundtrips_through_cli_without_numeric_fallback() {
    let directory = tempfile::tempdir().unwrap();
    let expected = save(directory.path());
    let result = run(directory.path(), true, true);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let actual: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(actual["schema_version"], 3);
    assert_eq!(
        actual["evaluation"],
        serde_json::to_value(&expected.evaluation).unwrap()
    );
    assert_eq!(
        actual["support_preparation"],
        serde_json::to_value(&expected.support).unwrap()
    );
    let EffectValue::Known {
        value: ParameterValue::Quantity(value),
    } = &expected.evaluation.results[0].value
    else {
        panic!("expected known output metric: {expected:?}");
    };
    assert_eq!(value.value(), 16.0);
    let without = run(directory.path(), false, true);
    assert!(
        without.status.success(),
        "{}",
        String::from_utf8_lossy(&without.stderr)
    );
    let without: serde_json::Value = serde_json::from_slice(&without.stdout).unwrap();
    assert_ne!(
        actual["evaluation"]["identity"],
        without["evaluation"]["identity"]
    );
    assert_eq!(
        without["evaluation"]["results"][0]["value"]["status"],
        "unresolved"
    );
}

#[test]
fn output_artifact_requires_full_support_dependencies_and_exact_join() {
    let directory = tempfile::tempdir().unwrap();
    let result = run(directory.path(), true, false);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("required arguments"));
    assert!(result.stdout.is_empty());
    save(directory.path());
    let path = directory.path().join("outputs.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["rules"] = value["receiving"].clone();
    fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
    let result = run(directory.path(), true, true);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("binding mismatch"));
    assert!(result.stdout.is_empty());
}
