//! Authored evaluation relationships construct the existing checked graph.
//! Only a complete checked owner can discard inherited dependency commitments.
use super::{files, fixture};
use poe_optimizer_authoring::OwnedReleaseEvaluationContent;
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_content::{ContentDigestError, digest_owned},
    owned_definitions::OwnedDefinitionKey,
    owned_rules::{RuleExpression, RuleNode},
    owned_schema::{SchemaClosure, SchemaFacet, SchemaGap, SchemaState},
    owned_supports::SupportTypePredicate,
};
use poe_optimizer_data::{
    owned_evaluation_budget::{EvaluationBudgetLimits, charge_evaluation_entries},
    owned_stages::StageStorageError,
};
use poe_optimizer_engine::owned_plan::{
    OwnedSupportEffectPlan, OwnedSupportMetricPlan, OwnedSupportMetricReport,
    SupportEffectPlanInputs,
};
use poe_optimizer_import::{
    owned_recipe::CheckedOwnedRecipe,
    owned_release::*,
    owned_release_evaluation::{OwnedReleaseEvaluationError, OwnedReleaseEvaluationInput},
};
use std::sync::Arc;

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}

fn stage(input: OwnedReleaseInput) -> StagedOwnedRelease {
    assemble_owned_release(input, Default::default()).unwrap()
}

fn content(
    input: OwnedReleaseInput,
) -> (
    CheckedOwnedRecipe,
    OwnedReleaseEvaluationInput,
    OwnedReleaseEvaluationContent,
) {
    let release = stage(input);
    let checked = load_owned_release_artifacts(&files(&release), Default::default()).unwrap();
    let recipe = checked.recipe().clone();
    let original = checked.evaluation().unwrap().input().clone();
    let authored = OwnedReleaseEvaluationContent::from_checked(checked.into_evaluation().unwrap());
    (recipe, original, authored)
}

fn report(release: &CheckedOwnedRelease) -> OwnedSupportMetricReport {
    let recipe = release.recipe();
    let evaluation = release.evaluation().unwrap();
    let support = evaluation.support().unwrap();
    let effects = Arc::new(
        OwnedSupportEffectPlan::compile_with_outputs(
            SupportEffectPlanInputs {
                request: Arc::new(fixture().request.clone()),
                definitions: Arc::clone(recipe.schema()),
                rules: Arc::clone(recipe.compiled_rules()),
                routing: Arc::clone(recipe.routing()),
                stages: Arc::new(support.stages().clone()),
                preparation: Arc::new(support.preparation().clone()),
                inputs: Arc::new(support.inputs().clone()),
                receiving: Arc::new(support.receiving().clone()),
            },
            Arc::new(support.outputs().unwrap().clone()),
            Default::default(),
            Default::default(),
        )
        .unwrap(),
    );
    let plan =
        OwnedSupportMetricPlan::compile(effects, Arc::new(evaluation.metrics().clone())).unwrap();
    plan.evaluate(&mut plan.new_scratch()).unwrap()
}

#[test]
fn checked_content_roundtrip_preserves_input_receipt_and_runtime_result() {
    let baseline = stage(fixture().input.clone());
    let (recipe, original, authored) = content(fixture().input.clone());
    let bytes = serde_json::to_vec(&authored).unwrap();
    let restored: OwnedReleaseEvaluationContent = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(restored, authored);
    let rebuilt = restored.into_input(&recipe, Default::default()).unwrap();
    assert_eq!(rebuilt, original);
    assert_eq!(
        serde_json::to_vec(&rebuilt).unwrap(),
        serde_json::to_vec(&original).unwrap()
    );
    let mut input = baseline.input().clone();
    input.evaluation = Some(rebuilt);
    let rebuilt = stage(input);
    assert_eq!(rebuilt.receipt(), baseline.receipt());
    assert_eq!(files(&rebuilt), files(&baseline));
    let checked = load_owned_release_artifacts(&files(&rebuilt), Default::default()).unwrap();
    assert_eq!(report(&checked), fixture().expected);
    assert_eq!(
        OwnedReleaseEvaluationContent::from_checked(checked.into_evaluation().unwrap()),
        authored
    );
}

