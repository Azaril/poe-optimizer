//! One-off receipt authoring only. The caller first compiles/reproduces both
//! candidate bundles; this helper verifies the bounded inverse and writes two
//! fresh dependent receipt candidates. It never installs maintained artifacts.
use crate::owned_skill_catalog::{OwnedReversePrimaryCensus, OwnedSkillRolePackageInput};
use poe_optimizer_core::owned_content::digest_owned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

const DATA: &str = "data/owned/poe2/3887ae68";
const MAX_BYTES: usize = 8 * 1024 * 1024;
type ReadSet = Vec<(PathBuf, Vec<u8>)>;

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read(path: &Path, reads: &mut ReadSet) -> Vec<u8> {
    assert!(fs::metadata(path).unwrap().len() <= MAX_BYTES as u64);
    let bytes = fs::read(path).unwrap();
    assert!(bytes.len() <= MAX_BYTES);
    assert!(reads.iter().all(|(old, _)| old != path));
    reads.push((path.to_path_buf(), bytes.clone()));
    bytes
}
fn pinned(path: &Path, expected: &str, reads: &mut ReadSet) -> Vec<u8> {
    let bytes = read(path, reads);
    assert_eq!(hash(&bytes), expected, "{}", path.display());
    bytes
}
fn unchanged(reads: &ReadSet) {
    for (path, bytes) in reads {
        assert_eq!(fs::read(path).unwrap(), *bytes, "{}", path.display());
    }
}
fn artifact_pin(value: &Value) -> &Value {
    let rows = value["artifacts"].as_array().unwrap();
    let matches: Vec<_> = rows
        .iter()
        .filter(|row| row["file"] == "roles.json")
        .collect();
    assert_eq!(matches.len(), 1);
    matches[0]
}

fn checked_transition(
    root: &Path,
    candidate: &Path,
    family: &str,
    old_roles_hash: &str,
    old_transition_hash: &str,
    reads: &mut ReadSet,
) -> Value {
    let old_dir = root.join(DATA).join(family);
    assert_ne!(
        old_dir.canonicalize().unwrap(),
        candidate.canonicalize().unwrap()
    );
    let old_roles = pinned(&old_dir.join("roles.json"), old_roles_hash, reads);
    let old_transition = pinned(&old_dir.join("transition.json"), old_transition_hash, reads);
    let new_roles = read(&candidate.join("roles.json"), reads);
    let new_transition = read(&candidate.join("transition.json"), reads);
    let old_role_value: Value = serde_json::from_slice(&old_roles).unwrap();
    let mut role_inverse: Value = serde_json::from_slice(&new_roles).unwrap();
    assert!(old_role_value.get("reverse_primary").is_none());
    let census = role_inverse
        .as_object_mut()
        .unwrap()
        .remove("reverse_primary")
        .unwrap();
    assert_eq!(census["kind"], "observed");
    assert_eq!(census["routes"].as_array().unwrap().len(), 1436);
    assert_eq!(
        role_inverse, old_role_value,
        "only required census may be added"
    );
    let typed: OwnedSkillRolePackageInput = serde_json::from_slice(&new_roles).unwrap();
    assert_eq!(typed.roles.len(), 966);
    assert!(matches!(
        typed.reverse_primary,
        OwnedReversePrimaryCensus::Observed { .. }
    ));
    let identity = digest_owned("owned-skill-role-package-v1", &typed, MAX_BYTES).unwrap();

    let old: Value = serde_json::from_slice(&old_transition).unwrap();
    let new: Value = serde_json::from_slice(&new_transition).unwrap();
    let mut inverse = new.clone();
    let identity_path = if family == "import/compiled" {
        assert_eq!(new["document_kind"], "owned_catalog_recipe_transition");
        "/roles"
    } else {
        assert_eq!(family, "current");
        assert_eq!(new["document_kind"], "owned_successor_bundle");
        for path in ["/before/roles", "/input"] {
            let next = new.pointer(path).unwrap().as_str().unwrap();
            assert_eq!(next.len(), 64);
            assert!(next.bytes().all(|b| b.is_ascii_hexdigit()));
            assert_ne!(new.pointer(path), old.pointer(path));
            *inverse.pointer_mut(path).unwrap() = old.pointer(path).unwrap().clone();
        }
        "/after/roles"
    };
    assert_eq!(*new.pointer(identity_path).unwrap(), json!(identity));
    assert_ne!(new.pointer(identity_path), old.pointer(identity_path));
    *inverse.pointer_mut(identity_path).unwrap() = old.pointer(identity_path).unwrap().clone();
    for (transition, roles) in [(&old, &old_roles), (&new, &new_roles)] {
        let pin = artifact_pin(transition);
        assert_eq!(pin["bytes"], json!(roles.len()));
        assert_eq!(pin["sha256"], hash(roles));
    }
    *inverse["artifacts"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["file"] == "roles.json")
        .unwrap() = artifact_pin(&old).clone();
    assert_eq!(inverse, old, "exact whole transition inverse");
    assert_ne!(hash(&new_transition), old_transition_hash);
    json!({"family":family,"before_sha256":old_transition_hash,
        "after_sha256":hash(&new_transition),"before_bytes":old_transition.len(),
        "after_bytes":new_transition.len(),"roles_sha256":hash(&new_roles),
        "roles_identity":identity,"whole_role_inverse":true,"whole_transition_inverse":true})
}

