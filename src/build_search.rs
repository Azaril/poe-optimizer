//! Native graph search with component preparation, independent locks and fresh verification.
#[path = "build_search_seeds.rs"]
mod seeds;
use poe_optimizer_core::{
    candidate::*,
    evaluation::*,
    metrics::MetricQuery,
    objective::{ObjectiveSpec, ScoringPolicy},
    options::EvaluationOptions,
};
use poe_optimizer_data::class_tree::{AttributeOption, PassiveAllocationSelection};
use poe_optimizer_import::controlled_build::*;
use poe_optimizer_native::{NativeBackend, PreparedBuildCandidates};
use poe_optimizer_search::*;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    io::{self, Write},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
#[derive(clap::Args)]
pub(crate) struct Args {
    #[arg(long)]
    problem: PathBuf,
    #[command(flatten)]
    data: super::data_loading::DataArgs,
    #[arg(long, default_value_t = 1)]
    jobs: usize,
    #[arg(long, default_value_t = 1000)]
    max_evaluations: usize,
    #[arg(long, default_value_t = 10000)]
    max_proposals: usize,
    #[arg(long, default_value_t = 128)]
    max_rounds: usize,
    #[arg(long, default_value_t = 300)]
    timeout_seconds: u64,
    #[arg(long, default_value_t = 0)]
    seed: u64,
    #[arg(long,value_enum,default_value_t=super::mutation_search::NativeEvaluation::Typed)]
    native_evaluation: super::mutation_search::NativeEvaluation,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long)]
    export: Option<PathBuf>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Problem {
    schema_version: u32,
    template: PathBuf,
    #[serde(default)]
    equipment: Vec<EquipmentAlternative>,
    objective: ObjectiveSpec,
    constraints: CandidateConstraints,
    #[serde(default)]
    attribute_locks: AttributeOptionLocks,
    #[serde(default)]
    initial_allocations: Vec<PassiveAllocationSelection>,
}
#[derive(Debug, Clone, Serialize)]
struct State {
    #[serde(flatten)]
    selection: BuildSelection,
    #[serde(skip)]
    admitted: Result<AdmittedBuildSelection, String>,
}
impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        self.selection == other.selection
    }
}
impl Eq for State {}
impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for State {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.selection.cmp(&other.selection)
    }
}
struct Domain {
    domain: ControlledBuildDomain,
    pool: rayon::ThreadPool,
    preparations: AtomicUsize,
    seeds: Vec<BuildSelection>,
    deadline: Instant,
}
impl Domain {
    fn prepare(
        &self,
        values: Vec<BuildSelection>,
        control: Option<&EvaluationControl<'_>>,
    ) -> Vec<State> {
        self.pool.install(|| {
            values
                .into_par_iter()
                .map_init(
                    poe_optimizer_native::ActorScratch::default,
                    |scratch, selection| {
                        let admitted = if Instant::now() >= self.deadline
                            || control.is_some_and(EvaluationControl::should_stop)
                        {
                            Err("Search stopped before source assembly".into())
                        } else {
                            self.preparations.fetch_add(1, Ordering::Relaxed);
                            self.domain
                                .admit(selection.clone(), scratch)
                                .map_err(|e| e.to_string())
                        };
                        State {
                            selection,
                            admitted,
                        }
                    },
                )
                .collect()
        })
    }
    fn neighbors(
        &self,
        parent: &BuildSelection,
        limit: usize,
        control: &EvaluationControl<'_>,
    ) -> Vec<BuildSelection> {
        if limit == 0 || control.should_stop() {
            return vec![];
        }
        let catalog = self.domain.catalog();
        let graph = &catalog.catalog().passive_nodes;
        let mut out = Vec::new();
        // Source component swaps are independent of passive mutation; generated
        // states subsequently pass the same complete structural/requirement admission.
        for slot in &catalog.catalog().equipment_slots {
            for (id, _) in catalog
                .items()
                .filter(|(_, item)| item.allowed_slots().contains(slot))
            {
                let mut next = parent.clone();
                next.candidate.equipment.insert(slot.clone(), id.into());
                out.push(next);
                if out.len() >= limit || control.should_stop() {
                    return out;
                }
            }
            if slot != "Weapon 1" {
                let mut next = parent.clone();
                next.candidate.equipment.remove(slot);
                out.push(next);
                if out.len() >= limit || control.should_stop() {
                    return out;
                }
            }
        }
        if let Some(skill) = parent.candidate.skills.values().next() {
            for id in catalog.catalog().supports.keys() {
                let mut next = parent.clone();
                let target = next.candidate.skills.values_mut().next().unwrap();
                if skill.support_instance_ids.contains(id) {
                    target.support_instance_ids.remove(id);
                } else {
                    target.support_instance_ids.insert(id.clone());
                }
                out.push(next);
                if out.len() >= limit || control.should_stop() {
                    return out;
                }
            }
        }
        let mut allocated = parent.candidate.passives.clone();
        if let Some(class) = catalog.catalog().classes.get(&parent.candidate.class_id) {
            allocated.insert(class.start_node_id);
        }
        if let Some(asc) = parent
            .candidate
            .ascendancy_id
            .as_ref()
            .and_then(|id| catalog.catalog().ascendancies.get(id))
        {
            allocated.insert(asc.start_node_id);
        }
        let mut frontier = BTreeSet::new();
        for id in &allocated {
            if let Some(node) = graph.get(id) {
                frontier.extend(
                    node.links
                        .iter()
                        .filter(|id| !allocated.contains(id))
                        .copied(),
                );
            }
        }
        for id in frontier {
            if !graph.get(&id).is_some_and(|node| {
                matches!(
                    node.kind,
                    PassiveKind::Ordinary | PassiveKind::Ascendancy { .. }
                )
            }) {
                continue;
            }
            let options = catalog
                .data()
                .snapshot()
                .tree()
                .allocation_nodes
                .get(&id)
                .map(|node| node.attribute_options.keys().copied().collect::<Vec<_>>())
                .unwrap_or_default();
            if options.is_empty() {
                let mut next = parent.clone();
                next.candidate.passives.insert(id);
                out.push(next);
                if out.len() >= limit || control.should_stop() {
                    return out;
                }
            } else {
                for option in options {
                    let mut next = parent.clone();
                    next.candidate.passives.insert(id);
                    next.attribute_options.insert(id, option);
                    out.push(next);
                    if out.len() >= limit || control.should_stop() {
                        return out;
                    }
                }
            }
        }
        for id in &parent.candidate.passives {
            let mut next = parent.clone();
            next.candidate.passives.remove(id);
            next.attribute_options.remove(id);
            out.push(next);
            if out.len() >= limit || control.should_stop() {
                return out;
            }
            if parent.attribute_options.contains_key(id) {
                for option in AttributeOption::ALL {
                    let mut next = parent.clone();
                    next.attribute_options.insert(*id, option);
                    out.push(next);
                    if out.len() >= limit || control.should_stop() {
                        return out;
                    }
                }
            }
        }
        // Class/ascendancy moves preserve physical choices. Connectivity/ownership
        // may reject them; root-only restarts provide another route into each class.
        for class_id in catalog.catalog().classes.keys() {
            for asc in std::iter::once(None).chain(
                catalog
                    .catalog()
                    .ascendancies
                    .iter()
                    .filter(|(_, asc)| &asc.class_id == class_id)
                    .map(|(id, _)| Some(id.clone())),
            ) {
                let mut next = parent.clone();
                next.candidate.class_id = class_id.clone();
                next.candidate.ascendancy_id = asc;
                out.push(next);
                if out.len() >= limit || control.should_stop() {
                    return out;
                }
            }
        }
        out
    }
}
impl SearchDomain<State> for Domain {
    fn deduplication_key(&self, state: &State) -> Option<Vec<u8>> {
        Some(state.selection.fingerprint().into_bytes())
    }

