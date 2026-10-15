//! Skill endpoints extend the existing receiving fixture and occurrence rules.
use super::*;
use poe_optimizer_core::owned_readiness::*;

fn endpoint(path: Vec<DeclaredSlot<GrantSlotDefId>>) -> SupportReceiverEndpoint {
    SupportReceiverEndpoint::Skill {
        path,
        admission: SupportAdmissionContext::AssignedSkill,
    }
}
fn skill_programs(rules: &mut RulePackageInput) {
    for owner in &mut rules.owners {
        for p in &mut owner.programs.members {
            if p.context == RuleEntityKind::Action {
                p.context = RuleEntityKind::Skill;
            }
        }
    }
}
fn install(mut f: Fixture) -> Fixture {
    f.input
        .roles
        .iter_mut()
        .find(|r| r.id == key("actions"))
        .unwrap()
        .kind = SupportReceiverKind::Skill;
    f.input.targets[0].roles.members[0].endpoints = complete(vec![
        endpoint(vec![]),
        SupportReceiverEndpoint::Skill {
            path: child_path(),
            admission: SupportAdmissionContext::ReceivingSkill {
                summoner_path: Some(vec![]),
            },
        },
    ]);
    // The Gem's container is not the child Skill reached through these grants.
    f.input.targets[1].roles.members[0].endpoints = complete(vec![endpoint(vec![
        slot(active(), "actor-crosslink"),
        skill_grant(),
    ])]);
    f
}
fn fixture() -> Fixture {
    install(Fixture::with(|_| {}, skill_programs, |_| {}))
}
fn receiver_value(schema: &mut SchemaPackageInput) {
    schema.definitions.push(DefinitionDescriptor::Stat(entry(
        id("receiver-value"),
        StatSchema {
            value: ComputedValueType::Boolean,
            targets: vec![RuleEntityKind::Skill, RuleEntityKind::Actor],
        },
    )));
}

#[test]
fn exact_skill_roots_and_generated_paths_roundtrip_without_output_expansion() {
    let f = fixture();
    let p = f.build(f.input.clone()).unwrap();
    assert!(p.declarations_complete());
    assert_eq!(p.resources().expanded_endpoints, 4);
    let bytes = encode_support_receiving(&p, Default::default()).unwrap();
    let restored = decode_support_receiving(
        &bytes,
        &f.schema,
        &f.rules,
        &f.preparation,
        &f.inputs,
        &f.stages,
        Default::default(),
    )
    .unwrap();
    assert_eq!(restored.input(), p.input());
    assert_eq!(restored.identity(), p.identity());
    let mut reverse = f.input.clone();
    reverse.targets[0].roles.members[0]
        .endpoints
        .members
        .reverse();
    assert_eq!(f.build(reverse).unwrap().identity(), p.identity());
    let limits = SupportReceivingStorageLimits {
        max_expanded_endpoints: 3,
        ..Default::default()
    };
    assert!(matches!(
        f.build_with(f.input.clone(), limits),
        Err(SupportReceivingStorageError::Limit("expanded endpoints"))
    ));
}

#[test]
fn containers_actors_and_receiving_admission_do_not_invent_skill_occurrences() {
    let f = fixture();
    let mut input = f.input.clone();
    input.targets[1].roles.members[0].endpoints = complete(vec![endpoint(vec![])]);
    f.bad(input, "semantic Skill root or exact generated skill");
    let mut input = f.input.clone();
    input.targets[0].roles.members[0].endpoints = complete(vec![endpoint(vec![actor_grant()])]);
    f.bad(input, "semantic Skill root or exact generated skill");
    let mut input = f.input.clone();
    input.targets[0].roles.members[0].endpoints = complete(vec![SupportReceiverEndpoint::Skill {
        path: vec![],
        admission: SupportAdmissionContext::ReceivingSkill {
            summoner_path: None,
        },
    }]);
    f.bad(input, "exact generated skill");
    for path in [child_path(), vec![actor_grant()]] {
        let mut input = f.input.clone();
        input.targets[0].roles.members[0].endpoints =
            complete(vec![SupportReceiverEndpoint::Skill {
                path: child_path(),
                admission: SupportAdmissionContext::ReceivingSkill {
                    summoner_path: Some(path),
                },
            }]);
        f.bad(input, "summoner");
    }
}

