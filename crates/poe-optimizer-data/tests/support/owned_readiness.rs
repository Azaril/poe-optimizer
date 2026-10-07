//! Readiness authoring rejects incomplete or ambiguous early authority.
use super::*;
use poe_optimizer_core::{
    owned_readiness::*,
    owned_support_receiving::{OWNED_SUPPORT_RECEIVING_VERSION, SupportRolePrograms},
};
fn parameter() -> DeclaredSlot<ParameterSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(id("skill")),
        slot: id("final-level"),
    }
}
fn partial(subject: SchemaSubject) -> SchemaClosure {
    SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject,
            facet: SchemaFacet::InputSchema,
            code: key("unknown-inputs"),
        }],
    }
}
fn fixture() -> Fixture {
    let mut f = Fixture::new();
    let mut schema = f.schema.input().clone();
    for definition in &mut schema.definitions {
        if let DefinitionDescriptor::Skill(DefinitionEntry {
            schema: SchemaState::Known(skill),
            ..
        }) = definition
        {
            skill.declarations.parameters = DeclaredSet::complete(vec![parameter()]);
        }
    }
    schema.slots.push(SlotDescriptor::Parameter(entry(
        parameter(),
        ParameterSlotSchema {
            skill_input: None,
            value: ValueSchema::Integer(IntegerRange {
                minimum: BoundedInteger::new(1).unwrap(),
                maximum: BoundedInteger::new(100).unwrap(),
            }),
            presence: SlotPresence::RequiredOnce,
            sites: vec![],
        },
    )));
    f.schema = OwnedDefinitionSchemaPackage::new(schema, OwnedSchemaLimits::default()).unwrap();
    let mut rules = f.rules.input().clone();
    rules.definitions = f.schema.identity().clone();
    rules.operations_version = key(OWNED_RULE_OPERATIONS_V16);
    rules.effect_applications = Some(empty());
    f.rules = OwnedRulePackage::new(rules, &f.schema, RuleStorageLimits::default()).unwrap();
    let mut routing = f.routing.input().clone();
    routing.definitions = f.schema.identity().clone();
    f.routing = OwnedActionRouting::new(routing, &f.schema, RoutingLimits::default()).unwrap();
    f.input.definitions = f.schema.identity().clone();
    f.input.rules = *f.rules.identity();
    f.input.routing = *f.routing.identity();
    f.input.schema_version = OWNED_EVALUATION_STAGES_V2;
    f.input.effect_applications = Some(empty());
    f.input.readiness = Some(ReadinessInput {
        skills: vec![SkillReadiness {
            participation: None,
            skill: id("skill"),
            parameters: DeclaredSet::complete(vec![ParameterReadiness {
                parameter: parameter(),
                phase: ReadinessPhase::Execution,
            }]),
        }],
        programs: DeclaredSet::complete(vec![
            ReadinessProgram {
                owner: owner("a"),
                program: key("same-local-id"),
                phase: ReadinessPhase::Preparation,
                role: ReadinessProgramRole::PreparationFacts,
                outputs: vec![stat("a")],
            },
            ReadinessProgram {
                owner: owner("b"),
                program: key("same-local-id"),
                phase: ReadinessPhase::Execution,
                role: ReadinessProgramRole::Execution,
                outputs: vec![],
            },
        ]),
    });
    f
}
#[test]
fn checked_readiness_is_canonical_indexed_and_bound_to_v16() {
    let mut f = fixture();
    let package = f.package().unwrap();
    assert_eq!(
        package.parameter_phase(&id("skill"), &parameter()),
        Some(ReadinessPhase::Execution)
    );
    assert_eq!(
        package
            .program_readiness(&owner("a"), &key("same-local-id"))
            .unwrap()
            .phase,
        ReadinessPhase::Preparation
    );
    assert!(package.skill_readiness(&id("skill")).is_some());
    let bytes = encode_evaluation_stages(&package, StageStorageLimits::default()).unwrap();
    let decoded = decode_evaluation_stages(
        &bytes,
        &f.schema,
        &f.rules,
        &f.routing,
        StageStorageLimits::default(),
    )
    .unwrap();
    assert_eq!(package.identity(), decoded.identity());
    f.input
        .readiness
        .as_mut()
        .unwrap()
        .programs
        .members
        .reverse();
    assert_eq!(f.package().unwrap().identity(), package.identity());
    f.change_rules(|r| r.operations_version = key(OWNED_RULE_OPERATIONS_V15));
    assert!(f.package().is_err());
}
#[test]
fn inherited_source_readiness_requires_v3_stages_through_v21() {
    // V21 adds ordered reads and retains the explicit V18+ stage contract.
    // A caller upgrading operations must also supply the versioned stage DTO.
    for operations in [
        OWNED_RULE_OPERATIONS_V18,
        OWNED_RULE_OPERATIONS_V19,
        OWNED_RULE_OPERATIONS_V20,
        OWNED_RULE_OPERATIONS_V21,
    ] {
        let mut f = fixture();
        f.change_rules(|r| {
            r.operations_version = key(operations);
            if operations == OWNED_RULE_OPERATIONS_V21 {
                r.contribution_queries = Some(DeclaredSet::complete(vec![]));
            }
        });
        assert!(
            matches!(
                f.package(),
                Err(StageStorageError::Invalid(
                    "readiness requires operations V16 and stages V2 with explicit metadata"
                ))
            ),
            "{operations} cannot use stages V2"
        );
        f.input.schema_version = OWNED_EVALUATION_STAGES_V3;
        let package = f.package().unwrap();
        let bytes = encode_evaluation_stages(&package, StageStorageLimits::default()).unwrap();
        let decoded = decode_evaluation_stages(
            &bytes,
            &f.schema,
            &f.rules,
            &f.routing,
            StageStorageLimits::default(),
        )
        .unwrap();
        assert_eq!(package.identity(), decoded.identity());
        assert_eq!(
            decoded.parameter_phase(&id("skill"), &parameter()),
            Some(ReadinessPhase::Execution)
        );
    }
    for stages in [OWNED_EVALUATION_STAGES_V2, OWNED_EVALUATION_STAGES_V3] {
        let mut f = fixture();
        f.change_rules(|r| {
            r.operations_version = key("owned-domain-operations-v999");
            r.effect_applications = None;
        });
        f.input.effect_applications = None;
        f.input.schema_version = stages;
        assert!(
            matches!(
                f.package(),
                Err(StageStorageError::Invalid(
                    "readiness requires operations V16 and stages V2 with explicit metadata"
                ))
            ),
            "unknown operations cannot inherit readiness"
        );
    }
}
#[test]
fn omission_is_legacy_only_and_explicit_null_never_requests_early_authority() {
    assert_eq!(OWNED_RULE_OPERATIONS_VERSION, OWNED_RULE_OPERATIONS_V14);
    assert_eq!(OWNED_EVALUATION_STAGES_VERSION, 1);
    assert_eq!(OWNED_SUPPORT_RECEIVING_VERSION, 1);
    let f = Fixture::new();
    let package = f.package().unwrap();
    let bytes = encode_evaluation_stages(&package, StageStorageLimits::default()).unwrap();
    let mut wire: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(wire.get("readiness").is_none());
    let decoded: EvaluationStagesInput = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_vec(&decoded).unwrap(), bytes);
    wire["readiness"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<EvaluationStagesInput>(wire).is_err());
    let row = SupportRolePrograms {
        role: key("r"),
        applicability: key("a"),
        delivery: vec![],
        preparation: None,
    };
    let mut wire = serde_json::to_value(&row).unwrap();
    assert!(wire.get("preparation").is_none());
    wire["preparation"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<SupportRolePrograms>(wire).is_err());
    let mut f = fixture();
    f.input.readiness = None;
    assert!(f.package().is_err());
    f.input.schema_version = 1;
    assert!(f.package().is_err());
}
#[test]
fn required_input_partition_rejects_missing_duplicate_foreign_and_partial_rows() {
    for mutation in 0..5 {
        let mut f = fixture();
        let row = &mut f.input.readiness.as_mut().unwrap().skills[0];
        match mutation {
            0 => row.parameters.members.clear(),
            1 => row
                .parameters
                .members
                .push(row.parameters.members[0].clone()),
            2 => row.parameters.members[0].parameter.slot = id("unknown"),
            3 => {
                row.parameters.closure = partial(SchemaSubject::Definition(
                    id::<SkillDefinition>("skill").address(),
                ))
            }
            _ => {
                row.parameters.members[0].parameter.declaration = SlotOwnerDefId::Skill(id("other"))
            }
        }
        assert!(
            f.package().is_err(),
            "invalid partition {mutation} accepted"
        );
    }
}
#[test]
fn complete_program_partition_and_exact_typed_outputs_are_mandatory() {
    for mutation in 0..7 {
        let mut f = fixture();
        let readiness = f.input.readiness.as_mut().unwrap();
        match mutation {
            0 => readiness.programs.members.clear(),
            1 => readiness
                .programs
                .members
                .push(readiness.programs.members[0].clone()),
            2 => readiness.programs.closure = partial(owner("a")),
            3 => readiness.programs.members[0].outputs.clear(),
            4 => readiness.programs.members[0].outputs.push(stat("a")),
            5 => {
                readiness.programs.members[0].outputs = vec![StageChannel::Stat {
                    scope: RuleEntityKind::Skill,
                    stat: id("a"),
                }]
            }
            _ => readiness.programs.members[0].phase = ReadinessPhase::Execution,
        }
        assert!(
            f.package().is_err(),
            "invalid program declaration {mutation} accepted"
        );
    }
}
#[test]
fn early_roles_cannot_relabel_execution_support_or_final_outputs() {
    for role in [
        ReadinessProgramRole::Execution,
        ReadinessProgramRole::FinalInputAssembly,
        ReadinessProgramRole::SupportedPreparationProperty,
        ReadinessProgramRole::SupportPreparationApplicability,
    ] {
        let mut f = fixture();
        f.input.readiness.as_mut().unwrap().programs.members[0].role = role;
        assert!(f.package().is_err());
    }
    let mut f = fixture();
    f.change_rules(|rules| {
        rules.owners[1].programs.members[0].effects[0].effect = RuleEffectKind::Derive {
            entity: RuleEntity::Current,
            stat: id("a"),
            value: key("one"),
        }
    });
    assert!(f.package().is_err());
}
#[test]
fn readiness_uses_the_existing_wire_entry_and_work_budgets() {
    let f = fixture();
    let package = f.package().unwrap();
    let used = package.resources();
    for limits in [
        StageStorageLimits {
            max_entries: used.entries - 1,
            ..Default::default()
        },
        StageStorageLimits {
            max_work: used.work - 1,
            ..Default::default()
        },
        StageStorageLimits {
            max_wire_bytes: 1,
            ..Default::default()
        },
    ] {
        assert!(
            OwnedEvaluationStages::new(f.input.clone(), &f.schema, &f.rules, &f.routing, limits)
                .is_err()
        );
    }
}

