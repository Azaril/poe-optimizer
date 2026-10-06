//! Source relation storage uses the existing receiving fixture, not a second model.
use super::*;
use poe_optimizer_core::{owned_readiness::*, owned_source_properties::*};

fn source_owner() -> SchemaSubject {
    SchemaSubject::Definition(DefinitionAddress::Skill(id("parent")))
}
fn external_owner() -> SchemaSubject {
    SchemaSubject::Definition(DefinitionAddress::Modifier(id("source-modifier")))
}
fn property(name: &str, context: RuleEntityKind) -> RuleProgram {
    RuleProgram {
        id: key(name),
        context,
        reads: vec![],
        nodes: vec![RuleNode {
            id: key("value"),
            expression: RuleExpression::Literal {
                value: ParameterValue::Integer(BoundedInteger::new(2).unwrap()),
            },
        }],
        effects: vec![RuleEffect {
            id: key("property"),
            when: None,
            effect: RuleEffectKind::Contribute {
                entity: RuleEntity::PropertyOwner,
                stat: id("source-property"),
                contribution: ContributionKind::Add,
                value: key("value"),
            },
        }],
    }
}
fn assembly() -> RuleProgram {
    RuleProgram {
        id: key("source-assembly"),
        context: RuleEntityKind::Skill,
        reads: vec![RuleRead {
            id: key("incoming"),
            value_type: ComputedValueType::Integer,
            source: RuleReadSource::Contributions {
                entity: RuleEntity::PropertyOwner,
                stat: id("source-property"),
                contribution: ContributionKind::Add,
                reduction: ContributionReduction::Sum,
                empty: ParameterValue::Integer(BoundedInteger::new(0).unwrap()),
            },
        }],
        nodes: vec![RuleNode {
            id: key("value"),
            expression: RuleExpression::Read {
                input: key("incoming"),
            },
        }],
        effects: vec![RuleEffect {
            id: key("final"),
            when: None,
            effect: RuleEffectKind::Derive {
                entity: RuleEntity::Current,
                stat: id("source-final"),
                value: key("value"),
            },
        }],
    }
}
fn fixture() -> Fixture {
    fixture_with_rules(|_| {})
}
fn fixture_with_rules(edit: impl FnOnce(&mut RulePackageInput)) -> Fixture {
    fixture_with(|_| {}, edit)
}
fn fixture_with(
    edit_schema: impl FnOnce(&mut SchemaPackageInput),
    edit: impl FnOnce(&mut RulePackageInput),
) -> Fixture {
    let mut f = Fixture::with(
        |schema| {
            schema
                .definitions
                .push(DefinitionDescriptor::Modifier(entry(
                    id("source-modifier"),
                    ModifierSchema {
                        declarations: ports(),
                    },
                )));
            for name in [
                "source-count",
                "source-input",
                "source-property",
                "source-final",
            ] {
                schema.definitions.push(DefinitionDescriptor::Stat(entry(
                    id(name),
                    StatSchema {
                        value: ComputedValueType::Integer,
                        targets: vec![RuleEntityKind::Skill],
                    },
                )));
            }
            edit_schema(schema);
        },
        |rules| {
            rules.operations_version = key(OWNED_RULE_OPERATIONS_V18);
            rules.effect_applications = Some(complete(vec![]));
            rules.owners.push(DefinitionRules {
                owner: external_owner(),
                programs: complete(vec![property(
                    "source-external",
                    RuleEntityKind::EquipmentUse,
                )]),
            });
            rules.owners.push(DefinitionRules {
                owner: source_owner(),
                programs: complete(vec![assembly()]),
            });
            rules
                .owners
                .iter_mut()
                .find(|o| o.owner == owner("support-a"))
                .unwrap()
                .programs
                .members
                .push(property("source-supported", RuleEntityKind::SupportOrigin));
            edit(rules);
        },
        |stages| {
            stages.schema_version = OWNED_EVALUATION_STAGES_V3;
            stages.effect_applications = Some(complete(vec![]));
            for (name, before) in [
                ("source-census", "prepare"),
                ("source-properties", "source-census"),
                ("source-assembly", "source-properties"),
            ] {
                stages.stages.push(EvaluationStage {
                    id: key(name),
                    predecessors: vec![key(before)],
                });
            }
            let programs = stages
                .programs
                .members
                .iter_mut()
                .map(|p| {
                    let (role, outputs) = match p.program.as_str() {
                        "source-external" | "source-supported" => {
                            p.stage = key("source-properties");
                            (
                                if p.program == key("source-external") {
                                    ReadinessProgramRole::SourceExternalProperty
                                } else {
                                    ReadinessProgramRole::SourceSupportedProperty
                                },
                                vec![StageChannel::Contributions {
                                    scope: RuleEntityKind::Skill,
                                    stat: id("source-property"),
                                    contribution: ContributionKind::Add,
                                }],
                            )
                        }
                        "source-assembly" => {
                            p.stage = key("source-assembly");
                            (
                                ReadinessProgramRole::SourceFinalInputAssembly,
                                vec![StageChannel::Stat {
                                    scope: RuleEntityKind::Skill,
                                    stat: id("source-final"),
                                }],
                            )
                        }
                        _ => (ReadinessProgramRole::Execution, vec![]),
                    };
                    ReadinessProgram {
                        owner: p.owner.clone(),
                        program: p.program.clone(),
                        phase: if role == ReadinessProgramRole::Execution {
                            ReadinessPhase::Execution
                        } else {
                            ReadinessPhase::Preparation
                        },
                        role,
                        outputs,
                    }
                })
                .collect();
            stages.readiness = Some(ReadinessInput {
                skills: vec![],
                programs: complete(programs),
            });
        },
    );
    let mut prep = f.preparation.input().clone();
    prep.effects.push(key("source-support-effect"));
    for name in ["support-a", "support-b"] {
        prep.supports.push(SupportPreparationEntry {
            gem: id(name),
            preparation: SchemaState::Known(SupportPreparationDefinition {
                effect: key("source-support-effect"),
                families: None,
                plus_version_of: None,
                requires: None,
                excludes: None,
                added_types: vec![],
                gems_only: false,
                from_item: false,
                is_support: true,
                is_trigger: false,
                ignore_minion_types: false,
            }),
        });
    }
    f.preparation =
        OwnedSupportPreparation::new(prep, &f.schema, &f.rules, Default::default()).unwrap();
    let mut inputs = f.inputs.input().clone();
    inputs.preparation = *f.preparation.identity();
    f.inputs = OwnedSupportInputBindings::new(
        inputs,
        &f.schema,
        &f.rules,
        &f.preparation,
        &f.stages,
        Default::default(),
    )
    .unwrap();
    f.input.schema_version = OWNED_SUPPORT_RECEIVING_V3;
    f.input.preparation = *f.preparation.identity();
    f.input.inputs = *f.inputs.identity();
    f.input.source_properties = Some(SourcePropertyPreparationInput {
        relations: complete(vec![SourcePropertyRelation {
            id: key("source"),
            owner: SupportTargetDefinition::Skill(id("parent")),
            occurrence: SourcePropertyOccurrence::AuthoredSkillUseV1,
            aliases: SourcePropertyAliasPolicy::RejectSharedBackingGemV1,
            context: SourcePropertyContext::PlayerScenarioV1,
            census: SourcePropertyCensus::ExactSelectedPositionV1,
            census_stage: key("source-census"),
            effects: complete(vec![SourcePropertyEffect {
                endpoint: SourcePropertyEffectEndpoint::DirectOwner {},
                admission: SupportAdmissionContext::AssignedSkill,
            }]),
            inputs: vec![id("source-input")],
            channels: complete(vec![SourcePropertyChannel {
                stat: id("source-property"),
                contribution: ContributionKind::Add,
            }]),
            external: complete(vec![SourcePropertyExternalProgram {
                owner: external_owner(),
                program: key("source-external"),
            }]),
            supports: complete(vec![SourcePropertySupportPrograms {
                gem: id("support-a"),
                counted: true,
                programs: complete(vec![key("source-supported")]),
            }]),
            assembly: complete(vec![key("source-assembly")]),
            non_hidden_count: id("source-count"),
        }]),
    });
    f
}
fn relation(input: &mut SupportReceivingInput) -> &mut SourcePropertyRelation {
    &mut input.source_properties.as_mut().unwrap().relations.members[0]
}

