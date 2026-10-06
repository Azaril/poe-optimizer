//! Participation adds one execution requirement to the checked readiness graph.
use super::*;
use poe_optimizer_core::owned_readiness::*;

fn usage_owner() -> SchemaSubject {
    SchemaSubject::Definition(id::<UsagePolicyDefinition>("requested").address())
}
fn usage_parameter() -> DeclaredSlot<ParameterSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::UsagePolicy(id("requested")),
        slot: id("enabled"),
    }
}
fn participation_channel() -> StageChannel {
    StageChannel::Stat {
        scope: RuleEntityKind::Skill,
        stat: id("requested-participation"),
    }
}
fn fixture() -> Fixture {
    let mut f = Fixture::new();
    let mut schema = f.schema.input().clone();
    schema.schema_version = OWNED_SCHEMA_PACKAGE_V4;
    schema.definitions.extend([
        DefinitionDescriptor::Stat(entry(
            id("requested-participation"),
            StatSchema {
                value: ComputedValueType::Boolean,
                targets: vec![RuleEntityKind::Skill],
            },
        )),
        DefinitionDescriptor::UsagePolicy(entry(
            id("requested"),
            UsagePolicySchema {
                targets: vec![UsageTargetKind::Skill],
                declarations: DeclaredSlots {
                    parameters: DeclaredSet::complete(vec![usage_parameter()]),
                    choices: empty(),
                    grants: empty(),
                    actors: empty(),
                    skill_grants: empty(),
                    outputs: empty(),
                    sockets: empty(),
                },
            },
        )),
    ]);
    schema.slots.push(SlotDescriptor::Parameter(entry(
        usage_parameter(),
        ParameterSlotSchema {
            value: ValueSchema::Boolean,
            presence: SlotPresence::RequiredOnce,
            sites: vec![ParameterSite::UsagePolicyParameter],
            skill_input: None,
        },
    )));
    f.schema = OwnedDefinitionSchemaPackage::new(schema, Default::default()).unwrap();
    let identity = f.schema.identity().clone();
    f.change_rules(|rules| {
        rules.definitions = identity.clone();
        rules.operations_version = key(OWNED_RULE_OPERATIONS_V21);
        rules.effect_applications = Some(empty());
        rules.ordered_contributions = Some(empty());
        rules.owners.push(DefinitionRules {
            owner: usage_owner(),
            programs: DeclaredSet::complete(vec![RuleProgram {
                id: key("requested-fact"),
                context: RuleEntityKind::Skill,
                reads: vec![RuleRead {
                    id: key("requested"),
                    value_type: ComputedValueType::Boolean,
                    source: RuleReadSource::Parameter {
                        slot: usage_parameter(),
                    },
                }],
                nodes: vec![RuleNode {
                    id: key("value"),
                    expression: RuleExpression::Read {
                        input: key("requested"),
                    },
                }],
                effects: vec![RuleEffect {
                    id: key("participation"),
                    when: None,
                    effect: RuleEffectKind::Derive {
                        entity: RuleEntity::Current,
                        stat: id("requested-participation"),
                        value: key("value"),
                    },
                }],
            }]),
        });
    });
    f.change_routing(|routing| routing.definitions = identity.clone());
    f.input.definitions = identity;
    f.input.schema_version = OWNED_EVALUATION_STAGES_V4;
    f.input.effect_applications = Some(empty());
    f.input.programs.members.push(StagedRuleProgram {
        owner: usage_owner(),
        program: key("requested-fact"),
        stage: key("prepare"),
    });
    f.input.readiness = Some(ReadinessInput {
        skills: vec![SkillReadiness {
            skill: id("skill"),
            parameters: empty(),
            participation: Some(id("requested-participation")),
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
            ReadinessProgram {
                owner: usage_owner(),
                program: key("requested-fact"),
                phase: ReadinessPhase::Preparation,
                role: ReadinessProgramRole::PreparationFacts,
                outputs: vec![participation_channel()],
            },
        ]),
    });
    f
}
fn classification(f: &mut Fixture) -> &mut ReadinessProgram {
    f.input
        .readiness
        .as_mut()
        .unwrap()
        .programs
        .members
        .iter_mut()
        .find(|r| r.owner == usage_owner())
        .unwrap()
}
fn rebind_schema(f: &mut Fixture, change: impl FnOnce(&mut SchemaPackageInput)) {
    let mut schema = f.schema.input().clone();
    change(&mut schema);
    f.schema = OwnedDefinitionSchemaPackage::new(schema, Default::default()).unwrap();
    let identity = f.schema.identity().clone();
    f.change_rules(|r| r.definitions = identity.clone());
    f.change_routing(|r| r.definitions = identity.clone());
    f.input.definitions = identity;
}
#[test]
fn current_participation_is_indexed_and_identity_bound_without_another_input_store() {
    let mut f = fixture();
    let package = f.package().unwrap();
    assert_eq!(
        package.skill_participation(&id("skill")),
        Some(&id("requested-participation"))
    );
    assert_eq!(package.skill_participation(&id("unknown")), None);
    let bytes = encode_evaluation_stages(&package, Default::default()).unwrap();
    let reread =
        decode_evaluation_stages(&bytes, &f.schema, &f.rules, &f.routing, Default::default())
            .unwrap();
    assert_eq!(reread.identity(), package.identity());
    f.input.readiness.as_mut().unwrap().skills[0].participation = None;
    let absent = f.package().unwrap();
    assert_ne!(absent.identity(), package.identity());
    assert!(absent.skill_participation(&id("skill")).is_none());
    // Both alias spellings refer to the exact current Skill context.
    f = fixture();
    f.change_rules(|r| {
        let p = &mut r
            .owners
            .iter_mut()
            .find(|o| o.owner == usage_owner())
            .unwrap()
            .programs
            .members[0];
        let RuleEffectKind::Derive { entity, .. } = &mut p.effects[0].effect else {
            unreachable!()
        };
        *entity = RuleEntity::Skill;
    });
    assert!(f.package().is_ok());
}
#[test]
fn requirement_needs_the_explicit_current_stage_and_operation_admission() {
    for version in [OWNED_EVALUATION_STAGES_V2, OWNED_EVALUATION_STAGES_V3] {
        let mut f = fixture();
        f.input.schema_version = version;
        assert!(f.package().is_err());
    }
    for operations in [
        OWNED_RULE_OPERATIONS_V18,
        OWNED_RULE_OPERATIONS_V19,
        OWNED_RULE_OPERATIONS_V20,
    ] {
        let mut f = fixture();
        f.change_rules(|r| {
            r.operations_version = key(operations);
            r.ordered_contributions = None;
        });
        assert!(matches!(
            f.package(),
            Err(StageStorageError::Invalid(
                "skill participation requires stages V4 and operations V21"
            ))
        ));
        f.input.readiness.as_mut().unwrap().skills[0].participation = None;
        assert!(f.package().is_ok());
    }
}
#[test]
fn participation_rejects_unknown_foreign_wrong_type_and_multi_scope_channels() {
    for kind in 0..6 {
        let mut f = fixture();
        match kind {
            0 => f.input.readiness.as_mut().unwrap().skills[0].participation = Some(id("missing")),
            1 => {
                f.input.readiness.as_mut().unwrap().skills[0].participation = Some(StatDefId::new(
                    GameVersionNamespace::new("foreign", "v1").unwrap(),
                    key("requested-participation"),
                ))
            }
            2..=4 => rebind_schema(&mut f, |schema| {
                for row in &mut schema.definitions {
                    if let DefinitionDescriptor::Stat(DefinitionEntry {
                        id: stat,
                        schema: SchemaState::Known(s),
                    }) = row
                        && *stat == id("requested-participation")
                    {
                        match kind {
                            2 => s.value = ComputedValueType::Integer,
                            3 => s.targets = vec![RuleEntityKind::Actor],
                            _ => s.targets.push(RuleEntityKind::Actor),
                        }
                    }
                }
            }),
            _ => {
                let row = f.input.readiness.as_ref().unwrap().skills[0].clone();
                f.input.readiness.as_mut().unwrap().skills.push(row);
            }
        }
        assert!(f.package().is_err(), "invalid participation {kind}");
    }
}
#[test]
fn participation_preserves_complete_required_inputs_and_early_owner_authority() {
    for kind in 0..4 {
        let mut f = fixture();
        match kind {
            0 => {
                f.input.readiness.as_mut().unwrap().skills[0]
                    .parameters
                    .closure = SchemaClosure::Partial {
                    gaps: vec![SchemaGap {
                        subject: SchemaSubject::Definition(
                            id::<SkillDefinition>("skill").address(),
                        ),
                        facet: SchemaFacet::InputSchema,
                        code: key("missing-input"),
                    }],
                }
            }
            1 => classification(&mut f).outputs.clear(),
            2 => f.change_rules(|r| {
                r.owners
                    .iter_mut()
                    .find(|o| o.owner == usage_owner())
                    .unwrap()
                    .programs
                    .closure = SchemaClosure::Partial {
                    gaps: vec![SchemaGap {
                        subject: usage_owner(),
                        facet: SchemaFacet::GameRules,
                        code: key("missing-programs"),
                    }],
                }
            }),
            _ => f.input.readiness.as_mut().unwrap().skills[0]
                .parameters
                .members
                .push(ParameterReadiness {
                    parameter: DeclaredSlot {
                        declaration: SlotOwnerDefId::Skill(id("skill")),
                        slot: id("unreviewed"),
                    },
                    phase: ReadinessPhase::Preparation,
                }),
        }
        assert!(f.package().is_err(), "lost authority check {kind}");
    }
}
#[test]
fn all_potential_participation_writers_require_early_fact_authority_before_guards() {
    for guarded in [false, true] {
        let mut f = fixture();
        if guarded {
            f.change_rules(|r| {
                let p = &mut r
                    .owners
                    .iter_mut()
                    .find(|o| o.owner == usage_owner())
                    .unwrap()
                    .programs
                    .members[0];
                p.nodes.push(RuleNode {
                    id: key("disabled"),
                    expression: RuleExpression::Literal {
                        value: ParameterValue::Boolean(false),
                    },
                });
                p.effects[0].when = Some(key("disabled"));
            });
        }
        let row = classification(&mut f);
        row.phase = ReadinessPhase::Execution;
        row.role = ReadinessProgramRole::Execution;
        row.outputs.clear();
        assert!(matches!(
            f.package(),
            Err(StageStorageError::Invalid(
                "participation writer requires an early Skill preparation-fact derivation"
            ))
        ));
    }
    let mut f = fixture();
    classification(&mut f).role = ReadinessProgramRole::SourceFinalInputAssembly;
    assert!(f.package().is_err());
    let mut f = fixture();
    f.change_rules(|r| {
        let p = &mut r
            .owners
            .iter_mut()
            .find(|o| o.owner == usage_owner())
            .unwrap()
            .programs
            .members[0];
        let mut duplicate = p.effects[0].clone();
        duplicate.id = key("duplicate");
        p.effects.push(duplicate);
    });
    assert!(f.package().is_err());
}
#[test]
fn missing_participation_producer_remains_a_concrete_dependency_not_a_default() {
    let mut f = fixture();
    f.change_rules(|r| r.owners.retain(|o| o.owner != usage_owner()));
    f.input
        .programs
        .members
        .retain(|p| p.owner != usage_owner());
    f.input
        .readiness
        .as_mut()
        .unwrap()
        .programs
        .members
        .retain(|p| p.owner != usage_owner());
    let package = f.package().unwrap();
    assert_eq!(
        package.skill_participation(&id("skill")),
        Some(&id("requested-participation"))
    );
    // No producer or true default is manufactured by Data; Engine binds the missing value.
    assert!(
        !f.rules
            .input()
            .owners
            .iter()
            .any(|o| o.owner == usage_owner())
    );
}
#[test]
fn participation_uses_existing_entry_work_and_wire_limits() {
    let f = fixture();
    let package = f.package().unwrap();
    let use_ = package.resources();
    for limits in [
        StageStorageLimits {
            max_entries: use_.entries - 1,
            ..Default::default()
        },
        StageStorageLimits {
            max_work: use_.work - 1,
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