#[test]
fn partial_schema_inputs_and_early_owner_rules_never_authorize_early_readiness() {
    let mut f = fixture();
    let mut schema = f.schema.input().clone();
    for definition in &mut schema.definitions {
        if let DefinitionDescriptor::Skill(DefinitionEntry {
            schema: SchemaState::Known(skill),
            ..
        }) = definition
        {
            skill.declarations.parameters.closure = partial(SchemaSubject::Definition(
                id::<SkillDefinition>("skill").address(),
            ));
        }
    }
    f.schema = OwnedDefinitionSchemaPackage::new(schema, OwnedSchemaLimits::default()).unwrap();
    let identity = f.schema.identity().clone();
    f.change_rules(|rules| rules.definitions = identity.clone());
    f.change_routing(|routing| routing.definitions = identity.clone());
    f.input.definitions = identity;
    assert!(f.package().is_err());
    let mut f = fixture();
    f.change_rules(|rules| {
        rules.owners[0].programs.closure = SchemaClosure::Partial {
            gaps: vec![SchemaGap {
                subject: owner("a"),
                facet: SchemaFacet::GameRules,
                code: key("missing-rules"),
            }],
        }
    });
    assert!(f.package().is_err());
}
#[test]
fn potential_final_writers_are_checked_before_conditions() {
    let mut f = fixture();
    f.change_rules(|rules| {
        let program = &mut rules.owners[0].programs.members[0];
        program.nodes.push(RuleNode {
            id: key("disabled"),
            expression: RuleExpression::Literal {
                value: ParameterValue::Boolean(false),
            },
        });
        let mut duplicate = program.effects[0].clone();
        duplicate.id = key("duplicate");
        duplicate.when = Some(key("disabled"));
        program.effects.push(duplicate);
    });
    assert!(f.package().is_err());
}

