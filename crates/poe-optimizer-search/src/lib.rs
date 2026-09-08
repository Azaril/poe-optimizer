//! Bounded host-side search. Candidate legality, proposals and calculation are replaceable.
//! CPU evaluations use a local Rayon pool; process-supervision waits use scoped OS threads.

pub mod discrete;

use poe_optimizer_core::{metrics::MetricMeasurement, objective::*};
use rayon::prelude::*;
use serde::Serialize;
use std::{
    cmp::Ordering,
    collections::BTreeSet,
    sync::atomic::{AtomicBool, Ordering as AtomicOrdering},
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Serialize)]
pub struct SearchBudget {
    /// Includes failed calculations and reserved fresh verification attempts.
    pub max_evaluations: usize,
    pub max_proposals: usize,
    pub max_rounds: usize,
    pub duration: Duration,
    pub jobs: usize,
    pub beam_per_status: usize,
    pub archive_size: usize,
    pub verification_attempts: usize,
    pub seed: u64,
}
impl Default for SearchBudget {
    fn default() -> Self {
        Self {
            max_evaluations: 1000,
            max_proposals: 100_000,
            max_rounds: 1000,
            duration: Duration::from_secs(300),
            jobs: 1,
            beam_per_status: 8,
            archive_size: 10,
            verification_attempts: 1,
            seed: 0,
        }
    }
}

/// A complete finite catalog, or initial states for a heuristic domain-specific proposer.
/// Finite means exactly the supplied list, never the entire game's search space.
pub enum SearchPlan<C> {
    Finite(Vec<C>),
    Explore(Vec<C>),
}

pub trait SearchDomain<C>: Sync {
    /// Optional compact identity for domains whose candidates own prepared data.
    /// It must be stable, complete, at most256bytes and equivalent to candidate
    /// equality throughout this run. None preserves the original Ord identity.
    /// Only keys are retained for deduplication; prepared inputs can leave memory
    /// when they leave the current batch and bounded archives.
    fn deduplication_key(&self, _candidate: &C) -> Option<Vec<u8>> {
        None
    }

    /// Stochastic proposers may have empty samples before later radii/restarts.
    /// Round and duration limits still bound these retries.
    fn can_propose_after_empty(&self) -> bool {
        false
    }
    /// Reject illegal, locked-out or unsupported states before spending a calculation.
    fn validate(&self, candidate: &C, control: &EvaluationControl<'_>) -> Result<(), String>;
    /// Generate complete states, including coupled mutations/restarts when useful.
    /// `parents` retains both feasible and infeasible exploration states.
    /// Return at most `limit` proposals; a larger response is a contract error.
    fn propose(
        &self,
        _parents: &[C],
        _round: usize,
        _seed: u64,
        _limit: usize,
        _control: &EvaluationControl<'_>,
    ) -> Result<Vec<C>, String> {
        Ok(Vec::new())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionKind {
    RustCpu,
    ExternalProcess,
}

pub struct EvaluationControl<'a> {
    deadline: Instant,
    cancelled: &'a AtomicBool,
}
impl EvaluationControl<'_> {
    pub fn remaining(&self) -> Duration {
        self.deadline.saturating_duration_since(Instant::now())
    }
    pub fn should_stop(&self) -> bool {
        self.cancelled.load(AtomicOrdering::Relaxed) || self.remaining().is_zero()
    }
}

#[derive(Debug, Clone)]
pub struct CandidateMeasurements {
    pub measurements: Vec<MetricMeasurement>,
    /// Retained through selection/verification; search cannot upgrade coverage evidence.
    pub diagnostic_only: bool,
}

pub trait CandidateEvaluator<C>: Sync {
    fn execution_kind(&self) -> ExecutionKind;
    /// Every call must compute fresh (no hidden cache), including repeated finalists.
    /// Must honor `control`. External backends must enforce process deadlines themselves.
    /// The adapter also checks requested-versus-realized candidate state before returning.
    fn evaluate(
        &self,
        candidate: &C,
        control: &EvaluationControl<'_>,
    ) -> Result<CandidateMeasurements, String>;

