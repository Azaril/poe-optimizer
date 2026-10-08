//! Actual authored support programs over explicitly finite Djinn and physical
//! Ice occurrences. No assertion promotes the originals to complete builds.
#[allow(dead_code)]
#[path = "owned_bidding_delivery_fixture.rs"]
mod bidding_fixture;
#[path = "owned_magnified_area_fixture.rs"]
mod fixture;
use fixture::*;
use poe_optimizer_core::{
    build_identity::SupportAssignmentId, owned_build::*, owned_definitions::*, owned_rules::*,
};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use std::{collections::BTreeSet, sync::Arc};

fn evaluate(w: &World) -> SupportEffectsReport {
    let p = w.plan();
    p.evaluate(&mut p.new_scratch()).unwrap()
}
fn effects(r: &SupportEffectsReport) -> &OwnedEffectsReport {
    let SupportEffectsOutcome::Evaluated { effects } = &r.outcome else {
        panic!("finite component: {r:?}")
    };
    assert!(r.gaps.is_empty(), "{:?}", r.gaps);
    effects
}
fn application(e: &BoundEffectResult) -> Option<&SupportApplicationKey> {
    if let RuleOrigin::SupportApplication { application } = &e.key.invocation.origin {
        Some(application)
    } else {
        None
    }
}
fn contributions<'a>(
    r: &'a OwnedEffectsReport,
    a: &ActionSelection,
    stat: &StatDefId,
) -> Vec<&'a BoundEffectResult> {
    r.effects.iter().filter(|e|matches!(&e.target,BoundEffectTarget::Contribution{key} if key.entity==ConcreteEntity::Action(Box::new(a.clone())) && &key.stat==stat)).collect()
}
fn selected(w: &World, source: usize, effect: &str) -> SupportAssignmentId {
    let target = SkillTarget::Authored(id(20 + source as u64));
    let gem = w.inner.support_gem(effect);
    let assignments: Vec<_> = w
        .inner
        .build
        .supports
        .iter()
        .filter(|s| {
            s.target == target
                && w.inner
                    .build
                    .gems
                    .iter()
                    .any(|g| g.id == s.support && g.definition == gem)
        })
        .collect();
    assert_eq!(assignments.len(), 1);
    assignments[0].id
}
fn check_applicability(r: &OwnedEffectsReport, app: &SupportApplicationKey, admitted: bool) {
    let rows: Vec<_> = r.effects.iter().filter(|e| matches!(
        &e.target, BoundEffectTarget::Value { key: PlanValueKey::SupportApplicability { application } }
        if application.as_ref() == app
    )).collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].key.invocation.program,
        key("magnified-area-applicability")
    );
    assert_eq!(
        rows[0].value,
        EffectValue::Known {
            value: ParameterValue::Boolean(admitted)
        }
    );
}
fn check(
    w: &World,
    r: &OwnedEffectsReport,
    tiers: [&str; 3],
    winners: Option<[SupportAssignmentId; 3]>,
    area: Option<bool>,
) {
    assert!(r.gaps.is_empty(), "{:?}", r.gaps);
    let area_stat: StatDefId = decode(&w.bindings["channels"]["area"]);
    let cost_stat: StatDefId = decode(&w.bindings["channels"]["cost_factor"]);
    let damage: StatDefId = decode(&w.bindings["channels"]["damage_factor"]);
    let percent: UnitDefId = decode(&w.bindings["percent_unit"]);
    let ratio: UnitDefId = decode(&w.bindings["factor_unit"]);
    let winners = winners.unwrap_or_else(|| std::array::from_fn(|i| selected(w, i, tiers[i])));
    let mut applications = BTreeSet::new();
    for (source, tier) in tiers.into_iter().enumerate() {
        for action in w.actions(source) {
            let admitted = w.admitted(&action, tier);
            let area =
                area.unwrap_or_else(|| w.source_fact(&action)["area_eligible"].as_bool().unwrap());
            let a = contributions(r, &action, &area_stat);
            let c = contributions(r, &action, &cost_stat);
            let d = contributions(r, &action, &damage);
            assert_eq!(a.len(), 1, "one exact area contribution per recipient");
            assert_eq!(c.len(), 1, "one exact cost contribution per recipient");
            assert_eq!(d.len(), usize::from(tier == II));
            let app = application(a[0]).unwrap();
            assert_eq!(
                app.prepared.origin,
                SupportOrigin::Assignment(winners[source])
            );
            assert_eq!(
                app.prepared.target,
                SkillTarget::Authored(id(20 + source as u64))
            );
            assert_eq!(app.prepared.position, 0);
            assert_eq!(
                app.receiver,
                SupportReceiverKey::Action(Box::new(action.clone()))
            );
            assert!(applications.insert(app.clone()));
            check_applicability(r, app, admitted);
            for (rows, stat, kind, amount, unit) in [
                (
                    &a,
                    &area_stat,
                    ContributionKind::Increase,
                    if tier == I { 35.0 } else { 45.0 },
                    &percent,
                ),
                (&c, &cost_stat, ContributionKind::Multiply, 1.3, &ratio),
            ] {
                let row = rows[0];
                assert_eq!(application(row), Some(app));
                assert_eq!(row.key.invocation.program, key("magnified-area-and-cost"));
                let BoundEffectTarget::Contribution { key } = &row.target else {
                    unreachable!()
                };
                assert_eq!(&key.stat, stat);
                assert_eq!(key.kind, kind);
                assert_eq!(
                    row.value,
                    if admitted {
                        EffectValue::Known {
                            value: quantity(amount, unit),
                        }
                    } else {
                        EffectValue::Inactive
                    }
                );
            }
            if tier == II {
                assert_eq!(application(d[0]), Some(app));
                assert_eq!(d[0].key.invocation.program, key("magnified-area-damage"));
                assert_eq!(
                    d[0].value,
                    if admitted && area {
                        EffectValue::Known {
                            value: quantity(1.0, &ratio),
                        }
                    } else {
                        EffectValue::Inactive
                    }
                );
            }
        }
    }
    assert_eq!(
        applications.len(),
        (0..=w.ice_source)
            .map(|s| w.actions(s).len())
            .sum::<usize>()
    );
    for row in &r.effects {
        if let Some(app) = application(row) {
            assert!(applications.contains(app));
        }
    }
}

