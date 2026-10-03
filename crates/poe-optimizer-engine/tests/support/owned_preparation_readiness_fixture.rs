//! Closed readiness witness. Synthetic owned definitions, real public support execution.
#[allow(dead_code, unused_imports)]
#[path = "owned_support_delivery_fixture.rs"]
pub mod delivery;
use delivery::fixture as base;
pub use delivery::{Fixture, def, effect, key, occurrence, subject};
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_metrics::*, owned_readiness::*, owned_rules::*,
    owned_schema::*, owned_stages::*, owned_support_receiving::*,
};
use poe_optimizer_data::{
    owned_metrics::{MetricMappingLimits, OwnedMetricMapping},
    owned_rules::OwnedRulePackage,
    owned_schema::OwnedDefinitionSchemaPackage,
    owned_stages::OwnedEvaluationStages,
    owned_support_inputs::OwnedSupportInputBindings,
    owned_support_receiving::OwnedSupportReceiving,
    owned_supports::OwnedSupportPreparation,
};
use poe_optimizer_engine::{owned_plan::*, owned_rules::*, owned_supports::*};
use std::{collections::BTreeSet, sync::Arc};

pub type Inputs = SupportEffectPlanInputs<OwnedDefinitionSchemaPackage>;
pub type Effects = OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>;
pub type Metrics = OwnedSupportMetricPlan<OwnedDefinitionSchemaPackage>;
pub type Checked<T> = std::result::Result<T, String>;

fn known<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn ports() -> DeclaredSlots {
    DeclaredSlots {
        parameters: DeclaredSet::complete(vec![]),
        choices: DeclaredSet::complete(vec![]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    }
}
fn range() -> IntegerRange {
    IntegerRange {
        minimum: BoundedInteger::new(0).unwrap(),
        maximum: BoundedInteger::new(100).unwrap(),
    }
}
pub fn summon_supply() -> DeclaredSlot<SkillGrantSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Gem(def("summoner")),
        slot: def("summon-supply"),
    }
}
pub fn summon_grant() -> DeclaredSlot<GrantSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Gem(def("summoner")),
        slot: def("summon-grant"),
    }
}
pub fn summon_parameter(name: &str) -> DeclaredSlot<ParameterSlotDefId> {
    base::parameter(SlotOwnerDefId::Skill(def("summon")), name)
}
pub fn ability_parameter(name: &str) -> DeclaredSlot<ParameterSlotDefId> {
    base::parameter(SlotOwnerDefId::Skill(def("ability")), name)
}
pub fn actor_slot() -> DeclaredSlot<ActorSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(def("summon")),
        slot: def("child"),
    }
}
pub fn actor_grant() -> DeclaredSlot<GrantSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(def("summon")),
        slot: def("child-grant"),
    }
}
pub fn ability_supply(name: &str) -> DeclaredSlot<SkillGrantSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Actor(def("family")),
        slot: def(name),
    }
}
pub fn ability_grant(name: &str) -> DeclaredSlot<GrantSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Actor(def("family")),
        slot: def(name),
    }
}
pub fn summon_provider(use_id: u64) -> ProviderKey {
    ProviderKey {
        root: ProviderRoot::SkillUse(occurrence(use_id)),
        grant_path: vec![summon_grant()],
    }
}
pub fn actor(use_id: u64) -> ActorKey {
    ActorKey::Owned(Box::new(OwnedActorKey {
        provider: summon_provider(use_id),
        slot: actor_slot(),
    }))
}
fn target_root(root: ProviderRoot, name: &str) -> SkillTarget {
    SkillTarget::Generated(Box::new(GeneratedSkillKey {
        provider: ProviderKey {
            root,
            grant_path: vec![summon_grant(), actor_grant()],
        },
        slot: ability_supply(name),
    }))
}
pub fn target(use_id: u64, name: &str) -> SkillTarget {
    target_root(ProviderRoot::SkillUse(occurrence(use_id)), name)
}
pub fn action(use_id: u64, name: &str) -> ActionSelection {
    let SkillTarget::Generated(target) = target(use_id, name) else {
        unreachable!()
    };
    let mut provider = target.provider;
    provider.grant_path.push(ability_grant(name));
    ActionSelection {
        action: ActionKey {
            actor: actor(use_id),
            provider,
            output: delivery::output(),
        },
        part: def("part"),
        mode: def("mode"),
        stat_set: def("set"),
    }
}
pub fn quantity(value: f64) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(value, def("count")).unwrap())
}
pub fn program_mut<'a>(
    f: &'a mut Fixture,
    owner: SchemaSubject,
    name: &str,
) -> &'a mut RuleProgram {
    f.owner_mut(&owner)
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == key(name))
        .unwrap()
}
pub fn actor_owner() -> SchemaSubject {
    subject(def::<ActorDefinition>("family"))
}
pub fn child_owner() -> SchemaSubject {
    subject(def::<SkillDefinition>("ability"))
}
pub fn summon_owner() -> SchemaSubject {
    subject(def::<SkillDefinition>("summon"))
}
pub fn physical_owner() -> SchemaSubject {
    subject(def::<GemDefinition>("summoner"))
}

