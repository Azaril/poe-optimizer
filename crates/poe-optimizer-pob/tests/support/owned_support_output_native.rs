//! Test-only authoring of ordinary typed rules from observed source definitions.
//! The occurrence topology is controlled; production has no skill-name dispatch.
#[allow(dead_code)]
#[path = "../../../poe-optimizer-engine/tests/support/owned_support_delivery_fixture.rs"]
mod delivery;
use delivery::{def, key, occurrence, subject};
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*, owned_stages::*,
    owned_support_inputs::*, owned_support_outputs::*, owned_support_receiving::*,
    owned_supports::*,
};
use poe_optimizer_data::{owned_rules::*, owned_support_outputs::*};
use poe_optimizer_engine::{owned_plan::*, owned_supports::*};
use std::sync::Arc;

pub const TYPES: [&str; 4] = ["attack", "ammo", "minion", "undamageable"];
pub enum Receiver {
    Action,
    SummonedActor,
}
pub struct Facts {
    pub types: [bool; 4],
    pub minion_types_present: bool,
    pub percent: f64,
    pub receiver: Receiver,
}
fn quantity(n: f64, unit: &str) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(n, def(unit)).unwrap())
}
fn stat(
    name: &str,
    value: ComputedValueType,
    targets: Vec<RuleEntityKind>,
) -> DefinitionDescriptor {
    DefinitionDescriptor::Stat(DefinitionEntry {
        id: def(name),
        schema: SchemaState::Known(StatSchema { value, targets }),
    })
}
fn child_slot() -> DeclaredSlot<ActorSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(def("ability")),
        slot: def("receiver-child"),
    }
}
fn child_grant() -> DeclaredSlot<GrantSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(def("ability")),
        slot: def("receiver-child-grant"),
    }
}
fn child_actor() -> ActorKey {
    let SkillTarget::Generated(target) = delivery::target(30, "first") else {
        unreachable!()
    };
    let mut provider = target.provider;
    provider.grant_path.push(DeclaredSlot {
        declaration: SlotOwnerDefId::Actor(def("family")),
        slot: def("first"),
    });
    ActorKey::Owned(Box::new(OwnedActorKey {
        provider,
        slot: child_slot(),
    }))
}
fn reducer(name: &str, context: RuleEntityKind, channels: &[&str]) -> RuleProgram {
    RuleProgram {
        id: key(name),
        context,
        reads: channels
            .iter()
            .map(|name| RuleRead {
                id: key(name),
                value_type: ComputedValueType::Quantity {
                    unit: def("factor"),
                },
                source: RuleReadSource::Contributions {
                    entity: RuleEntity::Current,
                    stat: def(name),
                    contribution: ContributionKind::Multiply,
                    reduction: ContributionReduction::Product,
                    empty: quantity(1.0, "factor"),
                },
            })
            .collect(),
        nodes: channels
            .iter()
            .map(|name| delivery::fixture::read_node(name, name))
            .collect(),
        effects: channels
            .iter()
            .map(|name| delivery::fixture::derive(name, RuleEntity::Current, name, name))
            .collect(),
    }
}
fn delivery_program(
    context: RuleEntityKind,
    percent: f64,
    channels: &[&str],
    attack_guard: bool,
) -> RuleProgram {
    let mut nodes = vec![
        RuleNode {
            id: key("percent"),
            expression: RuleExpression::Literal {
                value: quantity(percent, "quality"),
            },
        },
        RuleNode {
            id: key("fraction"),
            expression: RuleExpression::PercentAsFactor {
                percent: key("percent"),
                unit: def("factor"),
            },
        },
        RuleNode {
            id: key("one"),
            expression: RuleExpression::Literal {
                value: quantity(1.0, "factor"),
            },
        },
        RuleNode {
            id: key("factor"),
            expression: RuleExpression::Add {
                left: key("one"),
                right: key("fraction"),
            },
        },
    ];
    let mut reads = vec![];
    if attack_guard {
        reads.push(RuleRead {
            id: key("attack"),
            value_type: ComputedValueType::Boolean,
            source: RuleReadSource::Stat {
                entity: RuleEntity::Skill,
                stat: def("final-attack"),
            },
        });
        nodes.push(delivery::fixture::read_node("attack", "attack"));
    }
    RuleProgram {
        id: key(if attack_guard {
            "action-deliver"
        } else {
            "actor-deliver"
        }),
        context,
        reads,
        nodes,
        effects: channels
            .iter()
            .map(|name| RuleEffect {
                id: key(name),
                when: attack_guard.then(|| key("attack")),
                effect: RuleEffectKind::Contribute {
                    entity: RuleEntity::Current,
                    stat: def(name),
                    contribution: ContributionKind::Multiply,
                    value: key("factor"),
                },
            })
            .collect(),
    }
}

