mod build_search;
#[cfg(feature = "pob")]
mod catalog_search;
mod configuration_inspect;
mod data_loading;
#[cfg(feature = "pob")]
mod game_data_extract;
mod mutation_search;
mod native_benchmark;

use clap::{Parser, Subcommand};
use poe_optimizer_core::MAX_WIRE_BYTES;
#[cfg(feature = "pob")]
use poe_optimizer_core::{
    PROTOCOL_VERSION, WorkerFailure, WorkerHello, WorkerRequest, WorkerResponse,
};
use poe_optimizer_core::{
    evaluation::*,
    metrics::MetricDefinition,
    metrics::{ActorScope, MetricQuery},
    objective::{ObjectiveSpec, ScoringPolicy},
    options::EvaluationOptions,
};
use poe_optimizer_import::{MAX_XML_BYTES, decode_build};
use std::{
    fs::File,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

#[cfg(feature = "pob")]
use std::io::BufRead;

#[derive(Parser)]
#[command(
    name = "poe-optimizer",
    version,
    about = "Experimental Path of Exile 2 build evaluator",
    long_about = "Import build XML/share codes and select a native Rust or optional PoB reference backend. Native coverage is currently restricted and rejects unsupported builds. Controlled search supports both backends; source-data extraction requires the PoB reference feature. Results remain diagnostic."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Action>,
}

