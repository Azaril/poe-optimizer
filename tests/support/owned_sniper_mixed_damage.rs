//! Actual published inherited + stacked application consumer, joined to item
//! preparation and exact minion recipients. No final damage-domain closure.
use super::*;
use poe_optimizer_core::owned_stages::{EvaluationStage, FrozenStageChannel, StageChannel};
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry, owned_release::StagedOwnedRelease,
};

const STAGE: &str = "mixed-minion-damage";
pub(super) fn install(w: &mut sniper::World, endpoint: &StagedOwnedRelease) {
    mixed_damage_family::assert_component(endpoint);
    let m = mixed_damage_family::migration();
    let c = mixed_damage_family::consumer();
    let inner = &mut w.base.source.base.inner;
    assert!(
        RuleOperationsVersion::parse(inner.operations.as_str())
            .unwrap()
            .supports_application_group_contributions()
    );
    for row in m.schema {
        let SchemaExtensionEntry::Definition(d) = row else {
            panic!()
        };
        if let Some(old) = inner
            .schema
            .definitions
            .iter()
            .find(|old| old.address() == d.address())
        {
            assert_eq!(old, &d);
        } else {
            inner.schema.definitions.push(d.clone());
        }
        inner.owner_mut(SchemaSubject::Definition(d.address()));
    }
    for owner in c.owners {
        let existing = inner.owner_mut(owner.owner.clone());
        assert!(existing.programs.members.is_empty() && existing.programs.is_complete());
        *existing = owner;
    }
    for r in c.receivers {
        assert!(!w.receivers.members.iter().any(|old| old.id == r.id));
        w.receivers.members.push(r);
    }
    for q in mixed_damage_family::queries() {
        assert!(
            !w.base
                .contribution_queries
                .members
                .iter()
                .any(|old| old.id == q.id)
        );
        w.base.contribution_queries.members.push(q);
    }
}
pub(super) fn configure(s: &mut EvaluationStagesInput) {
    s.stages.push(EvaluationStage {
        id: key(STAGE),
        predecessors: vec![
            key("passive-damage-receive"),
            key("offering-application"),
            key("gigantic-benefits"),
        ],
    });
    for row in &mut s.programs.members {
        if [
            mixed_damage_family::PROGRAM,
            mixed_damage_family::FACTOR,
            mixed_damage_family::MORE,
        ]
        .contains(&row.program.as_str())
        {
            row.stage = key(STAGE);
        }
    }
    s.frozen_channels.extend([
        FrozenStageChannel {
            channel: StageChannel::Stat {
                scope: RuleEntityKind::Actor,
                stat: d(0x1d),
            },
            stage: key("population"),
        },
        FrozenStageChannel {
            channel: StageChannel::Contributions {
                scope: RuleEntityKind::Actor,
                stat: d(0x330b),
                contribution: ContributionKind::Multiply,
            },
            stage: key("gigantic-benefits"),
        },
        FrozenStageChannel {
            channel: StageChannel::Stat {
                scope: RuleEntityKind::Actor,
                stat: d(0x1d34),
            },
            stage: key("passive-damage-receive"),
        },
        FrozenStageChannel {
            channel: StageChannel::Contributions {
                scope: RuleEntityKind::Actor,
                stat: d(0x322d),
                contribution: ContributionKind::Add,
            },
            stage: key("offering-application"),
        },
    ]);
}
fn expected(case: &str) -> (f64, f64, f64) {
    let v = mixed_damage_family::vectors()
        .into_iter()
        .find(|v| v["case"] == case)
        .unwrap();
    (
        v["records"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["value"].as_f64().unwrap())
            .sum(),
        v["increased_factor"].as_f64().unwrap(),
        v["more_factor"].as_f64().unwrap(),
    )
}
fn check(w: &World, r: &SupportEffectsReport, case: &str) {
    check_cases(w, r, [case, case]);
}
fn check_cases(w: &World, r: &SupportEffectsReport, cases: [&str; 2]) {
    assert!(r.gaps.is_empty(), "{:?}", r.gaps);
    let e = sniper::offering::effects(r);
    assert!(e.gaps.is_empty(), "{:?}", e.gaps);
    let rows: Vec<_> = e
        .effects
        .iter()
        .filter(|e| {
            [
                mixed_damage_family::PROGRAM,
                mixed_damage_family::FACTOR,
                mixed_damage_family::MORE,
            ]
            .contains(&e.key.invocation.program.as_str())
        })
        .collect();
    assert_eq!(rows.len(), 6, "three outputs per exact recipient");
    for (i, case) in cases.into_iter().enumerate() {
        let (increase, factor, more) = expected(case);
        assert_eq!(
            w.value(e, i, false, 0x3353),
            &EffectValue::Known {
                value: quantity(increase, &d(2))
            }
        );
        assert_eq!(
            w.value(e, i, false, 0x3354),
            &EffectValue::Known {
                value: quantity(factor, &d(1))
            }
        );
        let recipient = w.sniper.actor(i);
        assert_eq!(
            w.value(e, i, false, 0x330b),
            &EffectValue::Known {
                value: quantity(more, &d(1))
            }
        );
        assert_eq!(rows.iter().filter(|r|matches!(&r.key.invocation.origin,RuleOrigin::Receiver {actor,..} if *actor==recipient)).count(),3);
    }
}
fn unresolved(w: &World, r: &SupportEffectsReport) {
    let e = sniper::offering::effects(r);
    for i in 0..2 {
        for stat in [0x3353, 0x3354] {
            assert!(matches!(
                w.value(e, i, false, stat),
                EffectValue::Unresolved { .. }
            ));
        }
    }
}
#[test]
#[ignore = "requires current mixed-damage release and retained source observations"]
fn mixed_damage_matches_actual_source_consumers_after_non_stacking() {
    mixed_damage_family::check_source(true);
    let a = World::load();
    check(&a, &a.evaluate(), "original-05");
    check(&a, &a.evaluate(), "repeat-original-05");
    check(&a, &a.evaluate(), "offering-duplicate-equal");
    let repaired = passive_damage_native::repaired(&a);
    check(&repaired, &repaired.evaluate(), "without-plain-node");
    for (levels, name) in [
        ([1, 1], "offering-level-1"),
        ([30, 20], "offering-higher-first"),
        ([20, 30], "offering-higher-last"),
    ] {
        let mut b = a.clone();
        for (i, l) in levels.into_iter().enumerate() {
            b.sniper.base.raw(3 + i, l, 0.);
        }
        check(&b, &b.evaluate(), name);
    }
    let mut disabled = a.clone();
    for i in 0..2 {
        offering_application_native::override_active(&mut disabled, i, false);
    }
    check(&disabled, &disabled.evaluate(), "offering-disabled");
    let mut quality = a.clone();
    quality.sniper.raw(0, 20, 1., 0.);
    quality.sniper.raw(1, 20, 20., 0.);
    check_cases(
        &quality,
        &quality.evaluate(),
        ["sniper-quality-1", "sniper-quality-20"],
    );
    let mut removed = a.clone();
    gigantic_native::remove(&mut removed);
    removed.sniper.raw(1, 20, 20., 0.);
    check_cases(
        &removed,
        &removed.evaluate(),
        ["without-gigantic", "quality-20-without-gigantic"],
    );
}
#[test]
#[ignore = "requires current mixed-damage release; exact producer failures"]
fn mixed_damage_does_not_replace_missing_inherited_or_application_inputs_with_zero() {
    let mut w = World::load();
    w.offering.preferences.remove(0);
    unresolved(&w, &w.evaluate());
    let mut missing = World::load();
    missing
        .sniper
        .base
        .source
        .base
        .inner
        .owner_mut(subject(d::<StatDefinition>(0x1d34)))
        .programs
        .members
        .clear();
    // A receiver pointing at the now-missing program is rejected during planning.
    assert!(missing.checked_plan().is_err());
    let mut inherited = World::load();
    inherited
        .sniper
        .receivers
        .members
        .retain(|r| r.id != key("sniper-received-owner-minion-damage"));
    unresolved(&inherited, &inherited.evaluate());
    let mut absent = World::load();
    absent.sniper.base.effect_applications.members.clear();
    assert!(
        absent.checked_plan().is_err(),
        "declared group cannot disappear from data"
    );
}
#[test]
#[ignore = "requires current mixed-damage release; membership and stage gates"]
fn mixed_damage_rejects_membership_holes_and_premature_reads() {
    for query in [mixed_damage_family::QUERY, mixed_damage_family::MORE_QUERY] {
        let mut w = World::load();
        let q = w
            .sniper
            .base
            .contribution_queries
            .members
            .iter_mut()
            .find(|q| q.id == key(query))
            .unwrap();
        q.groups[0].members.members.clear();
        assert!(w.checked_plan().is_err());
    }
    let w = World::load();
    for program in [mixed_damage_family::PROGRAM, mixed_damage_family::MORE] {
        assert!(
            w.checked_plan_configured(|s| {
                s.programs
                    .members
                    .iter_mut()
                    .find(|r| r.program == key(program))
                    .unwrap()
                    .stage = key("facts");
            })
            .is_err()
        );
    }
}