#[test]
fn source_v3_is_canonical_bound_and_preserves_legacy_wire() {
    let f = fixture();
    let package = f.build(f.input.clone()).unwrap();
    assert!(package.declarations_complete());
    assert_eq!(
        package.source_properties().unwrap().relations.members.len(),
        1
    );
    let bytes = encode_support_receiving(&package, Default::default()).unwrap();
    let rebuilt = decode_support_receiving(
        &bytes,
        &f.schema,
        &f.rules,
        &f.preparation,
        &f.inputs,
        &f.stages,
        Default::default(),
    )
    .unwrap();
    assert_eq!(package.identity(), rebuilt.identity());
    assert_eq!(
        bytes,
        encode_support_receiving(&rebuilt, Default::default()).unwrap()
    );
    let legacy = Fixture::new();
    let before = legacy.build(legacy.input.clone()).unwrap();
    let raw = serde_json::to_value(before.input()).unwrap();
    assert!(raw.get("source_properties").is_none());
    let round: SupportReceivingInput = serde_json::from_value(raw.clone()).unwrap();
    assert!(round.source_properties.is_none());
    assert_eq!(legacy.build(round).unwrap().identity(), before.identity());
    let mut null = raw;
    null["source_properties"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<SupportReceivingInput>(null).is_err());
    assert_eq!(OWNED_RULE_OPERATIONS_VERSION, OWNED_RULE_OPERATIONS_V14);
    assert_eq!(OWNED_SUPPORT_RECEIVING_VERSION, 1);
    assert_eq!(OWNED_EVALUATION_STAGES_VERSION, 1);
}