    /// Independently compute a fresh finalist under the reserved evaluation budget.
    /// The default preserves adapters whose search and verification paths are identical.
    /// Adapters with a prepared search path may override this with a complete realization
    /// and evaluation path. It must honor the same control and must not reuse a cached
    /// search result. Search compares its returned evidence with the ranked candidate.
    fn verify(
        &self,
        candidate: &C,
        control: &EvaluationControl<'_>,
    ) -> Result<CandidateMeasurements, String> {
        self.evaluate(candidate, control)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RankedCandidate<C> {
    pub candidate: C,
    pub assessment: ObjectiveAssessment,
    pub diagnostic_only: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct Verification<C> {
    pub candidate: C,
    pub consistent: bool,
    pub diagnostic_only: bool,
    pub fresh_assessment: Option<ObjectiveAssessment>,
    pub error: Option<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Termination {
    FiniteDomainProcessed,
    EvaluationBudget,
    ProposalBudget,
    RoundBudget,
    TimeBudget,
    Cancelled,
    SearchStalled,
}
#[derive(Debug, Default, Serialize)]
pub struct SearchStatistics {
    pub proposals: usize,
    pub duplicates: usize,
    pub rejected: usize,
    pub evaluations: usize,
    pub evaluation_failures: usize,
    pub unavailable: usize,
    pub discarded_late: usize,
    pub verification_evaluations: usize,
    pub rounds: usize,
}
#[derive(Debug, Serialize)]
pub struct SearchReport<C> {
    pub schema_version: u32,
    pub budget: SearchBudget,
    pub termination: Termination,
    pub seed: u64,
    pub jobs: usize,
    pub elapsed_ms: f64,
    pub statistics: SearchStatistics,
    pub feasible: Vec<RankedCandidate<C>>,
    pub infeasible: Vec<RankedCandidate<C>>,
    pub verifications: Vec<Verification<C>>,
    /// Bounded samples; aggregate counts above include all failures.
    pub errors: Vec<String>,
}

fn ranking<C: Ord>(a: &RankedCandidate<C>, b: &RankedCandidate<C>) -> Ordering {
    // Only finite, available assessments enter archives.
    a.assessment
        .total_normalized_violation
        .finite()
        .unwrap()
        .total_cmp(&b.assessment.total_normalized_violation.finite().unwrap())
        .then_with(|| {
            a.assessment
                .constraints
                .iter()
                .filter(|c| c.status == ConstraintStatus::Violated)
                .count()
                .cmp(
                    &b.assessment
                        .constraints
                        .iter()
                        .filter(|c| c.status == ConstraintStatus::Violated)
                        .count(),
                )
        })
        .then_with(|| {
            b.assessment
                .objective_score
                .finite()
                .unwrap()
                .total_cmp(&a.assessment.objective_score.finite().unwrap())
        })
        .then_with(|| a.candidate.cmp(&b.candidate))
}
fn sample(errors: &mut Vec<String>, error: String) {
    if errors.len() < 32 {
        errors.push(error);
    }
}
fn stopped(control: &EvaluationControl<'_>) -> Option<Termination> {
    if control.cancelled.load(AtomicOrdering::Relaxed) {
        Some(Termination::Cancelled)
    } else if control.remaining().is_zero() {
        Some(Termination::TimeBudget)
    } else {
        None
    }
}
fn next_seed(seed: &mut u64) -> u64 {
    *seed = seed.wrapping_add(0x9e3779b97f4a7c15);
    let mut n = *seed;
    n = (n ^ (n >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    n = (n ^ (n >> 27)).wrapping_mul(0x94d049bb133111eb);
    n ^ (n >> 31)
}

/// Deterministic batch integration, with per-run deduplication and separate status beams.
/// Wall-clock cutoffs and backend nondeterminism can change the evaluated prefix.
/// Cancellation is cooperative; scoped calls are always joined before this returns.
pub fn search<C, D, E, P>(
    domain: &D,
    evaluator: &E,
    policy: &P,
    plan: SearchPlan<C>,
    budget: &SearchBudget,
    cancelled: &AtomicBool,
) -> Result<SearchReport<C>, String>
where
    C: Clone + Ord + Send + Sync,
    D: SearchDomain<C>,
    E: CandidateEvaluator<C>,
    P: ScoringPolicy + Sync,
{
    if budget.max_evaluations == 0
        || budget.max_proposals == 0
        || budget.max_rounds == 0
        || budget.jobs == 0
        || budget.beam_per_status == 0
        || budget.archive_size == 0
        || budget.duration.is_zero()
        || budget.verification_attempts >= budget.max_evaluations
    {
        return Err("Search limits must be positive; reserve fewer verification attempts than total evaluations".into());
    }
    let started = Instant::now();
    let deadline = started
        .checked_add(budget.duration)
        .ok_or("Search duration is too large")?;
    let control = EvaluationControl {
        deadline,
        cancelled,
    };
    let pool = if evaluator.execution_kind() == ExecutionKind::RustCpu {
        Some(
            rayon::ThreadPoolBuilder::new()
                .num_threads(budget.jobs.min(budget.max_evaluations))
                .build()
                .map_err(|e| e.to_string())?,
        )
    } else {
        None
    };
    let finite = matches!(plan, SearchPlan::Finite(_));
    let mut proposals = match plan {
        SearchPlan::Finite(c) | SearchPlan::Explore(c) => c,
    };
    if proposals.len() > budget.max_proposals {
        return Err("Initial plan exceeds proposal budget".into());
    }
    let mut report = SearchReport {
        schema_version: 1,
        budget: budget.clone(),
        termination: Termination::SearchStalled,
        seed: budget.seed,
        jobs: budget.jobs,
        elapsed_ms: 0.0,
        statistics: SearchStatistics::default(),
        feasible: Vec::new(),
        infeasible: Vec::new(),
        verifications: Vec::new(),
        errors: Vec::new(),
    };
    let mut seen = BTreeSet::new();
    let mut seen_keys = BTreeSet::new();
    let mut seed = budget.seed;
    let search_limit = budget.max_evaluations - budget.verification_attempts;
    let keep = budget.archive_size.max(budget.beam_per_status);
    'rounds: loop {
        if let Some(reason) = stopped(&control) {
            report.termination = reason;
            break;
        }
        if report.statistics.rounds >= budget.max_rounds {
            report.termination = Termination::RoundBudget;
            break;
        }
        report.statistics.rounds += 1;
        // Ordering is independent of worker assignment, including tie breaking and deduplication.
        proposals.sort();
        let mut eligible = Vec::new();
        for candidate in proposals.drain(..) {
            if report.statistics.proposals >= budget.max_proposals {
                report.termination = Termination::ProposalBudget;
                break 'rounds;
            }
            report.statistics.proposals += 1;
            let fresh = if let Some(key) = domain.deduplication_key(&candidate) {
                if key.is_empty() || key.len() > 256 {
                    return Err("Candidate deduplication key must contain1..256bytes".into());
                }
                seen_keys.insert(key)
            } else {
                seen.insert(candidate.clone())
            };
            if !fresh {
                report.statistics.duplicates += 1;
                continue;
            }
            if let Some(reason) = stopped(&control) {
                report.termination = reason;
                break 'rounds;
            }
            match domain.validate(&candidate, &control) {
                Ok(()) => eligible.push(candidate),
                Err(error) => {
                    report.statistics.rejected += 1;
                    sample(&mut report.errors, error);
                }
            }
        }
        for chunk in eligible.chunks(budget.jobs) {
            if let Some(reason) = stopped(&control) {
                report.termination = reason;
                break 'rounds;
            }
            let available = search_limit - report.statistics.evaluations;
            if available == 0 {
                report.termination = Termination::EvaluationBudget;
                break 'rounds;
            }
            let truncated = chunk.len() > available;
            let chunk = &chunk[..chunk.len().min(available)];
            let run = |candidate: &C| {
                if control.should_stop() {
                    return None;
                }
                let value = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    evaluator.evaluate(candidate, &control)
                }))
                .unwrap_or_else(|_| Err("Candidate evaluator panicked".into()));
                Some((value, control.should_stop()))
            };
            let results: Vec<_> = if let Some(pool) = &pool {
                pool.install(|| chunk.par_iter().map(run).collect())
            } else {
                std::thread::scope(|scope| {
                    let threads: Vec<_> = chunk
                        .iter()
                        .map(|c| std::thread::Builder::new().spawn_scoped(scope, move || run(c)))
                        .collect();
                    threads
                        .into_iter()
                        .map(|t| match t {
                            Ok(t) => t.join().unwrap_or_else(|_| {
                                Some((
                                    Err("Evaluator thread panicked".into()),
                                    control.should_stop(),
                                ))
                            }),
                            Err(error) => Some((
                                Err(format!("Cannot start evaluator thread: {error}")),
                                control.should_stop(),
                            )),
                        })
                        .collect()
                })
            };
            for (candidate, result) in chunk.iter().zip(results) {
                let Some((result, late)) = result else {
                    continue;
                };
                report.statistics.evaluations += 1;
                if late {
                    report.statistics.discarded_late += 1;
                    continue;
                }
                let measurements = match result {
                    Ok(value) => value,
                    Err(error) => {
                        report.statistics.evaluation_failures += 1;
                        sample(&mut report.errors, error);
                        continue;
                    }
                };
                let assessment = match policy.assess(&measurements.measurements) {
                    Ok(value) => value,
                    Err(error) => {
                        report.statistics.evaluation_failures += 1;
                        sample(&mut report.errors, error.to_string());
                        continue;
                    }
                };
                if !valid_assessment_numbers(&assessment)
                    || assessment.status == AssessmentStatus::Unavailable
                    || assessment.objective_score.finite().is_none()
                    || assessment.total_normalized_violation.finite().is_none()
                {
                    report.statistics.unavailable += 1;
                    continue;
                }
                let entry = RankedCandidate {
                    candidate: candidate.clone(),
                    assessment,
                    diagnostic_only: measurements.diagnostic_only,
                };
                let archive = if entry.assessment.status == AssessmentStatus::ConstraintsSatisfied {
                    &mut report.feasible
                } else {
                    &mut report.infeasible
                };
                archive.push(entry);
                archive.sort_by(ranking);
                archive.truncate(keep);
            }
            if truncated {
                report.termination = stopped(&control).unwrap_or(Termination::EvaluationBudget);
                break 'rounds;
            }
        }
        if let Some(reason) = stopped(&control) {
            report.termination = reason;
            break;
        }
        // A finite supplied domain is processed only if every eligible state got an attempt.
        // When the last batch exactly meets the budget, completing the list still counts.
        if finite {
            report.termination = Termination::FiniteDomainProcessed;
            break;
        }
        if report.statistics.evaluations >= search_limit {
            report.termination = Termination::EvaluationBudget;
            break;
        }
        let remaining = budget.max_proposals - report.statistics.proposals;
        if remaining == 0 {
            report.termination = Termination::ProposalBudget;
            break;
        }
        let parents: Vec<_> = report
            .feasible
            .iter()
            .take(budget.beam_per_status)
            .chain(report.infeasible.iter().take(budget.beam_per_status))
            .map(|c| c.candidate.clone())
            .collect();
        if report.statistics.rounds >= budget.max_rounds {
            report.termination = Termination::RoundBudget;
            break;
        }
        if let Some(reason) = stopped(&control) {
            report.termination = reason;
            break;
        }
        let generated = domain.propose(
            &parents,
            report.statistics.rounds,
            next_seed(&mut seed),
            remaining,
            &control,
        );
        if let Some(reason) = stopped(&control) {
            report.termination = reason;
            break;
        }
        proposals = generated?;
        if proposals.len() > remaining {
            return Err("Proposal generator exceeded its limit".into());
        }
        if proposals.is_empty() && !domain.can_propose_after_empty() {
            report.termination = Termination::SearchStalled;
            break;
        }
    }
    report.feasible.truncate(budget.archive_size);
    report.infeasible.truncate(budget.archive_size);
    for entry in report.feasible.iter().take(budget.verification_attempts) {
        if control.should_stop() {
            break;
        }
        report.statistics.evaluations += 1;
        report.statistics.verification_evaluations += 1;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            evaluator.verify(&entry.candidate, &control)
        }))
        .unwrap_or_else(|_| Err("Candidate evaluator panicked".into()));
        let mut verification = Verification {
            candidate: entry.candidate.clone(),
            consistent: false,
            diagnostic_only: entry.diagnostic_only,
            fresh_assessment: None,
            error: None,
        };
        if control.should_stop() {
            report.statistics.discarded_late += 1;
            verification.error =
                Some("Fresh verification exceeded deadline or was cancelled".into());
        } else {
            match result {
                Ok(measurements) => {
                    verification.diagnostic_only |= measurements.diagnostic_only;
                    match policy.assess(&measurements.measurements) {
                        Ok(assessment) => {
                            verification.consistent =
                                same_assessment(&entry.assessment, &assessment);
                            verification.fresh_assessment = Some(assessment);
                            if !verification.consistent {
                                verification.error =
                                    Some("Fresh measurements differ from search evidence".into());
                            }
                        }
                        Err(error) => verification.error = Some(error.to_string()),
                    }
                }
                Err(error) => verification.error = Some(error),
            }
        }
        if let Some(error) = &verification.error {
            report.statistics.evaluation_failures += 1;
            sample(&mut report.errors, error.clone());
        }
        report.verifications.push(verification);
    }
    report.elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    Ok(report)
}

