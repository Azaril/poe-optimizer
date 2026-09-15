//! CLI host for owned rule storage, compilation and explicit component probes.
//! A probe supplies resolved facts; it cannot certify a build or provider binding.
use poe_optimizer_core::{
    owned_definitions::OwnedDefinitionKey,
    owned_schema::{SchemaClosure, SchemaSubject},
};
use poe_optimizer_data::{
    owned_rules::{RuleStorageLimits, decode_rule_package, encode_rule_package},
    owned_schema::{OwnedSchemaLimits, decode_schema_package},
};
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, RuleFact, RuleLimits};
use serde::Deserialize;
use std::{
    error::Error,
    fs::File,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};
#[derive(clap::Args)]
pub(crate) struct Args {
    /// A caller-supplied owned rule package JSON file.
    input: PathBuf,
    /// Exact owned definition schema referenced by this rule package.
    #[arg(long)]
    definitions: PathBuf,
    /// Optional explicit owner/program/facts JSON for a component calculation.
    #[arg(long)]
    probe: Option<PathBuf>,
    /// Write canonical rule data to a new file after successful validation.
    #[arg(long)]
    canonical_output: Option<PathBuf>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Probe {
    owner: SchemaSubject,
    program: OwnedDefinitionKey,
    facts: Vec<RuleFact>,
}
fn read(path: &Path, limit: usize) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "owned rule input exceeds byte bound",
        ));
    }
    Ok(bytes)
}
pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let schema_limits = OwnedSchemaLimits::default();
    let definitions = decode_schema_package(
        &read(&args.definitions, schema_limits.max_wire_bytes)?,
        schema_limits,
    )?;
    let storage_limits = RuleStorageLimits::default();
    let rules = decode_rule_package(
        &read(&args.input, storage_limits.max_wire_bytes)?,
        &definitions,
        storage_limits,
    )?;
    let limits = RuleLimits::default();
    let compiled = CompiledRulePackage::compile(rules.input(), &definitions, limits)?;
    let probe = if let Some(path) = &args.probe {
        let probe: Probe = serde_json::from_slice(&read(path, limits.max_wire_bytes)?)?;
        Some(compiled.evaluate(
            &probe.owner,
            &probe.program,
            &probe.facts,
            &definitions,
            &mut compiled.new_scratch(),
        )?)
    } else {
        None
    };
    let canonical = encode_rule_package(&rules, storage_limits)?;
    if let Some(path) = &args.canonical_output {
        super::write_new(path, &canonical)?;
    }
    let report = serde_json::json!({
        "schema_version":1,"document_kind":"owned_rule_package_report",
        "package":rules.identity(),"compiled":compiled.identity(),"definitions":rules.definitions(),
        "resources":rules.resources(),"canonical_output":args.canonical_output,
        "partial_owners":rules.input().owners.iter().filter(|o|matches!(o.programs.closure,SchemaClosure::Partial{..})).count(),
        "probe":probe,
        "verification":{"envelope":"valid","operations":"compiled","provider_resolution":"not_run",
            "calculation":if args.probe.is_some(){"explicit_fact_component"}else{"not_run"},
            "whole_build_parity":"not_established"}
    });
    let mut stdout = io::stdout().lock();
    serde_json::to_writer_pretty(&mut stdout, &report)?;
    stdout.write_all(b"\n")?;
    Ok(())
}
