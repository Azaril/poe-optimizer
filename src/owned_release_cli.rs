//! Offline release assembly, explicit migrations and atomic publication.
use super::{
    owned_io::{invalid, read_release_input as read},
    owned_release_loader::load_release,
};
use poe_optimizer_import::{
    owned_release::{
        OWNED_RELEASE_VERSION, OwnedReleaseInput, OwnedReleaseLimits, StagedOwnedRelease,
        assemble_owned_release, decode_owned_release, stage_checked_owned_release,
    },
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
    owned_release_revision::{OwnedReleaseRevisionInput, compile_owned_release_revision},
    owned_successor::{SuccessorBindings, SuccessorBundleLimits},
};
use std::{
    error::Error,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

#[derive(clap::Args)]
pub(crate) struct Args {
    /// Exact-bound release JSON, checked successor directory, or release directory.
    input: PathBuf,
    /// Explicit schema correction, bound to the complete checked input release.
    #[arg(long, conflicts_with = "migration")]
    revision: Option<PathBuf>,
    /// Explicit contract migration, bound to the complete checked input release.
    #[arg(long, conflicts_with = "revision")]
    migration: Option<PathBuf>,
    /// New destination directory; existing output is never replaced.
    #[arg(long)]
    output: PathBuf,
}

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn load_successor(
    root: &Path,
    remaining: &mut usize,
    limits: OwnedReleaseLimits,
) -> Result<StagedOwnedRelease> {
    let prior = super::owned_tree_cli::load_checked_bundle(
        root,
        remaining,
        SuccessorBundleLimits::default(),
    )?;
    let expected = prior.bindings().clone();
    let input = prior.input;
    let staged = assemble_owned_release(
        OwnedReleaseInput {
            schema_version: OWNED_RELEASE_VERSION,
            recipe: input.prior,
            mapping: input.mapping,
            roles: input.roles,
            normalization: input.normalization,
            rewards: input.rewards,
            items: input.items,
            item_source: input.item_source,
            tree: prior.tree,
            evaluation: None,
            query_sets: input.query_sets,
            provenance: vec![],
        },
        limits,
    )?;
    let receipt = staged.receipt();
    let actual = SuccessorBindings {
        registry: receipt.registry,
        definitions: receipt.definitions.clone(),
        mapping: receipt.mapping,
        roles: receipt.roles,
        normalization: receipt.normalization,
        rewards: receipt.rewards,
        rules: receipt.rules,
        routing: receipt.routing,
    };
    if actual != expected {
        return Err(invalid("prior manifest endpoint identities differ"));
    }
    Ok(staged)
}

pub(crate) fn run(args: Args) -> Result<()> {
    let limits = OwnedReleaseLimits::default();
    let mut remaining = limits.max_input_bytes;
    let mut staged = if args.input.is_dir() {
        // Presence selects the release parser even if its receipt is malformed.
        // Never fall back to legacy parsing after a release validation failure.
        match fs::symlink_metadata(args.input.join("release.json")) {
            Ok(_) => {
                stage_checked_owned_release(load_release(&args.input, &mut remaining, limits)?)?
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                load_successor(&args.input, &mut remaining, limits)?
            }
            Err(e) => return Err(e.into()),
        }
    } else {
        decode_owned_release(
            &read(&args.input, &mut remaining, limits.max_input_bytes)?,
            limits,
        )?
    };
    if let Some(revision) = args.revision {
        let policy: OwnedReleaseRevisionInput =
            serde_json::from_slice(&read(&revision, &mut remaining, limits.max_artifact_bytes)?)?;
        staged = compile_owned_release_revision(&staged, policy, limits)?;
    }
    if let Some(migration) = args.migration {
        let policy: OwnedReleaseMigrationInput = serde_json::from_slice(&read(
            &migration,
            &mut remaining,
            limits.max_artifact_bytes,
        )?)?;
        staged = compile_owned_release_migration(&staged, policy, limits)?;
    }
    super::owned_recipe_cli::publish_artifacts(&args.output, staged.artifacts())?;
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, staged.receipt())?;
    stdout.write_all(b"\n")?;
    Ok(())
}
