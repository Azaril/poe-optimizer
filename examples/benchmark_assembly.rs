//! Developer timings for component assembly/admission and already-admitted measures.
//! Run with --release --no-default-features. This is not an optimizer-quality benchmark.
#![recursion_limit = "256"]
use clap::{Parser, ValueEnum};
use poe_optimizer_core::{
    candidate::{CandidateBudgets, CandidateConstraints, PassiveKind},
    evaluation::{Engine, EvaluationBudget, EvaluationEngine, EvaluationRequest, SharedBackend},
    options::EvaluationOptions,
};
use poe_optimizer_data::{
    class_tree::{self, AttributeOption, PassiveAllocationSelection},
    game_data::bundled_snapshot,
};
use poe_optimizer_import::controlled_build::{
    AdmittedBuildSelection, AttributeOptionLocks, BuildSelection, ControlledBuildCatalog,
    ControlledBuildDomain, EquipmentAlternative,
};
use poe_optimizer_native::{
    ActorScratch, CompiledGameData, HostClock, NativeBackend, NativeMetricSnapshot,
    NativeMetricValue, PreparedBuildCandidates,
};
use rayon::prelude::*;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    error::Error,
    hint::black_box,
    sync::Arc,
    time::Instant,
};

const TEMPLATE: &str = include_str!("../tests/fixtures/builds/mace-passive-equipment.xml");
const EQUIPMENT: &str = include_str!("passive-equipment-search.json");
const SOURCE: &str = include_str!("benchmark_assembly.rs");
const MAX_EVALUATIONS: usize = 100_000_000;

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Mode {
    AdmitMeasure,
    Measure,
}
impl Mode {
    fn name(self) -> &'static str {
        match self {
            Self::AdmitMeasure => "fresh_admission_and_measure",
            Self::Measure => "already_admitted_measure",
        }
    }
}
#[derive(Parser)]
struct Args {
    /// Use the receiving-defence source template and supplied equipment corpus.
    #[arg(long)]
    receiving_defence: bool,
    /// Use fixed local armour slots together with receiving defences.
    #[arg(long, conflicts_with = "receiving_defence")]
    local_armour: bool,
    /// Approximate duration of each sample; calibration is reported separately.
    #[arg(long, default_value_t = 1200)]
    sample_ms: u64,
    #[arg(long, default_value_t = 3)]
    repeats: usize,
    #[arg(long, value_delimiter = ',', default_value = "1,2,4,32")]
    jobs: Vec<usize>,
    #[arg(
        long,
        value_enum,
        value_delimiter = ',',
        default_value = "admit-measure,measure"
    )]
    modes: Vec<Mode>,
    #[arg(long)]
    cpu_label: Option<String>,
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn milliseconds(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}
fn allocation_selection(
    catalog: &ControlledBuildCatalog,
    allocation: &PassiveAllocationSelection,
) -> BuildSelection {
    let mut value = catalog.source_selection();
    value.candidate.class_id = allocation.class_id.to_string();
    value.candidate.ascendancy_id = allocation.ascendancy_id.clone();
    value.candidate.passives = allocation
        .ordinary_nodes
        .union(&allocation.ascendancy_nodes)
        .copied()
        .collect();
    value.attribute_options = allocation.attribute_options.clone();
    value
}
// Find one short connected attribute path per class using selected source data.
// This setup uses structural admission, not a guessed list of physical node IDs.
fn attribute_path(catalog: &ControlledBuildCatalog, class: &str) -> Option<BuildSelection> {
    let mut initial = catalog.source_selection();
    initial.candidate.class_id = class.into();
    initial.candidate.ascendancy_id = None;
    initial.candidate.passives.clear();
    initial.attribute_options.clear();
    let root = catalog.catalog().classes[class].start_node_id;
    let mut visited = BTreeSet::from([root]);
    let mut queue = VecDeque::from([(root, initial)]);
    while let Some((at, selected)) = queue.pop_front() {
        if selected.candidate.passives.len() >= 4 {
            continue;
        }
        for next in &catalog.catalog().passive_nodes[&at].links {
            if visited.contains(next)
                || !matches!(
                    catalog.catalog().passive_nodes[next].kind,
                    PassiveKind::Ordinary
                )
            {
                continue;
            }
            let mut trial = selected.clone();
            trial.candidate.passives.insert(*next);
            if catalog.compiled().snapshot().tree().allocation_nodes[next]
                .attribute_options
                .contains_key(&AttributeOption::Strength)
            {
                trial
                    .attribute_options
                    .insert(*next, AttributeOption::Strength);
            }
            let resolved = trial
                .allocation(catalog.catalog())
                .ok()?
                .resolve(catalog.compiled().snapshot());
            if resolved.is_err() {
                continue;
            }
            visited.insert(*next);
            if !trial.attribute_options.is_empty() {
                return Some(trial);
            }
            queue.push_back((*next, trial));
        }
    }
    None
}
fn selections(catalog: &ControlledBuildCatalog) -> Result<Vec<BuildSelection>, Box<dyn Error>> {
    let mut trees = class_tree::selections(catalog.compiled().snapshot().tree())?
        .into_iter()
        .map(|value| allocation_selection(catalog, &value.into()))
        .collect::<Vec<_>>();
    for class in catalog.catalog().classes.keys() {
        if let Some(path) = attribute_path(catalog, class) {
            for option in AttributeOption::ALL {
                let mut selected = path.clone();
                selected
                    .attribute_options
                    .values_mut()
                    .for_each(|v| *v = option);
                trees.push(selected);
            }
        }
    }
    let data = catalog.compiled().snapshot().package();
    let mut loadouts = vec![Vec::new()];
    for (index, support) in data.supports.iter().enumerate() {
        loadouts.push(vec![support.id.clone()]);
        for other in &data.supports[index + 1..] {
            let mut pair = vec![support.id.clone(), other.id.clone()];
            pair.sort();
            if data.validate_mace_support_loadout(&pair).is_ok() {
                loadouts.push(pair);
            }
        }
    }
    let weapons = catalog
        .items()
        .filter(|(_, item)| item.weapon().is_some())
        .map(|(id, _)| id.to_owned())
        .collect::<Vec<_>>();
    let mut amulets = vec![None];
    amulets.extend(
        catalog
            .items()
            .filter(|(_, item)| item.allowed_slots().iter().any(|slot| slot == "Amulet"))
            .map(|(id, _)| Some(id.to_owned())),
    );
    let armour_slots = ["Helmet", "Gloves", "Boots"].map(|slot| {
        let mut choices = vec![None];
        choices.extend(
            catalog
                .items()
                .filter(|(_, item)| item.allowed_slots().iter().any(|allowed| allowed == slot))
                .map(|(id, _)| Some(id.to_owned())),
        );
        (slot, choices)
    });
    let mut unique = BTreeMap::new();
    // An intentionally bounded varied corpus, not a Cartesian candidate/result cache.
    // Every tree sees all support loadouts; equipment choices rotate twice per loadout.
    for (tree_index, tree) in trees.iter().enumerate() {
        for (support_index, supports) in loadouts.iter().enumerate() {
            for variant in 0..2 {
                let mut selection = tree.clone();
                selection.candidate.equipment.insert(
                    "Weapon 1".into(),
                    weapons[(tree_index + support_index + variant) % weapons.len()].clone(),
                );
                let amulet = &amulets[(tree_index + support_index + variant) % amulets.len()];
                selection.candidate.equipment.remove("Amulet");
                if let Some(amulet) = amulet {
                    selection
                        .candidate
                        .equipment
                        .insert("Amulet".into(), amulet.clone());
                }
                for (index, (slot, choices)) in armour_slots.iter().enumerate() {
                    selection.candidate.equipment.remove(*slot);
                    if let Some(item) =
                        &choices[(tree_index + support_index + variant + index) % choices.len()]
                    {
                        selection
                            .candidate
                            .equipment
                            .insert((*slot).into(), item.clone());
                    }
                }
                selection
                    .candidate
                    .skills
                    .values_mut()
                    .next()
                    .ok_or("Missing active skill")?
                    .support_instance_ids = supports
                    .iter()
                    .map(|key| {
                        catalog
                            .support_instance(key)
                            .expect("catalog support")
                            .to_owned()
                    })
                    .collect();
                unique.insert(selection.fingerprint(), selection);
            }
        }
    }
    Ok(unique.into_values().collect())
}
fn snapshot_checksum(snapshot: &NativeMetricSnapshot) -> u64 {
    // Integer reduction is independent of Rayon floating-point reduction order.
    // Consume every finite metric and the availability status, not only DPS.
    snapshot
        .values()
        .iter()
        .enumerate()
        .fold(0_u64, |sum, (index, value)| {
            let bits = match value {
                NativeMetricValue::Finite(value) => value.to_bits(),
                NativeMetricValue::Unavailable(_) => 0x9e37_79b9_7f4a_7c15,
                NativeMetricValue::NonFinite(_) => panic!("Non-finite benchmark input"),
            };
            sum.wrapping_add(bits.rotate_left((index as u32 * 7) % 64))
        })
}
fn measured_checksum(
    snapshot: &NativeMetricSnapshot,
    handle: &AdmittedBuildSelection,
    include_receiving: bool,
) -> u64 {
    let metrics = snapshot_checksum(snapshot);
    if !include_receiving {
        return metrics;
    }
    let r = black_box(
        handle
            .actor()
            .receiving()
            .expect("complete receiving preparation"),
    );
    [
        r.armour,
        r.evasion,
        r.energy_shield,
        r.resistances.fire,
        r.resistances.cold,
        r.resistances.lightning,
        r.resistances.chaos,
        r.resistance_totals.fire,
        r.resistance_totals.cold,
        r.resistance_totals.lightning,
        r.resistance_totals.chaos,
        r.resistance_cap,
        r.resistance_floor,
    ]
    .into_iter()
    .enumerate()
    .fold(metrics, |sum, (index, value)| {
        sum.wrapping_add(value.to_bits().rotate_left((index as u32 * 11 + 3) % 64))
    })
}

