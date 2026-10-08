//! Native retained support applications close the ordinary graph in one attempt.
#[allow(dead_code)]
#[path = "support/owned_support_delivery_fixture.rs"]
mod delivery_fixture;
use delivery_fixture::*;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::RuleEntityKind,
};
use poe_optimizer_engine::{owned_plan::*, owned_supports::*};
use rayon::prelude::*;
use std::collections::BTreeSet;
use std::sync::Arc;
#[test]
fn missing_composed_source_coverage_blocks_retained_delivery_before_selection() {
    let f = source_fixture();
    let args = inputs_with_rules(&f, |rules| rules.support_discovery = None, |_| {});
    let plan =
        OwnedSupportEffectPlan::compile(args, Default::default(), Default::default()).unwrap();
    let result = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(matches!(
        result.outcome,
        SupportEffectsOutcome::Unavailable { .. }
    ));
    assert!(
        result
            .gaps
            .iter()
            .any(|gap| gap.reason == PlanGapReason::MissingSupportSources)
    );
}
#[test]
fn retained_applications_feed_exact_action_and_actor_final_reductions() {
    let plan = compile(&source_fixture());
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let effects = evaluated(&report);
    let applications: BTreeSet<_> = effects.effects.iter().filter_map(application).collect();
    assert_eq!(
        applications.len(),
        9,
        "three retained origins with one actor and two action receivers each"
    );
    assert_eq!(
        applications
            .iter()
            .filter(|a| matches!(a.receiver, SupportReceiverKey::Actor(_)))
            .count(),
        3
    );
    for a in applications {
        assert!(
            [61, 63, 65]
                .into_iter()
                .any(|n| a.prepared.origin == SupportOrigin::Assignment(occurrence(n)))
        );
        assert_eq!(a.prepared.position, 0);
        let expected = if a.prepared.target == target(31, "first") {
            25
        } else {
            16
        };
        let deliveries: Vec<_> = effects
            .effects
            .iter()
            .filter(|e| {
                application(e) == Some(a) && e.key.invocation.program.as_str().ends_with("deliver")
            })
            .collect();
        assert_eq!(deliveries.len(), 1);
        assert_eq!(deliveries[0].value, known(expected));
        let app = effects
            .values
            .iter()
            .find(|v| {
                v.key
                    == PlanValueKey::SupportApplicability {
                        application: Box::new(a.clone()),
                    }
            })
            .unwrap();
        assert_eq!(
            app.value,
            EffectValue::Known {
                value: ParameterValue::Boolean(true)
            }
        );
        if let SupportReceiverKey::Action(action) = &a.receiver {
            assert_eq!(action.action.provider.grant_path.len(), 2);
            assert_eq!(
                final_value(effects, &ConcreteEntity::Action(action.clone())),
                &known(expected)
            );
        }
    }
    assert_eq!(
        final_value(effects, &ConcreteEntity::Actor(child_actor(30))),
        &known(32)
    );
    assert_eq!(
        final_value(effects, &ConcreteEntity::Actor(child_actor(31))),
        &known(25)
    );
}

#[test]
fn native_false_applicability_is_a_boolean_producer_and_delivery_is_inactive() {
    let mut f = source_fixture();
    let program = f
        .owner_mut(&subject(def::<SkillDefinition>("ability")))
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == key("target-facts"))
        .unwrap();
    program
        .nodes
        .iter_mut()
        .find(|n| n.id == key("spell"))
        .unwrap()
        .expression = RuleExpression::Literal {
        value: ParameterValue::Boolean(false),
    };
    let plan = compile(&f);
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let effects = evaluated(&report);
    let apps: Vec<_> = effects
        .values
        .iter()
        .filter(|v| matches!(v.key, PlanValueKey::SupportApplicability { .. }))
        .collect();
    assert_eq!(apps.len(), 9);
    assert!(apps.iter().all(|v| v.value
        == EffectValue::Known {
            value: ParameterValue::Boolean(false)
        }));
    assert!(
        effects
            .effects
            .iter()
            .filter(|e| application(e).is_some()
                && e.key.invocation.program.as_str().ends_with("deliver"))
            .all(|e| e.value == EffectValue::Inactive)
    );
    assert_eq!(
        final_value(effects, &ConcreteEntity::Actor(child_actor(30))),
        &known(0)
    );
    assert_eq!(
        final_value(effects, &ConcreteEntity::Actor(child_actor(31))),
        &known(0)
    );
}

