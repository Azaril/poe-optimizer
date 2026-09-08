//! Source-only configuration evidence, independent of skill admission or calculation.
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    io::{self, Write},
    path::PathBuf,
};

#[derive(clap::Args)]
pub(crate) struct Args {
    /// One caller-supplied PoB build XML document or share code.
    input: PathBuf,
    /// Write a source-projection report to a new file instead of stdout.
    #[arg(long)]
    output: Option<PathBuf>,
    /// Look up authored settings in the selected configuration catalog; does not evaluate effects.
    #[arg(long)]
    with_definitions: bool,
    #[command(flatten)]
    data: crate::data_loading::DataArgs,
}

pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let source = super::read_input(&args.input)?;
    let imported = poe_optimizer_import::decode_build(&source)?;
    let projection = poe_optimizer_import::configuration::project_xml(&imported.xml)?;
    let mut report = serde_json::json!({
        "schema_version": 1,
        "scope": "configuration_source_projection_v1",
        "status": "source_projected",
        "input": {
            "format": imported.format,
            "input_sha256": format!("{:x}", Sha256::digest(&source)),
            "xml_sha256": imported.sha256,
            "input_bytes": source.len(),
            "xml_bytes": imported.xml.len()
        },
        "configuration": projection.diagnostic(),
        "verification": {
            "effective_configuration": "not_evaluated",
            "game_mechanics": "not_evaluated",
            "build_legality": "not_checked",
            "reference_calculation": "not_run"
        },
        "implementation_sha256": format!("{:x}", Sha256::digest(concat!(
            include_str!("configuration_inspect.rs"),
            include_str!("../crates/poe-optimizer-import/src/configuration.rs"),
            include_str!("../crates/poe-optimizer-import/src/configuration_definitions.rs"),
            include_str!("../crates/poe-optimizer-import/src/source_xml.rs"),
            include_str!("../crates/poe-optimizer-import/src/xml_compat.rs"),
            include_str!("../crates/poe-optimizer-import/src/lib.rs"),
            include_str!("../crates/poe-optimizer-core/src/options.rs"),
            include_str!("../Cargo.toml"),
            include_str!("../Cargo.lock")
        ).as_bytes()))
    });
    if args.with_definitions || args.data.data.is_some() {
        let snapshot = args.data.snapshot()?;
        let definitions = poe_optimizer_import::configuration_definitions::lookup_definitions(
            &projection,
            &snapshot,
        )?;
        report["definition_lookup"] = serde_json::to_value(definitions)?;
        report["definition_implementation_sha256"] =
            poe_optimizer_data::implementation_fingerprint().into();
    }
    let bytes = serde_json::to_vec_pretty(&report)?;
    if let Some(path) = args.output {
        super::write_new(&path, &bytes)?;
    } else {
        let mut stdout = io::stdout().lock();
        stdout.write_all(&bytes)?;
        stdout.write_all(b"\n")?;
    }
    Ok(())
}