#[test]
fn distinct_relative_final_destinations_remain_for_concrete_occurrence_binding() {
    let mut f = fixture();
    f.change_rules(|rules| {
        let program = &mut rules.owners[0].programs.members[0];
        program.effects.push(RuleEffect {
            id: key("player-result"),
            when: None,
            effect: RuleEffectKind::Derive {
                entity: RuleEntity::Player,
                stat: id("a"),
                value: key("one"),
            },
        });
    });
    // Current can denote an owned actor; Player denotes a different actor. The
    // concrete resolver still rejects aliases when Current actually is Player.
    assert!(f.package().is_ok());
}
#[test]
fn ordinary_execution_conflicts_remain_the_concrete_retained_plan_responsibility() {
    let mut f = fixture();
    f.change_rules(|rules| {
        let program = &mut rules.owners[1].programs.members[0];
        let mut duplicate = program.effects[0].clone();
        duplicate.id = key("other-execution-result");
        program.effects.push(duplicate);
    });
    // Readiness must not make unretained ordinary templates compete. Existing
    // final-plan compilation still validates every concrete retained writer.
    assert!(f.package().is_ok());
}

fn local_item_fixture(
    context: RuleEntityKind,
    destination: RuleEntity,
    value_type: ComputedValueType,
    value: ParameterValue,
) -> Fixture {
    let mut f = fixture();
    let scope = if destination == RuleEntity::Modifier {
        RuleEntityKind::Modifier
    } else {
        context
    };
    let local_owner =
        SchemaSubject::Definition(id::<ModifierDefinition>("local-modifier").address());
    let mut schema = f.schema.input().clone();
    schema.definitions.extend([
        DefinitionDescriptor::Modifier(entry(
            id("local-modifier"),
            ModifierSchema {
                declarations: DeclaredSlots {
                    parameters: empty(),
                    choices: empty(),
                    grants: empty(),
                    actors: empty(),
                    skill_grants: empty(),
                    outputs: empty(),
                    sockets: empty(),
                },
            },
        )),
        DefinitionDescriptor::Stat(entry(
            id("local-fact"),
            StatSchema {
                value: value_type,
                targets: vec![scope],
            },
        )),
        DefinitionDescriptor::Option(entry(id("local-option"), OptionSchema {})),
    ]);
    f.schema = OwnedDefinitionSchemaPackage::new(schema, OwnedSchemaLimits::default()).unwrap();
    let identity = f.schema.identity().clone();
    f.change_rules(|rules| {
        rules.definitions = identity.clone();
        rules.operations_version = key(OWNED_RULE_OPERATIONS_V20);
        let row = &mut rules.owners[0];
        row.owner = local_owner.clone();
        let program = &mut row.programs.members[0];
        program.context = context;
        program.nodes[0].expression = RuleExpression::Literal { value };
        program.effects[0].effect = RuleEffectKind::Derive {
            entity: destination,
            stat: id("local-fact"),
            value: key("one"),
        };
    });
    f.change_routing(|routing| routing.definitions = identity.clone());
    f.input.definitions = identity;
    f.input.schema_version = OWNED_EVALUATION_STAGES_V4;
    f.input.programs.members[0].owner = local_owner.clone();
    let output = StageChannel::Stat {
        scope,
        stat: id("local-fact"),
    };
    f.input.frozen_channels[0].channel = output.clone();
    let row = &mut f.input.readiness.as_mut().unwrap().programs.members[0];
    row.owner = local_owner;
    row.outputs = vec![output];
    f
}