struct Workload<'a> {
    domain: &'a ControlledBuildDomain,
    prepared: &'a PreparedBuildCandidates,
    selections: &'a [BuildSelection],
    handles: &'a [AdmittedBuildSelection],
    checksums: Vec<u64>,
    include_receiving: bool,
}
impl Workload<'_> {
    fn expected(&self, evaluations: usize) -> u64 {
        let cycle = self.checksums.iter().fold(0_u64, |a, b| a.wrapping_add(*b));
        self.checksums[..evaluations % self.checksums.len()]
            .iter()
            .fold(
                cycle.wrapping_mul((evaluations / self.checksums.len()) as u64),
                |a, b| a.wrapping_add(*b),
            )
    }
    fn run(&self, pool: &rayon::ThreadPool, mode: Mode, evaluations: usize) -> Result<u64, String> {
        pool.install(|| {
            (0..evaluations)
                .into_par_iter()
                .map_init(ActorScratch::default, |scratch, iteration| {
                    let index = iteration % self.handles.len();
                    let measure = |handle: &AdmittedBuildSelection| {
                        self.prepared
                            .measure(black_box(handle))
                            .map(|snapshot| {
                                measured_checksum(
                                    black_box(&snapshot),
                                    handle,
                                    self.include_receiving,
                                )
                            })
                            .map_err(|error| error.to_string())
                    };
                    match mode {
                        Mode::AdmitMeasure => {
                            // Includes selection cloning, structural/data proof, actor program
                            // execution, requirements, owned handle construction and destruction.
                            let handle = self
                                .domain
                                .admit(black_box(self.selections[index].clone()), scratch)
                                .map_err(|error| error.to_string())?;
                            measure(&handle)
                        }
                        Mode::Measure => measure(&self.handles[index]),
                    }
                })
                .try_reduce(|| 0_u64, |a, b| Ok(a.wrapping_add(b)))
        })
    }
    fn calibrate(
        &self,
        pool: &rayon::ThreadPool,
        mode: Mode,
        target_ms: u64,
    ) -> Result<(usize, Value), Box<dyn Error>> {
        let mut evaluations = self.handles.len();
        let mut attempts = 0;
        let start = Instant::now();
        loop {
            let pilot = Instant::now();
            let checksum = self.run(pool, mode, evaluations)?;
            let elapsed = pilot.elapsed().as_secs_f64();
            assert_eq!(checksum, self.expected(evaluations), "calibration checksum");
            attempts += evaluations;
            if elapsed >= 0.05 || evaluations >= MAX_EVALUATIONS / 2 {
                let estimate =
                    (evaluations as f64 * target_ms as f64 / 1000.0 / elapsed).ceil() as usize;
                let selected = estimate.clamp(self.handles.len(), MAX_EVALUATIONS);
                return Ok((
                    selected,
                    json!({"evaluations":attempts,"elapsed_ms":milliseconds(start),
                    "selected_sample_evaluations":selected,"pilot_evaluations":evaluations,
                    "pilot_ms":elapsed*1000.0}),
                ));
            }
            evaluations = (evaluations * 4).min(MAX_EVALUATIONS);
        }
    }
}
fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    if !(100..=10_000).contains(&args.sample_ms)
        || !(1..=10).contains(&args.repeats)
        || args.jobs.is_empty()
        || args.jobs.len() > 16
        || args.jobs.iter().any(|j| !(1..=256).contains(j))
        || args.jobs.iter().collect::<BTreeSet<_>>().len() != args.jobs.len()
        || args.modes.is_empty()
        || args.modes.len() > 2
        || (args.modes.len() == 2 && args.modes[0] == args.modes[1])
    {
        return Err("Require 100..10000 sample-ms, 1..10 repeats, distinct jobs in 1..256 and distinct modes".into());
    }
    let include_receiving = args.receiving_defence || args.local_armour;
    let (candidate_set, template_path, template, equipment_path, equipment_json) =
        if args.local_armour {
            (
                "local-armour",
                "tests/fixtures/builds/mace-local-armour.xml",
                include_str!("../tests/fixtures/builds/mace-local-armour.xml"),
                "examples/local-armour-search.json",
                include_str!("local-armour-search.json"),
            )
        } else if args.receiving_defence {
            (
                "receiving-defence",
                "tests/fixtures/builds/mace-receiving-defence.xml",
                include_str!("../tests/fixtures/builds/mace-receiving-defence.xml"),
                "examples/receiving-defence-search.json",
                include_str!("receiving-defence-search.json"),
            )
        } else {
            (
                "passive-equipment",
                "tests/fixtures/builds/mace-passive-equipment.xml",
                TEMPLATE,
                "examples/passive-equipment-search.json",
                EQUIPMENT,
            )
        };
    let start = Instant::now();
    let snapshot = Arc::new(bundled_snapshot()?);
    let compiled = Arc::new(CompiledGameData::compile(snapshot.clone())?);
    let backend = Arc::new(NativeBackend::with_data(compiled.clone(), HostClock)?);
    let dataset_ms = milliseconds(start);
    let start = Instant::now();
    let input: Value = serde_json::from_str(equipment_json)?;
    let equipment: Vec<EquipmentAlternative> = serde_json::from_value(input["equipment"].clone())?;
    let catalog = Arc::new(ControlledBuildCatalog::new(
        compiled,
        template.into(),
        equipment,
    )?);
    let catalog_ms = milliseconds(start);
    let start = Instant::now();
    let constraints = CandidateConstraints {
        budgets: CandidateBudgets {
            ordinary_passive_points: 4,
            ascendancy_passive_points: 1,
            active_skill_count: 1,
            supports_per_skill: 2,
            ..Default::default()
        },
        ..Default::default()
    };
    let domain = ControlledBuildDomain::new(
        catalog.clone(),
        constraints.clone(),
        AttributeOptionLocks::default(),
    )?;
    let domain_ms = milliseconds(start);
    let start = Instant::now();
    let prepared = backend.prepare_controlled_build(&catalog, &[])?;
    let prepared_ms = milliseconds(start);
    let start = Instant::now();
    let proposed = selections(&catalog)?;
    let proposal_ms = milliseconds(start);
    let structural_candidates = proposed.len();
    let start = Instant::now();
    let mut selections = Vec::new();
    let mut handles = Vec::new();
    let mut rejected = BTreeMap::<String, usize>::new();
    let mut scratch = ActorScratch::default();
    for selection in proposed {
        match domain.admit(selection.clone(), &mut scratch) {
            Ok(handle) => {
                selections.push(selection);
                handles.push(handle);
            }
            Err(error) => {
                *rejected.entry(error.to_string()).or_default() += 1;
            }
        }
    }
    let admission_setup_ms = milliseconds(start);
    if handles.is_empty() {
        return Err("No legal benchmark inputs".into());
    }
    let start = Instant::now();
    let checksums = handles
        .iter()
        .map(|handle| {
            prepared
                .measure(handle)
                .map(|s| measured_checksum(&s, handle, include_receiving))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let initial_measure_ms = milliseconds(start);
    let distinct_output_checksums = checksums.iter().copied().collect::<BTreeSet<_>>().len();
    if distinct_output_checksums < 2 {
        return Err("Benchmark needs varying outputs".into());
    }
    // Fresh complete native document path, outside timed loops and without a result cache.
    let start = Instant::now();
    let engine = Engine::new(SharedBackend::new(backend.clone()));
    let validation_indices = (0..16.min(handles.len()))
        .map(|i| i * handles.len() / 16.min(handles.len()))
        .collect::<Vec<_>>();
    for index in &validation_indices {
        let result = engine.evaluate(
            &EvaluationRequest {
                build: domain.materialize(&handles[*index])?,
                options: EvaluationOptions::default(),
                metrics: vec![],
            },
            EvaluationBudget { timeout_ms: 30_000 },
        )?;
        let actual = prepared.snapshot_measurements(&prepared.measure(&handles[*index])?);
        assert_eq!(
            serde_json::to_value(actual)?,
            serde_json::to_value(result.measurements)?,
            "fresh full-document comparison {index}"
        );
        if include_receiving {
            let attachment = result
                .attachments
                .iter()
                .find(|value| value.media_type.contains("native-profile+"))
                .ok_or("Missing native profile evidence")?;
            let profile: Value = serde_json::from_str(&attachment.content)?;
            let expected = poe_optimizer_import::actor_assembly::receiving_defence_evidence(
                handles[*index]
                    .actor()
                    .receiving()
                    .ok_or("Missing receiver output")?,
            );
            assert_eq!(
                expected, profile["receiving_defence"],
                "fresh receiving comparison {index}"
            );
        }
        assert!(result.diagnostic_only);
    }
    let validation_ms = milliseconds(start);
    let workload = Workload {
        domain: &domain,
        prepared: &prepared,
        selections: &selections,
        handles: &handles,
        checksums,
        include_receiving,
    };
    let mut samples = Vec::new();
    let mut worker_setups = Vec::new();
    for jobs in &args.jobs {
        let start = Instant::now();
        let pool = rayon::ThreadPoolBuilder::new().num_threads(*jobs).build()?;
        let pool_ms = milliseconds(start);
        let mut calibrated = Vec::new();
        for mode in &args.modes {
            let (evaluations, calibration) = workload.calibrate(&pool, *mode, args.sample_ms)?;
            calibrated.push((*mode, evaluations));
            worker_setups.push(json!({"jobs":jobs,"mode":mode.name(),"pool_setup_ms":pool_ms,"calibration":calibration}));
        }
        for repeat in 0..args.repeats {
            for offset in 0..calibrated.len() {
                let (mode, evaluations) = calibrated[(offset + repeat) % calibrated.len()];
                let start = Instant::now();
                let checksum = workload.run(&pool, mode, evaluations)?;
                let elapsed = start.elapsed().as_secs_f64();
                assert_eq!(checksum, workload.expected(evaluations), "sample checksum");
                samples.push(json!({"jobs":jobs,"mode":mode.name(),"repeat":repeat,
                    "evaluations":evaluations,"elapsed_ms":elapsed*1000.0,
                    "evaluations_per_second":evaluations as f64/elapsed,"checksum":format!("{checksum:016x}")}));
            }
        }
    }
    let selected_fingerprints = selections
        .iter()
        .map(BuildSelection::fingerprint)
        .collect::<Vec<_>>();
    let class_ids = selections
        .iter()
        .map(|s| s.candidate.class_id.clone())
        .collect::<BTreeSet<_>>();
    let ascendancies = selections
        .iter()
        .map(|s| s.candidate.ascendancy_id.clone())
        .collect::<BTreeSet<_>>();
    let trees = selections
        .iter()
        .map(|s| serde_json::to_string(&s.allocation(catalog.catalog()).unwrap()).unwrap())
        .collect::<BTreeSet<_>>();
    let equipment = selections
        .iter()
        .map(|s| serde_json::to_string(&s.candidate.equipment).unwrap())
        .collect::<BTreeSet<_>>();
    let supports = handles
        .iter()
        .map(|h| h.support_keys().to_vec())
        .collect::<BTreeSet<_>>();
    let attributes = selections
        .iter()
        .flat_map(|s| s.attribute_options.values().copied())
        .collect::<BTreeSet<_>>();
    let executable = std::env::current_exe()?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema_version":1,"status":"developer_native_assembly_benchmark","diagnostic_only":true,
            "scope":"bounded_varied_admitted_mace_builds_not_whole_optimizer_or_full_game_coverage",
            "candidate_set":candidate_set,"receiving_defence":include_receiving,"local_armour":args.local_armour,
            "checksum_scope":if include_receiving { "metric_snapshot_and_all_13_receiving_output_fields" } else { "metric_snapshot" },
            "metric_ids":poe_optimizer_native::metric_catalog().into_iter().map(|metric|metric.id).collect::<Vec<_>>(),
            "fixed_template":template_path,"template_sha256":hash(template.as_bytes()),
            "equipment_source":equipment_path,"equipment_source_fields_used":["equipment"],
            "equipment_source_sha256":hash(equipment_json.as_bytes()),"benchmark_source_sha256":hash(SOURCE.as_bytes()),
            "executable_sha256":hash(&std::fs::read(executable)?),"backend":backend.identity(),
            "data_identity":snapshot.identity(),"data_manifest":snapshot.package().manifest,"data_trust":snapshot.trust(),
            "catalog_identity":catalog.catalog().identity,"cpu_label":args.cpu_label,
            "available_parallelism":std::thread::available_parallelism()?.get(),"os":std::env::consts::OS,
            "arch":std::env::consts::ARCH,"debug_assertions":cfg!(debug_assertions),
            "constraints":constraints,"attribute_locks":AttributeOptionLocks::default(),
            "input_generation":"legacy reviewed class/tree choices plus one source-connected attribute path per class; all 0..2-support loadouts, rotating two weapon/amulet/optional-armour selections per tree/loadout",
            "corpus":{"proposed":structural_candidates,"admitted":handles.len(),"rejected":structural_candidates-handles.len(),
                "rejection_counts":rejected,"selection_fingerprints_sha256":hash(&serde_json::to_vec(&selected_fingerprints)?),
                "class_ids":class_ids,"ascendancies":ascendancies,"distinct_allocations":trees.len(),
                "distinct_equipment_selections":equipment.len(),"distinct_support_loadouts":supports.len(),
                "attribute_options":attributes,"distinct_output_checksums":distinct_output_checksums},
            "setup":{"dataset_ms":dataset_ms,"catalog_ms":catalog_ms,"domain_ms":domain_ms,
                "prepared_components_ms":prepared_ms,"proposal_generation_ms":proposal_ms,
                "admission_ms":admission_setup_ms,"admission_attempts":structural_candidates,
                "initial_measure_ms":initial_measure_ms,"initial_measure_calculations":handles.len(),
                "full_document_equivalence_ms":validation_ms,"full_document_equivalence_pairs":validation_indices.len(),
                "complete_receiving_equivalence_pairs":if include_receiving { validation_indices.len() } else { 0 },
                "prepared_components_parse_source_once":true},
            "footprints":{"catalog":catalog.footprint(),"prepared":prepared.footprint(),
                "actor_scratch_inline_bytes":ActorScratch::storage_bytes(),
                "benchmark_selection_inline_bytes":std::mem::size_of_val(selections.as_slice()),
                "benchmark_admitted_handle_inline_bytes":std::mem::size_of_val(handles.as_slice()),
                "benchmark_checksum_bytes":std::mem::size_of_val(workload.checksums.as_slice()),
                "excludes":"shared dataset, owned selection/tree/item strings and collections, map/Arc/allocator metadata, thread stacks; not process peak memory"},
            "target_sample_ms":args.sample_ms,"worker_setups":worker_setups,"samples":samples,
            "checksums_match_preflight_for_every_calibration_and_sample":true,
            "limitations":[
                "Admission mode includes candidate cloning, structural validation/resolution, actor program execution, requirements, owned handle allocation/destruction and fresh skill metrics; it allocates.",
                "Measure mode reuses already-admitted handles containing prepared actor resources and receiving defences. It excludes actor assembly/admission and is not whole-evaluator or optimizer throughput.",
                "Both timed modes omit XML, JSON, diagnostics, exports, deadline checks, owned measurement conversion, objective scoring and search proposal generation. Neither uses an evaluation-result cache.",
                "The benchmark retains a bounded input/handle corpus for repeatability; production component catalogs do not retain a Cartesian candidate-result cache.",
                "Only equipment is read from the example JSON. The fixed source template, generated corpus and explicit benchmark budgets govern this run; JSON objective/search settings are not interpreted.",
                "Reported footprint fields are partial capacity estimates; preparing admitted benchmark handles and catalogs is timed separately.",
                "Receiving-defence mode additionally reads and checksums all 13 prepared receiving fields, including Armour/Evasion ratings and uncapped resistance totals. That consumption overhead is included in both timed modes; the default retains its metric-only checksum.",
                "Calibration and worker startup are outside samples. Automatic iteration counts vary by API and worker count; checksums are compared with the exact rotating input sequence.",
                "Fresh full native document comparisons are setup evidence, not independent Path of Building parity; separate source/full-build tests provide that evidence."
            ]
        }))?
    );
    Ok(())
}