#[derive(Subcommand)]
enum Action {
    /// Inspect authored configuration values without calculating or admitting build mechanics.
    InspectConfiguration(configuration_inspect::Args),
    /// Measure native fixed-input API throughput with a bounded local Rayon pool.
    BenchmarkNative(native_benchmark::Args),
    /// Search supplied Mace item/support choices (experimental supported profile).
    SearchExperimental(mutation_search::Args),
    /// Search connected passive allocations and supplied equipment with the native evaluator.
    SearchBuild(build_search::Args),
    /// Generate the current native game-data package and source evidence from pinned PoB.
    #[cfg(feature = "pob")]
    ExtractGameData(game_data_extract::Args),
    #[command(name = "__game-data-worker", hide = true)]
    #[cfg(feature = "pob")]
    GameDataWorker {
        #[arg(long)]
        pob: PathBuf,
        #[arg(long)]
        artifact: PathBuf,
        #[arg(long)]
        error_file: PathBuf,
    },
    /// Export the pinned passive-tree data from an isolated, bounded extraction worker.
    #[cfg(feature = "pob")]
    ExtractTree {
        #[arg(long, default_value = "vendor/path-of-building-poe2")]
        pob: PathBuf,
        #[arg(long, default_value = "0_5")]
        tree_version: String,
        #[arg(long, default_value_t = 30)]
        timeout_seconds: u64,
        #[arg(long)]
        output: PathBuf,
    },
    #[command(name = "__tree-worker", hide = true)]
    #[cfg(feature = "pob")]
    TreeWorker {
        #[arg(long)]
        pob: PathBuf,
        #[arg(long)]
        tree_version: String,
        #[arg(long)]
        artifact: PathBuf,
        #[arg(long)]
        error_file: PathBuf,
    },
    /// Compare a supplied catalog of complete builds (developer diagnostic harness).
    #[cfg(feature = "pob")]
    SearchCalibration(catalog_search::Args),
    /// List the typed measurement catalog without starting a calculation.
    Metrics {
        #[command(flatten)]
        data: data_loading::DataArgs,
        #[arg(long, value_enum, default_value_t = default_backend())]
        backend: BackendChoice,
    },
    /// Decode and validate a PoB XML file or share code, preserving exact XML bytes.
    Import {
        input: PathBuf,
        /// Write decoded XML to a new file; existing files are never overwritten.
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Assess a saved evaluation against an objective without starting a calculation.
    Assess {
        input: PathBuf,
        #[arg(long)]
        objective: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Evaluate with the selected calculation backend and optionally assess an objective.
    Evaluate {
        #[command(flatten)]
        data: data_loading::DataArgs,
        input: PathBuf,
        #[arg(long, value_enum, default_value_t = default_backend())]
        backend: BackendChoice,
        #[arg(long, default_value = "vendor/path-of-building-poe2")]
        pob: PathBuf,
        /// Shared calculation deadline; includes process startup when using the PoB backend.
        #[arg(long, default_value_t = 30)]
        timeout_seconds: u64,
        /// Save the JSON snapshot instead of writing it to stdout.
        #[arg(long)]
        output: Option<PathBuf>,
        /// Also save the normalized build export to a new file.
        #[arg(long)]
        export: Option<PathBuf>,
        /// JSON document containing explicit skill selection and/or encounter overrides.
        #[arg(long)]
        options: Option<PathBuf>,
        /// Request a typed metric, e.g. player.life or minion.selected_hit_dps; repeatable.
        #[arg(long = "metric", value_parser = parse_metric)]
        metrics: Vec<MetricQuery>,
        /// Assess a scalar objective and typed constraints from a JSON specification.
        #[arg(long)]
        objective: Option<PathBuf>,
        /// Include backend-specific raw diagnostic attachments in the JSON result.
        #[arg(long)]
        raw: bool,
    },
    #[command(name = "__worker", hide = true)]
    #[cfg(feature = "pob")]
    Worker {
        #[arg(long)]
        pob: PathBuf,
        #[arg(long)]
        scratch: PathBuf,
    },
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum BackendChoice {
    Native,
    #[cfg(feature = "pob")]
    Pob,
}
fn default_backend() -> BackendChoice {
    #[cfg(feature = "pob")]
    {
        BackendChoice::Pob
    }
    #[cfg(not(feature = "pob"))]
    {
        BackendChoice::Native
    }
}
fn make_backend(
    selection: BackendChoice,
    _pob: PathBuf,
    data: &data_loading::DataArgs,
) -> Result<Box<dyn CalculationBackend + Send + Sync>, Box<dyn std::error::Error>> {
    Ok(match selection {
        BackendChoice::Native => Box::new(data.backend()?),
        #[cfg(feature = "pob")]
        BackendChoice::Pob => {
            if data.is_selected() {
                return Err("--data is supported only by the native backend".into());
            }
            Box::new(poe_optimizer_pob::backend::PobBackend::new(
                std::env::current_exe()?,
                _pob,
            ))
        }
    })
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Some(Action::BenchmarkNative(args)) => native_benchmark::run(args)?,
        Some(Action::SearchExperimental(args)) => mutation_search::run(args)?,
        Some(Action::SearchBuild(args)) => build_search::run(args)?,
        #[cfg(feature = "pob")]
        Some(Action::ExtractGameData(args)) => game_data_extract::run(args)?,
        #[cfg(feature = "pob")]
        Some(Action::GameDataWorker {
            pob,
            artifact,
            error_file,
        }) => {
            poe_optimizer_pob::game_data_worker::worker(&pob, &artifact, &error_file)?;
        }
        #[cfg(feature = "pob")]
        Some(Action::ExtractTree {
            pob,
            tree_version,
            timeout_seconds,
            output,
        }) => {
            if output.exists() {
                return Err("Output already exists".into());
            }
            destination_identity(&output)?;
            let snapshot = poe_optimizer_pob::tree_worker::extract_tree(
                &std::env::current_exe()?,
                &pob,
                &tree_version,
                std::time::Duration::from_secs(timeout_seconds),
            )?;
            let fingerprint = snapshot.sha256()?;
            write_new(&output, &serde_json::to_vec_pretty(&snapshot)?)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "schema_version":1, "status":"extracted_source_data", "snapshot_sha256":fingerprint,
                    "classes":snapshot.classes.len(), "ascendancies":snapshot.ascendancies.len(), "nodes":snapshot.nodes.len(),
                    "dangling_connections":snapshot.dangling_connections.len(), "unsupported_mechanics":snapshot.unsupported_mechanics,
                    "source":snapshot.identity, "output":output
                }))?
            );
        }
        #[cfg(feature = "pob")]
        Some(Action::TreeWorker {
            pob,
            tree_version,
            artifact,
            error_file,
        }) => {
            poe_optimizer_pob::tree_worker::worker(&pob, &tree_version, &artifact, &error_file)?;
        }
        #[cfg(feature = "pob")]
        Some(Action::SearchCalibration(args)) => catalog_search::run(args)?,
        None => {
            use clap::CommandFactory;
            Cli::command().print_help()?;
            println!();
        }
        Some(Action::Metrics { backend, data }) => {
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &make_backend(backend, PathBuf::new(), &data)?
                        .capabilities()
                        .metrics
                )?
            );
        }
        Some(Action::InspectConfiguration(args)) => configuration_inspect::run(args)?,
        Some(Action::Import { input, output }) => {
            let imported = decode_build(&read_input(&input)?)?;
            if let Some(path) = output {
                write_new(&path, imported.xml.as_bytes())?;
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "format": imported.format,
                    "xml_sha256": imported.sha256,
                    "xml_bytes": imported.xml.len(),
                    "validation": "container_only"
                }))?
            );
        }
        Some(Action::Evaluate {
            data,
            input,
            backend,
            pob,
            timeout_seconds,
            output,
            export,
            options,
            mut metrics,
            objective,
            raw,
        }) => {
            // Export metadata preserves the selected dataset even though PoB XML has no data-package field.
            let export_manifest = if matches!(backend, BackendChoice::Native) {
                export.as_ref().map(|path| {
                    let mut name = path.as_os_str().to_os_string();
                    name.push(".data.json");
                    PathBuf::from(name)
                })
            } else {
                None
            };
            // Reject conflicting/existing destinations before spending the evaluation budget.
            for path in [&output, &export, &export_manifest].into_iter().flatten() {
                if path.exists() {
                    return Err(format!("Output already exists: {}", path.display()).into());
                }
            }
            let output_identity = output.as_deref().map(destination_identity).transpose()?;
            let export_identity = export.as_deref().map(destination_identity).transpose()?;
            if output_identity.is_some() && output_identity == export_identity {
                return Err("JSON and XML outputs need different paths".into());
            }
            if let Some(path) = &export_manifest {
                let metadata_identity = destination_identity(path)?;
                if output_identity.as_ref() == Some(&metadata_identity)
                    || export_identity.as_ref() == Some(&metadata_identity)
                {
                    return Err("Data metadata, JSON and XML outputs need different paths".into());
                }
            }
            let imported = decode_build(&read_input(&input)?)?;
            let options = match options {
                Some(path) => read_json::<EvaluationOptions>(&path, 64 * 1024)?,
                None => EvaluationOptions::default(),
            };
            let data_started = std::time::Instant::now();
            let backend = make_backend(backend, pob, &data)?;
            let data_load_ms = data_started.elapsed().as_secs_f64() * 1000.0;
            let engine = Engine::new(backend);
            let policy = objective
                .map(|path| {
                    read_json::<ObjectiveSpec>(&path, 64 * 1024)?
                        .compile(&engine.capabilities().metrics)
                        .map_err(Box::<dyn std::error::Error>::from)
                })
                .transpose()?;
            if !metrics.is_empty()
                && let Some(policy) = &policy
            {
                for query in policy.required_metrics() {
                    if !metrics.contains(&query) {
                        metrics.push(query);
                    }
                }
            }
            let mut result = engine.evaluate(
                &EvaluationRequest {
                    build: BuildDocument {
                        format: BuildFormat::PathOfBuilding2Xml,
                        content: imported.xml,
                    },
                    options,
                    metrics,
                },
                EvaluationBudget {
                    timeout_ms: timeout_seconds
                        .checked_mul(1000)
                        .ok_or("Timeout is too large")?,
                },
            )?;
            if !raw {
                result.attachments.clear();
            }
            let assessment = policy
                .as_ref()
                .map(|policy| policy.assess(&result.measurements))
                .transpose()?;
            let mut report = serde_json::json!({
                "schema_version": 3,
                "status": "experimental_evaluation",
                "initialization": {"backend_and_data_ms": data_load_ms},
                "source": { "format": imported.format, "xml_sha256": imported.sha256 },
                "evaluation": result,
            });
            if let Some(assessment) = assessment {
                report["objective_assessment"] = serde_json::to_value(assessment)?;
            }
            let mut json = serde_json::to_vec_pretty(&report)?;
            json.push(b'\n');
            if let Some(path) = output {
                write_new(&path, &json)?;
            } else {
                io::stdout().write_all(&json)?;
            }
            if let Some(path) = export {
                let document = result
                    .exports
                    .first()
                    .ok_or("Backend did not provide a build export")?;
                write_new(&path, document.content.as_bytes())?;
                if let Some(metadata_path) = export_manifest {
                    use sha2::{Digest, Sha256};
                    let metadata = serde_json::json!({
                        "schema_version":1, "status":"native_export_data",
                        "backend":result.backend, "warnings":result.warnings,
                        "xml_sha256":format!("{:x}",Sha256::digest(document.content.as_bytes())),
                        "package_path_hint":data.data, "uses_packaged_default":data.data.is_none(),
                        "reload_requirement":"Load a package matching backend.data before evaluating this XML; the path hint is not identity or trust."
                    });
                    let mut bytes = serde_json::to_vec_pretty(&metadata)?;
                    bytes.push(b'\n');
                    write_new(&metadata_path, &bytes)?;
                }
            }
        }
        Some(Action::Assess {
            input,
            objective,
            output,
        }) => {
            if output.as_ref().is_some_and(|path| path.exists()) {
                return Err("Output already exists".into());
            }
            let saved: SavedEvaluation = read_json(&input, MAX_WIRE_BYTES)?;
            if ![2, 3].contains(&saved.schema_version) || saved.status != "experimental_evaluation"
            {
                return Err(
                    "Assess requires an evaluation report with schema version 2 or 3".into(),
                );
            }
            if saved.schema_version == 3
                && saved.evaluation.backend.id == "native-poe2"
                && saved.evaluation.backend.data.is_none()
            {
                return Err("Schema 3 native evaluation requires a data identity".into());
            }
            saved.evaluation.validate_recorded()?;
            // Use the recorded metric versions, never today's PoB catalog or a Lua process.
            // A filtered report can only assess metrics it actually recorded.
            let catalog: Vec<_> = saved
                .evaluation
                .measurements
                .iter()
                .map(|measurement| MetricDefinition {
                    id: measurement.query.id.clone(),
                    unit: measurement.unit,
                    actors: vec![measurement.query.actor],
                    schema_version: measurement.schema_version,
                    description: "Recorded evaluation measurement".into(),
                })
                .collect();
            let policy = read_json::<ObjectiveSpec>(&objective, 64 * 1024)?.compile(&catalog)?;
            let assessment = policy.assess(&saved.evaluation.measurements)?;
            let report = serde_json::json!({
                "schema_version": 1, "status": "diagnostic_objective_assessment",
                "source": saved.source,
                "backend": saved.evaluation.backend,
                "build": saved.evaluation.build,
                "context": saved.evaluation.context,
                "coverage": saved.evaluation.coverage,
                "evaluation_diagnostic_only": saved.evaluation.diagnostic_only,
                "warnings": saved.evaluation.warnings,
                "objective_assessment": assessment,
            });
            let mut bytes = serde_json::to_vec_pretty(&report)?;
            bytes.push(b'\n');
            if let Some(path) = output {
                write_new(&path, &bytes)?;
            } else {
                io::stdout().write_all(&bytes)?;
            }
        }
        #[cfg(feature = "pob")]
        Some(Action::Worker { pob, scratch }) => worker(&pob, &scratch)?,
    }
    Ok(())
}