#[test]
#[ignore = "requires checked Magnified publication; finite component only"]
fn magnified_actual_tiers_reach_exact_player_and_minion_receivers() {
    let mut w = World::load();
    assert!(w.originals.iter().all(|o| !o.programs.is_complete()));
    let first = evaluate(&w);
    check(&w, effects(&first), [I; 3], None, Some(true));
    for source in 0..=w.ice_source {
        w.inner.change_support(source, II);
    }
    let second = evaluate(&w);
    check(&w, effects(&second), [II; 3], None, Some(true));
    w.set_area_fact(Some(false));
    check(&w, effects(&evaluate(&w)), [II; 3], None, Some(false));
    assert!(
        w.actions(0)
            .iter()
            .any(|a| a.action.actor == ActorKey::Player)
    );
    assert!(
        w.actions(0)
            .iter()
            .any(|a| matches!(a.action.actor, ActorKey::Owned(_)))
    );
    assert!(matches!(
        w.inner.build.skills[w.ice_source].source,
        AuthoredSkillSource::Gem(_)
    ));
}

#[test]
#[ignore = "requires checked Magnified publication; finite component only"]
fn magnified_removal_disable_and_same_skill_occurrences_are_independent() {
    let mut w = World::load();
    // Two independent authored Sand roots share definitions, never occurrences.
    w.inner.sources[1] = 0;
    w.inner.build.skills[1].source = w.inner.build.skills[0].source.clone();
    w.inner.build.skills[1].parameters = w.inner.build.skills[0].parameters.clone();
    w.inner.change_support(1, II);
    check(&w, effects(&evaluate(&w)), [I, II, I], None, Some(true));
    let target = SkillTarget::Authored(id(20));
    let first = w
        .inner
        .build
        .supports
        .iter()
        .find(|s| s.target == target)
        .unwrap()
        .id;
    let mut disabled = w.clone();
    disabled
        .inner
        .build
        .supports
        .iter_mut()
        .find(|s| s.id == first)
        .unwrap()
        .enabled = false;
    let result = evaluate(&disabled);
    let r = effects(&result);
    assert!(
        !r.effects
            .iter()
            .any(|e| application(e).is_some_and(|a| a.prepared.target == target))
    );
    let other = SkillTarget::Authored(id(21));
    assert!(
        r.effects
            .iter()
            .any(|e| application(e).is_some_and(|a| a.prepared.target == other))
    );
    let mut removed = w;
    let gem = removed
        .inner
        .build
        .supports
        .iter()
        .find(|s| s.id == first)
        .unwrap()
        .support;
    removed.inner.build.supports.retain(|s| s.id != first);
    removed.inner.build.gems.retain(|g| g.id != gem);
    removed
        .inner
        .build
        .authored_support_order
        .as_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r.target == target)
        .unwrap()
        .assignments
        .clear();
    let r = evaluate(&removed);
    assert!(
        !effects(&r)
            .effects
            .iter()
            .any(|e| application(e).is_some_and(|a| a.prepared.target == target))
    );
}

