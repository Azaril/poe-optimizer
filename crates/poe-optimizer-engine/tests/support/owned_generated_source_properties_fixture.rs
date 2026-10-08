//! Generated sources reuse the complete source-property fixture and its raw slots.
use super::fixture as source;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_readiness::*, owned_rules::*, owned_schema::*,
    owned_source_properties::*, owned_stages::*, owned_support_receiving::*,
};
pub use source::{
    Checked, Effects, Fixture, Inputs, base, def, effect, key, occurrence, readiness, subject,
};

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
pub fn modifier() -> SlotOwnerDefId {
    SlotOwnerDefId::Modifier(def("modifier"))
}
pub fn passive() -> SlotOwnerDefId {
    SlotOwnerDefId::PassiveNode(def("source-node"))
}
pub fn supply(owner: SlotOwnerDefId) -> DeclaredSlot<SkillGrantSlotDefId> {
    DeclaredSlot {
        declaration: owner,
        slot: def("generated-source-supply"),
    }
}
pub fn grant(owner: SlotOwnerDefId) -> DeclaredSlot<GrantSlotDefId> {
    DeclaredSlot {
        declaration: owner,
        slot: def("generated-source-grant"),
    }
}
pub fn target(root: ProviderRoot, owner: SlotOwnerDefId) -> SkillTarget {
    SkillTarget::Generated(Box::new(GeneratedSkillKey {
        provider: ProviderKey {
            root,
            grant_path: vec![],
        },
        slot: supply(owner),
    }))
}
pub fn node_target(id: u64) -> SkillTarget {
    target(ProviderRoot::Allocation(occurrence(id)), passive())
}
pub fn item_target(equipment: u64, modifier_id: u64) -> SkillTarget {
    target(
        ProviderRoot::ItemModifier {
            equipment_use: occurrence(equipment),
            modifier: occurrence(modifier_id),
        },
        modifier(),
    )
}
pub fn final_key(target: SkillTarget) -> poe_optimizer_engine::owned_plan::PlanValueKey {
    let SkillTarget::Generated(skill) = target else {
        panic!("generated final input")
    };
    poe_optimizer_engine::owned_plan::PlanValueKey::SkillParameter {
        skill,
        parameter: source::final_parameter(),
    }
}
pub fn relation(
    skill_supply: DeclaredSlot<SkillGrantSlotDefId>,
    name: &str,
) -> SourcePropertyRelation {
    let mut relation = source::relation();
    relation.id = key(name);
    relation.owner = SupportTargetDefinition::Skill(def("summon"));
    relation.occurrence = SourcePropertyOccurrence::GeneratedSkill { skill_supply };
    relation.effects = DeclaredSet::complete(vec![SourcePropertyEffect {
        endpoint: SourcePropertyEffectEndpoint::OwnerSkill {},
        admission: SupportAdmissionContext::AssignedSkill,
    }]);
    relation.assembly = DeclaredSet::complete(vec![SourcePropertyAssemblyProgram {
        program: key("source-assembly"),
        binding: SourcePropertyAssemblyBinding::ExactSupplyingProvider,
    }]);
    relation
}

