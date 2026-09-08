//! Developer benchmark of mixed candidates across native API layers.
//! Run with --release --no-default-features; JSON keeps setup, checksum and scope explicit.
use clap::{Parser, ValueEnum};
use poe_optimizer_core::{
    evaluation::*,
    metrics::{ActorScope, MetricMeasurement, MetricQuery},
    options::EvaluationOptions,
};
use poe_optimizer_data::{
    class_tree,
    game_data::{GameDataLoader, LoadLimits, TrustPolicy, bundled_snapshot},
};
use poe_optimizer_import::controlled_mace::{
    ControlledMaceCatalog, MaceSupportLoadout, NormalMaceAlternative,
};
use poe_optimizer_native::{CompiledGameData, HostClock, NativeBackend, NativeCalculation};
use rayon::prelude::*;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{error::Error, hint::black_box, path::PathBuf, sync::Arc, time::Instant};
#[derive(Clone, Copy, ValueEnum)]
enum CandidateSet {
    Normal,
    LocalWeapons,
}
#[derive(Parser)]
struct Args {
    /// Select supplied weapon/support axes; template, trees and benchmark metrics stay fixed.
    #[arg(long, value_enum, default_value = "normal")]
    candidate_set: CandidateSet,
    #[arg(long, default_value_t = 20_000)]
    evaluations: usize,
    #[arg(long, default_value_t = 3)]
    repeats: usize,
    #[arg(long, value_delimiter = ',', default_value = "1,2,4")]
    jobs: Vec<usize>,
    #[arg(
        long,
        value_delimiter = ',',
        default_value = "document,prepared_result,pure_calculation,typed_snapshot,typed_owned_measurements"
    )]
    modes: Vec<String>,
    #[arg(long)]
    cpu_label: Option<String>,
    #[arg(long)]
    data: Option<PathBuf>,
}
fn measurement_dps(values: &[MetricMeasurement]) -> f64 {
    values
        .iter()
        .find(|m| m.query.id == "selected_hit_dps")
        .unwrap()
        .value
        .finite()
        .unwrap()
}
fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    if args.evaluations == 0
        || args.evaluations > 1_000_000
        || !(1..=10).contains(&args.repeats)
        || args.jobs.is_empty()
        || args.jobs.len() > 16
        || args.jobs.iter().any(|j| !(1..=256).contains(j))
    {
        return Err(
            "Require 1..1000000 evaluations, 1..10 repeats and 1..16 worker counts in 1..256"
                .into(),
        );
    }
    let allowed_modes = [
        "document",
        "prepared_result",
        "pure_calculation",
        "typed_snapshot",
        "typed_owned_measurements",
    ];
    if args.modes.is_empty()
        || args.modes.len() > allowed_modes.len()
        || args
            .modes
            .iter()
            .any(|mode| !allowed_modes.contains(&mode.as_str()))
        || args
            .modes
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != args.modes.len()
    {
        return Err("Require a nonempty set of distinct known benchmark modes".into());
    }
    let start = Instant::now();
    let snapshot = Arc::new(if let Some(path) = &args.data {
        let bytes = std::fs::read(path)?;
        GameDataLoader::from_bytes(&bytes, &TrustPolicy::AllowCustom, &LoadLimits::default())?
    } else {
        bundled_snapshot()?
    });
    let backend = Arc::new(NativeBackend::with_data(
        Arc::new(CompiledGameData::compile(snapshot.clone())?),
        HostClock,
    )?);
    let engine = Engine::new(SharedBackend::new(backend.clone()));
    let dataset_ms = start.elapsed().as_secs_f64() * 1000.0;
    // These fixtures supply only weapon and support axes. Their template, search
    // settings and objective are not benchmark inputs.
    let (candidate_set, candidate_source, candidate_json) = match args.candidate_set {
        CandidateSet::Normal => (
            "normal",
            "examples/mace-support-search.json",
            include_str!("mace-support-search.json"),
        ),
        CandidateSet::LocalWeapons => (
            "local-weapons",
            "examples/mace-local-weapon-search.json",
            include_str!("mace-local-weapon-search.json"),
        ),
    };
    let input: serde_json::Value = serde_json::from_str(candidate_json)?;
    let weapons: Vec<NormalMaceAlternative> = serde_json::from_value(input["weapons"].clone())?;
    let loadouts: Vec<MaceSupportLoadout> =
        serde_json::from_value(input["support_loadouts"].clone())?;
    let start = Instant::now();
    let catalog = ControlledMaceCatalog::with_tree_loadouts(
        snapshot.clone(),
        include_str!("../tests/fixtures/calibration/mace-wooden.xml").into(),
        weapons,
        loadouts,
        class_tree::selections(snapshot.tree())?,
    )?;
    let catalog_ms = start.elapsed().as_secs_f64() * 1000.0;
    let metrics = vec![MetricQuery {
        actor: ActorScope::Player,
        id: "selected_hit_dps".into(),
    }];
    let request = |build| EvaluationRequest {
        build,
        options: EvaluationOptions::default(),
        metrics: metrics.clone(),
    };
    let budget = EvaluationBudget { timeout_ms: 30_000 };
    let baseline = engine.evaluate(&request(catalog.template_build()), budget)?;
    let scenario = catalog.bind_native_baseline(&baseline, &backend.identity())?;
    let start = Instant::now();
    let components = catalog.native_components(&scenario, &backend.identity())?;
    let typed = backend.prepare_controlled_mace(&components, &metrics)?;
    let typed_ms = start.elapsed().as_secs_f64() * 1000.0;
    let start = Instant::now();
    let legal = catalog
        .alternatives()
        .iter()
        .filter(|a| catalog.validate_requirements(&a.candidate).is_ok())
        .collect::<Vec<_>>();
    if legal.is_empty() {
        return Err("Benchmark dataset has no legal candidates".into());
    }
    let handles = legal
        .iter()
        .map(|a| catalog.validated_native_candidate(&a.candidate, &components))
        .collect::<Result<Vec<_>, _>>()?;
    let handles_ms = start.elapsed().as_secs_f64() * 1000.0;
    let start = Instant::now();
    let requests = legal
        .iter()
        .map(|a| Ok(request(catalog.materialize(&a.candidate)?)))
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
    let document_materialization_ms = start.elapsed().as_secs_f64() * 1000.0;
    let start = Instant::now();
    let documents = requests
        .iter()
        .map(|r| backend.prepare(r))
        .collect::<Result<Vec<_>, _>>()?;
    let document_preparation_ms = start.elapsed().as_secs_f64() * 1000.0;
    // Compare every rotating input before timing. These validation calculations
    // are counted separately and are not presented as timed benchmark attempts.
    let validation_start = Instant::now();
    for (i, document) in documents.iter().enumerate() {
        let doc = match document.calculate()? {
            NativeCalculation::Mace(o) => o.hit_dps,
            _ => unreachable!(),
        };
        let prepared = typed.calculate(&handles[i])?.hit_dps;
        assert_eq!(doc, prepared, "candidate {}", legal[i].id);
    }
    let validation_ms = validation_start.elapsed().as_secs_f64() * 1000.0;
    let mut samples = Vec::new();
    let mut expected_checksum = None;
    for jobs in &args.jobs {
        let start = Instant::now();
        let pool = rayon::ThreadPoolBuilder::new().num_threads(*jobs).build()?;
        let pool_setup_ms = start.elapsed().as_secs_f64() * 1000.0;
        for repeat in 0..args.repeats {
            // Rotate API order to reduce one-direction thermal/warm-cache bias.
            let modes = &args.modes;
            for offset in 0..modes.len() {
                let mode = modes[(offset + repeat) % modes.len()].as_str();
                let start = Instant::now();
                let checksum = pool.install(|| {
                    (0..args.evaluations)
                        .into_par_iter()
                        .map(|iteration| {
                            let i = iteration % handles.len();
                            let dps = match mode {
                                "document" => measurement_dps(
                                    &black_box(
                                        engine.evaluate(black_box(&requests[i]), budget).unwrap(),
                                    )
                                    .measurements,
                                ),
                                "prepared_result" => measurement_dps(
                                    &black_box(
                                        backend
                                            .evaluate_prepared(black_box(&documents[i]), budget)
                                            .unwrap(),
                                    )
                                    .measurements,
                                ),
                                "pure_calculation" => {
                                    match black_box(documents[i].calculate().unwrap()) {
                                        NativeCalculation::Mace(o) => o.hit_dps,
                                        _ => unreachable!(),
                                    }
                                }
                                "typed_snapshot" => black_box(
                                    backend
                                        .evaluate_controlled_mace(
                                            &typed,
                                            black_box(&handles[i]),
                                            budget,
                                        )
                                        .unwrap(),
                                )
                                .values()
                                .last()
                                .unwrap()
                                .finite()
                                .unwrap(),
                                "typed_owned_measurements" => {
                                    let value = backend
                                        .evaluate_controlled_mace(
                                            &typed,
                                            black_box(&handles[i]),
                                            budget,
                                        )
                                        .unwrap();
                                    measurement_dps(&black_box(typed.snapshot_measurements(&value)))
                                }
                                _ => unreachable!(),
                            };
                            black_box(dps).to_bits()
                        })
                        .reduce(|| 0u64, u64::wrapping_add)
                });
                let elapsed = start.elapsed().as_secs_f64();
                if let Some(expected) = expected_checksum {
                    assert_eq!(checksum, expected, "{mode}/{jobs}/{repeat}");
                } else {
                    expected_checksum = Some(checksum);
                }
                samples.push(json!({"mode":mode,"jobs":jobs,"repeat":repeat,"evaluations":args.evaluations,"elapsed_ms":elapsed*1000.0,"evaluations_per_second":args.evaluations as f64/elapsed,"checksum":format!("{checksum:016x}"),"pool_setup_ms":pool_setup_ms}));
            }
        }
    }
    let executable = std::env::current_exe()?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema_version":1,"status":"developer_native_candidate_benchmark","diagnostic_only":true,
            "scope":"mixed_admitted_mace_candidates_not_general_game_or_optimizer_quality",
            "candidate_set":candidate_set,"candidate_source":candidate_source,
            "candidate_source_fields_used":["weapons","support_loadouts"],
            "fixed_template":"tests/fixtures/calibration/mace-wooden.xml",
            "tree_selections":"all_admitted_class_tree_selections_from_selected_data",
            "cpu_label":args.cpu_label,"available_parallelism":std::thread::available_parallelism()?.get(),
            "os":std::env::consts::OS,"arch":std::env::consts::ARCH,"debug_assertions":cfg!(debug_assertions),
            "executable_sha256":format!("{:x}",Sha256::digest(std::fs::read(executable)?)),
            "backend":backend.identity(),"data_trust":snapshot.trust(),"catalog":catalog.catalog().identity,
            "structural_candidates":catalog.alternatives().len(),"legal_candidates":handles.len(),
            "typed_footprint":typed.footprint(),
            "setup":{"dataset_ms":dataset_ms,"catalog_ms":catalog_ms,"typed_components_ms":typed_ms,"handles_ms":handles_ms,"document_materialization_ms":document_materialization_ms,"document_preparation_ms":document_preparation_ms,"baseline_calculations":1,"equivalence_calculations":2*handles.len(),"equivalence_ms":validation_ms},
            "storage":{"request_xml_bytes":requests.iter().map(|r|r.build.content.len()).sum::<usize>(),"candidate_handles_inline_bytes":std::mem::size_of_val(handles.as_slice()),"footprint_excludes":"shared data, import catalog, allocator metadata and Arc control blocks"},
            "checksum_matches_all_selected_modes_workers_and_repeats":true,"selected_modes":args.modes,"samples":samples,
            "limitations":["Candidate-set fixtures supply only weapons/support_loadouts; their template, objective, locks, neighborhood and tree_search settings are not used.","Pure calculation omits metrics and deadline checks; other modes include different API/result work.","Component capacity estimates are not process peak-memory measurements.","Whole-search/catalog costs and realistic build coverage remain separate evidence."]
        }))?
    );
    Ok(())
}
