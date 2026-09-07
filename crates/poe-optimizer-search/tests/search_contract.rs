use poe_optimizer_core::{metrics::*, objective::*};
use poe_optimizer_search::*;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::time::Duration;

fn query(id: &str) -> MetricQuery {
    MetricQuery {
        actor: ActorScope::Player,
        id: id.into(),
    }
}
fn value(id: &str, n: f64) -> MetricMeasurement {
    MetricMeasurement {
        query: query(id),
        unit: MetricUnit::Damage,
        value: MeasurementValue::from_number(n),
        schema_version: 1,
    }
}
fn policy(strict: bool) -> CompiledObjective {
    ObjectiveSpec {
        schema_version: 1,
        objective: ObjectivePolicy::Scalar {
            metric: query("score"),
            unit: MetricUnit::Damage,
            direction: Direction::Maximize,
        },
        constraints: vec![MetricConstraint {
            id: "floor".into(),
            metric: query("floor"),
            unit: MetricUnit::Damage,
            operator: if strict {
                Comparison::GreaterThan
            } else {
                Comparison::AtLeast
            },
            threshold: 0.,
            violation_scale: 1.,
        }],
    }
    .compile(&["score", "floor"].map(|id| MetricDefinition {
        id: id.into(),
        unit: MetricUnit::Damage,
        actors: vec![ActorScope::Player],
        description: String::new(),
        schema_version: 1,
    }))
    .unwrap()
}
struct Domain;
impl SearchDomain<u8> for Domain {
    fn validate(&self, candidate: &u8, _: &EvaluationControl<'_>) -> Result<(), String> {
        if *candidate < 64 {
            Ok(())
        } else {
            Err("Outside six binary dimensions".into())
        }
    }
    fn propose(
        &self,
        parents: &[u8],
        round: usize,
        _seed: u64,
        limit: usize,
        _: &EvaluationControl<'_>,
    ) -> Result<Vec<u8>, String> {
        let mut candidates = Vec::new();
        for parent in parents {
            for first in 0..6 {
                candidates.push(parent ^ (1 << first));
                // A varying neighborhood crosses a two-choice interaction trap.
                if round.is_multiple_of(2) {
                    for second in (first + 1)..6 {
                        candidates.push(parent ^ (1 << first) ^ (1 << second));
                    }
                }
            }
        }
        candidates.truncate(limit);
        Ok(candidates)
    }
}
struct Evaluator<F> {
    kind: ExecutionKind,
    f: F,
}
impl<F: Fn(u8) -> Result<(f64, f64), String> + Sync> CandidateEvaluator<u8> for Evaluator<F> {
    fn execution_kind(&self) -> ExecutionKind {
        self.kind
    }
    fn evaluate(
        &self,
        candidate: &u8,
        _: &EvaluationControl<'_>,
    ) -> Result<CandidateMeasurements, String> {
        let (score, floor) = (self.f)(*candidate)?;
        Ok(CandidateMeasurements {
            measurements: vec![value("score", score), value("floor", floor)],
            diagnostic_only: true,
        })
    }
}
fn budget() -> SearchBudget {
    SearchBudget {
        max_evaluations: 100,
        max_proposals: 10_000,
        max_rounds: 12,
        duration: Duration::from_secs(10),
        jobs: 4,
        beam_per_status: 2,
        archive_size: 4,
        verification_attempts: 1,
        seed: 42,
    }
}
fn landscape(c: u8) -> Result<(f64, f64), String> {
    // Four dimensions monotonically improve. The last two must move together.
    let pair = match c & 3 {
        0 => 10.,
        3 => 20.,
        _ => 0.,
    };
    Ok((pair + (c >> 2).count_ones() as f64, 1.))
}

#[test]
fn coordinated_search_matches_exhaustive_six_dimension_optimum() {
    let evaluator = Evaluator {
        kind: ExecutionKind::RustCpu,
        f: landscape,
    };
    let cancel = AtomicBool::new(false);
    let exact = search(
        &Domain,
        &evaluator,
        &policy(false),
        SearchPlan::Finite((0..64).collect()),
        &budget(),
        &cancel,
    )
    .unwrap();
    let guided = search(
        &Domain,
        &evaluator,
        &policy(false),
        SearchPlan::Explore(vec![0]),
        &budget(),
        &cancel,
    )
    .unwrap();
    assert_eq!(exact.termination, Termination::FiniteDomainProcessed);
    assert_eq!(exact.feasible[0].candidate, 63);
    assert_eq!(guided.feasible[0].candidate, exact.feasible[0].candidate);
    assert!(guided.statistics.duplicates > 0);
    assert!(guided.statistics.evaluations <= 65);
    assert!(guided.verifications[0].consistent);
    assert!(guided.verifications[0].diagnostic_only);
}

