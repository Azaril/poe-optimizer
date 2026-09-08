//! Caller-driven structural evidence, independent of native mechanic coverage.
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
    /// Write the inspection report to a new file instead of stdout.
    #[arg(long)]
    output: Option<PathBuf>,
}
pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let source = super::read_input(&args.input)?;
    let imported = poe_optimizer_import::decode_build(&source)?;
    let projection = poe_optimizer_import::build_source::project_xml(&imported.xml)?;
    // Configuration has a separate typed grammar; its diagnostics must not erase
    // other containers or imply that preserved unknown records are evaluated.
    let configuration = match poe_optimizer_import::configuration::project_xml(&imported.xml) {
        Ok(config) => {
            serde_json::json!({"status": "source_projected", "projection": config.diagnostic()})
        }
        Err(error) => serde_json::json!({"status": "not_projected", "error": error}),
    };
    let report = serde_json::json!({
        "schema_version": 1,
        "scope": "build_source_projection_v1",
        "status": "source_projected",
        "input": {
            "format": imported.format,
            "input_sha256": format!("{:x}", Sha256::digest(&source)),
            "xml_sha256": imported.sha256,
            "input_bytes": source.len(),
            "xml_bytes": imported.xml.len()
        },
        "build": projection,
        "configuration": configuration,
        "verification": {
            "calculation_context": "not_resolved",
            "effective_configuration": "not_evaluated",
            "game_mechanics": "not_evaluated",
            "build_legality": "not_checked",
            "native_admission": "not_checked",
            "reference_calculation": "not_run"
        },
        "implementation_sha256": format!("{:x}", Sha256::digest(concat!(
            include_str!("build_inspect.rs"),
            include_str!("../crates/poe-optimizer-import/src/build_source.rs"),
            include_str!("../crates/poe-optimizer-import/src/configuration.rs"),
            include_str!("../crates/poe-optimizer-import/src/source_xml.rs"),
            include_str!("../crates/poe-optimizer-import/src/xml_compat.rs"),
            include_str!("../crates/poe-optimizer-import/src/lib.rs"),
            include_str!("../crates/poe-optimizer-core/src/options.rs"),
            include_str!("../Cargo.toml"),
            include_str!("../Cargo.lock")
        ).as_bytes()))
    });
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
