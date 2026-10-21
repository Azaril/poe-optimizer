//! Slot presence and exact computed occurrence values through shared Player rules.
#[path = "support/owned_player_equipment_slots_fixture.rs"]
mod fixture;
use fixture::*;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_readiness::*, owned_rules::*, owned_schema::*,
    owned_stages::*,
};
use poe_optimizer_engine::{owned_plan::*, owned_rules::CompiledRulePackage};
use rayon::prelude::*;
use std::sync::Arc;

#[test]
fn occupied_caster_and_empty_slot_have_distinct_values_with_lazy_empty_branch() {
    let mut f = fixture();
    let report = evaluate(&f);
    let r = delivery::evaluated(&report);
    assert_eq!(count(r), 8);
    for slot in ["weapon", "other"] {
        assert_eq!(value(r, &format!("{slot}-occupied")), &boolean(true));
        assert_eq!(value(r, &format!("{slot}-capability")), &boolean(false));
        assert_eq!(value(r, &format!("{slot}-value")), &delivery::known(27));
    }
    f.build.equipment.retain(|e| e.id != occurrence(6));
    let report = evaluate(&f);
    let r = delivery::evaluated(&report);
    assert_eq!(value(r, "weapon-occupied"), &boolean(false));
    for name in ["weapon-capability", "weapon-value"] {
        assert!(matches!(
            value(r, name),
            EffectValue::Unresolved {
                reason: PlanGapReason::MissingProducer,
                ..
            }
        ));
    }
    assert_eq!(value(r, "weapon-lazy"), &delivery::known(17));
    assert_eq!(value(r, "other-value"), &delivery::known(27));
}

#[test]
fn repeated_templates_and_loadout_alternatives_rebind_exact_equipment_uses() {
    let mut f = fixture();
    let mut other = f.build.items[0].clone();
    other.id = occurrence(83);
    other.item_level = Some(41);
    other.modifiers.clear();
    other.modifier_orders.contributions.clear();
    other.modifier_orders.numeric_transforms.clear();
    other.parameters[0].value = ParameterValue::Boolean(true);
    f.build.items.push(other);
    f.build.equipment[1].item = occurrence(83);
    f.build.equipment[0].scope = LoadoutScope::Selected {
        loadouts: vec![occurrence(1)],
    };
    let mut alternative = f.build.equipment[0].clone();
    alternative.id = occurrence(84);
    alternative.item = occurrence(83);
    alternative.scope = LoadoutScope::Selected {
        loadouts: vec![occurrence(2)],
    };
    f.build.equipment.push(alternative);
    let report = evaluate(&f);
    let r = delivery::evaluated(&report);
    assert_eq!(value(r, "weapon-value"), &delivery::known(27));
    assert_eq!(value(r, "other-value"), &delivery::known(41));
    assert_eq!(value(r, "weapon-capability"), &boolean(false));
    assert_eq!(value(r, "other-capability"), &boolean(true));
    f.build.active_weapon_loadout = occurrence(2);
    let report = evaluate(&f);
    let r = delivery::evaluated(&report);
    assert_eq!(value(r, "weapon-value"), &delivery::known(41));
    assert_eq!(value(r, "weapon-capability"), &boolean(true));
    f.build.equipment.reverse();
    let reversed = evaluate(&f);
    assert_eq!(
        value(delivery::evaluated(&reversed), "weapon-value"),
        &delivery::known(41)
    );
}

#[test]
fn ambiguous_or_unknown_occupants_never_become_empty_and_partial_mechanics_stay_blocked() {
    let mut f = fixture();
    f.build.equipment[1].destination = f.build.equipment[0].destination.clone();
    let report = evaluate(&f);
    let r = delivery::evaluated(&report);
    for name in [
        "weapon-occupied",
        "weapon-value",
        "weapon-capability",
        "weapon-lazy",
    ] {
        assert!(matches!(
            value(r, name),
            EffectValue::Unresolved {
                reason: PlanGapReason::UnsupportedRelation,
                ..
            }
        ));
    }
    for unknown in [false, true] {
        let mut f = fixture();
        if unknown {
            for d in &mut f.schema.definitions {
                if let DefinitionDescriptor::ItemTemplate(e) = d {
                    e.schema = SchemaState::Unmapped {
                        gaps: vec![SchemaGap {
                            subject: base::item_owner(),
                            facet: SchemaFacet::InputSchema,
                            code: key("unknown-item"),
                        }],
                    };
                }
            }
            f.owners.retain(|o| o.owner != base::item_owner());
        } else {
            f.owner_mut(&base::item_owner()).programs.closure =
                readiness::partial(base::item_owner(), SchemaFacet::GameRules);
        }
        let report = evaluate(&f);
        assert!(matches!(
            report.outcome,
            SupportEffectsOutcome::Unavailable { .. }
        ));
        assert!(report.gaps.iter().any(|g| g.reason
            == if unknown {
                PlanGapReason::SchemaUnresolved
            } else {
                PlanGapReason::PartialPrograms
            }));
    }
}

