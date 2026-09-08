//! Developer package authoring helper. Section digests do not confer reviewed trust.
use poe_optimizer_data::game_data::{GameDataLoader, GameDataPackage, LoadLimits, TrustPolicy};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 3 {
        return Err("usage: seal_package INPUT.json NEW_OUTPUT.json".into());
    }
    let limits = LoadLimits::default();
    if !std::fs::metadata(&args[1])?.is_file() {
        return Err("input must be a regular file".into());
    }
    let input = File::open(&args[1])?;
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
        .open(&args[2])?;
    output.write_all(&bytes)?;
    output.sync_all()?;
    println!("{} custom_unreviewed", snapshot.identity().content_sha256);
    Ok(())
}
