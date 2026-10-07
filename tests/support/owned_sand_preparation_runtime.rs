//! Checked source-property staging for the explicitly finite Sand component world.
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_readiness::*, owned_rules::*, owned_schema::*,
    owned_stages::*, owned_support_inputs::*, owned_support_receiving::*, owned_supports::*,
};
use poe_optimizer_data::{
    owned_routing::OwnedActionRouting, owned_rules::OwnedRulePackage,
    owned_schema::OwnedDefinitionSchemaPackage, owned_stages::OwnedEvaluationStages,
    owned_support_inputs::OwnedSupportInputBindings,
    owned_support_receiving::OwnedSupportReceiving, owned_supports::OwnedSupportPreparation,
};
use poe_optimizer_engine::{owned_plan::*, owned_rules::CompiledRulePackage};
use std::sync::Arc;
fn key(name: &str) -> OwnedDefinitionKey {
    name.parse().unwrap()
}

pub fn compile(
    world: &super::World,
) -> std::result::Result<OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>, String> {
    let recipe = &world.recipe;
    let build = &world.build;
    let scenario = &world.scenario;
    let quality_unit = super::def(2);
    let queries = QueryInput {
        game_version: super::ns(),
        requests: vec![],
    };
    let namespace = recipe.schema.namespace.clone();
    let unused_input = |name: &str| -> StatDefId {
        DefId::parse(
            namespace.clone(),
            format!("fixture.sand-no-physical-supports.{name}"),
        )
        .unwrap()
    };
    assert!(matches!(
        recipe.rules.operations_version.as_str(),
        OWNED_RULE_OPERATIONS_V20 | OWNED_RULE_OPERATIONS_V21 | OWNED_RULE_OPERATIONS_V22
    ));
    let unused = [
        (
            unused_input("support-level"),
            RuleEntityKind::SupportOrigin,
            ComputedValueType::Integer,
        ),
        (
            unused_input("support-quality"),
            RuleEntityKind::SupportOrigin,
            ComputedValueType::Quantity {
                unit: quality_unit.clone(),
            },
        ),
        (
            unused_input("target-false"),
            RuleEntityKind::Skill,
            ComputedValueType::Boolean,
        ),
        (
            unused_input("target-player"),
            RuleEntityKind::Skill,
            ComputedValueType::Boolean,
        ),
    ];
    let mut schema_input = recipe.schema.clone();
    for (id, scope, value) in &unused {
        assert!(
            !schema_input
                .definitions
                .iter()
                .any(|d| d.address() == id.address())
        );
        schema_input
            .definitions
            .push(DefinitionDescriptor::Stat(DefinitionEntry {
                id: id.clone(),
                schema: SchemaState::Known(StatSchema {
                    value: value.clone(),
                    targets: vec![*scope],
                }),
            }));
    }
    let schema = Arc::new(
        OwnedDefinitionSchemaPackage::new(schema_input, Default::default())
            .map_err(|e| format!("{e:?}"))?,
    );
    let mut rules_input = recipe.rules.clone();
    rules_input.definitions = schema.identity().clone();
    // This finite world has no physical Gems, equipment-origin skills, child
    // attack types or support candidates. These exact targets still need
    // explicit typed facts; missing metadata must never become a default.
    // Real provider-specific admission remains outside this fixture's claim.
    for owner in [0x322, 0xd8] {
        rules_input
            .owners
            .iter_mut()
            .find(|o| o.owner == super::subject(super::def::<SkillDefinition>(owner)))
            .unwrap()
            .programs
            .members
            .push(RuleProgram {
                id: key("finite-target-facts"),
                context: RuleEntityKind::Skill,
                reads: vec![],
                nodes: vec![
                    RuleNode {
                        id: key("false"),
                        expression: RuleExpression::Literal {
                            value: ParameterValue::Boolean(false),
                        },
                    },
                    RuleNode {
                        id: key("player"),
                        expression: RuleExpression::Literal {
                            value: ParameterValue::Boolean(true),
                        },
                    },
                ],
                effects: [("false", "target-false"), ("player", "target-player")]
                    .into_iter()
                    .map(|(value, stat)| RuleEffect {
                        id: key(value),
                        when: None,
                        effect: RuleEffectKind::Derive {
                            entity: RuleEntity::Current,
                            stat: unused_input(stat),
                            value: key(value),
                        },
                    })
                    .collect(),
            });
    }
    let stored = OwnedRulePackage::new(rules_input, schema.as_ref(), Default::default())
        .map_err(|e| format!("{e:?}"))?;
    let rules = Arc::new(
        CompiledRulePackage::compile_stored(&stored, schema.as_ref(), Default::default())
            .map_err(|e| format!("{e:?}"))?,
    );
    let mut routing_input = recipe.routing.clone();
    routing_input.definitions = schema.identity().clone();
    let routing = Arc::new(
        OwnedActionRouting::new(routing_input, schema.as_ref(), Default::default())
            .map_err(|e| format!("{e:?}"))?,
    );
    let programs: Vec<_> = stored
        .input()
        .owners
        .iter()
        .flat_map(|o| o.programs.members.iter().map(move |p| (o, p)))
        .collect();
    let stages = Arc::new(
        OwnedEvaluationStages::new(
            EvaluationStagesInput {
                schema_version: OWNED_EVALUATION_STAGES_V3,
                namespace: namespace.clone(),
                release: key("finite-sand-stages"),
                definitions: schema.identity().clone(),
                rules: *stored.identity(),
                routing: *routing.identity(),
                stages: [
                    "prepare",
                    "source-census",
                    "source-properties",
                    "source-assembly",
                    "descendants",
                    "execute",
                ]
                .into_iter()
                .enumerate()
                .map(|(i, name)| EvaluationStage {
                    id: key(name),
                    predecessors: i
                        .checked_sub(1)
                        .map(|j| {
                            vec![key([
                                "prepare",
                                "source-census",
                                "source-properties",
                                "source-assembly",
                                "descendants",
                                "execute",
                            ][j])]
                        })
                        .unwrap_or_default(),
                })
                .collect(),
                programs: DeclaredSet::complete(
                    programs
                        .iter()
                        .map(|(o, p)| StagedRuleProgram {
                            owner: o.owner.clone(),
                            program: p.id.clone(),
                            stage: key(super::classification(p).1),
                        })
                        .collect(),
                ),
                effect_applications: Some(DeclaredSet::complete(vec![])),
                routing_stage: key("execute"),
                frozen_channels: unused
                    .iter()
                    .map(|(id, scope, _)| FrozenStageChannel {
                        channel: StageChannel::Stat {
                            scope: *scope,
                            stat: id.clone(),
                        },
                        stage: key("prepare"),
                    })
                    .collect(),
                readiness: Some(ReadinessInput {
                    skills: vec![
                        SkillReadiness {
                            participation: None,
                            skill: super::def(0x322),
                            parameters: DeclaredSet::complete(vec![
                                ParameterReadiness {
                                    parameter: super::slot(0x3261, 0x322),
                                    phase: ReadinessPhase::Structural,
                                },
                                ParameterReadiness {
                                    parameter: super::slot(0x3262, 0x322),
                                    phase: ReadinessPhase::Structural,
                                },
                            ]),
                        },
                        SkillReadiness {
                            participation: None,
                            skill: super::def(0xd8),
                            parameters: DeclaredSet::complete(vec![
                                ParameterReadiness {
                                    parameter: super::slot(0x3350, 0xd8),
                                    phase: ReadinessPhase::Execution,
                                },
                                ParameterReadiness {
                                    parameter: super::slot(0x3351, 0xd8),
                                    phase: ReadinessPhase::Execution,
                                },
                            ]),
                        },
                    ],
                    programs: DeclaredSet::complete(
                        programs
                            .iter()
                            .map(|(o, p)| ReadinessProgram {
                                owner: o.owner.clone(),
                                program: p.id.clone(),
                                phase: if super::classification(p).0
                                    == ReadinessProgramRole::Execution
                                {
                                    ReadinessPhase::Execution
                                } else {
                                    ReadinessPhase::Preparation
                                },
                                role: super::classification(p).0,
                                outputs: if super::classification(p).0
                                    == ReadinessProgramRole::Execution
                                {
                                    vec![]
                                } else {
                                    super::outputs(p)
                                },
                            })
                            .collect(),
                    ),
                }),
            },
            schema.as_ref(),
            &stored,
            &routing,
            Default::default(),
        )
        .map_err(|e| format!("{e:?}"))?,
    );
    let preparation = Arc::new(
        OwnedSupportPreparation::new(
            SupportPreparationInput {
                schema_version: OWNED_SUPPORT_PREPARATION_VERSION,
                namespace: namespace.clone(),
                release: key("finite-sand-no-physical-supports"),
                definitions: schema.identity().clone(),
                rules: *stored.identity(),
                policy: SupportPreparationPolicy::OrderedReplacementRetryFrontierV1,
                quality_unit: quality_unit.clone(),
                types: vec![],
                effects: vec![],
                families: vec![],
                supports: vec![],
            },
            schema.as_ref(),
            &stored,
            Default::default(),
        )
        .map_err(|e| format!("{e:?}"))?,
    );
    let absent = unused_input("target-false");
    let inputs = Arc::new(
        OwnedSupportInputBindings::new(
            SupportInputBindingsInput {
                schema_version: OWNED_SUPPORT_INPUT_BINDINGS_VERSION,
                namespace: namespace.clone(),
                release: key("finite-sand-no-physical-support-inputs"),
                definitions: schema.identity().clone(),
                rules: *stored.identity(),
                preparation: *preparation.identity(),
                stages: *stages.identity(),
                preparation_stage: key("prepare"),
                effective_level: unused_input("support-level"),
                effective_quality: unused_input("support-quality"),
                target: SupportTargetInputBindings {
                    skill_types: vec![],
                    minion_types: OptionalTypeInputs {
                        present: absent.clone(),
                        members: vec![],
                    },
                    summoner: OptionalTypeContextInputs {
                        present: absent.clone(),
                        skill_types: vec![],
                        minion_types: OptionalTypeInputs {
                            present: absent.clone(),
                            members: vec![],
                        },
                    },
                    cannot_be_supported: absent.clone(),
                    has_gem: absent.clone(),
                    from_item: absent,
                    is_player_actor: unused_input("target-player"),
                },
            },
            schema.as_ref(),
            &stored,
            &preparation,
            &stages,
            Default::default(),
        )
        .map_err(|e| format!("{e:?}"))?,
    );
    let receiving = Arc::new(
        OwnedSupportReceiving::new(
            SupportReceivingInput {
                schema_version: OWNED_SUPPORT_RECEIVING_V3,
                namespace: namespace.clone(),
                release: key("finite-sand-receivers"),
                definitions: schema.identity().clone(),
                rules: *stored.identity(),
                preparation: *preparation.identity(),
                inputs: *inputs.identity(),
                stages: *stages.identity(),
                roles: vec![],
                targets: vec![
                    SupportTargetReceivingRoles {
                        owner: SupportTargetDefinition::Skill(super::def(0x322)),
                        roles: DeclaredSet::complete(vec![]),
                    },
                    SupportTargetReceivingRoles {
                        owner: SupportTargetDefinition::Skill(super::def(0xd8)),
                        roles: DeclaredSet::complete(vec![]),
                    },
                ],
                supports: vec![],
                source_properties: Some(world.relations.clone()),
            },
            schema.as_ref(),
            &stored,
            &preparation,
            &inputs,
            &stages,
            Default::default(),
        )
        .map_err(|e| format!("{e:?}"))?,
    );
    let limits = OwnedInputLimits::default();
    let request = OwnedEvaluationRequest::new(
        BuildSpec::new(build.clone(), limits).map_err(|e| format!("{e:?}"))?,
        ScenarioSpec::new(scenario.clone(), limits).map_err(|e| format!("{e:?}"))?,
        QuerySpec::new(queries.clone(), limits).map_err(|e| format!("{e:?}"))?,
        limits,
    )
    .map_err(|e| format!("{e:?}"))?;
    OwnedSupportEffectPlan::compile(
        SupportEffectPlanInputs {
            request: Arc::new(request),
            definitions: schema,
            rules,
            routing,
            stages,
            preparation,
            inputs,
            receiving,
        },
        Default::default(),
        Default::default(),
    )
    .map_err(|e| format!("{e:?}"))
}