#[derive(serde::Deserialize)]
struct SavedEvaluation {
    schema_version: u32,
    status: String,
    source: serde_json::Value,
    evaluation: EvaluationResult,
}

fn read_json<T: serde::de::DeserializeOwned>(
    path: &Path,
    limit: usize,
) -> Result<T, Box<dyn std::error::Error>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(format!("JSON input exceeds {limit} bytes").into());
    }
    Ok(serde_json::from_slice(&bytes)?)
}

fn read_input(path: &Path) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(MAX_XML_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_XML_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Build source exceeds the XML input limit",
        ));
    }
    Ok(bytes)
}

fn destination(path: &Path) -> io::Result<PathBuf> {
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Output needs a file name"))?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    Ok(parent.canonicalize()?.join(name))
}

fn destination_identity(path: &Path) -> io::Result<PathBuf> {
    let path = destination(path)?;
    #[cfg(windows)]
    let path = PathBuf::from(path.to_string_lossy().to_lowercase());
    Ok(path)
}

fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let path = destination(path)?;
    let mut file = tempfile::NamedTempFile::new_in(path.parent().expect("absolute output parent"))?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    file.persist_noclobber(path).map_err(|error| error.error)?;
    Ok(())
}
#[cfg(feature = "pob")]
fn worker(pob: &Path, scratch: &Path) -> Result<(), Box<dyn std::error::Error>> {
    send_json(&WorkerHello {
        protocol_version: PROTOCOL_VERSION,
        backend: "mlua-luajit".into(),
    })?;
    let mut line = Vec::new();
    io::stdin()
        .lock()
        .take(MAX_WIRE_BYTES as u64 + 1)
        .read_until(b'\n', &mut line)?;
    if line.len() > MAX_WIRE_BYTES || !line.ends_with(b"\n") {
        return Err("Missing, oversized or unterminated worker request".into());
    }
    let request: WorkerRequest = serde_json::from_slice(&line)?;
    let result = if request.protocol_version != PROTOCOL_VERSION {
        Err(WorkerFailure {
            code: "protocol_version".into(),
            message: "Unsupported protocol version".into(),
        })
    } else {
        poe_optimizer_pob::runtime::evaluate_with_options(
            pob,
            scratch,
            &request.xml,
            &request.options,
        )
        .map_err(|error| WorkerFailure {
            code: "evaluation_failed".into(),
            message: error.to_string(),
        })
    };
    send_json(&WorkerResponse {
        protocol_version: PROTOCOL_VERSION,
        request_id: request.request_id,
        result,
    })?;
    Ok(())
}

#[cfg(feature = "pob")]
fn send_json(value: &impl serde::Serialize) -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = serde_json::to_vec(value)?;
    bytes.push(b'\n');
    if bytes.len() > MAX_WIRE_BYTES {
        return Err("Worker response exceeds protocol limit".into());
    }
    let mut stdout = io::stdout().lock();
    stdout.write_all(&bytes)?;
    stdout.flush()?;
    Ok(())
}

fn parse_metric(value: &str) -> Result<MetricQuery, String> {
    let (actor, id) = value
        .split_once('.')
        .ok_or("Use player.metric_id or minion.metric_id")?;
    let actor = match actor {
        "player" => ActorScope::Player,
        "minion" => ActorScope::SelectedMinion,
        _ => return Err("Metric actor must be player or minion".into()),
    };
    if id.is_empty() {
        return Err("Metric ID must not be empty".into());
    }
    Ok(MetricQuery {
        actor,
        id: id.into(),
    })
}