pub fn fixture() -> Fixture {
    let mut f = source::fixture();
    f.schema.schema_version = poe_optimizer_data::owned_schema::OWNED_SCHEMA_PACKAGE_V5;
    for slot in &mut f.schema.slots {
        if let SlotDescriptor::Parameter(row) = slot
            && row.id == source::final_parameter()
        {
            let SchemaState::Known(schema) = &mut row.schema else {
                unreachable!()
            };
            schema.skill_input = Some(SkillInputAuthority::Projected);
        }
        if let SlotDescriptor::Grant(row) = slot
            && [
                readiness::actor_grant(),
                readiness::ability_grant("first"),
                readiness::ability_grant("second"),
            ]
            .contains(&row.id)
        {
            let SchemaState::Known(schema) = &mut row.schema else {
                unreachable!()
            };
            // The root role remains stable through every generated descendant.
            schema
                .provider_roles
                .extend([ProviderRole::ItemModifier, ProviderRole::Allocation]);
        }
    }
    // Existing Gem parents now supply generated property owners. Their two
    // retained support sequences remain independent, with the same raw inputs.
    for assignment in &mut f.build.supports {
        let SkillTarget::Authored(id) = assignment.target else {
            unreachable!()
        };
        assignment.target = SkillTarget::Generated(Box::new(GeneratedSkillKey {
            provider: ProviderKey {
                root: ProviderRoot::SkillUse(id),
                grant_path: vec![],
            },
            slot: readiness::summon_supply(),
        }));
    }
    for sequence in f.build.authored_support_order.as_mut().unwrap() {
        let SkillTarget::Authored(id) = sequence.target else {
            unreachable!()
        };
        sequence.target = SkillTarget::Generated(Box::new(GeneratedSkillKey {
            provider: ProviderKey {
                root: ProviderRoot::SkillUse(id),
                grant_path: vec![],
            },
            slot: readiness::summon_supply(),
        }));
    }
    let original = Fixture::new();
    f.build.items = original.build.items;
    f.build.equipment = original.build.equipment;
    f.schema.definitions.extend([
        DefinitionDescriptor::PointPool(known(
            def("source-pool"),
            PointPoolSchema {
                scope: PointPoolScope::Either,
            },
        )),
        DefinitionDescriptor::PassiveNode(known(
            def("source-node"),
            PassiveNodeSchema {
                pools: DeclaredSet::complete(vec![def("source-pool")]),
                adjacent: DeclaredSet::complete(vec![]),
                declarations: ports(),
            },
        )),
    ]);
    for id in [80, 81] {
        f.build.allocations.push(Allocation {
            id: occurrence(id),
            node: def("source-node"),
            pool: def("source-pool"),
            scope: LoadoutScope::Shared,
            access: AllocationAccess::Ordinary,
            choices: vec![],
        });
    }
    for owner in [modifier(), passive()] {
        for descriptor in &mut f.schema.definitions {
            let declarations = match descriptor {
                DefinitionDescriptor::Modifier(row)
                    if owner == SlotOwnerDefId::Modifier(row.id.clone()) =>
                {
                    let SchemaState::Known(schema) = &mut row.schema else {
                        unreachable!()
                    };
                    Some(&mut schema.declarations)
                }
                DefinitionDescriptor::PassiveNode(row)
                    if owner == SlotOwnerDefId::PassiveNode(row.id.clone()) =>
                {
                    let SchemaState::Known(schema) = &mut row.schema else {
                        unreachable!()
                    };
                    Some(&mut schema.declarations)
                }
                _ => None,
            };
            if let Some(declarations) = declarations {
                declarations.grants.members.push(grant(owner.clone()));
                declarations
                    .skill_grants
                    .members
                    .push(supply(owner.clone()));
            }
        }
        let (owner_subject, context, role, raw) = match &owner {
            SlotOwnerDefId::Modifier(id) => (
                subject(id.clone()),
                RuleEntityKind::EquipmentUse,
                ProviderRole::ItemModifier,
                RuleReadSource::Parameter {
                    slot: base::parameter(owner.clone(), "roll"),
                },
            ),
            SlotOwnerDefId::PassiveNode(id) => (
                subject(id.clone()),
                RuleEntityKind::Actor,
                ProviderRole::Allocation,
                RuleReadSource::CharacterLevel,
            ),
            _ => unreachable!(),
        };
        f.schema.slots.extend([
            SlotDescriptor::SkillGrant(known(
                supply(owner.clone()),
                SkillGrantSlotSchema {
                    preset_inputs: None,
                    skill: def("summon"),
                    outputs: DeclaredSet::complete(vec![source::output()]),
                },
            )),
            SlotDescriptor::Grant(known(
                grant(owner.clone()),
                GrantSlotSchema {
                    provider_roles: vec![role],
                    target: GrantTarget::Skill(supply(owner.clone())),
                },
            )),
        ]);
        let supply_program = RuleProgram {
            id: key("supply-summon"),
            context,
            reads: vec![base::read("raw", raw)],
            nodes: vec![
                base::read_node("raw", "raw"),
                base::bool_node("active", true),
            ],
            effects: vec![
                effect(
                    "grant",
                    RuleEffectKind::ActivateGrant {
                        slot: grant(owner.clone()),
                        enabled: key("active"),
                    },
                ),
                effect(
                    "raw",
                    RuleEffectKind::ProjectSkillParameter {
                        skill: supply(owner.clone()),
                        parameter: readiness::summon_parameter("level"),
                        value: key("raw"),
                    },
                ),
                effect(
                    "enabled",
                    RuleEffectKind::ProjectSkillParameter {
                        skill: supply(owner.clone()),
                        parameter: readiness::summon_parameter("enabled"),
                        value: key("active"),
                    },
                ),
            ],
        };
        let mut assembly =
            readiness::program_mut(&mut f, readiness::physical_owner(), "source-assembly").clone();
        assembly.context = context;
        // Natural parent reads distinguish modifier rolls from child parameters,
        // while the independent property reduction stays on the exact source.
        assembly.reads[0].source = match &owner {
            SlotOwnerDefId::Modifier(_) => RuleReadSource::Parameter {
                slot: base::parameter(owner.clone(), "roll"),
            },
            SlotOwnerDefId::PassiveNode(_) => RuleReadSource::Stat {
                entity: RuleEntity::Current,
                stat: def("external-base"),
            },
            _ => unreachable!(),
        };
        assembly.effects[0].effect = RuleEffectKind::ProjectSkillParameter {
            skill: supply(owner.clone()),
            parameter: source::final_parameter(),
            value: key("final"),
        };
        if let Some(row) = f.owners.iter_mut().find(|row| row.owner == owner_subject) {
            row.programs.members.extend([supply_program, assembly]);
        } else {
            f.owners.push(DefinitionRules {
                owner: owner_subject,
                programs: DeclaredSet::complete(vec![supply_program, assembly]),
            });
        }
        f.owners.push(DefinitionRules {
            owner: SchemaSubject::Slot(GrantSlotDefId::address(&grant(owner))),
            programs: DeclaredSet::complete(vec![]),
        });
    }
    f
}