#[test]
fn authored_content_binds_each_dependency_to_the_changed_checked_recipe() {
    let (_, original, authored) = content(fixture().input.clone());
    let saved = authored.clone();
    let baseline = stage(fixture().input.clone());
    let mut input = baseline.input().clone();
    let program = input
        .recipe
        .rules
        .owners
        .iter_mut()
        .flat_map(|owner| &mut owner.programs.members)
        .find(|program| program.id == key("action-final"))
        .unwrap();
    // Change the exact rule package without changing any numeric expression or
    // referenced owner/program. This tests dependency construction independently
    // of source labels and without weakening scheduling coverage.
    program.nodes.push(RuleNode {
        id: key("authored-content-marker"),
        expression: RuleExpression::Literal {
            value: ParameterValue::Boolean(false),
        },
    });
    let recipe = CheckedOwnedRecipe::new(input.recipe.clone(), Default::default()).unwrap();
    assert_ne!(
        recipe.rules().identity(),
        baseline.assembled().rules().identity()
    );
    assert!(matches!(
        assemble_owned_release(input.clone(), Default::default()),
        Err(OwnedReleaseError::Evaluation(_))
    ));
    let rebuilt = authored.into_input(&recipe, Default::default()).unwrap();
    assert_eq!(rebuilt.metrics, original.metrics);
    let old = original.support.as_ref().unwrap();
    let support = rebuilt.support.as_ref().unwrap();
    for binding in [
        support.stages.rules,
        support.preparation.rules,
        support.inputs.rules,
        support.receiving.rules,
        support.outputs.as_ref().unwrap().rules,
    ] {
        assert_eq!(binding, *recipe.rules().identity());
    }
    assert_eq!(support.stages.routing, *recipe.routing().identity());
    assert_ne!(support.inputs.preparation, old.inputs.preparation);
    assert_ne!(support.inputs.stages, old.inputs.stages);
    assert_eq!(support.receiving.preparation, support.inputs.preparation);
    assert_eq!(support.receiving.stages, support.inputs.stages);
    assert_ne!(support.receiving.inputs, old.receiving.inputs);
    let outputs = support.outputs.as_ref().unwrap();
    assert_eq!(outputs.preparation, support.inputs.preparation);
    assert_eq!(outputs.stages, support.inputs.stages);
    assert_eq!(outputs.inputs, support.receiving.inputs);
    assert_ne!(outputs.receiving, old.outputs.as_ref().unwrap().receiving);
    input.evaluation = Some(rebuilt);
    let release = stage(input);
    let checked = load_owned_release_artifacts(&files(&release), Default::default()).unwrap();
    let result = report(&checked);
    assert_ne!(
        result.evaluation.identity,
        fixture().expected.evaluation.identity
    );
    assert_eq!(
        result.evaluation.results,
        fixture().expected.evaluation.results
    );
    assert_eq!(result.evaluation.gaps, fixture().expected.evaluation.gaps);
    assert_eq!(
        result.evaluation.requirements,
        fixture().expected.evaluation.requirements
    );
    assert_eq!(result.support, fixture().expected.support);
    assert_eq!(
        OwnedReleaseEvaluationContent::from_checked(checked.into_evaluation().unwrap()),
        saved
    );
}

#[test]
fn source_prefix_checks_and_runtime_fragments_never_authorize_seal_stripping() {
    let mut stale = fixture().input.clone();
    stale
        .evaluation
        .as_mut()
        .unwrap()
        .support
        .as_mut()
        .unwrap()
        .stages
        .rules = digest_owned("stale-evaluation-source", &1, 100).unwrap();
    // This boundary authenticates Import dependencies only. Success is not a
    // checked evaluation owner and cannot be consumed by from_checked().
    check_release_source_dependencies(&stale, Default::default()).unwrap();
    assert!(matches!(
        assemble_owned_release(stale, Default::default()),
        Err(OwnedReleaseError::Evaluation(_))
    ));
    let runtime = fixture().input.evaluation.as_ref().unwrap();
    assert!(
        serde_json::from_value::<OwnedReleaseEvaluationContent>(
            serde_json::to_value(runtime).unwrap()
        )
        .is_err()
    );
}

#[test]
fn authored_content_preserves_absent_evaluation_support_and_outputs() {
    let mut ordinary = fixture().input.clone();
    ordinary.evaluation = None;
    ordinary.schema_version = OWNED_RELEASE_VERSION;
    let ordinary = stage(ordinary);
    assert!(
        load_owned_release_artifacts(&files(&ordinary), Default::default())
            .unwrap()
            .into_evaluation()
            .is_none()
    );
    for support_mode in 0..3 {
        let mut input = fixture().input.clone();
        match support_mode {
            0 => input.evaluation.as_mut().unwrap().support = None,
            1 => {
                input
                    .evaluation
                    .as_mut()
                    .unwrap()
                    .support
                    .as_mut()
                    .unwrap()
                    .outputs = None
            }
            _ => {}
        }
        let (recipe, original, authored) = content(input);
        assert_eq!(authored.support.is_some(), support_mode != 0);
        if let Some(support) = &authored.support {
            assert_eq!(support.outputs.is_some(), support_mode == 2);
        }
        let wire = serde_json::to_value(&authored).unwrap();
        if support_mode == 0 {
            assert!(wire.get("support").is_none());
        } else if support_mode == 1 {
            assert!(wire["support"].get("outputs").is_none());
        }
        assert_eq!(
            authored.into_input(&recipe, Default::default()).unwrap(),
            original
        );
    }
}