/// `output` must be a fresh child of an existing directory under resolved runs/.
/// Candidate paths identify the already reproduced compiled/current bundles.
/// Returns the same compact report it writes; all installation is caller-owned.
pub fn rebind_receipts(
    root: &Path,
    output: &Path,
    compiled_candidate: &Path,
    current_candidate: &Path,
) -> Value {
    let root = root.canonicalize().unwrap();
    let parent = output.parent().unwrap().canonicalize().unwrap();
    assert!(parent.starts_with(root.join("runs").canonicalize().unwrap()));
    let output = parent.join(output.file_name().unwrap());
    assert!(!output.exists(), "fresh receipt output only");
    let mut reads = Vec::new();
    let source = read(
        &root.join("runs/owned_reverse_primary_receipt_rebind.rs"),
        &mut reads,
    );
    let compiled = checked_transition(
        &root,
        compiled_candidate,
        "import/compiled",
        "7a2191cdc79594cf25955a1204aadd7287766fafa323ac48c662cd553f6e0d1f",
        "5ca97989545ed0ef5441d580dca09d9f6bf9dbe39531ee9f59f2b14b780093a0",
        &mut reads,
    );
    let current = checked_transition(
        &root,
        current_candidate,
        "current",
        "611713865bbf4ab237dd132ee79c232c6e81eb76c35dbf2e998233121874e04b",
        "a84e9c57648d6d8108d3291120d0b84e649632d4ae597e63a14aaae818233a7e",
        &mut reads,
    );
    let specs = [
        (
            "import/policies/transition.json",
            "/compiled_transition_sha256",
            "c0e958785176c256794547ce400e4980f5e24b9ef178bf43e981b8ff7c87c4db",
            &compiled,
        ),
        (
            "allocations/source-facts.json",
            "/bundle_transition_sha256",
            "e92aecb56d7674a11f15c3e6df28ffdb618829b63c94332df2116d473d741c23",
            &current,
        ),
    ];
    let mut changes = Vec::new();
    let mut staged = Vec::new();
    for (relative, pointer, old_hash, transition) in specs {
        let relative = format!("{DATA}/{relative}");
        let before = pinned(&root.join(&relative), old_hash, &mut reads);
        let old_digest = transition["before_sha256"].as_str().unwrap();
        let new_digest = transition["after_sha256"].as_str().unwrap();
        let old_value: Value = serde_json::from_slice(&before).unwrap();
        assert_eq!(old_value.pointer(pointer).unwrap(), old_digest);
        let before_text = std::str::from_utf8(&before).unwrap();
        assert_eq!(before_text.matches(old_digest).count(), 1);
        let after = before_text.replacen(old_digest, new_digest, 1).into_bytes();
        assert_eq!(after.len(), before.len());
        let mut inverse: Value = serde_json::from_slice(&after).unwrap();
        assert_eq!(inverse.pointer(pointer).unwrap(), new_digest);
        *inverse.pointer_mut(pointer).unwrap() = json!(old_digest);
        assert_eq!(inverse, old_value, "whole receipt inverse");
        let after_text = std::str::from_utf8(&after).unwrap();
        assert_eq!(after_text.matches(new_digest).count(), 1);
        assert_eq!(
            after_text.replacen(new_digest, old_digest, 1).as_bytes(),
            before
        );
        changes.push(json!({"path":relative,"pointer":pointer,
            "before_sha256":old_hash,"after_sha256":hash(&after),"bytes":after.len(),
            "before_value":old_digest,"after_value":new_digest,
            "whole_value_inverse":true,"whole_byte_inverse":true}));
        staged.push((relative, before, after));
    }
    unchanged(&reads);
    fs::create_dir(&output).unwrap();
    for (relative, before, after) in &staged {
        for (lane, bytes) in [("before", before), ("files", after)] {
            let path = output.join(lane).join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .unwrap();
            file.write_all(bytes).unwrap();
            assert_eq!(&fs::read(path).unwrap(), bytes);
        }
    }
    unchanged(&reads);
    let report = json!({"schema_version":1,"scope":"reverse-primary-two-dependent-receipts",
        "producer_sha256":hash(&source),"source_authority_changed":false,
        "installed":false,"compiled":compiled,"current":current,"changes":changes,
        "immutable_inputs":reads.iter().map(|(path,bytes)|json!({
            "path":path,"bytes":bytes.len(),"sha256":hash(bytes)})).collect::<Vec<_>>()});
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output.join("report.json"))
        .unwrap();
    file.write_all(&serde_json::to_vec_pretty(&report).unwrap())
        .unwrap();
    unchanged(&reads);
    report
}
