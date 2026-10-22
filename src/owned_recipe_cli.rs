//! Offline host publication. Semantic assembly remains in Import.
pub(crate) use poe_optimizer_host::publish_artifacts;
use poe_optimizer_import::owned_recipe::{OwnedRecipeLimits, decode_owned_recipe};
use std::{
    error::Error,
    fs::File,
    io::{self, Read, Write},
    path::PathBuf,
};

#[derive(clap::Args)]
pub(crate) struct Args {
    /// Persisted registry, schema, rule and routing recipe JSON.
    input: PathBuf,
    /// New directory for registry/schema/rules/routing/manifest JSON artifacts.
    #[arg(long)]
    output: PathBuf,
}

pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    if !cfg!(any(windows, target_os = "linux")) {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "atomic recipe directory publication is supported on Windows and Linux",
        )
        .into());
    }
    let limits = OwnedRecipeLimits::default();
    let mut input = Vec::new();
    File::open(&args.input)?
        .take(limits.max_wire_bytes as u64 + 1)
        .read_to_end(&mut input)?;
    let staged = decode_owned_recipe(&input, limits)?;
    publish_artifacts(
        &args.output,
        staged.artifacts().iter().map(|a| (a.name(), a.bytes())),
    )?;
    let manifest = staged
        .artifacts()
        .iter()
        .find(|a| a.name() == "manifest.json")
        .expect("fixed artifact set");
    let mut stdout = io::stdout().lock();
    stdout.write_all(manifest.bytes())?;
    stdout.write_all(b"\n")?;
    Ok(())
}
