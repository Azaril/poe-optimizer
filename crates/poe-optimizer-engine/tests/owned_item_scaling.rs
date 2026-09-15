//! Direct-owned numerical recipes: no imported labels, Lua, or source-field reads.
//! The injected scale preserves ((100 + quality) / 100) before truncation.
#[allow(dead_code)]
#[path = "support/owned_plan_fixture.rs"]
mod support;
use poe_optimizer_core::{
    build_identity::BuildRevision, owned_build::*, owned_definitions::*, owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_engine::owned_plan::*;
use support::*;

fn known<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn qty(value: f64) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(value, def("percentage-points")).unwrap())
}
fn quantity_type() -> ComputedValueType {
    ComputedValueType::Quantity {
        unit: def("percentage-points"),
    }
}
fn range() -> QuantityRange {
    QuantityRange {
        minimum: FiniteQuantity::new(0.0, def("percentage-points")).unwrap(),
        maximum: FiniteQuantity::new(100.0, def("percentage-points")).unwrap(),
    }
}
fn item_slot(template: &str, input: &str) -> DeclaredSlot<ParameterSlotDefId> {
    parameter(
        SlotOwnerDefId::ItemTemplate(def(template)),
        &format!("{template}-{input}"),
    )
}
fn modifier_slot(name: &str) -> DeclaredSlot<ParameterSlotDefId> {
    parameter(SlotOwnerDefId::Modifier(def("modifier")), name)
}
fn typed_read(id: &str, value_type: ComputedValueType, source: RuleReadSource) -> RuleRead {
    RuleRead {
        id: key(id),
        value_type,
        source,
    }
}
fn parameter_schema(
    f: &mut Fixture,
    slot: DeclaredSlot<ParameterSlotDefId>,
    value: ValueSchema,
    presence: SlotPresence,
    site: ParameterSite,
) {
    for d in &mut f.schema.definitions {
        let declarations = match d {
            DefinitionDescriptor::ItemTemplate(row)
                if slot.declaration == SlotOwnerDefId::ItemTemplate(row.id.clone()) =>
            {
                let SchemaState::Known(s) = &mut row.schema else {
                    unreachable!()
                };
                &mut s.declarations
            }
            DefinitionDescriptor::Modifier(row)
                if slot.declaration == SlotOwnerDefId::Modifier(row.id.clone()) =>
            {
                let SchemaState::Known(s) = &mut row.schema else {
                    unreachable!()
                };
                &mut s.declarations
            }
            _ => continue,
        };
        declarations.parameters.members.push(slot.clone());
    }
    f.schema.slots.push(SlotDescriptor::Parameter(known(
        slot,
        ParameterSlotSchema {
            value,
            presence,
            sites: vec![site],
        },
    )));
}
fn item_program(template: &str) -> RuleProgram {
    let mut reads = Vec::new();
    let mut nodes = Vec::new();
    let mut effects = Vec::new();
    for name in ["present", "amount", "cold", "fire"] {
        reads.push(typed_read(
            name,
            if name == "amount" {
                quantity_type()
            } else {
                ComputedValueType::Boolean
            },
            RuleReadSource::Parameter {
                slot: item_slot(template, name),
            },
        ));
        nodes.push(read_node(name, name));
        effects.push(derive(
            name,
            RuleEntity::Current,
            &format!("catalyst-{name}"),
            name,
        ));
    }
    reads.extend([
        typed_read(
            "ordinary-present",
            ComputedValueType::Boolean,
            RuleReadSource::HasItemQuality {
                quality: def("ordinary"),
            },
        ),
        typed_read(
            "ordinary-amount",
            quantity_type(),
            RuleReadSource::ItemQualityAmount {
                quality: def("ordinary"),
            },
        ),
    ]);
    nodes.extend([
        read_node("ordinary-present", "ordinary-present"),
        read_node("ordinary-amount", "ordinary-amount"),
        node("zero", RuleExpression::Literal { value: qty(0.0) }),
        node(
            "ordinary",
            RuleExpression::Select {
                condition: key("ordinary-present"),
                when_true: key("ordinary-amount"),
                when_false: key("zero"),
            },
        ),
    ]);
    effects.push(derive(
        "ordinary",
        RuleEntity::Current,
        "ordinary-amount",
        "ordinary",
    ));
    RuleProgram {
        id: key("item-properties"),
        context: RuleEntityKind::EquipmentUse,
        reads,
        nodes,
        effects,
    }
}
fn modifier_program() -> RuleProgram {
    let mut reads = vec![read(
        "roll",
        RuleReadSource::Parameter {
            slot: modifier_slot("roll"),
        },
    )];
    for name in ["cold", "fire", "unscalable"] {
        reads.push(typed_read(
            name,
            ComputedValueType::Boolean,
            RuleReadSource::Parameter {
                slot: modifier_slot(name),
            },
        ));
    }
    for name in ["present", "amount", "cold", "fire"] {
        reads.push(typed_read(
            &format!("item-{name}"),
            if name == "amount" {
                quantity_type()
            } else {
                ComputedValueType::Boolean
            },
            RuleReadSource::Stat {
                entity: RuleEntity::Current,
                stat: def(&format!("catalyst-{name}")),
            },
        ));
    }
    let mut nodes: Vec<_> = reads
        .iter()
        .map(|r| read_node(r.id.as_str(), r.id.as_str()))
        .collect();
    nodes.extend([
        node("one-point", RuleExpression::Literal { value: qty(1.0) }),
        node("hundred", RuleExpression::Literal { value: qty(100.0) }),
        node(
            "base",
            RuleExpression::ScaleInteger {
                value: key("one-point"),
                count: key("roll"),
            },
        ),
        node(
            "match-cold",
            RuleExpression::All {
                values: vec![key("cold"), key("item-cold")],
            },
        ),
        node(
            "match-fire",
            RuleExpression::All {
                values: vec![key("fire"), key("item-fire")],
            },
        ),
        node(
            "matching",
            RuleExpression::Any {
                values: vec![key("match-cold"), key("match-fire")],
            },
        ),
        node(
            "enabled",
            RuleExpression::All {
                values: vec![key("item-present"), key("matching")],
            },
        ),
        // This operation order is intentional: 1 + quality/100 loses a unit at
        // the subsequent truncation for base=1000 and quality=0.7.
        node(
            "numerator",
            RuleExpression::Add {
                left: key("hundred"),
                right: key("item-amount"),
            },
        ),
        node(
            "factor",
            RuleExpression::Ratio {
                numerator: key("numerator"),
                denominator: key("hundred"),
                unit: def("factor"),
            },
        ),
        node(
            "scaled",
            RuleExpression::Scale {
                value: key("base"),
                factor: key("factor"),
            },
        ),
        node(
            "truncated",
            RuleExpression::Round {
                value: key("scaled"),
                quantum: FiniteQuantity::new(1.0, def("percentage-points")).unwrap(),
                mode: RuleRounding::Truncate,
            },
        ),
        node(
            "matching-value",
            RuleExpression::Select {
                condition: key("enabled"),
                when_true: key("truncated"),
                when_false: key("base"),
            },
        ),
        node(
            "result",
            RuleExpression::Select {
                condition: key("unscalable"),
                when_true: key("base"),
                when_false: key("matching-value"),
            },
        ),
    ]);
    RuleProgram {
        id: key("catalyst-scaled"),
        context: RuleEntityKind::EquipmentUse,
        reads,
        nodes,
        effects: vec![effect(
            "result",
            RuleEffectKind::Contribute {
                entity: RuleEntity::Player,
                stat: def("scaled"),
                contribution: ContributionKind::Add,
                value: key("result"),
            },
        )],
    }
}
fn item_values(template: &str, amount: f64, cold: bool, fire: bool) -> Vec<ParameterAssignment> {
    [
        ("present", ParameterValue::Boolean(true)),
        ("amount", qty(amount)),
        ("cold", ParameterValue::Boolean(cold)),
        ("fire", ParameterValue::Boolean(fire)),
    ]
    .into_iter()
    .map(|(name, value)| ParameterAssignment {
        slot: item_slot(template, name),
        value,
    })
    .collect()
}
fn rolls(amount: i64, cold: bool, fire: bool, unscalable: bool) -> Vec<ParameterAssignment> {
    [
        ("roll", integer(amount)),
        ("cold", ParameterValue::Boolean(cold)),
        ("fire", ParameterValue::Boolean(fire)),
        ("unscalable", ParameterValue::Boolean(unscalable)),
    ]
    .into_iter()
    .map(|(name, value)| ParameterAssignment {
        slot: modifier_slot(name),
        value,
    })
    .collect()
}
fn fixture() -> Fixture {
    let mut f = Fixture::new();
    // Keep the shared fixture's complete synthetic input declarations, replacing
    // only its numerical programs. This does not assert real game coverage.
    for owner in &mut f.owners {
        owner.programs.members.clear();
    }
    let mut second = f
        .schema
        .definitions
        .iter()
        .find_map(|d| {
            if let DefinitionDescriptor::ItemTemplate(row) = d
                && let SchemaState::Known(s) = &row.schema
            {
                Some(s.clone())
            } else {
                None
            }
        })
        .unwrap();
    second.declarations.parameters.members.clear();
    second.equipment_slots.members = vec![def("third")];
    f.schema.definitions.extend([
        DefinitionDescriptor::ItemTemplate(known(def("second-item"), second)),
        DefinitionDescriptor::EquipmentSlot(known(
            def("third"),
            EquipmentSlotSchema {
                scope: ScopePolicy::Either,
            },
        )),
        DefinitionDescriptor::Unit(known(
            def("percentage-points"),
            UnitSchema {
                dimension: UnitDimension::PercentagePoints,
            },
        )),
        DefinitionDescriptor::Unit(known(
            def("factor"),
            UnitSchema {
                dimension: UnitDimension::DimensionlessFactor,
            },
        )),
        DefinitionDescriptor::Quality(known(def("ordinary"), QualitySchema { amount: range() })),
    ]);
    for (name, ty, target) in [
        (
            "catalyst-present",
            ComputedValueType::Boolean,
            RuleEntityKind::EquipmentUse,
        ),
        (
            "catalyst-cold",
            ComputedValueType::Boolean,
            RuleEntityKind::EquipmentUse,
        ),
        (
            "catalyst-fire",
            ComputedValueType::Boolean,
            RuleEntityKind::EquipmentUse,
        ),
        (
            "catalyst-amount",
            quantity_type(),
            RuleEntityKind::EquipmentUse,
        ),
        (
            "ordinary-amount",
            quantity_type(),
            RuleEntityKind::EquipmentUse,
        ),
        ("scaled", quantity_type(), RuleEntityKind::Actor),
    ] {
        f.schema.definitions.push(DefinitionDescriptor::Stat(known(
            def(name),
            StatSchema {
                value: ty,
                targets: vec![target],
            },
        )));
    }
    for d in &mut f.schema.definitions {
        if let DefinitionDescriptor::ItemTemplate(row) = d
            && let SchemaState::Known(s) = &mut row.schema
        {
            s.quality = QualityUseSchema {
                presence: QualityPresence::Optional,
                allowed_kinds: DeclaredSet::complete(vec![def("ordinary")]),
            };
        }
    }
    for (template, amount, cold, fire) in [
        ("item", 20.0, true, false),
        ("second-item", 50.0, false, true),
    ] {
        for name in ["present", "amount", "cold", "fire"] {
            parameter_schema(
                &mut f,
                item_slot(template, name),
                if name == "amount" {
                    ValueSchema::Quantity(range())
                } else {
                    ValueSchema::Boolean
                },
                SlotPresence::OptionalOnce,
                ParameterSite::ItemParameter,
            );
        }
        let owner = subject(def::<ItemTemplateDefinition>(template));
        let program = item_program(template);
        if template == "item" {
            f.owner_mut(&owner).programs.members.push(program);
            f.build.items[0]
                .parameters
                .extend(item_values(template, amount, cold, fire));
        } else {
            f.owners.push(DefinitionRules {
                owner,
                programs: DeclaredSet::complete(vec![program]),
            });
            f.build.items.push(ItemRecord {
                id: occurrence(8),
                template: def(template),
                parameters: item_values(template, amount, cold, fire),
                item_level: Some(20),
                quality: None,
                modifier_order: vec![occurrence(9)],
                modifiers: vec![RolledModifier {
                    id: occurrence(9),
                    definition: def("modifier"),
                    rolls: rolls(41, false, true, false),
                }],
            });
        }
    }
    for name in ["cold", "fire", "unscalable"] {
        parameter_schema(
            &mut f,
            modifier_slot(name),
            ValueSchema::Boolean,
            SlotPresence::RequiredOnce,
            ParameterSite::ModifierRoll,
        );
    }
    for s in &mut f.schema.slots {
        if let SlotDescriptor::Parameter(row) = s
            && row.id == modifier_slot("roll")
            && let SchemaState::Known(s) = &mut row.schema
        {
            s.value = ValueSchema::Integer(IntegerRange {
                minimum: BoundedInteger::new(-10000).unwrap(),
                maximum: BoundedInteger::new(10000).unwrap(),
            });
        }
    }
    f.owner_mut(&modifier_owner())
        .programs
        .members
        .push(modifier_program());
    f.build.items[0].modifiers[0].rolls = rolls(49, true, false, false);
    f.build.items[0].modifiers[1].rolls = rolls(-49, false, true, false);
    f.build.equipment.push(EquipmentUse {
        id: occurrence(10),
        item: occurrence(8),
        destination: EquipmentDestination::CharacterSlot(def("third")),
        scope: LoadoutScope::Shared,
    });
    f
}
fn evaluate(f: &Fixture) -> OwnedEffectsReport {
    let p = f.compile().unwrap();
    let r = p.evaluate(&mut p.new_scratch()).unwrap();
    assert!(r.gaps.is_empty(), "unexpected plan gaps");
    r
}
fn scaled(report: &OwnedEffectsReport, equipment_use: u64, modifier: u64) -> &EffectValue {
    let origin = RuleOrigin::Provider {
        provider: ProviderKey {
            root: ProviderRoot::ItemModifier {
                equipment_use: occurrence(equipment_use),
                modifier: occurrence(modifier),
            },
            grant_path: vec![],
        },
    };
    let rows: Vec<_> = report
        .effects
        .iter()
        .filter(|e| {
            e.key.invocation.origin == origin && e.key.invocation.program == key("catalyst-scaled")
        })
        .collect();
    assert_eq!(rows.len(), 1, "one exact modifier occurrence");
    &rows[0].value
}
fn expected(report: &OwnedEffectsReport, use_id: u64, modifier: u64, value: f64) {
    assert_eq!(
        scaled(report, use_id, modifier),
        &EffectValue::Known { value: qty(value) }
    );
}
fn set_item(f: &mut Fixture, item: usize, name: &str, value: Option<ParameterValue>) {
    let slot = item_slot(if item == 0 { "item" } else { "second-item" }, name);
    f.build.items[item].parameters.retain(|p| p.slot != slot);
    if let Some(value) = value {
        f.build.items[item]
            .parameters
            .push(ParameterAssignment { slot, value });
    }
}
fn set_roll(f: &mut Fixture, item: usize, modifier: usize, name: &str, value: ParameterValue) {
    f.build.items[item].modifiers[modifier]
        .rolls
        .iter_mut()
        .find(|p| p.slot == modifier_slot(name))
        .unwrap()
        .value = value;
}