#[test]
fn occupied_missing_computed_producer_is_unresolved_and_does_not_seed_false() {
    let mut f = fixture();
    f.owner_mut(&base::item_owner())
        .programs
        .members
        .retain(|p| p.id != key("slot-capability") && p.id != key("slot-value"));
    let report = evaluate(&f);
    let r = delivery::evaluated(&report);
    assert_eq!(value(r, "weapon-occupied"), &boolean(true));
    for name in ["weapon-capability", "weapon-value", "weapon-lazy"] {
        assert!(matches!(
            value(r, name),
            EffectValue::Unresolved {
                reason: PlanGapReason::MissingProducer,
                ..
            }
        ));
    }
}

#[test]
fn computed_slot_reads_join_actual_readiness_stage_and_cycle_proofs() {
    let mut f = fixture();
    let cycle = f
        .owner_mut(&base::item_owner())
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == key("slot-value"))
        .unwrap();
    cycle.reads[0].source = RuleReadSource::Stat {
        entity: RuleEntity::Player,
        stat: def("weapon-value"),
    };
    rejected(compile(&f), "cycle");
    let f = fixture();
    let early = |s: &mut EvaluationStagesInput| {
        edit_early(
            s,
            &actor_owner(),
            "read-weapon",
            ReadinessPhase::Preparation,
            ["occupied", "capability", "value", "lazy"]
                .into_iter()
                .map(|n| StageChannel::Stat {
                    scope: RuleEntityKind::Actor,
                    stat: def(&format!("weapon-{n}")),
                })
                .collect(),
        )
    };
    rejected(
        inputs_with(&f, |_| {}, early).and_then(readiness::compile_inputs),
        "later",
    );
}

#[test]
fn query_order_and_worker_reuse_do_not_change_shared_player_slot_state() {
    let a = fixture();
    let mut b = fixture();
    b.build.equipment.retain(|e| e.id != occurrence(6));
    let plans = [
        Arc::new(compile(&a).unwrap()),
        Arc::new(compile(&b).unwrap()),
    ];
    let expected: Vec<_> = plans
        .iter()
        .map(|p| p.evaluate(&mut p.new_scratch()).unwrap())
        .collect();
    let mut scratch = plans[0].new_scratch();
    for i in [0, 1, 0] {
        assert_eq!(plans[i].evaluate(&mut scratch).unwrap(), expected[i]);
    }
    let results: Vec<_> = (0..12)
        .into_par_iter()
        .map_init(
            || plans[0].new_scratch(),
            |s, i| (i % 2, plans[i % 2].evaluate(s).unwrap()),
        )
        .collect();
    for (i, r) in results {
        assert_eq!(r, expected[i]);
    }
    for empty in [false, true] {
        let mut f = fixture();
        f.queries.requests.reverse();
        if empty {
            f.queries.requests.clear();
        }
        let report = evaluate(&f);
        let r = delivery::evaluated(&report);
        assert_eq!(count(r), 8);
        for name in ["weapon-occupied", "weapon-value", "other-value"] {
            assert_eq!(
                value(r, name),
                value(delivery::evaluated(&expected[0]), name)
            );
        }
    }
}