#[test]
fn runtime_and_authored_views_have_identical_costs_errors_and_partial_debits() {
    let limits = EvaluationBudgetLimits::new(Default::default(), Default::default()).unwrap();
    for support_mode in 0..3 {
        let mut input = fixture().input.clone();
        match support_mode {
            0 => input.evaluation.as_mut().unwrap().support = None,
            1 => {
                input
                    .evaluation
                    .as_mut()
                    .unwrap()
                    .support
                    .as_mut()
                    .unwrap()
                    .outputs = None
            }
            _ => {}
        }
        let (_, runtime, authored) = content(input);
        let maximum = OwnedReleaseLimits::default().max_validation_entries;
        let mut remainder = maximum;
        charge_evaluation_entries((&runtime).into(), limits, &mut remainder).unwrap();
        let cost = maximum - remainder;
        for available in 0..=cost + 1 {
            let mut runtime_remaining = available;
            let mut authored_remaining = available;
            let runtime_result =
                charge_evaluation_entries((&runtime).into(), limits, &mut runtime_remaining)
                    .map_err(|error| error.to_string());
            let authored_result =
                charge_evaluation_entries((&authored).into(), limits, &mut authored_remaining)
                    .map_err(|error| error.to_string());
            assert_eq!(runtime_result, authored_result);
            assert_eq!(runtime_remaining, authored_remaining);
            assert_eq!(runtime_result.is_ok(), available >= cost);
        }
    }
}

#[test]
fn authored_coverage_is_checked_and_partial_evidence_is_preserved() {
    let (recipe, _, authored) = content(fixture().input.clone());
    let mut missing = authored.clone();
    missing
        .support
        .as_mut()
        .unwrap()
        .stages
        .programs
        .members
        .pop()
        .unwrap();
    assert!(matches!(
        missing.into_input(&recipe, Default::default()),
        Err(OwnedReleaseError::Evaluation(
            OwnedReleaseEvaluationError::Stages(StageStorageError::Invalid(
                "complete partition omits known programs"
            ))
        ))
    ));
    let mut malformed = authored.clone();
    malformed.support.as_mut().unwrap().stages.programs.closure =
        SchemaClosure::Partial { gaps: vec![] };
    assert!(matches!(
        malformed.into_input(&recipe, Default::default()),
        Err(OwnedReleaseError::Evaluation(
            OwnedReleaseEvaluationError::Stages(StageStorageError::Invalid(
                "partial program partition requires gaps"
            ))
        ))
    ));
    let mut partial = authored;
    let stages = &mut partial.support.as_mut().unwrap().stages;
    let closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: stages.programs.members[0].owner.clone(),
            facet: SchemaFacet::GameRules,
            code: key("not-yet-reviewed"),
        }],
    };
    stages.programs.closure = closure.clone();
    let rebuilt = partial.into_input(&recipe, Default::default()).unwrap();
    assert_eq!(rebuilt.support.unwrap().stages.programs.closure, closure);
}

fn limits(entries: usize) -> OwnedReleaseLimits {
    OwnedReleaseLimits {
        max_validation_entries: entries,
        ..Default::default()
    }
}

