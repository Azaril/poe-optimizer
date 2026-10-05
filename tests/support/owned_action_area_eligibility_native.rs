//! Actual published eligibility programs; no supplied Area facts or usage values.
#[allow(dead_code)]
#[path = "owned_bidding_delivery_fixture.rs"]
mod bidding_fixture;
#[allow(dead_code)]
#[path = "owned_magnified_area_fixture.rs"]
mod fixture;
use fixture::*;
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_rules::*};
use poe_optimizer_engine::owned_plan::*;
use poe_optimizer_import::owned_release_migration::OwnedReleaseMigrationInput;
use rayon::prelude::*;
use std::{collections::BTreeSet, path::PathBuf, sync::OnceLock};

fn world() -> World {
    static WORLD: OnceLock<World> = OnceLock::new();
    WORLD.get_or_init(|| {
        let path = PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_AREA_RELEASE").expect("checked Area publication"));
        let endpoint = crate::release::load(&path);
        crate::family::assert_endpoint(&endpoint);
        let migration: OwnedReleaseMigrationInput = crate::family::read("migration.json");
        let mut w = World::load_release(&path, false);
        w.set_area_fact(None);
        assert!(w.area_usage.is_empty());
        assert_eq!(w.inner.operations.as_str(), OWNED_RULE_OPERATIONS_V20);
        for owner in &migration.owners {
            let actual = endpoint.input().recipe.rules.owners.iter().find(|r| r.owner == owner.owner).unwrap();
            assert_eq!(actual, owner);
            let program = actual.programs.members.iter().find(|p| p.id.as_str() == crate::family::PROGRAM).unwrap().clone();
            w.inner.owner_mut(owner.owner.clone()).programs.members.push(program);
        }
        let channel: StatDefId = decode(&w.bindings["channels"]["area_eligible"]);
        let producers: Vec<_> = w.inner.owners.iter().flat_map(|o| &o.programs.members)
            .filter(|p| p.effects.iter().any(|e| matches!(&e.effect, RuleEffectKind::Derive { stat, .. } if stat == &channel))).collect();
        assert_eq!(producers.len(), 11);
        assert!(producers.iter().all(|p| p.id.as_str() == crate::family::PROGRAM));
        w
    }).clone()
}