#[test]
fn final_grant_aliases_reject_without_collapsing_distinct_provider_ancestors() {
    let f = install(Fixture::with(
        |schema| {
            for definition in &mut schema.definitions {
                match definition {
                    DefinitionDescriptor::Actor(DefinitionEntry {
                        id: actor_id,
                        schema: SchemaState::Known(s),
                    }) if *actor_id == id("minion") => {
                        s.declarations
                            .grants
                            .members
                            .push(slot(actor(), "skill-alias"));
                    }
                    DefinitionDescriptor::Skill(DefinitionEntry {
                        id: skill,
                        schema: SchemaState::Known(s),
                    }) if *skill == id("parent") => {
                        s.declarations
                            .grants
                            .members
                            .push(slot(parent(), "actor-alias"));
                    }
                    _ => {}
                }
            }
            for (grant, target) in [
                (
                    slot(actor(), "skill-alias"),
                    GrantTarget::Skill(slot(actor(), "ability")),
                ),
                (
                    slot(parent(), "actor-alias"),
                    GrantTarget::Actor(slot(parent(), "actor")),
                ),
            ] {
                schema.slots.push(SlotDescriptor::Grant(entry(
                    grant,
                    GrantSlotSchema {
                        provider_roles: vec![ProviderRole::SkillUse],
                        target,
                    },
                )));
            }
        },
        skill_programs,
        |_| {},
    ));
    let mut input = f.input.clone();
    input.targets[0].roles.members[0]
        .endpoints
        .members
        .push(endpoint(vec![actor_grant(), slot(actor(), "skill-alias")]));
    f.bad(input, "duplicate normalized receiving endpoint");
    let mut input = f.input.clone();
    input.targets[0].roles.members[0]
        .endpoints
        .members
        .push(endpoint(vec![slot(parent(), "actor-alias"), skill_grant()]));
    assert_eq!(
        f.build(input).unwrap().resources().expanded_endpoints,
        5,
        "distinct entering provider ancestors remain distinct exact Skill addresses"
    );
}

#[test]
fn skill_membership_and_program_authority_stay_explicit() {
    let f = fixture();
    let mut input = f.input.clone();
    input.supports[0].receivers.members.remove(0);
    f.bad(input.clone(), "omits known delivery/applicability programs");
    input.supports[0].receivers.closure = partial(owner("support-a"));
    assert!(!f.build(input).unwrap().declarations_complete());
    let mut input = f.input.clone();
    input.targets[0].roles.members[0].endpoints.closure = partial(SchemaSubject::Definition(
        DefinitionAddress::Skill(id("parent")),
    ));
    assert!(!f.build(input).unwrap().declarations_complete());
    let mut input = f.input.clone();
    input.supports[0].receivers.members[0].applicability = key("actor-app");
    f.bad(input, "receiving context");
    for entity in [
        RuleEntity::Skill,
        RuleEntity::Actor,
        RuleEntity::AssignedSkill,
    ] {
        let f = install(Fixture::with(
            receiver_value,
            |rules| {
                skill_programs(rules);
                rules.owners[0].programs.members[3].effects[0].effect = RuleEffectKind::Derive {
                    entity,
                    stat: id("receiver-value"),
                    value: key("value"),
                };
            },
            |_| {},
        ));
        f.bad(f.input.clone(), "receiving scope");
    }
    let f = install(Fixture::with(
        receiver_value,
        |rules| {
            skill_programs(rules);
            rules.owners[0].programs.members[3].effects[0].effect = RuleEffectKind::Derive {
                entity: RuleEntity::Current,
                stat: id("receiver-value"),
                value: key("value"),
            };
        },
        |_| {},
    ));
    assert!(f.build(f.input.clone()).is_ok());
}

