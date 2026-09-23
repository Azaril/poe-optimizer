//! Storage and semantic compiler contracts; occurrence folding is tested separately.
#[allow(dead_code)]
#[path = "support/owned_rule_fixture.rs"]
mod support;
use poe_optimizer_core::{
    owned_build::ParameterValue, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::{owned_rules::*, owned_schema::*};
use poe_optimizer_engine::owned_rules::{
    CompiledRulePackage, EffectDisposition, RuleFact, RuleLimits,
};

fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn id<K: DefinitionDomain>(namespace: &GameVersionNamespace, s: &str) -> DefId<K> {
    DefId::parse(namespace.clone(), s).unwrap()
}
fn integer(value: i64) -> BoundedInteger {
    BoundedInteger::new(value).unwrap()
}
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
struct Fixture {
    schema: OwnedDefinitionSchemaPackage,
    rules: RulePackageInput,
}
impl Fixture {
    fn new() -> Self {
        let f = support::fixture();
        let mut schema = f.schema.input().clone();
        let ns = schema.namespace.clone();
        let factor = ComputedValueType::Quantity {
            unit: id(&ns, "unit.factor"),
        };
        for (name, value, targets) in [
            ("channel", factor.clone(), vec![RuleEntityKind::Modifier]),
            (
                "channel-other",
                factor.clone(),
                vec![RuleEntityKind::Modifier],
            ),
            ("initial", factor.clone(), vec![RuleEntityKind::Modifier]),
            ("final", factor.clone(), vec![RuleEntityKind::Modifier]),
            (
                "predicate",
                ComputedValueType::Boolean,
                vec![RuleEntityKind::Modifier],
            ),
            ("actor-only", factor.clone(), vec![RuleEntityKind::Actor]),
            (
                "damage-channel",
                ComputedValueType::Quantity {
                    unit: id(&ns, "unit.damage"),
                },
                vec![RuleEntityKind::Modifier],
            ),
            (
                "different-factor",
                ComputedValueType::Quantity {
                    unit: id(&ns, "unit.other-factor"),
                },
                vec![RuleEntityKind::Modifier],
            ),
        ] {
            schema
                .definitions
                .push(DefinitionDescriptor::Stat(DefinitionEntry {
                    id: id(&ns, name),
                    schema: SchemaState::Known(StatSchema { value, targets }),
                }));
        }
        schema
            .definitions
            .push(DefinitionDescriptor::Unit(DefinitionEntry {
                id: id(&ns, "unit.other-factor"),
                schema: SchemaState::Known(UnitSchema {
                    dimension: UnitDimension::DimensionlessFactor,
                }),
            }));
        for name in ["recipient", "other-recipient"] {
            schema
                .definitions
                .push(DefinitionDescriptor::Modifier(DefinitionEntry {
                    id: id(&ns, name),
                    schema: SchemaState::Known(ModifierSchema {
                        declarations: empty_slots(),
                    }),
                }));
        }
        let schema = OwnedDefinitionSchemaPackage::new(schema, Default::default()).unwrap();
        let producer = RuleProgram {
            id: key("source"),
            context: RuleEntityKind::EquipmentUse,
            reads: vec![],
            nodes: vec![RuleNode {
                id: key("amount"),
                expression: RuleExpression::Literal {
                    value: ParameterValue::Quantity(
                        FiniteQuantity::new(0.2, id(&ns, "unit.factor")).unwrap(),
                    ),
                },
            }],
            effects: vec![RuleEffect {
                id: key("project"),
                when: None,
                effect: RuleEffectKind::ProjectModifierTransform {
                    stat: id(&ns, "channel"),
                    targets: vec![ModifierTransformTarget {
                        definition: id(&ns, "recipient"),
                        when: Some(id(&ns, "predicate")),
                    }],
                    order: integer(0),
                    operation: ModifierTransformOperation::Add,
                    value: key("amount"),
                },
            }],
        };
        let recipient = RuleProgram {
            id: key("recipient"),
            context: RuleEntityKind::EquipmentUse,
            reads: vec![RuleRead {
                id: key("fold"),
                value_type: factor,
                source: RuleReadSource::ModifierTransforms {
                    stat: id(&ns, "channel"),
                    initial: id(&ns, "initial"),
                },
            }],
            nodes: vec![RuleNode {
                id: key("fold"),
                expression: RuleExpression::Read { input: key("fold") },
            }],
            effects: vec![RuleEffect {
                id: key("result"),
                when: None,
                effect: RuleEffectKind::Derive {
                    entity: RuleEntity::Modifier,
                    stat: id(&ns, "final"),
                    value: key("fold"),
                },
            }],
        };
        let mut rules = f.rules;
        rules.definitions = schema.identity().clone();
        rules.receivers = DeclaredSet::complete(vec![]);
        rules.tables.clear();
        rules.owners = vec![
            DefinitionRules {
                owner: SchemaSubject::Definition(
                    id::<ModifierDefinition>(&ns, "modifier.conditioned").address(),
                ),
                programs: DeclaredSet::complete(vec![producer]),
            },
            DefinitionRules {
                owner: SchemaSubject::Definition(
                    id::<ModifierDefinition>(&ns, "recipient").address(),
                ),
                programs: DeclaredSet::complete(vec![recipient]),
            },
        ];
        Self { schema, rules }
    }
    fn stat(&self, name: &str) -> StatDefId {
        id(&self.rules.namespace, name)
    }
    fn producer(&mut self) -> &mut RuleProgram {
        &mut self.rules.owners[0].programs.members[0]
    }
    fn recipient(&mut self) -> &mut RuleProgram {
        &mut self.rules.owners[1].programs.members[0]
    }
    fn compiled(
        &self,
    ) -> Result<CompiledRulePackage, poe_optimizer_engine::owned_rules::RuleError> {
        CompiledRulePackage::compile(&self.rules, &self.schema, Default::default())
    }
    fn stored(&self) -> Result<OwnedRulePackage, RuleStorageError> {
        OwnedRulePackage::new(self.rules.clone(), &self.schema, Default::default())
    }
}
#[test]
fn v10_roundtrip_and_component_evaluation_preserve_typed_projection_payload() {
    let f = Fixture::new();
    let stored = f.stored().unwrap();
    assert_eq!(stored.resources().transform_targets, 1);
    let bytes = encode_rule_package(&stored, Default::default()).unwrap();
    assert_eq!(
        decode_rule_package(&bytes, &f.schema, Default::default())
            .unwrap()
            .input(),
        &f.rules
    );
    let compiled = f.compiled().unwrap();
    let mut scratch = compiled.new_scratch();
    let producer = compiled
        .evaluate(
            &f.rules.owners[0].owner,
            &key("source"),
            &[],
            &f.schema,
            &mut scratch,
        )
        .unwrap();
    assert_eq!(
        producer.effects[0].effect,
        f.rules.owners[0].programs.members[0].effects[0].effect
    );
    assert!(
        matches!(&producer.effects[0].disposition, EffectDisposition::Applied { value: ParameterValue::Quantity(v) } if v.value() == 0.2)
    );
    // A component fact exercises the read's exact unit contract, not graph fold authority.
    let facts = [RuleFact {
        read: key("fold"),
        value: ParameterValue::Quantity(
            FiniteQuantity::new(2.4, id(&f.rules.namespace, "unit.factor")).unwrap(),
        ),
    }];
    let result = compiled
        .evaluate(
            &f.rules.owners[1].owner,
            &key("recipient"),
            &facts,
            &f.schema,
            &mut scratch,
        )
        .unwrap();
    assert_eq!(
        result.effects[0].disposition,
        EffectDisposition::Applied {
            value: facts[0].value.clone()
        }
    );
    let absent = compiled
        .evaluate(
            &f.rules.owners[1].owner,
            &key("recipient"),
            &[],
            &f.schema,
            &mut scratch,
        )
        .unwrap();
    assert!(matches!(
        absent.effects[0].disposition,
        EffectDisposition::Unresolved { .. }
    ));
}
#[test]
fn old_operation_versions_reject_each_new_variant_without_changing_old_serialization() {
    for version in [
        OWNED_RULE_OPERATIONS_V6,
        OWNED_RULE_OPERATIONS_V7,
        OWNED_RULE_OPERATIONS_V8,
        OWNED_RULE_OPERATIONS_V9,
    ] {
        for projection in [false, true] {
            let mut f = Fixture::new();
            f.rules.operations_version = key(version);
            if projection {
                f.rules.owners.remove(1);
            } else {
                f.rules.owners.remove(0);
            }
            assert!(f.stored().is_err(), "{version}/{projection}");
            assert!(f.compiled().is_err(), "{version}/{projection}");
        }
        let mut f = Fixture::new();
        f.rules.operations_version = key(version);
        f.rules.owners.truncate(1);
        let stat = f.stat("initial");
        f.producer().effects[0].effect = RuleEffectKind::Derive {
            entity: RuleEntity::Modifier,
            stat,
            value: key("amount"),
        };
        let bytes = serde_json::to_vec(&f.rules).unwrap();
        let stored = f.stored().unwrap();
        assert_eq!(
            encode_rule_package(&stored, Default::default()).unwrap(),
            bytes
        );
        assert_eq!(
            serde_json::to_vec(f.compiled().unwrap().input()).unwrap(),
            bytes
        );
    }
}
#[test]
fn projection_and_read_reject_foreign_contexts_types_units_and_missing_definitions() {
    for case in 0..13 {
        let mut f = Fixture::new();
        let wrong = f.stat(match case {
            4 => "missing",
            5 => "actor-only",
            6 => "damage-channel",
            7 => "different-factor",
            8 => "predicate",
            _ => "channel",
        });
        match case {
            0 => f.producer().context = RuleEntityKind::Actor,
            1 => f.rules.owners[0].owner = SchemaSubject::Definition(f.stat("initial").address()),
            2 => f.recipient().context = RuleEntityKind::Actor,
            3 => f.rules.owners[1].owner = SchemaSubject::Definition(f.stat("final").address()),
            4..=6 => {
                if let RuleEffectKind::ProjectModifierTransform { stat, .. } =
                    &mut f.producer().effects[0].effect
                {
                    *stat = wrong;
                }
            }
            7 => {
                if let RuleReadSource::ModifierTransforms { initial, .. } =
                    &mut f.recipient().reads[0].source
                {
                    *initial = wrong;
                }
            }
            8 => {
                if let RuleReadSource::ModifierTransforms { stat, .. } =
                    &mut f.recipient().reads[0].source
                {
                    *stat = wrong;
                }
            }
            9 => {
                if let RuleEffectKind::ProjectModifierTransform { targets, .. } =
                    &mut f.producer().effects[0].effect
                {
                    targets[0].when = Some(wrong);
                }
            }
            10 => {
                let unknown = id(&f.rules.namespace, "missing-recipient");
                if let RuleEffectKind::ProjectModifierTransform { targets, .. } =
                    &mut f.producer().effects[0].effect
                {
                    targets[0].definition = unknown;
                }
            }
            11 => f.recipient().reads[0].value_type = ComputedValueType::Boolean,
            _ => {
                f.producer().nodes[0].expression = RuleExpression::Literal {
                    value: ParameterValue::Integer(integer(2)),
                }
            }
        }
        assert!(f.compiled().is_err(), "case {case}");
    }
}
#[test]
fn recipient_membership_and_explicit_order_are_unique_across_owner_programs() {
    for case in 0..4 {
        let mut f = Fixture::new();
        match case {
            0 => {
                if let RuleEffectKind::ProjectModifierTransform { targets, .. } =
                    &mut f.producer().effects[0].effect
                {
                    targets.clear();
                }
            }
            1 => {
                if let RuleEffectKind::ProjectModifierTransform { targets, .. } =
                    &mut f.producer().effects[0].effect
                {
                    targets.push(targets[0].clone());
                }
            }
            2 => {
                if let RuleEffectKind::ProjectModifierTransform { order, .. } =
                    &mut f.producer().effects[0].effect
                {
                    *order = integer(-1);
                }
            }
            _ => {
                let mut second = f.producer().clone();
                second.id = key("another-program");
                f.rules.owners[0].programs.members.push(second);
            }
        }
        assert!(f.stored().is_err(), "case {case}");
        assert!(f.compiled().is_err(), "case {case}");
    }
    let mut f = Fixture::new();
    let mut second = f.producer().clone();
    second.id = key("another-program");
    if let RuleEffectKind::ProjectModifierTransform {
        order, operation, ..
    } = &mut second.effects[0].effect
    {
        *order = integer(1);
        *operation = ModifierTransformOperation::Multiply;
    }
    f.rules.owners[0].programs.members.push(second);
    f.stored().unwrap();
    f.compiled().unwrap();
}
#[test]
fn target_budget_applies_to_all_effects_before_cloning_or_target_indexes() {
    let mut f = Fixture::new();
    let target = ModifierTransformTarget {
        definition: id(&f.rules.namespace, "other-recipient"),
        when: None,
    };
    if let RuleEffectKind::ProjectModifierTransform { targets, .. } =
        &mut f.producer().effects[0].effect
    {
        targets.push(target);
    }
    f.stored().unwrap();
    f.compiled().unwrap();
    let err = OwnedRulePackage::new(
        f.rules.clone(),
        &f.schema,
        RuleStorageLimits {
            max_transform_targets: 1,
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("transform targets"), "{err}");
    let err = CompiledRulePackage::compile(
        &f.rules,
        &f.schema,
        RuleLimits {
            max_transform_targets: 1,
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(err.path.contains("transform_targets"), "{err}");
    assert!(
        OwnedRulePackage::new(
            f.rules.clone(),
            &f.schema,
            RuleStorageLimits {
                max_transform_targets: 0,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        CompiledRulePackage::compile(
            &f.rules,
            &f.schema,
            RuleLimits {
                max_transform_targets: 0,
                ..Default::default()
            }
        )
        .is_err()
    );
}
#[test]
fn strict_wire_rejects_unknown_fields_and_invalid_target_guard() {
    let f = Fixture::new();
    let mut value = serde_json::to_value(&f.rules).unwrap();
    value["owners"][0]["programs"]["members"][0]["effects"][0]["effect"]["legacy_source"] =
        serde_json::json!("not-owned");
    assert!(serde_json::from_value::<RulePackageInput>(value).is_err());
    let mut target = serde_json::to_value(ModifierTransformTarget {
        definition: id(&f.rules.namespace, "recipient"),
        when: None,
    })
    .unwrap();
    target["when"] = serde_json::json!(false);
    assert!(serde_json::from_value::<ModifierTransformTarget>(target).is_err());
}