#[test]
fn any_matching_property_scales_once_and_backing_edits_reach_each_equipment_use() {
    let mut f = fixture();
    let before = evaluate(&f);
    for use_id in [6, 7] {
        expected(&before, use_id, 4, 58.0);
        expected(&before, use_id, 5, -49.0);
    }
    expected(&before, 10, 9, 61.0);
    // The same modifier has two matching labels, but the scalar is applied once.
    set_item(&mut f, 0, "fire", Some(ParameterValue::Boolean(true)));
    set_roll(&mut f, 0, 0, "fire", ParameterValue::Boolean(true));
    let both = evaluate(&f);
    for use_id in [6, 7] {
        expected(&both, use_id, 4, 58.0);
        expected(&both, use_id, 5, -58.0);
    }
    set_item(&mut f, 0, "amount", Some(qty(40.0)));
    f.build.revision = BuildRevision::from_u64(2);
    let after = evaluate(&f);
    for use_id in [6, 7] {
        expected(&after, use_id, 4, 68.0);
        expected(&after, use_id, 5, -68.0);
    }
    expected(&after, 10, 9, 61.0);
    assert_ne!(both.identity, after.identity);
    assert_eq!(
        both.effects.iter().map(|e| &e.key).collect::<Vec<_>>(),
        after.effects.iter().map(|e| &e.key).collect::<Vec<_>>()
    );
}

