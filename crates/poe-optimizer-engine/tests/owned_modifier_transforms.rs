//! Ordered native item-modifier transforms bind actual input occurrences.
//! No imported source labels, precomputed RuleFacts, or PoB runtime participate.
#[allow(dead_code)]
#[path = "support/owned_plan_fixture.rs"]
mod support;
use poe_optimizer_core::{
    build_identity::{BuildRevision, ModifierInstanceId},
    owned_build::*,
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::{OwnedDefinitionSchemaPackage, OwnedSchemaLimits};
use poe_optimizer_engine::{
    owned_plan::*,
    owned_rules::{CompiledRulePackage, RuleLimits},
};
use support::*;

fn known<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn qty(value: f64, unit: &str) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(value, def(unit)).unwrap())
}
fn factor_type() -> ComputedValueType {
    ComputedValueType::Quantity {
        unit: def("factor"),
    }
}
fn slot(owner: &str, name: &str) -> DeclaredSlot<ParameterSlotDefId> {
    parameter(SlotOwnerDefId::Modifier(def(owner)), name)
}
fn typed_read(id: &str, value_type: ComputedValueType, source: RuleReadSource) -> RuleRead {
    RuleRead {
        id: key(id),
        value_type,
        source,
    }
}
fn modifier_subject(name: &str) -> SchemaSubject {
    subject(def::<ModifierDefinition>(name))
}
fn add_parameter(f: &mut Fixture, owner: &str, name: &str, value: ValueSchema) {
    let id = slot(owner, name);
    let row = f
        .schema
        .definitions
        .iter_mut()
        .find_map(|row| match row {
            DefinitionDescriptor::Modifier(row) if row.id == def(owner) => Some(row),
            _ => None,
        })
        .unwrap();
    let SchemaState::Known(schema) = &mut row.schema else {
        unreachable!()
    };
    schema.declarations.parameters.members.push(id.clone());
    f.schema.slots.push(SlotDescriptor::Parameter(known(
        id,
        ParameterSlotSchema {
            value,
            presence: SlotPresence::RequiredOnce,
            sites: vec![ParameterSite::ModifierRoll],
        },
    )));
}
fn factor_schema() -> ValueSchema {
    ValueSchema::Quantity(QuantityRange {
        minimum: FiniteQuantity::new(-100.0, def("factor")).unwrap(),
        maximum: FiniteQuantity::new(100.0, def("factor")).unwrap(),
    })
}
fn projection(id: &str, order: i64, operation: ModifierTransformOperation) -> RuleEffect {
    effect(
        id,
        RuleEffectKind::ProjectModifierTransform {
            stat: def("transform-channel"),
            targets: vec![ModifierTransformTarget {
                definition: def("modifier"),
                when: Some(def("eligible")),
            }],
            order: BoundedInteger::new(order).unwrap(),
            operation,
            value: key("amount"),
        },
    )
}
fn producer_program(name: &str, operation: ModifierTransformOperation) -> RuleProgram {
    RuleProgram {
        id: key("transform"),
        context: RuleEntityKind::EquipmentUse,
        reads: vec![typed_read(
            "amount",
            factor_type(),
            RuleReadSource::Parameter {
                slot: slot(name, &format!("{name}-amount")),
            },
        )],
        nodes: vec![read_node("amount", "amount")],
        effects: vec![projection("project", 0, operation)],
    }
}
fn rolled(id: u64, definition: &str, amount: f64) -> RolledModifier {
    RolledModifier {
        id: occurrence(id),
        definition: def(definition),
        rolls: vec![ParameterAssignment {
            slot: slot(definition, &format!("{definition}-amount")),
            value: qty(amount, "factor"),
        }],
    }
}
fn fixture() -> Fixture {
    let mut f = Fixture::new();
    for owner in [class_owner(), item_owner(), modifier_owner()] {
        f.owner_mut(&owner).programs.members.clear();
    }
    for (name, dimension) in [
        ("factor", UnitDimension::DimensionlessFactor),
        ("damage", UnitDimension::Damage),
    ] {
        f.schema.definitions.push(DefinitionDescriptor::Unit(known(
            def(name),
            UnitSchema { dimension },
        )));
    }
    for (name, value) in [
        ("transform-channel", factor_type()),
        ("initial", factor_type()),
        ("effective-factor", factor_type()),
        ("eligible", ComputedValueType::Boolean),
        (
            "effective-magnitude",
            ComputedValueType::Quantity {
                unit: def("damage"),
            },
        ),
    ] {
        f.schema.definitions.push(DefinitionDescriptor::Stat(known(
            def(name),
            StatSchema {
                value,
                targets: vec![RuleEntityKind::Modifier],
            },
        )));
    }
    let mut declarations = match f
        .schema
        .definitions
        .iter()
        .find_map(|row| match row {
            DefinitionDescriptor::Modifier(row) => Some(row),
            _ => None,
        })
        .unwrap()
        .schema
        .clone()
    {
        SchemaState::Known(value) => value.declarations,
        _ => unreachable!(),
    };
    declarations.parameters.members.clear();
    for name in ["add", "multiply", "other-target"] {
        f.schema
            .definitions
            .push(DefinitionDescriptor::Modifier(known(
                def(name),
                ModifierSchema {
                    declarations: declarations.clone(),
                },
            )));
        f.owners.push(DefinitionRules {
            owner: modifier_subject(name),
            programs: DeclaredSet::complete(vec![]),
        });
    }
    for row in &mut f.schema.definitions {
        if let DefinitionDescriptor::ItemTemplate(row) = row
            && let SchemaState::Known(schema) = &mut row.schema
        {
            schema
                .modifiers
                .members
                .extend([def("add"), def("multiply"), def("other-target")]);
        }
    }
    add_parameter(&mut f, "modifier", "initial-roll", factor_schema());
    add_parameter(&mut f, "modifier", "eligible-roll", ValueSchema::Boolean);
    for (name, operation) in [
        ("add", ModifierTransformOperation::Add),
        ("multiply", ModifierTransformOperation::Multiply),
    ] {
        add_parameter(&mut f, name, &format!("{name}-amount"), factor_schema());
        f.owner_mut(&modifier_subject(name))
            .programs
            .members
            .push(producer_program(name, operation));
    }
    let inputs = [
        ("initial", "initial-roll", factor_type()),
        ("eligible", "eligible-roll", ComputedValueType::Boolean),
    ];
    for (name, parameter_name, value_type) in inputs {
        f.owner_mut(&modifier_owner())
            .programs
            .members
            .push(RuleProgram {
                id: key(name),
                context: RuleEntityKind::EquipmentUse,
                reads: vec![typed_read(
                    "input",
                    value_type,
                    RuleReadSource::Parameter {
                        slot: slot("modifier", parameter_name),
                    },
                )],
                nodes: vec![read_node("input", "input")],
                effects: vec![derive("value", RuleEntity::Modifier, name, "input")],
            });
    }
    f.owner_mut(&modifier_owner())
        .programs
        .members
        .push(RuleProgram {
            id: key("effective"),
            context: RuleEntityKind::EquipmentUse,
            reads: vec![
                typed_read(
                    "factor",
                    factor_type(),
                    RuleReadSource::ModifierTransforms {
                        stat: def("transform-channel"),
                        initial: def("initial"),
                    },
                ),
                read(
                    "nominal",
                    RuleReadSource::Parameter {
                        slot: slot("modifier", "roll"),
                    },
                ),
            ],
            nodes: vec![
                read_node("factor", "factor"),
                read_node("nominal", "nominal"),
                node(
                    "quantum",
                    RuleExpression::Literal {
                        value: qty(1.0, "damage"),
                    },
                ),
                node(
                    "nominal-quantity",
                    RuleExpression::ScaleInteger {
                        value: key("quantum"),
                        count: key("nominal"),
                    },
                ),
                node(
                    "magnitude",
                    RuleExpression::Scale {
                        value: key("nominal-quantity"),
                        factor: key("factor"),
                    },
                ),
            ],
            effects: vec![
                derive("factor", RuleEntity::Modifier, "effective-factor", "factor"),
                derive(
                    "magnitude",
                    RuleEntity::Modifier,
                    "effective-magnitude",
                    "magnitude",
                ),
            ],
        });
    let first = &mut f.build.items[0].modifiers[0];
    first.rolls[0].value = integer(25);
    first.rolls.extend([
        ParameterAssignment {
            slot: slot("modifier", "initial-roll"),
            value: qty(1.0, "factor"),
        },
        ParameterAssignment {
            slot: slot("modifier", "eligible-roll"),
            value: ParameterValue::Boolean(true),
        },
    ]);
    let second = &mut f.build.items[0].modifiers[1];
    second.rolls[0].value = integer(35);
    second.rolls.extend([
        ParameterAssignment {
            slot: slot("modifier", "initial-roll"),
            value: qty(1.5, "factor"),
        },
        ParameterAssignment {
            slot: slot("modifier", "eligible-roll"),
            value: ParameterValue::Boolean(true),
        },
    ]);
    f.build.items[0]
        .modifiers
        .extend([rolled(8, "add", 0.2), rolled(9, "multiply", 2.0)]);
    f.build.items[0].modifier_order.extend([
        occurrence::<ModifierInstanceId>(8),
        occurrence::<ModifierInstanceId>(9),
    ]);
    f
}
fn program<'a>(f: &'a mut Fixture, owner: &str, id: &str) -> &'a mut RuleProgram {
    f.owner_mut(&modifier_subject(owner))
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == key(id))
        .unwrap()
}
fn value(report: &OwnedEffectsReport, equipment: u64, modifier: u64, stat: &str) -> EffectValue {
    let wanted = PlanValueKey::Stat {
        entity: ConcreteEntity::Modifier(ProviderKey {
            root: ProviderRoot::ItemModifier {
                equipment_use: occurrence(equipment),
                modifier: occurrence(modifier),
            },
            grant_path: vec![],
        }),
        stat: def(stat),
    };
    report
        .values
        .iter()
        .find(|row| row.key == wanted)
        .unwrap_or_else(|| panic!("missing {wanted:?}: {report:?}"))
        .value
        .clone()
}
fn known_quantity(
    report: &OwnedEffectsReport,
    equipment: u64,
    modifier: u64,
    stat: &str,
    expected: f64,
    unit: &str,
) {
    let EffectValue::Known {
        value: ParameterValue::Quantity(actual),
    } = value(report, equipment, modifier, stat)
    else {
        panic!("expected known {stat}: {report:?}");
    };
    assert_eq!(actual.unit(), &def::<UnitDefinition>(unit));
    assert!(
        (actual.value() - expected).abs() < 1e-12,
        "expected {expected}, got {}",
        actual.value()
    );
}
fn report(f: &Fixture) -> OwnedEffectsReport {
    let p = f.compile().unwrap();
    p.evaluate(&mut p.new_scratch()).unwrap()
}
fn assert_factors(report: &OwnedEffectsReport, first: f64, second: f64) {
    for equipment in [6, 7] {
        known_quantity(report, equipment, 4, "effective-factor", first, "factor");
        known_quantity(report, equipment, 5, "effective-factor", second, "factor");
        known_quantity(
            report,
            equipment,
            4,
            "effective-magnitude",
            25.0 * first,
            "damage",
        );
        known_quantity(
            report,
            equipment,
            5,
            "effective-magnitude",
            35.0 * second,
            "damage",
        );
    }
}
fn remove_producer(f: &mut Fixture, id: u64) {
    f.build.items[0]
        .modifiers
        .retain(|m| m.id != occurrence(id));
    f.build.items[0]
        .modifier_order
        .retain(|m| *m != occurrence(id));
}
fn assert_rules_rejected(f: &Fixture) {
    let schema =
        OwnedDefinitionSchemaPackage::new(f.schema.clone(), OwnedSchemaLimits::default()).unwrap();
    let input = RulePackageInput {
        schema_version: OWNED_RULE_PACKAGE_VERSION,
        namespace: ns(),
        release: key("rules"),
        semantics_version: key("test-v1"),
        operations_version: key(OWNED_RULE_OPERATIONS_VERSION),
        definitions: schema.identity().clone(),
        owners: f.owners.clone(),
        tables: f.tables.clone(),
        receivers: f.receivers.clone(),
    };
    assert!(
        CompiledRulePackage::compile(&input, &schema, RuleLimits::default()).is_err(),
        "duplicate transform positions accepted by rule compiler"
    );
}

