//! This relation adds only checked computed reads to the existing Player owner.
use super::*;

fn fixture() -> (OwnedDefinitionSchemaPackage, RulePackageInput) {
    let mut schema_input = schema(declarations()).input().clone();
    schema_input.definitions.extend([
        DefinitionDescriptor::EquipmentSlot(DefinitionEntry {
            id: id("hand"),
            schema: SchemaState::Known(EquipmentSlotSchema {
                scope: ScopePolicy::Either,
            }),
        }),
        DefinitionDescriptor::Stat(DefinitionEntry {
            id: id("local"),
            schema: SchemaState::Known(StatSchema {
                value: ComputedValueType::Quantity { unit: id("count") },
                targets: vec![RuleEntityKind::EquipmentUse],
            }),
        }),
        DefinitionDescriptor::Capability(DefinitionEntry {
            id: id("profile"),
            schema: SchemaState::Known(CapabilitySchema {
                targets: vec![RuleEntityKind::EquipmentUse],
            }),
        }),
        DefinitionDescriptor::Unit(DefinitionEntry {
            id: id("count"),
            schema: SchemaState::Known(UnitSchema {
                dimension: UnitDimension::Count,
            }),
        }),
    ]);
    let schema = OwnedDefinitionSchemaPackage::new(schema_input, Default::default()).unwrap();
    let mut rules = input(&schema);
    rules.operations_version = key(OWNED_RULE_OPERATIONS_V21);
    rules.effect_applications = Some(DeclaredSet::complete(vec![]));
    rules.ordered_contributions = Some(DeclaredSet::complete(vec![]));
    rules.owners[0].programs.members[0].reads = vec![
        read(
            PlayerEquipmentSlotRead::Occupied,
            ComputedValueType::Boolean,
            "occupied",
        ),
        read(
            PlayerEquipmentSlotRead::Stat { stat: id("local") },
            ComputedValueType::Quantity { unit: id("count") },
            "amount",
        ),
        read(
            PlayerEquipmentSlotRead::Capability {
                capability: id("profile"),
            },
            ComputedValueType::Boolean,
            "profile",
        ),
    ];
    (schema, rules)
}
fn read(read: PlayerEquipmentSlotRead, value_type: ComputedValueType, name: &str) -> RuleRead {
    RuleRead {
        id: key(name),
        value_type,
        source: RuleReadSource::PlayerEquipmentSlot {
            slot: id("hand"),
            read,
        },
    }
}
fn reject_both(rules: RulePackageInput, schema: &OwnedDefinitionSchemaPackage) {
    assert!(validate_existing_actor_rules(&rules, schema, Default::default()).is_err());
    rejected(rules, schema);
}

#[test]
fn explicit_slot_reads_round_trip_and_bind_their_exact_type_without_producers() {
    let (schema, rules) = fixture();
    let package = OwnedRulePackage::new(rules, &schema, Default::default()).unwrap();
    let bytes = encode_rule_package(&package, Default::default()).unwrap();
    let restored = decode_rule_package(&bytes, &schema, Default::default()).unwrap();
    assert_eq!(restored.identity(), package.identity());
    assert_eq!(restored.input(), package.input());
    // Storage certifies declarations, never the availability of equipment or outputs.
    assert_eq!(package.input().owners.len(), 1);
    let mut other = package.input().clone();
    other.owners[0].programs.members[0].reads.remove(0);
    assert_ne!(
        OwnedRulePackage::new(other, &schema, Default::default())
            .unwrap()
            .identity(),
        package.identity()
    );
}

#[test]
fn slot_reads_require_current_operations_and_exact_explicit_player_owner() {
    let (schema, rules) = fixture();
    for version in [OWNED_RULE_OPERATIONS_V14, OWNED_RULE_OPERATIONS_V20] {
        let mut bad = rules.clone();
        bad.operations_version = key(version);
        reject_both(bad, &schema);
    }
    for registry in [None, Some(DeclaredSet::complete(vec![]))] {
        let mut bad = rules.clone();
        bad.existing_actor_rules = registry;
        reject_both(bad, &schema);
    }
    let mut bad = rules.clone();
    bad.owners[0].owner = SchemaSubject::Definition(DefinitionAddress::Stat(id("life")));
    reject_both(bad, &schema);
    for context in [
        RuleEntityKind::EquipmentUse,
        RuleEntityKind::Skill,
        RuleEntityKind::SupportOrigin,
    ] {
        let mut bad = rules.clone();
        bad.owners[0].programs.members[0].context = context;
        reject_both(bad, &schema);
    }
    // Merely listing an Actor owner does not confer a Player target.
    let mut bad = rules;
    bad.existing_actor_rules.as_mut().unwrap().members[0]
        .targets
        .clear();
    reject_both(bad, &schema);
}