#[test]
fn raw_compiler_cannot_bypass_slot_type_namespace_or_existing_player_authority() {
    let f = fixture();
    let mut raw = None;
    let input = inputs_with(&f, |r| raw = Some(r.clone()), |_| {}).unwrap();
    let raw = raw.unwrap();
    for case in [
        "unlisted",
        "old",
        "wrong-type",
        "unknown-slot",
        "raw-parameter",
    ] {
        let mut rules = raw.clone();
        if case == "unlisted" {
            rules.existing_actor_rules = None;
        }
        if case == "old" {
            rules.operations_version = key(OWNED_RULE_OPERATIONS_V20);
            rules.contribution_queries = None;
        }
        let p = &mut rules
            .owners
            .iter_mut()
            .find(|o| o.owner == actor_owner())
            .unwrap()
            .programs
            .members[0];
        match case {
            "wrong-type" => p.reads[0].value_type = ComputedValueType::Integer,
            "unknown-slot" => {
                p.reads[0].source = RuleReadSource::PlayerEquipmentSlot {
                    slot: def("unknown"),
                    read: PlayerEquipmentSlotRead::Occupied,
                }
            }
            "raw-parameter" => {
                p.reads[0].source = RuleReadSource::Parameter {
                    slot: base::parameter(SlotOwnerDefId::ItemTemplate(def("item")), "needs-level"),
                }
            }
            _ => {}
        }
        assert!(
            CompiledRulePackage::compile(&rules, input.definitions.as_ref(), Default::default())
                .is_err(),
            "{case}"
        );
    }
}

#[test]
fn listed_actor_owner_does_not_authorize_a_generated_actor_invocation() {
    let mut f = fixture();
    f.build.supports.clear();
    f.build.authored_support_order = Some(vec![]);
    f.queries.requests.clear();
    for slot in &mut f.schema.slots {
        if let SlotDescriptor::Actor(e) = slot
            && e.id == readiness::actor_slot()
            && let SchemaState::Known(s) = &mut e.schema
        {
            s.provider_definition = Some(def("slot-player"));
            s.skills = DeclaredSet::complete(vec![]);
        }
    }
    rejected(compile(&f), "explicit existing Player Actor invocation");
}

// This is an Engine-contract witness using synthetic same-group definitions,
// not canonical FireResist admission or a complete PoB build. Original03's fire
// and all-elemental modifiers are distinct groups; their semantic admission and
// the producer/copy census remain separate data work.
mod grouped_slot_copy_contract {
    use super::*;