fn effects(report: &SupportEffectsReport) -> &OwnedEffectsReport {
    let SupportEffectsOutcome::Evaluated { effects } = &report.outcome else {
        panic!("finite owned component unavailable: {report:?}")
    };
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    effects
}
fn evaluate(w: &World) -> SupportEffectsReport {
    let p = w.plan();
    p.evaluate(&mut p.new_scratch()).unwrap()
}
fn check(w: &World, report: &SupportEffectsReport, tier: &str) {
    assert!(w.request().scenario().input().usage.is_empty());
    let r = effects(report);
    let area_flag: StatDefId = decode(&w.bindings["channels"]["area_eligible"]);
    let channels: Vec<StatDefId> = ["area", "cost_factor", "damage_factor"]
        .into_iter()
        .map(|n| decode(&w.bindings["channels"][n]))
        .collect();
    let percent: UnitDefId = decode(&w.bindings["percent_unit"]);
    let ratio: UnitDefId = decode(&w.bindings["factor_unit"]);
    let mut seen = BTreeSet::new();
    for source in 0..=w.ice_source {
        for action in w.actions(source) {
            let expected = w.source_fact(&action)["area_eligible"].as_bool().unwrap();
            let entity = ConcreteEntity::Action(Box::new(action.clone()));
            let values: Vec<_> = r
                .values
                .iter()
                .filter(|v| {
                    v.key
                        == PlanValueKey::Stat {
                            entity: entity.clone(),
                            stat: area_flag.clone(),
                        }
                })
                .collect();
            assert_eq!(values.len(), 1);
            assert_eq!(
                values[0].value,
                EffectValue::Known {
                    value: ParameterValue::Boolean(expected)
                }
            );
            let producers: Vec<_> = r
                .effects
                .iter()
                .filter(|e| {
                    e.key.invocation.program.as_str() == crate::family::PROGRAM
                        && e.key.invocation.entity == entity
                })
                .collect();
            assert_eq!(producers.len(), 1);
            assert_eq!(producers[0].value, values[0].value);
            assert!(
                matches!(&producers[0].key.invocation.origin, RuleOrigin::Provider { provider } if provider == &action.action.provider)
            );
            assert!(seen.insert(action.clone()));
            let admitted = w.admitted(&action, tier);
            for (i, stat) in channels.iter().enumerate() {
                let rows: Vec<_> = r.effects.iter().filter(|e| matches!(&e.target, BoundEffectTarget::Contribution { key } if key.entity == entity && &key.stat == stat)).collect();
                assert_eq!(rows.len(), usize::from(i < 2 || tier == II));
                if rows.is_empty() {
                    continue;
                }
                let RuleOrigin::SupportApplication { application } = &rows[0].key.invocation.origin
                else {
                    panic!("retained support source")
                };
                assert_eq!(
                    application.receiver,
                    SupportReceiverKey::Action(Box::new(action.clone()))
                );
                assert_eq!(
                    application.prepared.target,
                    SkillTarget::Authored(id(20 + source as u64))
                );
                if !admitted || (i == 2 && !expected) {
                    assert_eq!(rows[0].value, EffectValue::Inactive);
                } else {
                    let (amount, unit) = match i {
                        0 => (if tier == I { 35.0 } else { 45.0 }, &percent),
                        1 => (1.3, &ratio),
                        _ => (1.0, &ratio),
                    };
                    assert_eq!(
                        rows[0].value,
                        EffectValue::Known {
                            value: quantity(amount, unit)
                        }
                    );
                }
            }
        }
    }
    assert_eq!(seen.len(), 14);
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_AREA_RELEASE checked publication"]
fn actual_output_rules_produce_all_fourteen_flags_and_gate_both_support_tiers() {
    let mut w = world();
    check(&w, &evaluate(&w), I);
    for source in 0..=w.ice_source {
        w.inner.change_support(source, II);
    }
    check(&w, &evaluate(&w), II);
    let flags: BTreeSet<_> = w
        .actions(0)
        .iter()
        .map(|a| w.source_fact(a)["area_eligible"].as_bool().unwrap())
        .collect();
    assert_eq!(
        flags,
        [false, true].into_iter().collect(),
        "contrasting stat sets stay separate"
    );
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_AREA_RELEASE checked publication"]
fn removing_real_eligibility_producer_keeps_damage_unresolved_and_other_lanes_known() {
    let mut w = world();
    for source in 0..=w.ice_source {
        w.inner.change_support(source, II);
    }
    let target = w.actions(w.ice_source)[0].action.output.clone();
    w.inner
        .owners
        .iter_mut()
        .find(|o| {
            o.owner
                == poe_optimizer_core::owned_schema::SchemaSubject::Slot(
                    poe_optimizer_core::owned_schema::SlotAddress::ActionOutput(target.clone()),
                )
        })
        .unwrap()
        .programs
        .members
        .retain(|p| p.id.as_str() != crate::family::PROGRAM);
    let r = evaluate(&w);
    let r = effects(&r);
    let area_flag: StatDefId = decode(&w.bindings["channels"]["area_eligible"]);
    let damage: StatDefId = decode(&w.bindings["channels"]["damage_factor"]);
    for action in w.actions(w.ice_source) {
        let entity = ConcreteEntity::Action(Box::new(action));
        let missing = r.values.iter().find(|v| {
            v.key
                == PlanValueKey::Stat {
                    entity: entity.clone(),
                    stat: area_flag.clone(),
                }
        });
        assert!(
            missing.is_none(),
            "an absent producer must not invent a final stat value"
        );
        let rows:Vec<_> = r.effects.iter().filter(|e|matches!(&e.target,BoundEffectTarget::Contribution { key } if key.entity==entity)).collect();
        assert_eq!(rows.len(), 3);
        for row in rows {
            let BoundEffectTarget::Contribution { key } = &row.target else {
                unreachable!()
            };
            if key.stat == damage {
                assert!(
                    matches!(&row.value,EffectValue::Unresolved { reason:PlanGapReason::MissingProducer, read:Some(read) } if read.as_str()=="area-eligible")
                );
            } else {
                assert!(matches!(row.value, EffectValue::Known { .. }));
            }
        }
    }
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_AREA_RELEASE checked publication"]
fn native_eligibility_repeats_across_scratch_and_parallel_workers() {
    let mut w = world();
    let a = w.plan();
    for source in 0..=w.ice_source {
        w.inner.change_support(source, II);
    }
    let b = w.plan();
    let mut scratch = a.new_scratch();
    let first = a.evaluate(&mut scratch).unwrap();
    let second = b.evaluate(&mut scratch).unwrap();
    check(&w, &second, II);
    assert!(
        first != second,
        "different support tier must change the result"
    );
    assert!(
        a.evaluate(&mut scratch).unwrap() == first,
        "same-request A/B/A replay changed"
    );
    let rows: Vec<_> = (0..16)
        .into_par_iter()
        .map(|_| b.evaluate(&mut b.new_scratch()).unwrap())
        .collect();
    assert!(rows.iter().all(|r| r == &second));
}
