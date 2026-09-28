//! Storage-version compatibility, independent of evaluator numerical semantics.
#[allow(dead_code)]
#[path = "support/owned_rule_receiver_fixture.rs"]
mod fixture;

use fixture::{id, input, key, schema_input};
use poe_optimizer_core::{
    owned_build::ParameterValue, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::{owned_rules::*, owned_schema::*};

fn empty_slots() -> DeclaredSlots {
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

fn schema() -> OwnedDefinitionSchemaPackage {
    let mut raw = schema_input();
    for definition in &mut raw.definitions {
        if let DefinitionDescriptor::Stat(DefinitionEntry {
            schema: SchemaState::Known(stat),
            ..
        }) = definition
        {
            stat.targets.push(RuleEntityKind::EquipmentUse);
        }
    }
    raw.definitions
        .push(DefinitionDescriptor::ItemTemplate(DefinitionEntry {
            id: id("equipment"),
            schema: SchemaState::Known(ItemTemplateSchema {
                item_level: IntegerRange {
                    minimum: BoundedInteger::new(1).unwrap(),
                    maximum: BoundedInteger::new(100).unwrap(),
                },
                equipment_slots: DeclaredSet::complete(vec![]),
                socket_destinations: DeclaredSet::complete(vec![]),
                modifiers: DeclaredSet::complete(vec![]),
                quality: QualityUseSchema {
                    presence: QualityPresence::Optional,
                    allowed_kinds: DeclaredSet::complete(vec![]),
                },
                declarations: empty_slots(),
            }),
        }));
    for name in ["producer", "recipient"] {
        raw.definitions
            .push(DefinitionDescriptor::Modifier(DefinitionEntry {
                id: id(name),
                schema: SchemaState::Known(ModifierSchema {
                    declarations: empty_slots(),
                }),
            }));
    }
    for name in ["transform-channel", "initial", "transformed"] {
        raw.definitions
            .push(DefinitionDescriptor::Stat(DefinitionEntry {
                id: id(name),
                schema: SchemaState::Known(StatSchema {
                    value: ComputedValueType::Quantity { unit: id("factor") },
                    targets: vec![RuleEntityKind::Modifier],
                }),
            }));
    }
    OwnedDefinitionSchemaPackage::new(raw, OwnedSchemaLimits::default()).unwrap()
}

fn check_roundtrip(
    mut raw: RulePackageInput,
    schema: &OwnedDefinitionSchemaPackage,
    version: &str,
) {
    raw.operations_version = key(version);
    let limits = RuleStorageLimits::default();
    let package = OwnedRulePackage::new(raw, schema, limits).unwrap();
    let bytes = encode_rule_package(&package, limits).unwrap();
    let restored = decode_rule_package(&bytes, schema, limits).unwrap();
    assert_eq!(restored.input().operations_version.as_str(), version);
    assert_eq!(restored.input(), package.input());
    assert_eq!(restored.identity(), package.identity());
    assert_eq!(encode_rule_package(&restored, limits).unwrap(), bytes);
}

fn check_downgrade(
    mut raw: RulePackageInput,
    schema: &OwnedDefinitionSchemaPackage,
    version: &str,
    expected: &str,
) {
    raw.operations_version = key(version);
    let limits = RuleStorageLimits::default();
    let wire = serde_json::to_vec(&raw).unwrap();
    for result in [
        OwnedRulePackage::new(raw, schema, limits),
        decode_rule_package(&wire, schema, limits),
    ] {
        assert!(matches!(result, Err(RuleStorageError::Structure(message)) if message == expected));
    }
}

#[test]
fn equipment_receivers_keep_v9_v10_and_latest_without_silent_version_upgrade() {
    let schema = schema();
    let mut raw = input(&schema);
    raw.owners[0].programs.members.truncate(1);
    raw.owners[0].programs.members[0].context = RuleEntityKind::EquipmentUse;
    raw.receivers.members.truncate(1);
    raw.receivers.members[0].targets = vec![StatReceiverTarget::EquipmentTemplate {
        template: id("equipment"),
    }];
    for version in [
        "owned-domain-operations-v9",
        "owned-domain-operations-v10",
        OWNED_RULE_OPERATIONS_VERSION,
    ] {
        check_roundtrip(raw.clone(), &schema, version);
    }
    check_downgrade(
        raw,
        &schema,
        "owned-domain-operations-v8",
        "equipment receivers require owned-domain-operations-v9",
    );
}

#[test]
fn modifier_transforms_keep_v10_and_latest_with_independent_read_and_effect_guards() {
    let schema = schema();
    let producer = RuleProgram {
        id: key("project"),
        context: RuleEntityKind::EquipmentUse,
        reads: vec![],
        nodes: vec![RuleNode {
            id: key("amount"),
            expression: RuleExpression::Literal {
                value: ParameterValue::Quantity(FiniteQuantity::new(0.2, id("factor")).unwrap()),
            },
        }],
        effects: vec![RuleEffect {
            id: key("project"),
            when: None,
            effect: RuleEffectKind::ProjectModifierTransform {
                stat: id("transform-channel"),
                targets: vec![ModifierTransformTarget {
                    definition: id("recipient"),
                    when: None,
                }],
                order: BoundedInteger::new(0).unwrap(),
                operation: ModifierTransformOperation::Add,
                value: key("amount"),
            },
        }],
    };
    let recipient = RuleProgram {
        id: key("fold"),
        context: RuleEntityKind::EquipmentUse,
        reads: vec![RuleRead {
            id: key("fold"),
            value_type: ComputedValueType::Quantity { unit: id("factor") },
            source: RuleReadSource::ModifierTransforms {
                stat: id("transform-channel"),
                initial: id("initial"),
            },
        }],
        nodes: vec![RuleNode {
            id: key("fold"),
            expression: RuleExpression::Read { input: key("fold") },
        }],
        effects: vec![RuleEffect {
            id: key("derive"),
            when: None,
            effect: RuleEffectKind::Derive {
                entity: RuleEntity::Modifier,
                stat: id("transformed"),
                value: key("fold"),
            },
        }],
    };
    // Separate packages ensure a projection rejection cannot mask a missing
    // read-version guard, or vice versa. Storage does not prove runtime coverage.
    for (owner, program) in [("producer", producer), ("recipient", recipient)] {
        let mut raw = input(&schema);
        raw.receivers = DeclaredSet::complete(vec![]);
        raw.owners = vec![DefinitionRules {
            owner: SchemaSubject::Definition(DefinitionAddress::Modifier(id(owner))),
            programs: DeclaredSet::complete(vec![program]),
        }];
        for version in ["owned-domain-operations-v10", OWNED_RULE_OPERATIONS_VERSION] {
            check_roundtrip(raw.clone(), &schema, version);
        }
        check_downgrade(
            raw,
            &schema,
            "owned-domain-operations-v9",
            "modifier transforms require owned-domain-operations-v10",
        );
    }
}