    fn qty(n: f64) -> ParameterValue {
        ParameterValue::Quantity(FiniteQuantity::new(n, def("copy-percent")).unwrap())
    }
    fn quantity() -> ComputedValueType {
        ComputedValueType::Quantity {
            unit: def("copy-percent"),
        }
    }
    fn literal(name: &str, value: ParameterValue) -> RuleNode {
        base::node(name, RuleExpression::Literal { value })
    }
    fn read(name: &str, value_type: ComputedValueType, source: RuleReadSource) -> RuleRead {
        RuleRead {
            id: key(name),
            value_type,
            source,
        }
    }
    fn roll_slot() -> DeclaredSlot<ParameterSlotDefId> {
        base::parameter(SlotOwnerDefId::Modifier(def("modifier")), "copy-roll")
    }
    fn contribute(
        name: &str,
        entity: RuleEntity,
        stat: &str,
        kind: ContributionKind,
        value: &str,
    ) -> RuleEffect {
        effect(
            name,
            RuleEffectKind::Contribute {
                entity,
                stat: def(stat),
                contribution: kind,
                value: key(value),
            },
        )
    }
    fn set_rolls(item: &mut ItemRecord, values: &[f64]) {
        assert_eq!(item.modifiers.len(), values.len());
        for (m, &n) in item.modifiers.iter_mut().zip(values) {
            m.rolls.retain(|p| p.slot != roll_slot());
            m.rolls.push(ParameterAssignment {
                slot: roll_slot(),
                value: qty(n),
            });
        }
    }
    fn world(percent: f64) -> Fixture {
        let mut f = fixture();
        f.build.equipment.truncate(1);
        // The second semantic order is deliberately different. Contributions
        // must still follow 4,5, never the transforms permutation or storage.
        f.build.items[0]
            .modifier_orders
            .numeric_transforms
            .reverse();
        for (name, dimension) in [
            ("copy-percent", UnitDimension::PercentagePoints),
            ("copy-factor", UnitDimension::DimensionlessFactor),
        ] {
            f.schema.definitions.push(DefinitionDescriptor::Unit(known(
                def(name),
                UnitSchema { dimension },
            )));
        }
        for (name, value, target) in [
            ("copy-local-add", quantity(), RuleEntityKind::EquipmentUse),
            (
                "copy-local-present-add",
                ComputedValueType::Boolean,
                RuleEntityKind::EquipmentUse,
            ),
            ("copy-local", quantity(), RuleEntityKind::EquipmentUse),
            (
                "copy-local-present",
                ComputedValueType::Boolean,
                RuleEntityKind::EquipmentUse,
            ),
            ("copy-player-add", quantity(), RuleEntityKind::Actor),
            ("copy-final", quantity(), RuleEntityKind::Actor),
            (
                "copy-active",
                ComputedValueType::Boolean,
                RuleEntityKind::Actor,
            ),
            ("copy-value", quantity(), RuleEntityKind::Actor),
        ] {
            f.schema.definitions.push(DefinitionDescriptor::Stat(known(
                def(name),
                StatSchema {
                    value,
                    targets: vec![target],
                },
            )));
        }
        let DefinitionDescriptor::Modifier(row) = f.schema.definitions.iter_mut().find(|d| {
            matches!(d, DefinitionDescriptor::Modifier(row) if row.id == def::<ModifierDefinition>("modifier"))
        }).unwrap() else { unreachable!() };
        let SchemaState::Known(schema) = &mut row.schema else {
            unreachable!()
        };
        schema.declarations.parameters.members.push(roll_slot());
        f.schema.slots.push(SlotDescriptor::Parameter(known(
            roll_slot(),
            ParameterSlotSchema {
                skill_input: None,
                value: ValueSchema::Quantity(QuantityRange {
                    minimum: FiniteQuantity::new(-1e16, def("copy-percent")).unwrap(),
                    maximum: FiniteQuantity::new(1e16, def("copy-percent")).unwrap(),
                }),
                presence: SlotPresence::RequiredOnce,
                sites: vec![ParameterSite::ModifierRoll],
            },
        )));
        set_rolls(&mut f.build.items[0], &[22.0, 16.0]);
        f.owner_mut(&base::modifier_owner())
            .programs
            .members
            .push(RuleProgram {
                id: key("copy-supply"),
                context: RuleEntityKind::EquipmentUse,
                reads: vec![read(
                    "roll",
                    quantity(),
                    RuleReadSource::Parameter { slot: roll_slot() },
                )],
                nodes: vec![
                    base::read_node("roll", "roll"),
                    literal("present", ParameterValue::Boolean(true)),
                ],
                effects: vec![
                    contribute(
                        "local",
                        RuleEntity::Current,
                        "copy-local-add",
                        ContributionKind::Add,
                        "roll",
                    ),
                    contribute(
                        "present",
                        RuleEntity::Current,
                        "copy-local-present-add",
                        ContributionKind::Flag,
                        "present",
                    ),
                    contribute(
                        "direct",
                        RuleEntity::Player,
                        "copy-player-add",
                        ContributionKind::Add,
                        "roll",
                    ),
                ],
            });
        f.owner_mut(&base::item_owner())
            .programs
            .members
            .push(RuleProgram {
                id: key("copy-subtotal"),
                context: RuleEntityKind::EquipmentUse,
                reads: vec![
                    read(
                        "sum",
                        quantity(),
                        RuleReadSource::ContributionQuery {
                            entity: RuleEntity::Current,
                            query: key("copy-local-query"),
                            group: key("all"),
                        },
                    ),
                    read(
                        "present",
                        ComputedValueType::Boolean,
                        RuleReadSource::ContributionQuery {
                            entity: RuleEntity::Current,
                            query: key("copy-presence-query"),
                            group: key("all"),
                        },
                    ),
                ],
                nodes: vec![
                    base::read_node("sum", "sum"),
                    base::read_node("present", "present"),
                ],
                effects: vec![
                    base::derive("subtotal", RuleEntity::Current, "copy-local", "sum"),
                    base::derive(
                        "present",
                        RuleEntity::Current,
                        "copy-local-present",
                        "present",
                    ),
                ],
            });
        let mut copy = contribute(
            "copy",
            RuleEntity::Current,
            "copy-player-add",
            ContributionKind::Add,
            "rounded",
        );
        copy.when = Some(key("active"));
        f.owner_mut(&actor_owner()).programs.members.extend([
            RuleProgram {
                id: key("copy-selected-slot"),
                context: RuleEntityKind::Actor,
                reads: vec![
                    slot_read(
                        "occupied",
                        "weapon",
                        PlayerEquipmentSlotRead::Occupied,
                        ComputedValueType::Boolean,
                    ),
                    slot_read(
                        "sum",
                        "weapon",
                        PlayerEquipmentSlotRead::Stat {
                            stat: def("copy-local"),
                        },
                        quantity(),
                    ),
                    slot_read(
                        "present",
                        "weapon",
                        PlayerEquipmentSlotRead::Stat {
                            stat: def("copy-local-present"),
                        },
                        ComputedValueType::Boolean,
                    ),
                ],
                nodes: vec![
                    base::read_node("occupied", "occupied"),
                    base::read_node("sum", "sum"),
                    base::read_node("present", "present"),
                    literal("percent", qty(percent)),
                    literal("zero", qty(0.0)),
                    literal("false", ParameterValue::Boolean(false)),
                    base::node(
                        "positive",
                        RuleExpression::Compare {
                            operation: RuleComparison::Greater,
                            left: key("percent"),
                            right: key("zero"),
                        },
                    ),
                    base::node(
                        "present-positive",
                        RuleExpression::All {
                            values: vec![key("present"), key("positive")],
                        },
                    ),
                    base::node(
                        "active",
                        RuleExpression::Select {
                            condition: key("occupied"),
                            when_true: key("present-positive"),
                            when_false: key("false"),
                        },
                    ),
                    base::node(
                        "factor",
                        RuleExpression::PercentAsFactor {
                            percent: key("percent"),
                            unit: def("copy-factor"),
                        },
                    ),
                    base::node(
                        "scaled",
                        RuleExpression::Scale {
                            value: key("sum"),
                            factor: key("factor"),
                        },
                    ),
                    // Only the finite integral positive-total branch is claimed.
                    // Fractional/high-precision/tagged groups need their own data.
                    base::node(
                        "hundredth",
                        RuleExpression::Round {
                            value: key("scaled"),
                            quantum: FiniteQuantity::new(0.01, def("copy-percent")).unwrap(),
                            mode: RuleRounding::NearestTiesPositive,
                        },
                    ),
                    base::node(
                        "rounded",
                        RuleExpression::Round {
                            value: key("hundredth"),
                            quantum: FiniteQuantity::new(1.0, def("copy-percent")).unwrap(),
                            mode: RuleRounding::Truncate,
                        },
                    ),
                    base::node(
                        "diagnostic",
                        RuleExpression::Select {
                            condition: key("active"),
                            when_true: key("rounded"),
                            when_false: key("zero"),
                        },
                    ),
                ],
                effects: vec![
                    copy,
                    base::derive("active", RuleEntity::Current, "copy-active", "active"),
                    base::derive("value", RuleEntity::Current, "copy-value", "diagnostic"),
                ],
            },
            RuleProgram {
                id: key("copy-final"),
                context: RuleEntityKind::Actor,
                reads: vec![read(
                    "sum",
                    quantity(),
                    RuleReadSource::ContributionQuery {
                        entity: RuleEntity::Current,
                        query: key("copy-final-query"),
                        group: key("all"),
                    },
                )],
                nodes: vec![base::read_node("sum", "sum")],
                effects: vec![base::derive(
                    "final",
                    RuleEntity::Current,
                    "copy-final",
                    "sum",
                )],
            },
        ]);
        f
    }
    fn member(effect_name: &str) -> ContributionMember {
        ContributionMember {
            producer: ContributionProducer::ProgramEffect(ProgramContributionProducer {
                owner: base::modifier_owner(),
                program: key("copy-supply"),
                effect: key(effect_name),
                origin: ContributionOrigin::ItemModifier {
                    slots: vec![def("weapon"), def("other")],
                },
            }),
            order: Some(ContributionOrder {
                source_rank: 0,
                program_rank: 0,
                effect_rank: 0,
                slot_ranks: vec![
                    ContributionSlotRank {
                        slot: def("weapon"),
                        rank: 0,
                    },
                    ContributionSlotRank {
                        slot: def("other"),
                        rank: 1,
                    },
                ],
            }),
        }
    }
    fn queries() -> DeclaredSet<ContributionQuery> {
        let mut flag = member("present");
        flag.order = None;
        let copy = ContributionMember {
            producer: ContributionProducer::ProgramEffect(ProgramContributionProducer {
                owner: actor_owner(),
                program: key("copy-selected-slot"),
                effect: key("copy"),
                origin: ContributionOrigin::ExistingActor {
                    application: key("slot-player"),
                },
            }),
            order: Some(ContributionOrder {
                source_rank: 1,
                program_rank: 0,
                effect_rank: 0,
                slot_ranks: vec![],
            }),
        };
        DeclaredSet::complete(
            [
                (
                    "copy-local-query",
                    "copy-local-add",
                    ContributionKind::Add,
                    vec![member("local")],
                ),
                (
                    "copy-presence-query",
                    "copy-local-present-add",
                    ContributionKind::Flag,
                    vec![flag],
                ),
                (
                    "copy-final-query",
                    "copy-player-add",
                    ContributionKind::Add,
                    vec![member("direct"), copy],
                ),
            ]
            .into_iter()
            .map(|(id, stat, contribution, members)| {
                let boolean = contribution == ContributionKind::Flag;
                ContributionQuery {
                    id: key(id),
                    stat: def(stat),
                    contribution,
                    groups: vec![ContributionGroup {
                        id: key("all"),
                        reduction: if boolean {
                            ContributionReduction::Any
                        } else {
                            ContributionReduction::Sum
                        },
                        ordering: if boolean {
                            ContributionOrdering::Unordered
                        } else {
                            ContributionOrdering::Ordered
                        },
                        empty: Some(if boolean {
                            ParameterValue::Boolean(false)
                        } else {
                            qty(0.0)
                        }),
                        members: DeclaredSet::complete(members),
                    }],
                }
            })
            .collect(),
        )
    }
    fn plan(f: &Fixture, q: DeclaredSet<ContributionQuery>) -> Effects {
        readiness::compile_inputs(
            inputs_with(
                f,
                |r| {
                    r.operations_version = key(OWNED_RULE_OPERATIONS_V23);
                    r.contribution_queries = Some(q);
                },
                |_| {},
            )
            .unwrap(),
        )
        .unwrap()
    }
    fn report(f: &Fixture) -> OwnedEffectsReport {
        let p = plan(f, queries());
        let result = p.evaluate(&mut p.new_scratch()).unwrap();
        assert!(result.gaps.is_empty(), "{:?}", result.gaps);
        delivery::evaluated(&result).clone()
    }
    fn expected(r: &OwnedEffectsReport, copy: f64, final_value: f64, active: bool) {
        assert_eq!(
            value(r, "copy-value"),
            &EffectValue::Known { value: qty(copy) }
        );
        assert_eq!(
            value(r, "copy-final"),
            &EffectValue::Known {
                value: qty(final_value)
            }
        );
        assert_eq!(value(r, "copy-active"), &boolean(active));
        let copies: Vec<_> = r
            .effects
            .iter()
            .filter(|e| {
                e.key.invocation.program == key("copy-selected-slot") && e.key.effect == key("copy")
            })
            .collect();
        assert_eq!(copies.len(), 1);
        assert_eq!(
            copies[0].value,
            if active {
                EffectValue::Known { value: qty(copy) }
            } else {
                EffectValue::Inactive
            }
        );
    }