pub fn fixture() -> Fixture {
    let mut f = delivery::source_fixture();
    // The shared fixture has a physical root that directly supplies the actor.
    // Insert a real supplied summon; every target below retains both entering grants.
    for definition in &mut f.schema.definitions {
        match definition {
            DefinitionDescriptor::Gem(row) if row.id == def::<GemDefinition>("summoner") => {
                let SchemaState::Known(schema) = &mut row.schema else {
                    unreachable!()
                };
                schema.skills = DeclaredSet::complete(vec![def("summon")]);
                schema.declarations.actors.members.clear();
                schema.declarations.grants.members = vec![summon_grant()];
                schema.declarations.skill_grants.members = vec![summon_supply()];
            }
            DefinitionDescriptor::Metric(row) => {
                row.schema = SchemaState::Known(MetricSchema {
                    targets: vec![MetricTargetKind::Action],
                    unit: def("count"),
                    actor_roles: vec![MetricActorRole::Owned],
                    provider_roles: vec![ProviderRole::SkillUse],
                })
            }
            DefinitionDescriptor::Skill(row) if row.id == def::<SkillDefinition>("ability") => {
                let SchemaState::Known(schema) = &mut row.schema else {
                    unreachable!()
                };
                schema
                    .declarations
                    .parameters
                    .members
                    .push(ability_parameter("preparation-level"));
            }
            _ => {}
        }
    }
    for slot in &mut f.schema.slots {
        match slot {
            SlotDescriptor::Actor(row) if row.id == base::child_slot() => row.id = actor_slot(),
            SlotDescriptor::Grant(row) if row.id == base::child_grant() => {
                row.id = actor_grant();
                let SchemaState::Known(schema) = &mut row.schema else {
                    unreachable!()
                };
                schema.target = GrantTarget::Actor(actor_slot());
            }
            _ => {}
        }
    }
    for owner in &mut f.owners {
        if owner.owner == SchemaSubject::Slot(SlotAddress::Actor(base::child_slot())) {
            owner.owner = SchemaSubject::Slot(SlotAddress::Actor(actor_slot()));
        } else if owner.owner == SchemaSubject::Slot(SlotAddress::Grant(base::child_grant())) {
            owner.owner = SchemaSubject::Slot(SlotAddress::Grant(actor_grant()));
        }
    }
    let mut summon_ports = ports();
    summon_ports.parameters.members = vec![summon_parameter("level"), summon_parameter("enabled")];
    summon_ports.actors.members = vec![actor_slot()];
    summon_ports.grants.members = vec![actor_grant()];
    f.schema.definitions.push(DefinitionDescriptor::Skill(known(
        def("summon"),
        SkillSchema {
            directly_selectable: false,
            declarations: summon_ports,
        },
    )));
    f.schema.slots.extend([
        SlotDescriptor::Parameter(known(
            ability_parameter("preparation-level"),
            ParameterSlotSchema {
                value: ValueSchema::Integer(range()),
                presence: SlotPresence::RequiredOnce,
                sites: vec![],
            },
        )),
        SlotDescriptor::SkillGrant(known(
            summon_supply(),
            SkillGrantSlotSchema {
                skill: def("summon"),
                outputs: DeclaredSet::complete(vec![]),
            },
        )),
        SlotDescriptor::Grant(known(
            summon_grant(),
            GrantSlotSchema {
                provider_roles: vec![ProviderRole::SkillUse],
                target: GrantTarget::Skill(summon_supply()),
            },
        )),
        SlotDescriptor::Parameter(known(
            summon_parameter("level"),
            ParameterSlotSchema {
                value: ValueSchema::Integer(range()),
                presence: SlotPresence::RequiredOnce,
                sites: vec![],
            },
        )),
        SlotDescriptor::Parameter(known(
            summon_parameter("enabled"),
            ParameterSlotSchema {
                value: ValueSchema::Boolean,
                presence: SlotPresence::RequiredOnce,
                sites: vec![],
            },
        )),
    ]);
    let root_program = program_mut(&mut f, physical_owner(), "supply-child");
    root_program.id = key("supply-summon");
    root_program.nodes.push(base::bool_node("present", true));
    root_program.effects = vec![
        effect(
            "activate-summon",
            RuleEffectKind::ActivateGrant {
                slot: summon_grant(),
                enabled: key("present"),
            },
        ),
        effect(
            "summon-level",
            RuleEffectKind::ProjectSkillParameter {
                skill: summon_supply(),
                parameter: summon_parameter("level"),
                value: key("level"),
            },
        ),
        effect(
            "summon-enabled",
            RuleEffectKind::ProjectSkillParameter {
                skill: summon_supply(),
                parameter: summon_parameter("enabled"),
                value: key("enabled"),
            },
        ),
    ];
    f.owners.push(DefinitionRules {
        owner: summon_owner(),
        programs: DeclaredSet::complete(vec![RuleProgram {
            id: key("supply-child"),
            context: RuleEntityKind::Skill,
            reads: vec![
                base::read(
                    "level",
                    RuleReadSource::Parameter {
                        slot: summon_parameter("level"),
                    },
                ),
                RuleRead {
                    id: key("enabled"),
                    value_type: ComputedValueType::Boolean,
                    source: RuleReadSource::Parameter {
                        slot: summon_parameter("enabled"),
                    },
                },
            ],
            nodes: vec![
                base::read_node("level", "level"),
                base::read_node("enabled", "enabled"),
            ],
            effects: vec![
                effect(
                    "actor-level",
                    RuleEffectKind::ProjectActorStat {
                        actor: actor_slot(),
                        stat: def("child-level"),
                        value: key("level"),
                    },
                ),
                effect(
                    "activate-actor",
                    RuleEffectKind::ActivateGrant {
                        slot: actor_grant(),
                        enabled: key("enabled"),
                    },
                ),
            ],
        }]),
    });
    // Final child inputs deliberately depend on admitted support properties.
    // Their structural grants must be separate programs in the initial prefix.
    let family = f.owner_mut(&actor_owner());
    family.programs.members.clear();
    for name in ["first", "second"] {
        family.programs.members.push(RuleProgram {
            id: key(&format!("activate-{name}")),
            context: RuleEntityKind::Actor,
            reads: vec![],
            nodes: vec![base::bool_node("enabled", true)],
            effects: vec![effect(
                "activate",
                RuleEffectKind::ActivateGrant {
                    slot: ability_grant(name),
                    enabled: key("enabled"),
                },
            )],
        });
        family.programs.members.push(RuleProgram {
            id: key(&format!("prepare-{name}")),
            context: RuleEntityKind::Actor,
            reads: vec![base::read(
                "base",
                RuleReadSource::Stat {
                    entity: RuleEntity::Current,
                    stat: def("child-plus"),
                },
            )],
            nodes: vec![base::read_node("base", "base")],
            effects: vec![effect(
                "preparation-level",
                RuleEffectKind::ProjectSkillParameter {
                    skill: ability_supply(name),
                    parameter: ability_parameter("preparation-level"),
                    value: key("base"),
                },
            )],
        });
        family.programs.members.push(RuleProgram {
            id: key(&format!("assemble-{name}")),
            context: RuleEntityKind::Actor,
            reads: vec![
                base::read(
                    "base",
                    RuleReadSource::Stat {
                        entity: RuleEntity::Current,
                        stat: def("child-plus"),
                    },
                ),
                base::contributions("properties", RuleEntity::Current, "prepared-property"),
            ],
            nodes: vec![
                base::read_node("base", "base"),
                base::read_node("properties", "properties"),
                base::node(
                    "final-level",
                    RuleExpression::Add {
                        left: key("base"),
                        right: key("properties"),
                    },
                ),
                base::literal("quality", 0),
            ],
            effects: vec![
                effect(
                    "final-level",
                    RuleEffectKind::ProjectSkillParameter {
                        skill: ability_supply(name),
                        parameter: ability_parameter("level"),
                        value: key("final-level"),
                    },
                ),
                effect(
                    "final-quality",
                    RuleEffectKind::ProjectSkillParameter {
                        skill: ability_supply(name),
                        parameter: ability_parameter("quality"),
                        value: key("quality"),
                    },
                ),
            ],
        });
    }
    // Preparation uses an independent early actor fact, not the final child parameter.
    let facts = program_mut(&mut f, child_owner(), "target-facts");
    facts.reads[0].source = RuleReadSource::Parameter {
        slot: ability_parameter("preparation-level"),
    };
    f.schema.definitions.extend([
        DefinitionDescriptor::Stat(known(
            def("prepared-property"),
            StatSchema {
                value: ComputedValueType::Integer,
                targets: vec![RuleEntityKind::Actor],
            },
        )),
        DefinitionDescriptor::Stat(known(
            def("readiness-metric"),
            StatSchema {
                value: ComputedValueType::Quantity { unit: def("count") },
                targets: vec![RuleEntityKind::Skill, RuleEntityKind::Action],
            },
        )),
    ]);
    f.owner_mut(&child_owner())
        .programs
        .members
        .push(RuleProgram {
            id: key("execution-value"),
            context: RuleEntityKind::Skill,
            reads: vec![
                base::read(
                    "level",
                    RuleReadSource::Parameter {
                        slot: ability_parameter("level"),
                    },
                ),
                base::read(
                    "quality",
                    RuleReadSource::Parameter {
                        slot: ability_parameter("quality"),
                    },
                ),
            ],
            nodes: vec![
                base::read_node("level", "level"),
                base::read_node("quality", "quality"),
                base::node(
                    "total",
                    RuleExpression::Add {
                        left: key("level"),
                        right: key("quality"),
                    },
                ),
                base::node(
                    "unit",
                    RuleExpression::Literal {
                        value: quantity(1.0),
                    },
                ),
                base::node(
                    "quantity",
                    RuleExpression::ScaleInteger {
                        value: key("unit"),
                        count: key("total"),
                    },
                ),
            ],
            effects: vec![base::derive(
                "quantity",
                RuleEntity::Current,
                "readiness-metric",
                "quantity",
            )],
        });
    let action_owner = SchemaSubject::Slot(SlotAddress::ActionOutput(delivery::output()));
    f.owner_mut(&action_owner).programs.members = vec![RuleProgram {
        id: key("execution-metric"),
        context: RuleEntityKind::Action,
        reads: vec![RuleRead {
            id: key("value"),
            value_type: ComputedValueType::Quantity { unit: def("count") },
            source: RuleReadSource::Stat {
                entity: RuleEntity::Skill,
                stat: def("readiness-metric"),
            },
        }],
        nodes: vec![base::read_node("value", "value")],
        effects: vec![base::derive(
            "metric",
            RuleEntity::Current,
            "readiness-metric",
            "value",
        )],
    }];
    let retarget = |old: &SkillTarget| {
        let SkillTarget::Generated(old) = old else {
            unreachable!()
        };
        target_root(old.provider.root.clone(), old.slot.slot.key().as_str())
    };
    for support in &mut f.build.supports {
        support.target = retarget(&support.target);
    }
    for sequence in f.build.support_origins.as_mut().unwrap() {
        sequence.target = retarget(&sequence.target);
    }
    f.queries.requests = [(30, "first"), (30, "second"), (31, "first")]
        .into_iter()
        .map(|(id, name)| MetricRequest {
            id: QueryId::new(format!("{id}-{name}")).unwrap(),
            metric: def("requested"),
            target: MetricTarget::Action(Box::new(action(id, name))),
        })
        .collect();
    f
}

