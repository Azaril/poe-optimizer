//! Review regressions for sampled empty rounds and huge implicit axis domains.
use poe_optimizer_core::{metrics::*, objective::*};
use poe_optimizer_search::{discrete::*, *};
use std::{
    collections::BTreeMap,
    sync::{Mutex, atomic::AtomicBool},
    time::Duration,
};

struct ObservedRound {
    round: usize,
    parents: Vec<DiscretePoint>,
    proposals: Vec<DiscretePoint>,
}

struct SamplingDomain {
    space: DiscreteSpace,
    settings: Neighborhood,
    force_seed: Option<u64>,
    observed: Mutex<Vec<ObservedRound>>,
}

impl SearchDomain<DiscretePoint> for SamplingDomain {
    fn can_propose_after_empty(&self) -> bool {
        self.space.size() != Some(1)
    }

    fn validate(&self, point: &DiscretePoint, _: &EvaluationControl<'_>) -> Result<(), String> {
        self.space.validate(point)
    }

    fn propose(
        &self,
        parents: &[DiscretePoint],
        round: usize,
        seed: u64,
        limit: usize,
        control: &EvaluationControl<'_>,
    ) -> Result<Vec<DiscretePoint>, String> {
        let proposals = self.space.propose(
            parents,
            round,
            self.force_seed.unwrap_or(seed),
            limit,
            &self.settings,
            control,
        )?;
        self.observed.lock().unwrap().push(ObservedRound {
            round,
            parents: parents.to_vec(),
            proposals: proposals.clone(),
        });
        Ok(proposals)
    }
}

struct FirstAxisEvaluator;
impl CandidateEvaluator<DiscretePoint> for FirstAxisEvaluator {
    fn execution_kind(&self) -> ExecutionKind {
        ExecutionKind::RustCpu
    }

    fn evaluate(
        &self,
        point: &DiscretePoint,
        _: &EvaluationControl<'_>,
    ) -> Result<CandidateMeasurements, String> {
        Ok(CandidateMeasurements {
            diagnostic_only: true,
            measurements: vec![MetricMeasurement {
                query: query(),
                unit: MetricUnit::Damage,
                schema_version: 1,
                value: MeasurementValue::from_number(f64::from(point.choices[0])),
            }],
        })
    }
}

fn query() -> MetricQuery {
    MetricQuery {
        actor: ActorScope::Player,
        id: "first_axis".into(),
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
        id: "first_axis".into(),
        unit: MetricUnit::Damage,
        actors: vec![ActorScope::Player],
        description: String::new(),
        schema_version: 1,
    }])
    .unwrap()
}

fn budget(rounds: usize) -> SearchBudget {
    SearchBudget {
        max_evaluations: 100,
        max_proposals: 1000,
        max_rounds: rounds,
        duration: Duration::from_secs(10),
        jobs: 2,
        beam_per_status: 2,
        archive_size: 2,
        verification_attempts: 1,
        seed: 734,
    }
}

#[test]
fn empty_random_sample_does_not_suppress_a_later_coordinated_move() {
    let domain = SamplingDomain {
        space: DiscreteSpace::new("empty-sample-v1".into(), vec![2, 2], BTreeMap::new()).unwrap(),
        settings: Neighborhood {
            proposals_per_round: 1,
            restart_every: 4,
            max_changed_axes: 0,
        },
        // With parents00/01, seed1 draws axis1 on all four single-axis attempts.
        force_seed: Some(1),
        observed: Mutex::new(vec![]),
    };
    let initial = vec![
        domain.space.point(vec![0, 0]).unwrap(),
        domain.space.point(vec![0, 1]).unwrap(),
    ];
    let report = search(
        &domain,
        &FirstAxisEvaluator,
        &policy(),
        SearchPlan::Explore(initial),
        &budget(3),
        &AtomicBool::new(false),
    )
    .unwrap();
    let observed = domain.observed.lock().unwrap();
    assert_eq!(observed.len(), 2);
    assert_eq!(observed[0].round, 1);
    assert!(observed[0].proposals.is_empty());
    assert_eq!(observed[1].round, 2);
    assert_eq!(
        observed[1].proposals,
        vec![domain.space.point(vec![1, 1]).unwrap()]
    );
    assert_eq!(report.feasible[0].candidate.choices, vec![1, 1]);
    assert_eq!(report.termination, Termination::RoundBudget);
    assert!(report.verifications[0].consistent);
}

#[test]
fn maximum_cardinality_proposals_keep_high_choices_and_nonzero_locks_in_bounds() {
    let locks = BTreeMap::from([(1, u32::MAX - 2), (31, 42), (100, u32::MAX - 1)]);
    let domain = SamplingDomain {
        space: DiscreteSpace::new(
            "huge-proposals-v1".into(),
            vec![u32::MAX; 128],
            locks.clone(),
        )
        .unwrap(),
        settings: Neighborhood {
            proposals_per_round: 16,
            restart_every: 2,
            max_changed_axes: 0,
        },
        force_seed: None,
        observed: Mutex::new(vec![]),
    };
    assert_eq!(domain.space.size(), None);
    let mut high_choices = vec![u32::MAX - 1; 128];
    for (axis, choice) in &locks {
        high_choices[*axis] = *choice;
    }
    let initial = domain.space.point(high_choices).unwrap();
    let report = search(
        &domain,
        &FirstAxisEvaluator,
        &policy(),
        SearchPlan::Explore(vec![initial]),
        &budget(4),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(report.termination, Termination::RoundBudget);
    let observed = domain.observed.lock().unwrap();
    assert_eq!(observed.len(), 3);
    let mut saw_full_restart = false;
    for sample in observed.iter() {
        assert!(!sample.proposals.is_empty());
        assert!(sample.proposals.len() <= 16);
        assert!(sample.proposals.windows(2).all(|pair| pair[0] < pair[1]));
        for point in &sample.proposals {
            domain.space.validate(point).unwrap();
            assert_eq!(point.choices.len(), 128);
            assert!(point.choices.iter().all(|choice| *choice < u32::MAX));
            for (axis, choice) in &locks {
                assert_eq!(point.choices[*axis], *choice);
            }
            assert!(!sample.parents.contains(point));
            let nearest_parent_distance = sample
                .parents
                .iter()
                .map(|parent| {
                    parent
                        .choices
                        .iter()
                        .zip(&point.choices)
                        .filter(|(before, after)| before != after)
                        .count()
                })
                .min()
                .unwrap();
            if sample.round == 2 && nearest_parent_distance > 2 {
                saw_full_restart = true;
            }
            if sample.round == 1 {
                assert_eq!(nearest_parent_distance, 1);
            }
        }
    }
    assert!(
        saw_full_restart,
        "The restart round must exercise large implicit draws"
    );
    assert!(report.statistics.proposals <= 1 + 3 * 16);
}