#[test]
fn source_receiving_v3_accepts_bound_v4_stages_without_gaining_ordinary_property_authority() {
    let mut f = fixture();
    let old = f.build(f.input.clone()).unwrap();
    let routing = OwnedActionRouting::new(
        ActionRoutingInput {
            schema_version: OWNED_ACTION_ROUTING_VERSION,
            namespace: ns(),
            release: key("routing"),
            definitions: f.schema.identity().clone(),
            outputs: vec![],
        },
        &f.schema,
        Default::default(),
    )
    .unwrap();
    let mut stages = f.stages.input().clone();
    stages.schema_version = OWNED_EVALUATION_STAGES_V4;
    f.stages = OwnedEvaluationStages::new(
        stages.clone(),
        &f.schema,
        &f.rules,
        &routing,
        Default::default(),
    )
    .unwrap();
    let mut inputs = f.inputs.input().clone();
    inputs.stages = *f.stages.identity();
    f.inputs = OwnedSupportInputBindings::new(
        inputs,
        &f.schema,
        &f.rules,
        &f.preparation,
        &f.stages,
        Default::default(),
    )
    .unwrap();
    f.input.stages = *f.stages.identity();
    f.input.inputs = *f.inputs.identity();
    let checked = f.build(f.input.clone()).unwrap();
    assert_ne!(checked.identity(), old.identity());
    assert_eq!(
        checked.input().source_properties,
        old.input().source_properties
    );
    let bytes = encode_support_receiving(&checked, Default::default()).unwrap();
    let round = decode_support_receiving(
        &bytes,
        &f.schema,
        &f.rules,
        &f.preparation,
        &f.inputs,
        &f.stages,
        Default::default(),
    )
    .unwrap();
    assert_eq!(checked.identity(), round.identity());

    // Local item Derive authority is independent of relation-only PropertyOwner.
    let row = stages
        .readiness
        .as_mut()
        .unwrap()
        .programs
        .members
        .iter_mut()
        .find(|p| p.program == key("source-external"))
        .unwrap();
    row.role = ReadinessProgramRole::PreparationFacts;
    assert!(matches!(
        OwnedEvaluationStages::new(stages, &f.schema, &f.rules, &routing, Default::default(),),
        Err(StageStorageError::Invalid(
            "source property scope requires an explicit V3 source role"
        ))
    ));
}

