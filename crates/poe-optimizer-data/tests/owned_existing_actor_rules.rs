use poe_optimizer_core::{
    owned_build::ParameterValue, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::{owned_rules::*, owned_schema::*};

#[path = "support/owned_player_equipment_slots.rs"]
mod equipment_slots;

fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("existing-actor", "test").unwrap()
}
fn id<K: DefinitionDomain>(s: &str) -> DefId<K> {
    DefId::new(ns(), key(s))
}
fn subject() -> SchemaSubject {
    SchemaSubject::Definition(DefinitionAddress::Actor(id("player-rules")))
}
fn declarations() -> DeclaredSlots {
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
fn schema(declarations: DeclaredSlots) -> OwnedDefinitionSchemaPackage {
    OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
            namespace: ns(),
            release: key("schema"),
            semantics_version: key("test"),
            slots: vec![],
            definitions: vec![
                DefinitionDescriptor::Actor(DefinitionEntry {
                    id: id("player-rules"),
                    schema: SchemaState::Known(ActorSchema { declarations }),
                }),
                DefinitionDescriptor::Stat(DefinitionEntry {
                    id: id("life"),
                    schema: SchemaState::Known(StatSchema {
                        value: ComputedValueType::Integer,
                        targets: vec![RuleEntityKind::Actor],
                    }),
                }),
            ],
        },
        Default::default(),
    )
    .unwrap()
}
fn input(schema: &OwnedDefinitionSchemaPackage) -> RulePackageInput {
    RulePackageInput {
        support_discovery: None,
        schema_version: OWNED_RULE_PACKAGE_VERSION,
        namespace: ns(),
        release: key("rules"),
        semantics_version: key("test"),
        operations_version: key(OWNED_RULE_OPERATIONS_V14),
        definitions: schema.identity().clone(),
        tables: vec![],
        receivers: DeclaredSet::complete(vec![]),
        effect_applications: None,
        contribution_queries: None,
        existing_actor_rules: Some(DeclaredSet::complete(vec![ExistingActorRuleApplication {
            id: key("player"),
            owner: id("player-rules"),
            targets: vec![ExistingActorRuleTarget::Player],
        }])),
        owners: vec![DefinitionRules {
            owner: subject(),
            programs: DeclaredSet::complete(vec![RuleProgram {
                id: key("life"),
                context: RuleEntityKind::Actor,
                reads: vec![],
                nodes: vec![RuleNode {
                    id: key("amount"),
                    expression: RuleExpression::Literal {
                        value: ParameterValue::Integer(BoundedInteger::new(16).unwrap()),
                    },
                }],
                effects: vec![RuleEffect {
                    id: key("base"),
                    when: None,
                    effect: RuleEffectKind::Contribute {
                        entity: RuleEntity::Current,
                        stat: id("life"),
                        contribution: ContributionKind::Add,
                        value: key("amount"),
                    },
                }],
            }]),
        }],
    }
}
fn gap() -> SchemaGap {
    SchemaGap {
        subject: subject(),
        facet: SchemaFacet::GameRules,
        code: key("remaining-player-rules"),
    }
}
fn rejected(input: RulePackageInput, schema: &OwnedDefinitionSchemaPackage) {
    assert!(OwnedRulePackage::new(input, schema, Default::default()).is_err());
}

fn shared_role_schema(roles: Vec<AssumptionTargetKind>) -> OwnedDefinitionSchemaPackage {
    let mut raw = schema(declarations()).input().clone();
    let slot = poe_optimizer_core::owned_build::DeclaredSlot {
        declaration: SlotOwnerDefId::Actor(id("supplier")),
        slot: id("child"),
    };
    let mut supplier = declarations();
    supplier.actors.members.push(slot.clone());
    raw.definitions.extend([
        DefinitionDescriptor::Actor(DefinitionEntry {
            id: id("supplier"),
            schema: SchemaState::Known(ActorSchema {
                declarations: supplier,
            }),
        }),
        DefinitionDescriptor::Actor(DefinitionEntry {
            id: id("profile"),
            schema: SchemaState::Known(ActorSchema {
                declarations: declarations(),
            }),
        }),
        DefinitionDescriptor::ExternalInput(DefinitionEntry {
            id: id("local-condition"),
            schema: SchemaState::Known(ExternalInputSchema {
                value: ValueSchema::Boolean,
                targets: roles,
            }),
        }),
    ]);
    raw.slots.push(SlotDescriptor::Actor(DefinitionEntry {
        id: slot,
        schema: SchemaState::Known(ActorSlotSchema {
            skills: DeclaredSet::complete(vec![]),
            outputs: DeclaredSet::complete(vec![]),
            provider_definition: Some(id("profile")),
        }),
    }));
    OwnedDefinitionSchemaPackage::new(raw, Default::default()).unwrap()
}

