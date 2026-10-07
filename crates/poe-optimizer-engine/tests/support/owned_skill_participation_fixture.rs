//! Finite synthetic usage/ancestry fixture over the public staged support engine.
//! No game definitions or imported enabled-field semantics are asserted here.
#![allow(dead_code)]
#[path = "owned_source_properties_fixture.rs"]
pub mod sources;
pub use delivery::fixture as base;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_readiness::*, owned_rules::*, owned_schema::*,
    owned_source_properties::*, owned_stages::*, owned_support_receiving::*,
};
use poe_optimizer_engine::owned_plan::*;
pub use readiness::delivery;
pub use readiness::{Checked, Effects, Fixture, Inputs, def, effect, key, occurrence, subject};
pub use sources::readiness;

pub fn known<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
pub fn ports() -> DeclaredSlots {
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
pub fn policy_owner() -> SchemaSubject {
    subject(def::<UsagePolicyDefinition>("participation-policy"))
}
pub fn parameter() -> DeclaredSlot<ParameterSlotDefId> {
    base::parameter(
        SlotOwnerDefId::UsagePolicy(def("participation-policy")),
        "requested",
    )
}
pub fn install(f: &mut Fixture) {
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
                    parameters: DeclaredSet::complete(vec![parameter()]),
                    ..ports()
                },
            },
        )),
    ]);
    f.schema.slots.push(SlotDescriptor::Parameter(known(
        parameter(),
        ParameterSlotSchema {
            skill_input: None,
            value: ValueSchema::Boolean,
            presence: SlotPresence::OptionalOnce,
            sites: vec![ParameterSite::UsagePolicyParameter],
        },
    )));
    f.owners.push(DefinitionRules {
        owner: policy_owner(),
        programs: DeclaredSet::complete(vec![RuleProgram {
            id: key("requested-usage"),
            context: RuleEntityKind::Skill,
            reads: vec![RuleRead {
                id: key("requested"),
                value_type: ComputedValueType::Boolean,
                source: RuleReadSource::Parameter { slot: parameter() },
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
    for id in [30, 31] {
        f.scenario
            .usage
            .push(usage(sources::member(id), Some(true)));
        for name in ["first", "second"] {
            f.scenario
                .usage
                .push(usage(readiness::target(id, name), Some(true)));
        }
    }
}
pub fn fixture() -> Fixture {
    let mut f = readiness::fixture();
    install(&mut f);
    f
}
pub fn usage(target: SkillTarget, requested: Option<bool>) -> UsagePolicySelection {
    UsagePolicySelection {
        policy: def("participation-policy"),
        target: UsageTarget::Skill(target),
        parameters: requested
            .map(|v| ParameterAssignment {
                slot: parameter(),
                value: ParameterValue::Boolean(v),
            })
            .into_iter()
            .collect(),
    }
}
pub fn set(f: &mut Fixture, target: SkillTarget, value: Option<bool>) {
    let row = f
        .scenario
        .usage
        .iter_mut()
        .find(|row| row.target == UsageTarget::Skill(target.clone()))
        .unwrap();
    *row = usage(target, value);
}
pub fn configure_stages(stages: &mut EvaluationStagesInput) {
    stages.schema_version = 4;
    for row in &mut stages.readiness.as_mut().unwrap().skills {
        row.participation = Some(def("requested-participation"));
    }
    for row in &mut stages.readiness.as_mut().unwrap().programs.members {
        if row.program == key("requested-usage") || row.program == key("requested-duplicate") {
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
    }
}
pub fn inputs_with(
    f: &Fixture,
    edit_rules: impl FnOnce(&mut RulePackageInput),
    edit_stages: impl FnOnce(&mut EvaluationStagesInput),
    edit_receiving: impl FnOnce(&mut SupportReceivingInput),
) -> Checked<Inputs> {
    readiness::inputs_with_operations(
        f,
        false,
        OWNED_RULE_OPERATIONS_V21,
        |rules| {
            rules.contribution_queries = Some(DeclaredSet::complete(vec![]));
            edit_rules(rules);
        },
        |s| {
            configure_stages(s);
            edit_stages(s);
        },
        |r| {
            r.schema_version = 3;
            r.source_properties = Some(SourcePropertyPreparationInput {
                relations: DeclaredSet::complete(vec![]),
            });
            edit_receiving(r);
        },
    )
}
pub fn inputs(f: &Fixture) -> Checked<Inputs> {
    inputs_with(f, |_| {}, |_| {}, |_| {})
}
pub fn compile(f: &Fixture) -> Checked<Effects> {
    readiness::compile_inputs(inputs(f)?)
}
pub fn evaluate(f: &Fixture) -> SupportEffectsReport {
    let p = compile(f).unwrap();
    p.evaluate(&mut p.new_scratch()).unwrap()
}
pub fn activation<'a>(r: &'a OwnedEffectsReport, t: &SkillTarget) -> &'a EffectValue {
    &r.values
        .iter()
        .find(|v| {
            v.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Skill(Box::new(t.clone())),
                    stat: def("requested-participation"),
                }
        })
        .unwrap()
        .value
}
pub fn final_level<'a>(r: &'a OwnedEffectsReport, id: u64, name: &str) -> &'a EffectValue {
    let SkillTarget::Generated(skill) = readiness::target(id, name) else {
        unreachable!()
    };
    &r.values
        .iter()
        .find(|v| {
            v.key
                == PlanValueKey::SkillParameter {
                    skill: skill.clone(),
                    parameter: readiness::ability_parameter("level"),
                }
        })
        .unwrap()
        .value
}
pub fn bool_value(v: bool) -> EffectValue {
    EffectValue::Known {
        value: ParameterValue::Boolean(v),
    }
}
pub fn rejected<T>(r: Checked<T>, needle: &str) {
    let Err(e) = r else {
        panic!("expected rejection containing {needle}")
    };
    assert!(e.contains(needle), "expected {needle:?}, got {e:?}");
}
pub fn add_application(f: &mut Fixture) {
    f.schema.definitions.push(DefinitionDescriptor::Stat(known(
        def("participation-buff"),
        StatSchema {
            value: ComputedValueType::Integer,
            targets: vec![RuleEntityKind::Actor],
        },
    )));
}
pub fn application() -> EffectApplicationRule {
    EffectApplicationRule {
        id: key("finite-participation-buff"),
        source: EffectApplicationSource::Skill {
            skill: def("ability"),
        },
        targets: vec![EffectApplicationTarget::OwnedSlot {
            slot: readiness::actor_slot(),
        }],
        activation: key("active"),
        program: RuleProgram {
            id: key("buff"),
            context: RuleEntityKind::Actor,
            reads: vec![],
            nodes: vec![base::bool_node("active", true), base::literal("value", 5)],
            effects: vec![effect(
                "buff",
                RuleEffectKind::Contribute {
                    entity: RuleEntity::Current,
                    stat: def("participation-buff"),
                    contribution: ContributionKind::Add,
                    value: key("value"),
                },
            )],
        },
        stacking: vec![EffectStackingRule {
            effect: key("buff"),
            family: key("finite-participation"),
            modifier: key("buff"),
            reduction: EffectStackingReduction::Maximum,
        }],
    }
}
pub fn application_inputs(f: &Fixture) -> Checked<Inputs> {
    inputs_with(
        f,
        |r| r.effect_applications = Some(DeclaredSet::complete(vec![application()])),
        |s| {
            s.effect_applications = Some(DeclaredSet::complete(vec![StagedEffectApplication {
                application: key("finite-participation-buff"),
                stage: key("execute"),
            }]));
        },
        |_| {},
    )
}

pub fn root_slot(owner: SlotOwnerDefId) -> DeclaredSlot<SkillGrantSlotDefId> {
    DeclaredSlot {
        declaration: owner,
        slot: def("requested-root"),
    }
}
pub fn root_grant(owner: SlotOwnerDefId) -> DeclaredSlot<GrantSlotDefId> {
    DeclaredSlot {
        declaration: owner,
        slot: def("requested-root-grant"),
    }
}
pub fn root_target(root: ProviderRoot, owner: SlotOwnerDefId) -> SkillTarget {
    SkillTarget::Generated(Box::new(GeneratedSkillKey {
        provider: ProviderKey {
            root,
            grant_path: vec![],
        },
        slot: root_slot(owner),
    }))
}
pub fn root_targets() -> Vec<SkillTarget> {
    vec![
        root_target(
            ProviderRoot::EquipmentUse(occurrence(6)),
            SlotOwnerDefId::ItemTemplate(def("item")),
        ),
        root_target(
            ProviderRoot::EquipmentUse(occurrence(7)),
            SlotOwnerDefId::ItemTemplate(def("item")),
        ),
        root_target(
            ProviderRoot::Allocation(occurrence(80)),
            SlotOwnerDefId::PassiveNode(def("participation-node")),
        ),
        SkillTarget::Authored(occurrence(81)),
        SkillTarget::Authored(occurrence(82)),
    ]
}
pub fn roots_fixture() -> Fixture {
    let mut f = fixture();
    let original = Fixture::new();
    f.build.items = original.build.items;
    f.build.equipment = original.build.equipment;
    f.schema.definitions.extend([
        DefinitionDescriptor::Stat(known(
            def("root-result"),
            StatSchema {
                value: ComputedValueType::Integer,
                targets: vec![RuleEntityKind::Skill],
            },
        )),
        DefinitionDescriptor::PointPool(known(
            def("participation-pool"),
            PointPoolSchema {
                scope: PointPoolScope::Either,
            },
        )),
        DefinitionDescriptor::PassiveNode(known(
            def("participation-node"),
            PassiveNodeSchema {
                pools: DeclaredSet::complete(vec![def("participation-pool")]),
                adjacent: DeclaredSet::complete(vec![]),
                declarations: ports(),
            },
        )),
    ]);
    f.build.allocations.push(Allocation {
        id: occurrence(80),
        node: def("participation-node"),
        pool: def("participation-pool"),
        scope: LoadoutScope::Shared,
        access: AllocationAccess::Ordinary,
        choices: vec![],
    });
    for id in [81, 82] {
        f.build.skills.push(SkillUse {
            id: occurrence(id),
            source: AuthoredSkillSource::Direct(def("skill")),
            enabled: true,
            scope: LoadoutScope::Shared,
            parameters: None,
        });
    }
    for owner in [
        SlotOwnerDefId::ItemTemplate(def("item")),
        SlotOwnerDefId::PassiveNode(def("participation-node")),
    ] {
        for row in &mut f.schema.definitions {
            let declarations = match row {
                DefinitionDescriptor::ItemTemplate(e)
                    if owner == SlotOwnerDefId::ItemTemplate(e.id.clone()) =>
                {
                    let SchemaState::Known(s) = &mut e.schema else {
                        unreachable!()
                    };
                    Some(&mut s.declarations)
                }
                DefinitionDescriptor::PassiveNode(e)
                    if owner == SlotOwnerDefId::PassiveNode(e.id.clone()) =>
                {
                    let SchemaState::Known(s) = &mut e.schema else {
                        unreachable!()
                    };
                    Some(&mut s.declarations)
                }
                _ => None,
            };
            if let Some(d) = declarations {
                d.grants.members.push(root_grant(owner.clone()));
                d.skill_grants.members.push(root_slot(owner.clone()));
            }
        }
        let (subject, context, role) = match &owner {
            SlotOwnerDefId::ItemTemplate(d) => (
                subject(d.clone()),
                RuleEntityKind::EquipmentUse,
                ProviderRole::EquipmentUse,
            ),
            SlotOwnerDefId::PassiveNode(d) => (
                subject(d.clone()),
                RuleEntityKind::Actor,
                ProviderRole::Allocation,
            ),
            _ => unreachable!(),
        };
        f.schema.slots.extend([
            SlotDescriptor::SkillGrant(known(
                root_slot(owner.clone()),
                SkillGrantSlotSchema {
                    preset_inputs: None,
                    skill: def("skill"),
                    outputs: DeclaredSet::complete(vec![base::output()]),
                },
            )),
            SlotDescriptor::Grant(known(
                root_grant(owner.clone()),
                GrantSlotSchema {
                    provider_roles: vec![role],
                    target: GrantTarget::Skill(root_slot(owner.clone())),
                },
            )),
        ]);
        let program = RuleProgram {
            id: key("supply-requested-root"),
            context,
            reads: vec![],
            nodes: vec![base::bool_node("supplied", true)],
            effects: vec![effect(
                "supply",
                RuleEffectKind::ActivateGrant {
                    slot: root_grant(owner.clone()),
                    enabled: key("supplied"),
                },
            )],
        };
        if let Some(row) = f.owners.iter_mut().find(|r| r.owner == subject) {
            row.programs.members.push(program);
        } else {
            f.owners.push(DefinitionRules {
                owner: subject,
                programs: DeclaredSet::complete(vec![program]),
            });
        }
        f.owners.push(DefinitionRules {
            owner: SchemaSubject::Slot(GrantSlotDefId::address(&root_grant(owner))),
            programs: DeclaredSet::complete(vec![]),
        });
    }
    f.owner_mut(&subject(def::<SkillDefinition>("skill")))
        .programs
        .members
        .push(RuleProgram {
            id: key("root-execution"),
            context: RuleEntityKind::Skill,
            reads: vec![],
            nodes: vec![base::literal("result", 9)],
            effects: vec![base::derive(
                "result",
                RuleEntity::Current,
                "root-result",
                "result",
            )],
        });
    f.routes
        .push(poe_optimizer_core::owned_routing::ActionOutputRoutes {
            output: base::output(),
            source_selectors: Some(DeclaredSet::complete(vec![])),
            routes: DeclaredSet::complete(vec![]),
        });
    f.scenario
        .usage
        .extend(root_targets().into_iter().map(|t| usage(t, Some(true))));
    f
}
pub fn roots_inputs(f: &Fixture) -> Checked<Inputs> {
    inputs_with(
        f,
        |_| {},
        |s| {
            let readiness = s.readiness.as_mut().unwrap();
            readiness.skills.push(SkillReadiness {
                skill: def("skill"),
                parameters: DeclaredSet::complete(vec![]),
                participation: Some(def("requested-participation")),
            });
            for row in &mut readiness.programs.members {
                if row.program != key("supply-requested-root") {
                    continue;
                }
                let owner = match &row.owner {
                    SchemaSubject::Definition(DefinitionAddress::ItemTemplate(d)) => {
                        SlotOwnerDefId::ItemTemplate(d.clone())
                    }
                    SchemaSubject::Definition(DefinitionAddress::PassiveNode(d)) => {
                        SlotOwnerDefId::PassiveNode(d.clone())
                    }
                    _ => unreachable!(),
                };
                row.phase = ReadinessPhase::Structural;
                row.role = ReadinessProgramRole::PreparationFacts;
                row.outputs = vec![StageChannel::Grant {
                    slot: root_grant(owner),
                }];
                s.programs
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
pub fn root_value<'a>(r: &'a OwnedEffectsReport, t: &SkillTarget) -> Option<&'a EffectValue> {
    r.values
        .iter()
        .find(|v| {
            v.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Skill(Box::new(t.clone())),
                    stat: def("root-result"),
                }
        })
        .map(|r| &r.value)
}
