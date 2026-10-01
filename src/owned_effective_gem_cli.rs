//! Offline host for checked, schema-bound Gem input rule fragments.
//! Publication does not install the fragments or change coverage in a release.
use poe_optimizer_data::owned_schema::{OwnedSchemaLimits, decode_schema_package};
use poe_optimizer_import::owned_effective_gem_recipe::{
    EffectiveGemRecipeInput, EffectiveGemRecipeLimits, compile_effective_gem_recipe,
};
use std::{
    error::Error,
    io::{self, Write},
    path::PathBuf,
};

#[derive(clap::Args)]
pub(crate) struct Args {
    /// Explicit active pre-support or support-preparation recipe policy JSON.
    input: PathBuf,
    /// Exact definition schema declaring all owners, units, inputs and channels.
    #[arg(long)]
    definitions: PathBuf,
    /// New directory for programs.json and report.json; existing output is preserved.
    #[arg(long)]
    output: PathBuf,
}

pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let schema_limits = OwnedSchemaLimits::default();
    let mut remaining = schema_limits.max_wire_bytes;
    let schema = decode_schema_package(
        &super::owned_tree_cli::read(&args.definitions, &mut remaining)?,
        schema_limits,
    )?;
    let limits = EffectiveGemRecipeLimits::default();
    let mut remaining = limits.max_policy_bytes;
    let input: EffectiveGemRecipeInput =
        serde_json::from_slice(&super::owned_tree_cli::read(&args.input, &mut remaining)?)?;
    let compiled = compile_effective_gem_recipe(&input, &schema, limits)?;
    let programs = serde_json::to_vec(&compiled)?;
    let report = serde_json::to_vec_pretty(&serde_json::json!({
        "schema_version": 1,
        "document_kind": "owned_effective_gem_input_recipe",
        "definitions": compiled.definitions,
        "policy": compiled.policy,
        "programs": compiled.programs.len(),
        "work_used": compiled.work_used,
        "verification": {
            "operations": "compiled",
            "source_execution": "not_run",
            "release_installation": "not_run",
            "coverage": "unchanged",
            "final_active_inputs": "not_produced",
            "calculation": "not_run",
            "whole_build_parity": "not_established"
        }
    }))?;
    super::owned_recipe_cli::publish_artifacts(
        &args.output,
        [
            ("programs.json", programs.as_slice()),
            ("report.json", report.as_slice()),
        ],
    )?;
    let mut stdout = io::stdout().lock();
    stdout.write_all(&report)?;
    stdout.write_all(b"\n")?;
    Ok(())
}