#[test]
fn slot_reads_reject_missing_foreign_mistyped_and_wrong_scope_definitions() {
    let (schema, rules) = fixture();
    for slot in [
        id("missing"),
        EquipmentSlotDefId::new(
            GameVersionNamespace::new("foreign", "test").unwrap(),
            key("hand"),
        ),
    ] {
        let mut bad = rules.clone();
        let RuleReadSource::PlayerEquipmentSlot { slot: actual, .. } =
            &mut bad.owners[0].programs.members[0].reads[0].source
        else {
            unreachable!()
        };
        *actual = slot;
        reject_both(bad, &schema);
    }
    for source in [
        PlayerEquipmentSlotRead::Stat {
            stat: id("missing"),
        },
        PlayerEquipmentSlotRead::Stat { stat: id("life") },
        PlayerEquipmentSlotRead::Capability {
            capability: id("missing"),
        },
    ] {
        let mut bad = rules.clone();
        bad.owners[0].programs.members[0].reads[0] =
            read(source, ComputedValueType::Integer, "occupied");
        reject_both(bad, &schema);
    }
    for i in 0..3 {
        let mut bad = rules.clone();
        bad.owners[0].programs.members[0].reads[i].value_type = ComputedValueType::Integer;
        reject_both(bad, &schema);
    }
    let mut bad = rules.clone();
    bad.owners[0].programs.members[0].reads[1].value_type = ComputedValueType::Quantity {
        unit: id("different-unit"),
    };
    reject_both(bad, &schema);
    let mut schema_input = schema.input().clone();
    for row in &mut schema_input.definitions {
        if let DefinitionDescriptor::Capability(entry) = row {
            entry.schema = SchemaState::Known(CapabilitySchema {
                targets: vec![RuleEntityKind::Actor],
            });
        }
    }
    let wrong_scope = OwnedDefinitionSchemaPackage::new(schema_input, Default::default()).unwrap();
    let mut bad = rules;
    bad.definitions = wrong_scope.identity().clone();
    reject_both(bad, &wrong_scope);
}

#[test]
fn slot_authority_does_not_extend_to_effect_applications_or_raw_provider_inputs() {
    let (schema, mut rules) = fixture();
    let program = rules.owners[0].programs.members[0].clone();
    rules.owners[0].programs.members[0].reads.clear();
    rules
        .effect_applications
        .as_mut()
        .unwrap()
        .members
        .push(EffectApplicationRule {
            id: key("not-player-application"),
            source: EffectApplicationSource::Skill {
                skill: id("unused"),
            },
            targets: vec![EffectApplicationTarget::Player],
            activation: key("amount"),
            program,
            stacking: vec![],
        });
    assert!(matches!(
        validate_existing_actor_rules(&rules, &schema, Default::default()),
        Err(RuleStorageError::Structure(
            "effect applications cannot read Player equipment slots"
        ))
    ));
    let (schema, mut rules) = fixture();
    rules.owners[0].programs.members[0].reads[0].source = RuleReadSource::ItemLevel;
    reject_both(rules, &schema);
}

#[test]
fn unused_partial_slot_readers_still_validate_and_existing_limits_apply() {
    let (schema, mut rules) = fixture();
    rules.owners[0].programs.closure = SchemaClosure::Partial { gaps: vec![gap()] };
    rules.existing_actor_rules.as_mut().unwrap().closure =
        SchemaClosure::Partial { gaps: vec![gap()] };
    let package = OwnedRulePackage::new(rules.clone(), &schema, Default::default()).unwrap();
    assert!(!package.input().owners[0].programs.is_complete());
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
    assert!(
        OwnedRulePackage::new(
            rules.clone(),
            &schema,
            RuleStorageLimits {
                max_reads: 2,
                ..Default::default()
            }
        )
        .is_err()
    );
    let bytes = encode_rule_package(&package, Default::default()).unwrap();
    assert!(
        decode_rule_package(
            &bytes,
            &schema,
            RuleStorageLimits {
                max_wire_bytes: bytes.len() - 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    rules.owners[0].programs.members[0].reads[0].value_type = ComputedValueType::Integer;
    reject_both(rules, &schema);
}