pub fn inputs_with(
    f: &Fixture,
    edit_rules: impl FnOnce(&mut RulePackageInput),
    edit_stages: impl FnOnce(&mut EvaluationStagesInput),
    edit_receiving: impl FnOnce(&mut SupportReceivingInput),
) -> Checked<Inputs> {
    inputs_with_operations(
        f,
        OWNED_RULE_OPERATIONS_V18,
        edit_rules,
        edit_stages,
        edit_receiving,
    )
}
pub fn inputs_with_operations(
    f: &Fixture,
    operations: &str,
    edit_rules: impl FnOnce(&mut RulePackageInput),
    edit_stages: impl FnOnce(&mut EvaluationStagesInput),
    edit_receiving: impl FnOnce(&mut SupportReceivingInput),
) -> Checked<Inputs> {
    source::inputs_with_operations(f, operations, edit_rules, edit_stages, |receiving| {
        receiving.source_properties.as_mut().unwrap().relations = DeclaredSet::complete(vec![
            relation(readiness::summon_supply(), "generated-gem"),
            relation(supply(modifier()), "generated-item"),
            relation(supply(passive()), "generated-node"),
        ]);
        edit_receiving(receiving);
    })
}
pub fn inputs(f: &Fixture) -> Checked<Inputs> {
    inputs_with(f, |_| {}, |_| {}, |_| {})
}
pub fn compile(f: &Fixture) -> Checked<Effects> {
    readiness::compile_inputs(inputs(f)?)
}

