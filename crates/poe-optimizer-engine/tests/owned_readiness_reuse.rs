//! Resource replay and worker isolation through the public readiness metric plan.
#[allow(dead_code)]
#[path = "support/owned_preparation_readiness_fixture.rs"]
mod support;
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use std::sync::Arc;
use support::*;

fn plans() -> [Metrics; 2] {
    let original = fixture();
    let mut changed = fixture();
    changed
        .build
        .gems
        .iter_mut()
        .find(|gem| gem.id == occurrence(28))
        .unwrap()
        .level = 13;
    [
        metrics(compile(&original).unwrap()),
        metrics(compile(&changed).unwrap()),
    ]
}

fn measure(plan: &Metrics, scratch: &mut OwnedPlanScratch) -> (OwnedSupportMetricReport, usize) {
    let allowance = PlanLimits::default().max_work;
    let mut remaining = allowance;
    let report = plan.evaluate_with_budget(scratch, &mut remaining).unwrap();
    let used = allowance - remaining;
    assert!(used > 1 && used < allowance);
    (report, used)
}

#[test]
fn readiness_replays_exact_attempt_work_and_uses_one_decreasing_caller_budget() {
    let plan = metrics(compile(&fixture()).unwrap());
    let (expected, used) = measure(&plan, &mut plan.new_scratch());
    assert_eq!(numbers(&expected), [40.0, 40.0, 44.0]);
    let mut scratch = plan.new_scratch();

    // Public APIs expose total attempt work, not a prefix execution counter.
    // Exact replay includes graph preparation, admission, assembly and metrics;
    // neither a warm scratch nor the suffix can reset the caller's allowance.
    let mut exact = used;
    let replay = plan.evaluate_with_budget(&mut scratch, &mut exact).unwrap();
    assert!(
        replay == expected,
        "exact-work replay changed the full report"
    );
    assert_eq!(exact, 0);
    let mut shared = used.checked_mul(2).unwrap();
    for expected_remaining in [used, 0] {
        let report = plan
            .evaluate_with_budget(&mut scratch, &mut shared)
            .unwrap();
        assert!(report == expected, "warm attempt changed the full report");
        assert_eq!(shared, expected_remaining);
    }
    let mut one_short = used - 1;
    assert!(matches!(
        plan.evaluate_with_budget(&mut scratch, &mut one_short),
        Err(PlanError::Limit("work"))
    ));
    assert_eq!(one_short, 0);
    let (recovered, recovered_used) = measure(&plan, &mut scratch);
    assert!(
        recovered == expected,
        "exhausted attempt contaminated the retry"
    );
    assert_eq!(recovered_used, used);
}

#[test]
fn changed_physical_root_inputs_survive_a_b_a_scratch_reuse_without_stale_values() {
    let [a, b] = plans();
    assert_ne!(a.identity(), b.identity());
    assert_ne!(a.effect_plan().identity(), b.effect_plan().identity());
    let expected_a = measure(&a, &mut a.new_scratch());
    let expected_b = measure(&b, &mut b.new_scratch());
    assert_eq!(numbers(&expected_a.0), [40.0, 40.0, 44.0]);
    assert_eq!(numbers(&expected_b.0), [46.0, 46.0, 44.0]);
    let mut scratch = a.new_scratch();
    for (plan, expected) in [(&a, &expected_a), (&b, &expected_b), (&a, &expected_a)] {
        let actual = measure(plan, &mut scratch);
        assert!(
            actual == *expected,
            "A/B/A report or resource use differs from fresh evaluation"
        );
    }
}

#[test]
fn unresolved_preparation_and_exhausted_attempts_clear_reused_scratch() {
    let [a, b] = plans();
    let mut missing = fixture();
    remove_projection(
        &mut missing,
        actor_owner(),
        "prepare-first",
        "preparation-level",
    );
    let unavailable = metrics(compile(&missing).unwrap());
    let expected_a = measure(&a, &mut a.new_scratch());
    let expected_b = measure(&b, &mut b.new_scratch());
    let expected_unavailable = measure(&unavailable, &mut unavailable.new_scratch());
    assert!(!matches!(
        expected_unavailable.0.support,
        SupportMetricStatus::Evaluated
    ));
    assert!(
        expected_unavailable
            .0
            .evaluation
            .results
            .iter()
            .all(|row| !matches!(row.value, EffectValue::Known { .. }))
    );
    let mut scratch = a.new_scratch();
    for (plan, expected) in [
        (&a, &expected_a),
        (&unavailable, &expected_unavailable),
        (&b, &expected_b),
    ] {
        let actual = measure(plan, &mut scratch);
        assert!(
            actual == *expected,
            "unavailable attempt leaked or retained stale values"
        );
    }
    for mut budget in [0, expected_a.1 - 1] {
        assert!(matches!(
            a.evaluate_with_budget(&mut scratch, &mut budget),
            Err(PlanError::Limit("work"))
        ));
        assert_eq!(budget, 0);
        let recovered = measure(&b, &mut scratch);
        assert!(
            recovered == expected_b,
            "early/late work failure contaminated a different plan"
        );
    }
    assert!(
        measure(&a, &mut scratch) == expected_a,
        "final A retry differs from fresh execution"
    );
}

#[test]
fn rayon_workers_reuse_private_readiness_scratch_with_deterministic_reports_and_work() {
    let plans = Arc::new(plans());
    let expected = [
        measure(&plans[0], &mut plans[0].new_scratch()),
        measure(&plans[1], &mut plans[1].new_scratch()),
    ];
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let batches: Vec<_> = pool.install(|| {
        (0..8)
            .into_par_iter()
            .map_init(
                || plans[0].new_scratch(),
                |scratch, batch| {
                    // Several A/B/A cycles per job ensure actual scratch reuse even
                    // if Rayon partitions the small outer iterator into singletons.
                    (0..12)
                        .map(|step| {
                            let selected = (batch + step) % 2;
                            (selected, measure(&plans[selected], scratch))
                        })
                        .collect::<Vec<_>>()
                },
            )
            .collect()
    });
    assert_eq!(batches.len(), 8);
    for batch in batches {
        assert_eq!(batch.len(), 12);
        for (selected, actual) in batch {
            assert!(
                actual == expected[selected],
                "parallel reuse changed a report or its work consumption"
            );
        }
    }
}