fn missing_quality(w: &mut World) {
    w.sniper
        .base
        .source
        .base
        .inner
        .owner_mut(subject(d::<SkillDefinition>(0x12)))
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == key("ordinary-population-inputs"))
        .unwrap()
        .effects
        .retain(|e| e.id != key("project-quality-factor"));
}
fn more_unresolved(w: &World, report: &SupportEffectsReport) {
    for i in 0..2 {
        assert!(matches!(
            w.value(sniper::offering::effects(report), i, false, 0x330b),
            EffectValue::Unresolved { .. }
        ));
    }
}
#[test]
#[ignore = "requires current mixed-damage release; MORE unknown and coverage gates"]
fn damage_more_requires_quality_status_and_complete_membership() {
    let a = World::load();
    let mut quality = a.clone();
    missing_quality(&mut quality);
    more_unresolved(&quality, &quality.evaluate());
    let mut status = a.clone();
    status
        .sniper
        .receivers
        .members
        .retain(|r| r.stat != d(0x3308));
    more_unresolved(&status, &status.evaluate());
    let mut partial = a.clone();
    partial
        .sniper
        .base
        .contribution_queries
        .members
        .iter_mut()
        .find(|q| q.id == key(mixed_damage_family::MORE_QUERY))
        .unwrap()
        .groups[0]
        .members
        .closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: subject(d::<StatDefinition>(0x330b)),
            facet: SchemaFacet::GameRules,
            code: key("counterfactual-unreviewed-damage-more"),
        }],
    };
    let p = partial.plan();
    assert!(
        p.gaps()
            .iter()
            .any(|g| g.reason == PlanGapReason::IncompleteContributors)
    );
    let r = p.evaluate(&mut p.new_scratch()).unwrap();
    assert_eq!(
        r.outcome,
        SupportEffectsOutcome::Unavailable {
            cause: EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                read: None
            },
            input: None,
        }
    );
}

