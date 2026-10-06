use poe_optimizer_core::{
    owned_build::ParameterValue, owned_definitions::*, owned_routing::*, owned_rules::*,
    owned_schema::*, owned_stages::*, owned_support_inputs::*, owned_support_outputs::*,
    owned_support_receiving::*, owned_supports::*,
};
use poe_optimizer_data::{
    owned_routing::*, owned_rules::*, owned_schema::*, owned_stages::*, owned_support_inputs::*,
    owned_support_outputs::*, owned_support_receiving::*, owned_supports::*,
};

pub fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
pub fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("support-output-tests", "v1").unwrap()
}
pub fn id<K: DefinitionDomain>(s: &str) -> DefId<K> {
    DefId::new(ns(), key(s))
}
pub fn owner() -> SchemaSubject {
    SchemaSubject::Definition(DefinitionAddress::Skill(id("skill")))
}
pub fn complete<T>(members: Vec<T>) -> DeclaredSet<T> {
    DeclaredSet::complete(members)
}
pub fn partial() -> SchemaClosure {
    SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: owner(),
            facet: SchemaFacet::GameRules,
            code: key("unconverted"),
        }],
    }
}
fn known<I, D>(id: I, schema: D) -> DefinitionEntry<I, D> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn ports() -> DeclaredSlots {
    DeclaredSlots {
        parameters: complete(vec![]),
        choices: complete(vec![]),
        grants: complete(vec![]),
        actors: complete(vec![]),
        skill_grants: complete(vec![]),
        outputs: complete(vec![]),
        sockets: complete(vec![]),
    }
}
pub fn types(prefix: &str) -> Vec<SupportTypeStat> {
    ["spell", "duration"]
        .into_iter()
        .map(|name| SupportTypeStat {
            support_type: key(name),
            stat: id(&format!("{prefix}-{name}")),
        })
        .collect()
}
fn target() -> SupportTargetInputBindings {
    SupportTargetInputBindings {
        skill_types: types("initial"),
        minion_types: OptionalTypeInputs {
            present: id("minion-present"),
            members: types("minion"),
        },
        summoner: OptionalTypeContextInputs {
            present: id("summoner-present"),
            skill_types: types("summoner"),
            minion_types: OptionalTypeInputs {
                present: id("summoner-minion-present"),
                members: types("summoner-minion"),
            },
        },
        cannot_be_supported: id("cannot-support"),
        has_gem: id("has-gem"),
        from_item: id("from-item"),
        is_player_actor: id("player-actor"),
    }
}
pub fn initial_booleans(t: &SupportTargetInputBindings) -> Vec<StatDefId> {
    let mut result = vec![
        t.minion_types.present.clone(),
        t.summoner.present.clone(),
        t.summoner.minion_types.present.clone(),
        t.cannot_be_supported.clone(),
        t.has_gem.clone(),
        t.from_item.clone(),
        t.is_player_actor.clone(),
    ];
    for rows in [
        &t.skill_types,
        &t.minion_types.members,
        &t.summoner.skill_types,
        &t.summoner.minion_types.members,
    ] {
        result.extend(rows.iter().map(|row| row.stat.clone()));
    }
    result
}
pub fn consumer() -> RuleProgram {
    RuleProgram {
        id: key("consumer"),
        context: RuleEntityKind::Skill,
        reads: vec![RuleRead {
            id: key("final-type"),
            value_type: ComputedValueType::Boolean,
            source: RuleReadSource::Stat {
                entity: RuleEntity::Current,
                stat: id("final-duration"),
            },
        }],
        nodes: vec![
            RuleNode {
                id: key("value"),
                expression: RuleExpression::Read {
                    input: key("final-type"),
                },
            },
            RuleNode {
                id: key("false"),
                expression: RuleExpression::Literal {
                    value: ParameterValue::Boolean(false),
                },
            },
        ],
        effects: vec![RuleEffect {
            id: key("requirement"),
            when: None,
            effect: RuleEffectKind::Requirement {
                satisfied: key("value"),
                code: key("test"),
            },
        }],
    }
}

