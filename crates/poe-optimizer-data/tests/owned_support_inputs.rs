//! Static input-channel contracts, with no caller values or coverage promotion.
use poe_optimizer_core::{
    owned_content::digest_owned, owned_definitions::*, owned_routing::*, owned_rules::*,
    owned_schema::*, owned_stages::*, owned_support_inputs::*, owned_supports::*,
};
use poe_optimizer_data::{
    owned_routing::*, owned_rules::*, owned_schema::*, owned_stages::*, owned_support_inputs::*,
    owned_supports::*,
};

fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("support-input-tests", "v1").unwrap()
}
fn id<K: DefinitionDomain>(s: &str) -> DefId<K> {
    DefId::new(ns(), key(s))
}
fn empty<T>() -> DeclaredSet<T> {
    DeclaredSet::complete(vec![])
}
fn known<I, D>(id: I, schema: D) -> DefinitionEntry<I, D> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn types(prefix: &str) -> Vec<SupportTypeStat> {
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
        skill_types: types("skill"),
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
fn booleans(target: &SupportTargetInputBindings) -> Vec<StatDefId> {
    let mut result = vec![
        target.minion_types.present.clone(),
        target.summoner.present.clone(),
        target.summoner.minion_types.present.clone(),
        target.cannot_be_supported.clone(),
        target.has_gem.clone(),
        target.from_item.clone(),
        target.is_player_actor.clone(),
    ];
    for rows in [
        &target.skill_types,
        &target.minion_types.members,
        &target.summoner.skill_types,
        &target.summoner.minion_types.members,
    ] {
        result.extend(rows.iter().map(|row| row.stat.clone()));
    }
    result
}
struct Fixture {
    schema: OwnedDefinitionSchemaPackage,
    rules: OwnedRulePackage,
    routing: OwnedActionRouting,
    preparation: OwnedSupportPreparation,
    stages: OwnedEvaluationStages,
    input: SupportInputBindingsInput,
}
impl Fixture {
    fn new() -> Self {
        let target = target();
        let mut definitions = vec![
            DefinitionDescriptor::Unit(known(
                id("quality-unit"),
                UnitSchema {
                    dimension: UnitDimension::PercentagePoints,
                },
            )),
            DefinitionDescriptor::Unit(known(
                id("other-unit"),
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
                id("wrong-quality"),
                StatSchema {
                    value: ComputedValueType::Quantity {
                        unit: id("other-unit"),
                    },
                    targets: vec![RuleEntityKind::SupportOrigin],
                },
            )),
            DefinitionDescriptor::Stat(known(
                id("wrong-boolean"),
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
            DefinitionDescriptor::Stat(DefinitionEntry {
                id: id("unmapped"),
                schema: SchemaState::Unmapped {
                    gaps: vec![SchemaGap {
                        subject: SchemaSubject::Definition(DefinitionAddress::Stat(id("unmapped"))),
                        facet: SchemaFacet::InputSchema,
                        code: key("not-converted"),
                    }],
                },
            }),
        ];
        definitions.extend(booleans(&target).into_iter().map(|stat| {
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
        let rules = OwnedRulePackage::new(
            RulePackageInput {
                schema_version: OWNED_RULE_PACKAGE_VERSION,
                namespace: ns(),
                release: key("rules"),
                semantics_version: key("v1"),
                operations_version: key(OWNED_RULE_OPERATIONS_V12),
                definitions: schema.identity().clone(),
                tables: vec![],
                owners: vec![],
                receivers: empty(),
            },
            &schema,
            Default::default(),
        )
        .unwrap();
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
        let mut frozen_channels: Vec<_> = booleans(&target)
            .into_iter()
            .map(|stat| FrozenStageChannel {
                channel: StageChannel::Stat {
                    scope: RuleEntityKind::Skill,
                    stat,
                },
                stage: key("prepare"),
            })
            .collect();
        for name in ["effective-level", "effective-quality"] {
            frozen_channels.push(FrozenStageChannel {
                channel: StageChannel::Stat {
                    scope: RuleEntityKind::SupportOrigin,
                    stat: id(name),
                },
                stage: key("earlier"),
            });
        }
        let stages = OwnedEvaluationStages::new(
            EvaluationStagesInput {
                schema_version: OWNED_EVALUATION_STAGES_VERSION,
                namespace: ns(),
                release: key("stages"),
                definitions: schema.identity().clone(),
                rules: *rules.identity(),
                routing: *routing.identity(),
                stages: vec![
                    EvaluationStage {
                        id: key("earlier"),
                        predecessors: vec![],
                    },
                    EvaluationStage {
                        id: key("prepare"),
                        predecessors: vec![key("earlier")],
                    },
                    EvaluationStage {
                        id: key("later"),
                        predecessors: vec![key("prepare")],
                    },
                    EvaluationStage {
                        id: key("independent"),
                        predecessors: vec![],
                    },
                ],
                programs: empty(),
                routing_stage: key("later"),
                frozen_channels,
            },
            &schema,
            &rules,
            &routing,
            Default::default(),
        )
        .unwrap();
        let input = SupportInputBindingsInput {
            schema_version: OWNED_SUPPORT_INPUT_BINDINGS_VERSION,
            namespace: ns(),
            release: key("bindings"),
            definitions: schema.identity().clone(),
            rules: *rules.identity(),
            preparation: *preparation.identity(),
            stages: *stages.identity(),
            preparation_stage: key("prepare"),
            effective_level: id("effective-level"),
            effective_quality: id("effective-quality"),
            target,
        };
        Self {
            schema,
            rules,
            routing,
            preparation,
            stages,
            input,
        }
    }
    fn build(
        &self,
        input: SupportInputBindingsInput,
    ) -> Result<OwnedSupportInputBindings, SupportInputStorageError> {
        OwnedSupportInputBindings::new(
            input,
            &self.schema,
            &self.rules,
            &self.preparation,
            &self.stages,
            Default::default(),
        )
    }
    fn restage(&mut self, input: EvaluationStagesInput) {
        self.stages = OwnedEvaluationStages::new(
            input,
            &self.schema,
            &self.rules,
            &self.routing,
            Default::default(),
        )
        .unwrap();
        self.input.stages = *self.stages.identity();
    }
}

#[test]
fn canonical_type_maps_roundtrip_without_claiming_values_or_owner_closure() {
    let fixture = Fixture::new();
    let package = fixture.build(fixture.input.clone()).unwrap();
    assert_eq!(package.resources().entries, 17);
    assert!(fixture.rules.input().owners.is_empty());
    let mut reversed = fixture.input.clone();
    for rows in [
        &mut reversed.target.skill_types,
        &mut reversed.target.minion_types.members,
        &mut reversed.target.summoner.skill_types,
        &mut reversed.target.summoner.minion_types.members,
    ] {
        rows.reverse();
    }
    let reordered = fixture.build(reversed).unwrap();
    assert_eq!(reordered.identity(), package.identity());
    let wire = encode_support_input_bindings(&package, Default::default()).unwrap();
    let decoded = decode_support_input_bindings(
        &wire,
        &fixture.schema,
        &fixture.rules,
        &fixture.preparation,
        &fixture.stages,
        Default::default(),
    )
    .unwrap();
    assert_eq!(decoded.identity(), package.identity());
    assert_eq!(
        encode_support_input_bindings(&decoded, Default::default()).unwrap(),
        wire
    );
    package
        .verify_bindings(
            &fixture.schema,
            &fixture.rules,
            &fixture.preparation,
            &fixture.stages,
        )
        .unwrap();
    let mut changed = fixture.input.clone();
    changed.target.skill_types[0].stat = changed.target.has_gem.clone();
    assert_ne!(
        fixture.build(changed).unwrap().identity(),
        package.identity()
    );
}

#[test]
fn exact_vocabulary_is_required_in_all_four_type_maps() {
    let fixture = Fixture::new();
    for which in 0..4 {
        for malformed in 0..4 {
            let mut raw = fixture.input.clone();
            let rows = match which {
                0 => &mut raw.target.skill_types,
                1 => &mut raw.target.minion_types.members,
                2 => &mut raw.target.summoner.skill_types,
                _ => &mut raw.target.summoner.minion_types.members,
            };
            match malformed {
                0 => {
                    rows.pop();
                }
                1 => rows.push(rows[0].clone()),
                2 => rows[1].support_type = rows[0].support_type.clone(),
                _ => rows[0].support_type = key("unlisted"),
            }
            assert!(matches!(
                fixture.build(raw),
                Err(SupportInputStorageError::Invalid(_))
            ));
        }
    }
}

#[test]
fn level_quality_flags_and_type_members_require_exact_known_scopes_and_types() {
    let fixture = Fixture::new();
    for which in 0..9 {
        let mut raw = fixture.input.clone();
        match which {
            0 => raw.effective_level = id("effective-quality"),
            1 => raw.effective_quality = id("wrong-quality"),
            2 => raw.target.has_gem = id("wrong-scope"),
            3 => raw.target.from_item = id("wrong-boolean"),
            4 => raw.target.minion_types.present = id("unmapped"),
            5 => raw.target.summoner.present = id("missing"),
            6 => raw.target.summoner.minion_types.present = id("wrong-boolean"),
            7 => raw.target.skill_types[0].stat = id("wrong-boolean"),
            _ => {
                raw.target.cannot_be_supported = DefId::new(
                    GameVersionNamespace::new("foreign", "v1").unwrap(),
                    key("cannot-support"),
                )
            }
        }
        assert!(matches!(
            fixture.build(raw),
            Err(SupportInputStorageError::Invalid(_))
        ));
    }
}

#[test]
fn all_presence_channels_are_required_on_the_wire() {
    let fixture = Fixture::new();
    for which in 0..3 {
        let mut value = serde_json::to_value(&fixture.input).unwrap();
        let object = match which {
            0 => &mut value["target"]["minion_types"],
            1 => &mut value["target"]["summoner"],
            _ => &mut value["target"]["summoner"]["minion_types"],
        };
        object.as_object_mut().unwrap().remove("present");
        assert!(serde_json::from_value::<SupportInputBindingsInput>(value).is_err());
    }
    let mut value = serde_json::to_value(&fixture.input).unwrap();
    value["target"]["summoner"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<SupportInputBindingsInput>(value).is_err());
}

#[test]
fn input_channels_must_freeze_no_later_than_the_declared_preparation_stage() {
    for invalid_stage in [None, Some("later"), Some("independent")] {
        let mut fixture = Fixture::new();
        let mut stages = fixture.stages.input().clone();
        let index = stages.frozen_channels.iter().position(|row| matches!(&row.channel, StageChannel::Stat { stat, .. } if stat == &fixture.input.target.has_gem)).unwrap();
        if let Some(stage) = invalid_stage {
            stages.frozen_channels[index].stage = key(stage);
        } else {
            stages.frozen_channels.remove(index);
        }
        fixture.restage(stages);
        assert!(matches!(
            fixture.build(fixture.input.clone()),
            Err(SupportInputStorageError::Invalid(_))
        ));
    }
    let fixture = Fixture::new();
    let mut raw = fixture.input.clone();
    raw.preparation_stage = key("missing");
    assert!(matches!(
        fixture.build(raw),
        Err(SupportInputStorageError::Invalid(
            "unknown preparation stage"
        ))
    ));
}

#[test]
fn exact_artifact_binding_and_explicit_version_are_checked_before_use() {
    let fixture = Fixture::new();
    let different = digest_owned("different-test-identity", &1, 100).unwrap();
    for which in 0..5 {
        let mut raw = fixture.input.clone();
        match which {
            0 => raw.rules = different,
            1 => raw.preparation = different,
            2 => raw.stages = different,
            3 => raw.definitions.release = "different".into(),
            _ => raw.namespace = GameVersionNamespace::new("foreign", "v1").unwrap(),
        }
        assert!(matches!(
            fixture.build(raw),
            Err(SupportInputStorageError::Binding)
        ));
    }
    let package = fixture.build(fixture.input.clone()).unwrap();
    let mut stages = fixture.stages.input().clone();
    stages.release = key("changed-stage-package");
    let changed = OwnedEvaluationStages::new(
        stages,
        &fixture.schema,
        &fixture.rules,
        &fixture.routing,
        Default::default(),
    )
    .unwrap();
    assert!(matches!(
        package.verify_bindings(
            &fixture.schema,
            &fixture.rules,
            &fixture.preparation,
            &changed
        ),
        Err(SupportInputStorageError::Binding)
    ));
    let mut raw = fixture.input.clone();
    raw.schema_version += 1;
    assert!(matches!(
        fixture.build(raw),
        Err(SupportInputStorageError::Version(2))
    ));
}

#[test]
fn old_operations_cannot_enable_preparation_scope_bindings() {
    let mut fixture = Fixture::new();
    let mut rules = fixture.rules.input().clone();
    rules.operations_version = key(OWNED_RULE_OPERATIONS_V11);
    fixture.rules = OwnedRulePackage::new(rules, &fixture.schema, Default::default()).unwrap();
    let mut preparation = fixture.preparation.input().clone();
    preparation.rules = *fixture.rules.identity();
    fixture.preparation = OwnedSupportPreparation::new(
        preparation,
        &fixture.schema,
        &fixture.rules,
        Default::default(),
    )
    .unwrap();
    let mut stages = fixture.stages.input().clone();
    stages.rules = *fixture.rules.identity();
    fixture.restage(stages);
    fixture.input.rules = *fixture.rules.identity();
    fixture.input.preparation = *fixture.preparation.identity();
    assert!(matches!(
        fixture.build(fixture.input.clone()),
        Err(SupportInputStorageError::Invalid(
            "support input bindings require owned-domain-operations-v12"
        ))
    ));
}

#[test]
fn tighter_aggregate_work_and_wire_budgets_apply_to_new_decode_and_encode() {
    let fixture = Fixture::new();
    let package = fixture.build(fixture.input.clone()).unwrap();
    let wire = encode_support_input_bindings(&package, Default::default()).unwrap();
    for limits in [
        SupportInputStorageLimits {
            max_entries: package.resources().entries - 1,
            ..Default::default()
        },
        SupportInputStorageLimits {
            max_work: package.resources().work - 1,
            ..Default::default()
        },
        SupportInputStorageLimits {
            max_wire_bytes: wire.len() - 1,
            ..Default::default()
        },
    ] {
        assert!(
            OwnedSupportInputBindings::new(
                fixture.input.clone(),
                &fixture.schema,
                &fixture.rules,
                &fixture.preparation,
                &fixture.stages,
                limits
            )
            .is_err()
        );
        assert!(
            decode_support_input_bindings(
                &wire,
                &fixture.schema,
                &fixture.rules,
                &fixture.preparation,
                &fixture.stages,
                limits
            )
            .is_err()
        );
        assert!(encode_support_input_bindings(&package, limits).is_err());
    }
    let mut raw = fixture.input.clone();
    raw.target
        .skill_types
        .resize(100, raw.target.skill_types[0].clone());
    assert!(matches!(
        OwnedSupportInputBindings::new(
            raw,
            &fixture.schema,
            &fixture.rules,
            &fixture.preparation,
            &fixture.stages,
            SupportInputStorageLimits {
                max_entries: 20,
                ..Default::default()
            }
        ),
        Err(SupportInputStorageError::Limit("entries"))
    ));
}
