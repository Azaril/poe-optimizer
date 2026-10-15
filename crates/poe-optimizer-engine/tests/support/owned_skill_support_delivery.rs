//! Skill recipients share the existing native graph, without Action demand.
use super::*;
use poe_optimizer_core::{owned_schema::*, owned_stages::*, owned_support_receiving::*};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, RuleLimits};

fn world() -> Fixture {
    let mut f = source_fixture();
    for definition in &mut f.schema.definitions {
        if let DefinitionDescriptor::Stat(DefinitionEntry {
            id,
            schema: SchemaState::Known(schema),
        }) = definition
            && *id == def::<StatDefinition>("support-total")
        {
            schema.targets.push(RuleEntityKind::Skill);
        }
    }
    f.owner_mut(&subject(def::<SkillDefinition>("ability")))
        .programs
        .members
        .push(final_program("skill-final", RuleEntityKind::Skill));
    f
}
fn inputs_for(
    f: &Fixture,
    edit_rules: impl FnOnce(&mut RulePackageInput),
    edit_stages: impl FnOnce(&mut EvaluationStagesInput),
    edit_receiving: impl FnOnce(&mut SupportReceivingInput),
) -> Inputs {
    inputs_with_stage_packages(
        f,
        |rules| {
            let programs = &mut rules
                .owners
                .iter_mut()
                .find(|o| o.owner == support_owner())
                .unwrap()
                .programs
                .members;
            programs.retain(|p| {
                ![
                    key("actor-app"),
                    key("action-app"),
                    key("actor-deliver"),
                    key("action-deliver"),
                ]
                .contains(&p.id)
            });
            programs.extend([
                support_program("skill-app", RuleEntityKind::Skill, Some(true)),
                support_program("skill-deliver", RuleEntityKind::Skill, None),
            ]);
            edit_rules(rules);
        },
        |_| {},
        |stages| {
            stages.programs.members.retain(|p| {
                ![
                    key("actor-app"),
                    key("action-app"),
                    key("actor-deliver"),
                    key("action-deliver"),
                ]
                .contains(&p.program)
            });
            for p in &mut stages.programs.members {
                if p.program == key("skill-final") {
                    p.stage = key("final");
                }
            }
            for (program, stage) in [("skill-app", "apply"), ("skill-deliver", "deliver")] {
                stages.programs.members.push(StagedRuleProgram {
                    owner: support_owner(),
                    program: key(program),
                    stage: key(stage),
                });
            }
            edit_stages(stages);
        },
        |receiving| {
            receiving.roles = vec![SupportReceivingRole {
                id: key("skill"),
                kind: SupportReceiverKind::Skill,
            }];
            receiving.targets[0].roles.members = vec![SupportReceivingRoleBinding {
                role: key("skill"),
                endpoints: DeclaredSet::complete(vec![SupportReceiverEndpoint::Skill {
                    path: vec![],
                    admission: SupportAdmissionContext::AssignedSkill,
                }]),
            }];
            receiving.supports[0].receivers.members = vec![SupportRolePrograms {
                preparation: None,
                role: key("skill"),
                applicability: key("skill-app"),
                delivery: vec![key("skill-deliver")],
            }];
            edit_receiving(receiving);
        },
    )
}
fn plan(f: &Fixture) -> Plan {
    Plan::compile(
        inputs_for(f, |_| {}, |_| {}, |_| {}),
        Default::default(),
        Default::default(),
    )
    .unwrap()
}
#[test]
fn skill_support_delivery_keeps_sources_and_exact_recipients_without_actions() {
    let plan = plan(&world());
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let effects = evaluated(&report);
    assert!(
        !effects
            .effects
            .iter()
            .any(|e| matches!(e.key.invocation.entity, ConcreteEntity::Action(_)))
    );
    let applications: BTreeSet<_> = effects.effects.iter().filter_map(application).collect();
    assert_eq!(applications.len(), 3);
    for a in applications {
        let SupportReceiverKey::Skill(skill) = &a.receiver else {
            panic!("exact Skill receiver")
        };
        assert_eq!(skill.as_ref(), &a.prepared.target);
        let expected = if **skill == target(31, "first") {
            25
        } else {
            16
        };
        let root = if **skill == target(31, "first") {
            31
        } else {
            30
        };
        let origin = if **skill == target(30, "first") {
            61
        } else if root == 30 {
            63
        } else {
            65
        };
        assert_eq!(
            a.prepared.origin,
            SupportOrigin::Assignment(occurrence(origin))
        );
        let delivered: Vec<_> = effects
            .effects
            .iter()
            .filter(|e| {
                application(e) == Some(a) && e.key.invocation.program == key("skill-deliver")
            })
            .collect();
        assert_eq!(delivered.len(), 1);
        assert_eq!(
            delivered[0].key.invocation.entity,
            ConcreteEntity::Skill(skill.clone())
        );
        assert_eq!(delivered[0].value, known(expected));
        assert_eq!(
            final_value(effects, &ConcreteEntity::Skill(skill.clone())),
            &known(expected)
        );
    }
    // No Actor contribution or Action selection was created by the Skill endpoint.
    for root in [30, 31] {
        assert_eq!(
            final_value(effects, &ConcreteEntity::Actor(child_actor(root))),
            &known(0)
        );
    }
}
#[test]
fn skill_support_missing_partial_or_unavailable_never_becomes_known_delivery() {
    for mode in 0..4 {
        let args = inputs_for(
            &world(),
            |_| {},
            |_| {},
            |r| match mode {
                0 => r.supports.clear(),
                1 => r.targets.clear(),
                2 => {
                    r.targets[0].roles.closure = partial(subject(def::<SkillDefinition>("ability")))
                }
                _ => {
                    r.targets[0].roles.members[0].endpoints.closure =
                        partial(subject(def::<SkillDefinition>("ability")))
                }
            },
        );
        let p = Plan::compile(args, Default::default(), Default::default()).unwrap();
        let report = p.evaluate(&mut p.new_scratch()).unwrap();
        assert!(
            matches!(report.outcome, SupportEffectsOutcome::Unavailable { .. }),
            "mode {mode}: {report:?}"
        );
        assert!(
            report
                .gaps
                .iter()
                .any(|g| g.reason == PlanGapReason::PartialReceivers)
        );
    }
    let mut f = world();
    f.build
        .skills
        .iter_mut()
        .find(|s| s.id == occurrence(30))
        .unwrap()
        .enabled = false;
    let p = plan(&f);
    let report = p.evaluate(&mut p.new_scratch()).unwrap();
    let effects = evaluated(&report);
    let apps: BTreeSet<_> = effects.effects.iter().filter_map(application).collect();
    assert_eq!(apps.len(), 1);
    assert!(
        apps.iter()
            .all(|a| a.receiver == SupportReceiverKey::Skill(Box::new(target(31, "first"))))
    );
    assert_eq!(
        final_value(
            effects,
            &ConcreteEntity::Skill(Box::new(target(31, "first")))
        ),
        &known(25)
    );
}
#[test]
fn skill_support_fresh_reused_parallel_and_changed_source_results_agree() {
    let a = plan(&world());
    let first = a.evaluate(&mut a.new_scratch()).unwrap();
    evaluated(&first);
    let mut f = world();
    f.build
        .gems
        .iter_mut()
        .find(|g| g.id == occurrence(29))
        .unwrap()
        .level = 30;
    let b = plan(&f);
    let mut scratch = a.new_scratch();
    assert_eq!(a.evaluate(&mut scratch).unwrap(), first);
    let changed = b.evaluate(&mut scratch).unwrap();
    assert_eq!(changed, b.evaluate(&mut b.new_scratch()).unwrap());
    assert_eq!(
        final_value(
            evaluated(&changed),
            &ConcreteEntity::Skill(Box::new(target(31, "first")))
        ),
        &known(35)
    );
    assert_ne!(first.identity, changed.identity);
    assert_eq!(a.evaluate(&mut scratch).unwrap(), first);
    let reports: Vec<_> = (0..16)
        .into_par_iter()
        .map(|_| a.evaluate(&mut a.new_scratch()).unwrap())
        .collect();
    assert!(reports.iter().all(|r| r == &first));
}
#[test]
fn skill_applicability_requires_support_owner_and_does_not_grant_actor_access() {
    let f = world();
    let definitions =
        OwnedDefinitionSchemaPackage::new(f.schema.clone(), Default::default()).unwrap();
    for mode in 0..4 {
        let mut rules = fixture::raw_rules(&f, &definitions, OWNED_RULE_OPERATIONS_V13);
        let mut program = support_program("skill-app-control", RuleEntityKind::Skill, Some(true));
        let owner = match mode {
            0 => subject(def::<SkillDefinition>("ability")),
            3 => subject(def::<GemDefinition>("summoner")),
            _ => support_owner(),
        };
        if mode == 1 {
            program.reads.push(fixture::read(
                "actor",
                RuleReadSource::Stat {
                    entity: RuleEntity::Actor,
                    stat: def("support-total"),
                },
            ));
        }
        if mode == 2 {
            program.effects.push(effect(
                "actor-write",
                RuleEffectKind::Contribute {
                    entity: RuleEntity::Actor,
                    stat: def("support-total"),
                    contribution: ContributionKind::Add,
                    value: key("one"),
                },
            ));
            program.nodes.push(fixture::literal("one", 1));
        }
        rules
            .owners
            .iter_mut()
            .find(|o| o.owner == owner)
            .unwrap()
            .programs
            .members
            .push(program);
        let error = CompiledRulePackage::compile(&rules, &definitions, RuleLimits::default())
            .expect_err("no non-support applicability or Skill-to-Actor authority");
        let message = error.to_string();
        assert!(
            message.contains(if mode == 0 || mode == 3 {
                "SupportAssignment Gem"
            } else {
                "relative Actor"
            }),
            "{message}"
        );
    }
}
