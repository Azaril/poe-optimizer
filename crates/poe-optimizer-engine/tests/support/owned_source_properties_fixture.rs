//! Synthetic source relations reuse the existing complete readiness graph.
#[allow(dead_code, unused_imports)]
#[path = "owned_preparation_readiness_fixture.rs"]
pub mod readiness;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_readiness::*, owned_routing::*, owned_rules::*,
    owned_schema::*, owned_source_properties::*, owned_stages::*, owned_support_receiving::*,
};
use poe_optimizer_engine::owned_plan::*;
pub use readiness::delivery::fixture as base;
pub use readiness::{
    Checked, Effects, Fixture, Inputs, Metrics, def, effect, key, occurrence, subject,
};
use std::sync::Arc;

pub fn owner(id: u64) -> SkillTarget {
    SkillTarget::Authored(occurrence(id))
}
pub fn member(id: u64) -> SkillTarget {
    SkillTarget::Generated(Box::new(GeneratedSkillKey {
        provider: ProviderKey {
            root: ProviderRoot::SkillUse(occurrence(id)),
            grant_path: vec![],
        },
        slot: readiness::summon_supply(),
    }))
}
pub fn final_parameter() -> DeclaredSlot<ParameterSlotDefId> {
    readiness::summon_parameter("source-final")
}
pub fn output() -> DeclaredSlot<ActionOutputDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(def("summon")),
        slot: def("source-output"),
    }
}
pub fn action(id: u64, set: &str) -> ActionSelection {
    ActionSelection {
        action: ActionKey {
            actor: ActorKey::Player,
            provider: readiness::summon_provider(id),
            output: output(),
        },
        part: def("part"),
        mode: def("mode"),
        stat_set: def(set),
    }
}
fn known<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn integer_stat(name: &str, target: RuleEntityKind) -> DefinitionDescriptor {
    DefinitionDescriptor::Stat(known(
        def(name),
        StatSchema {
            value: ComputedValueType::Integer,
            targets: vec![target],
        },
    ))
}
fn contribute(name: &str, read: RuleReadSource, context: RuleEntityKind) -> RuleProgram {
    RuleProgram {
        id: key(name),
        context,
        reads: vec![base::read("input", read)],
        nodes: vec![base::read_node("value", "input")],
        effects: vec![effect(
            "property",
            RuleEffectKind::Contribute {
                entity: RuleEntity::PropertyOwner,
                stat: def("source-add"),
                contribution: ContributionKind::Add,
                value: key("value"),
            },
        )],
    }
}
pub fn fixture() -> Fixture {
    let mut f = readiness::fixture();
    f.schema.definitions.extend([
        integer_stat("source-add", RuleEntityKind::Skill),
        integer_stat("source-count", RuleEntityKind::Skill),
        integer_stat("external-base", RuleEntityKind::Actor),
        DefinitionDescriptor::ActionStatSet(known(def("source-second"), ActionStatSetSchema {})),
    ]);
    for descriptor in &mut f.schema.definitions {
        match descriptor {
            DefinitionDescriptor::Skill(row) if row.id == def::<SkillDefinition>("summon") => {
                let SchemaState::Known(schema) = &mut row.schema else {
                    unreachable!()
                };
                schema
                    .declarations
                    .parameters
                    .members
                    .push(final_parameter());
                schema.declarations.outputs.members.push(output());
            }
            DefinitionDescriptor::Metric(row) if row.id == def::<MetricDefinition>("requested") => {
                let SchemaState::Known(schema) = &mut row.schema else {
                    unreachable!()
                };
                schema.actor_roles = vec![MetricActorRole::Player];
            }
            _ => {}
        }
    }
    for slot in &mut f.schema.slots {
        if let SlotDescriptor::SkillGrant(row) = slot
            && row.id == readiness::summon_supply()
        {
            let SchemaState::Known(schema) = &mut row.schema else {
                unreachable!()
            };
            schema.outputs.members.push(output());
        }
    }
    f.schema.slots.extend([
        SlotDescriptor::Parameter(known(
            final_parameter(),
            ParameterSlotSchema {
                skill_input: None,
                value: ValueSchema::Integer(IntegerRange {
                    minimum: BoundedInteger::new(0).unwrap(),
                    maximum: BoundedInteger::new(100).unwrap(),
                }),
                presence: SlotPresence::RequiredOnce,
                sites: vec![],
            },
        )),
        SlotDescriptor::ActionOutput(known(
            output(),
            ActionOutputSchema {
                actor_role: DeclaredActorRole::Player,
                parts: DeclaredSet::complete(vec![def("part")]),
                modes: DeclaredSet::complete(vec![def("mode")]),
                stat_sets: DeclaredSet::complete(vec![def("set"), def("source-second")]),
                choices: DeclaredSet::complete(vec![]),
            },
        )),
    ]);
    f.routes.push(ActionOutputRoutes {
        output: output(),
        routes: DeclaredSet::complete(vec![]),
        source_selectors: Some(DeclaredSet::complete(vec![])),
    });

    // Each physical input owner and each supplied effect has its own early facts.
    let facts = readiness::program_mut(&mut f, readiness::child_owner(), "target-facts").clone();
    let mut root_facts = facts.clone();
    root_facts.id = key("target-facts-source");
    root_facts.reads[0].source = RuleReadSource::GemLevel;
    f.owner_mut(&readiness::physical_owner())
        .programs
        .members
        .push(root_facts);
    let mut effect_facts = facts;
    effect_facts.id = key("target-facts-summon");
    effect_facts.reads[0].source = RuleReadSource::Parameter {
        slot: readiness::summon_parameter("level"),
    };
    f.owner_mut(&readiness::summon_owner())
        .programs
        .members
        .push(effect_facts);

    f.owner_mut(&base::class_owner()).programs.members.extend([
        RuleProgram {
            id: key("target-facts-player"),
            context: RuleEntityKind::Actor,
            reads: vec![base::read("level", RuleReadSource::CharacterLevel)],
            nodes: vec![base::read_node("level", "level")],
            effects: vec![base::derive(
                "base",
                RuleEntity::Current,
                "external-base",
                "level",
            )],
        },
        contribute(
            "source-external",
            RuleReadSource::Stat {
                entity: RuleEntity::Current,
                stat: def("external-base"),
            },
            RuleEntityKind::Actor,
        ),
    ]);
    f.owner_mut(&readiness::delivery::support_owner())
        .programs
        .members
        .push(contribute(
            "source-supported",
            RuleReadSource::GemLevel,
            RuleEntityKind::SupportOrigin,
        ));
    f.owner_mut(&readiness::physical_owner())
        .programs
        .members
        .push(RuleProgram {
            id: key("source-assembly"),
            context: RuleEntityKind::Actor,
            reads: vec![
                base::read("raw", RuleReadSource::GemLevel),
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
            effects: vec![effect(
                "final",
                RuleEffectKind::ProjectSkillParameter {
                    skill: readiness::summon_supply(),
                    parameter: final_parameter(),
                    value: key("final"),
                },
            )],
        });
    f.owner_mut(&readiness::summon_owner())
        .programs
        .members
        .push(RuleProgram {
            id: key("source-value"),
            context: RuleEntityKind::Skill,
            reads: vec![base::read(
                "level",
                RuleReadSource::Parameter {
                    slot: final_parameter(),
                },
            )],
            nodes: vec![
                base::read_node("level", "level"),
                base::node(
                    "unit",
                    RuleExpression::Literal {
                        value: readiness::quantity(1.0),
                    },
                ),
                base::node(
                    "quantity",
                    RuleExpression::ScaleInteger {
                        value: key("unit"),
                        count: key("level"),
                    },
                ),
            ],
            effects: vec![base::derive(
                "value",
                RuleEntity::Current,
                "readiness-metric",
                "quantity",
            )],
        });
    let action_program = readiness::program_mut(
        &mut f,
        SchemaSubject::Slot(SlotAddress::ActionOutput(readiness::delivery::output())),
        "execution-metric",
    )
    .clone();
    f.owners.push(DefinitionRules {
        owner: SchemaSubject::Slot(SlotAddress::ActionOutput(output())),
        programs: DeclaredSet::complete(vec![action_program]),
    });
    f.build.supports.retain(|assignment| {
        [
            occurrence(60),
            occurrence(61),
            occurrence(64),
            occurrence(65),
        ]
        .contains(&assignment.id)
    });
    for assignment in &mut f.build.supports {
        assignment.target = if [occurrence(60), occurrence(61)].contains(&assignment.id) {
            owner(30)
        } else {
            owner(31)
        };
    }
    f.build.support_origins = Some(vec![
        SupportOriginSequence {
            target: owner(30),
            origins: vec![
                SupportOrigin::Assignment(occurrence(60)),
                SupportOrigin::Assignment(occurrence(61)),
            ],
        },
        SupportOriginSequence {
            target: owner(31),
            origins: vec![
                SupportOrigin::Assignment(occurrence(64)),
                SupportOrigin::Assignment(occurrence(65)),
            ],
        },
    ]);
    f.queries.requests = [30, 31]
        .into_iter()
        .flat_map(|id| {
            ["set", "source-second"]
                .into_iter()
                .map(move |set| MetricRequest {
                    id: QueryId::new(format!("{id}-{set}")).unwrap(),
                    metric: def("requested"),
                    target: MetricTarget::Action(Box::new(action(id, set))),
                })
        })
        .collect();
    f
}
pub fn relation() -> SourcePropertyRelation {
    SourcePropertyRelation {
        id: key("source"),
        owner: SupportTargetDefinition::Gem(def("summoner")),
        occurrence: SourcePropertyOccurrence::AuthoredSkillUseV1,
        aliases: SourcePropertyAliasPolicy::RejectSharedBackingGemV1,
        context: SourcePropertyContext::PlayerScenarioV1,
        census: SourcePropertyCensus::ExactSelectedPositionV1,
        census_stage: key("property-app"),
        effects: DeclaredSet::complete(vec![SourcePropertyEffect {
            endpoint: SourcePropertyEffectEndpoint::Generated {
                path: vec![],
                skill_supply: readiness::summon_supply(),
            },
            admission: SupportAdmissionContext::ReceivingSkill {
                summoner_path: None,
            },
        }]),
        inputs: vec![def("skill-base")],
        channels: DeclaredSet::complete(vec![SourcePropertyChannel {
            stat: def("source-add"),
            contribution: ContributionKind::Add,
        }]),
        external: DeclaredSet::complete(vec![SourcePropertyExternalProgram {
            owner: base::class_owner(),
            program: key("source-external"),
        }]),
        supports: DeclaredSet::complete(vec![SourcePropertySupportPrograms {
            gem: def("support"),
            counted: true,
            programs: DeclaredSet::complete(vec![key("source-supported")]),
        }]),
        assembly: DeclaredSet::complete(vec![key("source-assembly")]),
        non_hidden_count: def("source-count"),
    }
}
pub fn inputs(f: &Fixture) -> Checked<Inputs> {
    inputs_with(f, |_| {}, |_| {}, |_| {})
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
        OWNED_RULE_OPERATIONS_V18,
        edit_rules,
        |stages| {
            stages.schema_version = 3;
            let readiness = stages.readiness.as_mut().unwrap();
            if f.schema.slots.iter().any(|slot| matches!(slot, SlotDescriptor::Parameter(row) if row.id == direct_parameter())) {
            readiness.skills.push(GeneratedSkillReadiness { skill:def("skill"),parameters:DeclaredSet::complete(vec![ParameterReadiness {parameter:direct_parameter(),phase:ReadinessPhase::Structural}]) });
        }
            readiness
                .skills
                .iter_mut()
                .find(|row| row.skill == def::<SkillDefinition>("summon"))
                .unwrap()
                .parameters
                .members
                .push(ParameterReadiness {
                    parameter: final_parameter(),
                    phase: ReadinessPhase::Execution,
                });
            for row in &mut readiness.programs.members {
                let (role, outputs) = match row.program.as_str() {
                    "source-external" => (
                        ReadinessProgramRole::SourceExternalProperty,
                        vec![StageChannel::Contributions {
                            scope: RuleEntityKind::Skill,
                            stat: def("source-add"),
                            contribution: ContributionKind::Add,
                        }],
                    ),
                    "source-supported" => (
                        ReadinessProgramRole::SourceSupportedProperty,
                        vec![StageChannel::Contributions {
                            scope: RuleEntityKind::Skill,
                            stat: def("source-add"),
                            contribution: ContributionKind::Add,
                        }],
                    ),
                    "source-assembly" => (
                        ReadinessProgramRole::SourceFinalInputAssembly,
                        vec![StageChannel::SkillParameter {
                            parameter: final_parameter(),
                        }],
                    ),
                    "source-direct-assembly" => (
                        ReadinessProgramRole::SourceFinalInputAssembly,
                        vec![StageChannel::Stat {
                            scope: RuleEntityKind::Skill,
                            stat: def("direct-final"),
                        }],
                    ),
                    _ => continue,
                };
                row.phase = ReadinessPhase::Preparation;
                row.role = role;
                row.outputs = outputs;
                stages
                    .programs
                    .members
                    .iter_mut()
                    .find(|binding| binding.owner == row.owner && binding.program == row.program)
                    .unwrap()
                    .stage = key(
                    if row.program == key("source-assembly")
                        || row.program == key("source-direct-assembly")
                    {
                        "apply"
                    } else {
                        "properties"
                    },
                );
            }
            edit_stages(stages);
        },
        |receiving| {
            receiving.schema_version = 3;
            receiving.targets.extend(
                [
                    SupportTargetDefinition::Gem(def("summoner")),
                    SupportTargetDefinition::Skill(def("summon")),
                ]
                .into_iter()
                .map(|owner| SupportTargetReceivingRoles {
                    owner,
                    roles: DeclaredSet::complete(vec![]),
                }),
            );
            receiving.source_properties = Some(SourcePropertyPreparationInput {
                relations: DeclaredSet::complete(vec![relation()]),
            });
            edit_receiving(receiving);
        },
    )
}
pub fn compile(f: &Fixture) -> Checked<Effects> {
    readiness::compile_inputs(inputs(f)?)
}
pub fn metrics(plan: Effects) -> Metrics {
    readiness::metrics(plan)
}
pub fn without_supports(f: &mut Fixture) {
    f.build.supports.clear();
    for sequence in f.build.support_origins.as_mut().unwrap() {
        sequence.origins.clear();
    }
}
pub fn evaluated(report: &SupportEffectsReport) -> &OwnedEffectsReport {
    readiness::delivery::evaluated(report)
}
pub fn value<'a>(report: &'a OwnedEffectsReport, key: &PlanValueKey) -> &'a EffectValue {
    &report
        .values
        .iter()
        .find(|row| &row.key == key)
        .unwrap()
        .value
}
pub fn final_key(id: u64) -> PlanValueKey {
    let SkillTarget::Generated(skill) = member(id) else {
        unreachable!()
    };
    PlanValueKey::SkillParameter {
        skill,
        parameter: final_parameter(),
    }
}
pub fn count_key(id: u64) -> PlanValueKey {
    PlanValueKey::Stat {
        entity: ConcreteEntity::Skill(Box::new(owner(id))),
        stat: def("source-count"),
    }
}

