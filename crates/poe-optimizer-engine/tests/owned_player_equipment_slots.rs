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
    other.modifier_order.clear();
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
