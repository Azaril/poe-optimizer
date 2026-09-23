//! Offline preparation of explicitly selected socketed augment lines.
//! This command does not enable, scale, parse or evaluate the prepared effects.
use super::owned_tree_cli::read;
use poe_optimizer_import::{
    owned_augment_reconstruction::{
        AugmentReconstructionLimits, AugmentReconstructionPolicy, AugmentReconstructionRequest,
        reconstruct_owned_augments,
    },
    owned_augments::{AugmentCatalogLimits, decode_owned_augments},
};
use std::{
    error::Error,
    io::{self, Write},
    path::PathBuf,
};

#[derive(clap::Args)]
pub(crate) struct Args {
    /// Finite native catalog. No source scripts or PoB checkout are loaded.
    #[arg(long)]
    catalog: PathBuf,
    /// Explicit catalog digest, dialect and item-template bindings.
    #[arg(long)]
    policy: PathBuf,
    /// Explicit host, socketed occurrences, categories and unapplied context.
    #[arg(long)]
    request: PathBuf,
    /// New output directory receiving preparation.json; never overwritten.
    #[arg(long)]
    output: PathBuf,
    /// Lower the retained-representation and serialized-output bound.
    #[arg(long, default_value_t = 4 * 1024 * 1024)]
    max_output_bytes: usize,
}

pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let catalog_limits = AugmentCatalogLimits::default();
    let mut left = catalog_limits.max_catalog_bytes;
    let catalog = decode_owned_augments(&read(&args.catalog, &mut left)?, catalog_limits)?;
    let limits = AugmentReconstructionLimits {
        max_output_bytes: args.max_output_bytes,
        ..Default::default()
    };
    let mut left = limits.max_wire_bytes;
    let policy: AugmentReconstructionPolicy =
        serde_json::from_slice(&read(&args.policy, &mut left)?)?;
    let mut left = limits.max_wire_bytes;
    let request: AugmentReconstructionRequest =
        serde_json::from_slice(&read(&args.request, &mut left)?)?;
    let report = reconstruct_owned_augments(&catalog, &policy, &request, limits)?;
    // Reconstruction has already checked exact compact JSON before allocating.
    let bytes = serde_json::to_vec(&report)?;
    super::owned_recipe_cli::publish_artifacts(
        &args.output,
        [("preparation.json", bytes.as_slice())],
    )?;
    let mut stdout = io::stdout().lock();
    stdout.write_all(&bytes)?;
    stdout.write_all(b"\n")?;
    Ok(())
}