pub struct Fixture {
    pub schema: OwnedDefinitionSchemaPackage,
    pub rules: OwnedRulePackage,
    pub preparation: OwnedSupportPreparation,
    pub inputs: OwnedSupportInputBindings,
    pub receiving: OwnedSupportReceiving,
    pub stages: OwnedEvaluationStages,
    pub input: SupportOutputBindingsInput,
}
impl Fixture {
    pub fn new() -> Self {
        Self::with(|_| {}, |_| {})
    }
    pub fn with(
        edit_rules: impl FnOnce(&mut RulePackageInput),
        edit_stages: impl FnOnce(&mut EvaluationStagesInput),
    ) -> Self {
        let target = target();
        let mut definitions = vec![
            DefinitionDescriptor::Skill(known(
                id("skill"),
                SkillSchema {
                    directly_selectable: true,
                    declarations: ports(),
                },
            )),
            DefinitionDescriptor::Modifier(known(
                id("modifier"),
                ModifierSchema {
                    declarations: ports(),
                },
            )),
            DefinitionDescriptor::Unit(known(
                id("quality-unit"),
                UnitSchema {
                    dimension: UnitDimension::PercentagePoints,
                },
            )),
            DefinitionDescriptor::Stat(known(
                id("effective-level"),
                StatSchema {
                    value: ComputedValueType::Integer,
                    targets: vec![RuleEntityKind::SupportOrigin],
                },
            )),
            DefinitionDescriptor::Stat(known(
                id("effective-quality"),
                StatSchema {
                    value: ComputedValueType::Quantity {
                        unit: id("quality-unit"),
                    },
                    targets: vec![RuleEntityKind::SupportOrigin],
                },
            )),
            DefinitionDescriptor::Stat(known(
                id("wrong-value"),
                StatSchema {
                    value: ComputedValueType::Integer,
                    targets: vec![RuleEntityKind::Skill],
                },
            )),
            DefinitionDescriptor::Stat(known(
                id("wrong-scope"),
                StatSchema {
                    value: ComputedValueType::Boolean,
                    targets: vec![RuleEntityKind::Actor],
                },
            )),
            DefinitionDescriptor::Stat(known(
                id("multiple-scopes"),
                StatSchema {
                    value: ComputedValueType::Boolean,
                    targets: vec![RuleEntityKind::Skill, RuleEntityKind::Actor],
                },
            )),
            DefinitionDescriptor::Stat(DefinitionEntry {
                id: id("unmapped"),
                schema: SchemaState::Unmapped {
                    gaps: vec![SchemaGap {
                        subject: SchemaSubject::Definition(DefinitionAddress::Stat(id("unmapped"))),
                        facet: SchemaFacet::InputSchema,
                        code: key("unconverted"),
                    }],
                },
            }),
        ];
        let mut booleans = initial_booleans(&target);
        booleans.extend(types("final").into_iter().map(|row| row.stat));
        definitions.extend(booleans.into_iter().map(|stat| {
            DefinitionDescriptor::Stat(known(
                stat,
                StatSchema {
                    value: ComputedValueType::Boolean,
                    targets: vec![RuleEntityKind::Skill],
                },
            ))
        }));
        let schema = OwnedDefinitionSchemaPackage::new(
            SchemaPackageInput {
                schema_version: OWNED_SCHEMA_PACKAGE_V4,
                namespace: ns(),
                release: key("schema"),
                semantics_version: key("v1"),
                definitions,
                slots: vec![],
            },
            Default::default(),
        )
        .unwrap();
        let mut rules = RulePackageInput {
            ordered_contributions: None,
            effect_applications: None,
            schema_version: OWNED_RULE_PACKAGE_VERSION,
            namespace: ns(),
            release: key("rules"),
            semantics_version: key("v1"),
            operations_version: key(OWNED_RULE_OPERATIONS_V13),
            definitions: schema.identity().clone(),
            tables: vec![],
            owners: vec![DefinitionRules {
                owner: owner(),
                programs: complete(vec![consumer()]),
            }],
            receivers: complete(vec![]),
        };
        edit_rules(&mut rules);
        let rules = OwnedRulePackage::new(rules, &schema, Default::default()).unwrap();
        let routing = OwnedActionRouting::new(
            ActionRoutingInput {
                schema_version: OWNED_ACTION_ROUTING_VERSION,
                namespace: ns(),
                release: key("routing"),
                definitions: schema.identity().clone(),
                outputs: vec![],
            },
            &schema,
            Default::default(),
        )
        .unwrap();
        let preparation = OwnedSupportPreparation::new(
            SupportPreparationInput {
                schema_version: OWNED_SUPPORT_PREPARATION_VERSION,
                namespace: ns(),
                release: key("preparation"),
                definitions: schema.identity().clone(),
                rules: *rules.identity(),
                policy: SupportPreparationPolicy::OrderedReplacementRetryFrontierV1,
                quality_unit: id("quality-unit"),
                types: vec![key("spell"), key("duration")],
                effects: vec![],
                families: vec![],
                supports: vec![],
            },
            &schema,
            &rules,
            Default::default(),
        )
        .unwrap();
        let mut frozen_channels: Vec<_> = initial_booleans(&target)
            .into_iter()
            .map(|stat| FrozenStageChannel {
                channel: StageChannel::Stat {
                    scope: RuleEntityKind::Skill,
                    stat,
                },
                stage: key("prepare"),
            })
            .collect();
        frozen_channels.extend(types("final").into_iter().map(|row| FrozenStageChannel {
            channel: StageChannel::Stat {
                scope: RuleEntityKind::Skill,
                stat: row.stat,
            },
            stage: key("output"),
        }));
        for name in ["effective-level", "effective-quality"] {
            frozen_channels.push(FrozenStageChannel {
                channel: StageChannel::Stat {
                    scope: RuleEntityKind::SupportOrigin,
                    stat: id(name),
                },
                stage: key("prepare"),
            });
        }
        let mut stages = EvaluationStagesInput {
            readiness: None,
            effect_applications: None,
            schema_version: OWNED_EVALUATION_STAGES_VERSION,
            namespace: ns(),
            release: key("stages"),
            definitions: schema.identity().clone(),
            rules: *rules.identity(),
            routing: *routing.identity(),
            stages: vec![
                EvaluationStage {
                    id: key("prepare"),
                    predecessors: vec![],
                },
                EvaluationStage {
                    id: key("output"),
                    predecessors: vec![key("prepare")],
                },
                EvaluationStage {
                    id: key("consume"),
                    predecessors: vec![key("output")],
                },
                EvaluationStage {
                    id: key("independent"),
                    predecessors: vec![],
                },
            ],
            programs: complete(
                rules
                    .input()
                    .owners
                    .iter()
                    .flat_map(|o| {
                        o.programs.members.iter().map(|p| StagedRuleProgram {
                            owner: o.owner.clone(),
                            program: p.id.clone(),
                            stage: key("consume"),
                        })
                    })
                    .collect(),
            ),
            routing_stage: key("consume"),
            frozen_channels,
        };
        edit_stages(&mut stages);
        let stages =
            OwnedEvaluationStages::new(stages, &schema, &rules, &routing, Default::default())
                .unwrap();
        let inputs = OwnedSupportInputBindings::new(
            SupportInputBindingsInput {
                schema_version: OWNED_SUPPORT_INPUT_BINDINGS_VERSION,
                namespace: ns(),
                release: key("inputs"),
                definitions: schema.identity().clone(),
                rules: *rules.identity(),
                preparation: *preparation.identity(),
                stages: *stages.identity(),
                preparation_stage: key("prepare"),
                effective_level: id("effective-level"),
                effective_quality: id("effective-quality"),
                target,
            },
            &schema,
            &rules,
            &preparation,
            &stages,
            Default::default(),
        )
        .unwrap();
        let receiving = OwnedSupportReceiving::new(
            SupportReceivingInput {
                source_properties: None,
                schema_version: OWNED_SUPPORT_RECEIVING_VERSION,
                namespace: ns(),
                release: key("receiving"),
                definitions: schema.identity().clone(),
                rules: *rules.identity(),
                preparation: *preparation.identity(),
                inputs: *inputs.identity(),
                stages: *stages.identity(),
                roles: vec![],
                targets: vec![],
                supports: vec![],
            },
            &schema,
            &rules,
            &preparation,
            &inputs,
            &stages,
            Default::default(),
        )
        .unwrap();
        let input = SupportOutputBindingsInput {
            schema_version: OWNED_SUPPORT_OUTPUT_BINDINGS_VERSION,
            namespace: ns(),
            release: key("outputs"),
            definitions: schema.identity().clone(),
            rules: *rules.identity(),
            preparation: *preparation.identity(),
            inputs: *inputs.identity(),
            receiving: *receiving.identity(),
            stages: *stages.identity(),
            output_stage: key("output"),
            final_skill_types: types("final"),
        };
        Self {
            schema,
            rules,
            preparation,
            inputs,
            receiving,
            stages,
            input,
        }
    }
    pub fn dependencies(&self) -> SupportOutputDependencies<'_, OwnedDefinitionSchemaPackage> {
        SupportOutputDependencies {
            definitions: &self.schema,
            rules: &self.rules,
            preparation: &self.preparation,
            inputs: &self.inputs,
            receiving: &self.receiving,
            stages: &self.stages,
        }
    }
    pub fn build(
        &self,
        input: SupportOutputBindingsInput,
    ) -> Result<OwnedSupportOutputBindings, SupportOutputStorageError> {
        OwnedSupportOutputBindings::new(input, &self.dependencies(), Default::default())
    }
}
