//! Filesystem host for source-to-owned action correspondence, without evaluation.
use super::owned_tree_cli::{invalid, read};
use poe_optimizer_core::build_identity::BuildLineage;
use poe_optimizer_import::{
    MAX_XML_BYTES,
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_release::OwnedReleaseLimits,
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
    owned_source_actions::{
        SourceActionCorrespondence, SourceActionCorrespondenceInput, SourceActionLimits,
        SourceActionRequest, SourceDirectActionRequest,
    },
};
use std::{
    error::Error,
    io::{self, Write},
    path::PathBuf,
};

#[derive(clap::Args)]
pub(crate) struct Args {
    /// File containing one caller-supplied PoB build XML document or share code.
    input: PathBuf,
    /// Validated owned release directory, including its artifact receipt.
    #[arg(long)]
    release: PathBuf,
    /// Compiled, release-bound source action correspondence JSON.
    #[arg(long)]
    correspondence: PathBuf,
    /// Exact physical or Direct locator and reference context, as declared by the adapter.
    #[arg(long)]
    request: PathBuf,
}

pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let release_limits = OwnedReleaseLimits::default();
    let mut left = release_limits.max_input_bytes;
    let release = super::owned_release_cli::load_release(&args.release, &mut left, release_limits)?;
    let limits = SourceActionLimits::default();
    let mut left = limits.max_wire_bytes;
    // Decode strict DTOs directly; a Value intermediary would hide duplicate keys.
    let input: SourceActionCorrespondenceInput =
        serde_json::from_slice(&read(&args.correspondence, &mut left)?)?;
    let direct = matches!(
        input,
        SourceActionCorrespondenceInput::PobManualDirectSingletonMinionActionsV1 { .. }
    );
    let correspondence = SourceActionCorrespondence::new(
        input,
        release.assembled().schema(),
        release.roles(),
        release.mapping(),
        limits,
    )?;
    let mut left = limits.max_wire_bytes;
    let request_bytes = read(&args.request, &mut left)?;
    let mut left = MAX_XML_BYTES;
    let decoded = decode_build(&read(&args.input, &mut left)?)?;
    let mut lineage = [0_u8; 16];
    getrandom::fill(&mut lineage)?;
    let source = ImportedBuildInstance::from_decoded(
        decoded,
        BuildLineage::from_bytes(lineage),
        InstanceImportLimits::default(),
    )?;
    let evidence = SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default())?;
    let report = if direct {
        let request: SourceDirectActionRequest = serde_json::from_slice(&request_bytes)?;
        serde_json::to_value(correspondence.resolve_direct(&evidence, &request)?)?
    } else {
        let request: SourceActionRequest = serde_json::from_slice(&request_bytes)?;
        serde_json::to_value(correspondence.resolve(&evidence, &request)?)?
    };
    let bytes = serde_json::to_vec(&serde_json::json!({
        "schema_version": 1,
        "document_kind": "owned_source_action_resolution",
        "release": release.receipt().input,
        "correspondence": correspondence.identity(),
        "source_sha256": source.source_sha256(),
        "source_execution": false,
        "calculation": false,
        "whole_build_parity": false,
        "report": report,
    }))?;
    // Import bounds the report; this fixed envelope adds only identities and flags.
    if bytes.len() > limits.max_output_bytes.saturating_add(1024) {
        return Err(invalid("source action report output byte limit"));
    }
    let mut stdout = io::stdout().lock();
    stdout.write_all(&bytes)?;
    stdout.write_all(b"\n")?;
    Ok(())
}
