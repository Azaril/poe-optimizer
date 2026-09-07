//! Regression cases for scheduler limits and the fresh-verification evidence boundary.
use poe_optimizer_core::{metrics::*, objective::*};
use poe_optimizer_search::*;
use std::{
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    time::{Duration, Instant},
};

fn metric(id: &str) -> MetricQuery {
    MetricQuery {
        actor: ActorScope::Player,
        id: id.to_owned(),
    }
}

fn measurements(version: u32) -> CandidateMeasurements {
    CandidateMeasurements {
        measurements: [("score", 10.0), ("floor", 5.0)]
            .map(|(id, value)| MetricMeasurement {
                query: metric(id),
                unit: MetricUnit::Damage,
                value: MeasurementValue::from_number(value),
                schema_version: version,
            })
            .to_vec(),
        diagnostic_only: true,
    }
}

fn policy(version: u32) -> CompiledObjective {
    ObjectiveSpec {
        schema_version: 1,
        objective: ObjectivePolicy::Scalar {
            metric: metric("score"),
            unit: MetricUnit::Damage,
            direction: Direction::Maximize,
        },
        constraints: vec![MetricConstraint {
            id: "floor".to_owned(),
            metric: metric("floor"),
            unit: MetricUnit::Damage,
            operator: Comparison::AtLeast,
            threshold: 0.0,
            violation_scale: 1.0,
        }],
    }
    .compile(&["score", "floor"].map(|id| MetricDefinition {
        id: id.to_owned(),
        unit: MetricUnit::Damage,
        actors: vec![ActorScope::Player],
        description: String::new(),
        schema_version: version,
    }))
    .unwrap()
}

fn limits() -> SearchBudget {
    SearchBudget {
        max_evaluations: 8,
        max_proposals: 32,
        max_rounds: 3,
        duration: Duration::from_secs(10),
        jobs: 2,
        beam_per_status: 2,
        archive_size: 2,
        verification_attempts: 0,
        seed: 91,
    }
}

struct Allow;
impl SearchDomain<u8> for Allow {
    fn validate(&self, _: &u8, _: &EvaluationControl<'_>) -> Result<(), String> {
        Ok(())
    }
}

struct Evaluator<F> {
    kind: ExecutionKind,
    run: F,
}
impl<F> CandidateEvaluator<u8> for Evaluator<F>
where
    F: Fn(u8, &EvaluationControl<'_>) -> Result<CandidateMeasurements, String> + Sync,
{
    fn execution_kind(&self) -> ExecutionKind {
        self.kind
    }
    fn evaluate(
        &self,
        candidate: &u8,
        control: &EvaluationControl<'_>,
    ) -> Result<CandidateMeasurements, String> {
        (self.run)(*candidate, control)
    }
}

#[test]
fn last_round_keeps_the_archive_without_calling_an_unusable_proposer() {
    struct LastRound(AtomicUsize);
    impl SearchDomain<u8> for LastRound {
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
            self.0.fetch_add(1, Ordering::SeqCst);
            Err("No next round is permitted, so this must not run".to_owned())
        }
    }
    let domain = LastRound(AtomicUsize::new(0));
    let report = search(
        &domain,
        &Evaluator {
            kind: ExecutionKind::RustCpu,
            run: |_, _: &EvaluationControl<'_>| Ok(measurements(1)),
        },
        &policy(1),
        SearchPlan::Explore(vec![0]),
        &SearchBudget {
            max_rounds: 1,
            ..limits()
        },
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(domain.0.load(Ordering::SeqCst), 0);
    assert_eq!(report.termination, Termination::RoundBudget);
    assert_eq!(report.statistics.rounds, 1);
    assert_eq!(report.statistics.evaluations, 1);
    assert_eq!(report.feasible[0].candidate, 0);
}

#[test]
fn cancellation_during_validation_stops_before_the_next_candidate_or_evaluation() {
    struct CancelValidation<'a> {
        cancelled: &'a AtomicBool,
        calls: AtomicUsize,
    }
    impl SearchDomain<u8> for CancelValidation<'_> {
        fn validate(&self, _: &u8, control: &EvaluationControl<'_>) -> Result<(), String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.cancelled.store(true, Ordering::SeqCst);
            assert!(control.should_stop());
            Ok(())
        }
    }
    let cancelled = AtomicBool::new(false);
    let domain = CancelValidation {
        cancelled: &cancelled,
        calls: AtomicUsize::new(0),
    };
    let calls = AtomicUsize::new(0);
    let report = search(
        &domain,
        &Evaluator {
            kind: ExecutionKind::ExternalProcess,
            run: |_, _: &EvaluationControl<'_>| {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(measurements(1))
            },
        },
        &policy(1),
        SearchPlan::Finite(vec![0, 1, 2]),
        &limits(),
        &cancelled,
    )
    .unwrap();
    assert_eq!(report.termination, Termination::Cancelled);
    assert_eq!(domain.calls.load(Ordering::SeqCst), 1);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(report.statistics.evaluations, 0);
    assert!(report.feasible.is_empty());
}