pub fn add_shared_final_stat(f: &mut Fixture) {
    f.schema.definitions.push(DefinitionDescriptor::Stat(known(
        def("direct-final"),
        StatSchema {
            value: ComputedValueType::Integer,
            targets: vec![RuleEntityKind::Skill],
        },
    )));
    let assembly = RuleProgram {
        id: key("source-direct-assembly"),
        context: RuleEntityKind::Skill,
        reads: vec![
            base::read(
                "raw",
                RuleReadSource::Parameter {
                    slot: readiness::summon_parameter("level"),
                },
            ),
            base::contributions("properties", RuleEntity::PropertyOwner, "source-add"),
        ],
        nodes: vec![
            base::read_node("raw", "raw"),
            base::read_node("properties", "properties"),
            base::node(
                "final",
                RuleExpression::Add {
                    left: key("raw"),
                    right: key("properties"),
                },
            ),
        ],
        effects: vec![base::derive(
            "final",
            RuleEntity::Current,
            "direct-final",
            "final",
        )],
    };
    f.owner_mut(&readiness::summon_owner())
        .programs
        .members
        .push(assembly);
    readiness::program_mut(f, readiness::summon_owner(), "source-value").reads[0].source =
        RuleReadSource::Stat {
            entity: RuleEntity::Current,
            stat: def("direct-final"),
        };
    for owner in &mut f.owners {
        for program in &mut owner.programs.members {
            if program.id != key("source-assembly") {
                continue;
            }
            program.reads = vec![base::read(
                "final",
                RuleReadSource::Stat {
                    entity: RuleEntity::PropertyOwner,
                    stat: def("direct-final"),
                },
            )];
            program.nodes = vec![base::read_node("final", "final")];
        }
    }
}
pub fn shared_final_inputs(f: &Fixture) -> Checked<Inputs> {
    inputs_with(
        f,
        |_| {},
        |_| {},
        |receiving| {
            let relations = &mut receiving
                .source_properties
                .as_mut()
                .unwrap()
                .relations
                .members;
            for relation in relations.iter_mut() {
                relation.inputs.push(def("direct-final"));
                relation
                    .assembly
                    .members
                    .push(SourcePropertyAssemblyProgram {
                        program: key("source-direct-assembly"),
                        binding: SourcePropertyAssemblyBinding::InputOwner,
                    });
            }
            let mut manual = relations[0].clone();
            manual.id = key("manual-shared-source");
            manual.occurrence = SourcePropertyOccurrence::AuthoredSkillUse {};
            manual
                .assembly
                .members
                .retain(|program| program.binding == SourcePropertyAssemblyBinding::InputOwner);
            relations.push(manual);
        },
    )
}

pub fn add_skill_parent(f: &mut Fixture) {
    let owner = SlotOwnerDefId::Skill(def("skill"));
    for definition in &mut f.schema.definitions {
        if let DefinitionDescriptor::Skill(row) = definition
            && row.id == def::<SkillDefinition>("skill")
        {
            let SchemaState::Known(schema) = &mut row.schema else {
                unreachable!()
            };
            schema
                .declarations
                .parameters
                .members
                .push(source::direct_parameter());
            schema
                .declarations
                .grants
                .members
                .push(grant(owner.clone()));
            schema
                .declarations
                .skill_grants
                .members
                .push(supply(owner.clone()));
        }
    }
    f.schema.slots.extend([
        SlotDescriptor::Parameter(known(
            source::direct_parameter(),
            ParameterSlotSchema {
                skill_input: Some(SkillInputAuthority::Authored),
                value: ValueSchema::Integer(IntegerRange {
                    minimum: BoundedInteger::new(0).unwrap(),
                    maximum: BoundedInteger::new(100).unwrap(),
                }),
                presence: SlotPresence::RequiredOnce,
                sites: vec![ParameterSite::SkillParameter],
            },
        )),
        SlotDescriptor::SkillGrant(known(
            supply(owner.clone()),
            SkillGrantSlotSchema {
                preset_inputs: None,
                skill: def("summon"),
                outputs: DeclaredSet::complete(vec![source::output()]),
            },
        )),
        SlotDescriptor::Grant(known(
            grant(owner.clone()),
            GrantSlotSchema {
                provider_roles: vec![ProviderRole::SkillUse],
                target: GrantTarget::Skill(supply(owner.clone())),
            },
        )),
    ]);
    let modifier_owner = base::modifier_owner();
    let mut programs: Vec<_> = f
        .owner_mut(&modifier_owner)
        .programs
        .members
        .iter()
        .filter(|p| p.id == key("supply-summon") || p.id == key("source-assembly"))
        .cloned()
        .collect();
    for program in &mut programs {
        program.context = RuleEntityKind::Skill;
        program.reads[0].source = RuleReadSource::Parameter {
            slot: source::direct_parameter(),
        };
        for effect in &mut program.effects {
            match &mut effect.effect {
                RuleEffectKind::ActivateGrant { slot, .. } => *slot = grant(owner.clone()),
                RuleEffectKind::ProjectSkillParameter { skill, .. } => {
                    *skill = supply(owner.clone())
                }
                _ => unreachable!(),
            }
        }
    }
    f.owner_mut(&subject(def::<SkillDefinition>("skill")))
        .programs
        .members
        .extend(programs);
    f.owners.push(DefinitionRules {
        owner: SchemaSubject::Slot(GrantSlotDefId::address(&grant(owner))),
        programs: DeclaredSet::complete(vec![]),
    });
    f.build.skills.push(SkillUse {
        id: occurrence(90),
        source: AuthoredSkillSource::Direct(def("skill")),
        enabled: true,
        scope: LoadoutScope::Shared,
        parameters: Some(vec![ParameterAssignment {
            slot: source::direct_parameter(),
            value: base::integer(14),
        }]),
    });
}

