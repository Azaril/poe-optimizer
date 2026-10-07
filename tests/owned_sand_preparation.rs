//! Real authored preparation arithmetic; explicitly finite source inventories.
#[allow(dead_code)]
#[path = "support/owned_sand_preparation_fixture.rs"]
mod fixture;
use fixture::*;
use poe_optimizer_core::{owned_build::*, owned_rules::*};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;

fn known(report: &OwnedEffectsReport, key: PlanValueKey, expected: ParameterValue) {
    assert_eq!(
        value(report, &key),
        Some(&EffectValue::Known { value: expected }),
        "{key:?}"
    );
}
fn check(report: &OwnedEffectsReport, target: SkillTarget, level: i64, quality: f64) {
    known(
        report,
        stat_key(target.clone(), 0x334e),
        ParameterValue::Integer(integer(level)),
    );
    known(
        report,
        stat_key(target.clone(), 0x334f),
        quantity(quality, 2),
    );
    known(
        report,
        command_parameter(&target, 0x3350),
        ParameterValue::Integer(integer(level)),
    );
    known(
        report,
        command_parameter(&target, 0x3351),
        quantity(quality, 2),
    );
    known(
        report,
        actor_level(&target),
        ParameterValue::Integer(integer(level * 2)),
    );
    known(
        report,
        stat_key(target, 0x32e0),
        ParameterValue::Integer(integer(0)),
    );
}
#[test]
fn manual_and_tree_share_rules_with_independent_raw_inputs_and_exact_descendants() {
    let w = World::new(20., 12.5, 17.25, 2.);
    let plan = w.compile().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let effects = evaluated(&report);
    assert!(report.gaps.is_empty(), "{report:?}");
    check(effects, manual(), 22, 12.5);
    check(effects, tree(), 3, 17.25);
    let assemblies: Vec<_> = effects
        .effects
        .iter()
        .filter(|e| e.key.invocation.program == key("sand-final-preparation"))
        .collect();
    assert_eq!(
        assemblies.len(),
        4,
        "two writes per exact source, not once per effect"
    );
    for e in assemblies {
        assert!(matches!(
            &e.key.invocation.origin,
            RuleOrigin::SourceProperty { .. }
        ));
    }
}
#[test]
fn final_cap_follows_ordinary_properties_and_quality_is_finite_transport() {
    for (raw, q, treeq, ordinary, level, treelevel) in [
        (40., -1., 101.5, 2., 40, 3),
        (1., 12.5, -1., 2., 3, 3),
        (20., 101.5, 12.5, 11., 31, 12),
    ] {
        let w = World::new(raw, q, treeq, ordinary);
        let plan = w.compile().unwrap();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        let effects = evaluated(&report);
        check(effects, manual(), level, q);
        check(effects, tree(), treelevel, treeq);
        known(
            effects,
            stat_key(manual(), 0x30ac),
            quantity(raw + ordinary, 0x295a),
        );
    }
}
#[test]
fn raw_domain_recovery_is_not_copied_and_bonuses_do_not_rescue_invalid_raw_level() {
    for raw in [0., -1., 1.5, 41.] {
        let w = World::new(raw, 0., 0., 2.);
        let plan = w.compile().unwrap();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        let effects = evaluated(&report);
        assert!(
            !matches!(
                value(effects, &stat_key(manual(), 0x334e)),
                Some(EffectValue::Known { .. })
            ),
            "raw {raw} recovered"
        );
        known(effects, stat_key(manual(), 0x334f), quantity(0., 2));
    }
}
#[test]
fn fresh_reused_and_parallel_scratch_give_identical_results() {
    let w = World::new(20., 12.5, 0., 2.);
    let plan = w.compile().unwrap();
    let expected = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(expected.gaps.is_empty(), "{expected:?}");
    check(evaluated(&expected), manual(), 22, 12.5);
    check(evaluated(&expected), tree(), 3, 0.);
    let mut scratch = plan.new_scratch();
    for _ in 0..3 {
        assert_eq!(plan.evaluate(&mut scratch).unwrap(), expected);
    }
    let actual: Vec<_> = (0..8)
        .into_par_iter()
        .map(|_| plan.evaluate(&mut plan.new_scratch()).unwrap())
        .collect();
    assert!(actual.iter().all(|r| r == &expected));
}
#[test]
fn authoring_draft_does_not_claim_closed_source_or_owner_coverage() {
    let relations: poe_optimizer_core::owned_source_properties::SourcePropertyPreparationInput =
        read("source-properties.json");
    assert!(!relations.relations.is_complete());
    assert_eq!(relations.relations.members.len(), 2);
    for r in relations.relations.members {
        assert!(!r.effects.is_complete());
        assert!(!r.channels.is_complete());
        assert!(!r.external.is_complete());
        assert!(!r.supports.is_complete());
        assert!(r.assembly.is_complete());
    }
    let extension: poe_optimizer_import::owned_recipe_extension::OwnedRecipeExtension =
        read("extension.json");
    assert!(extension.owners.iter().all(|o| !o.programs.is_complete()));
    assert!(
        extension
            .owners
            .iter()
            .flat_map(|o| &o.programs.members)
            .flat_map(|p| &p.reads)
            .all(|r| !matches!(
                r.source,
                RuleReadSource::GemLevel | RuleReadSource::GemQualityAmount { .. }
            ))
    );
}