/// Returns exact component factors, and selected contextual output facts.
pub fn evaluate(facts: Facts) -> OwnedEffectsReport {
    let mut f = delivery::source_fixture();
    let actor = matches!(facts.receiver, Receiver::SummonedActor);
    let channels: Vec<&str> = if actor {
        vec!["damage", "damage-taken"]
    } else {
        vec!["elemental-damage"]
    };
    f.build.supports.retain(|s| s.id == occurrence(60));
    f.build
        .support_origins
        .as_mut()
        .unwrap()
        .retain(|r| r.target == delivery::target(30, "first"));
    f.build.support_origins.as_mut().unwrap()[0].origins =
        vec![SupportOrigin::Assignment(occurrence(60))];
    f.schema
        .definitions
        .push(DefinitionDescriptor::Unit(DefinitionEntry {
            id: def("factor"),
            schema: SchemaState::Known(UnitSchema {
                dimension: UnitDimension::DimensionlessFactor,
            }),
        }));
    for channel in &channels {
        f.schema.definitions.push(stat(
            channel,
            ComputedValueType::Quantity {
                unit: def("factor"),
            },
            vec![RuleEntityKind::Actor, RuleEntityKind::Action],
        ));
    }
    for name in TYPES {
        for prefix in ["initial", "final"] {
            f.schema.definitions.push(stat(
                &format!("{prefix}-{name}"),
                ComputedValueType::Boolean,
                vec![RuleEntityKind::Skill],
            ));
        }
    }
    f.schema.definitions.push(stat(
        "no-types",
        ComputedValueType::Boolean,
        vec![RuleEntityKind::Skill],
    ));
    let target_facts = &mut f
        .owner_mut(&subject(def::<SkillDefinition>("ability")))
        .programs
        .members[0];
    for (name, present) in TYPES.into_iter().zip(facts.types) {
        let name = format!("initial-{name}");
        target_facts
            .nodes
            .push(delivery::fixture::bool_node(&name, present));
        target_facts.effects.push(delivery::fixture::derive(
            &name,
            RuleEntity::Current,
            &name,
            &name,
        ));
    }
    target_facts
        .nodes
        .push(delivery::fixture::bool_node("no-types", false));
    target_facts.effects.push(delivery::fixture::derive(
        "no-types",
        RuleEntity::Current,
        "no-types",
        "no-types",
    ));
    target_facts
        .nodes
        .iter_mut()
        .find(|n| n.id == key("minion-present"))
        .unwrap()
        .expression = RuleExpression::Literal {
        value: ParameterValue::Boolean(facts.minion_types_present),
    };
    let origin = &mut f.owner_mut(&delivery::support_owner()).programs.members[0];
    origin
        .nodes
        .iter_mut()
        .find(|n| n.id == key("effective"))
        .unwrap()
        .expression = RuleExpression::Read {
        input: key("level"),
    };
    f.owner_mut(&subject(def::<ActorDefinition>("family")))
        .programs
        .members
        .push(reducer(
            "factor-final-actor",
            RuleEntityKind::Actor,
            &channels,
        ));
    f.owner_mut(&SchemaSubject::Slot(SlotAddress::ActionOutput(
        delivery::output(),
    )))
    .programs
    .members
    .push(reducer(
        "factor-final-action",
        RuleEntityKind::Action,
        &channels,
    ));
    if actor {
        for descriptor in &mut f.schema.definitions {
            if let DefinitionDescriptor::Skill(DefinitionEntry {
                id,
                schema: SchemaState::Known(schema),
            }) = descriptor
                && *id == def::<SkillDefinition>("ability")
            {
                schema.declarations.actors.members.push(child_slot());
                schema.declarations.grants.members.push(child_grant());
            }
        }
        f.schema.slots.extend([
            SlotDescriptor::Actor(DefinitionEntry {
                id: child_slot(),
                schema: SchemaState::Known(ActorSlotSchema {
                    provider_definition: None,
                    skills: DeclaredSet::complete(vec![]),
                    outputs: DeclaredSet::complete(vec![]),
                }),
            }),
            SlotDescriptor::Grant(DefinitionEntry {
                id: child_grant(),
                schema: SchemaState::Known(GrantSlotSchema {
                    provider_roles: vec![ProviderRole::SkillUse],
                    target: GrantTarget::Actor(child_slot()),
                }),
            }),
        ]);
        let parent = &mut f
            .owner_mut(&subject(def::<SkillDefinition>("ability")))
            .programs
            .members[0];
        parent
            .nodes
            .push(delivery::fixture::bool_node("child-present", true));
        parent.effects.push(delivery::effect(
            "child",
            RuleEffectKind::ActivateGrant {
                slot: child_grant(),
                enabled: key("child-present"),
            },
        ));
        f.owners.extend([
            DefinitionRules {
                owner: SchemaSubject::Slot(SlotAddress::Actor(child_slot())),
                programs: DeclaredSet::complete(vec![reducer(
                    "factor-final-child",
                    RuleEntityKind::Actor,
                    &channels,
                )]),
            },
            DefinitionRules {
                owner: SchemaSubject::Slot(SlotAddress::Grant(child_grant())),
                programs: DeclaredSet::complete(vec![]),
            },
        ]);
    }
    let mut authored = None;
    let args = delivery::inputs_with_all_packages(
        &f,
        |rules| {
            let owner = rules
                .owners
                .iter_mut()
                .find(|r| r.owner == delivery::support_owner())
                .unwrap();
            let removed = if actor {
                ["action-app", "action-deliver"]
            } else {
                ["actor-app", "actor-deliver"]
            };
            owner
                .programs
                .members
                .retain(|p| !removed.iter().any(|name| p.id == key(name)));
            let program = owner
                .programs
                .members
                .iter_mut()
                .find(|p| {
                    p.id == key(if actor {
                        "actor-deliver"
                    } else {
                        "action-deliver"
                    })
                })
                .unwrap();
            *program = delivery_program(
                if actor {
                    RuleEntityKind::Actor
                } else {
                    RuleEntityKind::Action
                },
                facts.percent,
                &channels,
                !actor,
            );
            authored = Some(rules.clone());
        },
        |preparation| {
            preparation.types = TYPES.into_iter().map(key).collect();
            let SchemaState::Known(support) = &mut preparation.supports[0].preparation else {
                unreachable!()
            };
            support.requires = Some(if actor {
                SupportTypePredicate::Type(key("minion"))
            } else {
                SupportTypePredicate::Any(vec![
                    SupportTypePredicate::Type(key("attack")),
                    SupportTypePredicate::Type(key("ammo")),
                ])
            });
            support.excludes = if actor {
                Some(SupportTypePredicate::Type(key("undamageable")))
            } else {
                None
            };
        },
        |stages| {
            stages.stages.push(EvaluationStage {
                id: key("prepared-types"),
                predecessors: vec![key("prepare")],
            });
            stages
                .stages
                .iter_mut()
                .find(|s| s.id == key("apply"))
                .unwrap()
                .predecessors = vec![key("prepared-types")];
            let removed = if actor {
                ["action-app", "action-deliver"]
            } else {
                ["actor-app", "actor-deliver"]
            };
            stages.programs.members.retain(|row| {
                row.owner != delivery::support_owner()
                    || !removed.iter().any(|name| row.program == key(name))
            });
            for row in &mut stages.programs.members {
                if row.program.as_str().starts_with("factor-final-") {
                    row.stage = key("final");
                }
            }
            for name in TYPES {
                stages.frozen_channels.push(FrozenStageChannel {
                    channel: StageChannel::Stat {
                        scope: RuleEntityKind::Skill,
                        stat: def(&format!("initial-{name}")),
                    },
                    stage: key("prepare"),
                });
                stages.frozen_channels.push(FrozenStageChannel {
                    channel: StageChannel::Stat {
                        scope: RuleEntityKind::Skill,
                        stat: def(&format!("final-{name}")),
                    },
                    stage: key("prepared-types"),
                });
            }
            stages.frozen_channels.push(FrozenStageChannel {
                channel: StageChannel::Stat {
                    scope: RuleEntityKind::Skill,
                    stat: def("no-types"),
                },
                stage: key("prepare"),
            });
        },
        |inputs| {
            let empty_types: Vec<_> = TYPES
                .into_iter()
                .map(|name| SupportTypeStat {
                    support_type: key(name),
                    stat: def("no-types"),
                })
                .collect();
            inputs.target.skill_types = TYPES
                .into_iter()
                .map(|name| SupportTypeStat {
                    support_type: key(name),
                    stat: def(&format!("initial-{name}")),
                })
                .collect();
            inputs.target.minion_types.members = empty_types.clone();
            inputs.target.summoner.skill_types = empty_types.clone();
            inputs.target.summoner.minion_types.members = empty_types;
        },
        |receiving| {
            receiving.supports[0]
                .receivers
                .members
                .retain(|r| r.role == key(if actor { "actor" } else { "action" }));
            if actor {
                let row = receiving.targets[0]
                    .roles
                    .members
                    .iter_mut()
                    .find(|r| r.role == key("actor"))
                    .unwrap();
                row.endpoints = DeclaredSet::complete(vec![SupportReceiverEndpoint::Actor {
                    path: vec![child_grant()],
                    admission: SupportAdmissionContext::AssignedSkill,
                }]);
            }
        },
    );
    let stored = OwnedRulePackage::new(
        authored.unwrap(),
        args.definitions.as_ref(),
        RuleStorageLimits::default(),
    )
    .unwrap();
    assert_eq!(args.rules.source_identity(), Some(*stored.identity()));
    let outputs = Arc::new(
        OwnedSupportOutputBindings::new(
            SupportOutputBindingsInput {
                schema_version: OWNED_SUPPORT_OUTPUT_BINDINGS_VERSION,
                namespace: delivery::fixture::ns(),
                release: key("reference-outputs"),
                definitions: args.definitions.identity().clone(),
                rules: *stored.identity(),
                preparation: *args.preparation.identity(),
                inputs: *args.inputs.identity(),
                receiving: *args.receiving.identity(),
                stages: *args.stages.identity(),
                output_stage: key("prepared-types"),
                final_skill_types: TYPES
                    .into_iter()
                    .map(|name| SupportTypeStat {
                        support_type: key(name),
                        stat: def(&format!("final-{name}")),
                    })
                    .collect(),
            },
            &SupportOutputDependencies {
                definitions: args.definitions.as_ref(),
                rules: &stored,
                preparation: &args.preparation,
                inputs: &args.inputs,
                receiving: &args.receiving,
                stages: &args.stages,
            },
            SupportOutputStorageLimits::default(),
        )
        .unwrap(),
    );
    let plan = OwnedSupportEffectPlan::compile_with_outputs(
        args,
        outputs,
        PlanLimits::default(),
        SupportPreparationLimits::default(),
    )
    .unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    let SupportEffectsOutcome::Evaluated { effects } = report.outcome else {
        panic!("expected evaluated component: {report:?}")
    };
    effects
}

pub fn factors(report: &OwnedEffectsReport, stat: &str) -> Vec<(ConcreteEntity, f64)> {
    report
        .values
        .iter()
        .filter_map(|row| {
            let PlanValueKey::Stat { entity, stat: id } = &row.key else {
                return None;
            };
            if *id != def::<StatDefinition>(stat) {
                return None;
            }
            let EffectValue::Known {
                value: ParameterValue::Quantity(q),
            } = &row.value
            else {
                panic!("known factor {row:?}")
            };
            Some((entity.clone(), q.value()))
        })
        .collect()
}
pub fn expected_child() -> ConcreteEntity {
    ConcreteEntity::Actor(child_actor())
}
pub fn parent() -> ConcreteEntity {
    ConcreteEntity::Actor(delivery::child_actor(30))
}