#[test]
#[ignore = "requires checked Magnified publication; finite component only"]
fn magnified_duplicate_family_selection_retains_exact_physical_winner() {
    let mut w = World::load();
    let first = selected(&w, 0, I);
    w.inner.add_support(0, w.inner.support_index(I));
    let duplicate = w.inner.build.supports.last().unwrap().id;
    let gem = w.inner.build.supports.last().unwrap().support;
    for (quality, winner) in [(0.0, first), (15.0, duplicate)] {
        w.inner
            .build
            .gems
            .iter_mut()
            .find(|g| g.id == gem)
            .unwrap()
            .quality
            .as_mut()
            .unwrap()
            .amount = FiniteQuantity::new(quality, w.inner.quality_unit.clone()).unwrap();
        let r = evaluate(&w);
        check(
            &w,
            effects(&r),
            [I; 3],
            Some([winner, selected(&w, 1, I), selected(&w, 2, I)]),
            Some(true),
        );
    }
    for (first, last) in [(I, II), (II, I)] {
        let mut tiers = World::load();
        tiers.inner.change_support(0, first);
        tiers.inner.add_support(0, tiers.inner.support_index(last));
        let second = tiers.inner.build.supports.last().unwrap().id;
        check(
            &tiers,
            effects(&evaluate(&tiers)),
            [last, I, I],
            Some([second, selected(&tiers, 1, I), selected(&tiers, 2, I)]),
            Some(true),
        );
    }
}

#[test]
#[ignore = "requires checked Magnified publication; finite component only"]
fn magnified_missing_facts_partial_owners_and_disabled_roots_refuse_closure() {
    let mut missing = World::load();
    missing.inner.change_support(0, II);
    missing.set_area_fact(None);
    let r = evaluate(&missing);
    let r = effects(&r);
    assert!(
        r.gaps.is_empty(),
        "only the demanded Area fact may be missing"
    );
    let eligible: StatDefId = decode(&missing.bindings["channels"]["area_eligible"]);
    let damage: StatDefId = decode(&missing.bindings["channels"]["damage_factor"]);
    let owner = missing
        .originals
        .iter()
        .find(|o| o.owner == subject(missing.inner.support_gem(II)))
        .unwrap();
    let program = owner
        .programs
        .members
        .iter()
        .find(|p| p.id == key("magnified-area-damage"))
        .unwrap();
    assert!(program.reads.iter().any(|r| r.id == key("area-eligible")
        && r.source
            == RuleReadSource::Stat {
                entity: RuleEntity::Current,
                stat: eligible.clone()
            }));
    for source in 0..=missing.ice_source {
        for action in missing.actions(source) {
            let tier = if source == 0 { II } else { I };
            let admitted = missing.admitted(&action, tier);
            for (channel, amount, unit) in [
                (
                    "area",
                    if source == 0 { 45.0 } else { 35.0 },
                    "percent_unit",
                ),
                ("cost_factor", 1.3, "factor_unit"),
            ] {
                let rows =
                    contributions(r, &action, &decode(&missing.bindings["channels"][channel]));
                assert_eq!(rows.len(), 1);
                check_applicability(r, application(rows[0]).unwrap(), admitted);
                assert_eq!(
                    rows[0].value,
                    if admitted {
                        EffectValue::Known {
                            value: quantity(amount, &decode(&missing.bindings[unit])),
                        }
                    } else {
                        EffectValue::Inactive
                    }
                );
            }
            let rows = contributions(r, &action, &damage);
            assert_eq!(rows.len(), usize::from(source == 0));
            if source == 0 {
                assert_eq!(
                    rows[0].value,
                    if admitted {
                        EffectValue::Unresolved {
                            reason: PlanGapReason::MissingProducer,
                            read: Some(key("area-eligible")),
                        }
                    } else {
                        EffectValue::Inactive
                    }
                );
                assert_eq!(
                    application(rows[0]).unwrap().prepared.origin,
                    SupportOrigin::Assignment(selected(&missing, source, II))
                );
            }
        }
    }
    let mut disabled = World::load();
    disabled.inner.build.skills[0].enabled = false;
    assert!(!matches!(
        evaluate(&disabled).outcome,
        SupportEffectsOutcome::Evaluated { .. }
    ));
    let mut partial = World::load();
    let gem = partial.inner.support_gem(I);
    let closure = partial
        .originals
        .iter()
        .find(|o| o.owner == subject(gem.clone()))
        .unwrap()
        .programs
        .closure
        .clone();
    partial.inner.owner_mut(subject(gem)).programs.closure = closure;
    assert!(partial.checked_plan().is_err());
    let mut origins = World::load();
    origins.inner.build.authored_support_order = None;
    if let Ok(p) = origins.checked_plan() {
        assert!(!matches!(
            p.evaluate(&mut p.new_scratch()).unwrap().outcome,
            SupportEffectsOutcome::Evaluated { .. }
        ));
    }
    let mut receiving = World::load();
    receiving.inner.receiving = receiving.original_receiving.clone();
    if let Ok(p) = receiving.checked_plan() {
        assert!(!matches!(
            p.evaluate(&mut p.new_scratch()).unwrap().outcome,
            SupportEffectsOutcome::Evaluated { .. }
        ));
    }
}