pub fn property_program() -> RuleProgram {
    RuleProgram {
        id: key("prepared-actor-property"),
        context: RuleEntityKind::Actor,
        reads: vec![base::read(
            "level",
            RuleReadSource::Stat {
                entity: RuleEntity::SupportOrigin,
                stat: def("effective-level"),
            },
        )],
        nodes: vec![base::read_node("value", "level")],
        effects: vec![effect(
            "property",
            RuleEffectKind::Contribute {
                entity: RuleEntity::Current,
                stat: def("prepared-property"),
                contribution: ContributionKind::Add,
                value: key("value"),
            },
        )],
    }
}
fn scope(entity: RuleEntity, context: RuleEntityKind) -> RuleEntityKind {
    match entity {
        RuleEntity::Current => context,
        RuleEntity::Actor | RuleEntity::Player => RuleEntityKind::Actor,
        RuleEntity::Skill | RuleEntity::AssignedSkill => RuleEntityKind::Skill,
        RuleEntity::SupportOrigin => RuleEntityKind::SupportOrigin,
        _ => panic!("fixture output scope"),
    }
}
fn outputs(program: &RuleProgram) -> Vec<StageChannel> {
    program
        .effects
        .iter()
        .filter_map(|effect| {
            Some(match &effect.effect {
                RuleEffectKind::Derive { entity, stat, .. } => StageChannel::Stat {
                    scope: scope(*entity, program.context),
                    stat: stat.clone(),
                },
                RuleEffectKind::Contribute {
                    entity,
                    stat,
                    contribution,
                    ..
                } => StageChannel::Contributions {
                    scope: scope(*entity, program.context),
                    stat: stat.clone(),
                    contribution: *contribution,
                },
                RuleEffectKind::ActivateGrant { slot, .. } => {
                    StageChannel::Grant { slot: slot.clone() }
                }
                RuleEffectKind::ProjectActorStat { stat, .. } => StageChannel::Stat {
                    scope: RuleEntityKind::Actor,
                    stat: stat.clone(),
                },
                RuleEffectKind::ProjectSkillParameter { parameter, .. } => {
                    StageChannel::SkillParameter {
                        parameter: parameter.clone(),
                    }
                }
                RuleEffectKind::SupportApplicability { .. } => return None,
                _ => panic!("fixture output effect"),
            })
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
fn role(program: &RuleProgram) -> (ReadinessPhase, ReadinessProgramRole, &'static str) {
    use ReadinessPhase::*;
    use ReadinessProgramRole as Role;
    match program.id.as_str() {
        "supply-summon" => (Structural, Role::FinalInputAssembly, "prepare"),
        "prepare-first" | "prepare-second" => (Structural, Role::FinalInputAssembly, "prepare"),
        "supply-child" | "child-consumer" | "activate-first" | "activate-second" => {
            (Structural, Role::PreparationFacts, "prepare")
        }
        "origin-facts" => (Preparation, Role::PreparationFacts, "prepare"),
        name if name.starts_with("target-facts") => {
            (Preparation, Role::PreparationFacts, "prepare")
        }
        "prepared-actor-app" => (
            Preparation,
            Role::SupportPreparationApplicability,
            "property-app",
        ),
        "prepared-actor-property" => (
            Preparation,
            Role::SupportedPreparationProperty,
            "properties",
        ),
        name if name.starts_with("assemble-") => {
            (Preparation, Role::FinalInputAssembly, "assemble")
        }
        "actor-app" | "action-app" => (Execution, Role::Execution, "apply"),
        "actor-deliver" | "action-deliver" => (Execution, Role::Execution, "deliver"),
        _ => (Execution, Role::Execution, "execute"),
    }
}
pub fn inputs(f: &Fixture) -> Checked<Inputs> {
    inputs_with(f, false, |_| {}, |_| {}, |_| {})
}
pub fn inputs_with(
    f: &Fixture,
    legacy: bool,
    edit_rules: impl FnOnce(&mut RulePackageInput),
    edit_stages: impl FnOnce(&mut EvaluationStagesInput),
    edit_receiving: impl FnOnce(&mut SupportReceivingInput),
) -> Checked<Inputs> {
    // Reuse only checked package/scalar vocabulary setup, not a precompiled plan.
    let base = base::compile_inputs(f, target(30, "first"));
    let mut rule_input = base.rules.input().clone();
    rule_input.operations_version = key(if legacy {
        OWNED_RULE_OPERATIONS_V15
    } else {
        OWNED_RULE_OPERATIONS_V16
    });
    rule_input.effect_applications = Some(DeclaredSet::complete(vec![]));
    let support = rule_input
        .owners
        .iter_mut()
        .find(|o| o.owner == delivery::support_owner())
        .unwrap();
    support.programs.members.extend([
        delivery::support_program("actor-app", RuleEntityKind::Actor, Some(true)),
        delivery::support_program("action-app", RuleEntityKind::Action, Some(true)),
        delivery::support_program("actor-deliver", RuleEntityKind::Actor, None),
        delivery::support_program("action-deliver", RuleEntityKind::Action, None),
    ]);
    if !legacy {
        support.programs.members.extend([
            delivery::support_program("prepared-actor-app", RuleEntityKind::Actor, Some(true)),
            property_program(),
        ]);
    }
    edit_rules(&mut rule_input);
    let stored = OwnedRulePackage::new(rule_input, base.definitions.as_ref(), Default::default())
        .map_err(|e| e.to_string())?;
    let rules = Arc::new(
        CompiledRulePackage::compile_stored(&stored, base.definitions.as_ref(), Default::default())
            .map_err(|e| e.to_string())?,
    );
    let mut stage_input = base.stages.input().clone();
    stage_input.schema_version = if legacy { 1 } else { 2 };
    stage_input.rules = *stored.identity();
    stage_input.effect_applications = Some(DeclaredSet::complete(vec![]));
    let names = [
        "prepare",
        "property-app",
        "properties",
        "assemble",
        "apply",
        "deliver",
        "execute",
    ];
    stage_input.stages = names
        .iter()
        .enumerate()
        .map(|(index, name)| EvaluationStage {
            id: key(name),
            predecessors: index
                .checked_sub(1)
                .map(|i| vec![key(names[i])])
                .unwrap_or_default(),
        })
        .collect();
    stage_input.routing_stage = key("execute");
    stage_input.programs = DeclaredSet::complete(
        stored
            .input()
            .owners
            .iter()
            .flat_map(|owner| {
                owner
                    .programs
                    .members
                    .iter()
                    .map(|program| StagedRuleProgram {
                        owner: owner.owner.clone(),
                        program: program.id.clone(),
                        stage: key(role(program).2),
                    })
            })
            .collect(),
    );
    stage_input.readiness = (!legacy).then(|| ReadinessInput {
        skills: vec![
            GeneratedSkillReadiness {
                skill: def("summon"),
                parameters: DeclaredSet::complete(
                    ["level", "enabled"]
                        .into_iter()
                        .map(|name| ParameterReadiness {
                            parameter: summon_parameter(name),
                            phase: ReadinessPhase::Structural,
                        })
                        .collect(),
                ),
            },
            GeneratedSkillReadiness {
                skill: def("ability"),
                parameters: DeclaredSet::complete(
                    ["preparation-level", "level", "quality"]
                        .into_iter()
                        .map(|name| ParameterReadiness {
                            parameter: ability_parameter(name),
                            phase: if name == "preparation-level" {
                                ReadinessPhase::Preparation
                            } else {
                                ReadinessPhase::Execution
                            },
                        })
                        .collect(),
                ),
            },
        ],
        programs: DeclaredSet::complete(
            stored
                .input()
                .owners
                .iter()
                .flat_map(|owner| {
                    owner.programs.members.iter().map(|program| {
                        let (phase, role, _) = role(program);
                        ReadinessProgram {
                            owner: owner.owner.clone(),
                            program: program.id.clone(),
                            phase,
                            role,
                            outputs: if role == ReadinessProgramRole::Execution {
                                vec![]
                            } else {
                                outputs(program)
                            },
                        }
                    })
                })
                .collect(),
        ),
    });
    edit_stages(&mut stage_input);
    let stages = Arc::new(
        OwnedEvaluationStages::new(
            stage_input,
            base.definitions.as_ref(),
            &stored,
            &base.routing,
            Default::default(),
        )
        .map_err(|e| e.to_string())?,
    );
    let mut preparation_input = base.preparation.input().clone();
    preparation_input.rules = *stored.identity();
    let preparation = Arc::new(
        OwnedSupportPreparation::new(
            preparation_input,
            base.definitions.as_ref(),
            &stored,
            Default::default(),
        )
        .map_err(|e| e.to_string())?,
    );
    let mut inputs = base.inputs.input().clone();
    inputs.rules = *stored.identity();
    inputs.preparation = *preparation.identity();
    inputs.stages = *stages.identity();
    let inputs = Arc::new(
        OwnedSupportInputBindings::new(
            inputs,
            base.definitions.as_ref(),
            &stored,
            &preparation,
            &stages,
            Default::default(),
        )
        .map_err(|e| e.to_string())?,
    );
    let mut receiving = SupportReceivingInput {
        schema_version: if legacy { 1 } else { 2 },
        namespace: base::ns(),
        release: key("receiving"),
        definitions: base.definitions.identity().clone(),
        rules: *stored.identity(),
        preparation: *preparation.identity(),
        inputs: *inputs.identity(),
        stages: *stages.identity(),
        roles: vec![
            SupportReceivingRole {
                id: key("actor"),
                kind: SupportReceiverKind::Actor,
            },
            SupportReceivingRole {
                id: key("action"),
                kind: SupportReceiverKind::Action,
            },
        ],
        targets: vec![SupportTargetReceivingRoles {
            owner: SupportTargetDefinition::Skill(def("ability")),
            roles: DeclaredSet::complete(vec![
                SupportReceivingRoleBinding {
                    role: key("actor"),
                    endpoints: DeclaredSet::complete(vec![SupportReceiverEndpoint::Actor {
                        path: vec![],
                        admission: SupportAdmissionContext::AssignedSkill,
                    }]),
                },
                SupportReceivingRoleBinding {
                    role: key("action"),
                    endpoints: DeclaredSet::complete(vec![SupportReceiverEndpoint::Action {
                        path: vec![],
                        output: delivery::output(),
                        selection: SupportActionSelection::AllDeclared,
                        admission: SupportAdmissionContext::AssignedSkill,
                    }]),
                },
            ]),
        }],
        supports: vec![SupportReceivingEntry {
            gem: def("support"),
            receivers: DeclaredSet::complete(vec![
                SupportRolePrograms {
                    role: key("actor"),
                    applicability: key("actor-app"),
                    delivery: vec![key("actor-deliver")],
                    preparation: (!legacy).then(|| SupportPreparationPrograms {
                        applicability: key("prepared-actor-app"),
                        properties: vec![key("prepared-actor-property")],
                    }),
                },
                SupportRolePrograms {
                    role: key("action"),
                    applicability: key("action-app"),
                    delivery: vec![key("action-deliver")],
                    preparation: None,
                },
            ]),
        }],
    };
    edit_receiving(&mut receiving);
    let receiving = Arc::new(
        OwnedSupportReceiving::new(
            receiving,
            base.definitions.as_ref(),
            &stored,
            &preparation,
            &inputs,
            &stages,
            Default::default(),
        )
        .map_err(|e| e.to_string())?,
    );
    Ok(Inputs {
        request: base.request,
        definitions: base.definitions,
        rules,
        routing: base.routing,
        stages,
        preparation,
        inputs,
        receiving,
    })
}
pub fn compile_inputs(inputs: Inputs) -> Checked<Effects> {
    Effects::compile(
        inputs,
        PlanLimits::default(),
        SupportPreparationLimits::default(),
    )
    .map_err(|e| e.to_string())
}
pub fn compile(f: &Fixture) -> Checked<Effects> {
    compile_inputs(inputs(f)?)
}
pub fn metrics(effects: Effects) -> Metrics {
    let mapping = OwnedMetricMapping::new(
        MetricMappingInput {
            schema_version: OWNED_METRIC_MAPPING_VERSION,
            namespace: base::ns(),
            release: key("metrics"),
            definitions: effects.definitions().identity().clone(),
            bindings: vec![MetricStatBinding {
                metric: def("requested"),
                role: MetricBindingRole::Action,
                stat: def("readiness-metric"),
            }],
        },
        effects.definitions(),
        MetricMappingLimits::default(),
    )
    .unwrap();
    Metrics::compile(Arc::new(effects), Arc::new(mapping)).unwrap()
}
pub fn numbers(report: &OwnedSupportMetricReport) -> Vec<f64> {
    assert!(
        matches!(report.support, SupportMetricStatus::Evaluated),
        "{report:?}"
    );
    assert!(report.evaluation.gaps.is_empty(), "{report:?}");
    report
        .evaluation
        .results
        .iter()
        .map(|row| {
            let EffectValue::Known {
                value: ParameterValue::Quantity(value),
            } = &row.value
            else {
                panic!("expected complete metric: {row:?}")
            };
            assert_eq!(value.unit(), &def("count"));
            value.value()
        })
        .collect()
}
pub fn remove_projection(f: &mut Fixture, owner: SchemaSubject, program: &str, effect_id: &str) {
    program_mut(f, owner, program)
        .effects
        .retain(|effect| effect.id != key(effect_id));
}
pub fn partial(owner: SchemaSubject, facet: SchemaFacet) -> SchemaClosure {
    SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: owner,
            facet,
            code: key("unreviewed"),
        }],
    }
}

