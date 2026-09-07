use clap::{Parser, Subcommand};
use poe_optimizer_core::{
    MAX_WIRE_BYTES, PROTOCOL_VERSION, WorkerFailure, WorkerHello, WorkerRequest, WorkerResponse,
};
use poe_optimizer_core::{
    evaluation::*,
    metrics::{ActorScope, MetricQuery},
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
    long_about = "Import PoB XML/share codes and obtain fresh diagnostic PoB outputs through isolated mlua workers. Typed metric mappings and coverage are diagnostic; optimization is not implemented."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Action>,
}

#[derive(Subcommand)]
enum Action {
    /// List the typed measurement catalog without starting a calculation.
    Metrics,
    /// Decode and validate a PoB XML file or share code, preserving exact XML bytes.
    Import {
        input: PathBuf,
        /// Write decoded XML to a new file; existing files are never overwritten.
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Evaluate a build in a fresh supervised process; raw metrics are experimental.
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
            metrics,
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
                Some(path) => {
                    let mut bytes = Vec::new();
                    File::open(path)?
                        .take(64 * 1024 + 1)
                        .read_to_end(&mut bytes)?;
                    if bytes.len() > 64 * 1024 {
                        return Err("Evaluation options exceed 64 KiB".into());
                    }
                    serde_json::from_slice::<EvaluationOptions>(&bytes)?
                }
                None => EvaluationOptions::default(),
            };
            let backend =
                poe_optimizer_pob::backend::PobBackend::new(std::env::current_exe()?, pob);
            let engine = Engine::new(backend);
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
            let report = serde_json::json!({
                "schema_version": 2,
                "status": "experimental_evaluation",
                "source": { "format": imported.format, "xml_sha256": imported.sha256 },
                "evaluation": result,
            });
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
        Some(Action::Worker { pob, scratch }) => worker(&pob, &scratch)?,
    }
    Ok(())
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