#[test]
fn literal_percentage_order_and_signed_truncation_survive_fractional_quality() {
    let mut f = fixture();
    set_item(&mut f, 0, "fire", Some(ParameterValue::Boolean(true)));
    for (quality, base, expected_positive) in [
        (0.7, 1000, 1007.0),
        (20.0, 49, 58.0),
        (0.0, 49, 49.0),
        (50.0, 1, 1.0),
    ] {
        set_item(&mut f, 0, "amount", Some(qty(quality)));
        set_roll(&mut f, 0, 0, "roll", integer(base));
        set_roll(&mut f, 0, 1, "roll", integer(-base));
        let r = evaluate(&f);
        for use_id in [6, 7] {
            expected(&r, use_id, 4, expected_positive);
            expected(&r, use_id, 5, -expected_positive);
        }
    }
}

#[test]
fn unscalable_absent_and_unmatched_cases_do_not_demand_missing_quality() {
    let mut f = fixture();
    set_item(&mut f, 0, "amount", None);
    let missing = evaluate(&f);
    for use_id in [6, 7] {
        assert!(matches!(
            scaled(&missing, use_id, 4),
            EffectValue::Unresolved { .. }
        ));
        expected(&missing, use_id, 5, -49.0);
    }
    expected(&missing, 10, 9, 61.0); // Other template cannot supply the absent amount.
    set_item(&mut f, 0, "amount", Some(qty(0.0)));
    let zero = evaluate(&f);
    for use_id in [6, 7] {
        expected(&zero, use_id, 4, 49.0);
    }
    set_item(&mut f, 0, "amount", None);
    set_item(&mut f, 0, "present", Some(ParameterValue::Boolean(false)));
    let absent = evaluate(&f);
    for use_id in [6, 7] {
        expected(&absent, use_id, 4, 49.0);
    }
    set_item(&mut f, 0, "present", None);
    let unspecified = evaluate(&f);
    for use_id in [6, 7] {
        assert!(matches!(
            scaled(&unspecified, use_id, 4),
            EffectValue::Unresolved { .. }
        ));
    }
    for modifier in [0, 1] {
        set_roll(
            &mut f,
            0,
            modifier,
            "unscalable",
            ParameterValue::Boolean(true),
        );
    }
    let exempt = evaluate(&f);
    for use_id in [6, 7] {
        expected(&exempt, use_id, 4, 49.0);
        expected(&exempt, use_id, 5, -49.0);
    }
}