#[test]
fn population_projection_reads_injected_table_and_other_source_is_independent() {
    for (table_value, expected) in [(77, 77), (200, 100), (-10, 1)] {
        let mut w = World::new(20., 12.5, 0., 2.);
        w.recipe
            .rules
            .tables
            .iter_mut()
            .find(|t| t.id == key("sniper.actor-level"))
            .unwrap()
            .rows[21] = ParameterValue::Integer(integer(table_value));
        let plan = w.compile().unwrap();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        let effects = evaluated(&report);
        known(
            effects,
            actor_level(&manual()),
            ParameterValue::Integer(integer(expected)),
        );
        check(effects, tree(), 3, 0.);
        known(
            effects,
            command_parameter(&manual(), 0x3350),
            ParameterValue::Integer(integer(22)),
        );
    }
}

#[test]
fn missing_prepared_dimension_does_not_invent_missingness_for_other_dimension() {
    for missing in ["level", "quality"] {
        let mut w = World::new(20., 12.5, 0., 2.);
        w.recipe
            .rules
            .owners
            .iter_mut()
            .flat_map(|o| &mut o.programs.members)
            .find(|p| p.id == key("sand-ordinary-preparation"))
            .unwrap()
            .effects
            .retain(|e| e.id != key(missing));
        let plan = w.compile().unwrap();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        let effects = evaluated(&report);
        let (absent, present, expected) = if missing == "level" {
            (0x334e, 0x334f, quantity(12.5, 2))
        } else {
            (0x334f, 0x334e, ParameterValue::Integer(integer(22)))
        };
        known(effects, stat_key(manual(), present), expected);
        assert!(
            matches!(
                value(effects, &stat_key(manual(), absent)),
                Some(EffectValue::Unresolved { .. })
            ),
            "{report:?}"
        );
    }
}

#[test]
fn partial_source_inventory_is_rejected_instead_of_becoming_empty() {
    let mut w = World::new(20., 0., 0., 2.);
    let draft: poe_optimizer_core::owned_source_properties::SourcePropertyPreparationInput =
        read("source-properties.json");
    w.relations.relations.members[0].channels.closure =
        draft.relations.members[0].channels.closure.clone();
    let error = w
        .compile()
        .err()
        .expect("Partial channels must reject checked sidecar");
    assert!(error.to_lowercase().contains("complete"), "{error}");
}

#[test]
fn zero_physical_supports_do_not_drop_external_properties_or_cap_too_early() {
    let mut w = World::new(40., -1., 101.5, 2.);
    w.add_supported_properties(-3., 1.25);
    let plan = w.compile().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let effects = evaluated(&report);
    // 40 + ordinary 2 - supported 3 is 39, not 37. Tree 1 + 2 - 3 caps to 1.
    check(effects, manual(), 39, 0.25);
    check(effects, tree(), 1, 102.75);
    assert_eq!(
        effects
            .effects
            .iter()
            .filter(|e| e.key.invocation.program == key("finite-source-properties"))
            .count(),
        4,
        "two channels once per source, not once per summon/Command effect"
    );
}
