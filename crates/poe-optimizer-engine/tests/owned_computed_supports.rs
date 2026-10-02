//! Complete synthetic requests exercise computed input authority, not PoB fixture names.
#[allow(dead_code)]
#[path = "support/owned_computed_support_fixture.rs"]
mod support;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*, owned_stages::*,
};
use poe_optimizer_data::{
    owned_rules::{OwnedRulePackage, RuleStorageLimits},
    owned_schema::OwnedDefinitionSchemaPackage,
    owned_stages::{OwnedEvaluationStages, StageStorageLimits},
    owned_support_inputs::{OwnedSupportInputBindings, SupportInputStorageLimits},
};
use poe_optimizer_engine::{owned_plan::*, owned_rules::*, owned_supports::*};
use std::sync::Arc;
use support::*;

type Args = SupportPreparationPlanInputs<OwnedDefinitionSchemaPackage>;
fn origin(f: &mut Fixture) -> &mut RuleProgram {
    &mut f
        .owner_mut(&subject(def::<GemDefinition>("support")))
        .programs
        .members[0]
}
fn invert_levels(f: &mut Fixture) {
    let p = origin(f);
    p.nodes
        .iter_mut()
        .find(|n| n.id == key("effective"))
        .unwrap()
        .expression = RuleExpression::Subtract {
        left: key("base"),
        right: key("level"),
    };
}
fn stored(f: &Fixture, args: &Args) -> OwnedRulePackage {
    OwnedRulePackage::new(
        RulePackageInput {
            effect_applications: None,
            schema_version: OWNED_RULE_PACKAGE_VERSION,
            namespace: ns(),
            release: key("rules"),
            semantics_version: key("test-v1"),
            operations_version: key(OWNED_RULE_OPERATIONS_V12),
            definitions: args.definitions.identity().clone(),
            tables: f.tables.clone(),
            owners: f.owners.clone(),
            receivers: f.receivers.clone(),
        },
        args.definitions.as_ref(),
        RuleStorageLimits::default(),
    )
    .unwrap()
}
fn restage(
    f: &Fixture,
    args: &mut Args,
    stage: EvaluationStagesInput,
) -> std::result::Result<(), String> {
    let rules = stored(f, args);
    let stages = Arc::new(
        OwnedEvaluationStages::new(
            stage,
            args.definitions.as_ref(),
            &rules,
            &args.routing,
            StageStorageLimits::default(),
        )
        .map_err(|e| e.to_string())?,
    );
    let mut inputs = args.inputs.input().clone();
    inputs.stages = *stages.identity();
    args.inputs = Arc::new(
        OwnedSupportInputBindings::new(
            inputs,
            args.definitions.as_ref(),
            &rules,
            &args.preparation,
            &stages,
            SupportInputStorageLimits::default(),
        )
        .map_err(|e| e.to_string())?,
    );
    args.stages = stages;
    Ok(())
}
fn stages(f: &Fixture, args: &Args) -> EvaluationStagesInput {
    let mut stages = args.stages.input().clone();
    stages.stages = vec![
        EvaluationStage {
            id: key("base"),
            predecessors: vec![],
        },
        EvaluationStage {
            id: key("facts"),
            predecessors: vec![key("base")],
        },
        EvaluationStage {
            id: key("origins"),
            predecessors: vec![key("facts")],
        },
        EvaluationStage {
            id: key("prepare"),
            predecessors: vec![key("origins")],
        },
        EvaluationStage {
            id: key("later"),
            predecessors: vec![key("prepare")],
        },
    ];
    for row in &mut stages.programs.members {
        let p = f
            .owners
            .iter()
            .find(|o| o.owner == row.owner)
            .unwrap()
            .programs
            .members
            .iter()
            .find(|p| p.id == row.program)
            .unwrap();
        row.stage = key(match p.context {
            RuleEntityKind::SupportOrigin => "origins",
            RuleEntityKind::Skill => "facts",
            _ => "base",
        });
    }
    for frozen in &mut stages.frozen_channels {
        frozen.stage = key(match frozen.channel {
            StageChannel::Stat {
                scope: RuleEntityKind::SupportOrigin,
                ..
            } => "origins",
            _ => "facts",
        });
    }
    stages
}
fn finish(
    args: Args,
) -> std::result::Result<OwnedSupportPreparationPlan<OwnedDefinitionSchemaPackage>, PlanError> {
    OwnedSupportPreparationPlan::compile(
        args,
        PlanLimits::default(),
        SupportPreparationLimits::default(),
    )
}

