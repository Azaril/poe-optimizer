//! Explicit maintainer preparation of a source-audited package migration.
//! Writes a new directory only; never edits the compiled reviewed artifacts.
use poe_optimizer_pob::game_data::extract_pinned_game_data_for_review;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let destination = PathBuf::from(args.next().ok_or("provide a new output directory")?);
    if args.next().is_some() {
        return Err("expected exactly one output directory".into());
    }
    if destination.exists() {
        return Err("output directory already exists".into());
    }
    let source =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2");
    let extracted = extract_pinned_game_data_for_review(&source)?;
    let package = extracted.package.canonical_bytes()?;
    let tree = extracted.package.tree.canonical_bytes()?;
    fs::create_dir(&destination)?;
    for (name, bytes) in [("game-data", package), ("class-tree", tree)] {
        fs::write(
            destination.join(format!("{name}.sha256")),
            format!("{:x}\n", Sha256::digest(&bytes)),
        )?;
        fs::write(destination.join(format!("{name}.json")), bytes)?;
    }
    fs::write(
        destination.join("extraction.json"),
        serde_json::to_vec_pretty(&extracted.evidence)?,
    )?;
    println!(
        "Prepared pinned-source package for review: {}",
        destination.display()
    );
    Ok(())
}