#[test]
fn preset_input_operations_keep_checked_v3_readiness_and_source_authority() {
    let f = fixture_with(
        |schema| schema.schema_version = OWNED_SCHEMA_PACKAGE_V6,
        |rules| rules.operations_version = key(OWNED_RULE_OPERATIONS_V19),
    );
    let stored = f.build(f.input.clone()).unwrap();
    let bytes = encode_support_receiving(&stored, Default::default()).unwrap();
    let round = decode_support_receiving(
        &bytes,
        &f.schema,
        &f.rules,
        &f.preparation,
        &f.inputs,
        &f.stages,
        Default::default(),
    )
    .unwrap();
    assert_eq!(stored.identity(), round.identity());
    assert_eq!(f.stages.input().schema_version, OWNED_EVALUATION_STAGES_V3);
    let routing = OwnedActionRouting::new(
        ActionRoutingInput {
            schema_version: OWNED_ACTION_ROUTING_VERSION,
            namespace: ns(),
            release: key("routing"),
            definitions: f.schema.identity().clone(),
            outputs: vec![],
        },
        &f.schema,
        Default::default(),
    )
    .unwrap();
    for version in [OWNED_EVALUATION_STAGES_VERSION, OWNED_EVALUATION_STAGES_V2] {
        let mut raw = f.stages.input().clone();
        raw.schema_version = version;
        if version == OWNED_EVALUATION_STAGES_VERSION {
            raw.readiness = None;
        }
        assert!(matches!(
            OwnedEvaluationStages::new(raw, &f.schema, &f.rules, &routing, Default::default()),
            Err(StageStorageError::Invalid(_))
        ));
    }
}

#[test]
fn all_source_inventories_require_complete_membership() {
    let f = fixture();
    for target in 0..7 {
        let mut raw = f.input.clone();
        let closure = partial(source_owner());
        match target {
            0 => raw.source_properties.as_mut().unwrap().relations.closure = closure,
            1 => relation(&mut raw).effects.closure = closure,
            2 => relation(&mut raw).channels.closure = closure,
            3 => relation(&mut raw).external.closure = closure,
            4 => relation(&mut raw).supports.closure = closure,
            5 => relation(&mut raw).assembly.closure = closure,
            6 => relation(&mut raw).supports.members[0].programs.closure = closure,
            _ => unreachable!(),
        }
        f.bad(raw, "Complete");
    }
}

#[test]
fn source_wire_is_strict_and_versions_do_not_gain_authority() {
    let f = fixture();
    for version in [1, 2, 4] {
        let mut raw = f.input.clone();
        raw.schema_version = version;
        assert!(f.build(raw).is_err());
    }
    let mut absent = f.input.clone();
    absent.source_properties = None;
    f.bad(absent, "receiving V3");
    for pointer in [
        "/source_properties",
        "/source_properties/relations/members/0",
        "/source_properties/relations/members/0/effects/members/0/endpoint",
    ] {
        let mut value = serde_json::to_value(&f.input).unwrap();
        value.pointer_mut(pointer).unwrap()["unknown"] = true.into();
        assert!(
            serde_json::from_value::<SupportReceivingInput>(value).is_err(),
            "unknown field accepted at {pointer}"
        );
    }
    for field in ["aliases", "context", "census", "occurrence"] {
        let mut value = serde_json::to_value(&f.input).unwrap();
        value["source_properties"]["relations"]["members"][0][field] = "invented".into();
        assert!(serde_json::from_value::<SupportReceivingInput>(value).is_err());
    }
    assert_eq!(
        serde_json::to_value(SourcePropertyEffectEndpoint::DirectOwner {}).unwrap(),
        serde_json::json!({ "kind": "direct_owner" }),
        "strict empty-struct decoding must preserve the Direct endpoint wire"
    );
    let mut generated = serde_json::to_value(SourcePropertyEffectEndpoint::Generated {
        path: vec![],
        skill_supply: slot(parent(), "source-supply"),
    })
    .unwrap();
    assert!(serde_json::from_value::<SourcePropertyEffectEndpoint>(generated.clone()).is_ok());
    generated["unknown"] = true.into();
    assert!(serde_json::from_value::<SourcePropertyEffectEndpoint>(generated).is_err());
}

