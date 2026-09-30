//! Final membership is published once per exact, unambiguous preparation context.
#[allow(dead_code)]
#[path = "support/owned_support_output_fixture.rs"]
mod support;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*,
    owned_support_receiving::*,
};
use poe_optimizer_engine::{owned_plan::*, owned_supports::*};
use rayon::prelude::*;
use std::sync::Arc;
use support::delivery::occurrence;
use support::*;

#[test]
fn prepared_types_are_distinct_from_initial_inputs_and_feed_receiving_skill_reads() {
    let f = source_fixture();
    let plan = compile(&f);
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let effects = delivery::evaluated(&report);
    for target in [
        target(30, "first"),
        target(30, "second"),
        target(31, "first"),
    ] {
        assert_eq!(type_value(effects, &target, "duration"), &boolean(false));
        assert_eq!(
            type_value(effects, &target, "prepared-duration"),
            &boolean(true)
        );
        assert_eq!(
            type_value(effects, &target, "prepared-spell"),
            &boolean(true)
        );
    }
    let exports: Vec<_> = effects
        .effects
        .iter()
        .filter(|e| {
            matches!(
                e.key.invocation.origin,
                RuleOrigin::SupportPreparation { .. }
            )
        })
        .collect();
    assert_eq!(
        exports.len(),
        6,
        "two types per exact skill, independent of its two action parts"
    );
    for effect in exports {
        let RuleOrigin::SupportPreparation { context } = &effect.key.invocation.origin else {
            unreachable!()
        };
        assert_eq!(context.selection, context.target);
        assert!(context.summoner.is_none());
    }
    for (use_id, name, expected) in [(30, "first", 16), (30, "second", 16), (31, "first", 25)] {
        let key = PlanValueKey::Stat {
            entity: ConcreteEntity::Action(Box::new(action(use_id, name))),
            stat: def("prepared-result"),
        };
        assert_eq!(
            effects.values.iter().find(|v| v.key == key).unwrap().value,
            delivery::known(expected)
        );
    }
}

fn empty_target_fixture() -> Fixture {
    let mut f = source_fixture();
    f.build.supports.clear();
    f.build.support_origins = Some(vec![SupportOriginSequence {
        target: target(30, "first"),
        origins: vec![],
    }]);
    f.queries.requests = vec![MetricRequest {
        id: QueryId::new("one-action").unwrap(),
        metric: def("requested"),
        target: MetricTarget::Action(Box::new(action(30, "first"))),
    }];
    f
}

