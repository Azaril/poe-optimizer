//! Filesystem host for independently checked, immutable owned data releases.
use poe_optimizer_core::owned_definitions::OwnedDefinitionKey;
use poe_optimizer_import::{
    owned_recipe::{OWNED_RECIPE_VERSION, OwnedRecipeInput},
    owned_release::{
        OWNED_RELEASE_VERSION, OwnedReleaseInput, OwnedReleaseLimits, OwnedReleaseReceipt,
        StagedOwnedRelease, assemble_owned_release, decode_owned_release,
    },
    owned_release_revision::{OwnedReleaseRevisionInput, compile_owned_release_revision},
    owned_successor::{NamedQuerySet, SuccessorBindings, SuccessorBundleLimits},
};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
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
    #[arg(long)]
    revision: Option<PathBuf>,
    /// New destination directory; existing output is never replaced.
    #[arg(long)]
    output: PathBuf,
}

const REQUIRED: [&str; 11] = [
    "registry.json",
    "schema.json",
    "rules.json",
    "routing.json",
    "manifest.json",
    "mapping.json",
    "roles.json",
    "normalization.json",
    "rewards.json",
    "items.json",
    "item-source.json",
];
type Result<T> = std::result::Result<T, Box<dyn Error>>;
use super::owned_tree_cli::invalid;

fn query_name(name: &str) -> Option<&str> {
    let label = name.strip_prefix("queries-")?.strip_suffix(".json")?;
    (!label.is_empty()
        && label.len() <= 64
        && label
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_'))
    .then_some(label)
}

fn read(path: &Path, remaining: &mut usize, maximum: usize) -> Result<Vec<u8>> {
    if !fs::symlink_metadata(path)?.file_type().is_file() {
        return Err(invalid("release input must be a regular file"));
    }
    let mut bounded = (*remaining).min(maximum);
    let bytes = super::owned_tree_cli::read(path, &mut bounded)?;
    *remaining -= bytes.len();
    Ok(bytes)
}

fn decode<T: DeserializeOwned>(files: &BTreeMap<String, Vec<u8>>, name: &str) -> Result<T> {
    Ok(serde_json::from_slice(files.get(name).ok_or_else(
        || invalid(format!("missing release artifact: {name}")),
    )?)?)
}

pub(crate) fn load_release(
    root: &Path,
    remaining: &mut usize,
    limits: OwnedReleaseLimits,
) -> Result<StagedOwnedRelease> {
    let receipt_bytes = read(
        &root.join("release.json"),
        remaining,
        limits.max_artifact_bytes,
    )?;
    let receipt: OwnedReleaseReceipt = serde_json::from_slice(&receipt_bytes)?;
    let max_artifacts = REQUIRED.len() + limits.max_query_sets + 1;
    if receipt.schema_version != OWNED_RELEASE_VERSION
        || receipt.document_kind != "owned_data_release"
        || receipt.artifacts.len() > max_artifacts
        || receipt.query_sets > limits.max_query_sets
        || receipt.query_rows > limits.max_queries
        || receipt.provenance.len() > limits.max_provenance_entries
    {
        return Err(invalid("unsupported or oversized release receipt"));
    }
    // Validate every basename and the entire inventory before reading any listed
    // path. In particular, no relative path, symlink or extra directory is data.
    let mut names = BTreeSet::new();
    let mut total = 0usize;
    for row in &receipt.artifacts {
        if !(REQUIRED.contains(&row.file.as_str())
            || row.file == "tree-normalization.json"
            || query_name(&row.file).is_some())
            || !names.insert(row.file.as_str())
            || row.bytes > limits.max_artifact_bytes
            || row.sha256.len() != 64
            || !row
                .sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(invalid("invalid release artifact inventory"));
        }
        total = total
            .checked_add(row.bytes)
            .ok_or_else(|| invalid("release artifact byte overflow"))?;
    }
    if total > *remaining
        || REQUIRED.iter().any(|name| !names.contains(name))
        || names.contains("tree-normalization.json") != receipt.tree.is_some()
        || names
            .iter()
            .filter(|name| query_name(name).is_some())
            .count()
            != receipt.query_sets
    {
        return Err(invalid(
            "incomplete or oversized release artifact inventory",
        ));
    }
    let mut entries = 0usize;
    for entry in fs::read_dir(root)? {
        entries += 1;
        if entries > max_artifacts + 1 {
            return Err(invalid("release directory entry limit"));
        }
        let entry = entry?;
        let name = entry.file_name();
        if !entry.file_type()?.is_file()
            || !name
                .to_str()
                .is_some_and(|name| name == "release.json" || names.contains(name))
        {
            return Err(invalid(
                "release directory has an unlisted or non-file artifact",
            ));
        }
    }
    if entries != names.len() + 1 {
        return Err(invalid("release directory omitted an artifact"));
    }
    let mut files = BTreeMap::new();
    for row in &receipt.artifacts {
        let bytes = read(&root.join(&row.file), remaining, limits.max_artifact_bytes)?;
        if bytes.len() != row.bytes || format!("{:x}", Sha256::digest(&bytes)) != row.sha256 {
            return Err(invalid(format!(
                "release artifact hash/size differs: {}",
                row.file
            )));
        }
        files.insert(row.file.clone(), bytes);
    }
    // The receipt's ordered inventory retains authored query-set order. Query
    // rows likewise remain ordered; map iteration would lose this commitment.
    let query_sets = receipt
        .artifacts
        .iter()
        .filter_map(|row| {
            query_name(&row.file).map(|name| {
                Ok(NamedQuerySet {
                    name: OwnedDefinitionKey::new(name)?,
                    queries: decode(&files, &row.file)?,
                })
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let staged = assemble_owned_release(
        OwnedReleaseInput {
            schema_version: OWNED_RELEASE_VERSION,
            recipe: OwnedRecipeInput {
                schema_version: OWNED_RECIPE_VERSION,
                registry: decode(&files, "registry.json")?,
                schema: decode(&files, "schema.json")?,
                rules: decode(&files, "rules.json")?,
                routing: decode(&files, "routing.json")?,
            },
            mapping: decode(&files, "mapping.json")?,
            roles: decode(&files, "roles.json")?,
            normalization: decode(&files, "normalization.json")?,
            rewards: decode(&files, "rewards.json")?,
            items: decode(&files, "items.json")?,
            item_source: decode(&files, "item-source.json")?,
            tree: receipt
                .tree
                .map(|_| decode(&files, "tree-normalization.json"))
                .transpose()?,
            query_sets,
            provenance: receipt.provenance,
        },
        limits,
    )?;
    files.insert("release.json".into(), receipt_bytes);
    if staged.artifacts().count() != files.len() {
        return Err(invalid("release assembly artifact count differs"));
    }
    for (name, bytes) in staged.artifacts() {
        if files.get(name).map(Vec::as_slice) != Some(bytes) {
            return Err(invalid(format!(
                "release assembly artifact differs: {name}"
            )));
        }
    }
    Ok(staged)
}

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
            Ok(_) => load_release(&args.input, &mut remaining, limits)?,
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
    super::owned_recipe_cli::publish_artifacts(&args.output, staged.artifacts())?;
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, staged.receipt())?;
    stdout.write_all(b"\n")?;
    Ok(())
}