#[test]
#[ignore = "requires current mixed-damage release; independent quality and scratch reuse"]
fn damage_more_preserves_recipient_quality_and_unknowns_across_workers() {
    let a = World::load();
    let mut b = a.clone();
    b.sniper.raw(0, 20, 1., 0.);
    b.sniper.raw(1, 20, 20., 0.);
    let mut unknown = a.clone();
    missing_quality(&mut unknown);
    let (pa, pb, pu) = (a.plan(), b.plan(), unknown.plan());
    let mut scratch = pa.new_scratch();
    let ra = pa.evaluate(&mut scratch).unwrap();
    let ru = pu.evaluate(&mut scratch).unwrap();
    more_unresolved(&unknown, &ru);
    let rb = pb.evaluate(&mut scratch).unwrap();
    check_cases(&b, &rb, ["sniper-quality-1", "sniper-quality-20"]);
    assert!(
        pa.evaluate(&mut scratch).unwrap() == ra,
        "A after unknown and different qualities"
    );
    assert!(
        pb.evaluate(&mut pb.new_scratch()).unwrap() == rb,
        "fresh B agrees"
    );
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let results = pool.install(|| {
        (0..12)
            .into_par_iter()
            .map_init(
                || pa.new_scratch(),
                |s, i| match i % 3 {
                    0 => pa.evaluate(s).unwrap(),
                    1 => pu.evaluate(s).unwrap(),
                    _ => pb.evaluate(s).unwrap(),
                },
            )
            .collect::<Vec<_>>()
    });
    for (i, r) in results.iter().enumerate() {
        assert!(
            r == match i % 3 {
                0 => &ra,
                1 => &ru,
                _ => &rb,
            },
            "parallel report {i}"
        );
    }
}
#[test]
#[ignore = "requires current mixed-damage release; storage and parallel scratch"]
fn mixed_damage_reused_and_parallel_workers_preserve_unknowns_and_known_results() {
    let a = World::load();
    let mut b = a.clone();
    let mut unknown = a.clone();
    for i in 0..2 {
        offering_application_native::override_active(&mut b, i, false);
    }
    unknown.offering.preferences.remove(0);
    let (pa, pb, pu) = (a.plan(), b.plan(), unknown.plan());
    let mut scratch = pa.new_scratch();
    let ra = pa.evaluate(&mut scratch).unwrap();
    let ru = pu.evaluate(&mut scratch).unwrap();
    unresolved(&unknown, &ru);
    let rb = pb.evaluate(&mut scratch).unwrap();
    check(&b, &rb, "offering-disabled");
    assert_eq!(pa.evaluate(&mut scratch).unwrap(), ra);
    let mut shuffled = a.clone();
    shuffled.offering.preferences.reverse();
    shuffled
        .sniper
        .base
        .source
        .base
        .inner
        .build
        .allocations
        .reverse();
    assert!(
        shuffled.evaluate() == ra,
        "build storage permutations preserve the entire report"
    );
    // Authored query registry order is currently committed in package identity.
    // A distinct content identity must still produce identical values, origins,
    // application candidates and gaps; this is not a repeated-run exemption.
    shuffled.sniper.base.contribution_queries.members.reverse();
    let reordered = shuffled.evaluate();
    assert_ne!(reordered.identity, ra.identity);
    assert_eq!(reordered.gaps, ra.gaps);
    let (actual, expected) = (
        sniper::offering::effects(&reordered),
        sniper::offering::effects(&ra),
    );
    assert_ne!(actual.identity, expected.identity);
    assert_eq!(actual.gaps, expected.gaps);
    assert!(
        actual.effects == expected.effects,
        "exact effects survive authored registry permutation"
    );
    assert!(
        actual.values == expected.values,
        "exact values survive authored registry permutation"
    );
    assert!(
        actual.application_groups == expected.application_groups,
        "exact group evidence survives authored registry permutation"
    );
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let results = pool.install(|| {
        (0..12)
            .into_par_iter()
            .map_init(
                || pa.new_scratch(),
                |s, i| match i % 3 {
                    0 => pa.evaluate(s).unwrap(),
                    1 => pu.evaluate(s).unwrap(),
                    _ => pb.evaluate(s).unwrap(),
                },
            )
            .collect::<Vec<_>>()
    });
    for (i, r) in results.into_iter().enumerate() {
        assert_eq!(
            r,
            *match i % 3 {
                0 => &ra,
                1 => &ru,
                _ => &rb,
            }
        );
    }
}
