//! Synthetic owned definitions prove inherited delivery boundaries end to end.
//! These tests make no source-data or whole-game numerical parity claim.
#[allow(dead_code)]
use crate::engine_fixtures::delivery as delivery_fixture;
use delivery_fixture::*;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_routing::*, owned_rules::*, owned_schema::*,
    owned_stages::*, owned_support_receiving::*, owned_supports::*,
};
use poe_optimizer_engine::{owned_plan::*, owned_supports::*};
use std::collections::BTreeSet;

fn child_grant() -> DeclaredSlot<GrantSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(def("ability")),
        slot: def("nested-child"),
    }
}
fn child_supply() -> DeclaredSlot<SkillGrantSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(def("ability")),
        slot: def("nested-skill"),
    }
}
fn child_output() -> DeclaredSlot<ActionOutputDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(def("nested-ability")),
        slot: def("nested-output"),
    }
}
fn empty<T>() -> DeclaredSet<T> {
    DeclaredSet::complete(vec![])
}
fn ports() -> DeclaredSlots {
    DeclaredSlots {
        parameters: empty(),
        choices: empty(),
        grants: empty(),
        actors: empty(),
        skill_grants: empty(),
        outputs: empty(),
        sockets: empty(),
    }
}
fn flag(program: &mut RuleProgram, name: &str, value: bool) {
    program
        .nodes
        .iter_mut()
        .find(|n| n.id == key(name))
        .unwrap()
        .expression = RuleExpression::Literal {
        value: ParameterValue::Boolean(value),
    };
}
fn parent_facts(f: &mut Fixture) -> &mut RuleProgram {
    f.owner_mut(&subject(def::<SkillDefinition>("ability")))
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == key("target-facts"))
        .unwrap()
}

fn add_child(f: &mut Fixture, has_gem: bool) {
    let mut child_program = parent_facts(f).clone();
    child_program.reads.clear();
    child_program
        .nodes
        .iter_mut()
        .find(|n| n.id == key("base"))
        .unwrap()
        .expression = RuleExpression::Literal {
        value: integer(777),
    };
    flag(&mut child_program, "spell", false);
    flag(&mut child_program, "has-gem", has_gem);
    // Missing child summoner fields are deliberate: receiving metadata supplies
    // that relationship. A receiver must not read unused explicit target fields.
    child_program
        .effects
        .retain(|e| ![key("summoner-present"), key("summoner-minion-present")].contains(&e.id));
    for definition in &mut f.schema.definitions {
        if let DefinitionDescriptor::Skill(DefinitionEntry {
            id,
            schema: SchemaState::Known(schema),
        }) = definition
            && *id == def::<SkillDefinition>("ability")
        {
            schema.declarations.grants.members.push(child_grant());
            schema
                .declarations
                .skill_grants
                .members
                .push(child_supply());
        }
    }
    for slot in &mut f.schema.slots {
        if let SlotDescriptor::Actor(DefinitionEntry {
            id,
            schema: SchemaState::Known(schema),
        }) = slot
            && *id == fixture::child_slot()
        {
            schema.skills.members.push(def("nested-ability"));
        }
    }
    let mut child_ports = ports();
    child_ports.outputs.members.push(child_output());
    f.schema
        .definitions
        .push(DefinitionDescriptor::Skill(DefinitionEntry {
            id: def("nested-ability"),
            schema: SchemaState::Known(SkillSchema {
                directly_selectable: false,
                declarations: child_ports,
            }),
        }));
    f.schema.slots.extend([
        SlotDescriptor::SkillGrant(DefinitionEntry {
            id: child_supply(),
            schema: SchemaState::Known(SkillGrantSlotSchema {
                preset_inputs: None,
                skill: def("nested-ability"),
                outputs: DeclaredSet::complete(vec![child_output()]),
            }),
        }),
        SlotDescriptor::Grant(DefinitionEntry {
            id: child_grant(),
            schema: SchemaState::Known(GrantSlotSchema {
                provider_roles: vec![ProviderRole::SkillUse],
                target: GrantTarget::Skill(child_supply()),
            }),
        }),
        SlotDescriptor::ActionOutput(DefinitionEntry {
            id: child_output(),
            schema: SchemaState::Known(ActionOutputSchema {
                actor_role: DeclaredActorRole::ProviderActor,
                parts: DeclaredSet::complete(vec![def("part"), def("part-two")]),
                modes: DeclaredSet::complete(vec![def("mode")]),
                stat_sets: DeclaredSet::complete(vec![def("set")]),
                choices: empty(),
            }),
        }),
    ]);
    f.owner_mut(&subject(def::<SkillDefinition>("ability")))
        .programs
        .members
        .push(RuleProgram {
            id: key("nested-supply"),
            context: RuleEntityKind::Skill,
            reads: vec![],
            nodes: vec![fixture::bool_node("enabled", true)],
            effects: vec![effect(
                "activate",
                RuleEffectKind::ActivateGrant {
                    slot: child_grant(),
                    enabled: key("enabled"),
                },
            )],
        });
    f.owners.extend([
        DefinitionRules {
            owner: subject(def::<SkillDefinition>("nested-ability")),
            programs: DeclaredSet::complete(vec![child_program]),
        },
        DefinitionRules {
            owner: SchemaSubject::Slot(SlotAddress::ActionOutput(child_output())),
            programs: DeclaredSet::complete(vec![final_program(
                "action-final",
                RuleEntityKind::Action,
            )]),
        },
    ]);
    f.routes.push(ActionOutputRoutes {
        output: child_output(),
        routes: empty(),
        source_selectors: Some(empty()),
    });
}

