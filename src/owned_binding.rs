//! Host I/O only; definition binding belongs to the portable Core/Data contracts.
use poe_optimizer_core::{
    owned_binding::{BindingLimits, bind_owned_request},
    owned_build::{OwnedDocument, decode_owned},
};
use poe_optimizer_data::owned_schema::{OwnedSchemaLimits, decode_schema_package};
use std::{
    error::Error,
    fs::File,
    io::{self, Read, Write},
    path::PathBuf,
};
#[derive(clap::Args)]
pub(crate) struct Args {
    /// A complete owned request envelope (build, scenario and ordered queries).
    input: PathBuf,
    /// Injected owned definition schema package; no default game data is loaded.
    #[arg(long)]
    schema: PathBuf,
    /// Save the diagnostic report to a new file, preserving existing files.
    #[arg(long)]
    output: Option<PathBuf>,
    /// Maximum bounded schema/record work for this cold binding pass.
    #[arg(long,default_value_t=BindingLimits::default().max_work)]
    max_work: usize,
}
pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let limits = BindingLimits {
        max_work: args.max_work,
        ..BindingLimits::default()
    };
    let mut bytes = Vec::new();
    File::open(&args.input)?
        .take(limits.input.max_wire_bytes as u64 + 1)
        .read_to_end(&mut bytes)?;
    let OwnedDocument::Request(request) = decode_owned(&bytes, limits.input)? else {
        return Err("binding requires a complete owned request document".into());
    };
    let schema_limits = OwnedSchemaLimits::default();
    bytes.clear();
    File::open(&args.schema)?
        .take(schema_limits.max_wire_bytes as u64 + 1)
        .read_to_end(&mut bytes)?;
    let schema = decode_schema_package(&bytes, schema_limits)?;
    let report = bind_owned_request(&schema, &request, limits)?;
    let bytes = serde_json::to_vec_pretty(&serde_json::json!({
        "schema_version":1,"document_kind":"definition_binding_report","binding":report,
        "verification":{"legality":"not_checked","calculation":"not_run"}
    }))?;
    if let Some(path) = args.output {
        super::write_new(&path, &bytes)?;
    }
    let mut stdout = io::stdout().lock();
    stdout.write_all(&bytes)?;
    stdout.write_all(b"\n")?;
    Ok(())
}
