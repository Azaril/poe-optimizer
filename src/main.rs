mod catalog_search;
mod mutation_search;

use clap::{Parser, Subcommand};
use poe_optimizer_core::{
    MAX_WIRE_BYTES, PROTOCOL_VERSION, WorkerFailure, WorkerHello, WorkerRequest, WorkerResponse,
};
use poe_optimizer_core::{
    evaluation::*,
    metrics::MetricDefinition,
    metrics::{ActorScope, MetricQuery},
    objective::{ObjectiveSpec, ScoringPolicy},
    options::EvaluationOptions,
};
use poe_optimizer_pob::import::{MAX_XML_BYTES, decode_build};
use std::{
    fs::File,
    io::{self, BufRead, Read, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

#[derive(Parser)]
#[command(
    name = "poe-optimizer",
    version,
    about = "Experimental Path of Exile 2 build evaluator",
    long_about = "Import PoB XML/share codes and obtain fresh diagnostic PoB outputs through isolated mlua workers. Includes experimental controlled weapon/support search and pinned tree-data extraction. General build optimization is not implemented; calculation and search coverage remain diagnostic."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Action>,
}

#[derive(Subcommand)]
enum Action {
    /// Search supplied normal-Mace weapon/support choices (experimental supported profile).
    SearchExperimental(mutation_search::Args),
    /// Export the pinned passive-tree data from an isolated, bounded extraction worker.
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
    /// Search the four calibrated weapon/support alternatives (developer harness).
    SearchCalibration(catalog_search::Args),
    /// List the typed measurement catalog without starting a calculation.
    Metrics,
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
    /// Evaluate a build in a fresh process and optionally assess a configured objective.
    Evaluate {
        input: PathBuf,
        #[arg(long, default_value = "vendor/path-of-building-poe2")]
        pob: PathBuf,
        /// Includes worker startup, calculation, export and process exit.
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
    Worker {
        #[arg(long)]
        pob: PathBuf,
        #[arg(long)]
        scratch: PathBuf,
    },
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
        Some(Action::SearchExperimental(args)) => mutation_search::run(args)?,
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
        Some(Action::TreeWorker {
            pob,
            tree_version,
            artifact,
            error_file,
        }) => {
            poe_optimizer_pob::tree_worker::worker(&pob, &tree_version, &artifact, &error_file)?;
        }
        Some(Action::SearchCalibration(args)) => catalog_search::run(args)?,
        None => {
            use clap::CommandFactory;
            Cli::command().print_help()?;
            println!();
        }
        Some(Action::Metrics) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&poe_optimizer_pob::metrics::catalog())?
            );
        }
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
            input,
            pob,
            timeout_seconds,
            output,
            export,
            options,
            mut metrics,
            objective,
            raw,
        }) => {
            // Reject conflicting/existing destinations before spending the evaluation budget.
            for path in [&output, &export].into_iter().flatten() {
                if path.exists() {
                    return Err(format!("Output already exists: {}", path.display()).into());
                }
            }
            let output_identity = output.as_deref().map(destination_identity).transpose()?;
            let export_identity = export.as_deref().map(destination_identity).transpose()?;
            if output_identity.is_some() && output_identity == export_identity {
                return Err("JSON and XML outputs need different paths".into());
            }
            let imported = decode_build(&read_input(&input)?)?;
            let options = match options {
                Some(path) => read_json::<EvaluationOptions>(&path, 64 * 1024)?,
                None => EvaluationOptions::default(),
            };
            let backend =
                poe_optimizer_pob::backend::PobBackend::new(std::env::current_exe()?, pob);
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
                "schema_version": 2,
                "status": "experimental_evaluation",
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
            if saved.schema_version != 2 || saved.status != "experimental_evaluation" {
                return Err("Assess requires an evaluation report with schema version 2".into());
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