fn shared_role_input(schema: &OwnedDefinitionSchemaPackage) -> RulePackageInput {
    let mut raw = input(schema);
    raw.existing_actor_rules.as_mut().unwrap().members[0].targets = vec![
        ExistingActorRuleTarget::Player,
        ExistingActorRuleTarget::ScenarioEnemy,
        ExistingActorRuleTarget::OwnedSlot {
            slot: poe_optimizer_core::owned_build::DeclaredSlot {
                declaration: SlotOwnerDefId::Actor(id("supplier")),
                slot: id("child"),
            },
        },
    ];
    // Deliberately unread: eligibility is an inventory property, not a reachable
    // node or successful-value check.
    raw.owners[0].programs.members[0].reads.push(RuleRead {
        id: key("unused-local-condition"),
        value_type: ComputedValueType::Boolean,
        source: RuleReadSource::External {
            entity: RuleEntity::Current,
            input: id("local-condition"),
        },
    });
    raw
}

#[test]
fn one_local_inventory_applies_to_all_explicit_actor_roles_without_shared_permissions() {
    let schema = shared_role_schema(vec![
        AssumptionTargetKind::PlayerActor,
        AssumptionTargetKind::OwnedActor,
        AssumptionTargetKind::ScenarioEnemyActor,
    ]);
    let raw = shared_role_input(&schema);
    OwnedRulePackage::new(raw.clone(), &schema, Default::default()).unwrap();
    for source in [
        RuleReadSource::CharacterLevel,
        RuleReadSource::CharacterClassIs {
            class: id("unused-class"),
        },
        RuleReadSource::EnemyLevel,
    ] {
        let mut bad = raw.clone();
        bad.owners[0].programs.members[0].reads[0].source = source;
        rejected(bad, &schema);
    }
    let mut bad = raw;
    let RuleEffectKind::Contribute { entity, .. } =
        &mut bad.owners[0].programs.members[0].effects[0].effect
    else {
        unreachable!()
    };
    *entity = RuleEntity::Player;
    rejected(bad, &schema);
}

#[test]
fn unused_wrong_role_input_and_duplicate_supplied_profile_owner_are_rejected() {
    let schema = shared_role_schema(vec![AssumptionTargetKind::ScenarioEnemyActor]);
    rejected(shared_role_input(&schema), &schema);
    let mut raw = shared_role_input(&schema);
    raw.existing_actor_rules.as_mut().unwrap().members[0].targets =
        vec![ExistingActorRuleTarget::ScenarioEnemy];
    OwnedRulePackage::new(raw, &schema, Default::default()).unwrap();

    let mut wire = schema.input().clone();
    let SlotDescriptor::Actor(entry) = &mut wire.slots[0] else {
        unreachable!()
    };
    let SchemaState::Known(actor) = &mut entry.schema else {
        unreachable!()
    };
    actor.provider_definition = Some(id("player-rules"));
    let duplicate = OwnedDefinitionSchemaPackage::new(wire, Default::default()).unwrap();
    let mut raw = shared_role_input(&duplicate);
    raw.owners[0].programs.members[0].reads.clear();
    rejected(raw, &duplicate);
}

#[test]
fn current_empty_inventory_and_explicit_inventory_have_distinct_checked_identities() {
    let schema = schema(declarations());
    let rules = input(&schema);
    let explicit = OwnedRulePackage::new(rules.clone(), &schema, Default::default()).unwrap();
    let mut absent = rules.clone();
    absent.existing_actor_rules = None;
    let value = serde_json::to_value(&absent).unwrap();
    assert!(value.get("existing_actor_rules").is_none());
    assert_eq!(
        serde_json::from_value::<RulePackageInput>(value.clone()).unwrap(),
        absent
    );
    let absent = OwnedRulePackage::new(absent, &schema, Default::default()).unwrap();
    assert_ne!(explicit.identity(), absent.identity());
    let mut null = value;
    null["existing_actor_rules"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<RulePackageInput>(null).is_err());
    let mut unsupported = serde_json::to_value(&rules).unwrap();
    unsupported["existing_actor_rules"]["members"][0]["targets"][0] =
        serde_json::json!("all_actors");
    assert!(serde_json::from_value::<RulePackageInput>(unsupported).is_err());
}