#[test]
fn cooperative_proposer_error_after_cancellation_preserves_completed_work() {
    struct CancelProposal<'a>(&'a AtomicBool);
    impl SearchDomain<u8> for CancelProposal<'_> {
        fn validate(&self, _: &u8, _: &EvaluationControl<'_>) -> Result<(), String> {
            Ok(())
        }
        fn propose(
            &self,
            _: &[u8],
            _: usize,
            _: u64,
            _: usize,
            control: &EvaluationControl<'_>,
        ) -> Result<Vec<u8>, String> {
            self.0.store(true, Ordering::SeqCst);
            assert!(control.should_stop());
            Err("Proposal generation cancelled".to_owned())
        }
    }
    let cancelled = AtomicBool::new(false);
    let report = search(
        &CancelProposal(&cancelled),
        &Evaluator {
            kind: ExecutionKind::ExternalProcess,
            run: |_, _: &EvaluationControl<'_>| Ok(measurements(1)),
        },
        &policy(1),
        SearchPlan::Explore(vec![0]),
        &limits(),
        &cancelled,
    )
    .unwrap();
    assert_eq!(report.termination, Termination::Cancelled);
    assert_eq!(report.statistics.evaluations, 1);
    assert_eq!(report.feasible[0].candidate, 0);
}

#[test]
fn cancellation_wins_over_the_partial_batch_evaluation_limit() {
    for kind in [ExecutionKind::RustCpu, ExecutionKind::ExternalProcess] {
        let cancelled = AtomicBool::new(false);
        let calls = AtomicUsize::new(0);
        let report = search(
            &Allow,
            &Evaluator {
                kind,
                run: |_, _: &EvaluationControl<'_>| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    cancelled.store(true, Ordering::SeqCst);
                    Ok(measurements(1))
                },
            },
            &policy(1),
            SearchPlan::Finite(vec![0, 1, 2]),
            &SearchBudget {
                max_evaluations: 2,
                jobs: 4,
                ..limits()
            },
            &cancelled,
        )
        .unwrap();
        let attempts = calls.load(Ordering::SeqCst);
        assert!((1..=2).contains(&attempts));
        assert_eq!(report.termination, Termination::Cancelled);
        assert_eq!(report.statistics.evaluations, attempts);
        assert_eq!(report.statistics.discarded_late, attempts);
        assert!(report.feasible.is_empty());
    }
}

