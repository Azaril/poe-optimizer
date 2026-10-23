//! Filesystem boundary for offline final-release construction.
use clap::{Args, Subcommand};
use poe_optimizer_authoring::{
    AuthoringCheckpointReceipt, CheckedAuthoringCheckpoint, CheckpointBase, CheckpointLimits,
    ReleaseSource, build_authoring_checkpoint, load_authoring_checkpoint_artifacts,
    parse_release_manifest,
};
use poe_optimizer_core::owned_content::OwnedContentDigest;
use poe_optimizer_host::{ArtifactBundleLimits, ArtifactDirectory, ArtifactFile};
use poe_optimizer_import::owned_release::OwnedReleaseReceipt;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

#[derive(Subcommand)]
pub(super) enum Command {
    /// Compile and atomically publish a name ledger and runtime release together
    Build(BuildCommand),
}

#[derive(Args)]
pub(super) struct BuildCommand {
    /// Release manifest to build
    #[arg(value_name = "RELEASE.KDL")]
    manifest: PathBuf,
    /// New output directory; an existing directory is never overwritten
    #[arg(long, value_name = "NEW_DIRECTORY")]
    output: PathBuf,
    /// Explicitly start allocation history from the checked source registry
    #[arg(
        long,
        conflicts_with_all = ["predecessor", "expected_predecessor"],
        required_unless_present = "predecessor"
    )]
    bootstrap: bool,
    /// Immutable authoring checkpoint supplying allocation history
    #[arg(
        long,
        value_name = "CHECKPOINT_DIRECTORY",
        requires = "expected_predecessor"
    )]
    predecessor: Option<PathBuf>,
    /// Exact identity of the chosen checkpoint, not a mutable-head compare-and-swap
    #[arg(long, value_name = "DIGEST", requires = "predecessor")]
    expected_predecessor: Option<OwnedContentDigest>,
}

fn read(path: &Path, maximum: usize) -> Result<Vec<u8>, String> {
    if !fs::symlink_metadata(path)
        .map_err(|e| format!("cannot inspect '{}': {e}", path.display()))?
        .file_type()
        .is_file()
    {
        return Err(format!(
            "source is not a regular file: '{}'",
            path.display()
        ));
    }
    let mut bytes = Vec::new();
    File::open(path)
        .and_then(|file| file.take(maximum as u64 + 1).read_to_end(&mut bytes))
        .map_err(|e| format!("cannot read '{}': {e}", path.display()))?;
    if bytes.len() > maximum {
        return Err(format!(
            "source byte budget exceeded at '{}'",
            path.display()
        ));
    }
    Ok(bytes)
}

pub(super) fn run(command: Command) -> Result<ExitCode, String> {
    let Command::Build(command) = command;
    let checkpoint_limits = CheckpointLimits::default();
    let limits = checkpoint_limits.build;
    let predecessor = command
        .predecessor
        .as_ref()
        .map(|path| {
            load_checkpoint(
                path,
                command
                    .expected_predecessor
                    .expect("clap requires predecessor identity"),
                checkpoint_limits,
            )
        })
        .transpose()?;
    let manifest_path = command
        .manifest
        .canonicalize()
        .map_err(|e| format!("cannot resolve manifest: {e}"))?;
    let root = manifest_path
        .parent()
        .ok_or("manifest has no parent directory")?;
    let manifest_bytes = read(&manifest_path, limits.manifest.max_source_bytes)?;
    let text = std::str::from_utf8(&manifest_bytes).map_err(|_| "manifest is not UTF-8")?;
    let manifest = parse_release_manifest(text, limits).map_err(|e| e.to_string())?;
    let mut left = limits.max_encoded_bytes;
    let mut buffers = Vec::new();
    for source in manifest.sources() {
        let path = root
            .join(source.path())
            .canonicalize()
            .map_err(|e| format!("cannot resolve source '{}': {e}", source.path()))?;
        if !path.starts_with(root) {
            return Err("source resolves outside the manifest directory".into());
        }
        let bytes = read(&path, left)?;
        left -= bytes.len();
        buffers.push(bytes);
    }
    let sources: Vec<_> = manifest
        .sources()
        .iter()
        .zip(&buffers)
        .map(|(spec, bytes)| ReleaseSource {
            path: spec.path(),
            bytes,
        })
        .collect();
    let base = match (predecessor.as_ref(), command.bootstrap) {
        (None, true) => CheckpointBase::Bootstrap,
        (Some(prior), false) => CheckpointBase::Predecessor(prior),
        _ => return Err("select exactly one allocation bootstrap or predecessor".into()),
    };
    let checkpoint = build_authoring_checkpoint(&manifest_bytes, &sources, base, checkpoint_limits)
        .map_err(|e| e.to_string())?;
    let built = checkpoint.built();
    if built.decoded_bytes() * 5 >= limits.max_decoded_bytes * 4 {
        eprintln!(
            "warning: decoded release sources use {} of {} bytes; compact-format migration is required before expanding content",
            built.decoded_bytes(),
            limits.max_decoded_bytes
        );
    }
    let root_files: Vec<_> = checkpoint
        .root_artifacts()
        .map(|(name, bytes)| ArtifactFile { name, bytes })
        .collect();
    let runtime: Vec<_> = built
        .release()
        .artifacts()
        .map(|(name, bytes)| ArtifactFile { name, bytes })
        .collect();
    poe_optimizer_host::publish_artifact_bundle(
        &command.output,
        &root_files,
        &[ArtifactDirectory {
            name: "runtime",
            files: &runtime,
        }],
        ArtifactBundleLimits {
            max_entries: checkpoint_limits.max_entries,
            max_bytes: checkpoint_limits.max_checkpoint_bytes,
        },
    )
    .map_err(|e| format!("cannot publish authoring checkpoint: {e}"))?;
    eprintln!("checkpoint identity: {}", checkpoint.identity());
    let receipt = checkpoint
        .root_artifacts()
        .find(|(name, _)| *name == "checkpoint.json")
        .expect("checkpoint construction emits its receipt")
        .1;
    let mut stdout = io::stdout().lock();
    stdout
        .write_all(receipt)
        .and_then(|()| stdout.write_all(b"\n"))
        .map_err(|e| format!("cannot write checkpoint receipt: {e}"))?;
    Ok(ExitCode::SUCCESS)
}

