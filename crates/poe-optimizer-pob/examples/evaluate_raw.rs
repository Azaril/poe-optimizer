//! Internal unsupervised probe; use the CLI evaluate command for enforced deadlines.
use std::{fs, path::PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/path-of-building-poe2")
        .canonicalize()?;
    let xml = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/builds/pobarchives-Dfz36mCq.xml"),
    )?;
    std::env::set_current_dir(root.join("src"))?;
    let scratch = tempfile::tempdir()?;
    let snapshot = poe_optimizer_pob::runtime::evaluate(&root, scratch.path(), &xml)?;
    println!("{}", serde_json::to_string_pretty(&snapshot)?);
    Ok(())
}
