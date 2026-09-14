//! Host I/O only. All owned-format semantics live in the portable core library.
use poe_optimizer_core::owned_build::{
    OwnedDocument, OwnedInputLimits, decode_owned, encode_owned,
};
use std::{
    error::Error,
    fs::File,
    io::{self, Read, Write},
    path::PathBuf,
};

#[derive(clap::Args)]
pub(crate) struct Args {
    /// An owned build, scenario, query or request JSON document.
    input: PathBuf,
    /// Save deterministic owned JSON to a new file; existing files are preserved.
    #[arg(long)]
    canonical_output: Option<PathBuf>,
}

pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let limits = OwnedInputLimits::default();
    let mut bytes = Vec::new();
    File::open(&args.input)?
        .take(limits.max_wire_bytes as u64 + 1)
        .read_to_end(&mut bytes)?;
    let document = decode_owned(&bytes, limits)?;
    let canonical = encode_owned(&document, limits)?;
    if let Some(path) = &args.canonical_output {
        super::write_new(path, &canonical)?;
    }
    let kind = match &document {
        OwnedDocument::Build(_) => "build",
        OwnedDocument::Scenario(_) => "scenario",
        OwnedDocument::Query(_) => "query",
        OwnedDocument::Request(_) => "request",
    };
    let report = serde_json::json!({
        "schema_version": 1,
        "document_kind": kind,
        "owned_input_schema_version": poe_optimizer_core::owned_build::OWNED_INPUT_SCHEMA_VERSION,
        "canonical_bytes": canonical.len(),
        "canonical_output": args.canonical_output,
        "verification": {
            "structure": "valid",
            "definitions": "not_bound",
            "legality": "not_checked",
            "calculation": "not_run"
        }
    });
    let mut stdout = io::stdout().lock();
    serde_json::to_writer_pretty(&mut stdout, &report)?;
    stdout.write_all(b"\n")?;
    Ok(())
}
