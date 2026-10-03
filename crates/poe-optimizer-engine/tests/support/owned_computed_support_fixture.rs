//! Computed support preparation keeps the complete generated target activation path.
#[allow(dead_code)]
#[path = "owned_plan_fixture.rs"]
mod base_fixture;
pub use base_fixture::*;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_routing::*, owned_rules::*, owned_schema::*,
    owned_stages::*, owned_support_inputs::*, owned_supports::*,
};
use poe_optimizer_data::{
    owned_routing::{OwnedActionRouting, RoutingLimits},
    owned_rules::{OwnedRulePackage, RuleStorageLimits},
    owned_schema::{OWNED_SCHEMA_PACKAGE_V4, OwnedDefinitionSchemaPackage, OwnedSchemaLimits},
    owned_stages::{OwnedEvaluationStages, StageStorageLimits},
    owned_support_inputs::{OwnedSupportInputBindings, SupportInputStorageLimits},
    owned_supports::{OwnedSupportPreparation, SupportStorageLimits},
};
use poe_optimizer_engine::{
    owned_plan::{
        ComputedSupportOutcome, ComputedSupportReport, OwnedSupportPreparationPlan, PlanLimits,
        SupportPreparationPlanInputs,
    },
    owned_rules::*,
    owned_supports::*,
};
use std::sync::Arc;