#[test]
fn local_item_derivations_require_v4_and_preserve_exact_typed_outputs() {
    for (context, destination) in [
        (RuleEntityKind::EquipmentUse, RuleEntity::Current),
        (RuleEntityKind::EquipmentUse, RuleEntity::Modifier),
        (RuleEntityKind::Modifier, RuleEntity::Current),
    ] {
        for (value_type, value) in [
            (ComputedValueType::Boolean, ParameterValue::Boolean(true)),
            (
                ComputedValueType::Integer,
                ParameterValue::Integer(BoundedInteger::new(3).unwrap()),
            ),
            (
                ComputedValueType::Quantity { unit: id("factor") },
                ParameterValue::Quantity(
                    poe_optimizer_core::owned_definitions::FiniteQuantity::new(1.25, id("factor"))
                        .unwrap(),
                ),
            ),
            (
                ComputedValueType::Option,
                ParameterValue::Option(id("local-option")),
            ),
        ] {
            let mut f = local_item_fixture(context, destination, value_type, value);
            let checked = f.package().unwrap();
            let bytes = encode_evaluation_stages(&checked, StageStorageLimits::default()).unwrap();
            let decoded = decode_evaluation_stages(
                &bytes,
                &f.schema,
                &f.rules,
                &f.routing,
                StageStorageLimits::default(),
            )
            .unwrap();
            assert_eq!(checked.identity(), decoded.identity());
            for old_version in [
                OWNED_EVALUATION_STAGES_VERSION,
                OWNED_EVALUATION_STAGES_V2,
                OWNED_EVALUATION_STAGES_V3,
            ] {
                f.input.schema_version = old_version;
                assert!(
                    f.package().is_err(),
                    "old stages {old_version} gained local preparation authority"
                );
            }
            f.input.schema_version = 5;
            assert!(matches!(f.package(), Err(StageStorageError::Version(5))));
        }
    }
}