#[test]
fn missing_or_partial_inventory_keeps_the_whole_attempt_unavailable() {
    for mode in 0..5 {
        let mut f = source_fixture();
        if mode == 4 {
            f.owner_mut(&fixture::class_owner()).programs.closure = partial(fixture::class_owner());
        }
        let args = inputs(&f, |input| match mode {
            0 => input.supports.clear(),
            1 => input.targets.clear(),
            2 => input.supports[0].receivers.closure = partial(support_owner()),
            3 => {
                input.targets[0].roles.closure = partial(subject(def::<SkillDefinition>("ability")))
            }
            _ => {}
        });
        let plan = Plan::compile(
            args,
            PlanLimits::default(),
            SupportPreparationLimits::default(),
        )
        .unwrap();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert!(
            matches!(
                report.outcome,
                SupportEffectsOutcome::Unavailable {
                    cause: EffectValue::Unresolved {
                        reason: PlanGapReason::IncompleteContributors,
                        ..
                    },
                    ..
                }
            ),
            "mode {mode}: {report:?}"
        );
    }
}

#[test]
fn absent_authored_order_cannot_be_replaced_by_assignment_id_sorting() {
    let mut f = source_fixture();
    f.build.authored_support_order = None;
    let plan = compile(&f);
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(
        matches!(
            report.outcome,
            SupportEffectsOutcome::PreparationUnresolved {
                reason: SupportPreparationGap::OriginOrder,
                ..
            }
        ),
        "{report:?}"
    );
}

#[test]
fn scratch_reuse_a_b_a_and_budget_failure_match_fresh_complete_attempts() {
    let a = compile(&source_fixture());
    let mut changed = source_fixture();
    changed
        .build
        .gems
        .iter_mut()
        .find(|g| g.id == occurrence(29))
        .unwrap()
        .level = 30;
    let b = compile(&changed);
    let mut scratch = a.new_scratch();
    let first = a.evaluate(&mut scratch).unwrap();
    evaluated(&first);
    let middle = b.evaluate(&mut scratch).unwrap();
    assert_eq!(middle, b.evaluate(&mut b.new_scratch()).unwrap());
    assert_ne!(first.identity, middle.identity);
    assert_eq!(
        final_value(evaluated(&middle), &ConcreteEntity::Actor(child_actor(31))),
        &known(35)
    );
    let mut work = 1;
    assert!(matches!(
        b.evaluate_with_budget(&mut scratch, &mut work),
        Err(PlanError::Limit("work"))
    ));
    assert_eq!(work, 0);
    let mut work = PlanLimits::default().max_work;
    let again = a.evaluate_with_budget(&mut scratch, &mut work).unwrap();
    assert_eq!(first, again);
    let mut fresh_work = PlanLimits::default().max_work;
    assert_eq!(
        again,
        a.evaluate_with_budget(&mut a.new_scratch(), &mut fresh_work)
            .unwrap()
    );
    assert_eq!(work, fresh_work);
}

#[test]
fn disabled_generated_ancestor_never_instantiates_its_retained_applications() {
    let mut f = source_fixture();
    f.build
        .skills
        .iter_mut()
        .find(|s| s.id == occurrence(30))
        .unwrap()
        .enabled = false;
    let plan = compile(&f);
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let effects = evaluated(&report);
    let apps: BTreeSet<_> = effects.effects.iter().filter_map(application).collect();
    assert_eq!(apps.len(), 3);
    assert!(
        apps.iter()
            .all(|a| a.prepared.target == target(31, "first"))
    );
    assert_eq!(
        final_value(effects, &ConcreteEntity::Actor(child_actor(31))),
        &known(25)
    );
}

