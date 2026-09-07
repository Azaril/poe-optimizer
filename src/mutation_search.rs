//! Experimental, explicitly bounded normal-Mace mutation search.
use poe_optimizer_core::{
    candidate::{
        Candidate, CandidateBudgets, CandidateConstraints, CandidateDomain, SkillGroupLocks,
    },
    evaluation::{
        BackendIdentity, CalculationBackend, Engine, EvaluationBudget, EvaluationEngine,
        EvaluationRequest,
    },
    metrics::MetricQuery,
    objective::{ObjectiveSpec, ScoringPolicy},
    options::EvaluationOptions,
};
#[cfg(feature = "pob")]
use poe_optimizer_import::controlled_mace::VerifiedMaceScenario;
use poe_optimizer_import::{
    controlled_mace::{
        ControlledMaceCatalog, MaceSupportChoice, NormalMaceAlternative, VerifiedNativeMaceScenario,
    },
    decode_build,
};
use poe_optimizer_search::{discrete::*, *};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    io::{self, Write},
    path::PathBuf,
    sync::{Mutex, atomic::AtomicBool},
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, clap::ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Strategy {
    Exhaustive,
    Guided,
}
#[derive(clap::Args)]
pub(crate) struct Args {
    /// JSON problem containing a supported template, finite weapon/support choices and objective.
    #[arg(long)]
    problem: PathBuf,
    /// Native Rust calculation or optional PoB reference adapter.
    #[arg(long, value_enum, default_value_t = super::default_backend())]
    backend: super::BackendChoice,
    #[arg(long, default_value = "vendor/path-of-building-poe2")]
    pob: PathBuf,
    #[arg(long, default_value_t = 1)]
    jobs: usize,
    /// Total calculations, including one template and one reserved fresh finalist.
    #[arg(long, default_value_t = 130)]
    max_evaluations: usize,
    #[arg(long, default_value_t = 300)]
    timeout_seconds: u64,
    #[arg(long,value_enum,default_value_t=Strategy::Exhaustive)]
    strategy: Strategy,
    #[arg(long, default_value_t = 0)]
    seed: u64,
    #[arg(long, default_value_t = 64)]
    max_rounds: usize,
    #[arg(long, default_value_t = 4096)]
    max_proposals: usize,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long)]
    export: Option<PathBuf>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Problem {
    schema_version: u32,
    template: PathBuf,
    weapons: Vec<NormalMaceAlternative>,
    supports: Vec<MaceSupportChoice>,
    objective: ObjectiveSpec,
    #[serde(default)]
    locks: Locks,
    #[serde(default)]
    neighborhood: Neighborhood,
}
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
struct Locks {
    weapon_id: Option<String>,
    support: Option<MaceSupportChoice>,
}
struct Domain<'a> {
    registry: &'a ControlledMaceCatalog,
    rules: CandidateDomain,
    space: DiscreteSpace,
    weapons: Vec<String>,
    supports: Vec<MaceSupportChoice>,
    neighborhood: Neighborhood,
}
impl Domain<'_> {
    fn resolve(&self, point: &DiscretePoint) -> Result<&Candidate, String> {
        self.space.validate(point)?;
        self.registry
            .resolve_candidate(
                &self.weapons[point.choices[0] as usize],
                self.supports[point.choices[1] as usize],
            )
            .ok_or_else(|| "Unregistered complete candidate".into())
    }
}
impl SearchDomain<DiscretePoint> for Domain<'_> {
    fn can_propose_after_empty(&self) -> bool {
        self.space.size() != Some(1)
    }
    fn validate(
        &self,
        point: &DiscretePoint,
        control: &EvaluationControl<'_>,
    ) -> Result<(), String> {
        if control.should_stop() {
            return Err("Search stopped before validation".into());
        }
        let validation = self.rules.validate(self.resolve(point)?);
        if !validation.is_searchable() {
            return Err(format!("Candidate failed finite rules: {validation:?}"));
        }
        Ok(())
    }
    fn propose(
        &self,
        parents: &[DiscretePoint],
        round: usize,
        seed: u64,
        limit: usize,
        control: &EvaluationControl<'_>,
    ) -> Result<Vec<DiscretePoint>, String> {
        self.space
            .propose(parents, round, seed, limit, &self.neighborhood, control)
    }
}
type SearchEngine = Engine<Box<dyn CalculationBackend + Send + Sync>>;
enum Scenario {
    Native(VerifiedNativeMaceScenario),
    #[cfg(feature = "pob")]
    Pob(VerifiedMaceScenario),
}
struct Evaluator<'a> {
    domain: &'a Domain<'a>,
    engine: &'a SearchEngine,
    scenario: &'a Scenario,
    execution: ExecutionKind,
    identity: &'a BackendIdentity,
    metrics: Vec<MetricQuery>,
    warnings: Mutex<BTreeSet<String>>,
}
impl CandidateEvaluator<DiscretePoint> for Evaluator<'_> {
    fn execution_kind(&self) -> ExecutionKind {
        self.execution
    }
    fn evaluate(
        &self,
        point: &DiscretePoint,
        control: &EvaluationControl<'_>,
    ) -> Result<CandidateMeasurements, String> {
        if control.should_stop() {
            return Err("Search stopped before calculation".into());
        }
        let candidate = self.domain.resolve(point)?;
        let result = self
            .engine
            .evaluate(
                &EvaluationRequest {
                    build: self
                        .domain
                        .registry
                        .materialize(candidate)
                        .map_err(|e| e.to_string())?,
                    options: EvaluationOptions::default(),
                    metrics: self.metrics.clone(),
                },
                EvaluationBudget {
                    timeout_ms: control.remaining().min(Duration::from_secs(30)).as_millis() as u64,
                },
            )
            .map_err(|e| e.to_string())?;
        match self.scenario {
            Scenario::Native(scenario) => self
                .domain
                .registry
                .validate_native_realization(candidate, &result, scenario),
            #[cfg(feature = "pob")]
            Scenario::Pob(scenario) => self
                .domain
                .registry
                .validate_realization(candidate, &result, scenario),
        }
        .map_err(|error| error.to_string())?;
        if serde_json::to_value(&result.backend).map_err(|e| e.to_string())?
            != serde_json::to_value(self.identity).map_err(|e| e.to_string())?
        {
            return Err("Backend identity changed during search".into());
        }
        self.warnings
            .lock()
            .map_err(|_| "Warning lock poisoned")?
            .extend(result.warnings);
        Ok(CandidateMeasurements {
            measurements: result.measurements,
            diagnostic_only: result.diagnostic_only,
        })
    }
}

pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    if args.jobs == 0
        || args.max_evaluations < 3
        || args.timeout_seconds == 0
        || args.max_rounds == 0
        || args.max_proposals == 0
    {
        return Err("Require positive limits and at least three calculations (template, search, verification)".into());
    }
    let started = Instant::now();
    let deadline = started
        .checked_add(Duration::from_secs(args.timeout_seconds))
        .ok_or("Timeout is too large")?;
    for path in [&args.output, &args.export].into_iter().flatten() {
        if path.exists() {
            return Err(format!("Output already exists: {}", path.display()).into());
        }
    }
    let out = args
        .output
        .as_deref()
        .map(super::destination_identity)
        .transpose()?;
    let export = args
        .export
        .as_deref()
        .map(super::destination_identity)
        .transpose()?;
    if out.is_some() && out == export {
        return Err("JSON and XML outputs need different paths".into());
    }
    let problem = super::read_json::<Problem>(&args.problem, 512 * 1024)?;
    if problem.schema_version != 1 {
        return Err("Unsupported mutation problem schema".into());
    }
    problem.neighborhood.validate()?;
    let input = args
        .problem
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join(&problem.template);
    let imported = decode_build(&super::read_input(&input)?)?;
    let registry = ControlledMaceCatalog::new(
        imported.xml,
        problem.weapons.clone(),
        problem.supports.clone(),
    )?;
    let engine = Engine::new(super::make_backend(args.backend, args.pob.clone())?);
    let execution = match args.backend {
        super::BackendChoice::Native => ExecutionKind::RustCpu,
        #[cfg(feature = "pob")]
        super::BackendChoice::Pob => ExecutionKind::ExternalProcess,
    };
    let policy = problem.objective.compile(&engine.capabilities().metrics)?;
    let weapons: Vec<_> = registry
        .alternatives()
        .iter()
        .map(|a| a.weapon_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let supports: Vec<_> = registry
        .alternatives()
        .iter()
        .map(|a| a.support)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut locks = BTreeMap::new();
    if let Some(id) = &problem.locks.weapon_id {
        locks.insert(
            0,
            weapons
                .iter()
                .position(|v| v == id)
                .ok_or("Locked weapon is absent")? as u32,
        );
    }
    if let Some(support) = problem.locks.support {
        locks.insert(
            1,
            supports
                .iter()
                .position(|v| *v == support)
                .ok_or("Locked support is absent")? as u32,
        );
    }
    let mut constraints = CandidateConstraints {
        required_skill_ids: BTreeSet::from(["Melee1HMacePlayer".into()]),
        budgets: CandidateBudgets {
            active_skill_count: 1,
            supports_per_skill: 1,
            ..Default::default()
        },
        ..Default::default()
    };
    if let Some(id) = &problem.locks.weapon_id {
        let exemplar = registry
            .resolve_candidate(id, supports[0])
            .ok_or("Unknown locked weapon")?;
        let item = exemplar.equipment["Weapon 1"].clone();
        constraints.required_item_instance_ids.insert(item.clone());
        constraints
            .locks
            .equipment
            .insert("Weapon 1".into(), Some(item));
    }
    if let Some(support) = problem.locks.support {
        let exemplar = registry
            .resolve_candidate(&weapons[0], support)
            .ok_or("Unknown locked support")?;
        constraints.locks.skill_groups.insert(
            "pob-group-1".into(),
            SkillGroupLocks {
                exact_support_instance_ids: Some(
                    exemplar.skills["pob-group-1"].support_instance_ids.clone(),
                ),
                ..Default::default()
            },
        );
    }
    let id = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(
            "controlled-mace-axes-v1",
            &registry.catalog().identity,
            &weapons,
            &supports,
            &locks
        ))?)
    );
    let domain = Domain {
        registry: &registry,
        rules: CandidateDomain::new(registry.catalog().clone(), constraints.clone())?,
        space: DiscreteSpace::new(id, vec![weapons.len() as u32, supports.len() as u32], locks)?,
        weapons,
        supports,
        neighborhood: problem.neighborhood.clone(),
    };
    let plan = match args.strategy {
        Strategy::Exhaustive => SearchPlan::Finite(domain.space.enumerate(args.max_proposals)?),
        Strategy::Guided => SearchPlan::Explore(vec![domain.space.first()]),
    };
    let mut report = serde_json::json!({
        "schema_version":1,"status":"experimental_mutation_search","diagnostic_only":true,
        "requested_backend":engine.capabilities().id,"execution_kind":if execution == ExecutionKind::RustCpu {"rust_cpu"} else {"external_process"},
        "scope":"normal_mace_weapon_support_profile_v1","problem":problem,"template_xml_sha256":imported.sha256,
        "template":registry.template_build(),"catalog":registry.catalog(),"alternatives":registry.alternatives(),
        "candidate_constraints":constraints,"space":domain.space,"strategy":args.strategy,"neighborhood":domain.neighborhood,
        "run_budget":{"max_evaluations":args.max_evaluations,"timeout_seconds":args.timeout_seconds,"jobs":args.jobs,"seed":args.seed,"max_proposals":args.max_proposals,"max_rounds":args.max_rounds,"reserved_template_attempts":1,"reserved_verification_attempts":1},
        "preparation":{"attempts":0},"search":null,"best_verified":null,"export":{"status":"not_written"},"total_evaluations":0
    });
    if Instant::now() >= deadline {
        report["termination"] = serde_json::json!("time_budget");
        report["elapsed_ms"] = serde_json::json!(started.elapsed().as_secs_f64() * 1000.);
        return emit(&args, report);
    }
    report["preparation"]["attempts"] = serde_json::json!(1);
    report["total_evaluations"] = serde_json::json!(1);
    let baseline = engine.evaluate(
        &EvaluationRequest {
            build: registry.template_build(),
            options: EvaluationOptions::default(),
            metrics: policy.required_metrics(),
        },
        EvaluationBudget {
            timeout_ms: deadline
                .saturating_duration_since(Instant::now())
                .min(Duration::from_secs(30))
                .as_millis() as u64,
        },
    );
    let mut baseline = match baseline {
        Ok(result) => result,
        Err(error) => {
            report["preparation"]["error"] = serde_json::json!(error.to_string());
            report["termination"] = serde_json::json!("preparation_failed");
            report["elapsed_ms"] = serde_json::json!(started.elapsed().as_secs_f64() * 1000.);
            return emit(&args, report);
        }
    };
    let scenario = match args.backend {
        super::BackendChoice::Native => registry
            .bind_native_baseline(&baseline, &poe_optimizer_native::backend_identity())
            .map(Scenario::Native),
        #[cfg(feature = "pob")]
        super::BackendChoice::Pob => registry.bind_baseline(&baseline).map(Scenario::Pob),
    };
    baseline.attachments.clear();
    report["preparation"]["evaluation"] = serde_json::to_value(&baseline)?;
    let scenario = match scenario {
        Ok(value) => value,
        Err(error) => {
            report["preparation"]["error"] = serde_json::json!(error.to_string());
            report["termination"] = serde_json::json!("preparation_failed");
            report["elapsed_ms"] = serde_json::json!(started.elapsed().as_secs_f64() * 1000.);
            return emit(&args, report);
        }
    };
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        report["termination"] = serde_json::json!("time_budget");
        report["elapsed_ms"] = serde_json::json!(started.elapsed().as_secs_f64() * 1000.);
        return emit(&args, report);
    }
    let evaluator = Evaluator {
        domain: &domain,
        engine: &engine,
        scenario: &scenario,
        execution,
        identity: &baseline.backend,
        metrics: policy.required_metrics(),
        warnings: Mutex::new(BTreeSet::new()),
    };
    let budget = SearchBudget {
        max_evaluations: args.max_evaluations - 1,
        max_proposals: args.max_proposals,
        max_rounds: args.max_rounds,
        duration: remaining,
        jobs: args.jobs,
        beam_per_status: 8,
        archive_size: 10,
        verification_attempts: 1,
        seed: args.seed,
    };
    let result = search(
        &domain,
        &evaluator,
        &policy,
        plan,
        &budget,
        &AtomicBool::new(false),
    )?;
    report["total_evaluations"] = serde_json::json!(1 + result.statistics.evaluations);
    report["elapsed_ms"] = serde_json::json!(started.elapsed().as_secs_f64() * 1000.);
    report["termination"] = serde_json::to_value(result.termination)?;
    report["warnings"] = serde_json::to_value(
        &*evaluator
            .warnings
            .lock()
            .map_err(|_| "Warning lock poisoned")?,
    )?;
    let best = result.feasible.first().filter(|best| {
        result
            .verifications
            .iter()
            .any(|v| v.candidate == best.candidate && v.consistent)
    });
    if let Some(best) = best {
        let candidate = domain.resolve(&best.candidate)?;
        let source = registry.materialize(candidate)?;
        let alternative = registry
            .alternatives()
            .iter()
            .find(|a| &a.candidate == candidate)
            .ok_or("Missing finalist source")?;
        report["best_verified"] = serde_json::json!({"alternative_id":alternative.id,"candidate":candidate,"source_xml_sha256":alternative.xml_sha256,"assessment":best.assessment,"diagnostic_only":true});
        if let Some(path) = &args.export {
            super::write_new(path, source.content.as_bytes())?;
            report["export"] =
                serde_json::json!({"status":"written","source":"materialized_xml","path":path});
        }
    }
    if args.export.is_none() {
        report["export"] = serde_json::json!({"status":"not_requested"});
    } else if best.is_none() {
        report["export"]["reason"] = serde_json::json!(
            "No feasible best candidate passed fresh verification within the shared budget"
        );
    }
    report["search"] = serde_json::to_value(result)?;
    emit(&args, report)
}
fn emit(args: &Args, report: serde_json::Value) -> Result<(), Box<dyn Error>> {
    let mut bytes = serde_json::to_vec_pretty(&report)?;
    bytes.push(b'\n');
    if let Some(path) = &args.output {
        super::write_new(path, &bytes)?;
    } else {
        io::stdout().write_all(&bytes)?;
    }
    Ok(())
}