#[test]
fn consumed_zero_support_target_uses_proven_empty_order_and_does_not_visit_irrelevant_targets() {
    let f = empty_target_fixture();
    let plan = compile(&f);
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let effects = delivery::evaluated(&report);
    assert_eq!(
        type_value(effects, &target(30, "first"), "prepared-spell"),
        &boolean(true)
    );
    assert_eq!(
        type_value(effects, &target(30, "first"), "prepared-duration"),
        &boolean(false)
    );
    assert_eq!(
        effects
            .effects
            .iter()
            .filter(|e| matches!(
                e.key.invocation.origin,
                RuleOrigin::SupportPreparation { .. }
            ))
            .count(),
        2
    );
    assert!(!effects.values.iter().any(|row| row.key
        == PlanValueKey::Stat {
            entity: ConcreteEntity::Skill(Box::new(target(31, "second"))),
            stat: def("prepared-spell")
        }));
    let mut absent = f;
    absent.build.support_origins = None;
    let plan = compile(&absent);
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
fn unused_declared_reads_do_not_invent_preparation_demands() {
    let mut f = empty_target_fixture();
    f.owner_mut(&fixture::summoner_owner())
        .programs
        .members
        .push(RuleProgram {
            id: key("unused-observer"),
            context: RuleEntityKind::Skill,
            reads: vec![RuleRead {
                id: key("unused"),
                value_type: ComputedValueType::Boolean,
                source: RuleReadSource::Stat {
                    entity: RuleEntity::Skill,
                    stat: def("prepared-spell"),
                },
            }],
            nodes: vec![fixture::bool_node("yes", true)],
            effects: vec![fixture::effect(
                "require",
                RuleEffectKind::Requirement {
                    satisfied: key("yes"),
                    code: key("unused"),
                },
            )],
        });
    let (args, outputs) = inputs_with(
        &f,
        |_| {},
        |_| {},
        |stages| {
            stages
                .programs
                .members
                .iter_mut()
                .filter(|row| row.program == key("unused-observer"))
                .for_each(|row| row.stage = key("final"));
        },
        |_| {},
    );
    let plan = OwnedSupportEffectPlan::compile_with_outputs(
        args,
        outputs,
        PlanLimits::default(),
        SupportPreparationLimits::default(),
    )
    .unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let effects = delivery::evaluated(&report);
    assert_eq!(
        effects
            .effects
            .iter()
            .filter(|e| matches!(
                e.key.invocation.origin,
                RuleOrigin::SupportPreparation { .. }
            ))
            .count(),
        2
    );
}

#[test]
fn runtime_inactive_output_is_not_false_membership_and_sibling_remains_known() {
    let mut f = source_fixture();
    f.build
        .gems
        .iter_mut()
        .find(|gem| gem.id == occurrence(28))
        .unwrap()
        .parameters[0]
        .value = ParameterValue::Boolean(false);
    let plan = compile(&f);
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let effects = delivery::evaluated(&report);
    assert_eq!(
        type_value(effects, &target(30, "first"), "prepared-duration"),
        &EffectValue::Inactive
    );
    assert_eq!(
        type_value(effects, &target(31, "first"), "prepared-duration"),
        &boolean(true)
    );
}

#[test]
fn missing_initial_membership_never_becomes_false_even_for_an_empty_support_selection() {
    let mut f = empty_target_fixture();
    f.owner_mut(&subject(def::<SkillDefinition>("ability")))
        .programs
        .members
        .iter_mut()
        .find(|program| program.id == key("target-facts"))
        .unwrap()
        .effects
        .retain(|effect| effect.id != key("duration"));
    let plan = compile(&f);
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert_eq!(
        report.outcome,
        SupportEffectsOutcome::Unavailable {
            cause: EffectValue::Unresolved {
                reason: PlanGapReason::MissingProducer,
                read: None
            },
            input: Some(Box::new(PlanValueKey::Stat {
                entity: ConcreteEntity::Skill(Box::new(target(30, "first"))),
                stat: def("duration"),
            })),
        }
    );
}

#[test]
fn direct_and_inherited_contexts_for_one_skill_cannot_publish_a_shared_stat() {
    let mut f = source_fixture();
    f.build.gems.push(GemInstance {
        id: occurrence(57),
        definition: def("support"),
        parameters: vec![],
        level: 1,
        quality: None,
    });
    f.build.supports.push(SupportAssignment {
        id: occurrence(67),
        support: occurrence(57),
        target: SkillTarget::Authored(occurrence(30)),
        enabled: true,
    });
    f.build
        .support_origins
        .as_mut()
        .unwrap()
        .push(SupportOriginSequence {
            target: SkillTarget::Authored(occurrence(30)),
            origins: vec![SupportOrigin::Assignment(occurrence(67))],
        });
    let (args, outputs) = inputs_with(
        &f,
        |_| {},
        |_| {},
        |_| {},
        |receiving| {
            receiving.targets.push(SupportTargetReceivingRoles {
                owner: SupportTargetDefinition::Gem(def("summoner")),
                roles: DeclaredSet::complete(vec![SupportReceivingRoleBinding {
                    role: key("action"),
                    endpoints: DeclaredSet::complete(vec![SupportReceiverEndpoint::Action {
                        path: vec![
                            fixture::child_grant(),
                            DeclaredSlot {
                                declaration: SlotOwnerDefId::Actor(def("family")),
                                slot: def("first"),
                            },
                        ],
                        output: delivery::output(),
                        selection: SupportActionSelection::AllDeclared,
                        admission: SupportAdmissionContext::ReceivingSkill {
                            summoner_path: None,
                        },
                    }]),
                }]),
            });
        },
    );
    let error = OwnedSupportEffectPlan::compile_with_outputs(
        args,
        outputs,
        PlanLimits::default(),
        SupportPreparationLimits::default(),
    )
    .err()
    .expect("one target cannot receive competing context authority");
    assert!(
        matches!(error, PlanError::Invalid(message) if message.contains("competing preparation contexts"))
    );
}

#[test]
fn inherited_outputs_keep_child_types_parent_selection_and_explicit_summoner_context() {
    let mut f = source_fixture();
    let parent = SkillTarget::Authored(occurrence(30));
    let child = target(30, "first");
    // The parent's own frozen facts and its support scalar preparation differ
    // deliberately from the generated receiving skill's initial facts.
    let mut parent_facts = f
        .owner_mut(&subject(def::<SkillDefinition>("ability")))
        .programs
        .members
        .iter()
        .find(|p| p.id == key("target-facts"))
        .unwrap()
        .clone();
    parent_facts.id = key("parent-target-facts");
    parent_facts.reads.clear();
    parent_facts
        .nodes
        .iter_mut()
        .find(|n| n.id == key("base"))
        .unwrap()
        .expression = RuleExpression::Literal {
        value: delivery::integer(77),
    };
    f.owner_mut(&fixture::summoner_owner())
        .programs
        .members
        .push(parent_facts);
    f.owner_mut(&subject(def::<SkillDefinition>("ability")))
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
    for assignment in &mut f.build.supports {
        if assignment.target == child {
            assignment.target = parent.clone();
        }
    }
    for sequence in f.build.support_origins.as_mut().unwrap() {
        if sequence.target == child {
            sequence.target = parent.clone();
        }
    }
    let (args, outputs) = inputs_with(
        &f,
        |_| {},
        |_| {},
        |_| {},
        |receiving| {
            receiving.targets.push(SupportTargetReceivingRoles {
                owner: SupportTargetDefinition::Gem(def("summoner")),
                roles: DeclaredSet::complete(vec![SupportReceivingRoleBinding {
                    role: key("action"),
                    endpoints: DeclaredSet::complete(vec![SupportReceiverEndpoint::Action {
                        path: vec![
                            fixture::child_grant(),
                            DeclaredSlot {
                                declaration: SlotOwnerDefId::Actor(def("family")),
                                slot: def("first"),
                            },
                        ],
                        output: delivery::output(),
                        selection: SupportActionSelection::AllDeclared,
                        admission: SupportAdmissionContext::ReceivingSkill {
                            summoner_path: Some(vec![]),
                        },
                    }]),
                }]),
            });
        },
    );
    let plan = OwnedSupportEffectPlan::compile_with_outputs(
        args,
        outputs,
        PlanLimits::default(),
        SupportPreparationLimits::default(),
    )
    .unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let effects = delivery::evaluated(&report);
    assert_eq!(type_value(effects, &parent, "spell"), &boolean(true));
    assert_eq!(type_value(effects, &child, "spell"), &boolean(false));
    assert_eq!(type_value(effects, &child, "duration"), &boolean(false));
    assert_eq!(
        type_value(effects, &child, "prepared-spell"),
        &boolean(false)
    );
    assert_eq!(
        type_value(effects, &child, "prepared-duration"),
        &boolean(true)
    );
    let exports: Vec<_> = effects
        .effects
        .iter()
        .filter_map(|effect| {
            let RuleOrigin::SupportPreparation { context } = &effect.key.invocation.origin else {
                return None;
            };
            (context.target == child).then_some(context)
        })
        .collect();
    assert_eq!(exports.len(), 2);
    for context in exports {
        assert_eq!(context.selection, parent);
        assert_eq!(context.summoner, Some(parent.clone()));
    }
    let action = action(30, "first");
    let application = effects
        .effects
        .iter()
        .find(|effect| {
            effect.key.invocation.program == key("action-deliver")
                && delivery::application(effect).is_some_and(|key| {
                    key.receiver == SupportReceiverKey::Action(Box::new(action.clone()))
                })
        })
        .unwrap();
    let application_key = delivery::application(application).unwrap();
    assert_eq!(application_key.prepared.target, parent);
    assert_eq!(
        application_key.prepared.origin,
        SupportOrigin::Assignment(occurrence(61))
    );
    assert_eq!(
        application.value,
        delivery::known(81),
        "raw level2 plus parent-computed effective79; never recompute from child base12"
    );
    let downstream = PlanValueKey::Stat {
        entity: ConcreteEntity::Action(Box::new(action)),
        stat: def("prepared-result"),
    };
    assert_eq!(
        effects
            .values
            .iter()
            .find(|row| row.key == downstream)
            .unwrap()
            .value,
        delivery::known(81)
    );
}

#[test]
fn output_packages_are_identity_bound_and_reports_are_stable_under_reuse_and_parallel_work() {
    let f = source_fixture();
    let (args, old_outputs) = inputs(&f);
    let a = Arc::new(
        OwnedSupportEffectPlan::compile_with_outputs(
            args,
            old_outputs.clone(),
            PlanLimits::default(),
            SupportPreparationLimits::default(),
        )
        .unwrap(),
    );
    let mut changed = source_fixture();
    changed.schema.release = key("changed-schema");
    let (args, _) = inputs(&changed);
    assert!(
        OwnedSupportEffectPlan::compile_with_outputs(
            args,
            old_outputs,
            PlanLimits::default(),
            SupportPreparationLimits::default()
        )
        .is_err()
    );
    let mut scratch = a.new_scratch();
    let allowance = PlanLimits::default().max_work;
    let mut work = allowance;
    let expected = a.evaluate_with_budget(&mut scratch, &mut work).unwrap();
    let mut short = allowance - work - 1;
    assert!(matches!(
        a.evaluate_with_budget(&mut scratch, &mut short),
        Err(PlanError::Limit("work"))
    ));
    assert_eq!(short, 0);
    assert_eq!(expected, a.evaluate(&mut scratch).unwrap());
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let parallel: Vec<_> = pool.install(|| {
        (0..16)
            .into_par_iter()
            .map_init(
                || a.new_scratch(),
                |scratch, _| {
                    let mut remaining = allowance;
                    (
                        a.evaluate_with_budget(scratch, &mut remaining).unwrap(),
                        remaining,
                    )
                },
            )
            .collect()
    });
    assert!(
        parallel
            .iter()
            .all(|(report, remaining)| *report == expected && *remaining == work)
    );
}