pub fn direct_parameter() -> DeclaredSlot<ParameterSlotDefId> {
    base::parameter(SlotOwnerDefId::Skill(def("skill")), "direct-raw")
}
pub fn direct_fixture() -> Fixture {
    let mut f = fixture();
    f.schema.schema_version = poe_optimizer_data::owned_schema::OWNED_SCHEMA_PACKAGE_V5;
    f.schema
        .definitions
        .push(integer_stat("direct-final", RuleEntityKind::Skill));
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
                .push(direct_parameter());
        }
    }
    f.schema.slots.push(SlotDescriptor::Parameter(known(
        direct_parameter(),
        ParameterSlotSchema {
            value: ValueSchema::Integer(IntegerRange {
                minimum: BoundedInteger::new(0).unwrap(),
                maximum: BoundedInteger::new(100).unwrap(),
            }),
            presence: SlotPresence::RequiredOnce,
            sites: vec![ParameterSite::SkillParameter],
            skill_input: Some(SkillInputAuthority::Authored),
        },
    )));
    let mut facts =
        readiness::program_mut(&mut f, readiness::physical_owner(), "target-facts-source").clone();
    facts.id = key("target-facts-direct");
    facts.reads[0].source = RuleReadSource::Parameter {
        slot: direct_parameter(),
    };
    f.owner_mut(&subject(def::<SkillDefinition>("skill")))
        .programs
        .members = vec![
        facts,
        RuleProgram {
            id: key("source-direct-assembly"),
            context: RuleEntityKind::Skill,
            reads: vec![
                base::read(
                    "raw",
                    RuleReadSource::Parameter {
                        slot: direct_parameter(),
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
        },
    ];
    f.build.skills = vec![SkillUse {
        id: occurrence(32),
        source: AuthoredSkillSource::Direct(def("skill")),
        enabled: true,
        scope: LoadoutScope::Shared,
        parameters: Some(vec![ParameterAssignment {
            slot: direct_parameter(),
            value: base::integer(12),
        }]),
    }];
    f.build
        .supports
        .retain(|row| [occurrence(60), occurrence(61)].contains(&row.id));
    for row in &mut f.build.supports {
        row.target = owner(32)
    }
    f.build.support_origins = Some(vec![SupportOriginSequence {
        target: owner(32),
        origins: vec![
            SupportOrigin::Assignment(occurrence(60)),
            SupportOrigin::Assignment(occurrence(61)),
        ],
    }]);
    f.queries.requests.clear();
    f
}
pub fn direct_inputs(f: &Fixture) -> Checked<Inputs> {
    inputs_with(
        f,
        |_| {},
        |_| {},
        |receiving| {
            receiving.targets.push(SupportTargetReceivingRoles {
                owner: SupportTargetDefinition::Skill(def("skill")),
                roles: DeclaredSet::complete(vec![]),
            });
            let mut direct = relation();
            direct.id = key("direct-source");
            direct.owner = SupportTargetDefinition::Skill(def("skill"));
            direct.effects = DeclaredSet::complete(vec![SourcePropertyEffect {
                endpoint: SourcePropertyEffectEndpoint::DirectOwner {},
                admission: SupportAdmissionContext::AssignedSkill,
            }]);
            direct.assembly = DeclaredSet::complete(vec![key("source-direct-assembly")]);
            receiving
                .source_properties
                .as_mut()
                .unwrap()
                .relations
                .members
                .push(direct);
        },
    )
}

pub fn second_supply() -> DeclaredSlot<SkillGrantSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Gem(def("summoner")),
        slot: def("second-summon"),
    }
}
pub fn second_grant() -> DeclaredSlot<GrantSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Gem(def("summoner")),
        slot: def("second-summon-grant"),
    }
}
pub fn second_member(id: u64) -> SkillTarget {
    SkillTarget::Generated(Box::new(GeneratedSkillKey {
        provider: ProviderKey {
            root: ProviderRoot::SkillUse(occurrence(id)),
            grant_path: vec![],
        },
        slot: second_supply(),
    }))
}
pub fn two_effects(f: &mut Fixture) {
    for row in &mut f.schema.definitions {
        if let DefinitionDescriptor::Gem(row) = row
            && row.id == def::<GemDefinition>("summoner")
        {
            let SchemaState::Known(schema) = &mut row.schema else {
                unreachable!()
            };
            schema
                .declarations
                .skill_grants
                .members
                .push(second_supply());
            schema.declarations.grants.members.push(second_grant());
        }
    }
    f.schema.slots.extend([
        SlotDescriptor::SkillGrant(known(
            second_supply(),
            SkillGrantSlotSchema {
                skill: def("summon"),
                outputs: DeclaredSet::complete(vec![output()]),
            },
        )),
        SlotDescriptor::Grant(known(
            second_grant(),
            GrantSlotSchema {
                provider_roles: vec![ProviderRole::SkillUse],
                target: GrantTarget::Skill(second_supply()),
            },
        )),
    ]);
    let supply = readiness::program_mut(f, readiness::physical_owner(), "supply-summon");
    let mut effects = supply.effects.clone();
    for effect in &mut effects {
        effect.id = key(&format!("second-{}", effect.id));
        match &mut effect.effect {
            RuleEffectKind::ActivateGrant { slot, .. } => *slot = second_grant(),
            RuleEffectKind::ProjectSkillParameter { skill, .. } => *skill = second_supply(),
            _ => unreachable!(),
        }
    }
    supply.effects.extend(effects);
    let assembly = readiness::program_mut(f, readiness::physical_owner(), "source-assembly");
    assembly.effects.push(effect(
        "second-final",
        RuleEffectKind::ProjectSkillParameter {
            skill: second_supply(),
            parameter: final_parameter(),
            value: key("final"),
        },
    ));
}
pub fn second_relation_effect() -> SourcePropertyEffect {
    SourcePropertyEffect {
        endpoint: SourcePropertyEffectEndpoint::Generated {
            path: vec![],
            skill_supply: second_supply(),
        },
        admission: SupportAdmissionContext::ReceivingSkill {
            summoner_path: None,
        },
    }
}
pub fn family_fixture() -> Fixture {
    let mut f = fixture();
    two_effects(&mut f);
    let gem = f
        .schema
        .definitions
        .iter()
        .find_map(|row| match row {
            DefinitionDescriptor::Gem(row) if row.id == def::<GemDefinition>("support") => {
                Some(row.clone())
            }
            _ => None,
        })
        .unwrap();
    let origin =
        readiness::program_mut(&mut f, readiness::delivery::support_owner(), "origin-facts")
            .clone();
    for name in ["family-a", "family-b"] {
        let mut copy = gem.clone();
        copy.id = def(name);
        f.schema.definitions.push(DefinitionDescriptor::Gem(copy));
        f.owners.push(DefinitionRules {
            owner: subject(def::<GemDefinition>(name)),
            programs: DeclaredSet::complete(vec![origin.clone()]),
        });
    }
    for (gem, assignment, name) in [(80, 90, "family-a"), (81, 91, "family-b")] {
        f.build.gems.push(GemInstance {
            id: occurrence(gem),
            definition: def(name),
            level: 1,
            quality: None,
            parameters: vec![],
        });
        f.build.supports.push(SupportAssignment {
            id: occurrence(assignment),
            support: occurrence(gem),
            target: owner(30),
            enabled: true,
        });
    }
    f.build
        .supports
        .retain(|row| [occurrence(90), occurrence(91), occurrence(61)].contains(&row.id));
    f.build.support_origins = Some(vec![
        SupportOriginSequence {
            target: owner(30),
            origins: vec![
                SupportOrigin::Assignment(occurrence(90)),
                SupportOrigin::Assignment(occurrence(91)),
                SupportOrigin::Assignment(occurrence(61)),
            ],
        },
        SupportOriginSequence {
            target: owner(31),
            origins: vec![],
        },
    ]);
    f
}
pub fn family_inputs(f: &Fixture) -> Checked<Inputs> {
    use poe_optimizer_core::owned_supports::SupportPreparationEntry;
    use poe_optimizer_data::{
        owned_rules::OwnedRulePackage, owned_support_inputs::OwnedSupportInputBindings,
        owned_support_receiving::OwnedSupportReceiving, owned_supports::OwnedSupportPreparation,
    };
    // Reuse the complete package builders, then replace only this fixture's
    // finite support-family catalog and its exact dependency commitments.
    let mut raw_rules = None;
    let mut result = inputs_with(
        f,
        |rules| raw_rules = Some(rules.clone()),
        |_| {},
        |receiving| {
            receiving
                .source_properties
                .as_mut()
                .unwrap()
                .relations
                .members[0]
                .effects
                .members
                .push(second_relation_effect());
        },
    )?;
    let stored = OwnedRulePackage::new(
        raw_rules.unwrap(),
        result.definitions.as_ref(),
        Default::default(),
    )
    .map_err(|e| e.to_string())?;
    let mut raw = result.preparation.input().clone();
    raw.families = vec![key("family-a"), key("family-b")];
    raw.effects
        .extend([key("family-a-effect"), key("family-b-effect")]);
    let SchemaState::Known(definition) = &mut raw.supports[0].preparation else {
        unreachable!()
    };
    definition.families = Some(vec![key("family-a"), key("family-b")]);
    let combined = definition.clone();
    for name in ["family-a", "family-b"] {
        let mut definition = combined.clone();
        definition.effect = key(&format!("{name}-effect"));
        definition.families = Some(vec![key(name)]);
        raw.supports.push(SupportPreparationEntry {
            gem: def(name),
            preparation: SchemaState::Known(definition),
        });
    }
    let preparation = Arc::new(
        OwnedSupportPreparation::new(
            raw,
            result.definitions.as_ref(),
            &stored,
            Default::default(),
        )
        .map_err(|e| e.to_string())?,
    );
    let mut raw = result.inputs.input().clone();
    raw.preparation = *preparation.identity();
    let inputs = Arc::new(
        OwnedSupportInputBindings::new(
            raw,
            result.definitions.as_ref(),
            &stored,
            &preparation,
            &result.stages,
            Default::default(),
        )
        .map_err(|e| e.to_string())?,
    );
    let mut raw = result.receiving.input().clone();
    raw.preparation = *preparation.identity();
    raw.inputs = *inputs.identity();
    for name in ["family-a", "family-b"] {
        raw.supports.push(SupportReceivingEntry {
            gem: def(name),
            receivers: DeclaredSet::complete(vec![]),
        });
        raw.source_properties.as_mut().unwrap().relations.members[0]
            .supports
            .members
            .push(SourcePropertySupportPrograms {
                gem: def(name),
                counted: true,
                programs: DeclaredSet::complete(vec![]),
            });
    }
    let receiving = Arc::new(
        OwnedSupportReceiving::new(
            raw,
            result.definitions.as_ref(),
            &stored,
            &preparation,
            &inputs,
            &result.stages,
            Default::default(),
        )
        .map_err(|e| e.to_string())?,
    );
    result.preparation = preparation;
    result.inputs = inputs;
    result.receiving = receiving;
    Ok(result)
}
