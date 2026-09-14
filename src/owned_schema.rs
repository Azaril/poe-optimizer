//! File/stdout adapter; schema validation and indexing live in the portable Data crate.
use poe_optimizer_data::owned_schema::{
    OwnedSchemaLimits, decode_schema_package, encode_schema_package,
};
use std::{
    error::Error,
    fs::File,
    io::{self, Read, Write},
    path::PathBuf,
};

#[derive(clap::Args)]
pub(crate) struct Args {
    /// A versioned owned definition schema package JSON file.
    input: PathBuf,
    /// Save canonical JSON to a new file; existing files are preserved.
    #[arg(long)]
    canonical_output: Option<PathBuf>,
}

pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let limits = OwnedSchemaLimits::default();
    let mut bytes = Vec::new();
    File::open(&args.input)?
        .take(limits.max_wire_bytes as u64 + 1)
        .read_to_end(&mut bytes)?;
    let package = decode_schema_package(&bytes, limits)?;
    let canonical = encode_schema_package(&package, limits)?;
    if let Some(path) = &args.canonical_output {
        super::write_new(path, &canonical)?;
    }
    let report = serde_json::json!({
        "schema_version": 1,
        "document_kind": "definition_schema_package",
        "identity": package.identity(),
        "namespace": package.input().namespace,
        "definition_entries": package.input().definitions.len(),
        "slot_entries": package.input().slots.len(),
        "canonical_bytes": canonical.len(),
        "canonical_output": args.canonical_output,
        "verification": {
            "schema": "valid",
            "declared_topology": "potential_only",
            "build_binding": "not_run",
            "legality": "not_checked",
            "calculation": "not_run"
        }
    });
    let mut stdout = io::stdout().lock();
    serde_json::to_writer_pretty(&mut stdout, &report)?;
    stdout.write_all(b"\n")?;
    Ok(())
}
