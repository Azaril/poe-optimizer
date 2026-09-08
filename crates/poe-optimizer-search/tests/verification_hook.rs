//! Prepared search evaluation must be checked by a fresh, independently selectable path.
use poe_optimizer_core::{metrics::*, objective::*};
use poe_optimizer_search::*;
use std::{
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

fn measurement(score: f64, diagnostic_only: bool) -> CandidateMeasurements {
    CandidateMeasurements {
        measurements: vec![MetricMeasurement {
            query: MetricQuery {
                actor: ActorScope::Player,
                id: "score".into(),
            },
            unit: MetricUnit::Damage,
            value: MeasurementValue::from_number(score),
            schema_version: 1,
        }],
        diagnostic_only,
    }
}
fn policy() -> CompiledObjective {
    let query = MetricQuery {
        actor: ActorScope::Player,
        id: "score".into(),
    };
    ObjectiveSpec {
        schema_version: 1,
        objective: ObjectivePolicy::Scalar {
            metric: query.clone(),
            unit: MetricUnit::Damage,
            direction: Direction::Maximize,
        },
        constraints: vec![MetricConstraint {
            id: "floor".into(),
            metric: query,
            unit: MetricUnit::Damage,
            operator: Comparison::AtLeast,
            threshold: 0.0,
            violation_scale: 1.0,
        }],
    }
    .compile(&[MetricDefinition {
        id: "score".into(),
        unit: MetricUnit::Damage,
        actors: vec![ActorScope::Player],
        description: String::new(),
        schema_version: 1,
    }])
    .unwrap()
}
fn budget() -> SearchBudget {
    SearchBudget {
        max_evaluations: 5,
        max_proposals: 32,
        max_rounds: 2,
        duration: Duration::from_secs(10),
        jobs: 4,
        beam_per_status: 4,
        archive_size: 4,
        verification_attempts: 2,
        seed: 19,
    }
}
struct Allow;
impl SearchDomain<u8> for Allow {
    fn validate(&self, _: &u8, _: &EvaluationControl<'_>) -> Result<(), String> {
        Ok(())
    }
}
struct Separate<E, V> {
    kind: ExecutionKind,
    evaluate: E,
    verify: V,
}
impl<E, V> CandidateEvaluator<u8> for Separate<E, V>
where
    E: Fn(u8, &EvaluationControl<'_>) -> Result<CandidateMeasurements, String> + Sync,
    V: Fn(u8, &EvaluationControl<'_>) -> Result<CandidateMeasurements, String> + Sync,
{
    fn execution_kind(&self) -> ExecutionKind {
        self.kind
    }
    fn evaluate(
        &self,
        candidate: &u8,
        control: &EvaluationControl<'_>,
    ) -> Result<CandidateMeasurements, String> {
        (self.evaluate)(*candidate, control)
    }
    fn verify(
        &self,
        candidate: &u8,
        control: &EvaluationControl<'_>,
    ) -> Result<CandidateMeasurements, String> {
        (self.verify)(*candidate, control)
    }
}

#[test]
fn reserved_finalists_use_the_independent_hook_once_in_rank_order() {
    for kind in [ExecutionKind::RustCpu, ExecutionKind::ExternalProcess] {
        for jobs in [1, 4] {
            let evaluations = Mutex::new(Vec::new());
            let verifications = Mutex::new(Vec::new());
            let result = search(
                &Allow,
                &Separate {
                    kind,
                    evaluate: |candidate, control: &EvaluationControl<'_>| {
                        assert!(!control.should_stop());
                        evaluations.lock().unwrap().push(candidate);
                        Ok(measurement(f64::from(candidate), false))
                    },
                    verify: |candidate, control: &EvaluationControl<'_>| {
                        assert!(!control.should_stop());
                        verifications.lock().unwrap().push(candidate);
                        Ok(measurement(f64::from(candidate), false))
                    },
                },
                &policy(),
                SearchPlan::Finite(vec![0, 1, 2, 3]),
                &SearchBudget { jobs, ..budget() },
                &AtomicBool::new(false),
            )
            .unwrap();
            let mut searched = evaluations.into_inner().unwrap();
            searched.sort();
            assert_eq!(searched, vec![0, 1, 2]);
            assert_eq!(verifications.into_inner().unwrap(), vec![2, 1]);
            assert_eq!(result.termination, Termination::EvaluationBudget);
            assert_eq!(result.statistics.evaluations, 5);
            assert_eq!(result.statistics.verification_evaluations, 2);
            assert_eq!(result.statistics.evaluation_failures, 0);
            assert!(result.verifications.iter().all(|v| v.consistent));
        }
    }
}

#[test]
fn verification_failure_drift_and_panic_consume_one_reserved_attempt_without_retry() {
    for kind in [ExecutionKind::RustCpu, ExecutionKind::ExternalProcess] {
        for mode in ["failure", "drift", "panic"] {
            let evaluations = AtomicUsize::new(0);
            let verifications = AtomicUsize::new(0);
            let result = search(
                &Allow,
                &Separate {
                    kind,
                    evaluate: |_, _: &EvaluationControl<'_>| {
                        evaluations.fetch_add(1, Ordering::SeqCst);
                        Ok(measurement(1.0, true))
                    },
                    verify: |_, _: &EvaluationControl<'_>| {
                        verifications.fetch_add(1, Ordering::SeqCst);
                        match mode {
                            "failure" => Err("independent realization failed".into()),
                            "drift" => Ok(measurement(2.0, false)),
                            _ => panic!("independent realization panicked"),
                        }
                    },
                },
                &policy(),
                SearchPlan::Finite(vec![0]),
                &budget(),
                &AtomicBool::new(false),
            )
            .unwrap();
            assert_eq!(evaluations.load(Ordering::SeqCst), 1);
            assert_eq!(verifications.load(Ordering::SeqCst), 1);
            assert_eq!(result.statistics.evaluations, 2);
            assert_eq!(result.statistics.verification_evaluations, 1);
            assert_eq!(result.statistics.evaluation_failures, 1);
            assert_eq!(result.statistics.discarded_late, 0);
            assert_eq!(
                result.feasible[0].assessment.objective_value.finite(),
                Some(1.0)
            );
            let verification = &result.verifications[0];
            assert!(!verification.consistent);
            assert!(verification.diagnostic_only);
            assert!(verification.error.is_some());
            assert_eq!(verification.fresh_assessment.is_some(), mode == "drift");
        }
    }
}

