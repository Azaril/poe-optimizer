//! Component proof: actual modifier occurrences -> integer Actor contributions.
//! No source adapter, supplied RuleFact values, final attribute stages, or build parity.
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
use poe_optimizer_engine::owned_plan::*;
use std::{collections::BTreeSet, sync::Arc};
use support::*;

const CHANNELS: [&str; 3] = ["attribute-a", "attribute-b", "attribute-c"];

fn quantity(n: f64) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(n, def("count")).unwrap())
}
fn quantity_type() -> ComputedValueType {
    ComputedValueType::Quantity { unit: def("count") }
}
fn known<I, T>(id: I, value: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(value),
    }
}
fn stat_owner(name: &str) -> SchemaSubject {
    subject(def::<StatDefinition>(name))
}
fn actor_key(name: &str) -> PlanValueKey {
    PlanValueKey::Stat {
        entity: ConcreteEntity::Actor(ActorKey::Player),
        stat: def(name),
    }
}
fn modifier_key(equipment: u64, modifier: u64) -> PlanValueKey {
    PlanValueKey::Stat {
        entity: ConcreteEntity::Modifier(ProviderKey {
            root: ProviderRoot::ItemModifier {
                equipment_use: occurrence(equipment),
                modifier: occurrence(modifier),
            },
            grant_path: vec![],
        }),
        stat: def("modifier-magnitude"),
    }
}
fn value<'a>(report: &'a OwnedEffectsReport, key: &PlanValueKey) -> &'a EffectValue {
    let rows: Vec<_> = report.values.iter().filter(|row| &row.key == key).collect();
    assert_eq!(rows.len(), 1, "expected exact value {key:?}: {report:?}");
    &rows[0].value
}
fn known_totals(report: &OwnedEffectsReport, n: i64) {
    for name in CHANNELS {
        assert_eq!(
            value(report, &actor_key(name)),
            &EffectValue::Known { value: integer(n) }
        );
    }
}
fn evaluate(f: &Fixture) -> OwnedEffectsReport {
    let plan = f.compile().unwrap();
    plan.evaluate(&mut plan.new_scratch()).unwrap()
}

fn fixture() -> Fixture {
    let mut f = Fixture::new();
    for owner in [class_owner(), item_owner(), modifier_owner()] {
        f.owner_mut(&owner).programs.members.clear();
    }
    let roll = parameter(SlotOwnerDefId::Modifier(def("modifier")), "roll");
    let slot = f
        .schema
        .slots
        .iter_mut()
        .find_map(|row| match row {
            SlotDescriptor::Parameter(row) if row.id == roll => Some(row),
            _ => None,
        })
        .unwrap();
    let SchemaState::Known(schema) = &mut slot.schema else {
        unreachable!()
    };
    schema.value = ValueSchema::Quantity(QuantityRange {
        minimum: FiniteQuantity::new(-100.0, def("count")).unwrap(),
        maximum: FiniteQuantity::new(100.0, def("count")).unwrap(),
    });
    for (modifier, amount) in f.build.items[0].modifiers.iter_mut().zip([6.0, -2.0]) {
        modifier.rolls[0].value = quantity(amount);
    }
    f.schema.definitions.push(DefinitionDescriptor::Stat(known(
        def("modifier-magnitude"),
        StatSchema {
            value: quantity_type(),
            targets: vec![RuleEntityKind::Modifier],
        },
    )));
    for name in CHANNELS {
        f.schema.definitions.push(DefinitionDescriptor::Stat(known(
            def(name),
            StatSchema {
                value: ComputedValueType::Integer,
                targets: vec![RuleEntityKind::Actor],
            },
        )));
        let program = RuleProgram {
            id: key("sum-attributes"),
            context: RuleEntityKind::Actor,
            reads: vec![contributions("incoming", RuleEntity::Current, name)],
            nodes: vec![read_node("sum", "incoming")],
            effects: vec![derive("total", RuleEntity::Current, name, "sum")],
        };
        f.receivers.members.push(ActorStatReceiver {
            id: key(name),
            stat: def(name),
            program: program.id.clone(),
            targets: vec![ActorReceiverTarget::Player],
        });
        f.owners.push(DefinitionRules {
            owner: stat_owner(name),
            programs: DeclaredSet::complete(vec![program]),
        });
    }
    f.owner_mut(&modifier_owner()).programs.members = vec![
        RuleProgram {
            id: key("upstream-magnitude"),
            context: RuleEntityKind::EquipmentUse,
            reads: vec![RuleRead {
                id: key("roll"),
                value_type: quantity_type(),
                source: RuleReadSource::Parameter { slot: roll },
            }],
            nodes: vec![read_node("magnitude", "roll")],
            effects: vec![derive(
                "magnitude",
                RuleEntity::Modifier,
                "modifier-magnitude",
                "magnitude",
            )],
        },
        RuleProgram {
            id: key("project-all-attributes"),
            context: RuleEntityKind::EquipmentUse,
            reads: vec![RuleRead {
                id: key("magnitude"),
                value_type: quantity_type(),
                source: RuleReadSource::Stat {
                    entity: RuleEntity::Modifier,
                    stat: def("modifier-magnitude"),
                },
            }],
            nodes: vec![
                read_node("magnitude", "magnitude"),
                node(
                    "integer",
                    RuleExpression::QuantizeInteger {
                        value: key("magnitude"),
                        quantum: FiniteQuantity::new(1.0, def("count")).unwrap(),
                        mode: RuleRounding::Floor,
                    },
                ),
            ],
            effects: CHANNELS
                .into_iter()
                .map(|name| {
                    effect(
                        name,
                        RuleEffectKind::Contribute {
                            entity: RuleEntity::Player,
                            stat: def(name),
                            contribution: ContributionKind::Add,
                            value: key("integer"),
                        },
                    )
                })
                .collect(),
        },
    ];
    f
}

