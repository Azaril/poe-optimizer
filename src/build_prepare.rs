//! Native preparation through the same owner/view path as ordinary evaluation.
use poe_optimizer_core::{
    build_identity::BuildLineage, build_view::ViewRequest, options::EvaluationOptions,
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    selected_view::{ResolveLimits, resolve_view},
};
use poe_optimizer_native::{PreparationOutcome, PreparationRequest};
use std::{
    error::Error,
    io::{self, Write},
    path::PathBuf,
};
#[derive(clap::Args)]
pub(crate) struct Args {
    /// One caller-supplied PoB XML document or share code.
    input: PathBuf,
    /// Write to a new file. Existing files are never overwritten.
    #[arg(long)]
    output: Option<PathBuf>,
    /// Existing evaluation option schema (skill selection and encounter overrides).
    #[arg(long)]
    options: Option<PathBuf>,
    #[command(flatten)]
    data: crate::data_loading::DataArgs,
}
pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    if let Some(path) = &args.output {
        if path.exists() {
            return Err(format!("Output already exists: {}", path.display()).into());
        }
        super::destination_identity(path)?;
    }
    let imported = poe_optimizer_import::decode_build(&super::read_input(&args.input)?)?;
    let mut lineage = [0u8; 16];
    getrandom::fill(&mut lineage)?;
    let build = ImportedBuildInstance::from_decoded(
        imported,
        BuildLineage::from_bytes(lineage),
        InstanceImportLimits::default(),
    )?;
    let backend = args.data.backend()?;
    let options = match args.options {
        Some(path) => super::read_json::<EvaluationOptions>(&path, 64 * 1024)?,
        None => EvaluationOptions::default(),
    };
    let view = resolve_view(
        &build,
        backend.data().snapshot(),
        &ViewRequest::default(),
        ResolveLimits::default(),
    )?;
    let outcome = backend.prepare_view(&build, &view, &options, &[])?;
    let mut output = serde_json::json!({
        "schema_version":1,"scope":"native_preparation","backend":backend.identity(),
        "source_sha256":build.source_sha256(),"data_trust":backend.data().snapshot().trust(),
        "calculation":"not_run","whole_build_parity":"not_established",
        "requested":PreparationRequest { options: options.clone(), metric_queries: vec![] },
    });
    match outcome {
        PreparationOutcome::Ready(prepared) => {
            output["status"] = "ready_for_supported_native_metrics".into();
            output["selected_view"] = serde_json::to_value(prepared.selected_view())?;
            output["authored_skills"] = serde_json::to_value(prepared.authored_skills().report())?;
        }
        PreparationOutcome::Incomplete(report) => {
            output["status"] = "incomplete".into();
            output["preparation"] = serde_json::to_value(report)?;
        }
    }
    let bytes = serde_json::to_vec_pretty(&output)?;
    if let Some(path) = args.output {
        super::write_new(&path, &bytes)?;
    } else {
        let mut stdout = io::stdout().lock();
        stdout.write_all(&bytes)?;
        stdout.write_all(b"\n")?;
    }
    Ok(())
}