// Exercise an actual deadline, with a test-side escape hatch to avoid a stuck CI
// process if the control implementation regresses. No unbounded barrier is used.
fn wait_for_deadline(control: &EvaluationControl<'_>) {
    let test_limit = Instant::now() + Duration::from_secs(5);
    while !control.should_stop() && Instant::now() < test_limit {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(
        control.should_stop(),
        "Search failed to expose its deadline"
    );
}

#[test]
fn both_schedulers_discard_results_returned_after_the_deadline() {
    for kind in [ExecutionKind::RustCpu, ExecutionKind::ExternalProcess] {
        let calls = AtomicUsize::new(0);
        let report = search(
            &Allow,
            &Evaluator {
                kind,
                run: |_, control: &EvaluationControl<'_>| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    wait_for_deadline(control);
                    Ok(measurements(1))
                },
            },
            &policy(1),
            SearchPlan::Finite(vec![0]),
            &SearchBudget {
                duration: Duration::from_secs(1),
                jobs: 1,
                ..limits()
            },
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(report.termination, Termination::TimeBudget);
        assert_eq!(report.statistics.evaluations, 1);
        assert_eq!(report.statistics.discarded_late, 1);
        assert!(report.feasible.is_empty());
        assert!(report.verifications.is_empty());
    }
}

#[test]
fn fresh_backend_failure_counts_as_an_attempt_and_preserves_unverified_search_evidence() {
    let calls = AtomicUsize::new(0);
    let report = search(
        &Allow,
        &Evaluator {
            kind: ExecutionKind::ExternalProcess,
            run: |_, _: &EvaluationControl<'_>| {
                if calls.fetch_add(1, Ordering::SeqCst) == 0 {
                    Ok(measurements(1))
                } else {
                    Err("Fresh backend failure".to_owned())
                }
            },
        },
        &policy(1),
        SearchPlan::Finite(vec![0]),
        &SearchBudget {
            verification_attempts: 1,
            ..limits()
        },
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(report.statistics.evaluations, 2);
    assert_eq!(report.statistics.verification_evaluations, 1);
    assert_eq!(report.statistics.evaluation_failures, 1);
    assert_eq!(report.statistics.discarded_late, 0);
    assert!(
        report
            .errors
            .iter()
            .any(|error| error.contains("Fresh backend failure"))
    );
    assert_eq!(report.feasible[0].candidate, 0);
    assert!(!report.verifications[0].consistent);
    assert!(report.verifications[0].fresh_assessment.is_none());
}

#[test]
fn late_fresh_verification_is_counted_but_cannot_certify_the_incumbent() {
    let calls = AtomicUsize::new(0);
    let report = search(
        &Allow,
        &Evaluator {
            kind: ExecutionKind::ExternalProcess,
            run: |_, control: &EvaluationControl<'_>| {
                if calls.fetch_add(1, Ordering::SeqCst) != 0 {
                    wait_for_deadline(control);
                }
                Ok(measurements(1))
            },
        },
        &policy(1),
        SearchPlan::Finite(vec![0]),
        &SearchBudget {
            verification_attempts: 1,
            duration: Duration::from_secs(1),
            ..limits()
        },
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(report.statistics.evaluations, 2);
    assert_eq!(report.statistics.verification_evaluations, 1);
    assert_eq!(report.statistics.discarded_late, 1);
    assert_eq!(report.statistics.evaluation_failures, 1);
    assert_eq!(report.errors.len(), 1);
    assert_eq!(report.feasible[0].candidate, 0);
    assert!(!report.verifications[0].consistent);
    assert!(report.verifications[0].fresh_assessment.is_none());
    assert!(
        report.verifications[0]
            .error
            .as_ref()
            .unwrap()
            .contains("deadline")
    );
}

#[derive(Debug, Clone, Copy)]
enum Drift {
    Score,
    NonfiniteScore,
    Specification,
    ConstraintDefinition,
    MetricSchema,
    DuplicateAlways,
}
struct ChangingPolicy {
    inner: CompiledObjective,
    calls: AtomicUsize,
    drift: Drift,
}
impl ScoringPolicy for ChangingPolicy {
    fn required_metrics(&self) -> Vec<MetricQuery> {
        self.inner.required_metrics()
    }
    fn assess(
        &self,
        measurements: &[MetricMeasurement],
    ) -> Result<ObjectiveAssessment, ObjectiveError> {
        let mut assessment = self.inner.assess(measurements)?;
        let fresh = self.calls.fetch_add(1, Ordering::SeqCst) != 0;
        match self.drift {
            Drift::Score if fresh => {
                assessment.objective_score = MeasurementValue::from_number(25.0)
            }
            Drift::NonfiniteScore if fresh => {
                assessment.objective_score = MeasurementValue::from_number(f64::INFINITY)
            }
            Drift::Specification if fresh => {
                assessment.specification.constraints[0].threshold += 1e-12
            }
            Drift::ConstraintDefinition if fresh => {
                assessment.constraints[0].constraint.threshold += 1e-12
            }
            Drift::MetricSchema if fresh => assessment.measurements[0].schema_version += 1,
            Drift::DuplicateAlways => {
                assessment.measurements[1] = assessment.measurements[0].clone()
            }
            _ => {}
        }
        Ok(assessment)
    }
}

#[test]
fn verification_rejects_policy_drift_and_nonunique_or_changed_metric_schema_evidence() {
    for drift in [
        Drift::Score,
        Drift::NonfiniteScore,
        Drift::Specification,
        Drift::ConstraintDefinition,
        Drift::MetricSchema,
        Drift::DuplicateAlways,
    ] {
        // Large schema numbers expose accidental use of numeric result tolerances
        // on identity metadata: a difference of 1 must still be incompatible.
        let version = 1_000_000_000;
        let policy = ChangingPolicy {
            inner: policy(version),
            calls: AtomicUsize::new(0),
            drift,
        };
        let report = search(
            &Allow,
            &Evaluator {
                kind: ExecutionKind::RustCpu,
                run: |_, _: &EvaluationControl<'_>| Ok(measurements(version)),
            },
            &policy,
            SearchPlan::Finite(vec![0]),
            &SearchBudget {
                verification_attempts: 1,
                ..limits()
            },
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(report.statistics.evaluations, 2, "{drift:?}");
        assert_eq!(report.verifications.len(), 1, "{drift:?}");
        assert!(
            !report.verifications[0].consistent,
            "Incorrectly verified {drift:?}"
        );
        assert!(report.verifications[0].error.is_some(), "{drift:?}");
        assert_eq!(report.statistics.evaluation_failures, 1, "{drift:?}");
    }
}