#[test]
fn computed_origin_levels_reverse_physical_gem_order_without_sharing_siblings() {
    let mut f = generated_fixture();
    invert_levels(&mut f);
    for (use_id, slot, winner) in [(30, "first", 60), (30, "second", 62), (31, "first", 64)] {
        let report = evaluate(&f, target(use_id, slot));
        assert_eq!(prepared(&report).selected[0].assignment, occurrence(winner));
    }
}

#[test]
fn computed_quality_breaks_computed_level_ties_with_no_physical_quality_fallback() {
    let mut f = generated_fixture();
    let p = origin(&mut f);
    p.nodes
        .iter_mut()
        .find(|n| n.id == key("effective"))
        .unwrap()
        .expression = RuleExpression::Literal { value: integer(10) };
    p.nodes.extend([
        literal("three", 3),
        node(
            "quality-count",
            RuleExpression::Subtract {
                left: key("three"),
                right: key("level"),
            },
        ),
        node(
            "quality-point",
            RuleExpression::Literal {
                value: ParameterValue::Quantity(FiniteQuantity::new(1.0, def("quality")).unwrap()),
            },
        ),
    ]);
    p.nodes
        .iter_mut()
        .find(|n| n.id == key("quality"))
        .unwrap()
        .expression = RuleExpression::ScaleInteger {
        value: key("quality-point"),
        count: key("quality-count"),
    };
    assert!(f.build.gems.iter().all(|gem| gem.quality.is_none()));
    assert_eq!(
        prepared(&evaluate(&f, target(30, "first"))).selected[0].assignment,
        occurrence(60)
    );
}

#[test]
fn global_contribution_changes_reach_the_computed_support_selection() {
    let mut f = generated_fixture();
    let mut equipment = Fixture::new();
    f.owner_mut(&class_owner()).programs = equipment.owner_mut(&class_owner()).programs.clone();
    f.build.items = equipment.build.items;
    f.build.equipment = equipment.build.equipment;
    let p = origin(&mut f);
    p.reads.push(read(
        "player",
        RuleReadSource::Stat {
            entity: RuleEntity::Player,
            stat: def("actor-total"),
        },
    ));
    p.nodes.extend([
        read_node("player", "player"),
        literal("threshold", 20),
        node(
            "above",
            RuleExpression::Compare {
                operation: RuleComparison::Greater,
                left: key("player"),
                right: key("threshold"),
            },
        ),
        node(
            "inverse",
            RuleExpression::Subtract {
                left: key("base"),
                right: key("level"),
            },
        ),
    ]);
    p.nodes
        .iter_mut()
        .find(|n| n.id == key("effective"))
        .unwrap()
        .expression = RuleExpression::Select {
        condition: key("above"),
        when_true: key("level"),
        when_false: key("inverse"),
    };
    let before = compile(&f, target(30, "first"));
    f.build.items[0].modifiers[0].rolls[0].value = integer(10);
    let after = compile(&f, target(30, "first"));
    let mut scratch = before.new_scratch();
    let a = before.evaluate(&mut scratch).unwrap();
    let b = after.evaluate(&mut scratch).unwrap();
    assert_eq!(prepared(&a).selected[0].assignment, occurrence(60));
    assert_eq!(prepared(&b).selected[0].assignment, occurrence(61));
    assert_eq!(before.evaluate(&mut scratch).unwrap(), a);
    assert_ne!(a.identity, b.identity);
}