pub fn participation_inputs(f: &mut Fixture) -> Checked<Inputs> {
    let parameter = base::parameter(
        SlotOwnerDefId::UsagePolicy(def("participation-policy")),
        "requested",
    );
    f.schema.definitions.extend([
        DefinitionDescriptor::Stat(known(
            def("requested-participation"),
            StatSchema {
                value: ComputedValueType::Boolean,
                targets: vec![RuleEntityKind::Skill],
            },
        )),
        DefinitionDescriptor::UsagePolicy(known(
            def("participation-policy"),
            UsagePolicySchema {
                targets: vec![UsageTargetKind::Skill],
                declarations: DeclaredSlots {
                    parameters: DeclaredSet::complete(vec![parameter.clone()]),
                    ..ports()
                },
            },
        )),
    ]);
    f.schema.slots.push(SlotDescriptor::Parameter(known(
        parameter.clone(),
        ParameterSlotSchema {
            skill_input: None,
            value: ValueSchema::Boolean,
            presence: SlotPresence::OptionalOnce,
            sites: vec![ParameterSite::UsagePolicyParameter],
        },
    )));
    f.owners.push(DefinitionRules {
        owner: subject(def::<UsagePolicyDefinition>("participation-policy")),
        programs: DeclaredSet::complete(vec![RuleProgram {
            id: key("requested-usage"),
            context: RuleEntityKind::Skill,
            reads: vec![RuleRead {
                id: key("requested"),
                value_type: ComputedValueType::Boolean,
                source: RuleReadSource::Parameter {
                    slot: parameter.clone(),
                },
            }],
            nodes: vec![base::read_node("requested", "requested")],
            effects: vec![base::derive(
                "requested",
                RuleEntity::Current,
                "requested-participation",
                "requested",
            )],
        }]),
    });
    for target in [
        source::member(30),
        source::member(31),
        item_target(6, 4),
        item_target(6, 5),
        item_target(7, 4),
        item_target(7, 5),
        node_target(80),
        node_target(81),
    ] {
        let requested = target != node_target(80);
        f.scenario.usage.push(UsagePolicySelection {
            policy: def("participation-policy"),
            target: UsageTarget::Skill(target),
            parameters: vec![ParameterAssignment {
                slot: parameter.clone(),
                value: ParameterValue::Boolean(requested),
            }],
        });
    }
    inputs_with_operations(
        f,
        OWNED_RULE_OPERATIONS_V21,
        |rules| rules.contribution_queries = Some(DeclaredSet::complete(vec![])),
        |stages| {
            stages.schema_version = 4;
            let readiness = stages.readiness.as_mut().unwrap();
            readiness
                .skills
                .iter_mut()
                .find(|row| row.skill == def::<SkillDefinition>("summon"))
                .unwrap()
                .participation = Some(def("requested-participation"));
            for row in &mut readiness.programs.members {
                if row.program != key("requested-usage") {
                    continue;
                }
                row.phase = ReadinessPhase::Preparation;
                row.role = ReadinessProgramRole::PreparationFacts;
                row.outputs = vec![StageChannel::Stat {
                    scope: RuleEntityKind::Skill,
                    stat: def("requested-participation"),
                }];
                stages
                    .programs
                    .members
                    .iter_mut()
                    .find(|p| p.owner == row.owner && p.program == row.program)
                    .unwrap()
                    .stage = key("prepare");
            }
        },
        |_| {},
    )
}
