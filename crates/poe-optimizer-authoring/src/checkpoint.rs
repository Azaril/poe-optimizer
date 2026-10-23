//! Immutable authoring history around the existing closed runtime release.
//!
//! Source commitments record the exact encoded inputs, including comments and
//! transport bytes. They attest reproducibility, not game correctness. Loading
//! rechecks the complete runtime; a digest alone never grants allocation authority.
use crate::{
    AllocationError, AllocationLimits, BuiltRelease, NamedIdRegistry, NamesInput,
    ReleaseAllocationBase, ReleaseBuildError, ReleaseBuildLimits, ReleaseSource, build_release,
    parse_release_manifest,
};
use poe_optimizer_core::owned_content::{ContentDigestError, OwnedContentDigest, digest_owned};
use poe_optimizer_import::{
    owned_mapping::OwnedIdRegistry,
    owned_release::{OwnedReleaseError, load_owned_release_artifacts},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::{self, Write},
};

#[derive(Clone, Copy, Debug)]
pub struct CheckpointLimits {
    pub build: ReleaseBuildLimits,
    pub max_receipt_bytes: usize,
    pub max_names_bytes: usize,
    pub max_checkpoint_bytes: usize,
    /// Root files, the runtime directory, and its files all count.
    pub max_entries: usize,
}
impl Default for CheckpointLimits {
    fn default() -> Self {
        Self {
            build: ReleaseBuildLimits::default(),
            max_receipt_bytes: 8 * 1024 * 1024,
            max_names_bytes: 16 * 1024 * 1024,
            max_checkpoint_bytes: 88 * 1024 * 1024,
            max_entries: 128,
        }
    }
}
impl CheckpointLimits {
    pub fn validate(self) -> Result<()> {
        self.build.validate()?;
        let hard = Self::default();
        for (value, maximum) in [
            (self.max_receipt_bytes, hard.max_receipt_bytes),
            (self.max_names_bytes, hard.max_names_bytes),
            (self.max_checkpoint_bytes, hard.max_checkpoint_bytes),
            (self.max_entries, hard.max_entries),
        ] {
            if value == 0 || value > maximum {
                return Err(CheckpointError::Invalid("checkpoint limits"));
            }
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CheckpointError {
    #[error("invalid authoring checkpoint: {0}")]
    Invalid(&'static str),
    #[error("authoring checkpoint exceeds {0}")]
    Limit(&'static str),
    #[error(transparent)]
    Build(#[from] ReleaseBuildError),
    #[error(transparent)]
    Runtime(#[from] OwnedReleaseError),
    #[error(transparent)]
    Allocation(#[from] AllocationError),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
type Result<T> = std::result::Result<T, CheckpointError>;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckpointBlob {
    pub bytes: usize,
    pub sha256: String,
}
impl CheckpointBlob {
    fn of(bytes: &[u8]) -> Self {
        Self {
            bytes: bytes.len(),
            sha256: format!("{:x}", Sha256::digest(bytes)),
        }
    }
    fn validate(&self, maximum: usize) -> Result<()> {
        if self.bytes > maximum {
            return Err(CheckpointError::Limit("blob bytes"));
        }
        if self.sha256.len() != 64
            || !self
                .sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(CheckpointError::Invalid("blob hash"));
        }
        Ok(())
    }
    fn check(&self, bytes: &[u8]) -> Result<()> {
        if self != &Self::of(bytes) {
            return Err(CheckpointError::Invalid("blob hash/size differs"));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckpointPredecessor {
    pub checkpoint: OwnedContentDigest,
    pub names: OwnedContentDigest,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckpointNames {
    pub identity: OwnedContentDigest,
    pub blob: CheckpointBlob,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckpointRuntime {
    pub receipt: CheckpointBlob,
    pub input: OwnedContentDigest,
    pub registry: OwnedContentDigest,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckpointSourceFile {
    pub path: String,
    pub blob: CheckpointBlob,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckpointSource {
    pub manifest: CheckpointBlob,
    /// Sorted exact encoded file inventory; paths are labels, never load targets.
    pub files: Vec<CheckpointSourceFile>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoringCheckpointReceipt {
    pub version: u32,
    pub predecessor: Option<CheckpointPredecessor>,
    pub names: CheckpointNames,
    pub runtime: CheckpointRuntime,
    pub source: CheckpointSource,
}
impl AuthoringCheckpointReceipt {
    /// Identity of a claim. Only the checked artifact loader grants authority.
    pub fn identity(&self, limits: CheckpointLimits) -> Result<OwnedContentDigest> {
        limits.validate()?;
        if self.version != 1 || self.source.files.len() > limits.build.max_sources {
            return Err(CheckpointError::Invalid("version or source count"));
        }
        self.names.blob.validate(limits.max_names_bytes)?;
        self.runtime
            .receipt
            .validate(limits.build.release.max_artifact_bytes)?;
        self.source
            .manifest
            .validate(limits.build.manifest.max_source_bytes)?;
        let mut left = limits.build.max_encoded_bytes;
        let mut previous: Option<&str> = None;
        for row in &self.source.files {
            if !super::release::valid_path(&row.path)
                || previous.is_some_and(|p| p >= row.path.as_str())
            {
                return Err(CheckpointError::Invalid(
                    "source paths must be unique and sorted",
                ));
            }
            previous = Some(&row.path);
            row.blob.validate(left)?;
            left -= row.blob.bytes;
        }
        Ok(digest_owned(
            "owned-authoring-checkpoint-v1",
            self,
            limits.max_receipt_bytes,
        )?)
    }
}

/// Explicit immutable predecessor selection, never inferred from a missing file.
pub enum CheckpointBase<'a> {
    Bootstrap,
    Predecessor(&'a CheckedAuthoringCheckpoint),
}
pub struct CheckedAuthoringCheckpoint {
    receipt: AuthoringCheckpointReceipt,
    identity: OwnedContentDigest,
    names: NamedIdRegistry,
}
impl CheckedAuthoringCheckpoint {
    pub fn receipt(&self) -> &AuthoringCheckpointReceipt {
        &self.receipt
    }
    pub fn identity(&self) -> OwnedContentDigest {
        self.identity
    }
    pub fn names(&self) -> &NamedIdRegistry {
        &self.names
    }
}
pub struct StagedAuthoringCheckpoint {
    built: BuiltRelease,
    receipt: AuthoringCheckpointReceipt,
    identity: OwnedContentDigest,
    receipt_bytes: Vec<u8>,
    names_bytes: Vec<u8>,
}
impl StagedAuthoringCheckpoint {
    pub fn built(&self) -> &BuiltRelease {
        &self.built
    }
    pub fn receipt(&self) -> &AuthoringCheckpointReceipt {
        &self.receipt
    }
    pub fn identity(&self) -> OwnedContentDigest {
        self.identity
    }
    pub fn root_artifacts(&self) -> impl Iterator<Item = (&str, &[u8])> {
        [
            ("checkpoint.json", self.receipt_bytes.as_slice()),
            ("names.json", self.names_bytes.as_slice()),
        ]
        .into_iter()
    }
}

/// Compile the exact supplied source snapshot and stage both ledger and runtime.
/// No output is exposed if any compiler, name, final-release or byte-bound check fails.
pub fn build_authoring_checkpoint(
    manifest_bytes: &[u8],
    sources: &[ReleaseSource<'_>],
    base: CheckpointBase<'_>,
    limits: CheckpointLimits,
) -> Result<StagedAuthoringCheckpoint> {
    limits.validate()?;
    if manifest_bytes.len() > limits.build.manifest.max_source_bytes {
        return Err(CheckpointError::Limit("manifest bytes"));
    }
    let manifest_text = std::str::from_utf8(manifest_bytes)
        .map_err(|_| CheckpointError::Invalid("manifest UTF-8"))?;
    let manifest = parse_release_manifest(manifest_text, limits.build)?;
    let (allocation, predecessor) = match base {
        CheckpointBase::Bootstrap => (ReleaseAllocationBase::Bootstrap, None),
        CheckpointBase::Predecessor(prior) => (
            ReleaseAllocationBase::Predecessor {
                names: &prior.names,
                expected_names: prior.receipt.names.identity,
            },
            Some(CheckpointPredecessor {
                checkpoint: prior.identity,
                names: prior.receipt.names.identity,
            }),
        ),
    };
    let built = build_release(&manifest, sources, allocation, limits.build)?;
    let names = built.name_allocation().names();
    // The runtime registry is the sole registry snapshot in this checkpoint.
    if names.input().registry != built.release().receipt().registry
        || built.name_allocation().predecessor() != predecessor.as_ref().map(|p| p.names)
    {
        return Err(CheckpointError::Invalid(
            "allocation/runtime or predecessor binding",
        ));
    }
    let names_bytes = encode(names.input(), limits.max_names_bytes)?;
    let runtime_receipt = built
        .release()
        .artifacts()
        .find(|(name, _)| *name == "release.json")
        .ok_or(CheckpointError::Invalid("missing runtime receipt"))?
        .1;
    let mut files: Vec<_> = sources
        .iter()
        .map(|s| CheckpointSourceFile {
            path: s.path.to_owned(),
            blob: CheckpointBlob::of(s.bytes),
        })
        .collect();
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let receipt = AuthoringCheckpointReceipt {
        version: 1,
        predecessor,
        names: CheckpointNames {
            identity: names.identity()?,
            blob: CheckpointBlob::of(&names_bytes),
        },
        runtime: CheckpointRuntime {
            receipt: CheckpointBlob::of(runtime_receipt),
            input: built.release().receipt().input,
            registry: built.release().receipt().registry,
        },
        source: CheckpointSource {
            manifest: CheckpointBlob::of(manifest_bytes),
            files,
        },
    };
    let identity = receipt.identity(limits)?;
    let receipt_bytes = encode(&receipt, limits.max_receipt_bytes)?;
    bound_inventory(
        &receipt_bytes,
        &names_bytes,
        built.release().artifacts().map(|(_, b)| b),
        limits,
    )?;
    Ok(StagedAuthoringCheckpoint {
        built,
        receipt,
        identity,
        receipt_bytes,
        names_bytes,
    })
}

/// Host must reject symlinks and extras and perform bounded reads first. This
/// independently checks all bytes, exact runtime inventory and semantic bindings.
/// The expected identity pins the chosen immutable checkpoint, not a mutable head.
pub fn load_authoring_checkpoint_artifacts(
    checkpoint: &[u8],
    names: &[u8],
    runtime: &BTreeMap<String, Vec<u8>>,
    expected: OwnedContentDigest,
    limits: CheckpointLimits,
) -> Result<CheckedAuthoringCheckpoint> {
    limits.validate()?;
    bound_inventory(
        checkpoint,
        names,
        runtime.values().map(Vec::as_slice),
        limits,
    )?;
    let receipt: AuthoringCheckpointReceipt = serde_json::from_slice(checkpoint)?;
    let identity = receipt.identity(limits)?;
    if identity != expected || encode(&receipt, limits.max_receipt_bytes)? != checkpoint {
        return Err(CheckpointError::Invalid(
            "expected checkpoint identity or canonical receipt",
        ));
    }
    receipt.names.blob.check(names)?;
    receipt.runtime.receipt.check(
        runtime
            .get("release.json")
            .ok_or(CheckpointError::Invalid("missing runtime receipt"))?,
    )?;
    let checked = load_owned_release_artifacts(runtime, limits.build.release)?;
    if checked.receipt().input != receipt.runtime.input
        || checked.receipt().registry != receipt.runtime.registry
    {
        return Err(CheckpointError::Invalid("runtime receipt binding"));
    }
    let input: NamesInput = serde_json::from_slice(names)?;
    checked
        .recipe()
        .registry()
        .validate_limits(limits.build.declarations.registry)
        .map_err(AllocationError::from)?;
    let registry = OwnedIdRegistry::from_snapshot(checked.recipe().registry().clone());
    let names_checked = NamedIdRegistry::new(
        registry,
        input,
        AllocationLimits {
            symbols: limits.build.declarations.symbols,
            max_requests: limits.build.declarations.max_declarations,
            max_snapshot_bytes: limits.build.declarations.max_names_bytes,
        },
    )?;
    if names_checked.identity()? != receipt.names.identity
        || encode(names_checked.input(), limits.max_names_bytes)? != names
    {
        return Err(CheckpointError::Invalid(
            "name identity or canonical ledger",
        ));
    }
    Ok(CheckedAuthoringCheckpoint {
        receipt,
        identity,
        names: names_checked,
    })
}

fn bound_inventory<'a>(
    checkpoint: &[u8],
    names: &[u8],
    runtime: impl Iterator<Item = &'a [u8]>,
    limits: CheckpointLimits,
) -> Result<()> {
    if checkpoint.len() > limits.max_receipt_bytes || names.len() > limits.max_names_bytes {
        return Err(CheckpointError::Limit("root artifact bytes"));
    }
    let mut entries = limits
        .max_entries
        .checked_sub(3)
        .ok_or(CheckpointError::Limit("entries"))?;
    let mut left = limits.max_checkpoint_bytes;
    // Runtime loading charges the complete emitted inventory, which can exceed
    // the serialized authoring input by its manifests and receipts.
    let mut runtime_left = limits
        .build
        .release
        .max_input_bytes
        .min(limits.build.release.max_output_bytes);
    for size in [checkpoint.len(), names.len()] {
        left = left
            .checked_sub(size)
            .ok_or(CheckpointError::Limit("total bytes"))?;
    }
    for bytes in runtime {
        if bytes.len() > limits.build.release.max_artifact_bytes {
            return Err(CheckpointError::Limit("runtime artifact bytes"));
        }
        runtime_left = runtime_left
            .checked_sub(bytes.len())
            .ok_or(CheckpointError::Limit("runtime bytes"))?;
        entries = entries
            .checked_sub(1)
            .ok_or(CheckpointError::Limit("entries"))?;
        left = left
            .checked_sub(bytes.len())
            .ok_or(CheckpointError::Limit("total bytes"))?;
    }
    Ok(())
}

fn encode(value: &impl Serialize, maximum: usize) -> Result<Vec<u8>> {
    struct Bounded {
        bytes: Vec<u8>,
        maximum: usize,
        exceeded: bool,
    }
    impl Write for Bounded {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if bytes.len() > self.maximum - self.bytes.len() {
                self.exceeded = true;
                return Err(io::Error::other("checkpoint encoding limit"));
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut writer = Bounded {
        bytes: vec![],
        maximum,
        exceeded: false,
    };
    let result = serde_json::to_writer(&mut writer, value);
    if writer.exceeded {
        return Err(CheckpointError::Limit("encoded bytes"));
    }
    result?;
    Ok(writer.bytes)
}