pub const FLAGS: &[(&str, bool)] = &[
    ("spell", true),
    ("minion-present", false),
    ("summoner-present", false),
    ("summoner-minion-present", false),
    ("cannot-support", false),
    ("has-gem", false),
    ("from-item", false),
    ("is-player", false),
];
fn empty<T>() -> DeclaredSet<T> {
    DeclaredSet::complete(vec![])
}
fn entry<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
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
fn range() -> IntegerRange {
    IntegerRange {
        minimum: BoundedInteger::new(0).unwrap(),
        maximum: BoundedInteger::new(100).unwrap(),
    }
}
fn actor_owner() -> SchemaSubject {
    subject(def::<ActorDefinition>("family"))
}
fn supply(name: &str) -> DeclaredSlot<SkillGrantSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Actor(def("family")),
        slot: def(name),
    }
}
fn activation(name: &str) -> DeclaredSlot<GrantSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Actor(def("family")),
        slot: def(name),
    }
}
fn ability_input(name: &str) -> DeclaredSlot<ParameterSlotDefId> {
    parameter(SlotOwnerDefId::Skill(def("ability")), name)
}
fn entered_actor(use_id: u64) -> ProviderKey {
    let mut provider = summoner_provider(use_id);
    provider.grant_path.push(child_grant());
    provider
}
pub fn target(use_id: u64, name: &str) -> SkillTarget {
    SkillTarget::Generated(Box::new(GeneratedSkillKey {
        provider: entered_actor(use_id),
        slot: supply(name),
    }))
}
pub fn bool_node(name: &str, value: bool) -> RuleNode {
    node(
        name,
        RuleExpression::Literal {
            value: ParameterValue::Boolean(value),
        },
    )
}
fn rules(f: &mut Fixture, owner: SchemaSubject, programs: Vec<RuleProgram>) {
    f.owners.push(DefinitionRules {
        owner,
        programs: DeclaredSet::complete(programs),
    });
}
pub fn activation_program<'a>(f: &'a mut Fixture, name: &str) -> &'a mut RuleProgram {
    f.owner_mut(&actor_owner())
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == key(name))
        .unwrap()
}
pub fn generated_fixture() -> Fixture {
    let mut f = Fixture::new();
    f.schema.schema_version = OWNED_SCHEMA_PACKAGE_V4;
    f.build.items.clear();
    f.build.equipment.clear();
    f.build.skills.clear();
    f.build.gems.clear();
    f.queries.requests.clear();
    f.owner_mut(&class_owner()).programs.members.clear();
    f.add_generated_actors();
    for slot in &mut f.schema.slots {
        if let SlotDescriptor::Actor(DefinitionEntry {
            id,
            schema: SchemaState::Known(schema),
        }) = slot
            && *id == child_slot()
        {
            schema.provider_definition = Some(def("family"));
            schema.skills = DeclaredSet::complete(vec![def("ability")]);
        }
    }
    let mut actor_ports = ports();
    let mut ability_ports = ports();
    for name in ["level", "quality"] {
        ability_ports.parameters.members.push(ability_input(name));
        f.schema.slots.push(SlotDescriptor::Parameter(entry(
            ability_input(name),
            ParameterSlotSchema {
                skill_input: None,
                value: ValueSchema::Integer(range()),
                presence: SlotPresence::RequiredOnce,
                sites: vec![],
            },
        )));
    }
    let mut supply_programs = vec![];
    for name in ["first", "second"] {
        actor_ports.skill_grants.members.push(supply(name));
        actor_ports.grants.members.push(activation(name));
        f.schema.slots.extend([
            SlotDescriptor::SkillGrant(entry(
                supply(name),
                SkillGrantSlotSchema {
                    skill: def("ability"),
                    outputs: empty(),
                },
            )),
            SlotDescriptor::Grant(entry(
                activation(name),
                GrantSlotSchema {
                    provider_roles: vec![ProviderRole::SkillUse],
                    target: GrantTarget::Skill(supply(name)),
                },
            )),
        ]);
        supply_programs.push(RuleProgram {
            id: key(name),
            context: RuleEntityKind::Actor,
            reads: vec![read(
                "actor-level",
                RuleReadSource::Stat {
                    entity: RuleEntity::Current,
                    stat: def("child-plus"),
                },
            )],
            nodes: vec![
                read_node("level", "actor-level"),
                literal("zero", 0),
                bool_node("enabled", true),
            ],
            effects: vec![
                effect(
                    "activate",
                    RuleEffectKind::ActivateGrant {
                        slot: activation(name),
                        enabled: key("enabled"),
                    },
                ),
                effect(
                    "level",
                    RuleEffectKind::ProjectSkillParameter {
                        skill: supply(name),
                        parameter: ability_input("level"),
                        value: key("level"),
                    },
                ),
                effect(
                    "quality",
                    RuleEffectKind::ProjectSkillParameter {
                        skill: supply(name),
                        parameter: ability_input("quality"),
                        value: key("zero"),
                    },
                ),
            ],
        });
    }
    f.schema.definitions.extend([
        DefinitionDescriptor::Actor(entry(
            def("family"),
            ActorSchema {
                declarations: actor_ports,
            },
        )),
        DefinitionDescriptor::Skill(entry(
            def("ability"),
            SkillSchema {
                directly_selectable: false,
                declarations: ability_ports,
            },
        )),
        DefinitionDescriptor::Gem(entry(
            def("support"),
            GemSchema {
                level: range(),
                roles: vec![AuthoredGemRole::SupportAssignment],
                skills: empty(),
                quality: QualityUseSchema {
                    presence: QualityPresence::Forbidden,
                    allowed_kinds: empty(),
                },
                declarations: ports(),
            },
        )),
        DefinitionDescriptor::Unit(entry(
            def("quality"),
            UnitSchema {
                dimension: UnitDimension::PercentagePoints,
            },
        )),
        DefinitionDescriptor::Stat(entry(
            def("skill-base"),
            StatSchema {
                value: ComputedValueType::Integer,
                targets: vec![RuleEntityKind::Skill],
            },
        )),
        DefinitionDescriptor::Stat(entry(
            def("effective-level"),
            StatSchema {
                value: ComputedValueType::Integer,
                targets: vec![RuleEntityKind::SupportOrigin],
            },
        )),
        DefinitionDescriptor::Stat(entry(
            def("effective-quality"),
            StatSchema {
                value: ComputedValueType::Quantity {
                    unit: def("quality"),
                },
                targets: vec![RuleEntityKind::SupportOrigin],
            },
        )),
    ]);
    let mut skill_nodes = vec![read_node("base", "level")];
    let mut skill_effects = vec![derive("base", RuleEntity::Current, "skill-base", "base")];
    for &(name, value) in FLAGS {
        f.schema.definitions.push(DefinitionDescriptor::Stat(entry(
            def(name),
            StatSchema {
                value: ComputedValueType::Boolean,
                targets: vec![RuleEntityKind::Skill],
            },
        )));
        skill_nodes.push(bool_node(name, value));
        skill_effects.push(derive(name, RuleEntity::Current, name, name));
    }
    rules(&mut f, actor_owner(), supply_programs);
    rules(
        &mut f,
        subject(def::<SkillDefinition>("ability")),
        vec![RuleProgram {
            id: key("target-facts"),
            context: RuleEntityKind::Skill,
            reads: vec![read(
                "level",
                RuleReadSource::Parameter {
                    slot: ability_input("level"),
                },
            )],
            nodes: skill_nodes,
            effects: skill_effects,
        }],
    );
    rules(
        &mut f,
        subject(def::<GemDefinition>("support")),
        vec![RuleProgram {
            id: key("origin-facts"),
            context: RuleEntityKind::SupportOrigin,
            reads: vec![
                read("level", RuleReadSource::GemLevel),
                read(
                    "base",
                    RuleReadSource::Stat {
                        entity: RuleEntity::AssignedSkill,
                        stat: def("skill-base"),
                    },
                ),
            ],
            nodes: vec![
                read_node("level", "level"),
                read_node("base", "base"),
                node(
                    "effective",
                    RuleExpression::Add {
                        left: key("level"),
                        right: key("base"),
                    },
                ),
                node(
                    "quality",
                    RuleExpression::Literal {
                        value: ParameterValue::Quantity(
                            FiniteQuantity::new(0.0, def("quality")).unwrap(),
                        ),
                    },
                ),
            ],
            effects: vec![
                derive("level", RuleEntity::Current, "effective-level", "effective"),
                derive(
                    "quality",
                    RuleEntity::Current,
                    "effective-quality",
                    "quality",
                ),
            ],
        }],
    );
    let mut sequences = vec![];
    for (group, target) in [
        target(30, "first"),
        target(30, "second"),
        target(31, "first"),
    ]
    .into_iter()
    .enumerate()
    {
        let mut origins = vec![];
        for offset in 0..2 {
            let serial = (group * 2 + offset) as u64;
            let gem = occurrence(50 + serial);
            let assignment = occurrence(60 + serial);
            f.build.gems.push(GemInstance {
                id: gem,
                definition: def("support"),
                parameters: vec![],
                level: (offset + 1) as u16,
                quality: None,
            });
            f.build.supports.push(SupportAssignment {
                id: assignment,
                support: gem,
                target: target.clone(),
                enabled: true,
            });
            origins.push(SupportOrigin::Assignment(assignment));
        }
        sequences.push(SupportOriginSequence { target, origins });
    }
    f.build.support_origins = Some(sequences);
    f
}

