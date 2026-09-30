//! Computed support preparation keeps the complete generated target activation path.
#[allow(dead_code)]
#[path = "support/owned_computed_support_fixture.rs"]
mod fixture;
use fixture::*;
use poe_optimizer_core::{build_identity::SupportAssignmentId, owned_build::ParameterValue};
use poe_optimizer_engine::{owned_plan::*, owned_supports::*};
#[test]
fn generated_targets_keep_exact_parent_paths_slots_and_physical_origin_membership() {
    let f = generated_fixture();
    for (use_id, slot, winner) in [(30, "first", 61), (30, "second", 63), (31, "first", 65)] {
        let target = target(use_id, slot);
        let report = evaluate(&f, target.clone());
        assert!(report.gaps.is_empty(), "{:?}", report.gaps);
        let result = prepared(&report);
        assert_eq!(result.target, target);
        assert_eq!(
            result.ordered_origins,
            vec![
                occurrence::<SupportAssignmentId>(winner - 1),
                occurrence(winner)
            ]
        );
        assert_eq!(result.selected.len(), 1);
        assert_eq!(result.selected[0].assignment, occurrence(winner));
        assert!(result.selected[0].applicable);
    }
}

#[test]
fn generated_target_requires_final_entering_grant_and_all_required_inputs() {
    for mode in [
        "actor-false",
        "ability-false",
        "ability-missing",
        "input-missing",
    ] {
        let mut f = generated_fixture();
        match mode {
            "actor-false" => {
                f.build
                    .gems
                    .iter_mut()
                    .find(|gem| gem.id == occurrence(28))
                    .unwrap()
                    .parameters[0]
                    .value = ParameterValue::Boolean(false)
            }
            "ability-false" => {
                activation_program(&mut f, "first").nodes[2] = bool_node("enabled", false)
            }
            "ability-missing" => activation_program(&mut f, "first")
                .effects
                .retain(|effect| effect.id != key("activate")),
            "input-missing" => activation_program(&mut f, "first")
                .effects
                .retain(|effect| effect.id != key("quality")),
            _ => unreachable!(),
        }
        let report = evaluate(&f, target(30, "first"));
        if mode.ends_with("false") {
            assert!(
                matches!(
                    report.outcome,
                    ComputedSupportOutcome::Prepared {
                        result: SupportPreparationOutcome::Inactive { .. }
                    }
                ),
                "{mode}: {report:?}"
            );
        } else {
            assert!(
                matches!(
                    report.outcome,
                    ComputedSupportOutcome::Unavailable {
                        cause: EffectValue::Unresolved { .. },
                        ..
                    } | ComputedSupportOutcome::Prepared {
                        result: SupportPreparationOutcome::Unresolved { .. }
                    }
                ),
                "{mode}: {report:?}"
            );
        }
        if mode == "actor-false" {
            assert_eq!(
                prepared(&evaluate(&f, target(31, "first"))).selected[0].assignment,
                occurrence(65)
            );
        } else if mode != "ability-missing" {
            // The independently supplied sibling ability is not the first slot.
            assert_eq!(
                prepared(&evaluate(&f, target(30, "second"))).selected[0].assignment,
                occurrence(63)
            );
        } else {
            // Missing potential-supply authority retains the whole-plan gate,
            // even for a sibling whose own activation producer is present.
            assert!(matches!(
                evaluate(&f, target(30, "second")).outcome,
                ComputedSupportOutcome::Unavailable {
                    cause: EffectValue::Unresolved { .. },
                    ..
                }
            ));
        }
    }
}

#[test]
fn generated_preparation_reuses_scratch_across_targets_without_sibling_results() {
    let f = generated_fixture();
    let a = compile(&f, target(30, "first"));
    let b = compile(&f, target(31, "first"));
    let mut scratch = a.new_scratch();
    let first = a.evaluate(&mut scratch).unwrap();
    let sibling = b.evaluate(&mut scratch).unwrap();
    let repeated = a.evaluate(&mut scratch).unwrap();
    assert_eq!(first, repeated);
    assert_eq!(sibling, b.evaluate(&mut b.new_scratch()).unwrap());
    assert_ne!(
        prepared(&first).ordered_origins,
        prepared(&sibling).ordered_origins
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn generated_preparation_parallel_workers_keep_independent_attempts() {
    use rayon::prelude::*;
    let f = generated_fixture();
    let plans = [
        compile(&f, target(30, "first")),
        compile(&f, target(31, "first")),
    ];
    let expected: Vec<_> = plans
        .iter()
        .map(|plan| plan.evaluate(&mut plan.new_scratch()).unwrap())
        .collect();
    let results: Vec<_> = (0..16)
        .into_par_iter()
        .map(|index| {
            let plan = &plans[index % 2];
            plan.evaluate(&mut plan.new_scratch()).unwrap()
        })
        .collect();
    for (index, result) in results.into_iter().enumerate() {
        assert_eq!(result, expected[index % 2]);
    }
}