#[test]
fn actual_modifier_order_preserves_interleaved_addition_and_multiplication() {
    let mut f = fixture();
    let forward = report(&f);
    assert!(forward.gaps.is_empty(), "{:?}", forward.gaps);
    assert_factors(&forward, 2.4, 3.4);
    f.build.items[0].modifier_order =
        vec![occurrence(4), occurrence(5), occurrence(9), occurrence(8)];
    assert_factors(&report(&f), 2.2, 3.2);
}

#[test]
fn definition_storage_order_never_substitutes_for_the_item_order() {
    let mut f = fixture();
    f.build.items[0].modifiers.reverse();
    f.owners.reverse();
    for owner in &mut f.owners {
        owner.programs.members.reverse();
    }
    assert_factors(&report(&f), 2.4, 3.4);
}

#[test]
fn multiple_operations_from_one_occurrence_follow_explicit_positions() {
    let mut f = fixture();
    remove_producer(&mut f, 9);
    let p = program(&mut f, "add", "transform");
    p.nodes.push(node(
        "double",
        RuleExpression::Literal {
            value: qty(2.0, "factor"),
        },
    ));
    let mut second = projection(
        "z-first-in-storage",
        1,
        ModifierTransformOperation::Multiply,
    );
    let RuleEffectKind::ProjectModifierTransform { value, .. } = &mut second.effect else {
        unreachable!()
    };
    *value = key("double");
    p.effects.insert(0, second);
    assert_factors(&report(&f), 2.4, 3.4);
    let p = program(&mut f, "add", "transform");
    for effect in &mut p.effects {
        let RuleEffectKind::ProjectModifierTransform { order, .. } = &mut effect.effect else {
            unreachable!()
        };
        *order = BoundedInteger::new(1 - order.get()).unwrap();
    }
    assert_factors(&report(&f), 2.2, 3.2);
}