#[test]
fn source_channels_inputs_programs_and_stage_order_are_exact() {
    let f = fixture();
    let mut raw = f.input.clone();
    relation(&mut raw).non_hidden_count = id("flag");
    f.bad(raw, "Integer Skill");
    let mut raw = f.input.clone();
    relation(&mut raw).inputs = vec![id("level")];
    f.bad(raw, "Skill scope");
    let mut raw = f.input.clone();
    relation(&mut raw).channels.members[0].stat = id("flag");
    f.bad(raw, "typed Skill");
    let mut raw = f.input.clone();
    relation(&mut raw).channels.members.clear();
    f.bad(raw, "undeclared channel");
    let mut raw = f.input.clone();
    relation(&mut raw).external.members.clear();
    f.bad(raw, "lacks a complete relation");
    let mut raw = f.input.clone();
    relation(&mut raw).assembly.members = vec![key("unknown")];
    f.bad(raw, "unknown source property program");
    let mut raw = f.input.clone();
    relation(&mut raw).census_stage = key("source-properties");
    f.bad(raw, "follow census");
    let mut raw = f.input.clone();
    relation(&mut raw).census_stage = key("prepare");
    f.bad(raw, "follow support preparation");
    let mut raw = f.input.clone();
    relation(&mut raw).supports.members[0].gem = id("active");
    f.bad(raw, "known preparation");
}

#[test]
fn source_endpoints_cannot_alias_duplicate_or_infer_owned_actor_membership() {
    let f = fixture();
    let mut raw = f.input.clone();
    let duplicate = relation(&mut raw).effects.members[0].clone();
    relation(&mut raw).effects.members.push(duplicate);
    f.bad(raw, "duplicate");
    let mut raw = f.input.clone();
    relation(&mut raw).effects.members.clear();
    f.bad(raw, "eligible effects");
    let mut raw = f.input.clone();
    relation(&mut raw).effects.members[0].admission = SupportAdmissionContext::ReceivingSkill {
        summoner_path: None,
    };
    f.bad(raw, "Direct owner admission");
    let mut raw = f.input.clone();
    relation(&mut raw).effects.members[0] = SourcePropertyEffect {
        endpoint: SourcePropertyEffectEndpoint::Generated {
            path: vec![actor_grant()],
            skill_supply: slot(actor(), "child"),
        },
        admission: SupportAdmissionContext::ReceivingSkill {
            summoner_path: None,
        },
    };
    f.bad(raw, "Player skill providers");
    let mut raw = f.input.clone();
    relation(&mut raw).effects.members[0] = SourcePropertyEffect {
        endpoint: SourcePropertyEffectEndpoint::Generated {
            path: vec![],
            skill_supply: slot(child(), "foreign"),
        },
        admission: SupportAdmissionContext::ReceivingSkill {
            summoner_path: None,
        },
    };
    f.bad(raw, "another provider");
}

#[test]
fn source_direct_owner_requires_a_directly_selectable_skill_schema() {
    let f = fixture_with(
        |schema| {
            let row = schema
                .definitions
                .iter_mut()
                .find_map(|definition| match definition {
                    DefinitionDescriptor::Skill(row)
                        if row.id == id::<SkillDefinition>("parent") =>
                    {
                        Some(row)
                    }
                    _ => None,
                })
                .unwrap();
            let SchemaState::Known(skill) = &mut row.schema else {
                unreachable!()
            };
            skill.directly_selectable = false;
        },
        |_| {},
    );
    f.bad(f.input.clone(), "not directly selectable");
}

