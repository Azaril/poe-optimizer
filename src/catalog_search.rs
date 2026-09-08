//! Developer calibration search over the four immutable Mace fixtures.
use poe_optimizer_core::{
    candidate::{Candidate, CandidateBudgets, CandidateConstraints, CandidateDomain},
    evaluation::{BackendIdentity, Engine, EvaluationBudget, EvaluationEngine, EvaluationRequest},
    metrics::MetricQuery,
    objective::{ObjectiveSpec, ScoringPolicy},
    options::EvaluationOptions,
};
use poe_optimizer_pob::{
    backend::PobBackend,
    candidate::{PobBuildAlternative, PobCandidateCatalog},
};
use poe_optimizer_search::{
    CandidateEvaluator, CandidateMeasurements, EvaluationControl, ExecutionKind, SearchBudget,
    SearchDomain, SearchPlan,
};
use std::{
    collections::BTreeSet,
    error::Error,
    io::{self, Write},
    path::PathBuf,
    sync::{Mutex, atomic::AtomicBool},
    time::Duration,
};

const FIXTURES: &[(&str, &str)] = &[
    (
        "mace-wooden",
        include_str!("../tests/fixtures/calibration/mace-wooden.xml"),
    ),
    (
        "mace-wooden-brutality",
        include_str!("../tests/fixtures/calibration/mace-wooden-brutality.xml"),
    ),
    (
        "mace-smithing",
        include_str!("../tests/fixtures/calibration/mace-smithing.xml"),
    ),
    (
        "mace-smithing-brutality",
        include_str!("../tests/fixtures/calibration/mace-smithing-brutality.xml"),
    ),
];

#[derive(clap::Args)]
pub(crate) struct Args {
    /// Typed scalar objective and constraints; this command uses only four calibration builds.
    #[arg(long)]
    objective: PathBuf,
    #[arg(long, default_value = "vendor/path-of-building-poe2")]
    pob: PathBuf,
    /// Maximum concurrent isolated calculation workers.
    #[arg(long, default_value_t = 1, value_parser = positive_usize)]
    jobs: usize,
    /// Includes the one attempt reserved for fresh finalist verification; minimum two.
    #[arg(long, default_value_t = 5, value_parser = evaluation_limit)]
    max_evaluations: usize,
    /// Shared search and finalist-verification deadline, excluding input preparation.
    #[arg(long, default_value_t = 60, value_parser = positive_u64)]
    timeout_seconds: u64,
    /// Write the report to a new file instead of stdout.
    #[arg(long)]
    output: Option<PathBuf>,
    /// Save exact source XML only when the best feasible candidate passes fresh verification.
    #[arg(long)]
    export: Option<PathBuf>,
}

fn positive_usize(value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .ok()
        .filter(|number| *number > 0)
        .ok_or_else(|| "value must be a positive integer".into())
}
fn evaluation_limit(value: &str) -> Result<usize, String> {
    positive_usize(value).and_then(|number| {
        if number >= 2 {
            Ok(number)
        } else {
            Err("at least two evaluations are required, including fresh verification".into())
        }
    })
}
fn positive_u64(value: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .ok()
        .filter(|number| *number > 0)
        .ok_or_else(|| "value must be a positive integer".into())
}

struct FixtureDomain<'a> {
    candidate_domain: CandidateDomain,
    registry: &'a PobCandidateCatalog,
}
impl SearchDomain<Candidate> for FixtureDomain<'_> {
    fn validate(
        &self,
        candidate: &Candidate,
        control: &EvaluationControl<'_>,
    ) -> Result<(), String> {
        if control.should_stop() {
            return Err("Search stopped before candidate validation".into());
        }
        let validation = self.candidate_domain.validate(candidate);
        if !validation.is_searchable() {
            return Err(format!(
                "Candidate failed supplied finite rules: {validation:?}"
            ));
        }
        self.registry
            .materialize(candidate)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}

struct CalibrationEvaluator<'a> {
    registry: &'a PobCandidateCatalog,
    engine: Engine<PobBackend>,
    metrics: Vec<MetricQuery>,
    identity: Mutex<Option<BackendIdentity>>,
    warnings: Mutex<BTreeSet<String>>,
}

fn same_identity(left: &BackendIdentity, right: &BackendIdentity) -> bool {
    left == right
}

impl CandidateEvaluator<Candidate> for CalibrationEvaluator<'_> {
    fn execution_kind(&self) -> ExecutionKind {
        ExecutionKind::ExternalProcess
    }

    fn evaluate(
        &self,
        candidate: &Candidate,
        control: &EvaluationControl<'_>,
    ) -> Result<CandidateMeasurements, String> {
        if control.should_stop() {
            return Err("Search stopped before calculation".into());
        }
        let timeout_ms = control.remaining().min(Duration::from_secs(30)).as_millis() as u64;
        if timeout_ms == 0 {
            return Err("Search deadline has less than one millisecond remaining".into());
        }
        let build = self
            .registry
            .materialize(candidate)
            .map_err(|error| error.to_string())?;
        let result = self
            .engine
            .evaluate(
                &EvaluationRequest {
                    build,
                    options: EvaluationOptions::default(),
                    metrics: self.metrics.clone(),
                },
                EvaluationBudget { timeout_ms },
            )
            .map_err(|error| error.to_string())?;
        self.registry
            .validate_realization(candidate, &result)
            .map_err(|error| error.to_string())?;
        {
            let mut recorded = self
                .identity
                .lock()
                .map_err(|_| "Backend identity lock was poisoned")?;
            if let Some(identity) = recorded.as_ref() {
                if !same_identity(identity, &result.backend) {
                    return Err(
                        "Backend identity changed during the run; measurements were rejected"
                            .into(),
                    );
                }
            } else {
                *recorded = Some(result.backend.clone());
            }
        }
        self.warnings
            .lock()
            .map_err(|_| "Warning lock was poisoned")?
            .extend(result.warnings);
        Ok(CandidateMeasurements {
            measurements: result.measurements,
            diagnostic_only: result.diagnostic_only,
        })
    }
}

pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    for path in [&args.output, &args.export].into_iter().flatten() {
        if path.exists() {
            return Err(format!("Output already exists: {}", path.display()).into());
        }
    }
    let output_identity = args
        .output
        .as_deref()
        .map(super::destination_identity)
        .transpose()?;
    let export_identity = args
        .export
        .as_deref()
        .map(super::destination_identity)
        .transpose()?;
    if output_identity.is_some() && output_identity == export_identity {
        return Err("JSON and XML outputs need different paths".into());
    }
    let objective = super::read_json::<ObjectiveSpec>(&args.objective, 64 * 1024)?;
    let engine = Engine::new(PobBackend::new(std::env::current_exe()?, args.pob));
    let policy = objective.compile(&engine.capabilities().metrics)?;
    let registry = PobCandidateCatalog::from_builds(
        FIXTURES
            .iter()
            .map(|(id, xml)| PobBuildAlternative {
                id: (*id).into(),
                xml: (*xml).into(),
            })
            .collect(),
    )?;
    let constraints = CandidateConstraints {
        required_skill_ids: BTreeSet::from(["Melee1HMacePlayer".into()]),
        budgets: CandidateBudgets {
            active_skill_count: 1,
            supports_per_skill: 1,
            ..Default::default()
        },
        ..Default::default()
    };
    let domain = FixtureDomain {
        candidate_domain: CandidateDomain::new(registry.catalog().clone(), constraints.clone())?,
        registry: &registry,
    };
    let evaluator = CalibrationEvaluator {
        registry: &registry,
        engine,
        metrics: policy.required_metrics(),
        identity: Mutex::new(None),
        warnings: Mutex::new(BTreeSet::new()),
    };
    let budget = SearchBudget {
        max_evaluations: args.max_evaluations,
        max_proposals: FIXTURES.len(),
        max_rounds: 1,
        duration: Duration::from_secs(args.timeout_seconds),
        jobs: args.jobs,
        beam_per_status: FIXTURES.len(),
        archive_size: FIXTURES.len(),
        verification_attempts: 1,
        seed: 0,
    };
    let search = poe_optimizer_search::search(
        &domain,
        &evaluator,
        &policy,
        SearchPlan::Finite(
            registry
                .alternatives()
                .iter()
                .map(|alternative| alternative.candidate.clone())
                .collect(),
        ),
        &budget,
        &AtomicBool::new(false),
    )?;
    let best = search.feasible.first().filter(|entry| {
        search.verifications.iter().any(|verification| {
            verification.candidate == entry.candidate && verification.consistent
        })
    });
    let verified = best.map(|entry| {
        let alternative = registry
            .alternatives()
            .iter()
            .find(|alternative| alternative.candidate == entry.candidate)
            .expect("search returns only registered candidates");
        serde_json::json!({
            "alternative_id": alternative.id,
            "xml_sha256": alternative.xml_sha256,
            "candidate": entry.candidate,
            "assessment": entry.assessment,
            "diagnostic_only": true,
        })
    });
    let export_document = best
        .map(|entry| registry.materialize(&entry.candidate))
        .transpose()?;
    let export_status = if args.export.is_none() {
        serde_json::json!({"status": "not_requested"})
    } else if export_document.is_some() {
        serde_json::json!({"status": "written", "format": "path_of_building2_xml", "source": "exact_materialized_source"})
    } else {
        serde_json::json!({"status": "not_written", "reason": "No feasible best candidate passed fresh verification within the shared budget"})
    };
    let report = serde_json::json!({
        "schema_version": 1,
        "status": "experimental_calibration_search",
        "scope": "four_calibrated_mace_fixture_alternatives",
        "diagnostic_only": true,
        "objective": objective,
        "candidate_constraints": constraints,
        "catalog": registry.catalog(),
        "alternatives": registry.alternatives(),
        "backend": *evaluator.identity.lock().map_err(|_| "Backend identity lock was poisoned")?,
        "warnings": *evaluator.warnings.lock().map_err(|_| "Warning lock was poisoned")?,
        "worker_timeout_cap_ms": 30_000,
        "scenario": "Imported normal-enemy mapping fixture settings; no encounter override",
        "search": search,
        "best_verified": verified,
        "export": export_status,
    });
    let mut json = serde_json::to_vec_pretty(&report)?;
    json.push(b'\n');
    // An infeasible/partial search still produces useful evidence. No XML is emitted
    // unless the reserved fresh run verified the currently highest feasible entry.
    if let Some(path) = args.export
        && let Some(document) = export_document
    {
        super::write_new(&path, document.content.as_bytes())?;
    }
    if let Some(path) = args.output {
        super::write_new(&path, &json)?;
    } else {
        io::stdout().write_all(&json)?;
    }
    Ok(())
}