#[test]
fn v4_retains_source_operations_gate_and_has_its_own_identity_domain() {
    let mut f = fixture();
    f.change_rules(|rules| rules.operations_version = key(OWNED_RULE_OPERATIONS_V20));
    f.input.schema_version = OWNED_EVALUATION_STAGES_V3;
    let v3 = f.package().unwrap();
    f.input.schema_version = OWNED_EVALUATION_STAGES_V4;
    let v4 = f.package().unwrap();
    assert_ne!(v3.identity(), v4.identity());
    assert_eq!(
        *v4.identity(),
        poe_optimizer_core::owned_content::digest_owned(
            "owned-evaluation-stages-v4",
            v4.input(),
            StageStorageLimits::default().max_wire_bytes,
        )
        .unwrap(),
    );
    for operations in [
        OWNED_RULE_OPERATIONS_V16,
        OWNED_RULE_OPERATIONS_V17,
        "owned-domain-operations-v999",
    ] {
        let mut f = fixture();
        f.input.schema_version = OWNED_EVALUATION_STAGES_V4;
        f.change_rules(|rules| {
            rules.operations_version = key(operations);
            if operations == "owned-domain-operations-v999" {
                rules.effect_applications = None;
            }
        });
        if operations == "owned-domain-operations-v999" {
            f.input.effect_applications = None;
        }
        assert!(
            f.package().is_err(),
            "V4 accepted non-source/unknown operations {operations}"
        );
    }
    assert_eq!(OWNED_EVALUATION_STAGES_VERSION, 1);
    assert_eq!(OWNED_RULE_OPERATIONS_VERSION, OWNED_RULE_OPERATIONS_V14);
}