#[test]
fn integral_modifier_occurrences_project_into_three_distinct_actor_sums() {
    let f = fixture();
    let report = evaluate(&f);
    assert!(report.gaps.is_empty(), "{report:?}");
    // Upstream represents already formatted integral Counts, like the shipped
    // precision-zero producers. Conversion preserves 6 + (-2), twice for two uses.
    known_totals(&report, 8);
    let mut occurrences = BTreeSet::new();
    for row in &report.effects {
        let BoundEffectTarget::Contribution { key: contribution } = &row.target else {
            continue;
        };
        assert_eq!(contribution.entity, ConcreteEntity::Actor(ActorKey::Player));
        assert_eq!(contribution.kind, ContributionKind::Add);
        assert!(CHANNELS.iter().any(|name| contribution.stat == def(name)));
        let RuleOrigin::Provider { provider } = &row.key.invocation.origin else {
            panic!("physical provider expected")
        };
        let ProviderRoot::ItemModifier {
            equipment_use,
            modifier,
        } = provider.root
        else {
            panic!("modifier origin expected")
        };
        assert!(provider.grant_path.is_empty());
        assert_eq!(
            row.key.invocation.entity,
            ConcreteEntity::EquipmentUse(equipment_use)
        );
        let (raw, rounded) = if modifier == occurrence::<ModifierInstanceId>(4) {
            (6.0, 6)
        } else {
            assert_eq!(modifier, occurrence(5));
            (-2.0, -2)
        };
        assert_eq!(
            row.value,
            EffectValue::Known {
                value: integer(rounded)
            }
        );
        assert_eq!(
            value(
                &report,
                &modifier_key(
                    equipment_use.instance_id().local(),
                    modifier.instance_id().local()
                )
            ),
            &EffectValue::Known {
                value: quantity(raw)
            }
        );
        assert!(occurrences.insert((equipment_use, modifier, contribution.stat.clone())));
    }
    assert_eq!(
        occurrences.len(),
        12,
        "two modifiers x two uses x three channels"
    );
    let player_values: Vec<_> = report
        .values
        .iter()
        .filter_map(|row| match &row.key {
            PlanValueKey::Stat {
                entity: ConcreteEntity::Actor(ActorKey::Player),
                stat,
            } => Some(stat.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        player_values.into_iter().collect::<BTreeSet<_>>(),
        CHANNELS.into_iter().map(def::<StatDefinition>).collect()
    );
    assert_eq!(
        report
            .effects
            .iter()
            .filter(|row| matches!(row.key.invocation.origin, RuleOrigin::Receiver { .. }))
            .count(),
        3,
        "an All-like provider fans out; it never creates a fourth Actor channel"
    );
}

#[test]
fn off_loadout_equipment_use_neither_projects_nor_borrows_other_use_values() {
    let mut f = fixture();
    f.build.equipment[1].scope = LoadoutScope::Selected {
        loadouts: vec![occurrence(2)],
    };
    let first = evaluate(&f);
    assert!(first.gaps.is_empty());
    known_totals(&first, 4);
    for modifier in [4, 5] {
        assert!(
            !first
                .values
                .iter()
                .any(|row| row.key == modifier_key(7, modifier))
        );
    }
    let projected = first
        .effects
        .iter()
        .filter(|row| matches!(row.target, BoundEffectTarget::Contribution { .. }))
        .count();
    assert_eq!(projected, 6);
    f.build.active_weapon_loadout = occurrence(2);
    f.build.revision = BuildRevision::from_u64(2);
    let second = evaluate(&f);
    assert!(second.gaps.is_empty());
    known_totals(&second, 8);
    assert_eq!(
        value(&second, &modifier_key(7, 4)),
        &EffectValue::Known {
            value: quantity(6.0)
        }
    );
}

#[test]
fn missing_modifier_magnitude_never_becomes_a_zero_attribute_contribution() {
    let mut f = fixture();
    f.owner_mut(&modifier_owner())
        .programs
        .members
        .retain(|p| p.id != key("upstream-magnitude"));
    let report = evaluate(&f);
    for name in CHANNELS {
        assert!(matches!(
            value(&report, &actor_key(name)),
            EffectValue::Unresolved { .. }
        ));
    }
    let projected: Vec<_> = report
        .effects
        .iter()
        .filter(|row| matches!(row.target, BoundEffectTarget::Contribution { .. }))
        .collect();
    assert_eq!(projected.len(), 12);
    assert!(
        projected
            .iter()
            .all(|row| matches!(row.value, EffectValue::Unresolved { .. }))
    );
}

#[test]
fn partial_provider_programs_or_receiver_membership_retain_incomplete_totals() {
    for receiver_membership in [false, true] {
        let mut f = fixture();
        let owner = if receiver_membership {
            stat_owner(CHANNELS[0])
        } else {
            modifier_owner()
        };
        let partial = SchemaClosure::Partial {
            gaps: vec![SchemaGap {
                subject: owner.clone(),
                facet: SchemaFacet::GameRules,
                code: key("unconverted-attribute-rules"),
            }],
        };
        if receiver_membership {
            f.receivers.closure = partial;
        } else {
            f.owner_mut(&owner).programs.closure = partial;
        }
        let report = evaluate(&f);
        let expected = if receiver_membership {
            PlanGapReason::PartialReceivers
        } else {
            PlanGapReason::PartialPrograms
        };
        assert!(report.gaps.iter().any(|gap| gap.reason == expected));
        for name in CHANNELS {
            assert!(
                matches!(
                    value(&report, &actor_key(name)),
                    EffectValue::Unresolved {
                        reason: PlanGapReason::IncompleteContributors,
                        ..
                    }
                ),
                "{report:?}"
            );
        }
        let projected: Vec<_> = report
            .effects
            .iter()
            .filter(|row| matches!(row.target, BoundEffectTarget::Contribution { .. }))
            .collect();
        assert_eq!(projected.len(), 12);
        assert!(
            projected.iter().all(|row| matches!(
                row.value,
                EffectValue::Unresolved {
                    reason: PlanGapReason::IncompleteContributors,
                    ..
                }
            )),
            "relative stat reads retain the global contributor closure gate"
        );
        for equipment in [6, 7] {
            for (modifier, raw) in [(4, 6.0), (5, -2.0)] {
                let upstream = modifier_key(equipment, modifier);
                assert!(
                    matches!(
                        value(&report, &upstream),
                        EffectValue::Unresolved {
                            reason: PlanGapReason::IncompleteContributors,
                            ..
                        }
                    ),
                    "every published plan value retains the global closure gate"
                );
                let diagnostic: Vec<_> = report.effects.iter().filter(|row| {
                    matches!(&row.target, BoundEffectTarget::Value { key } if key == &upstream)
                }).collect();
                assert_eq!(diagnostic.len(), 1);
                assert_eq!(
                    diagnostic[0].value,
                    EffectValue::Known {
                        value: quantity(raw)
                    },
                    "upstream raw occurrence effect remains diagnostic, not a complete plan value"
                );
            }
        }
    }
}

#[test]
fn complete_empty_modifier_membership_uses_the_integer_sum_identity() {
    let mut f = fixture();
    f.build.items[0].modifiers.clear();
    f.build.items[0].modifier_order.clear();
    let report = evaluate(&f);
    assert!(report.gaps.is_empty());
    known_totals(&report, 0);
    assert!(
        !report
            .effects
            .iter()
            .any(|row| matches!(row.target, BoundEffectTarget::Contribution { .. }))
    );
}

#[test]
fn changed_rolls_and_independent_workers_keep_scratch_occurrence_bound() {
    let mut f = fixture();
    let a = Arc::new(f.compile().unwrap());
    let reference = a.evaluate(&mut a.new_scratch()).unwrap();
    known_totals(&reference, 8);
    f.build.items[0].modifiers[0].rolls[0].value = quantity(12.0);
    f.build.revision = BuildRevision::from_u64(2);
    let b = Arc::new(f.compile().unwrap());
    assert_ne!(a.identity(), b.identity());
    let changed = b.evaluate(&mut b.new_scratch()).unwrap();
    known_totals(&changed, 20);
    let mut scratch = a.new_scratch();
    assert_eq!(a.evaluate(&mut scratch).unwrap(), reference);
    assert_eq!(b.evaluate(&mut scratch).unwrap(), changed);
    assert_eq!(a.evaluate(&mut scratch).unwrap(), reference);
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let (a, b, reference, changed) =
                (a.clone(), b.clone(), reference.clone(), changed.clone());
            std::thread::spawn(move || {
                let mut scratch = a.new_scratch();
                for _ in 0..4 {
                    assert_eq!(a.evaluate(&mut scratch).unwrap(), reference);
                    assert_eq!(b.evaluate(&mut scratch).unwrap(), changed);
                    assert_eq!(a.evaluate(&mut scratch).unwrap(), reference);
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
}