#[test]
fn verification_preserves_diagnostic_evidence_from_either_path() {
    for (search_diagnostic, verification_diagnostic) in
        [(true, false), (false, true), (false, false)]
    {
        let result = search(
            &Allow,
            &Separate {
                kind: ExecutionKind::RustCpu,
                evaluate: |_, _: &EvaluationControl<'_>| Ok(measurement(1.0, search_diagnostic)),
                verify: |_, _: &EvaluationControl<'_>| {
                    Ok(measurement(1.0, verification_diagnostic))
                },
            },
            &policy(),
            SearchPlan::Finite(vec![0]),
            &budget(),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert!(result.verifications[0].consistent);
        assert_eq!(
            result.verifications[0].diagnostic_only,
            search_diagnostic || verification_diagnostic
        );
    }
}

#[test]
fn late_or_cancelled_verification_cannot_certify_or_start_another_finalist() {
    for kind in [ExecutionKind::RustCpu, ExecutionKind::ExternalProcess] {
        for cancel_in_verify in [false, true] {
            let cancelled = AtomicBool::new(false);
            let evaluations = AtomicUsize::new(0);
            let verifications = AtomicUsize::new(0);
            let result = search(
                &Allow,
                &Separate {
                    kind,
                    evaluate: |candidate, _: &EvaluationControl<'_>| {
                        evaluations.fetch_add(1, Ordering::SeqCst);
                        Ok(measurement(f64::from(candidate), false))
                    },
                    verify: |candidate, control: &EvaluationControl<'_>| {
                        verifications.fetch_add(1, Ordering::SeqCst);
                        if cancel_in_verify {
                            cancelled.store(true, Ordering::SeqCst);
                        } else {
                            let test_limit = Instant::now() + Duration::from_secs(5);
                            while !control.should_stop() && Instant::now() < test_limit {
                                std::thread::sleep(Duration::from_millis(1));
                            }
                        }
                        assert!(control.should_stop());
                        Ok(measurement(f64::from(candidate), false))
                    },
                },
                &policy(),
                SearchPlan::Finite(vec![0, 1]),
                &SearchBudget {
                    duration: Duration::from_secs(1),
                    ..budget()
                },
                &cancelled,
            )
            .unwrap();
            assert_eq!(evaluations.load(Ordering::SeqCst), 2);
            assert_eq!(verifications.load(Ordering::SeqCst), 1);
            assert_eq!(result.statistics.evaluations, 3);
            assert_eq!(result.statistics.verification_evaluations, 1);
            assert_eq!(result.statistics.discarded_late, 1);
            assert_eq!(result.statistics.evaluation_failures, 1);
            assert_eq!(result.verifications.len(), 1);
            let verification = &result.verifications[0];
            assert!(!verification.consistent);
            assert!(verification.fresh_assessment.is_none());
            assert!(
                verification
                    .error
                    .as_ref()
                    .unwrap()
                    .contains("deadline or was cancelled")
            );
        }
    }
}