#[test]
fn deterministic_across_job_counts_and_execution_kinds() {
    let mut baseline = None;
    for kind in [ExecutionKind::RustCpu, ExecutionKind::ExternalProcess] {
        for jobs in [1, 2, 4] {
            let b = SearchBudget { jobs, ..budget() };
            let result = search(
                &Domain,
                &Evaluator { kind, f: landscape },
                &policy(false),
                SearchPlan::Explore(vec![0]),
                &b,
                &AtomicBool::new(false),
            )
            .unwrap();
            let signature = (
                result
                    .feasible
                    .iter()
                    .map(|c| c.candidate)
                    .collect::<Vec<_>>(),
                result.statistics.evaluations,
                result.statistics.proposals,
            );
            if let Some(expected) = &baseline {
                assert_eq!(&signature, expected);
            } else {
                baseline = Some(signature);
            }
        }
    }
}

#[test]
fn strict_boundary_is_infeasible_even_with_zero_distance_and_high_score() {
    let result = search(
        &Domain,
        &Evaluator {
            kind: ExecutionKind::RustCpu,
            f: |c| Ok((100. - c as f64, c as f64)),
        },
        &policy(true),
        SearchPlan::Finite(vec![0, 1]),
        &budget(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(result.feasible[0].candidate, 1);
    assert_eq!(result.infeasible[0].candidate, 0);
    assert_eq!(
        result.infeasible[0]
            .assessment
            .total_normalized_violation
            .finite(),
        Some(0.)
    );
}

#[test]
fn infeasible_beam_survives_a_feasible_incumbent() {
    struct Bridge(Mutex<bool>);
    impl SearchDomain<u8> for Bridge {
        fn validate(&self, _: &u8, _: &EvaluationControl<'_>) -> Result<(), String> {
            Ok(())
        }
        fn propose(
            &self,
            parents: &[u8],
            round: usize,
            _: u64,
            _: usize,
            _: &EvaluationControl<'_>,
        ) -> Result<Vec<u8>, String> {
            if round == 1 {
                *self.0.lock().unwrap() = parents.contains(&0) && parents.contains(&1);
                Ok(if parents.contains(&1) {
                    vec![2]
                } else {
                    vec![]
                })
            } else {
                Ok(vec![])
            }
        }
    }
    let domain = Bridge(Mutex::new(false));
    let result = search(
        &domain,
        &Evaluator {
            kind: ExecutionKind::RustCpu,
            f: |c| Ok((c as f64, if c == 1 { -1. } else { 1. })),
        },
        &policy(false),
        SearchPlan::Explore(vec![0, 1]),
        &budget(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(*domain.0.lock().unwrap());
    assert_eq!(result.feasible[0].candidate, 2);
    assert_eq!(result.termination, Termination::SearchStalled);
}

#[test]
fn attempts_include_failures_dedup_and_reserved_verification_are_bounded() {
    let calls = AtomicUsize::new(0);
    let evaluator = Evaluator {
        kind: ExecutionKind::ExternalProcess,
        f: |c| {
            calls.fetch_add(1, Ordering::SeqCst);
            if c == 0 {
                Err("failed".into())
            } else {
                Ok((c as f64, 1.))
            }
        },
    };
    let b = SearchBudget {
        max_evaluations: 4,
        ..budget()
    };
    let result = search(
        &Domain,
        &evaluator,
        &policy(false),
        SearchPlan::Finite(vec![0, 0, 1, 2, 3, 64]),
        &b,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(result.termination, Termination::EvaluationBudget);
    assert_eq!(result.statistics.evaluations, 4);
    assert_eq!(calls.load(Ordering::SeqCst), 4);
    assert_eq!(result.statistics.evaluation_failures, 1);
    assert_eq!(result.statistics.duplicates, 1);
    assert_eq!(result.statistics.rejected, 1);
    assert_eq!(result.statistics.verification_evaluations, 1);
    assert_eq!(result.feasible[0].candidate, 2);
}

#[test]
fn exact_last_attempt_completes_finite_domain_and_partial_batch_does_not() {
    for size in 1..9 {
        for jobs in 1..5 {
            let b = SearchBudget {
                jobs,
                max_evaluations: 5,
                verification_attempts: 0,
                ..budget()
            };
            let result = search(
                &Domain,
                &Evaluator {
                    kind: ExecutionKind::RustCpu,
                    f: landscape,
                },
                &policy(false),
                SearchPlan::Finite((0..size).collect()),
                &b,
                &AtomicBool::new(false),
            )
            .unwrap();
            assert_eq!(result.statistics.evaluations, usize::from(size).min(5));
            assert_eq!(
                result.termination,
                if size <= 5 {
                    Termination::FiniteDomainProcessed
                } else {
                    Termination::EvaluationBudget
                }
            );
        }
    }
}

#[test]
fn unavailable_nonfinite_values_never_enter_archives() {
    let result = search(
        &Domain,
        &Evaluator {
            kind: ExecutionKind::RustCpu,
            f: |_| Ok((f64::INFINITY, 1.)),
        },
        &policy(false),
        SearchPlan::Finite(vec![0]),
        &budget(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(result.statistics.unavailable, 1);
    assert!(result.feasible.is_empty());
    assert!(result.infeasible.is_empty());
    assert!(result.verifications.is_empty());
}

#[test]
fn fresh_verification_detects_nondeterminism_without_upgrading_diagnostics() {
    let calls = AtomicUsize::new(0);
    let result = search(
        &Domain,
        &Evaluator {
            kind: ExecutionKind::RustCpu,
            f: |_| Ok((calls.fetch_add(1, Ordering::SeqCst) as f64, 1.)),
        },
        &policy(false),
        SearchPlan::Finite(vec![0]),
        &budget(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(!result.verifications[0].consistent);
    assert!(result.verifications[0].error.is_some());
    assert_eq!(
        result.feasible[0].assessment.objective_value.finite(),
        Some(0.)
    );
}

#[test]
fn cancellation_before_start_and_during_evaluation_join_workers() {
    let cancel = AtomicBool::new(true);
    let calls = AtomicUsize::new(0);
    let evaluator = Evaluator {
        kind: ExecutionKind::ExternalProcess,
        f: |_| {
            calls.fetch_add(1, Ordering::SeqCst);
            cancel.store(true, Ordering::SeqCst);
            Ok((1., 1.))
        },
    };
    let before = search(
        &Domain,
        &evaluator,
        &policy(false),
        SearchPlan::Finite(vec![0, 1]),
        &budget(),
        &cancel,
    )
    .unwrap();
    assert_eq!(before.termination, Termination::Cancelled);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    cancel.store(false, Ordering::SeqCst);
    let during = search(
        &Domain,
        &evaluator,
        &policy(false),
        SearchPlan::Finite(vec![0, 1]),
        &budget(),
        &cancel,
    )
    .unwrap();
    assert_eq!(during.termination, Termination::Cancelled);
    assert!(during.statistics.discarded_late > 0);
    assert!(during.feasible.is_empty());
}

#[test]
fn resource_limits_reject_invalid_settings_and_bound_duplicate_only_search() {
    let mut b = budget();
    b.jobs = 0;
    assert!(
        search(
            &Domain,
            &Evaluator {
                kind: ExecutionKind::RustCpu,
                f: landscape
            },
            &policy(false),
            SearchPlan::Finite(vec![0]),
            &b,
            &AtomicBool::new(false)
        )
        .is_err()
    );
    struct Duplicates;
    impl SearchDomain<u8> for Duplicates {
        fn validate(&self, _: &u8, _: &EvaluationControl<'_>) -> Result<(), String> {
            Ok(())
        }
        fn propose(
            &self,
            _: &[u8],
            _: usize,
            _: u64,
            _: usize,
            _: &EvaluationControl<'_>,
        ) -> Result<Vec<u8>, String> {
            Ok(vec![0])
        }
    }
    b = SearchBudget {
        max_rounds: 3,
        ..budget()
    };
    let report = search(
        &Duplicates,
        &Evaluator {
            kind: ExecutionKind::RustCpu,
            f: landscape,
        },
        &policy(false),
        SearchPlan::Explore(vec![0]),
        &b,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(report.termination, Termination::RoundBudget);
    assert_eq!(report.statistics.rounds, 3);
    assert_eq!(report.statistics.evaluations, 2);
}

#[test]
fn both_schedulers_actually_run_concurrently_within_job_limit() {
    for kind in [ExecutionKind::RustCpu, ExecutionKind::ExternalProcess] {
        let active = AtomicUsize::new(0);
        let peak = AtomicUsize::new(0);
        let barrier = std::sync::Barrier::new(2);
        let evaluator = Evaluator {
            kind,
            f: |_| {
                let n = active.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(n, Ordering::SeqCst);
                barrier.wait();
                active.fetch_sub(1, Ordering::SeqCst);
                Ok((1., 1.))
            },
        };
        let b = SearchBudget {
            jobs: 2,
            verification_attempts: 0,
            ..budget()
        };
        search(
            &Domain,
            &evaluator,
            &policy(false),
            SearchPlan::Finite(vec![0, 1]),
            &b,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(peak.load(Ordering::SeqCst), 2);
        assert_eq!(active.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn malformed_finite_policy_evidence_is_never_ranked_or_verified() {
    struct Corrupt(CompiledObjective);
    impl ScoringPolicy for Corrupt {
        fn required_metrics(&self) -> Vec<MetricQuery> {
            self.0.required_metrics()
        }
        fn assess(
            &self,
            measurements: &[MetricMeasurement],
        ) -> Result<ObjectiveAssessment, ObjectiveError> {
            let mut assessment = self.0.assess(measurements)?;
            assessment.constraints[0].observed = MeasurementValue::Finite { value: f64::NAN };
            Ok(assessment)
        }
    }
    let report = search(
        &Domain,
        &Evaluator {
            kind: ExecutionKind::RustCpu,
            f: landscape,
        },
        &Corrupt(policy(false)),
        SearchPlan::Finite(vec![0]),
        &budget(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(report.feasible.is_empty());
    assert!(report.verifications.is_empty());
    assert_eq!(report.statistics.unavailable, 1);
}