fn child_receivers(input: &mut SupportReceivingInput, with_summoner: bool) {
    let action = input.targets[0]
        .roles
        .members
        .iter_mut()
        .find(|r| r.role == key("action"))
        .unwrap();
    action.endpoints = DeclaredSet::complete(vec![SupportReceiverEndpoint::Action {
        path: vec![child_grant()],
        output: child_output(),
        selection: SupportActionSelection::AllDeclared,
        admission: SupportAdmissionContext::ReceivingSkill {
            summoner_path: with_summoner.then(Vec::new),
        },
    }]);
}
fn compile_args(args: Inputs) -> Plan {
    Plan::compile(
        args,
        PlanLimits::default(),
        SupportPreparationLimits::default(),
    )
    .unwrap()
}
fn check_child_action(action: &ActionSelection) {
    assert_eq!(action.action.output, child_output());
    assert_eq!(action.action.provider.grant_path.len(), 3);
    assert_eq!(
        action.action.provider.grant_path.last(),
        Some(&child_grant())
    );
}
fn applicability(report: &OwnedEffectsReport, application: &SupportApplicationKey) -> bool {
    let value = &report
        .values
        .iter()
        .find(|v| {
            v.key
                == PlanValueKey::SupportApplicability {
                    application: Box::new(application.clone()),
                }
        })
        .unwrap()
        .value;
    let EffectValue::Known {
        value: ParameterValue::Boolean(value),
    } = value
    else {
        panic!("known applicability")
    };
    *value
}

fn missing_child_flags(f: &mut Fixture) {
    let facts = f
        .owner_mut(&subject(def::<SkillDefinition>("nested-ability")))
        .programs
        .members
        .iter_mut()
        .find(|program| program.id == key("target-facts"))
        .unwrap();
    for name in ["has-gem", "from-item", "is-player"] {
        let before = facts.effects.len();
        facts.effects.retain(|effect| effect.id != key(name));
        assert_eq!(facts.effects.len() + 1, before);
    }
}

fn require_flag(preparation: &mut SupportPreparationInput, name: &str) {
    let SchemaState::Known(support) = &mut preparation.supports[0].preparation else {
        unreachable!()
    };
    match name {
        "has-gem" => support.gems_only = true,
        "from-item" => support.from_item = true,
        "is-player" => support.is_trigger = true,
        _ => unreachable!(),
    }
}