    #[test]
    fn same_group_is_rounded_after_sum_and_direct_values_are_delivered_once() {
        for (percent, copy, total) in [(28.0, 10.0, 48.0), (30.0, 11.0, 49.0)] {
            let f = world(percent);
            let r = report(&f);
            expected(&r, copy, total, true);
            let direct: Vec<_> = r
                .effects
                .iter()
                .filter(|e| {
                    e.key.invocation.program == key("copy-supply") && e.key.effect == key("direct")
                })
                .collect();
            assert_eq!(direct.len(), 2);
            for (id, amount) in [(4, 22.0), (5, 16.0)] {
                let exact = direct
                    .iter()
                    .find(|e| {
                        e.key.invocation.origin
                            == RuleOrigin::Provider {
                                provider: ProviderKey {
                                    root: ProviderRoot::ItemModifier {
                                        equipment_use: occurrence(6),
                                        modifier: occurrence(id),
                                    },
                                    grant_path: vec![],
                                },
                            }
                    })
                    .unwrap();
                assert_eq!(exact.value, EffectValue::Known { value: qty(amount) });
            }
        }
        // The 30% witness discriminates from summing separately truncated6+4.
        assert_ne!(11.0, (22.0_f64 * 0.3).trunc() + (16.0_f64 * 0.3).trunc());
    }