#[test]
fn preparation_rejects_potential_late_contributors_even_when_admission_would_be_false() {
    let mut f = source_fixture();
    let owner = f.owner_mut(&subject(def::<SkillDefinition>("ability")));
    owner
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == key("target-facts"))
        .unwrap()
        .nodes
        .iter_mut()
        .find(|n| n.id == key("spell"))
        .unwrap()
        .expression = RuleExpression::Literal {
        value: ParameterValue::Boolean(false),
    };
    f.owner_mut(&subject(def::<ActorDefinition>("family")))
        .programs
        .members
        .push(RuleProgram {
            id: key("prefix-observer"),
            context: RuleEntityKind::Actor,
            reads: vec![fixture::contributions(
                "late",
                RuleEntity::Actor,
                "support-total",
            )],
            nodes: vec![
                fixture::read_node("value", "late"),
                fixture::literal("zero", 0),
                fixture::node(
                    "satisfied",
                    RuleExpression::Compare {
                        operation: RuleComparison::GreaterOrEqual,
                        left: key("value"),
                        right: key("zero"),
                    },
                ),
            ],
            effects: vec![effect(
                "require",
                RuleEffectKind::Requirement {
                    satisfied: key("satisfied"),
                    code: key("late-proof"),
                },
            )],
        });
    let error = Plan::compile(
        inputs(&f, |_| {}),
        PlanLimits::default(),
        SupportPreparationLimits::default(),
    )
    .err()
    .expect("late contributions must not have a proven empty prefix identity");
    assert!(
        matches!(error, PlanError::Invalid(message) if message.contains("preparation prefix depends on a potential support delivery channel"))
    );
}

#[test]
fn retained_delivery_cannot_replace_an_existing_final_producer() {
    let args = inputs_with_rules(
        &source_fixture(),
        |rules| {
            let delivery = rules
                .owners
                .iter_mut()
                .find(|row| row.owner == support_owner())
                .unwrap()
                .programs
                .members
                .iter_mut()
                .find(|p| p.id == key("actor-deliver"))
                .unwrap();
            delivery.effects[0].effect = RuleEffectKind::Derive {
                entity: RuleEntity::Current,
                stat: def("support-total"),
                value: key("value"),
            };
        },
        |_| {},
    );
    let plan = Plan::compile(
        args,
        PlanLimits::default(),
        SupportPreparationLimits::default(),
    )
    .unwrap();
    let mut scratch = plan.new_scratch();
    let error = plan.evaluate(&mut scratch).unwrap_err();
    assert!(
        matches!(error, PlanError::Invalid(message) if message.contains("competing final producers"))
    );
    let valid = compile(&source_fixture());
    assert_eq!(
        valid.evaluate(&mut scratch).unwrap(),
        valid.evaluate(&mut valid.new_scratch()).unwrap()
    );
}

#[test]
fn dynamic_contribution_cycles_are_rejected_before_running_the_suffix() {
    let args = inputs_with_rules(
        &source_fixture(),
        |rules| {
            let delivery = rules
                .owners
                .iter_mut()
                .find(|row| row.owner == support_owner())
                .unwrap()
                .programs
                .members
                .iter_mut()
                .find(|p| p.id == key("actor-deliver"))
                .unwrap();
            delivery.reads = vec![fixture::read(
                "final",
                RuleReadSource::Stat {
                    entity: RuleEntity::Current,
                    stat: def("support-total"),
                },
            )];
            delivery.nodes = vec![fixture::read_node("value", "final")];
        },
        |_| {},
    );
    let plan = Plan::compile(
        args,
        PlanLimits::default(),
        SupportPreparationLimits::default(),
    )
    .unwrap();
    let mut scratch = plan.new_scratch();
    let error = plan.evaluate(&mut scratch).unwrap_err();
    assert!(
        matches!(error, PlanError::Invalid(ref message) if message.contains("cycle")),
        "{error}"
    );
    let valid = compile(&source_fixture());
    assert_eq!(
        valid.evaluate(&mut scratch).unwrap(),
        valid.evaluate(&mut valid.new_scratch()).unwrap()
    );
}

#[test]
fn shared_support_plan_parallel_workers_match_serial_reports_and_remaining_work() {
    let plan = Arc::new(compile(&source_fixture()));
    let mut scratch = plan.new_scratch();
    let expected: Vec<_> = (0..32)
        .map(|_| {
            let mut work = PlanLimits::default().max_work;
            let report = plan.evaluate_with_budget(&mut scratch, &mut work).unwrap();
            evaluated(&report);
            (report, work)
        })
        .collect();
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let actual: Vec<_> = pool.install(|| {
        (0..32)
            .into_par_iter()
            .map_init(
                || plan.new_scratch(),
                |scratch, _| {
                    let mut work = PlanLimits::default().max_work;
                    let report = plan.evaluate_with_budget(scratch, &mut work).unwrap();
                    (report, work)
                },
            )
            .collect()
    });
    assert_eq!(actual, expected);
}