#[test]
fn source_storage_bounds_are_tighten_only_and_charge_new_rows() {
    let f = fixture();
    let full = f.build(f.input.clone()).unwrap();
    let limit = SupportReceivingStorageLimits {
        max_entries: full.resources().entries - 1,
        ..SupportReceivingStorageLimits::default()
    };
    assert!(matches!(
        f.build_with(f.input.clone(), limit),
        Err(SupportReceivingStorageError::Limit("entries"))
    ));
    let limit = SupportReceivingStorageLimits {
        max_work: 1,
        ..SupportReceivingStorageLimits::default()
    };
    assert!(matches!(
        f.build_with(f.input.clone(), limit),
        Err(SupportReceivingStorageError::Limit("work"))
    ));
    let mut limit = SupportReceivingStorageLimits::default();
    limit.max_work += 1;
    assert!(matches!(
        f.build_with(f.input.clone(), limit),
        Err(SupportReceivingStorageError::InvalidLimit("work"))
    ));
}

#[test]
fn complete_empty_program_lists_do_not_launder_partial_rule_owners() {
    let f = fixture_with_rules(|rules| {
        rules
            .owners
            .iter_mut()
            .find(|o| o.owner == owner("support-b"))
            .unwrap()
            .programs
            .closure = partial(owner("support-b"));
    });
    let mut raw = f.input.clone();
    relation(&mut raw)
        .supports
        .members
        .push(SourcePropertySupportPrograms {
            gem: id("support-b"),
            counted: true,
            programs: complete(vec![]),
        });
    f.bad(raw, "Complete");
    let f = fixture_with_rules(|rules| {
        let row = rules
            .owners
            .iter_mut()
            .find(|o| o.owner == source_owner())
            .unwrap();
        row.programs.members.clear();
        row.programs.closure = partial(source_owner());
    });
    let mut raw = f.input.clone();
    relation(&mut raw).assembly.members.clear();
    f.bad(raw, "Complete");
}

#[test]
fn property_scope_cannot_downgrade_or_acquire_ordinary_role_authority() {
    let f = fixture();
    let mut old = f.rules.input().clone();
    old.operations_version = key(OWNED_RULE_OPERATIONS_V17);
    assert!(
        OwnedRulePackage::new(old, &f.schema, Default::default())
            .unwrap_err()
            .to_string()
            .contains("v18")
    );
    let routing = OwnedActionRouting::new(
        ActionRoutingInput {
            schema_version: OWNED_ACTION_ROUTING_VERSION,
            namespace: ns(),
            release: key("routing"),
            definitions: f.schema.identity().clone(),
            outputs: vec![],
        },
        &f.schema,
        Default::default(),
    )
    .unwrap();
    for ordinary in [
        ReadinessProgramRole::PreparationFacts,
        ReadinessProgramRole::Execution,
    ] {
        let mut stages = f.stages.input().clone();
        let row = stages
            .readiness
            .as_mut()
            .unwrap()
            .programs
            .members
            .iter_mut()
            .find(|p| p.program == key("source-external"))
            .unwrap();
        row.role = ordinary;
        if ordinary == ReadinessProgramRole::Execution {
            row.phase = ReadinessPhase::Execution;
            row.outputs.clear();
        }
        assert!(
            OwnedEvaluationStages::new(stages, &f.schema, &f.rules, &routing, Default::default())
                .unwrap_err()
                .to_string()
                .contains("explicit V3 source role")
        );
    }
    let mut stages = f.stages.input().clone();
    stages.schema_version = OWNED_EVALUATION_STAGES_V2;
    assert!(
        OwnedEvaluationStages::new(stages, &f.schema, &f.rules, &routing, Default::default())
            .is_err()
    );
}
