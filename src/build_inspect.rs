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
    /// Look up source references and inspect item loading using selected data; does not evaluate effects.
    #[arg(long)]
    with_definitions: bool,
    #[command(flatten)]
    data: crate::data_loading::DataArgs,
}
pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let source = super::read_input(&args.input)?;
    let imported = poe_optimizer_import::decode_build(&source)?;
    let projection = poe_optimizer_import::build_source::project_xml(&imported.xml)?;
    // Configuration has a separate typed grammar; its diagnostics must not erase
    // other containers or imply that preserved unknown records are evaluated.
    let config_projection = poe_optimizer_import::configuration::project_xml(&imported.xml);
    let configuration = match &config_projection {
        Ok(config) => {
            serde_json::json!({"status": "source_projected", "projection": config.diagnostic()})
        }
        Err(error) => serde_json::json!({"status": "not_projected", "error": error}),
    };
    let skill_projection = poe_optimizer_import::skill_source::project_xml(&imported.xml);
    let skills = match &skill_projection {
        Ok(skills) => serde_json::json!({"status": "source_projected", "projection": skills}),
        Err(error) => serde_json::json!({"status": "not_projected", "error": error}),
    };
    // Authored item loading instructions and jewel references remain independent
    // from equipment resolution, item parsing and passive-allocation admission.
    let item_projection = poe_optimizer_import::item_source::project_xml(&imported.xml);
    let items = match &item_projection {
        Ok(items) => serde_json::json!({"status": "source_projected", "projection": items}),
        Err(error) => serde_json::json!({"status": "not_projected", "error": error}),
    };
    let mut report = serde_json::json!({
        "schema_version": 3,
        "scope": "build_source_projection_v3",
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
        "skills": skills,
        "items": items,
        "verification": {
            "item_loading": "not_run",
            "equipment_resolution": "not_resolved",
            "passive_allocation": "not_checked",
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
            include_str!("../crates/poe-optimizer-import/src/skill_source.rs"),
            include_str!("../crates/poe-optimizer-import/src/item_source.rs"),
            include_str!("../crates/poe-optimizer-import/src/skill_definitions.rs"),
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
        // Definitions and item formatting share a portable snapshot, without a build evaluator.
        let snapshot = args.data.snapshot()?;
        report["definition_lookup"] = serde_json::json!({
            "data": snapshot.identity(),
            "data_trust": snapshot.trust(),
            "game_mechanics": "not_evaluated",
            "configuration": match &config_projection {
                Ok(config) => serde_json::json!({
                    "status": "looked_up",
                    "lookup": poe_optimizer_import::configuration_definitions::lookup_definitions(config, &snapshot)?
                }),
                Err(error) => serde_json::json!({"status": "not_looked_up", "source_error": error}),
            },
            "skills": match &skill_projection {
                Ok(skills) => serde_json::json!({
                    "status": "looked_up",
                    "lookup": poe_optimizer_import::skill_definitions::lookup_definitions(skills, &snapshot)?
                }),
                Err(error) => serde_json::json!({"status": "not_looked_up", "source_error": error}),
            },
            "items": match &item_projection {
                Ok(items) => {
                    let mut provider =
                        poe_optimizer_import::item_loading::BuiltinItemLoadProvider::new(&snapshot);
                    match poe_optimizer_import::item_loading::inspect(items, &snapshot, &mut provider) {
                        Ok(loaded) => serde_json::json!({"status": "load_reported", "report": loaded}),
                        Err(error) => serde_json::json!({"status": "not_reported", "error": error.to_string()}),
                    }
                },
                Err(error) => serde_json::json!({"status": "not_reported", "source_error": error}),
            }
        });
        report["definition_implementation_sha256"] =
            poe_optimizer_data::implementation_fingerprint().into();
        report["verification"]["item_loading"] =
            if report["definition_lookup"]["items"]["status"] == "load_reported" {
                "reported".into()
            } else {
                "not_reported".into()
            };
        report["item_loading_implementation_sha256"] =
            poe_optimizer_import::item_loading::implementation_fingerprint().into();
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