#[test]
fn authored_skill_targets_use_computed_facts_and_respect_enabled_state() {
    let mut f = generated_fixture();
    let mut target_program = f
        .owner_mut(&subject(def::<SkillDefinition>("ability")))
        .programs
        .members[0]
        .clone();
    target_program.reads.clear();
    target_program.nodes[0] = literal("base", 30);
    f.owner_mut(&subject(def::<SkillDefinition>("skill")))
        .programs
        .members
        .push(target_program);
    let target = SkillTarget::Authored(occurrence(20));
    f.build.skills.push(SkillUse {
        id: occurrence(20),
        source: AuthoredSkillSource::Direct(def("skill")),
        enabled: true,
        scope: LoadoutScope::Shared,
    });
    f.build
        .supports
        .retain(|s| s.id == occurrence(60) || s.id == occurrence(61));
    for support in &mut f.build.supports {
        support.target = target.clone();
    }
    f.build.support_origins = Some(vec![SupportOriginSequence {
        target: target.clone(),
        origins: vec![
            SupportOrigin::Assignment(occurrence(60)),
            SupportOrigin::Assignment(occurrence(61)),
        ],
    }]);
    assert_eq!(
        prepared(&evaluate(&f, target.clone())).selected[0].assignment,
        occurrence(61)
    );
    f.build
        .skills
        .iter_mut()
        .find(|s| s.id == occurrence(20))
        .unwrap()
        .enabled = false;
    assert!(matches!(
        evaluate(&f, target).outcome,
        ComputedSupportOutcome::Prepared {
            result: SupportPreparationOutcome::Inactive { .. }
        }
    ));
}

#[test]
fn explicit_stages_preserve_selection_and_bind_their_identity() {
    let f = generated_fixture();
    let baseline = evaluate(&f, target(30, "first"));
    let mut args = compile_inputs(&f, target(30, "first"));
    let stage_input = stages(&f, &args);
    restage(&f, &mut args, stage_input).unwrap();
    let plan = finish(args).unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert_eq!(prepared(&report), prepared(&baseline));
    assert_ne!(report.identity, baseline.identity);
}

#[test]
fn implicit_generated_activation_cannot_depend_on_a_later_stage() {
    let f = generated_fixture();
    let mut args = compile_inputs(&f, target(30, "first"));
    let mut stage_input = stages(&f, &args);
    for row in &mut stage_input.programs.members {
        if row.owner == subject(def::<ActorDefinition>("family")) {
            row.stage = key("later");
        }
    }
    restage(&f, &mut args, stage_input).unwrap();
    assert!(matches!(finish(args), Err(PlanError::Invalid(message)) if message.contains("stage")));
}

fn partial(subject: SchemaSubject) -> SchemaClosure {
    SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject,
            facet: SchemaFacet::GameRules,
            code: key("unconverted"),
        }],
    }
}

#[test]
fn whole_owner_coverage_is_required_even_with_only_one_support() {
    let mut f = generated_fixture();
    f.build.supports.retain(|s| s.id != occurrence(61));
    for row in f.build.support_origins.as_mut().unwrap() {
        row.origins
            .retain(|s| *s != SupportOrigin::Assignment(occurrence(61)));
    }
    f.owner_mut(&class_owner()).programs.closure = partial(class_owner());
    let report = evaluate(&f, target(30, "first"));
    assert!(matches!(
        report.outcome,
        ComputedSupportOutcome::Unavailable {
            cause: EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                ..
            },
            ..
        }
    ));
    assert!(
        report
            .gaps
            .iter()
            .any(|g| g.reason == PlanGapReason::PartialPrograms)
    );
}

#[test]
fn a_partial_stage_partition_does_not_create_execution_authority() {
    let f = generated_fixture();
    let mut args = compile_inputs(&f, target(30, "first"));
    let mut stages = args.stages.input().clone();
    stages.programs.closure = partial(class_owner());
    restage(&f, &mut args, stages).unwrap();
    let plan = finish(args).unwrap();
    assert!(matches!(
        plan.evaluate(&mut plan.new_scratch()).unwrap().outcome,
        ComputedSupportOutcome::Unavailable { .. }
    ));
}

#[test]
fn unsupported_delivery_is_not_skipped_to_claim_complete_preparation() {
    let mut f = generated_fixture();
    f.owner_mut(&subject(def::<GemDefinition>("support")))
        .programs
        .members
        .push(RuleProgram {
            id: key("delivery"),
            context: RuleEntityKind::Actor,
            reads: vec![],
            nodes: vec![bool_node("yes", true)],
            effects: vec![effect(
                "require",
                RuleEffectKind::Requirement {
                    satisfied: key("yes"),
                    code: key("required"),
                },
            )],
        });
    let report = evaluate(&f, target(30, "first"));
    assert!(matches!(
        report.outcome,
        ComputedSupportOutcome::Unavailable { .. }
    ));
    assert!(
        report
            .gaps
            .iter()
            .any(|g| g.reason == PlanGapReason::UnsupportedRelation)
    );
    // Ordinary effect plans retain their own support-delivery rejection too.
    assert!(
        f.compile()
            .unwrap()
            .gaps()
            .iter()
            .any(|g| g.reason == PlanGapReason::UnsupportedRelation)
    );
}