    #[test]
    fn present_zero_absent_group_empty_slot_and_nonpositive_bonus_stay_distinct() {
        let mut zero = world(30.0);
        set_rolls(&mut zero.build.items[0], &[0.0, 0.0]);
        expected(&report(&zero), 0.0, 0.0, true);
        let mut absent = zero;
        absent.build.items[0].modifiers.clear();
        absent.build.items[0].modifier_orders.contributions.clear();
        absent.build.items[0]
            .modifier_orders
            .numeric_transforms
            .clear();
        expected(&report(&absent), 0.0, 0.0, false);
        let mut empty = world(30.0);
        empty.build.equipment.clear();
        expected(&report(&empty), 0.0, 0.0, false);
        for percent in [0.0, -25.0] {
            expected(&report(&world(percent)), 0.0, 38.0, false);
        }
    }

    #[test]
    fn selected_slot_reads_exact_use_and_rebinds_alternatives_without_cross_slot_copy() {
        let mut f = world(30.0);
        let mut other = f.build.items[0].clone();
        other.id = occurrence(83);
        other.modifiers[0].id = occurrence(84);
        other.modifiers[1].id = occurrence(85);
        other.modifier_orders = ItemModifierOrders {
            contributions: vec![occurrence(84), occurrence(85)],
            numeric_transforms: vec![occurrence(85), occurrence(84)],
        };
        set_rolls(&mut other, &[10.0, 3.0]);
        f.build.items.push(other);
        f.build.equipment[0].scope = LoadoutScope::Selected {
            loadouts: vec![occurrence(1)],
        };
        f.build.equipment.push(EquipmentUse {
            id: occurrence(7),
            item: occurrence(83),
            destination: EquipmentDestination::CharacterSlot(def("other")),
            scope: LoadoutScope::Shared,
        });
        f.build.equipment.push(EquipmentUse {
            id: occurrence(86),
            item: occurrence(83),
            destination: EquipmentDestination::CharacterSlot(def("weapon")),
            scope: LoadoutScope::Selected {
                loadouts: vec![occurrence(2)],
            },
        });
        expected(&report(&f), 11.0, 62.0, true); //38 +13 +11
        f.build.active_weapon_loadout = occurrence(2);
        expected(&report(&f), 3.0, 29.0, true); //13 +13 +3
        f.build.equipment.reverse();
        f.build.items.reverse();
        expected(&report(&f), 3.0, 29.0, true);
    }