pub fn raw_rules(
    f: &Fixture,
    definitions: &OwnedDefinitionSchemaPackage,
    operations: &str,
) -> RulePackageInput {
    RulePackageInput {
        effect_applications: None,
        schema_version: OWNED_RULE_PACKAGE_VERSION,
        namespace: ns(),
        release: key("rules"),
        semantics_version: key("test-v1"),
        operations_version: key(operations),
        definitions: definitions.identity().clone(),
        tables: f.tables.clone(),
        owners: f.owners.clone(),
        receivers: f.receivers.clone(),
    }
}

pub fn raw_routing(f: &Fixture, definitions: &OwnedDefinitionSchemaPackage) -> ActionRoutingInput {
    ActionRoutingInput {
        schema_version: OWNED_ACTION_ROUTING_VERSION,
        namespace: ns(),
        release: key("routing"),
        definitions: definitions.identity().clone(),
        outputs: f.routes.clone(),
    }
}

pub fn raw_stages(
    f: &Fixture,
    definitions: &OwnedDefinitionSchemaPackage,
    stored_rules: &OwnedRulePackage,
    routing: &OwnedActionRouting,
) -> EvaluationStagesInput {
    let mut channels = vec![
        StageChannel::Stat {
            scope: RuleEntityKind::SupportOrigin,
            stat: def("effective-level"),
        },
        StageChannel::Stat {
            scope: RuleEntityKind::SupportOrigin,
            stat: def("effective-quality"),
        },
        StageChannel::Stat {
            scope: RuleEntityKind::Skill,
            stat: def("skill-base"),
        },
    ];
    channels.extend(FLAGS.iter().map(|(name, _)| StageChannel::Stat {
        scope: RuleEntityKind::Skill,
        stat: def(name),
    }));
    EvaluationStagesInput {
        readiness: None,
        effect_applications: None,
        schema_version: OWNED_EVALUATION_STAGES_VERSION,
        namespace: ns(),
        release: key("stages"),
        definitions: definitions.identity().clone(),
        rules: *stored_rules.identity(),
        routing: *routing.identity(),
        stages: vec![EvaluationStage {
            id: key("prepare"),
            predecessors: vec![],
        }],
        programs: DeclaredSet::complete(
            f.owners
                .iter()
                .flat_map(|owner| {
                    owner
                        .programs
                        .members
                        .iter()
                        .map(|program| StagedRuleProgram {
                            owner: owner.owner.clone(),
                            program: program.id.clone(),
                            stage: key("prepare"),
                        })
                })
                .collect(),
        ),
        routing_stage: key("prepare"),
        frozen_channels: channels
            .into_iter()
            .map(|channel| FrozenStageChannel {
                channel,
                stage: key("prepare"),
            })
            .collect(),
    }
}

pub fn raw_preparation(
    definitions: &OwnedDefinitionSchemaPackage,
    stored_rules: &OwnedRulePackage,
) -> SupportPreparationInput {
    SupportPreparationInput {
        schema_version: OWNED_SUPPORT_PREPARATION_VERSION,
        namespace: ns(),
        release: key("preparation"),
        definitions: definitions.identity().clone(),
        rules: *stored_rules.identity(),
        policy: SupportPreparationPolicy::OrderedReplacementRetryFrontierV1,
        quality_unit: def("quality"),
        types: vec![key("spell")],
        effects: vec![key("support-effect")],
        families: vec![],
        supports: vec![SupportPreparationEntry {
            gem: def("support"),
            preparation: SchemaState::Known(SupportPreparationDefinition {
                effect: key("support-effect"),
                families: None,
                plus_version_of: None,
                requires: Some(SupportTypePredicate::Type(key("spell"))),
                excludes: None,
                added_types: vec![],
                gems_only: false,
                from_item: false,
                is_support: true,
                is_trigger: false,
                ignore_minion_types: false,
            }),
        }],
    }
}