#[test]
#[ignore = "requires checked Magnified publication; finite component only"]
fn magnified_scratch_and_rayon_preserve_exact_contribution_provenance() {
    let a = World::load();
    let mut b = a.clone();
    b.inner.change_support(0, II);
    let pa = Arc::new(a.plan());
    let pb = b.plan();
    let mut scratch = pa.new_scratch();
    let first = pa.evaluate(&mut scratch).unwrap();
    check(&a, effects(&first), [I; 3], None, Some(true));
    check(
        &b,
        effects(&pb.evaluate(&mut scratch).unwrap()),
        [II, I, I],
        None,
        Some(true),
    );
    assert_eq!(first, pa.evaluate(&mut scratch).unwrap());
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let results = pool.install(|| {
        (0..12)
            .into_par_iter()
            .map_init(|| pa.new_scratch(), |s, _| pa.evaluate(s).unwrap())
            .collect::<Vec<_>>()
    });
    assert!(results.iter().all(|r| r == &first));
}

#[test]
#[ignore = "requires checked Magnified publication; finite component only"]
fn magnified_observed_action_matrix_keeps_admission_and_area_distinct() {
    let mut w = World::load();
    w.set_source_area_matrix();
    // These are source-observed fixture inputs, not production intrinsic facts.
    // The exact Action target keeps two stat sets of one output independent.
    assert_eq!(w.area_usage.len(), 14);
    let actions: Vec<_> = (0..=w.ice_source).flat_map(|s| w.actions(s)).collect();
    let mut identities = BTreeSet::new();
    for action in &actions {
        let row = w.source_fact(action);
        assert!(identities.insert((
            row["source_effect"].as_str().unwrap(),
            row["source_stat_set_index"].as_u64().unwrap()
        )));
        assert!(!row["evidence"].as_array().unwrap().is_empty());
    }
    assert_eq!(identities.len(), 14);
    let knife: Vec<_> = actions
        .iter()
        .filter(|a| w.source_fact(a)["source_effect"] == "KnifeThrowSandDjinn")
        .collect();
    assert_eq!(knife.len(), 2);
    assert_eq!(knife[0].action, knife[1].action);
    assert_ne!(knife[0].stat_set, knife[1].stat_set);
    let observed: BTreeSet<_> = knife
        .iter()
        .map(|a| w.source_fact(a)["area_eligible"].as_bool().unwrap())
        .collect();
    assert_eq!(observed, BTreeSet::from([false, true]));
    assert_eq!(actions.iter().filter(|a| !w.admitted(a, I)).count(), 2);
    assert_eq!(actions.iter().filter(|a| !w.admitted(a, II)).count(), 2);
    assert!(
        actions
            .iter()
            .filter(|a| !w.admitted(a, II))
            .all(|a| a.action.actor == ActorKey::Player)
    );
    for tier in [I, II] {
        for source in 0..=w.ice_source {
            w.inner.change_support(source, tier);
        }
        check(&w, effects(&evaluate(&w)), [tier; 3], None, None);
    }
}