#[test]
fn ordinary_quality_is_an_independent_typed_input_and_does_not_supply_catalyst_quality() {
    let mut f = fixture();
    let before = evaluate(&f);
    f.build.items[0].quality = Some(QualitySelection {
        kind: def("ordinary"),
        amount: FiniteQuantity::new(90.0, def("percentage-points")).unwrap(),
    });
    let with_quality = evaluate(&f);
    for use_id in [6, 7] {
        expected(&before, use_id, 4, 58.0);
        expected(&with_quality, use_id, 4, 58.0);
        let origin = RuleOrigin::Provider {
            provider: ProviderKey {
                root: ProviderRoot::EquipmentUse(occurrence(use_id)),
                grant_path: vec![],
            },
        };
        let ordinary = with_quality
            .effects
            .iter()
            .find(|e| {
                e.key.invocation.origin == origin
                    && e.key.invocation.program == key("item-properties")
                    && e.key.effect == key("ordinary")
            })
            .unwrap();
        assert_eq!(ordinary.value, EffectValue::Known { value: qty(90.0) });
    }
    set_item(&mut f, 0, "amount", None);
    let missing = evaluate(&f);
    for use_id in [6, 7] {
        assert!(matches!(
            scaled(&missing, use_id, 4),
            EffectValue::Unresolved { .. }
        ));
    }
}