/// Enumerate only immediate children and reject symlinks/special files. No source
/// or receipt supplied path can select an arbitrary nested filesystem target.
fn inventory(path: &Path, maximum: usize) -> Result<BTreeMap<String, bool>, String> {
    if !fs::symlink_metadata(path)
        .map_err(|e| format!("cannot inspect '{}': {e}", path.display()))?
        .file_type()
        .is_dir()
    {
        return Err(format!(
            "checkpoint path is not a regular directory: '{}'",
            path.display()
        ));
    }
    let mut files = BTreeMap::new();
    for entry in fs::read_dir(path).map_err(|e| format!("cannot list '{}': {e}", path.display()))? {
        if files.len() == maximum {
            return Err("checkpoint entry budget exceeded".into());
        }
        let entry = entry.map_err(|e| format!("cannot list checkpoint entry: {e}"))?;
        let kind = entry
            .file_type()
            .map_err(|e| format!("cannot inspect checkpoint entry: {e}"))?;
        if !kind.is_file() && !kind.is_dir() {
            return Err("checkpoint entries must be regular files/directories".into());
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "checkpoint entry is not UTF-8")?;
        if files.insert(name, kind.is_dir()).is_some() {
            return Err("duplicate checkpoint entry".into());
        }
    }
    Ok(files)
}

fn load_checkpoint(
    path: &Path,
    expected: OwnedContentDigest,
    limits: CheckpointLimits,
) -> Result<CheckedAuthoringCheckpoint, String> {
    limits.validate().map_err(|e| e.to_string())?;
    let actual = inventory(path, 3)?;
    let expected_root = BTreeMap::from([
        ("checkpoint.json".to_owned(), false),
        ("names.json".to_owned(), false),
        ("runtime".to_owned(), true),
    ]);
    if actual != expected_root {
        return Err("checkpoint root inventory differs".into());
    }
    let checkpoint = read(&path.join("checkpoint.json"), limits.max_receipt_bytes)?;
    let receipt: AuthoringCheckpointReceipt = serde_json::from_slice(&checkpoint)
        .map_err(|e| format!("invalid checkpoint receipt: {e}"))?;
    if receipt.identity(limits).map_err(|e| e.to_string())? != expected {
        return Err("expected predecessor checkpoint identity differs".into());
    }
    let mut left = limits
        .max_checkpoint_bytes
        .checked_sub(checkpoint.len())
        .ok_or("checkpoint byte budget exceeded")?;
    let names = read(&path.join("names.json"), limits.max_names_bytes.min(left))?;
    left -= names.len();
    let runtime_path = path.join("runtime");
    let actual = inventory(&runtime_path, limits.max_entries.saturating_sub(3))?;
    if actual.values().any(|is_dir| *is_dir) {
        return Err("runtime inventory contains a directory".into());
    }
    let runtime_receipt = read(
        &runtime_path.join("release.json"),
        limits.build.release.max_artifact_bytes.min(left),
    )?;
    left -= runtime_receipt.len();
    let runtime_manifest: OwnedReleaseReceipt = serde_json::from_slice(&runtime_receipt)
        .map_err(|e| format!("invalid runtime receipt: {e}"))?;
    runtime_manifest
        .validate_artifact_inventory(limits.build.release)
        .map_err(|e| e.to_string())?;
    let allowed: BTreeSet<_> = runtime_manifest
        .artifacts
        .iter()
        .map(|r| r.file.as_str())
        .chain(["release.json"])
        .collect();
    if actual.keys().map(String::as_str).collect::<BTreeSet<_>>() != allowed {
        return Err("runtime artifact inventory differs".into());
    }
    let mut runtime = BTreeMap::from([("release.json".to_owned(), runtime_receipt)]);
    for artifact in &runtime_manifest.artifacts {
        let bytes = read(&runtime_path.join(&artifact.file), artifact.bytes.min(left))?;
        left -= bytes.len();
        runtime.insert(artifact.file.clone(), bytes);
    }
    load_authoring_checkpoint_artifacts(&checkpoint, &names, &runtime, expected, limits)
        .map_err(|e| e.to_string())
}