#[test]
fn known_empty_sequence_preserves_explicit_initial_factors() {
    let mut f = fixture();
    remove_producer(&mut f, 8);
    remove_producer(&mut f, 9);
    let result = report(&f);
    assert!(result.gaps.is_empty());
    assert_factors(&result, 1.0, 1.5);
}

#[test]
fn predicates_read_each_target_occurrence_instead_of_the_producer_or_family() {
    let mut f = fixture();
    f.build.items[0].modifiers[1]
        .rolls
        .iter_mut()
        .find(|r| r.slot == slot("modifier", "eligible-roll"))
        .unwrap()
        .value = ParameterValue::Boolean(false);
    assert_factors(&report(&f), 2.4, 1.5);
}

#[test]
fn nonmatching_and_inactive_projections_do_not_change_initial_values() {
    let mut f = fixture();
    for owner in ["add", "multiply"] {
        let RuleEffectKind::ProjectModifierTransform { targets, .. } =
            &mut program(&mut f, owner, "transform").effects[0].effect
        else {
            unreachable!()
        };
        targets[0].definition = def("other-target");
        targets[0].when = None;
    }
    assert_factors(&report(&f), 1.0, 1.5);
    let mut f = fixture();
    for owner in ["add", "multiply"] {
        let p = program(&mut f, owner, "transform");
        p.nodes.push(node(
            "off",
            RuleExpression::Literal {
                value: ParameterValue::Boolean(false),
            },
        ));
        p.effects[0].when = Some(key("off"));
    }
    assert_factors(&report(&f), 1.0, 1.5);
}