#[test]
fn cancellation_after_ranking_skips_the_reserved_hook() {
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
            _: &EvaluationControl<'_>,
        ) -> Result<Vec<u8>, String> {
            self.0.store(true, Ordering::SeqCst);
            Err("proposal cancelled".into())
        }
    }
    for kind in [ExecutionKind::RustCpu, ExecutionKind::ExternalProcess] {
        let cancelled = AtomicBool::new(false);
        let verifications = AtomicUsize::new(0);
        let result = search(
            &CancelProposal(&cancelled),
            &Separate {
                kind,
                evaluate: |_, _: &EvaluationControl<'_>| Ok(measurement(1.0, false)),
                verify: |_, _: &EvaluationControl<'_>| {
                    verifications.fetch_add(1, Ordering::SeqCst);
                    Ok(measurement(1.0, false))
                },
            },
            &policy(),
            SearchPlan::Explore(vec![0]),
            &budget(),
            &cancelled,
        )
        .unwrap();
        assert_eq!(result.termination, Termination::Cancelled);
        assert_eq!(result.feasible.len(), 1);
        assert_eq!(result.statistics.evaluations, 1);
        assert_eq!(result.statistics.verification_evaluations, 0);
        assert!(result.verifications.is_empty());
        assert_eq!(verifications.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn no_verification_is_spent_without_a_reservation_or_a_feasible_candidate() {
    for kind in [ExecutionKind::RustCpu, ExecutionKind::ExternalProcess] {
        for (reservations, score) in [(0, 1.0), (2, -1.0)] {
            let evaluations = AtomicUsize::new(0);
            let verifications = AtomicUsize::new(0);
            let result = search(
                &Allow,
                &Separate {
                    kind,
                    evaluate: |_, _: &EvaluationControl<'_>| {
                        evaluations.fetch_add(1, Ordering::SeqCst);
                        Ok(measurement(score, false))
                    },
                    verify: |_, _: &EvaluationControl<'_>| {
                        verifications.fetch_add(1, Ordering::SeqCst);
                        Ok(measurement(score, false))
                    },
                },
                &policy(),
                SearchPlan::Finite(vec![0, 1]),
                &SearchBudget {
                    verification_attempts: reservations,
                    ..budget()
                },
                &AtomicBool::new(false),
            )
            .unwrap();
            assert_eq!(evaluations.load(Ordering::SeqCst), 2);
            assert_eq!(verifications.load(Ordering::SeqCst), 0);
            assert_eq!(result.statistics.evaluations, 2);
            assert_eq!(result.statistics.verification_evaluations, 0);
            assert!(result.verifications.is_empty());
        }
    }
}