fn prepared_fixture(entity: RuleEntity) -> Fixture {
    let mut f = install(Fixture::with(
        |schema| {
            schema.definitions.push(DefinitionDescriptor::Stat(entry(
                id("skill-property"),
                StatSchema {
                    value: ComputedValueType::Integer,
                    targets: vec![RuleEntityKind::Skill, RuleEntityKind::Actor],
                },
            )));
        },
        |rules| {
            skill_programs(rules);
            rules.operations_version = key(OWNED_RULE_OPERATIONS_V16);
            rules.effect_applications = Some(complete(vec![]));
            let row = &mut rules.owners[0];
            row.programs
                .members
                .push(program("preparation-app", RuleEntityKind::Skill, true));
            let mut property = program("preparation-property", RuleEntityKind::Skill, false);
            property.nodes[0].expression = RuleExpression::Literal {
                value: ParameterValue::Integer(BoundedInteger::new(2).unwrap()),
            };
            property.effects[0].effect = RuleEffectKind::Contribute {
                entity,
                stat: id("skill-property"),
                contribution: ContributionKind::Add,
                value: key("value"),
            };
            row.programs.members.push(property);
        },
        |stages| {
            stages.schema_version = OWNED_EVALUATION_STAGES_V2;
            stages.effect_applications = Some(complete(vec![]));
            for (name, before) in [
                ("preparation-app", "prepare"),
                ("preparation-property", "preparation-app"),
            ] {
                stages.stages.push(EvaluationStage {
                    id: key(name),
                    predecessors: vec![key(before)],
                });
            }
            stages
                .stages
                .iter_mut()
                .find(|s| s.id == key("applicable"))
                .unwrap()
                .predecessors
                .push(key("preparation-property"));
            let programs = stages
                .programs
                .members
                .iter_mut()
                .map(|p| {
                    let (role, outputs) = match p.program.as_str() {
                        "preparation-app" => {
                            p.stage = key("preparation-app");
                            (
                                ReadinessProgramRole::SupportPreparationApplicability,
                                vec![],
                            )
                        }
                        "preparation-property" => {
                            p.stage = key("preparation-property");
                            (
                                ReadinessProgramRole::SupportedPreparationProperty,
                                vec![StageChannel::Contributions {
                                    scope: if entity == RuleEntity::Actor {
                                        RuleEntityKind::Actor
                                    } else {
                                        RuleEntityKind::Skill
                                    },
                                    stat: id("skill-property"),
                                    contribution: ContributionKind::Add,
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
    ));
    f.input.schema_version = OWNED_SUPPORT_RECEIVING_V2;
    f.input.supports[0].receivers.members[0].preparation = Some(SupportPreparationPrograms {
        applicability: key("preparation-app"),
        properties: vec![key("preparation-property")],
    });
    f
}

#[test]
fn skill_preparation_accepts_current_properties_without_alias_or_execution_authority() {
    let f = prepared_fixture(RuleEntity::Current);
    assert!(f.build(f.input.clone()).unwrap().declarations_complete());
    for entity in [
        RuleEntity::Skill,
        RuleEntity::Actor,
        RuleEntity::AssignedSkill,
    ] {
        let error = std::panic::catch_unwind(|| prepared_fixture(entity))
            .err()
            .expect("foreign preparation output must reject");
        let text = error
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| error.downcast_ref::<&str>().copied())
            .unwrap();
        assert!(
            text.contains("effect is not authorized by preparation output role"),
            "{text}"
        );
    }
    let mut input = f.input.clone();
    input.supports[0].receivers.members[0]
        .preparation
        .as_mut()
        .unwrap()
        .properties = vec![key("action-delivery")];
    f.bad(
        input,
        "preparation property overlaps another receiving role",
    );
}
