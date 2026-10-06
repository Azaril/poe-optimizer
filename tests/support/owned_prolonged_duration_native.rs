//! Actual published physical Offering and Ice paths with finite component
//! boundaries. Admission inputs and final Offering inputs are test-owned;
//! published support bodies and grant/projection bodies remain unchanged.
//! These tests prove contribution factors, not final duration/cost or buff uptime.
#[allow(dead_code)]
#[path = "owned_prolonged_duration_fixture.rs"]
mod physical;
use crate::release;
use physical::fixture::{decode, def, id, key, quantity, subject};
use physical::{FINAL_BOUNDARY, OFFERINGS, SOURCES, World, set_fact};
use poe_optimizer_core::{
    build_identity::SupportAssignmentId, owned_build::*, owned_definitions::*, owned_rules::*,
};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use std::{collections::BTreeSet, sync::Arc};
const I: &str = "ProlongedDurationSupportPlayer";
const II: &str = "ProlongedDurationSupportPlayerTwo";

fn evaluate(w: &World) -> SupportEffectsReport {
    let plan = w.plan();
    plan.evaluate(&mut plan.new_scratch()).unwrap()
}
fn effects(report: &SupportEffectsReport) -> &OwnedEffectsReport {
    let SupportEffectsOutcome::Evaluated { effects } = &report.outcome else {
        panic!("finite component: {report:?}")
    };
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    assert!(effects.gaps.is_empty(), "{:?}", effects.gaps);
    effects
}
fn expected(w: &World, tier: &'static str) -> [Option<(&'static str, SupportAssignmentId)>; 3] {
    SOURCES.map(|s| Some((tier, w.selected(s))))
}
fn check(
    w: &World,
    report: &SupportEffectsReport,
    expected: [Option<(&str, SupportAssignmentId)>; 3],
) {
    check_admission(w, report, expected, [true; 3]);
}
fn check_admission(
    w: &World,
    report: &SupportEffectsReport,
    expected: [Option<(&str, SupportAssignmentId)>; 3],
    admitted: [bool; 3],
) {
    let report = effects(report);
    let unit: UnitDefId = decode(&w.bindings["factor_unit"]);
    let duration: StatDefId = decode(&w.bindings["channels"]["duration_factor"]);
    let cost: StatDefId = decode(&w.bindings["channels"]["cost_factor"]);
    let mut actual_applications = BTreeSet::new();
    for ((source, expected), admitted) in SOURCES.into_iter().zip(expected).zip(admitted) {
        let actions = w.actions(source);
        assert_eq!(actions.len(), if source == 2 { 2 } else { 1 });
        for action in actions {
            assert_eq!(action.action.actor, ActorKey::Player);
            assert_eq!(
                action.action.provider.grant_path,
                vec![decode(&w.target(source)["entering_grant"])]
            );
            let entity = ConcreteEntity::Action(Box::new(action.clone()));
            let rows: Vec<_> = report
                .effects
                .iter()
                .filter(|e| {
                    matches!(&e.target,
                BoundEffectTarget::Contribution { key } if key.entity == entity)
                })
                .collect();
            assert_eq!(rows.len(), if expected.is_some() { 2 } else { 0 });
            let Some((tier, winner)) = expected else {
                continue;
            };
            let support = w.bindings["supports"]
                .as_array()
                .unwrap()
                .iter()
                .find(|s| s["source_effect"] == tier)
                .unwrap();
            for (channel, value) in [
                (
                    &duration,
                    if tier == I {
                        1.3
                    } else {
                        assert_eq!(tier, II);
                        1.35
                    },
                ),
                (&cost, 1.2),
            ] {
                let joined: Vec<_> = rows.iter().filter(|e| matches!(&e.target, BoundEffectTarget::Contribution { key } if &key.stat == channel)).collect();
                assert_eq!(joined.len(), 1);
                let row = joined[0];
                let RuleOrigin::SupportApplication { application } = &row.key.invocation.origin
                else {
                    panic!("exact support provenance")
                };
                assert_eq!(
                    application.prepared.origin,
                    SupportOrigin::Assignment(winner)
                );
                assert_eq!(
                    application.prepared.target,
                    SkillTarget::Authored(id(20 + source as u64))
                );
                assert_eq!(application.prepared.position, 0);
                assert_eq!(
                    application.receiver,
                    SupportReceiverKey::Action(Box::new(action.clone()))
                );
                actual_applications.insert(application.clone());
                assert_eq!(
                    row.key.invocation.program,
                    key(support["programs"]["delivery"].as_str().unwrap())
                );
                assert!(
                    matches!(&row.target, BoundEffectTarget::Contribution { key } if key.kind == ContributionKind::Multiply)
                );
                assert_eq!(
                    row.value,
                    if admitted {
                        EffectValue::Known {
                            value: quantity(value, &unit),
                        }
                    } else {
                        EffectValue::Inactive
                    }
                );
            }
            let admissions: Vec<_> = report
                .effects
                .iter()
                .filter(|e| {
                    matches!(&e.target,
                BoundEffectTarget::Value { key: PlanValueKey::SupportApplicability { application } }
                    if application.prepared.origin == SupportOrigin::Assignment(winner)
                    && application.receiver == SupportReceiverKey::Action(Box::new(action.clone())))
                })
                .collect();
            assert_eq!(admissions.len(), 1);
            assert_eq!(
                admissions[0].key.invocation.program,
                key(support["programs"]["applicability"].as_str().unwrap())
            );
            assert_eq!(
                admissions[0].value,
                EffectValue::Known {
                    value: ParameterValue::Boolean(admitted)
                }
            );
        }
    }
    for row in &report.effects {
        if let BoundEffectTarget::Contribution { key } = &row.target {
            assert!([&duration, &cost].contains(&&key.stat));
            assert!(
                matches!(&key.entity, ConcreteEntity::Action(a) if SOURCES.into_iter().flat_map(|s| w.actions(s)).any(|expected| expected == **a)),
                "no Player, minion or unrelated receiver leak"
            );
        }
        if let RuleOrigin::SupportApplication { application } = &row.key.invocation.origin {
            assert!(actual_applications.contains(application));
        }
    }
    // The actual prefix's outputs are separately checked. The selector still
    // uses the existing explicitly finite preparation input boundary above.
    for assignment in w.base.inner.build.supports.iter().filter(|s| s.enabled) {
        let gem = w
            .base
            .inner
            .build
            .gems
            .iter()
            .find(|g| g.id == assignment.support)
            .unwrap();
        let binding = w.bindings["supports"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| decode::<GemDefId>(&s["gem"]) == gem.definition)
            .unwrap();
        let program = key(binding["programs"]["prepared_inputs"].as_str().unwrap());
        for (stat, value) in [
            (
                def::<StatDefinition>("def.00000000000030ae"),
                ParameterValue::Integer(BoundedInteger::new(1).unwrap()),
            ),
            (
                def::<StatDefinition>("def.00000000000030af"),
                ParameterValue::Quantity(gem.quality.as_ref().unwrap().amount.clone()),
            ),
        ] {
            let rows: Vec<_> = report.effects.iter().filter(|e| e.key.invocation.program == program && matches!(&e.target,
                BoundEffectTarget::Value { key: PlanValueKey::Stat { entity: ConcreteEntity::SupportOrigin(origin), stat: actual } }
                    if *origin == SupportOrigin::Assignment(assignment.id) && *actual == stat)).collect();
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].value, EffectValue::Known { value });
        }
    }
    for source in OFFERINGS {
        let root = ProviderKey {
            root: ProviderRoot::SkillUse(id(20 + source as u64)),
            grant_path: vec![],
        };
        let target = &w.bindings["target"];
        let generated = GeneratedSkillKey {
            provider: root,
            slot: decode(&target["primary_supply"]),
        };
        let quality_unit = &w.base.inner.quality_unit;
        for (field, expected) in [
            (
                "final_level_parameter",
                ParameterValue::Integer(BoundedInteger::new(22).unwrap()),
            ),
            ("final_quality_parameter", quantity(0.0, quality_unit)),
        ] {
            let value = PlanValueKey::SkillParameter {
                skill: Box::new(generated.clone()),
                parameter: decode(&target[field]),
            };
            let rows: Vec<_> = report
                .effects
                .iter()
                .filter(|e| matches!(&e.target, BoundEffectTarget::Value { key } if *key == value))
                .collect();
            assert_eq!(rows.len(), 1);
            assert_eq!(
                rows[0].key.invocation.program.as_str(),
                target["primary_supply_program"].as_str().unwrap()
            );
            assert_eq!(rows[0].value, EffectValue::Known { value: expected });
        }
    }
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_PROLONGED_RELEASE; finite component only"]
fn prolonged_real_player_offering_and_ice_contrast_deliver_both_tiers() {
    let mut w = World::load();
    assert!(w.originals.iter().all(|o| !o.programs.is_complete()));
    assert_ne!(
        w.base.inner.build.skills[1].source,
        w.base.inner.build.skills[2].source
    );
    let offering: GemDefId = decode(&w.bindings["target"]["physical_gem"]);
    let raw: Vec<_> = w
        .base
        .inner
        .build
        .gems
        .iter()
        .filter(|g| g.definition == offering)
        .collect();
    assert_eq!(raw.len(), 2);
    assert!(
        raw.iter().all(|g| g.level == 20),
        "raw physical levels differ from the explicit final22 boundary"
    );
    check(&w, &evaluate(&w), expected(&w, I));
    for source in SOURCES {
        w.base.inner.change_support(source, II);
    }
    check(&w, &evaluate(&w), expected(&w, II));
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_PROLONGED_RELEASE; finite component only"]
fn prolonged_removal_and_disable_do_not_affect_other_roots() {
    for source in SOURCES {
        for remove in [false, true] {
            let mut w = World::load();
            let mut wanted = expected(&w, I);
            let origin = wanted[source - 2].take().unwrap().1;
            if remove {
                let gem = w
                    .base
                    .inner
                    .build
                    .supports
                    .iter()
                    .find(|s| s.id == origin)
                    .unwrap()
                    .support;
                w.base.inner.build.supports.retain(|s| s.id != origin);
                w.base.inner.build.gems.retain(|g| g.id != gem);
                for row in w.base.inner.build.support_origins.as_mut().unwrap() {
                    row.origins
                        .retain(|o| *o != SupportOrigin::Assignment(origin));
                }
            } else {
                w.base
                    .inner
                    .build
                    .supports
                    .iter_mut()
                    .find(|s| s.id == origin)
                    .unwrap()
                    .enabled = false;
            }
            check(&w, &evaluate(&w), wanted);
        }
    }
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_PROLONGED_RELEASE; finite component only"]
fn prolonged_duplicate_quality_and_family_order_keep_one_exact_origin() {
    let mut w = World::load();
    let original = w.selected(3);
    let index = w.base.inner.support_index(I);
    w.base.inner.add_support(3, index);
    let duplicate = w.base.inner.build.supports.last().unwrap().clone();
    for (quality, winner) in [(0.0, original), (15.0, duplicate.id)] {
        w.base
            .inner
            .build
            .gems
            .iter_mut()
            .find(|g| g.id == duplicate.support)
            .unwrap()
            .quality
            .as_mut()
            .unwrap()
            .amount = FiniteQuantity::new(quality, w.base.inner.quality_unit.clone()).unwrap();
        check(
            &w,
            &evaluate(&w),
            [
                Some((I, w.selected(2))),
                Some((I, winner)),
                Some((I, w.selected(4))),
            ],
        );
    }
    let index = w.base.inner.support_index(II);
    w.base.inner.add_support(3, index);
    let last = w.base.inner.build.supports.last().unwrap().id;
    check(
        &w,
        &evaluate(&w),
        [
            Some((I, w.selected(2))),
            Some((II, last)),
            Some((I, w.selected(4))),
        ],
    );
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_PROLONGED_RELEASE; finite component only"]
fn prolonged_actual_partial_owners_and_receiving_remain_blocked() {
    let baseline = World::load();
    for owner in baseline
        .originals
        .iter()
        .chain(std::iter::once(&baseline.physical_owner))
    {
        let mut w = baseline.clone();
        if let Some(support) = w.bindings["supports"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| subject(decode::<GemDefId>(&s["gem"])) == owner.owner)
        {
            let tier = support["source_effect"].as_str().unwrap().to_owned();
            w.base.inner.change_support(3, &tier);
        }
        w.base.inner.owner_mut(owner.owner.clone()).programs.closure =
            owner.programs.closure.clone();
        assert!(
            w.checked_plan().is_err(),
            "actual Partial owner cannot become complete"
        );
    }
    let mut w = baseline;
    w.base.inner.receiving = w.receiving.clone();
    if let Ok(p) = w.checked_plan() {
        assert!(!matches!(
            p.evaluate(&mut p.new_scratch()).unwrap().outcome,
            SupportEffectsOutcome::Evaluated { .. }
        ));
    }
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_PROLONGED_RELEASE; finite component only"]
fn prolonged_duration_admission_checks_assigned_and_generated_skill_separately() {
    for assigned in [true, false] {
        let mut w = World::load();
        for target in &w.receiving.targets {
            for role in &target.roles.members {
                for endpoint in &role.endpoints.members {
                    assert!(matches!(endpoint.admission(),
                        poe_optimizer_core::owned_support_receiving::SupportAdmissionContext::ReceivingSkill {
                            summoner_path: None
                        }));
                }
            }
        }
        let owner = if assigned {
            subject(decode::<GemDefId>(&w.bindings["target"]["physical_gem"]))
        } else {
            subject(decode::<SkillDefId>(&w.bindings["target"]["primary_skill"]))
        };
        let facts = w
            .base
            .inner
            .owner_mut(owner)
            .programs
            .members
            .iter_mut()
            .find(|p| p.id == key("fixture.initial-facts"))
            .unwrap();
        set_fact(facts, "type.type.duration", false);
        let report = evaluate(&w);
        for source in OFFERINGS {
            let target = if assigned {
                SkillTarget::Authored(id(20 + source as u64))
            } else {
                SkillTarget::Generated(Box::new(GeneratedSkillKey {
                    provider: ProviderKey {
                        root: ProviderRoot::SkillUse(id(20 + source as u64)),
                        grant_path: vec![],
                    },
                    slot: decode(&w.bindings["target"]["primary_supply"]),
                }))
            };
            let fact = PlanValueKey::Stat {
                entity: ConcreteEntity::Skill(Box::new(target)),
                stat: def("fixture.type.type.duration"),
            };
            let rows: Vec<_> = effects(&report)
                .effects
                .iter()
                .filter(|e| {
                    e.key.invocation.program == key("fixture.initial-facts")
                        && matches!(&e.target, BoundEffectTarget::Value { key } if *key == fact)
                })
                .collect();
            assert_eq!(rows.len(), 1);
            assert_eq!(
                rows[0].value,
                EffectValue::Known {
                    value: ParameterValue::Boolean(false)
                }
            );
        }
        // ReceivingSkill with no summoner uses the generated effect's own
        // preparation. Its physical input owner still owns the selected support
        // position, but that owner's type fact is not a second admission gate.
        // A rejected receiver keeps its exact applications as false/Inactive
        // diagnostics; removal of an assignment is the separate zero-row case.
        check_admission(&w, &report, expected(&w, I), [true, assigned, assigned]);
    }
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_PROLONGED_RELEASE; finite component only"]
fn prolonged_missing_final_input_boundary_cannot_default_to_ready() {
    let mut w = World::load();
    w.base
        .inner
        .owner_mut(w.physical_owner.owner.clone())
        .programs
        .members
        .retain(|p| p.id != key(FINAL_BOUNDARY));
    let report = evaluate(&w);
    match &report.outcome {
        SupportEffectsOutcome::Unavailable { cause, .. } => assert!(matches!(
            cause,
            EffectValue::Unresolved {
                reason: PlanGapReason::MissingProducer,
                ..
            }
        )),
        SupportEffectsOutcome::Evaluated { effects } => {
            let missing: Vec<_> = effects
                .effects
                .iter()
                .filter(|e| {
                    e.key.invocation.program.as_str()
                        == w.bindings["target"]["primary_supply_program"]
                            .as_str()
                            .unwrap()
                        && matches!(
                            &e.target,
                            BoundEffectTarget::Value {
                                key: PlanValueKey::SkillParameter { .. }
                            }
                        )
                })
                .collect();
            assert_eq!(missing.len(), 4);
            assert!(missing.iter().all(|e| matches!(
                e.value,
                EffectValue::Unresolved {
                    reason: PlanGapReason::MissingProducer,
                    ..
                }
            )));
            assert!(!effects.effects.iter().any(|e| matches!(&e.target, BoundEffectTarget::Contribution { key }
                if matches!(&key.entity, ConcreteEntity::Action(a) if OFFERINGS.into_iter().flat_map(|s| w.actions(s)).any(|expected| expected == **a)))
                && matches!(e.value, EffectValue::Known { .. })));
        }
        other => panic!("missing final input must retain its cause: {other:?}"),
    }
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_PROLONGED_RELEASE; finite component only"]
fn prolonged_scratch_a_b_a_and_private_rayon_preserve_exact_receivers() {
    let a = World::load();
    let mut b = a.clone();
    b.base.inner.change_support(3, II);
    let pa = Arc::new(a.plan());
    let pb = b.plan();
    let mut scratch = pa.new_scratch();
    let first = pa.evaluate(&mut scratch).unwrap();
    check(&a, &first, expected(&a, I));
    check(
        &b,
        &pb.evaluate(&mut scratch).unwrap(),
        [
            Some((I, b.selected(2))),
            Some((II, b.selected(3))),
            Some((I, b.selected(4))),
        ],
    );
    assert_eq!(first, pa.evaluate(&mut scratch).unwrap());
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let reports = pool.install(|| {
        (0..12)
            .into_par_iter()
            .map_init(|| pa.new_scratch(), |s, _| pa.evaluate(s).unwrap())
            .collect::<Vec<_>>()
    });
    assert!(reports.iter().all(|r| r == &first));
}