fn assert_child_delivery(report: &SupportEffectsReport, child_applies: bool) {
    let effects = evaluated(report);
    let apps: BTreeSet<_> = effects.effects.iter().filter_map(application).collect();
    assert_eq!(apps.len(), 9);
    for app in apps {
        assert!(
            [61, 63, 65]
                .into_iter()
                .any(|id| app.prepared.origin == SupportOrigin::Assignment(occurrence(id)))
        );
        assert_eq!(app.prepared.position, 0);
        let amount = if app.prepared.target == target(31, "first") {
            25
        } else {
            16
        };
        let delivery = effects
            .effects
            .iter()
            .find(|effect| {
                application(effect) == Some(app)
                    && effect.key.invocation.program.as_str().ends_with("deliver")
            })
            .unwrap();
        match &app.receiver {
            SupportReceiverKey::Actor(_) => {
                assert!(applicability(effects, app));
                assert_eq!(delivery.value, known(amount));
            }
            SupportReceiverKey::Skill(_) => panic!("fixture declares Actor/Action endpoints only"),
            SupportReceiverKey::Action(action) => {
                check_child_action(action);
                assert_eq!(applicability(effects, app), child_applies);
                assert_eq!(
                    delivery.value,
                    if child_applies {
                        known(amount)
                    } else {
                        EffectValue::Inactive
                    }
                );
                assert_eq!(
                    final_value(effects, &ConcreteEntity::Action(action.clone())),
                    &known(if child_applies { amount } else { 0 })
                );
            }
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
fn unused_missing_child_flags_preserve_inherited_admission_and_origin_scalars() {
    let mut f = source_fixture();
    add_child(&mut f, false);
    missing_child_flags(&mut f);
    let plan = compile_args(inputs(&f, |receiving| child_receivers(receiving, true)));
    let mut scratch = plan.new_scratch();
    let expected = plan.evaluate(&mut scratch).unwrap();
    assert_child_delivery(&expected, true);
    assert_eq!(plan.evaluate(&mut scratch).unwrap(), expected);
    assert_eq!(plan.evaluate(&mut plan.new_scratch()).unwrap(), expected);
}

#[test]
fn consumed_missing_child_flag_preserves_exact_target_stat_and_cause() {
    let mut f = source_fixture();
    flag(parent_facts(&mut f), "has-gem", true);
    flag(parent_facts(&mut f), "is-player", true);
    add_child(&mut f, true);
    missing_child_flags(&mut f);
    // Keep one assigned target so the exact failing child is independent of
    // traversal order among unrelated generated skills.
    f.build
        .supports
        .retain(|support| support.target == target(30, "first"));
    f.build
        .authored_support_order
        .as_mut()
        .unwrap()
        .retain(|order| order.target == target(30, "first"));
    let baseline = compile_args(inputs(&f, |receiving| child_receivers(receiving, true)));
    let mut scratch = baseline.new_scratch();
    let expected = baseline.evaluate(&mut scratch).unwrap();
    assert_eq!(
        evaluated(&expected)
            .effects
            .iter()
            .filter_map(application)
            .collect::<BTreeSet<_>>()
            .len(),
        3
    );
    let SkillTarget::Generated(parent) = target(30, "first") else {
        unreachable!()
    };
    let mut provider = parent.provider;
    provider.grant_path.push(DeclaredSlot {
        declaration: SlotOwnerDefId::Actor(def("family")),
        slot: def("first"),
    });
    let child = SkillTarget::Generated(Box::new(GeneratedSkillKey {
        provider,
        slot: child_supply(),
    }));
    assert_ne!(child, target(30, "first"));
    for name in ["has-gem", "from-item", "is-player"] {
        let plan = compile_args(inputs_with_preparation(
            &f,
            |preparation| require_flag(preparation, name),
            |receiving| child_receivers(receiving, true),
        ));
        let report = plan.evaluate(&mut scratch).unwrap();
        assert_eq!(
            report.outcome,
            SupportEffectsOutcome::Unavailable {
                cause: EffectValue::Unresolved {
                    reason: PlanGapReason::MissingProducer,
                    read: None,
                },
                input: Some(Box::new(PlanValueKey::Stat {
                    entity: ConcreteEntity::Skill(Box::new(child.clone())),
                    stat: def(name),
                })),
            },
            "required child flag {name}"
        );
        assert_eq!(report, plan.evaluate(&mut plan.new_scratch()).unwrap());
        assert_eq!(baseline.evaluate(&mut scratch).unwrap(), expected);
    }
}

#[test]
fn known_child_type_rejection_with_missing_conditional_flags_keeps_delivery_inactive() {
    let mut f = source_fixture();
    flag(parent_facts(&mut f), "has-gem", true);
    flag(parent_facts(&mut f), "is-player", true);
    add_child(&mut f, true);
    missing_child_flags(&mut f);
    for name in ["has-gem", "from-item", "is-player"] {
        let plan = compile_args(inputs_with_preparation(
            &f,
            |preparation| require_flag(preparation, name),
            // Without explicit summoner inheritance the child's known false
            // spell type conclusively rejects each support despite its flag.
            |receiving| child_receivers(receiving, false),
        ));
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert_child_delivery(&report, false);
    }
}

#[test]
fn rejected_parent_does_not_gate_child_or_recompute_origin_scalars() {
    let mut f = source_fixture();
    add_child(&mut f, true);
    let args = inputs_with_preparation(
        &f,
        |preparation| {
            let SchemaState::Known(support) = &mut preparation.supports[0].preparation else {
                unreachable!()
            };
            support.gems_only = true;
        },
        |receiving| child_receivers(receiving, true),
    );
    let plan = compile_args(args);
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let effects = evaluated(&report);
    let apps: BTreeSet<_> = effects.effects.iter().filter_map(application).collect();
    assert_eq!(apps.len(), 9);
    for app in apps {
        assert!(
            [61, 63, 65]
                .into_iter()
                .any(|id| app.prepared.origin == SupportOrigin::Assignment(occurrence(id)))
        );
        assert_eq!(app.prepared.position, 0);
        match &app.receiver {
            SupportReceiverKey::Actor(_) => assert!(!applicability(effects, app)),
            SupportReceiverKey::Skill(_) => panic!("fixture declares Actor/Action endpoints only"),
            SupportReceiverKey::Action(action) => {
                check_child_action(action);
                assert!(applicability(effects, app));
                let expected = if app.prepared.target == target(31, "first") {
                    25
                } else {
                    16
                };
                assert_eq!(
                    final_value(effects, &ConcreteEntity::Action(action.clone())),
                    &known(expected)
                );
                // The child skill-base is 777, but raw + effective support level
                // still comes from the original assignment's parent context.
                let delivery = effects
                    .effects
                    .iter()
                    .find(|e| {
                        application(e) == Some(app)
                            && e.key.invocation.program == key("action-deliver")
                    })
                    .unwrap();
                assert_eq!(delivery.value, known(expected));
            }
        }
    }
    assert_eq!(
        final_value(effects, &ConcreteEntity::Actor(child_actor(30))),
        &known(0)
    );
    assert_eq!(
        final_value(effects, &ConcreteEntity::Actor(child_actor(31))),
        &known(0)
    );
}

fn distinct_supports(f: &mut Fixture, names: &[&str]) {
    let support_schema = f
        .schema
        .definitions
        .iter()
        .find_map(|d| {
            if let DefinitionDescriptor::Gem(entry) = d
                && entry.id == def::<GemDefinition>("support")
            {
                Some(entry.schema.clone())
            } else {
                None
            }
        })
        .unwrap();
    let origin_programs = f.owner_mut(&support_owner()).programs.clone();
    for name in names {
        f.schema
            .definitions
            .push(DefinitionDescriptor::Gem(DefinitionEntry {
                id: def(name),
                schema: support_schema.clone(),
            }));
        f.owners.push(DefinitionRules {
            owner: subject(def::<GemDefinition>(name)),
            programs: origin_programs.clone(),
        });
    }
    f.build.supports.retain(|s| s.target == target(30, "first"));
    f.build
        .authored_support_order
        .as_mut()
        .unwrap()
        .retain(|s| s.target == target(30, "first"));
    for (index, name) in names.iter().enumerate() {
        let (gem, assignment) = if index < 2 {
            (50 + index as u64, 60 + index as u64)
        } else {
            (56, 66)
        };
        if index < 2 {
            f.build
                .gems
                .iter_mut()
                .find(|g| g.id == occurrence(gem))
                .unwrap()
                .definition = def(name);
        } else {
            f.build.gems.push(GemInstance {
                id: occurrence(gem),
                definition: def(name),
                parameters: vec![],
                level: 3,
                quality: None,
            });
            f.build.supports.push(SupportAssignment {
                id: occurrence(assignment),
                support: occurrence(gem),
                target: target(30, "first"),
                enabled: true,
            });
            f.build.authored_support_order.as_mut().unwrap()[0]
                .assignments
                .push(occurrence(assignment));
        }
    }
}
fn multi_inputs(
    f: &Fixture,
    names: &[&str],
    edit_preparation: impl FnOnce(&mut SupportPreparationInput),
    with_summoner: bool,
) -> Inputs {
    inputs_with_stage_packages(
        f,
        |rules| {
            let programs: Vec<_> = rules
                .owners
                .iter()
                .find(|r| r.owner == support_owner())
                .unwrap()
                .programs
                .members
                .iter()
                .filter(|p| {
                    [
                        key("actor-app"),
                        key("action-app"),
                        key("actor-deliver"),
                        key("action-deliver"),
                    ]
                    .contains(&p.id)
                })
                .cloned()
                .collect();
            for name in names {
                rules
                    .owners
                    .iter_mut()
                    .find(|r| r.owner == subject(def::<GemDefinition>(name)))
                    .unwrap()
                    .programs
                    .members
                    .extend(programs.clone());
            }
        },
        |preparation| {
            let template = preparation.supports[0].preparation.clone();
            preparation.effects = names.iter().map(|name| key(name)).collect();
            preparation.supports = names
                .iter()
                .map(|name| {
                    let mut preparation = template.clone();
                    let SchemaState::Known(support) = &mut preparation else {
                        unreachable!()
                    };
                    support.effect = key(name);
                    SupportPreparationEntry {
                        gem: def(name),
                        preparation,
                    }
                })
                .collect();
            edit_preparation(preparation);
        },
        |stages| {
            for name in names {
                for (program, stage) in [
                    ("actor-app", "apply"),
                    ("action-app", "apply"),
                    ("actor-deliver", "deliver"),
                    ("action-deliver", "deliver"),
                ] {
                    stages.programs.members.push(StagedRuleProgram {
                        owner: subject(def::<GemDefinition>(name)),
                        program: key(program),
                        stage: key(stage),
                    });
                }
            }
        },
        |receiving| {
            child_receivers(receiving, with_summoner);
            let template = receiving.supports[0].receivers.clone();
            receiving.supports = names
                .iter()
                .map(|name| SupportReceivingEntry {
                    gem: def(name),
                    receivers: template.clone(),
                })
                .collect();
        },
    )
}

#[test]
fn replacement_origin_controls_missing_flag_diagnostics_and_retained_delivery() {
    for missing_from_item in [true, false] {
        let mut f = source_fixture();
        add_child(&mut f, false);
        let names = ["policy-a", "policy-b"];
        distinct_supports(&mut f, &names);
        assert_eq!(
            f.build.authored_support_order.as_ref().unwrap()[0].assignments,
            vec![occurrence(60), occurrence(61)]
        );
        let facts = parent_facts(&mut f);
        for name in ["has-gem", "from-item"] {
            if name == "from-item" && !missing_from_item {
                continue;
            }
            let before = facts.effects.len();
            facts.effects.retain(|effect| effect.id != key(name));
            assert_eq!(facts.effects.len() + 1, before);
        }
        let plan = compile_args(multi_inputs(
            &f,
            &names,
            |preparation| {
                for entry in &mut preparation.supports {
                    let SchemaState::Known(support) = &mut entry.preparation else {
                        unreachable!()
                    };
                    support.gems_only = entry.gem == def::<GemDefinition>("policy-a");
                    support.from_item = entry.gem == def::<GemDefinition>("policy-b");
                    support.plus_version_of =
                        (entry.gem == def::<GemDefinition>("policy-b")).then(|| key("policy-a"));
                }
            },
            true,
        ));
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        if missing_from_item {
            // B is A's declared plus version: ordered origin 1 replaces origin
            // 0 at retained position 0. Only B's consumed flag explains the gap.
            assert_eq!(
                report.outcome,
                SupportEffectsOutcome::Unavailable {
                    cause: EffectValue::Unresolved {
                        reason: PlanGapReason::MissingProducer,
                        read: None,
                    },
                    input: Some(Box::new(PlanValueKey::Stat {
                        entity: ConcreteEntity::Skill(Box::new(target(30, "first"))),
                        stat: def("from-item"),
                    })),
                }
            );
            continue;
        }

        // A's has-gem remains missing. Restoring only B's from-item producer
        // must allow the same retained origin to reach every declared receiver.
        let effects = evaluated(&report);
        let apps: BTreeSet<_> = effects.effects.iter().filter_map(application).collect();
        assert_eq!(apps.len(), 3);
        assert_eq!(
            apps.iter()
                .filter(|app| matches!(app.receiver, SupportReceiverKey::Actor(_)))
                .count(),
            1
        );
        for app in apps {
            assert_eq!(
                app.prepared.origin,
                SupportOrigin::Assignment(occurrence(61))
            );
            assert_eq!(app.prepared.target, target(30, "first"));
            assert_eq!(app.prepared.position, 0);
            assert!(applicability(effects, app));
            let delivery = effects
                .effects
                .iter()
                .find(|effect| {
                    application(effect) == Some(app)
                        && effect.key.invocation.program.as_str().ends_with("deliver")
                })
                .unwrap();
            assert_eq!(delivery.value, known(16));
            match &app.receiver {
                SupportReceiverKey::Actor(actor) => assert_eq!(
                    final_value(effects, &ConcreteEntity::Actor(actor.clone())),
                    &known(16)
                ),
                SupportReceiverKey::Action(action) => {
                    check_child_action(action);
                    assert_eq!(
                        final_value(effects, &ConcreteEntity::Action(action.clone())),
                        &known(16)
                    );
                }
                SupportReceiverKey::Skill(_) => {
                    panic!("fixture declares Actor/Action endpoints only")
                }
            }
        }
    }
}

#[test]
fn explicit_summoner_uses_native_parent_types_while_absent_summoner_starts_child_fresh() {
    for with_summoner in [false, true] {
        let mut f = source_fixture();
        flag(parent_facts(&mut f), "spell", false);
        flag(parent_facts(&mut f), "has-gem", true);
        add_child(&mut f, false);
        let names = ["producer", "consumer"];
        distinct_supports(&mut f, &names);
        let args = multi_inputs(
            &f,
            &names,
            |preparation| {
                for entry in &mut preparation.supports {
                    if entry.gem == def::<GemDefinition>("producer") {
                        let SchemaState::Known(support) = &mut entry.preparation else {
                            unreachable!()
                        };
                        support.requires = None;
                        support.added_types = vec![key("spell")];
                        support.gems_only = true;
                    }
                }
            },
            with_summoner,
        );
        let plan = compile_args(args);
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        let effects = evaluated(&report);
        let apps: BTreeSet<_> = effects.effects.iter().filter_map(application).collect();
        assert_eq!(apps.len(), 6);
        for app in apps {
            match &app.receiver {
                SupportReceiverKey::Actor(_) => assert!(applicability(effects, app)),
                SupportReceiverKey::Skill(_) => {
                    panic!("fixture declares Actor/Action endpoints only")
                }
                SupportReceiverKey::Action(action) => {
                    check_child_action(action);
                    let consumer = app.prepared.origin == SupportOrigin::Assignment(occurrence(61));
                    assert_eq!(applicability(effects, app), with_summoner && consumer);
                    assert_eq!(
                        final_value(effects, &ConcreteEntity::Action(action.clone())),
                        &known(if with_summoner { 16 } else { 0 })
                    );
                }
            }
        }
        assert_eq!(
            final_value(effects, &ConcreteEntity::Actor(child_actor(30))),
            &known(30)
        );
    }
}

#[test]
fn inherited_duplicate_positions_survive_without_reselection_and_deliver_twice() {
    let mut f = source_fixture();
    add_child(&mut f, false);
    let names = ["family-a", "family-b", "replacement"];
    distinct_supports(&mut f, &names);
    let args = multi_inputs(
        &f,
        &names,
        |preparation| {
            preparation.families = vec![key("a"), key("b")];
            for entry in &mut preparation.supports {
                let SchemaState::Known(support) = &mut entry.preparation else {
                    unreachable!()
                };
                support.families = Some(if entry.gem == def::<GemDefinition>("family-a") {
                    vec![key("a")]
                } else if entry.gem == def::<GemDefinition>("family-b") {
                    vec![key("b")]
                } else {
                    vec![key("a"), key("b")]
                });
            }
        },
        true,
    );
    let plan = compile_args(args);
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let effects = evaluated(&report);
    let apps: BTreeSet<_> = effects.effects.iter().filter_map(application).collect();
    assert_eq!(apps.len(), 6);
    assert_eq!(
        apps.iter()
            .map(|a| a.prepared.position)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([0, 1])
    );
    for app in apps {
        assert_eq!(
            app.prepared.origin,
            SupportOrigin::Assignment(occurrence(66))
        );
        assert_eq!(app.prepared.target, target(30, "first"));
        assert!(applicability(effects, app));
        let delivery = effects
            .effects
            .iter()
            .find(|e| {
                application(e) == Some(app)
                    && e.key.invocation.program.as_str().ends_with("deliver")
            })
            .unwrap();
        assert_eq!(delivery.value, known(18));
        if let SupportReceiverKey::Action(action) = &app.receiver {
            check_child_action(action);
            assert_eq!(
                final_value(effects, &ConcreteEntity::Action(action.clone())),
                &known(36)
            );
        }
    }
    assert_eq!(
        final_value(effects, &ConcreteEntity::Actor(child_actor(30))),
        &known(36)
    );
}

#[test]
fn exact_child_skill_admission_preserves_assignment_context_without_action_demand() {
    let mut f = source_fixture();
    add_child(&mut f, true);
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
    let args = inputs_with_packages(
        &f,
        |rules| {
            let programs = &mut rules
                .owners
                .iter_mut()
                .find(|owner| owner.owner == support_owner())
                .unwrap()
                .programs
                .members;
            for program in programs {
                if [key("action-app"), key("action-deliver")].contains(&program.id) {
                    program.context = RuleEntityKind::Skill;
                }
            }
        },
        |preparation| {
            let SchemaState::Known(support) = &mut preparation.supports[0].preparation else {
                unreachable!()
            };
            support.gems_only = true;
        },
        |receiving| {
            receiving
                .roles
                .iter_mut()
                .find(|role| role.id == key("action"))
                .unwrap()
                .kind = SupportReceiverKind::Skill;
            let role = receiving.targets[0]
                .roles
                .members
                .iter_mut()
                .find(|role| role.role == key("action"))
                .unwrap();
            role.endpoints = DeclaredSet::complete(vec![SupportReceiverEndpoint::Skill {
                path: vec![child_grant()],
                admission: SupportAdmissionContext::ReceivingSkill {
                    summoner_path: Some(vec![]),
                },
            }]);
        },
    );
    let plan = compile_args(args);
    let result = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let report = evaluated(&result);
    assert!(
        !report
            .effects
            .iter()
            .any(|effect| matches!(effect.key.invocation.entity, ConcreteEntity::Action(_)))
    );
    let applications: BTreeSet<_> = report.effects.iter().filter_map(application).collect();
    assert_eq!(applications.len(), 6);
    let mut children = BTreeSet::new();
    for application in applications {
        match &application.receiver {
            SupportReceiverKey::Actor(_) => assert!(!applicability(report, application)),
            SupportReceiverKey::Action(_) => panic!("this fixture declares no Action endpoint"),
            SupportReceiverKey::Skill(skill) => {
                assert!(applicability(report, application));
                assert_ne!(skill.as_ref(), &application.prepared.target);
                let SkillTarget::Generated(child) = skill.as_ref() else {
                    panic!("exact generated child")
                };
                let (root, name, origin) =
                    [(30, "first", 61), (30, "second", 63), (31, "first", 65)]
                        .into_iter()
                        .find(|(root, name, _)| application.prepared.target == target(*root, name))
                        .unwrap();
                assert_eq!(
                    application.prepared.origin,
                    SupportOrigin::Assignment(occurrence(origin))
                );
                let SkillTarget::Generated(parent) = target(root, name) else {
                    unreachable!()
                };
                let mut provider = parent.provider;
                provider.grant_path.push(DeclaredSlot {
                    declaration: SlotOwnerDefId::Actor(def("family")),
                    slot: def(name),
                });
                assert_eq!(
                    child.as_ref(),
                    &GeneratedSkillKey {
                        provider,
                        slot: child_supply()
                    }
                );
                let expected = if application.prepared.target == target(31, "first") {
                    25
                } else {
                    16
                };
                let effect = report
                    .effects
                    .iter()
                    .find(|effect| {
                        self::application(effect) == Some(application)
                            && effect.key.invocation.program == key("action-deliver")
                    })
                    .unwrap();
                assert_eq!(
                    effect.key.invocation.entity,
                    ConcreteEntity::Skill(skill.clone())
                );
                assert_eq!(effect.value, known(expected));
                // The child's own scalar is 777, but delivery must retain the
                // original assignment's support level and effective scalar.
                assert!(children.insert(skill.clone()));
            }
        }
    }
    assert_eq!(children.len(), 3);
}
