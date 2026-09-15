//! Host-only explicit successor publication; no source checkout or evaluation.
use poe_optimizer_core::owned_definitions::OwnedDefinitionKey;
use poe_optimizer_import::owned_successor::{
    NamedQuerySet, OWNED_SUCCESSOR_VERSION, SuccessorBundleInput, SuccessorBundleLimits,
    transition_owned_bundle,
};
use serde::de::DeserializeOwned;
use std::{
    error::Error,
    fs::File,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

#[derive(Clone)]
struct QueryPath {
    name: OwnedDefinitionKey,
    path: PathBuf,
}
fn query_path(value: &str) -> Result<QueryPath, String> {
    let (name, path) = value.split_once('=').ok_or("query set must be NAME=PATH")?;
    if path.is_empty() {
        return Err("query set path is empty".into());
    }
    Ok(QueryPath {
        name: OwnedDefinitionKey::new(name).map_err(|e| e.to_string())?,
        path: path.into(),
    })
}
#[derive(clap::Args)]
pub(crate) struct Args {
    /// Exact prior recipe for the carried import artifacts.
    input: PathBuf,
    /// Checked append-only successor recipe; existing schema declarations stay exact.
    #[arg(long)]
    successor: PathBuf,
    #[arg(long)]
    mapping: PathBuf,
    #[arg(long)]
    roles: PathBuf,
    #[arg(long)]
    normalization: PathBuf,
    #[arg(long)]
    rewards: PathBuf,
    /// Explicit successor-bound item line policy.
    #[arg(long)]
    items: PathBuf,
    /// Explicit successor-bound source layout policy.
    #[arg(long)]
    item_source: PathBuf,
    /// Independent ordered query list. Repeat with a unique safe name.
    #[arg(long = "query-set", value_parser = query_path, value_name = "NAME=PATH")]
    query_sets: Vec<QueryPath>,
    /// New directory; existing destinations are never replaced.
    #[arg(long)]
    output: PathBuf,
}
fn read<T: DeserializeOwned>(path: &Path, remaining: &mut usize) -> Result<T, Box<dyn Error>> {
    let mut bytes = vec![];
    File::open(path)?
        .take(*remaining as u64 + 1)
        .read_to_end(&mut bytes)?;
    *remaining = remaining.checked_sub(bytes.len()).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "aggregate successor input byte limit",
        )
    })?;
    Ok(serde_json::from_slice(&bytes)?)
}
pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let limits = SuccessorBundleLimits::default();
    if args.query_sets.len() > limits.max_query_sets {
        return Err(
            io::Error::new(io::ErrorKind::InvalidInput, "successor query-set limit").into(),
        );
    }
    let mut remaining = limits.max_input_bytes;
    let input = SuccessorBundleInput {
        schema_version: OWNED_SUCCESSOR_VERSION,
        prior: read(&args.input, &mut remaining)?,
        successor: read(&args.successor, &mut remaining)?,
        mapping: read(&args.mapping, &mut remaining)?,
        roles: read(&args.roles, &mut remaining)?,
        normalization: read(&args.normalization, &mut remaining)?,
        rewards: read(&args.rewards, &mut remaining)?,
        items: read(&args.items, &mut remaining)?,
        item_source: read(&args.item_source, &mut remaining)?,
        query_sets: args
            .query_sets
            .into_iter()
            .map(|q| {
                Ok(NamedQuerySet {
                    name: q.name,
                    queries: read(&q.path, &mut remaining)?,
                })
            })
            .collect::<Result<Vec<_>, Box<dyn Error>>>()?,
    };
    let staged = transition_owned_bundle(input, limits)?;
    super::owned_recipe_cli::publish_artifacts(&args.output, staged.artifacts())?;
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, staged.transition())?;
    stdout.write_all(b"\n")?;
    Ok(())
}
