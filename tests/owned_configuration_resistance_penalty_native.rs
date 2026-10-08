//! Data-authored Player resistance-penalty consumer, with no source runtime.
#[allow(dead_code)]
#[path = "support/owned_empty_support_domain.rs"]
mod empty_support;
#[path = "support/owned_configuration_resistance_penalty_native.rs"]
mod native;
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;
use native::*;
use poe_optimizer_core::owned_build::AssumptionTarget;
use poe_optimizer_engine::owned_plan::{EffectValue, PlanGapReason, SupportEffectsOutcome};
use rayon::prelude::*;
use std::path::PathBuf;

#[test]
fn authored_penalty_preserves_current_owners_and_uses_exact_player_channels() {
    authored();
    assert_eq!(
        World::new(None, false).recipe.rules.schema_version,
        poe_optimizer_core::owned_rules::OWNED_RULE_PACKAGE_VERSION
    );
}
#[test]
fn native_channels_match_every_fresh_source_control_and_fixed_rebuild() {
    fresh_control_parity();
}
#[test]
fn native_penalty_retains_zero_signed_fractional_and_out_of_list_values() {
    for value in [-1_000_000.0, -60.0, -30.0, -12.5, 0.0, 10.0, 1_000_000.0] {
        for rewards in [false, true] {
            let plan = World::new(Some(value), rewards).plan().unwrap();
            assert_penalty(
                &plan.evaluate(&mut plan.new_scratch()).unwrap(),
                value,
                rewards,
            );
        }
    }
}
#[test]
fn missing_player_penalty_cannot_be_filled_from_enemy_or_from_a_native_default() {
    for wrong_target in [false, true] {
        let mut w = World::new(wrong_target.then_some(-60.0), false);
        if wrong_target {
            w.scenario.assumptions[0].target = AssumptionTarget::Enemy;
            assert!(w.plan().is_err());
            continue;
        }
        let plan = w.plan().unwrap();
        let r = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert_eq!(
            effects(&r)
                .effects
                .iter()
                .filter(
                    |e| e.key.invocation.program.as_str() == "configured-player-resistance-penalty"
                )
                .count(),
            3
        );
        assert!(
            effects(&r)
                .effects
                .iter()
                .filter(
                    |e| e.key.invocation.program.as_str() == "configured-player-resistance-penalty"
                )
                .all(|e| matches!(
                    e.value,
                    EffectValue::Unresolved {
                        reason: PlanGapReason::MissingInput,
                        ..
                    }
                ))
        );
    }
}
#[test]
fn real_partial_owner_still_prevents_complete_preparation() {
    let mut w = World::new(Some(-60.0), true);
    w.actual_partial_actor();
    let p = w.plan().unwrap();
    let r = p.evaluate(&mut p.new_scratch()).unwrap();
    assert!(
        r.gaps
            .iter()
            .any(|g| g.reason == PlanGapReason::PartialPrograms)
    );
    assert!(!matches!(
        r.outcome,
        SupportEffectsOutcome::Evaluated { .. }
    ));
}
#[test]
fn penalty_and_rewards_are_deterministic_with_reused_scratch_and_rayon() {
    let a = World::new(Some(-60.0), true).plan().unwrap();
    let b = World::new(Some(0.0), true).plan().unwrap();
    let mut scratch = a.new_scratch();
    let first = a.evaluate(&mut scratch).unwrap();
    assert_penalty(&first, -60.0, true);
    assert_penalty(&b.evaluate(&mut scratch).unwrap(), 0.0, true);
    assert_eq!(a.evaluate(&mut scratch).unwrap(), first);
    let results: Vec<_> = (0..16)
        .into_par_iter()
        .map_init(
            || a.new_scratch(),
            |scratch, _| a.evaluate(scratch).unwrap(),
        )
        .collect();
    assert!(results.iter().all(|r| r == &first));
}
#[test]
#[ignore = "requires retained configuration reports, source checkout, and CONFIGURATION_PENALTY_RELEASE"]
fn published_penalty_uses_exact_authored_component_and_authenticated_source() {
    authenticated_source();
    endpoint(&PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_CONFIGURATION_PENALTY_RELEASE")
            .expect("published package"),
    ));
}