    fn can_propose_after_empty(&self) -> bool {
        false
    }
    fn validate(&self, state: &State, control: &EvaluationControl<'_>) -> Result<(), String> {
        if control.should_stop() {
            return Err("Search stopped before admitted handle validation".into());
        }
        let handle = state.admitted.as_ref().map_err(Clone::clone)?;
        self.domain
            .validate_handle(handle)
            .map_err(|e| e.to_string())
    }
    fn propose(
        &self,
        parents: &[State],
        round: usize,
        seed: u64,
        limit: usize,
        control: &EvaluationControl<'_>,
    ) -> Result<Vec<State>, String> {
        if control.should_stop() {
            return Ok(vec![]);
        }
        let mut unique = BTreeSet::new();
        let raw_limit = limit.min(256).saturating_mul(4);
        let parent_selections: Vec<_> = if parents.is_empty() {
            self.seeds.iter().collect()
        } else {
            parents.iter().map(|parent| &parent.selection).collect()
        };
        if !parent_selections.is_empty() {
            let offset = (seed as usize) % parent_selections.len();
            for step in 0..parent_selections.len() {
                if control.should_stop() || unique.len() >= raw_limit {
                    break;
                }
                let parent = parent_selections[(offset + step) % parent_selections.len()];
                unique.extend(self.neighbors(parent, raw_limit - unique.len(), control));
            }
        }
        if round.is_multiple_of(4) || parents.is_empty() {
            unique.extend(
                self.seeds
                    .iter()
                    .take(raw_limit.saturating_sub(unique.len()))
                    .cloned(),
            );
        }
        let mut values: Vec<_> = unique.into_iter().collect();
        // Deterministic Fisher-Yates avoids starving late sorted node/slot IDs.
        let mut random = seed ^ (round as u64).wrapping_mul(0x9e3779b97f4a7c15);
        for i in (1..values.len()).rev() {
            random = random
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            values.swap(i, (random as usize) % (i + 1));
        }
        values.truncate(limit.min(256));
        Ok(self.prepare(values, Some(control)))
    }
}
struct Evaluator<'a> {
    domain: &'a Domain,
    backend: &'a NativeBackend,
    prepared: &'a PreparedBuildCandidates,
    metrics: Vec<MetricQuery>,
    document: bool,
    finalists: Mutex<BTreeMap<BuildSelection, EvaluationResult>>,
}
impl Evaluator<'_> {
    fn document(
        &self,
        state: &State,
        control: &EvaluationControl<'_>,
        retain: bool,
    ) -> Result<CandidateMeasurements, String> {
        if control.should_stop() {
            return Err("Search stopped before document calculation".into());
        }
        let handle = state.admitted.as_ref().map_err(Clone::clone)?;
        let build = self
            .domain
            .domain
            .materialize(handle)
            .map_err(|e| e.to_string())?;
        let budget = EvaluationBudget {
            timeout_ms: control.remaining().as_millis().min(60_000) as u64,
        };
        let result = self
            .backend
            .calculate(
                &EvaluationRequest {
                    build,
                    options: EvaluationOptions::default(),
                    metrics: self.metrics.clone(),
                },
                budget,
            )
            .map_err(|e| e.to_string())?;
        self.domain
            .domain
            .catalog()
            .validate_native_realization(handle, &result, &self.backend.identity())
            .map_err(|e| e.to_string())?;
        let measurements = CandidateMeasurements {
            measurements: result.measurements.clone(),
            diagnostic_only: result.diagnostic_only,
        };
        if retain {
            self.finalists
                .lock()
                .map_err(|_| "Finalist lock poisoned")?
                .insert(state.selection.clone(), result);
        }
        Ok(measurements)
    }
}
impl CandidateEvaluator<State> for Evaluator<'_> {
    fn execution_kind(&self) -> ExecutionKind {
        ExecutionKind::RustCpu
    }
    fn evaluate(
        &self,
        state: &State,
        control: &EvaluationControl<'_>,
    ) -> Result<CandidateMeasurements, String> {
        if self.document {
            return self.document(state, control, false);
        }
        if control.should_stop() {
            return Err("Search stopped before typed calculation".into());
        }
        let handle = state.admitted.as_ref().map_err(Clone::clone)?;
        let snapshot = self
            .backend
            .evaluate_controlled_build(
                self.prepared,
                handle,
                EvaluationBudget {
                    timeout_ms: control.remaining().as_millis().min(60_000) as u64,
                },
            )
            .map_err(|e| e.to_string())?;
        Ok(CandidateMeasurements {
            measurements: self.prepared.snapshot_measurements(&snapshot),
            diagnostic_only: snapshot.diagnostic_only(),
        })
    }
    fn verify(
        &self,
        state: &State,
        control: &EvaluationControl<'_>,
    ) -> Result<CandidateMeasurements, String> {
        self.document(state, control, true)
    }
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    if !(1..=256).contains(&args.jobs)
        || args.max_evaluations < 3
        || args.max_proposals == 0
        || args.max_rounds == 0
        || args.timeout_seconds == 0
    {
        return Err("Positive bounded jobs/proposals/rounds/duration and at least three evaluations are required".into());
    }
    let started = Instant::now();
    let duration = Duration::from_secs(args.timeout_seconds);
    let deadline = started
        .checked_add(duration)
        .ok_or("Search duration exceeds clock range")?;
    let problem: Problem = super::read_json(&args.problem, 1024 * 1024)?;
    if ![7, 8, 9, 10].contains(&problem.schema_version) {
        return Err("Graph build search requires problem schema 7, 8, 9 or 10".into());
    }
    if problem.initial_allocations.len() > 256 {
        return Err("At most 256 explicit seed allocations are supported".into());
    }
    let template = args
        .problem
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join(&problem.template);
    let mut outputs = Vec::new();
    if let Some(path) = &args.output {
        outputs.push(path.clone());
    }
    if let Some(path) = &args.export {
        outputs.push(path.clone());
        outputs.push(PathBuf::from(format!("{}.data.json", path.display())));
    }
    let mut unique = BTreeSet::new();
    for path in &outputs {
        let absolute = super::destination_identity(path)?;
        if path.exists() || !unique.insert(absolute) {
            return Err("Output paths must be distinct new files".into());
        }
        if !path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| std::path::Path::new("."))
            .is_dir()
        {
            return Err("Output parent directory must exist".into());
        }
    }
    let source = poe_optimizer_import::decode_build(&super::read_input(&template)?)?.xml;
    let backend = args.data.backend()?;
    let catalog = Arc::new(ControlledBuildCatalog::new(
        backend.data().clone(),
        source,
        problem.equipment.clone(),
    )?);
    if problem.schema_version < 10 && catalog.uses_movement_scope() {
        return Err(
            "Body Armour or authored movement modifiers require graph problem schema 10".into(),
        );
    }
    if problem.schema_version < 9 && catalog.uses_local_armour_scope() {
        return Err("Local armour equipment requires graph problem schema 9".into());
    }
    if problem.schema_version == 7 && catalog.uses_receiving_defence_scope() {
        return Err(
            "Authored receiving-defence configuration or equipment requires graph problem schema 8"
                .into(),
        );
    }
    let mut seeds = vec![catalog.source_selection()];
    for allocation in &problem.initial_allocations {
        let mut selection = catalog.source_selection();
        selection.candidate.class_id = allocation.class_id.to_string();
        selection.candidate.ascendancy_id = allocation.ascendancy_id.clone();
        selection.candidate.passives = allocation
            .ordinary_nodes
            .union(&allocation.ascendancy_nodes)
            .copied()
            .collect();
        selection.attribute_options = allocation.attribute_options.clone();
        seeds.push(selection);
    }
    for class_id in catalog.catalog().classes.keys() {
        for asc in std::iter::once(None).chain(
            catalog
                .catalog()
                .ascendancies
                .iter()
                .filter(|(_, asc)| &asc.class_id == class_id)
                .map(|(id, _)| Some(id.clone())),
        ) {
            let mut selection = catalog.source_selection();
            selection.candidate.class_id = class_id.clone();
            selection.candidate.ascendancy_id = asc;
            selection.candidate.passives.clear();
            selection.attribute_options.clear();
            seeds.push(selection);
        }
    }
    let mut seed_repair_errors = Vec::new();
    seeds = seeds
        .into_iter()
        .map(|selection| {
            if Instant::now() >= deadline {
                return selection;
            }
            match seeds::repair_seed(
                &catalog,
                &problem.constraints,
                &problem.attribute_locks,
                selection.clone(),
            ) {
                Ok(repaired) => repaired,
                Err(error) => {
                    if seed_repair_errors.len() < 32 && !seed_repair_errors.contains(&error) {
                        seed_repair_errors.push(error);
                    }
                    selection
                }
            }
        })
        .collect();
    seeds.sort();
    seeds.dedup();
    let domain = Domain {
        domain: ControlledBuildDomain::new(
            catalog.clone(),
            problem.constraints.clone(),
            problem.attribute_locks.clone(),
        )?,
        pool: rayon::ThreadPoolBuilder::new()
            .num_threads(args.jobs)
            .build()?,
        preparations: AtomicUsize::new(0),
        seeds,
        deadline,
    };
    let policy = problem.objective.compile(&backend.capabilities().metrics)?;
    let metrics = policy.required_metrics();
    let prepared = backend.prepare_controlled_build(&catalog, &metrics)?;
    let initial = domain.prepare(
        domain
            .seeds
            .iter()
            .take(args.max_proposals)
            .cloned()
            .collect(),
        None,
    );
    let baseline_attempt = if initial.iter().any(|state| state.admitted.is_ok()) {
        Some(
            backend.calculate(
                &EvaluationRequest {
                    build: BuildDocument {
                        format: BuildFormat::PathOfBuilding2Xml,
                        content: catalog.template().source().into(),
                    },
                    options: EvaluationOptions::default(),
                    metrics: metrics.clone(),
                },
                EvaluationBudget {
                    timeout_ms: duration
                        .saturating_sub(started.elapsed())
                        .as_millis()
                        .min(60_000) as u64,
                },
            ),
        )
    } else {
        None
    };
    let baseline_attempts = usize::from(baseline_attempt.is_some());
    // An imported numerical failure must not suppress valid repaired alternatives.
    // Keep the failed full-document attempt visible and spend its budget once.
    let (baseline, baseline_error) = match baseline_attempt {
        Some(Ok(result)) => (Some(result), None),
        Some(Err(error)) => (None, Some(error)),
        None => (None, None),
    };
    if Instant::now() >= deadline {
        return Err("Search deadline exhausted during bounded input preparation".into());
    }
    let evaluator = Evaluator {
        domain: &domain,
        backend: &backend,
        prepared: &prepared,
        metrics,
        document: matches!(
            args.native_evaluation,
            super::mutation_search::NativeEvaluation::Document
        ),
        finalists: Mutex::new(BTreeMap::new()),
    };
    let budget = SearchBudget {
        max_evaluations: args.max_evaluations - baseline_attempts,
        max_proposals: args.max_proposals,
        max_rounds: args.max_rounds,
        duration: duration.saturating_sub(started.elapsed()),
        jobs: args.jobs,
        verification_attempts: 1,
        seed: args.seed,
        ..Default::default()
    };
    let result = search(
        &domain,
        &evaluator,
        &policy,
        SearchPlan::Explore(initial),
        &budget,
        &AtomicBool::new(false),
    )?;
    let best = result.feasible.first().filter(|best| {
        result
            .verifications
            .iter()
            .any(|v| v.candidate == best.candidate && v.consistent)
    });
    let mut report = serde_json::json!({"schema_version":problem.schema_version + 1,"scope":match problem.schema_version {10 => "movement_native_search_v1", 9 => "local_armour_native_search_v1", 8 => "receiving_defence_native_search_v1", _ => "connected_passive_equipment_native_search_v1"},"search_implementation_sha256":hash(concat!(include_str!("build_search.rs"),include_str!("build_search_seeds.rs"),include_str!("../crates/poe-optimizer-search/src/lib.rs")).as_bytes()),"diagnostic_only":true,
        "backend":backend.identity(),"data_trust":backend.data().snapshot().trust(),"problem":problem,"native_evaluation":args.native_evaluation,
        "seed_repair_errors":seed_repair_errors,"catalog":catalog.catalog().identity,"catalog_preparation":catalog.footprint(),"native_preparation":prepared.footprint(),
        "source_assembly_attempts":domain.preparations.load(Ordering::Relaxed),"total_evaluations":baseline_attempts+result.statistics.evaluations,
        "elapsed_ms":started.elapsed().as_secs_f64()*1000.0,"baseline":baseline,"baseline_error":baseline_error,"baseline_attempts":baseline_attempts,
        "export":{"status":"not_requested"},"best_verified":null});
    if let Some(best) = best {
        let finalists = evaluator
            .finalists
            .lock()
            .map_err(|_| "Finalist lock poisoned")?;
        let fresh = finalists
            .get(&best.candidate.selection)
            .ok_or("Verified finalist source missing")?;
        let xml = &fresh
            .exports
            .first()
            .ok_or("Verified native export missing")?
            .content;
        report["best_verified"] = serde_json::json!({"candidate":best.candidate.selection,"assessment":best.assessment,"source_xml_sha256":hash(xml.as_bytes()),"requirements":best.candidate.admitted.as_ref().unwrap().requirements(),"diagnostic_only":true});
        if let Some(path) = &args.export {
            super::write_new(path, xml.as_bytes())?;
            let manifest = PathBuf::from(format!("{}.data.json", path.display()));
            let metadata = serde_json::json!({"schema_version":1,"status":"native_export_data","backend":backend.identity(),"uses_packaged_default":args.data.data.is_none(),"xml_sha256":hash(xml.as_bytes()),"data_trust":backend.data().snapshot().trust(),"package_path_hint":args.data.data,"reload_requirement":"Load a package matching backend.data before evaluating this XML; the path hint is not identity or trust."});
            super::write_new(&manifest, &serde_json::to_vec_pretty(&metadata)?)?;
            report["export"] =
                serde_json::json!({"status":"written","path":path,"data_manifest":manifest});
        }
    } else if args.export.is_some() {
        report["export"] = serde_json::json!({"status":"not_written","reason":"No feasible finalist passed fresh verification"});
    }
    report["search"] = serde_json::to_value(result)?;
    let mut bytes = serde_json::to_vec_pretty(&report)?;
    bytes.push(b'\n');
    if let Some(path) = args.output {
        super::write_new(&path, &bytes)?;
    } else {
        io::stdout().write_all(&bytes)?;
    }
    Ok(())
}
