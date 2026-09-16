//! Optional offline acquisition of finite data; no native evaluation dependency.
use std::{
    error::Error,
    io::{self, Write},
    path::{Path, PathBuf},
};

#[derive(clap::Args)]
pub(crate) struct Args {
    /// Pinned checkout used only for offline data acquisition.
    #[arg(long)]
    source_root: PathBuf,
    /// New directory receiving finite data and separate acquisition evidence.
    #[arg(long)]
    output: PathBuf,
}

fn publish(
    output: &Path,
    catalog: &[u8],
    evidence: &impl serde::Serialize,
) -> Result<(), Box<dyn Error>> {
    let mut bytes = serde_json::to_vec_pretty(evidence)?;
    bytes.push(b'\n');
    super::owned_recipe_cli::publish_artifacts(
        output,
        [
            ("catalog.json", catalog),
            ("evidence.json", bytes.as_slice()),
        ],
    )?;
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, evidence)?;
    stdout.write_all(b"\n")?;
    Ok(())
}

pub(crate) fn actors(args: Args) -> Result<(), Box<dyn Error>> {
    let result = poe_optimizer_pob::owned_actor_baselines::export_owned_actor_baselines(
        &args.source_root,
        Default::default(),
    )?;
    publish(&args.output, result.catalog_bytes(), result.evidence())
}

pub(crate) fn augments(args: Args) -> Result<(), Box<dyn Error>> {
    let result = poe_optimizer_pob::owned_augments::export_owned_augments(
        &args.source_root,
        Default::default(),
    )?;
    publish(&args.output, result.catalog_bytes(), result.evidence())
}