#[test]
fn item_identity_prevents_transforms_from_leaking_to_another_equipped_item() {
    let mut f = fixture();
    let mut second = f.build.items[0].clone();
    second.id = occurrence(40);
    for (row, new_id) in second.modifiers.iter_mut().zip([41, 42, 43, 44]) {
        row.id = occurrence(new_id);
    }
    second.modifier_order = [41, 42, 43, 44].into_iter().map(occurrence).collect();
    second.modifiers[2].rolls[0].value = qty(0.4, "factor");
    second.modifiers[3].rolls[0].value = qty(3.0, "factor");
    f.build.items.push(second);
    f.build.equipment[1].item = occurrence(40);
    let result = report(&f);
    known_quantity(&result, 6, 4, "effective-factor", 2.4, "factor");
    known_quantity(&result, 6, 5, "effective-factor", 3.4, "factor");
    known_quantity(&result, 7, 41, "effective-factor", 4.2, "factor");
    known_quantity(&result, 7, 42, "effective-factor", 5.7, "factor");
}

#[test]
fn missing_initial_or_target_predicate_is_unresolved_and_never_an_identity() {
    for missing in ["initial", "eligible"] {
        let mut f = fixture();
        f.owner_mut(&modifier_owner())
            .programs
            .members
            .retain(|p| p.id != key(missing));
        let result = report(&f);
        for equipment in [6, 7] {
            for modifier in [4, 5] {
                assert!(
                    matches!(
                        value(&result, equipment, modifier, "effective-factor"),
                        EffectValue::Unresolved { .. }
                    ),
                    "missing {missing}: {result:?}"
                );
            }
        }
    }
}