    #[test]
    fn subtotal_follows_contribution_order_and_query_selection_does_not_change_it() {
        let mut f = world(100.0);
        let mut third = f.build.items[0].modifiers[0].clone();
        third.id = occurrence(87);
        f.build.items[0].modifiers.push(third);
        f.build.items[0].modifier_orders = ItemModifierOrders {
            contributions: vec![occurrence(4), occurrence(5), occurrence(87)],
            numeric_transforms: vec![occurrence(4), occurrence(87), occurrence(5)],
        };
        set_rolls(&mut f.build.items[0], &[1e16, -1e16, 1.0]);
        expected(&report(&f), 1.0, 2.0, true);
        f.build.items[0].modifiers.reverse();
        f.build.items[0]
            .modifier_orders
            .numeric_transforms
            .reverse();
        f.queries.requests.clear();
        expected(&report(&f), 1.0, 2.0, true);
        f.build.items[0].modifier_orders.contributions.swap(1, 2);
        expected(&report(&f), 0.0, 0.0, true);
    }

    #[test]
    fn partial_subtotal_census_is_unavailable_not_a_present_zero_or_usable_copy() {
        let f = world(30.0);
        let mut q = queries();
        q.members[0].groups[0].members.closure = readiness::partial(
            subject(def::<StatDefinition>("copy-local-add")),
            SchemaFacet::GameRules,
        );
        let p = plan(&f, q);
        let report = p.evaluate(&mut p.new_scratch()).unwrap();
        assert!(
            report
                .gaps
                .iter()
                .any(|g| g.reason == PlanGapReason::IncompleteContributors
                    && g.subject == Some(subject(def::<StatDefinition>("copy-local-add")))
                    && g.provider.is_none())
        );
        assert!(matches!(
            report.outcome,
            SupportEffectsOutcome::Unavailable {
                cause: EffectValue::Unresolved {
                    reason: PlanGapReason::IncompleteContributors,
                    read: None
                },
                input: None,
            }
        ));
    }
}