pub fn raw_support_inputs(
    definitions: &OwnedDefinitionSchemaPackage,
    stored_rules: &OwnedRulePackage,
    preparation: &OwnedSupportPreparation,
    stages: &OwnedEvaluationStages,
) -> SupportInputBindingsInput {
    let type_stats = || {
        vec![SupportTypeStat {
            support_type: key("spell"),
            stat: def("spell"),
        }]
    };
    SupportInputBindingsInput {
        schema_version: 1,
        namespace: ns(),
        release: key("inputs"),
        definitions: definitions.identity().clone(),
        rules: *stored_rules.identity(),
        preparation: *preparation.identity(),
        stages: *stages.identity(),
        preparation_stage: key("prepare"),
        effective_level: def("effective-level"),
        effective_quality: def("effective-quality"),
        target: SupportTargetInputBindings {
            skill_types: type_stats(),
            minion_types: OptionalTypeInputs {
                present: def("minion-present"),
                members: type_stats(),
            },
            summoner: OptionalTypeContextInputs {
                present: def("summoner-present"),
                skill_types: type_stats(),
                minion_types: OptionalTypeInputs {
                    present: def("summoner-minion-present"),
                    members: type_stats(),
                },
            },
            cannot_be_supported: def("cannot-support"),
            has_gem: def("has-gem"),
            from_item: def("from-item"),
            is_player_actor: def("is-player"),
        },
    }
}

pub fn compile_inputs(
    f: &Fixture,
    target: SkillTarget,
) -> SupportPreparationPlanInputs<OwnedDefinitionSchemaPackage> {
    let definitions = Arc::new(
        OwnedDefinitionSchemaPackage::new(f.schema.clone(), OwnedSchemaLimits::default()).unwrap(),
    );
    let rule_input = raw_rules(f, &definitions, OWNED_RULE_OPERATIONS_V12);
    let stored_rules = OwnedRulePackage::new(
        rule_input.clone(),
        definitions.as_ref(),
        RuleStorageLimits::default(),
    )
    .unwrap();
    let rules = Arc::new(
        CompiledRulePackage::compile_stored(
            &stored_rules,
            definitions.as_ref(),
            RuleLimits::default(),
        )
        .unwrap(),
    );
    let routing = Arc::new(
        OwnedActionRouting::new(
            raw_routing(f, &definitions),
            definitions.as_ref(),
            RoutingLimits::default(),
        )
        .unwrap(),
    );
    let stages = Arc::new(
        OwnedEvaluationStages::new(
            raw_stages(f, &definitions, &stored_rules, &routing),
            definitions.as_ref(),
            &stored_rules,
            &routing,
            StageStorageLimits::default(),
        )
        .unwrap(),
    );
    let preparation = Arc::new(
        OwnedSupportPreparation::new(
            raw_preparation(&definitions, &stored_rules),
            definitions.as_ref(),
            &stored_rules,
            SupportStorageLimits::default(),
        )
        .unwrap(),
    );
    let inputs = Arc::new(
        OwnedSupportInputBindings::new(
            raw_support_inputs(&definitions, &stored_rules, &preparation, &stages),
            definitions.as_ref(),
            &stored_rules,
            &preparation,
            &stages,
            SupportInputStorageLimits::default(),
        )
        .unwrap(),
    );
    SupportPreparationPlanInputs {
        request: Arc::new(f.request()),
        definitions,
        rules,
        routing,
        stages,
        preparation,
        inputs,
        target,
    }
}
pub fn compile(
    f: &Fixture,
    target: SkillTarget,
) -> OwnedSupportPreparationPlan<OwnedDefinitionSchemaPackage> {
    OwnedSupportPreparationPlan::compile(
        compile_inputs(f, target),
        PlanLimits::default(),
        SupportPreparationLimits::default(),
    )
    .unwrap()
}
pub fn evaluate(f: &Fixture, target: SkillTarget) -> ComputedSupportReport {
    let plan = compile(f, target);
    plan.evaluate(&mut plan.new_scratch()).unwrap()
}
pub fn prepared(report: &ComputedSupportReport) -> &PreparedSupports {
    let ComputedSupportOutcome::Prepared {
        result: SupportPreparationOutcome::Known(value),
    } = &report.outcome
    else {
        panic!("expected computed preparation: {report:?}");
    };
    value
}
