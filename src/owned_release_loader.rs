//! Checked filesystem consumption of immutable owned release publications.
use super::owned_io::{invalid, read_release_input as read};
use poe_optimizer_import::owned_release::{
    CheckedOwnedRelease, OwnedReleaseLimits, OwnedReleaseReceipt, load_owned_release_artifacts,
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fs,
    path::Path,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub(crate) fn load_release(
    root: &Path,
    remaining: &mut usize,
    limits: OwnedReleaseLimits,
) -> Result<CheckedOwnedRelease> {
    let receipt_bytes = read(
        &root.join("release.json"),
        remaining,
        limits.max_artifact_bytes,
    )?;
    let receipt: OwnedReleaseReceipt = serde_json::from_slice(&receipt_bytes)?;
    // Shared library validation establishes safe basenames and the complete
    // closed inventory before any declared path is opened by this host.
    let total = receipt.validate_artifact_inventory(limits)?;
    if total > *remaining {
        return Err(invalid(
            "incomplete or oversized release artifact inventory",
        ));
    }
    let names: BTreeSet<_> = receipt
        .artifacts
        .iter()
        .map(|row| row.file.as_str())
        .collect();
    let max_artifacts = receipt.artifacts.len();
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
    files.insert("release.json".into(), receipt_bytes);
    Ok(load_owned_release_artifacts(&files, limits)?)
}