#[test]
fn duplicate_targets_owners_and_missing_actor_authority_are_rejected() {
    let schema = schema(declarations());
    let valid = input(&schema);
    let mut bad = valid.clone();
    bad.existing_actor_rules.as_mut().unwrap().members[0]
        .targets
        .push(ExistingActorRuleTarget::Player);
    rejected(bad, &schema);
    let mut bad = valid.clone();
    let mut second = bad.existing_actor_rules.as_ref().unwrap().members[0].clone();
    second.id = key("duplicate-owner");
    bad.existing_actor_rules
        .as_mut()
        .unwrap()
        .members
        .push(second);
    rejected(bad, &schema);
    let mut bad = valid.clone();
    bad.existing_actor_rules.as_mut().unwrap().members[0]
        .targets
        .clear();
    rejected(bad, &schema);
    let mut bad = valid.clone();
    bad.existing_actor_rules.as_mut().unwrap().members[0].owner = id("missing");
    rejected(bad, &schema);
    let mut bad = valid.clone();
    bad.owners.clear();
    rejected(bad, &schema);
    let mut bad = valid.clone();
    bad.owners.push(bad.owners[0].clone());
    rejected(bad, &schema);
    let mut d = declarations();
    d.choices = DeclaredSet::partial(
        vec![],
        vec![SchemaGap {
            subject: subject(),
            facet: SchemaFacet::InputSchema,
            code: key("unknown-inputs"),
        }],
    );
    let partial_schema = schema_with(d);
    rejected(input(&partial_schema), &partial_schema);
}
fn schema_with(d: DeclaredSlots) -> OwnedDefinitionSchemaPackage {
    schema(d)
}

#[test]
fn provider_inputs_cross_actor_writes_and_grant_authority_are_not_inherited() {
    let schema = schema(declarations());
    let valid = input(&schema);
    for entity in [
        RuleEntity::Enemy,
        RuleEntity::Environment,
        RuleEntity::Modifier,
    ] {
        let mut bad = valid.clone();
        let RuleEffectKind::Contribute { entity: actual, .. } =
            &mut bad.owners[0].programs.members[0].effects[0].effect
        else {
            unreachable!()
        };
        *actual = entity;
        rejected(bad, &schema);
    }
    let mut bad = valid.clone();
    bad.owners[0].programs.members[0].context = RuleEntityKind::Action;
    rejected(bad, &schema);
    let mut bad = valid.clone();
    bad.owners[0].programs.members[0].reads.push(RuleRead {
        id: key("item"),
        value_type: ComputedValueType::Integer,
        source: RuleReadSource::ItemLevel,
    });
    rejected(bad, &schema);
    let mut bad = valid;
    bad.owners[0].programs.members[0].effects[0].effect = RuleEffectKind::Requirement {
        satisfied: key("amount"),
        code: key("unsupported"),
    };
    rejected(bad, &schema);
}

#[test]
fn partial_inventories_retain_gap_evidence_and_limits_apply_before_binding() {
    let schema = schema(declarations());
    let mut rules = input(&schema);
    rules.existing_actor_rules.as_mut().unwrap().closure =
        SchemaClosure::Partial { gaps: vec![gap()] };
    rules.owners[0].programs.closure = SchemaClosure::Partial { gaps: vec![gap()] };
    OwnedRulePackage::new(rules.clone(), &schema, Default::default()).unwrap();
    rules.existing_actor_rules.as_mut().unwrap().members.clear();
    OwnedRulePackage::new(rules.clone(), &schema, Default::default()).unwrap();
    rules.existing_actor_rules.as_mut().unwrap().closure = SchemaClosure::Partial { gaps: vec![] };
    rejected(rules, &schema);
    let rules = input(&schema);
    assert!(
        validate_existing_actor_rules(
            &rules,
            &schema,
            RuleStorageLimits {
                max_receiver_work: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[test]
fn actor_applications_cannot_impersonate_character_origins_in_ordered_queries() {
    let schema = schema(declarations());
    let mut rules = input(&schema);
    rules.operations_version = key(OWNED_RULE_OPERATIONS_V21);
    rules.effect_applications = Some(DeclaredSet::complete(vec![]));
    rules.contribution_queries = Some(DeclaredSet::complete(vec![ContributionQuery {
        id: key("life"),
        stat: id("life"),
        contribution: ContributionKind::Add,
        groups: vec![ContributionGroup {
            id: key("base"),
            reduction: ContributionReduction::Sum,
            ordering: ContributionOrdering::Ordered,
            empty: Some(ParameterValue::Integer(BoundedInteger::new(0).unwrap())),
            members: DeclaredSet::complete(vec![ContributionMember {
                producer: ContributionProducer::ProgramEffect(ProgramContributionProducer {
                    owner: subject(),
                    program: key("life"),
                    effect: key("base"),
                    origin: ContributionOrigin::Character,
                }),
                order: Some(ContributionOrder {
                    source_rank: 1,
                    program_rank: 0,
                    effect_rank: 0,
                    slot_ranks: vec![],
                }),
            }]),
        }],
    }]));
    rejected(rules, &schema);
}
