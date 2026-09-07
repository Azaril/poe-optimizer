use poe_optimizer_core::{metrics::*, objective::*};
use poe_optimizer_search::{discrete::*, *};
use std::{
    collections::BTreeMap,
    sync::{Mutex, atomic::AtomicBool},
    time::Duration,
};

struct Domain {
    space: DiscreteSpace,
    settings: Neighborhood,
    seen: Mutex<Vec<Vec<DiscretePoint>>>,
}
impl SearchDomain<DiscretePoint> for Domain {
    fn can_propose_after_empty(&self) -> bool {
        self.space.size() != Some(1)
    }
    fn validate(&self, p: &DiscretePoint, _: &EvaluationControl<'_>) -> Result<(), String> {
        self.space.validate(p)
    }
    fn propose(
        &self,
        parents: &[DiscretePoint],
        round: usize,
        seed: u64,
        limit: usize,
        control: &EvaluationControl<'_>,
    ) -> Result<Vec<DiscretePoint>, String> {
        let result = self
            .space
            .propose(parents, round, seed, limit, &self.settings, control)?;
        self.seen.lock().unwrap().push(result.clone());
        Ok(result)
    }
}
struct Evaluator;
impl CandidateEvaluator<DiscretePoint> for Evaluator {
    fn execution_kind(&self) -> ExecutionKind {
        ExecutionKind::RustCpu
    }
    fn evaluate(
        &self,
        p: &DiscretePoint,
        _: &EvaluationControl<'_>,
    ) -> Result<CandidateMeasurements, String> {
        let pair = match (p.choices[0], p.choices[1]) {
            (0, 0) => 10.,
            (1, 1) => 20.,
            _ => 0.,
        };
        Ok(CandidateMeasurements {
            diagnostic_only: true,
            measurements: vec![MetricMeasurement {
                query: query(),
                unit: MetricUnit::Damage,
                schema_version: 1,
                value: MeasurementValue::from_number(
                    pair + p.choices.iter().skip(2).map(|v| f64::from(*v)).sum::<f64>(),
                ),
            }],
        })
    }
}
fn query() -> MetricQuery {
    MetricQuery {
        actor: ActorScope::Player,
        id: "score".into(),
    }
}
fn policy() -> CompiledObjective {
    ObjectiveSpec {
        schema_version: 1,
        objective: ObjectivePolicy::Scalar {
            metric: query(),
            unit: MetricUnit::Damage,
            direction: Direction::Maximize,
        },
        constraints: vec![],
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
fn budget(jobs: usize) -> SearchBudget {
    SearchBudget {
        max_evaluations: 100,
        max_proposals: 1000,
        max_rounds: 20,
        beam_per_status: 1,
        archive_size: 1,
        duration: Duration::from_secs(10),
        jobs,
        verification_attempts: 1,
        seed: 734,
    }
}
fn domain(locks: BTreeMap<usize, u32>) -> Domain {
    Domain {
        space: DiscreteSpace::new("fixture-v1".into(), vec![2; 6], locks).unwrap(),
        settings: Neighborhood::default(),
        seen: Mutex::new(vec![]),
    }
}

#[test]
fn coupled_proposer_matches_exhaustive_reference_and_one_many_workers() {
    let mut expected = None;
    for jobs in [1, 4] {
        let domain = domain(BTreeMap::new());
        let exact = search(
            &domain,
            &Evaluator,
            &policy(),
            SearchPlan::Finite(domain.space.enumerate(64).unwrap()),
            &budget(jobs),
            &AtomicBool::new(false),
        )
        .unwrap();
        let guided = search(
            &domain,
            &Evaluator,
            &policy(),
            SearchPlan::Explore(vec![domain.space.first()]),
            &budget(jobs),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(guided.feasible[0].candidate.choices, vec![1; 6]);
        assert_eq!(guided.feasible[0].candidate, exact.feasible[0].candidate);
        assert!(guided.verifications[0].consistent);
        let signature = (
            guided.statistics.proposals,
            guided.statistics.evaluations,
            domain.seen.into_inner().unwrap(),
        );
        if let Some(expected) = &expected {
            assert_eq!(expected, &signature);
        } else {
            expected = Some(signature);
        }
    }
}

#[test]
fn fixed_axes_and_singleton_choices_survive_every_coupled_move_and_restart() {
    let domain = Domain {
        space: DiscreteSpace::new(
            "fixed".into(),
            vec![2, 2, 1, 3, 2, 2],
            BTreeMap::from([(1, 1), (4, 0)]),
        )
        .unwrap(),
        settings: Neighborhood {
            restart_every: 1,
            ..Default::default()
        },
        seen: Mutex::new(vec![]),
    };
    assert_eq!(domain.space.size(), Some(12));
    let expected = domain.space.enumerate(12).unwrap();
    assert_eq!(expected.len(), 12);
    assert!(expected.windows(2).all(|p| p[0] < p[1]));
    let result = search(
        &domain,
        &Evaluator,
        &policy(),
        SearchPlan::Explore(vec![domain.space.first()]),
        &budget(4),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(result.statistics.evaluations <= 13);
    for point in domain.seen.lock().unwrap().iter().flatten() {
        assert_eq!(point.choices[1], 1);
        assert_eq!(point.choices[2], 0);
        assert_eq!(point.choices[4], 0);
        assert!(expected.contains(point));
    }
}

#[test]
fn huge_domains_remain_implicit_and_explicit_enumeration_is_bounded() {
    let domain = DiscreteSpace::new("large".into(), vec![u32::MAX; 128], BTreeMap::new()).unwrap();
    assert_eq!(domain.size(), None);
    assert!(domain.enumerate(1000).is_err());
    let fixed = DiscreteSpace::new(
        "fixed".into(),
        vec![u32::MAX; 128],
        (0..128).map(|n| (n, 0)).collect(),
    )
    .unwrap();
    assert_eq!(fixed.size(), Some(1));
    assert_eq!(fixed.enumerate(1).unwrap(), vec![fixed.first()]);
}

#[test]
fn invalid_space_point_and_neighborhood_fail_before_use() {
    assert!(DiscreteSpace::new("".into(), vec![2], BTreeMap::new()).is_err());
    assert!(DiscreteSpace::new("x".into(), vec![], BTreeMap::new()).is_err());
    assert!(DiscreteSpace::new("x".into(), vec![0], BTreeMap::new()).is_err());
    assert!(DiscreteSpace::new("x".into(), vec![2], BTreeMap::from([(1, 0)])).is_err());
    assert!(DiscreteSpace::new("x".into(), vec![2], BTreeMap::from([(0, 2)])).is_err());
    let d = domain(BTreeMap::from([(0, 1)]));
    assert!(d.space.point(vec![0; 6]).is_err());
    assert!(d.space.point(vec![1; 5]).is_err());
    assert!(d.space.point(vec![2; 6]).is_err());
    let mut p = d.space.first();
    p.space_id = "other".into();
    assert!(d.space.validate(&p).is_err());
    assert!(
        Neighborhood {
            proposals_per_round: 0,
            ..Default::default()
        }
        .validate()
        .is_err()
    );
    assert!(
        Neighborhood {
            restart_every: 0,
            ..Default::default()
        }
        .validate()
        .is_err()
    );
    assert!(serde_json::from_str::<Neighborhood>(r#"{"unknown":1}"#).is_err());
}

#[test]
fn all_fixed_space_stalls_without_claiming_exhaustive_completion() {
    let domain = domain((0..6).map(|n| (n, 1)).collect());
    let result = search(
        &domain,
        &Evaluator,
        &policy(),
        SearchPlan::Explore(vec![domain.space.first()]),
        &budget(2),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(result.termination, Termination::SearchStalled);
    assert_eq!(result.statistics.evaluations, 2);
    assert!(domain.seen.lock().unwrap().iter().all(Vec::is_empty));
}
