//! Historical Offering component checks. Shared construction has no nested tests.
#[allow(dead_code)]
#[path = "owned_offering_final_inputs_fixture.rs"]
pub(crate) mod fixture;
use fixture::*;
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_rules::*, owned_stages::*};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use std::sync::Arc;

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_OFFERING_INPUTS_RELEASE; finite native component"]
fn offering_inputs_use_actual_items_and_independent_raw_sources() {
    let mut w = World::load();
    let r = evaluate(&w);
    check(&w, &r, [22, 22], [0.0, 0.0]);
    let direct: Vec<_> = effects(&r)
        .effects
        .iter()
        .filter(|e| {
            e.key.invocation.program == key("contribute-player-minion-gem-level")
                && e.value != EffectValue::Inactive
        })
        .collect();
    assert_eq!(direct.len(), 2);
    assert_ne!(direct[0].key, direct[1].key);
    for (equipment, modifier) in [(6002, 6001), (6012, 6011)] {
        let rows:Vec<_>=direct.iter().filter(|e|matches!(&e.key.invocation.origin,RuleOrigin::Provider{provider}
            if provider.root==ProviderRoot::ItemModifier{equipment_use:id(equipment),modifier:id(modifier)} && provider.grant_path.is_empty())).collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].value,
            EffectValue::Known {
                value: quantity(1.0, &def("def.000000000000295a"))
            }
        );
        assert!(
            matches!(&rows[0].target,BoundEffectTarget::Contribution{key}
            if key.entity==ConcreteEntity::Actor(ActorKey::Player) && key.stat==decode::<StatDefId>(&w.bindings["channels"]["global_minion_level"]) && key.kind==ContributionKind::Add)
        );
    }
    let copy: Vec<_> = effects(&r)
        .effects
        .iter()
        .filter(|e| {
            e.key.invocation.program == key("amulet-copy-minion-gem-level")
                && e.value != EffectValue::Inactive
        })
        .collect();
    assert_eq!(copy.len(), 1);
    assert_eq!(
        copy[0].value,
        EffectValue::Known {
            value: quantity(0.0, &def("def.000000000000295a"))
        }
    );
    assert!(
        matches!(&copy[0].key.invocation.origin,RuleOrigin::Provider{provider}
        if provider.root==ProviderRoot::ItemModifier{equipment_use:id(6002),modifier:id(6001)} && provider.grant_path.is_empty())
    );
    assert_eq!(
        w.tables[0].rows[21],
        quantity(62.0, &def("def.0000000000000002"))
    );
    w.raw(3, 17, 9.0);
    w.raw(4, 23, 15.0);
    check(&w, &evaluate(&w), [19, 25], [9.0, 15.0]);
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_OFFERING_INPUTS_RELEASE; finite native component"]
fn offering_inputs_respond_to_item_rolls_removal_and_explicit_snapshot() {
    let mut w = World::load();
    w.item_amount("helmet", 2.0);
    check(&w, &evaluate(&w), [23, 23], [0.0, 0.0]);
    w.snapshot(100.0);
    check(&w, &evaluate(&w), [24, 24], [0.0, 0.0]);
    w.remove_item("amulet");
    check(&w, &evaluate(&w), [22, 22], [0.0, 0.0]);
    w.remove_item("helmet");
    check(&w, &evaluate(&w), [20, 20], [0.0, 0.0]);
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_OFFERING_INPUTS_RELEASE; finite native component"]
fn offering_inputs_retain_both_support_tiers_removal_and_disabled_origins() {
    let mut w = World::load();
    for tier in [0usize, 1] {
        w.source.base.inner.build.supports.clear();
        w.source.base.inner.build.support_origins = Some(vec![]);
        for source in [2, 3, 4] {
            w.source.base.inner.add_support(source, tier);
        }
        let report = evaluate(&w);
        check(&w, &report, [22, 22], [0.0, 0.0]);
        check_census_and_delivery(&w, &report, Some(tier));
    }
    for s in &mut w.source.base.inner.build.supports {
        s.enabled = false;
    }
    let report = evaluate(&w);
    check(&w, &report, [22, 22], [0.0, 0.0]);
    check_census_and_delivery(&w, &report, None);
    w.source.base.inner.build.supports.clear();
    w.source.base.inner.build.support_origins = Some(vec![]);
    let report = evaluate(&w);
    check(&w, &report, [22, 22], [0.0, 0.0]);
    check_census_and_delivery(&w, &report, None);
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_OFFERING_INPUTS_RELEASE; finite native component"]
fn offering_inputs_refuse_fractional_out_of_domain_and_missing_producers() {
    for fractional in [true, false] {
        let mut w = World::load();
        if fractional {
            // Count is a quantity before assembly. The actual item numeric
            // recipe rounds ordinary item values, so use the declared raw
            // corruption input to reach the fractional assembly boundary.
            for source in [3, 4] {
                w.corruption(source, 0.5);
            }
        } else {
            w.item_amount("helmet", 40.0);
        }
        let r = evaluate(&w);
        match &r.outcome {
            SupportEffectsOutcome::Evaluated { effects } => {
                let levels: Vec<_> = effects
                    .effects
                    .iter()
                    .filter(|e| {
                        e.key.invocation.program == key("pain-offering-final-inputs")
                            && e.key.effect == key("project-final-level")
                    })
                    .collect();
                assert_eq!(levels.len(), 2);
                assert!(
                    levels.iter().all(|e| e.value == EffectValue::Inactive),
                    "{levels:?}"
                );
                assert!(
                    !effects
                        .effects
                        .iter()
                        .any(|e| e.key.invocation.program == key(TABLE_OBSERVATION)
                            && matches!(e.value, EffectValue::Known { .. }))
                );
            }
            SupportEffectsOutcome::Unavailable { cause, input } => {
                assert!(
                    matches!(
                        cause,
                        EffectValue::Inactive
                            | EffectValue::Unresolved {
                                reason: PlanGapReason::MissingInput,
                                ..
                            }
                    ),
                    "{cause:?}"
                );
                assert!(
                    matches!(input.as_deref(),Some(PlanValueKey::SkillParameter{parameter,..}) if *parameter==decode::<DeclaredSlot<ParameterSlotDefId>>(&w.bindings["target"]["final_level_parameter"])),
                    "{input:?}"
                );
            }
            other => panic!("final-domain refusal must identify actual final input: {other:?}"),
        }
    }
    for missing in [SNAPSHOT, "effective-amount", "pain-offering-final-inputs"] {
        let mut w = World::load();
        for o in &mut w.source.base.inner.owners {
            o.programs.members.retain(|p| p.id != key(missing));
        }
        w.item_programs.retain(|(_, p)| *p != key(missing));
        if missing == "pain-offering-final-inputs" {
            let error = w
                .checked_plan()
                .err()
                .expect("explicit assembly reference cannot disappear");
            assert!(error.contains("unknown source property program"), "{error}");
        } else {
            let plan = w
                .checked_plan()
                .expect("missing producer is a numerical obligation, not a malformed fixture");
            let r = plan.evaluate(&mut plan.new_scratch()).unwrap();
            match &r.outcome {
                SupportEffectsOutcome::Unavailable { cause, .. } => assert!(
                    matches!(
                        cause,
                        EffectValue::Unresolved {
                            reason: PlanGapReason::MissingProducer,
                            ..
                        }
                    ),
                    "{missing}: {cause:?}"
                ),
                SupportEffectsOutcome::Evaluated { effects } => {
                    let expected_read = key(if missing == SNAPSHOT {
                        "pre-amulet-percent"
                    } else {
                        "effective"
                    });
                    assert!(
                        effects.effects.iter().any(|e| e.key.invocation.program
                            == key("amulet-copy-minion-gem-level")
                            && e.value
                                == EffectValue::Unresolved {
                                    reason: PlanGapReason::MissingProducer,
                                    read: Some(expected_read.clone())
                                }),
                        "{missing}: actual copy dependency must remain unresolved"
                    );
                    assert!(!effects.effects.iter().any(|e| e.key.invocation.program
                        == key("pain-offering-final-inputs")
                        && e.key.effect == key("project-final-level")
                        && matches!(e.value, EffectValue::Known { .. })));
                }
                other => panic!("{missing}: unrelated preparation failure {other:?}"),
            }
        }
    }
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_OFFERING_INPUTS_RELEASE; finite native component"]
fn offering_inputs_preserve_actual_partial_owner_refusal() {
    let mut w = World::load();
    w.source
        .base
        .inner
        .owner_mut(w.actual_modifier.owner.clone())
        .programs
        .closure = w.actual_modifier.programs.closure.clone();
    let error = w
        .checked_plan()
        .err()
        .expect("actual Partial item family cannot be early-complete");
    assert!(error.contains("complete owner programs"), "{error}");
    let mut w = World::load();
    let owner = w.source.physical_owner.clone();
    w.source.base.inner.owner_mut(owner.owner).programs.closure = owner.programs.closure;
    let error = w
        .checked_plan()
        .err()
        .expect("actual Partial Offering owner remains unresolved");
    assert!(error.contains("complete owner programs"), "{error}");
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_OFFERING_INPUTS_RELEASE; finite native component"]
fn offering_inputs_require_explicit_local_readiness_and_early_item_dependencies() {
    let w = World::load();
    let old = w
        .checked_plan_variant(OWNED_EVALUATION_STAGES_V3, false)
        .err()
        .expect("historical V3 cannot authorize local item preparation writes");
    assert!(old.contains("preparation output role"), "{old}");
    let late = w
        .checked_plan_variant(OWNED_EVALUATION_STAGES_V4, true)
        .err()
        .expect("late item value cannot feed early source inputs");
    assert!(
        late.contains("stage") || late.contains("readiness") || late.contains("phase"),
        "{late}"
    );
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_OFFERING_INPUTS_RELEASE; finite native component"]
fn offering_inputs_reused_scratch_and_rayon_keep_exact_source_values() {
    let a = World::load();
    let mut b = a.clone();
    b.raw(3, 7, 13.0);
    b.snapshot(100.0);
    let ap = a.plan();
    let bp = b.plan();
    let mut scratch = ap.new_scratch();
    let first = ap.evaluate(&mut scratch).unwrap();
    check(&a, &first, [22, 22], [0.0, 0.0]);
    let middle = bp.evaluate(&mut scratch).unwrap();
    check(&b, &middle, [10, 23], [13.0, 0.0]);
    assert_eq!(first, ap.evaluate(&mut scratch).unwrap());
    let plan = Arc::new(ap);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let rows: Vec<_> = pool.install(|| {
        (0..12)
            .into_par_iter()
            .map(|_| plan.evaluate(&mut plan.new_scratch()).unwrap())
            .collect()
    });
    assert!(rows.iter().all(|r| r == &first));
}