#[test]
fn unknown_global_contributors_keep_both_populated_and_empty_channels_unresolved() {
    for empty in [false, true] {
        let mut f = fixture();
        if empty {
            remove_producer(&mut f, 8);
            remove_producer(&mut f, 9);
        }
        f.owner_mut(&class_owner()).programs.closure = SchemaClosure::Partial {
            gaps: vec![SchemaGap {
                subject: class_owner(),
                facet: SchemaFacet::GameRules,
                code: key("unconverted-class-effects"),
            }],
        };
        let result = report(&f);
        assert!(!result.gaps.is_empty());
        for equipment in [6, 7] {
            for modifier in [4, 5] {
                assert!(matches!(
                    value(&result, equipment, modifier, "effective-factor"),
                    EffectValue::Unresolved { .. }
                ));
            }
        }
    }
}

#[test]
fn duplicate_explicit_positions_and_dependency_cycles_are_rejected() {
    let mut duplicate = fixture();
    let mut p = program(&mut duplicate, "add", "transform").clone();
    p.id = key("same-position-different-program");
    duplicate
        .owner_mut(&modifier_subject("add"))
        .programs
        .members
        .push(p);
    assert_rules_rejected(&duplicate);
    let mut cycle = fixture();
    program(&mut cycle, "modifier", "initial").reads[0].source = RuleReadSource::Stat {
        entity: RuleEntity::Modifier,
        stat: def("effective-factor"),
    };
    assert!(
        cycle.compile().is_err(),
        "cyclic transform channel accepted"
    );
}

#[test]
fn candidate_edits_and_parallel_worker_scratch_cannot_retain_old_sequences() {
    let mut f = fixture();
    let first = f.compile().unwrap();
    f.build.items[0].modifier_order =
        vec![occurrence(4), occurrence(5), occurrence(9), occurrence(8)];
    f.build.revision = BuildRevision::from_u64(2);
    let second = f.compile().unwrap();
    assert_ne!(first.identity(), second.identity());
    let mut scratch = first.new_scratch();
    for _ in 0..3 {
        assert_factors(&first.evaluate(&mut scratch).unwrap(), 2.4, 3.4);
        assert_factors(&second.evaluate(&mut scratch).unwrap(), 2.2, 3.2);
        assert_factors(&first.evaluate(&mut scratch).unwrap(), 2.4, 3.4);
    }
    std::thread::scope(|scope| {
        for _ in 0..4 {
            let first = &first;
            let second = &second;
            scope.spawn(move || {
                let mut scratch = first.new_scratch();
                for _ in 0..8 {
                    assert_factors(&second.evaluate(&mut scratch).unwrap(), 2.2, 3.2);
                    assert_factors(&first.evaluate(&mut scratch).unwrap(), 2.4, 3.4);
                }
            });
        }
    });
}

#[test]
fn transform_dependency_expansion_respects_plan_limits() {
    let mut empty = fixture();
    remove_producer(&mut empty, 8);
    remove_producer(&mut empty, 9);
    let base_effects = report(&empty).effects.len();
    assert!(
        empty
            .compile_with(PlanLimits {
                max_effects: base_effects,
                ..PlanLimits::default()
            })
            .is_ok()
    );
    let populated = fixture();
    assert!(populated.compile().is_ok());
    assert!(matches!(
        populated.compile_with(PlanLimits {
            max_effects: base_effects,
            ..PlanLimits::default()
        }),
        Err(PlanError::Limit("effects"))
    ));
}