#[test]
fn shared_authoring_counter_preserves_prior_debits_and_confers_no_admission() {
    let (recipe, _, authored) = content(fixture().input.clone());
    let maximum = OwnedReleaseLimits::default().max_validation_entries;
    let mut measured = maximum;
    authored
        .charge_validation_entries(limits(maximum), &mut measured)
        .unwrap();
    let cost = maximum - measured;
    assert!(cost > 1);

    let mut exact = cost;
    authored
        .charge_validation_entries(limits(cost), &mut exact)
        .unwrap();
    assert_eq!(exact, 0);

    // Earlier source work has already used `cost` entries of this shared cap.
    let shared = limits(2 * cost + 17);
    let mut previously_debited = cost + 17;
    authored
        .charge_validation_entries(shared, &mut previously_debited)
        .unwrap();
    assert_eq!(previously_debited, 17);

    let mut twice = 2 * cost;
    for expected in [cost, 0] {
        authored
            .charge_validation_entries(limits(2 * cost), &mut twice)
            .unwrap();
        assert_eq!(twice, expected);
    }
    let mut one_short = 2 * cost - 1;
    authored
        .charge_validation_entries(limits(2 * cost - 1), &mut one_short)
        .unwrap();
    assert_eq!(one_short, cost - 1);
    assert!(matches!(
        authored.charge_validation_entries(limits(2 * cost - 1), &mut one_short),
        Err(OwnedReleaseError::Limit("validation entries"))
    ));
    // A failed traversal may have debited entries before reaching its limit.
    // Callers discard that failed compilation; this is not a rollback API.
    assert!(one_short < cost);

    let mut foreign_counter = cost + 1;
    assert!(matches!(
        authored.charge_validation_entries(limits(cost), &mut foreign_counter),
        Err(OwnedReleaseError::InvalidLimit("validation entries"))
    ));
    assert_eq!(foreign_counter, cost + 1);

    // Budget charging admits traversal only. One duplicate semantic row adds
    // exactly one entry, while the checked mapping constructor still rejects it.
    let mut duplicate = authored;
    duplicate
        .metrics
        .bindings
        .push(duplicate.metrics.bindings[0].clone());
    let mut duplicate_exact = cost + 1;
    duplicate
        .charge_validation_entries(limits(cost + 1), &mut duplicate_exact)
        .unwrap();
    assert_eq!(duplicate_exact, 0);
    assert!(matches!(
        duplicate.into_input(&recipe, limits(cost + 1)),
        Err(OwnedReleaseError::Evaluation(
            OwnedReleaseEvaluationError::Metrics(_)
        ))
    ));
}

#[test]
fn authored_aggregate_preflight_accepts_exact_budget_and_runs_before_hashing() {
    let (recipe, _, authored) = content(fixture().input.clone());
    let (mut low, mut high) = (1, OwnedReleaseLimits::default().max_validation_entries);
    authored.clone().into_input(&recipe, limits(high)).unwrap();
    while low < high {
        let middle = low + (high - low) / 2;
        match authored.clone().into_input(&recipe, limits(middle)) {
            Ok(_) => high = middle,
            Err(OwnedReleaseError::Limit("validation entries")) => low = middle + 1,
            Err(error) => panic!("unexpected aggregate admission error: {error}"),
        }
    }
    authored.clone().into_input(&recipe, limits(low)).unwrap();
    assert!(matches!(
        authored.clone().into_input(&recipe, limits(low - 1)),
        Err(OwnedReleaseError::Limit("validation entries"))
    ));
    // A duplicate metric is semantically invalid but contributes exactly one
    // additional row to structural preflight, before any constructor can reject it.
    let mut duplicate = authored.clone();
    duplicate
        .metrics
        .bindings
        .push(duplicate.metrics.bindings[0].clone());
    let mut tiny = limits(low);
    tiny.max_input_bytes = 1;
    assert!(matches!(
        duplicate.clone().into_input(&recipe, tiny),
        Err(OwnedReleaseError::Limit("validation entries"))
    ));
    tiny.max_validation_entries += 1;
    assert!(matches!(
        duplicate.clone().into_input(&recipe, tiny),
        Err(OwnedReleaseError::Digest(ContentDigestError::TooLarge {
            maximum: 1
        }))
    ));
    assert!(matches!(
        duplicate.into_input(&recipe, limits(low + 1)),
        Err(OwnedReleaseError::Evaluation(
            OwnedReleaseEvaluationError::Metrics(_)
        ))
    ));

    let mut deep = authored;
    let preparation = &mut deep.support.as_mut().unwrap().preparation;
    let SchemaState::Known(row) = &mut preparation.supports[0].preparation else {
        unreachable!()
    };
    let mut predicate = SupportTypePredicate::Type(preparation.types[0].clone());
    for _ in 0..=OwnedReleaseLimits::default()
        .evaluation
        .preparation
        .max_predicate_depth
    {
        predicate = SupportTypePredicate::Not(Box::new(predicate));
    }
    row.requires = Some(predicate);
    let tiny = OwnedReleaseLimits {
        max_input_bytes: 1,
        ..Default::default()
    };
    assert!(matches!(
        deep.into_input(&recipe, tiny),
        Err(OwnedReleaseError::Limit("support predicate depth"))
    ));
}