#[test]
fn missing_effective_value_retains_exact_physical_origin_and_stat() {
    let mut f = generated_fixture();
    origin(&mut f)
        .effects
        .retain(|effect| effect.id != key("level"));
    let report = evaluate(&f, target(30, "first"));
    let ComputedSupportOutcome::Unavailable {
        cause,
        input: Some(input),
    } = report.outcome
    else {
        panic!("{report:?}")
    };
    assert!(matches!(
        cause,
        EffectValue::Unresolved {
            reason: PlanGapReason::MissingProducer,
            ..
        }
    ));
    assert_eq!(
        *input,
        PlanValueKey::Stat {
            entity: ConcreteEntity::SupportOrigin(SupportOrigin::Assignment(occurrence(61))),
            stat: def("effective-level")
        }
    );
}

#[test]
fn missing_target_fact_is_not_treated_as_false_and_names_its_skill() {
    let mut f = generated_fixture();
    f.owner_mut(&subject(def::<SkillDefinition>("ability")))
        .programs
        .members[0]
        .effects
        .retain(|effect| effect.id != key("cannot-support"));
    let target = target(30, "first");
    let report = evaluate(&f, target.clone());
    let ComputedSupportOutcome::Unavailable {
        input: Some(input), ..
    } = report.outcome
    else {
        panic!("{report:?}")
    };
    assert_eq!(
        *input,
        PlanValueKey::Stat {
            entity: ConcreteEntity::Skill(Box::new(target)),
            stat: def("cannot-support")
        }
    );
}

#[test]
fn one_budget_spans_graph_export_and_preparation_and_failure_does_not_taint_retry() {
    let f = generated_fixture();
    let plan = compile(&f, target(30, "first"));
    let mut scratch = plan.new_scratch();
    let maximum = PlanLimits::default().max_work;
    let mut work = maximum;
    let expected = plan.evaluate_with_budget(&mut scratch, &mut work).unwrap();
    let cost = maximum - work;
    let mut short = cost - 1;
    assert!(matches!(
        plan.evaluate_with_budget(&mut scratch, &mut short),
        Err(PlanError::Limit("work"))
    ));
    assert_eq!(short, 0);
    assert_eq!(plan.evaluate(&mut scratch).unwrap(), expected);
    let args = compile_inputs(&f, target(30, "first"));
    let small = OwnedSupportPreparationPlan::compile(
        args,
        PlanLimits::default(),
        SupportPreparationLimits {
            max_work: 1,
            ..Default::default()
        },
    )
    .unwrap();
    let mut work = maximum;
    assert!(matches!(
        small.evaluate_with_budget(&mut scratch, &mut work),
        Err(PlanError::Limit("work"))
    ));
    assert!(
        work > 0 && work < maximum,
        "a component cap must preserve unused outer work"
    );
}

#[test]
fn invalid_and_tightened_component_limits_apply_before_success_or_growth() {
    let f = generated_fixture();
    for limits in [
        SupportPreparationLimits {
            max_work: 0,
            ..Default::default()
        },
        SupportPreparationLimits {
            max_origins: 0,
            ..Default::default()
        },
    ] {
        assert!(
            OwnedSupportPreparationPlan::compile(
                compile_inputs(&f, target(30, "first")),
                PlanLimits::default(),
                limits
            )
            .is_err()
        );
    }
    let plan = OwnedSupportPreparationPlan::compile(
        compile_inputs(&f, target(30, "first")),
        PlanLimits::default(),
        SupportPreparationLimits {
            max_origins: 1,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(matches!(
        plan.evaluate(&mut plan.new_scratch()),
        Err(PlanError::Limit("origins"))
    ));
}

#[test]
fn raw_compilation_cannot_impersonate_a_stored_rules_artifact() {
    let f = generated_fixture();
    let mut args = compile_inputs(&f, target(30, "first"));
    args.rules = Arc::new(
        CompiledRulePackage::compile(
            args.rules.input(),
            args.definitions.as_ref(),
            RuleLimits::default(),
        )
        .unwrap(),
    );
    assert!(
        matches!(finish(args), Err(PlanError::Invalid(message)) if message.contains("stored package"))
    );
}