pub fn package_program_mut<'a>(
    rules: &'a mut RulePackageInput,
    owner: &SchemaSubject,
    name: &str,
) -> &'a mut RuleProgram {
    rules
        .owners
        .iter_mut()
        .find(|row| &row.owner == owner)
        .unwrap()
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == key(name))
        .unwrap()
}

/// Independent ancestor input: descendants require it through topology, not a read.
pub fn ancestor_execution_input(f: &mut Fixture, produced: bool) {
    for definition in &mut f.schema.definitions {
        if let DefinitionDescriptor::Skill(row) = definition
            && row.id == def::<SkillDefinition>("summon")
        {
            let SchemaState::Known(schema) = &mut row.schema else {
                unreachable!()
            };
            schema
                .declarations
                .parameters
                .members
                .push(summon_parameter("execution-sentinel"));
        }
    }
    f.schema.slots.push(SlotDescriptor::Parameter(known(
        summon_parameter("execution-sentinel"),
        ParameterSlotSchema {
            value: ValueSchema::Integer(range()),
            presence: SlotPresence::RequiredOnce,
            sites: vec![],
        },
    )));
    if produced {
        let root = program_mut(f, physical_owner(), "supply-summon");
        root.effects.push(effect(
            "sentinel",
            RuleEffectKind::ProjectSkillParameter {
                skill: summon_supply(),
                parameter: summon_parameter("execution-sentinel"),
                value: key("level"),
            },
        ));
    }
}

pub fn classify_ancestor_execution(stages: &mut EvaluationStagesInput) {
    stages
        .readiness
        .as_mut()
        .unwrap()
        .skills
        .iter_mut()
        .find(|row| row.skill == def::<SkillDefinition>("summon"))
        .unwrap()
        .parameters
        .members
        .push(ParameterReadiness {
            parameter: summon_parameter("execution-sentinel"),
            phase: ReadinessPhase::Execution,
        });
}
