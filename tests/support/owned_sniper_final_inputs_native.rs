//! Physical Sniper component checks over the shared actual-input fixture.
#[allow(dead_code)]
#[path = "owned_sniper_final_inputs_fixture.rs"]
mod fixture;
use fixture::*;
use poe_optimizer_core::{owned_build::*, owned_definitions::*};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_SNIPER_INPUTS_RELEASE"]
fn sniper_real_item_inputs_reach_population_and_restore() {
    let w = World::load();
    w.check(&w.evaluate(), [22, 22], [0., 0.]);
    for (helmet, amulet, expected) in [(true, false, 21), (false, true, 21), (true, true, 20)] {
        let mut changed = w.clone();
        if helmet {
            changed.base.remove_item("helmet");
        }
        if amulet {
            changed.base.remove_item("amulet");
        }
        changed.check(&changed.evaluate(), [expected, expected], [0., 0.]);
        w.check(&w.evaluate(), [22, 22], [0., 0.]);
    }
}
#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_SNIPER_INPUTS_RELEASE"]
fn sniper_independent_raw_inputs_and_actual_population_requirements() {
    let mut w = World::load();
    w.raw(0, 12, 13., 1.);
    w.raw(1, 5, 7., 0.);
    w.check(&w.evaluate(), [15, 7], [13., 7.]);
    w.base.source.base.inner.build.character.level = 1;
    let report = w.evaluate();
    let SupportEffectsOutcome::Evaluated { effects } = &report.outcome else {
        panic!("{report:?}")
    };
    assert!(effects.effects.iter().any(|e| e.key.invocation.program
        == key("ordinary-population-requirements")
        && matches!(
            &e.value,
            EffectValue::Known {
                value: ParameterValue::Boolean(false)
            }
        )));
}
#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_SNIPER_INPUTS_RELEASE"]
fn sniper_final_inputs_refuse_invalid_or_missing_preparation() {
    for (level, delta) in [(20, 0.5), (40, 0.)] {
        let mut w = World::load();
        w.raw(0, level, 0., delta);
        w.raw(1, level, 0., delta);
        let report = w.evaluate();
        match &report.outcome {
            SupportEffectsOutcome::Evaluated { effects } => {
                let rows: Vec<_> = effects
                    .effects
                    .iter()
                    .filter(|e| {
                        e.key.invocation.program == key(ASSEMBLY)
                            && e.key.effect == key("project-final-level")
                    })
                    .collect();
                assert_eq!(rows.len(), 2);
                assert!(rows.iter().all(|e| e.value == EffectValue::Inactive));
                assert!(!effects.effects.iter().any(|e| e.key.invocation.program
                    == key("ordinary-population-inputs")
                    && e.key.effect == key("project-actor-level")
                    && matches!(e.value, EffectValue::Known { .. })));
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
                    matches!(input.as_deref(),Some(PlanValueKey::SkillParameter{parameter,..})if *parameter==decode::<DeclaredSlot<ParameterSlotDefId>>(&w.bindings["target"]["final_level_parameter"])),
                    "{input:?}"
                );
            }
            other => panic!("{other:?}"),
        }
    }
    let mut w = World::load();
    w.base
        .source
        .base
        .inner
        .owner_mut(subject(gem()))
        .programs
        .members
        .retain(|p| p.id != key(ASSEMBLY));
    assert!(
        w.checked_plan()
            .err()
            .expect("missing assembly must refuse")
            .contains("unknown source property program")
    );
    let mut w = World::load();
    let baseline = w.evaluate();
    let inventory = w.base.source.base.inner.build.authored_support_order.take();
    assert!(inventory.is_some());
    let missing = w.evaluate();
    assert!(
        matches!(
            missing.outcome,
            SupportEffectsOutcome::PreparationUnresolved {
                reason: poe_optimizer_engine::owned_supports::SupportPreparationGap::OriginOrder,
                origin_index: None,
                ..
            }
        ),
        "missing outer inventory must not become a complete empty list: {missing:?}"
    );
    w.base.source.base.inner.build.authored_support_order = inventory;
    let restored = w.evaluate();
    assert_eq!(restored, baseline);
    w.check(&restored, [22, 22], [0., 0.]);
}
#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_SNIPER_INPUTS_RELEASE"]
fn sniper_real_partial_owners_and_missing_item_receivers_remain_unavailable() {
    for actual_gem in [true, false] {
        let mut w = World::load();
        let owner = if actual_gem {
            w.actual_gem.clone()
        } else {
            w.base.actual_modifier.clone()
        };
        let target = w.base.source.base.inner.owner_mut(owner.owner.clone());
        *target = owner;
        let error = w
            .checked_plan()
            .err()
            .expect("actual Partial preparation owner must refuse");
        assert!(error.contains("complete owner programs"), "{error}");
    }
    let mut w = World::load();
    w.receivers.members.retain(|r| r.stat != stat(0x32e4));
    let report = w.evaluate();
    match &report.outcome {
        SupportEffectsOutcome::Unavailable { cause, .. } => assert!(
            matches!(
                cause,
                EffectValue::Unresolved {
                    reason: PlanGapReason::MissingProducer,
                    ..
                }
            ),
            "{cause:?}"
        ),
        SupportEffectsOutcome::Evaluated { effects } => {
            assert!(
                effects.effects.iter().any(|e| e.key.invocation.program
                    == key("amulet-copy-minion-gem-level")
                    && e.value
                        == EffectValue::Unresolved {
                            reason: PlanGapReason::MissingProducer,
                            read: Some(key("pre-amulet-percent")),
                        }),
                "actual item-copy snapshot dependency must remain missing"
            );
            let levels: Vec<_> = effects
                .effects
                .iter()
                .filter(|e| {
                    e.key.invocation.program == key(ASSEMBLY)
                        && e.key.effect == key("project-final-level")
                })
                .collect();
            assert_eq!(levels.len(), 2);
            assert!(
                levels
                    .iter()
                    .all(|e| !matches!(e.value, EffectValue::Known { .. })),
                "{levels:?}"
            );
            let qualities: Vec<_> = effects
                .effects
                .iter()
                .filter(|e| {
                    e.key.invocation.program == key(ASSEMBLY)
                        && e.key.effect == key("project-final-quality")
                })
                .collect();
            assert_eq!(qualities.len(), 2);
            assert!(
                qualities.iter().all(|e| e.value
                    == EffectValue::Known {
                        value: quantity(0., &def("def.0000000000000002")),
                    }),
                "independent raw quality must survive missing level contributors"
            );
        }
        other => panic!("{other:?}"),
    }
}
#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_SNIPER_INPUTS_RELEASE"]
fn sniper_source_inputs_are_deterministic_across_scratch_and_parallel_workers() {
    let a = World::load();
    let mut b = a.clone();
    b.raw(0, 12, 13., 0.);
    b.base.remove_item("helmet");
    let pa = a.plan();
    let pb = b.plan();
    let mut scratch = pa.new_scratch();
    let first = pa.evaluate(&mut scratch).unwrap();
    a.check(&first, [22, 22], [0., 0.]);
    let changed = pb.evaluate(&mut scratch).unwrap();
    b.check(&changed, [13, 21], [13., 0.]);
    assert_eq!(first, pa.evaluate(&mut scratch).unwrap());
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let reports = pool.install(|| {
        (0..16)
            .into_par_iter()
            .map(|i| {
                if i % 2 == 0 {
                    pa.evaluate(&mut pa.new_scratch()).unwrap()
                } else {
                    pb.evaluate(&mut pb.new_scratch()).unwrap()
                }
            })
            .collect::<Vec<_>>()
    });
    for (i, r) in reports.iter().enumerate() {
        assert_eq!(r, if i % 2 == 0 { &first } else { &changed });
    }
}
