//! Developer package authoring helper. Section digests do not confer reviewed trust.
use clap::Parser;
use poe_optimizer_data::game_data::{GameDataLoader, GameDataPackage, LoadLimits, TrustPolicy};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
};

/// Refresh package section digests and write a new custom, unreviewed package.
#[derive(Parser)]
#[command(name = "seal_package")]
struct Args {
    /// Package JSON to seal.
    #[arg(value_name = "INPUT.json")]
    input: PathBuf,
    /// New output JSON file; an existing file is never overwritten.
    #[arg(value_name = "NEW_OUTPUT.json")]
    output: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let limits = LoadLimits::default();
    if !std::fs::metadata(&args.input)?.is_file() {
        return Err("input must be a regular file".into());
    }
    let input = File::open(&args.input)?;
    if !input.metadata()?.is_file() || input.metadata()?.len() > limits.max_bytes as u64 {
        return Err("input must be a regular file within the package byte limit".into());
    }
    let mut bytes = Vec::new();
    input
        .take(limits.max_bytes as u64 + 1)
        .read_to_end(&mut bytes)?;
    let mut package = GameDataPackage::decode_for_authoring(&bytes, &limits)?;
    package.refresh_section_digests()?;
    let bytes = package.canonical_bytes()?;
    let snapshot = GameDataLoader::from_bytes(&bytes, &TrustPolicy::AllowCustom, &limits)?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args.output)?;
    output.write_all(&bytes)?;
    output.sync_all()?;
    println!("{} custom_unreviewed", snapshot.identity().content_sha256);
    Ok(())
}