#[test]
fn v4_local_permission_does_not_grant_streams_capabilities_or_foreign_contexts() {
    for kind in 0..4 {
        let mut f = local_item_fixture(
            RuleEntityKind::EquipmentUse,
            if kind == 2 {
                RuleEntity::Modifier
            } else {
                RuleEntity::Current
            },
            ComputedValueType::Integer,
            ParameterValue::Integer(BoundedInteger::new(1).unwrap()),
        );
        f.change_rules(|rules| {
            let p = &mut rules.owners[0].programs.members[0];
            p.effects[0].effect = match kind {
                0 => RuleEffectKind::Contribute {
                    entity: RuleEntity::Current,
                    stat: id("local-fact"),
                    contribution: ContributionKind::Add,
                    value: key("one"),
                },
                1 => RuleEffectKind::Capability {
                    entity: RuleEntity::Current,
                    capability: id("eligibility"),
                    enabled: key("one"),
                },
                2 => RuleEffectKind::Derive {
                    entity: RuleEntity::Modifier,
                    stat: id("local-fact"),
                    value: key("one"),
                },
                _ => RuleEffectKind::ProjectModifierTransform {
                    stat: id("transform"),
                    targets: vec![ModifierTransformTarget {
                        definition: id("local-modifier"),
                        when: None,
                    }],
                    order: BoundedInteger::new(0).unwrap(),
                    operation: ModifierTransformOperation::Multiply,
                    value: key("one"),
                },
            };
            if kind == 1 {
                p.nodes[0].expression = RuleExpression::Literal {
                    value: ParameterValue::Boolean(true),
                };
            }
            if kind == 2 {
                p.context = RuleEntityKind::Actor;
            }
            if kind == 3 {
                p.nodes[0].expression = RuleExpression::Literal {
                    value: ParameterValue::Quantity(
                        poe_optimizer_core::owned_definitions::FiniteQuantity::new(
                            1.5,
                            id("factor"),
                        )
                        .unwrap(),
                    ),
                };
            }
        });
        let output = match kind {
            0 => StageChannel::Contributions {
                scope: RuleEntityKind::EquipmentUse,
                stat: id("local-fact"),
                contribution: ContributionKind::Add,
            },
            1 => StageChannel::Capability {
                scope: RuleEntityKind::EquipmentUse,
                capability: id("eligibility"),
            },
            2 => StageChannel::Stat {
                scope: RuleEntityKind::Modifier,
                stat: id("local-fact"),
            },
            _ => StageChannel::ModifierTransforms {
                stat: id("transform"),
            },
        };
        f.input.frozen_channels.clear();
        f.input.readiness.as_mut().unwrap().programs.members[0].outputs = vec![output];
        assert!(
            matches!(
                f.package(),
                Err(StageStorageError::Invalid(
                    "effect is not authorized by preparation output role"
                ))
            ),
            "local permission admitted unrelated effect/context {kind}"
        );
    }
}

#[test]
fn v4_local_facts_preserve_complete_owner_exact_outputs_and_potential_writer_checks() {
    for mutation in 0..5 {
        let mut f = local_item_fixture(
            RuleEntityKind::EquipmentUse,
            RuleEntity::Modifier,
            ComputedValueType::Integer,
            ParameterValue::Integer(BoundedInteger::new(1).unwrap()),
        );
        match mutation {
            0 => f.input.readiness.as_mut().unwrap().programs.members[0]
                .outputs
                .clear(),
            1 => f.change_rules(|rules| {
                let o = &mut rules.owners[0];
                o.programs.closure = SchemaClosure::Partial {
                    gaps: vec![SchemaGap {
                        subject: o.owner.clone(),
                        facet: SchemaFacet::GameRules,
                        code: key("missing-local-rules"),
                    }],
                };
            }),
            2 => f.change_rules(|rules| {
                let p = &mut rules.owners[0].programs.members[0];
                p.nodes.push(RuleNode {
                    id: key("false"),
                    expression: RuleExpression::Literal {
                        value: ParameterValue::Boolean(false),
                    },
                });
                let mut duplicate = p.effects[0].clone();
                duplicate.id = key("duplicate");
                duplicate.when = Some(key("false"));
                p.effects.push(duplicate);
            }),
            3 => {
                let row = &mut f.input.readiness.as_mut().unwrap().programs.members[0];
                row.outputs = vec![StageChannel::Stat {
                    scope: RuleEntityKind::EquipmentUse,
                    stat: id("local-fact"),
                }];
            }
            _ => {
                f.input.readiness.as_mut().unwrap().programs.members[0].phase =
                    ReadinessPhase::Execution
            }
        }
        assert!(
            f.package().is_err(),
            "local facts bypassed existing guarantee {mutation}"
        );
    }
}