fn valid_assessment_numbers(a: &ObjectiveAssessment) -> bool {
    use poe_optimizer_core::metrics::MeasurementValue;
    let valid = |value: &MeasurementValue| !matches!(value, MeasurementValue::Finite {value} if !value.is_finite());
    valid(&a.objective_value)
        && valid(&a.objective_score)
        && valid(&a.total_normalized_violation)
        && a.measurements.iter().all(|m| valid(&m.value))
        && a.constraints.iter().all(|c| {
            valid(&c.observed)
                && c.shortfall.as_ref().is_none_or(valid)
                && c.normalized_violation.as_ref().is_none_or(valid)
        })
}
fn same_assessment(a: &ObjectiveAssessment, b: &ObjectiveAssessment) -> bool {
    if !valid_assessment_numbers(a)
        || !valid_assessment_numbers(b)
        || a.status != b.status
        || b.status != AssessmentStatus::ConstraintsSatisfied
        || a.schema_version != b.schema_version
        || a.objective_score.finite().is_none()
        || b.objective_score.finite().is_none()
        || a.total_normalized_violation.finite().is_none()
        || b.total_normalized_violation.finite().is_none()
        || a.measurements.iter().any(|m| m.value.finite().is_none())
        || b.measurements.iter().any(|m| m.value.finite().is_none())
        || a.measurements
            .iter()
            .map(|m| &m.query)
            .collect::<BTreeSet<_>>()
            .len()
            != a.measurements.len()
        || b.measurements
            .iter()
            .map(|m| &m.query)
            .collect::<BTreeSet<_>>()
            .len()
            != b.measurements.len()
    {
        return false;
    }
    if a.measurements.len() != b.measurements.len()
        || a.constraints.len() != b.constraints.len()
        || !a.measurements.iter().zip(&b.measurements).all(|(a, b)| {
            a.query == b.query && a.unit == b.unit && a.schema_version == b.schema_version
        })
        || !a.constraints.iter().zip(&b.constraints).all(|(a, b)| {
            serde_json::to_value(&a.constraint).ok() == serde_json::to_value(&b.constraint).ok()
        })
    {
        return false;
    }
    let (Ok(mut left), Ok(mut right)) = (serde_json::to_value(a), serde_json::to_value(b)) else {
        return false;
    };
    if left["specification"] != right["specification"] {
        return false;
    }
    // Exact policy metadata; tolerance applies only to the calculated evidence below.
    left.as_object_mut().unwrap().remove("specification");
    right.as_object_mut().unwrap().remove("specification");
    // Order is a part of the scoring-policy contract, including measurement/constraint order.
    same_evidence(&left, &right)
}
fn same_evidence(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    use serde_json::Value;
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => match (a.as_f64(), b.as_f64()) {
            (Some(a), Some(b)) => {
                a.is_finite()
                    && b.is_finite()
                    && (a - b).abs() <= 1e-8_f64.max(1e-9 * a.abs().max(b.abs()))
            }
            _ => false,
        },
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| same_evidence(a, b))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(k, a)| b.get(k).is_some_and(|b| same_evidence(a, b)))
        }
        _ => a == b,
    }
}
