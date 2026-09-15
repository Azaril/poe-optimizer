//! Common computed item properties, authored through exact template parameters.
//! Synthetic domain data only: no source labels, importer or cross-owner reads.
#[allow(dead_code)]
#[path = "support/owned_plan_fixture.rs"]
mod support;
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_engine::owned_plan::*;
use support::*;

fn known<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn quantity(n: f64, unit: &str) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(n, def(unit)).unwrap())
}
fn kind(unit: &str) -> ComputedValueType {
    ComputedValueType::Quantity { unit: def(unit) }
}
fn input(template: &str, name: &str) -> DeclaredSlot<ParameterSlotDefId> {
    parameter(SlotOwnerDefId::ItemTemplate(def(template)), name)
}
fn typed_read(id: &str, value_type: ComputedValueType, source: RuleReadSource) -> RuleRead {
    RuleRead {
        id: key(id),
        value_type,
        source,
    }
}
fn fixture() -> Fixture {
    let mut f = Fixture::new();
    let first_template = f
        .schema
        .definitions
        .iter()
        .find_map(|d| {
            if let DefinitionDescriptor::ItemTemplate(row) = d
                && let SchemaState::Known(schema) = &row.schema
            {
                Some(schema.clone())
            } else {
                None
            }
        })
        .unwrap();
    let mut second_template = first_template;
    second_template.declarations.parameters.members.clear();
    second_template.equipment_slots.members.push(def("third"));
    f.schema.definitions.extend([
        DefinitionDescriptor::ItemTemplate(known(def("other-item"), second_template)),
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
    ]);
    for (name, ty, target) in [
        (
            "catalyst-present",
            ComputedValueType::Boolean,
            RuleEntityKind::EquipmentUse,
        ),
        (
            "catalyst-amount",
            kind("percentage-points"),
            RuleEntityKind::EquipmentUse,
        ),
        (
            "scaled-cold",
            kind("percentage-points"),
            RuleEntityKind::Actor,
        ),
    ] {
        f.schema.definitions.push(DefinitionDescriptor::Stat(known(
            def(name),
            StatSchema {
                value: ty,
                targets: vec![target],
            },
        )));
    }
    // Distinct declared slots deliberately have different names, as well as
    // different owners. Common property identity is the shared owned StatDef.
    for (template, present, amount) in [
        ("item", "first-catalyst-enabled", "first-catalyst-quality"),
        (
            "other-item",
            "second-treatment-present",
            "second-treatment-amount",
        ),
    ] {
        for (slot, value, presence) in [
            (
                input(template, present),
                ValueSchema::Boolean,
                SlotPresence::RequiredOnce,
            ),
            (
                input(template, amount),
                ValueSchema::Quantity(QuantityRange {
                    minimum: FiniteQuantity::new(0.0, def("percentage-points")).unwrap(),
                    maximum: FiniteQuantity::new(100.0, def("percentage-points")).unwrap(),
                }),
                SlotPresence::OptionalOnce,
            ),
        ] {
            for row in &mut f.schema.definitions {
                if let DefinitionDescriptor::ItemTemplate(row) = row
                    && row.id == def(template)
                    && let SchemaState::Known(schema) = &mut row.schema
                {
                    schema.declarations.parameters.members.push(slot.clone());
                }
            }
            f.schema.slots.push(SlotDescriptor::Parameter(known(
                slot,
                ParameterSlotSchema {
                    value,
                    presence,
                    sites: vec![ParameterSite::ItemParameter],
                },
            )));
        }
        let program = RuleProgram {
            id: key("item-catalyst-properties"),
            context: RuleEntityKind::EquipmentUse,
            reads: vec![
                typed_read(
                    "present",
                    ComputedValueType::Boolean,
                    RuleReadSource::Parameter {
                        slot: input(template, present),
                    },
                ),
                typed_read(
                    "amount",
                    kind("percentage-points"),
                    RuleReadSource::Parameter {
                        slot: input(template, amount),
                    },
                ),
            ],
            nodes: vec![
                read_node("present", "present"),
                read_node("amount", "amount"),
            ],
            effects: vec![
                derive(
                    "present",
                    RuleEntity::Current,
                    "catalyst-present",
                    "present",
                ),
                derive("amount", RuleEntity::Current, "catalyst-amount", "amount"),
            ],
        };
        let owner = subject(def::<ItemTemplateDefinition>(template));
        if template == "item" {
            f.owner_mut(&owner).programs.members.push(program);
        } else {
            f.owners.push(DefinitionRules {
                owner,
                programs: DeclaredSet::complete(vec![program]),
            });
        }
    }
    f.build.items[0].parameters.extend([
        ParameterAssignment {
            slot: input("item", "first-catalyst-enabled"),
            value: ParameterValue::Boolean(true),
        },
        ParameterAssignment {
            slot: input("item", "first-catalyst-quality"),
            value: quantity(20.0, "percentage-points"),
        },
    ]);
    f.build.items[0].modifiers[0].rolls[0].value = integer(25);
    f.build.items[0].modifiers[1].rolls[0].value = integer(35);
    f.build.items.push(ItemRecord {
        id: occurrence(8),
        template: def("other-item"),
        parameters: vec![
            ParameterAssignment {
                slot: input("other-item", "second-treatment-present"),
                value: ParameterValue::Boolean(true),
            },
            ParameterAssignment {
                slot: input("other-item", "second-treatment-amount"),
                value: quantity(50.0, "percentage-points"),
            },
        ],
        item_level: Some(20),
        quality: None,
        modifiers: vec![RolledModifier {
            id: occurrence(9),
            definition: def("modifier"),
            rolls: vec![ParameterAssignment {
                slot: parameter(SlotOwnerDefId::Modifier(def("modifier")), "roll"),
                value: integer(40),
            }],
        }],
    });
    f.build.equipment.push(EquipmentUse {
        id: occurrence(10),
        item: occurrence(8),
        destination: EquipmentDestination::CharacterSlot(def("third")),
        scope: LoadoutScope::Shared,
    });
    // One shared modifier program reads its own roll and common computed item
    // properties. Its Parameter read never names either ItemTemplate.
    f.owner_mut(&modifier_owner())
        .programs
        .members
        .push(RuleProgram {
            id: key("catalyst-scaled-cold"),
            context: RuleEntityKind::EquipmentUse,
            reads: vec![
                read(
                    "roll",
                    RuleReadSource::Parameter {
                        slot: parameter(SlotOwnerDefId::Modifier(def("modifier")), "roll"),
                    },
                ),
                typed_read(
                    "present",
                    ComputedValueType::Boolean,
                    RuleReadSource::Stat {
                        entity: RuleEntity::Current,
                        stat: def("catalyst-present"),
                    },
                ),
                typed_read(
                    "amount",
                    kind("percentage-points"),
                    RuleReadSource::Stat {
                        entity: RuleEntity::Current,
                        stat: def("catalyst-amount"),
                    },
                ),
            ],
            nodes: vec![
                read_node("roll", "roll"),
                read_node("present", "present"),
                read_node("amount", "amount"),
                node(
                    "point",
                    RuleExpression::Literal {
                        value: quantity(1.0, "percentage-points"),
                    },
                ),
                node(
                    "base",
                    RuleExpression::ScaleInteger {
                        value: key("point"),
                        count: key("roll"),
                    },
                ),
                node(
                    "one",
                    RuleExpression::Literal {
                        value: quantity(1.0, "factor"),
                    },
                ),
                node(
                    "increase",
                    RuleExpression::PercentAsFactor {
                        percent: key("amount"),
                        unit: def("factor"),
                    },
                ),
                node(
                    "factor",
                    RuleExpression::Add {
                        left: key("one"),
                        right: key("increase"),
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
                    "selected",
                    RuleExpression::Select {
                        condition: key("present"),
                        when_true: key("scaled"),
                        when_false: key("base"),
                    },
                ),
            ],
            effects: vec![effect(
                "cold",
                RuleEffectKind::Contribute {
                    entity: RuleEntity::Player,
                    stat: def("scaled-cold"),
                    contribution: ContributionKind::Add,
                    value: key("selected"),
                },
            )],
        });
    f
}
fn evaluate(f: &Fixture) -> OwnedEffectsReport {
    let plan = f.compile().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(report.gaps.is_empty(), "{report:?}");
    report
}
fn cold(report: &OwnedEffectsReport, use_id: u64, modifier: u64) -> &EffectValue {
    let origin = RuleOrigin::Provider {
        provider: ProviderKey {
            root: ProviderRoot::ItemModifier {
                equipment_use: occurrence(use_id),
                modifier: occurrence(modifier),
            },
            grant_path: vec![],
        },
    };
    let rows: Vec<_> = report
        .effects
        .iter()
        .filter(|r| {
            r.key.invocation.origin == origin
                && r.key.invocation.program == key("catalyst-scaled-cold")
        })
        .collect();
    assert_eq!(rows.len(), 1, "{report:?}");
    &rows[0].value
}
fn assert_first_item(report: &OwnedEffectsReport, first: f64, second: f64) {
    for use_id in [6, 7] {
        assert_eq!(
            cold(report, use_id, 4),
            &EffectValue::Known {
                value: quantity(first, "percentage-points")
            }
        );
        assert_eq!(
            cold(report, use_id, 5),
            &EffectValue::Known {
                value: quantity(second, "percentage-points")
            }
        );
    }
}

#[test]
fn exact_template_parameters_feed_one_shared_modifier_for_repeated_equipment_uses() {
    let mut f = fixture();
    let before = evaluate(&f);
    assert_first_item(&before, 30.0, 42.0);
    assert_eq!(
        cold(&before, 10, 9),
        &EffectValue::Known {
            value: quantity(60.0, "percentage-points")
        }
    );
    let record_ids: Vec<_> = f
        .build
        .items
        .iter()
        .map(|i| (i.id, i.modifiers.iter().map(|m| m.id).collect::<Vec<_>>()))
        .collect();
    f.build.items[0]
        .parameters
        .iter_mut()
        .find(|p| p.slot == input("item", "first-catalyst-quality"))
        .unwrap()
        .value = quantity(40.0, "percentage-points");
    f.build.revision = poe_optimizer_core::build_identity::BuildRevision::from_u64(2);
    let after = evaluate(&f);
    assert_ne!(before.identity, after.identity);
    assert_first_item(&after, 35.0, 49.0);
    assert_eq!(
        cold(&after, 10, 9),
        &EffectValue::Known {
            value: quantity(60.0, "percentage-points")
        }
    );
    assert_eq!(
        record_ids,
        f.build
            .items
            .iter()
            .map(|i| (i.id, i.modifiers.iter().map(|m| m.id).collect::<Vec<_>>()))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        before.effects.iter().map(|e| &e.key).collect::<Vec<_>>(),
        after.effects.iter().map(|e| &e.key).collect::<Vec<_>>()
    );
}

#[test]
fn missing_item_amount_is_lazy_and_never_filled_from_a_sibling_template() {
    let mut f = fixture();
    f.build.items[1]
        .parameters
        .retain(|p| p.slot != input("other-item", "second-treatment-amount"));
    let missing = evaluate(&f);
    assert_first_item(&missing, 30.0, 42.0);
    assert!(matches!(
        cold(&missing, 10, 9),
        EffectValue::Unresolved { .. }
    ));
    // Explicit zero is a present, supported amount; it is not the missing case.
    f.build.items[1].parameters.push(ParameterAssignment {
        slot: input("other-item", "second-treatment-amount"),
        value: quantity(0.0, "percentage-points"),
    });
    let zero = evaluate(&f);
    assert_first_item(&zero, 30.0, 42.0);
    assert_eq!(
        cold(&zero, 10, 9),
        &EffectValue::Known {
            value: quantity(40.0, "percentage-points")
        }
    );
    f.build.items[1].parameters.pop();
    f.build.items[1].parameters[0].value = ParameterValue::Boolean(false);
    let absent = evaluate(&f);
    assert_first_item(&absent, 30.0, 42.0);
    assert_eq!(
        cold(&absent, 10, 9),
        &EffectValue::Known {
            value: quantity(40.0, "percentage-points")
        }
    );
    f.build
        .equipment
        .iter_mut()
        .find(|u| u.id == occurrence(10))
        .unwrap()
        .scope = LoadoutScope::Selected {
        loadouts: vec![occurrence(2)],
    };
    let inactive = evaluate(&f);
    assert_first_item(&inactive, 30.0, 42.0);
    assert!(!inactive.effects.iter().any(|e| matches!(&e.key.invocation.origin,
        RuleOrigin::Provider { provider: ProviderKey { root: ProviderRoot::ItemModifier { equipment_use, .. }, .. } }
        if *equipment_use == occurrence(10))));
}
