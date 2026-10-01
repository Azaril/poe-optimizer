//! Checked authoring delta; the emitted policy is fully explicit and self-contained.
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::OwnedDefinitionKey};
use poe_optimizer_import::{
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_tree_policy::AllocationAccessPolicy,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/scoped-allocation-access")
}
pub fn authoring() -> Value {
    serde_json::from_slice(&fs::read(data().join("authoring.json")).unwrap()).unwrap()
}
pub fn predecessor_policy() -> AllocationAccessPolicy {
    let a = authoring();
    let reference = &a["base_policy"];
    assert_eq!(reference["hash_encoding"], "utf8_lf");
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(reference["path"].as_str().unwrap());
    let text = fs::read_to_string(path).unwrap().replace("\r\n", "\n");
    assert_eq!(
        format!("{:x}", Sha256::digest(text.as_bytes())),
        reference["sha256"]
    );
    serde_json::from_str(&text).unwrap()
}
pub fn policy() -> AllocationAccessPolicy {
    let AllocationAccessPolicy::PobIndependentSavedPathsV1 { pools, nodes } = predecessor_policy()
    else {
        panic!("reviewed authoring family must remain V1")
    };
    AllocationAccessPolicy::PobIndependentSavedPathsV2 { pools, nodes }
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    let a = authoring();
    let r = serde_json::to_value(prior.receipt()).unwrap();
    assert_eq!(r["input"], a["before"]);
    for field in [
        "definitions",
        "registry",
        "mapping",
        "normalization",
        "tree",
    ] {
        assert_eq!(r[field], a[field], "exact predecessor {field}");
    }
    let mut full = prior.input().clone();
    assert!(full.evaluation.is_none());
    let tree = full.tree.as_mut().unwrap();
    assert_eq!(tree.content.access, Some(predecessor_policy()));
    assert_eq!(
        serde_json::to_value(tree.content.catalog).unwrap(),
        a["catalog"]
    );
    tree.content.access = Some(policy());
    full.provenance.push(OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new("independent-scoped-saved-allocation-access").unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-independent-scoped-allocation-access-v1",
            &(a, policy()),
            2 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    preservation(prior, &next);
    next
}
pub fn preservation(prior: &StagedOwnedRelease, next: &StagedOwnedRelease) {
    let mut restored = next.input().clone();
    let tree = restored.tree.as_mut().unwrap();
    assert_eq!(
        tree.content.access.replace(predecessor_policy()),
        Some(policy())
    );
    assert_eq!(
        serde_json::to_vec(tree).unwrap(),
        serde_json::to_vec(prior.input().tree.as_ref().unwrap()).unwrap(),
        "restored V1 tree bytes"
    );
    assert_eq!(
        restored.provenance.len(),
        prior.input().provenance.len() + 1
    );
    restored.provenance.pop();
    assert!(
        restored == *prior.input(),
        "only access profile and provenance change"
    );
    let before = serde_json::to_value(prior.receipt()).unwrap();
    let after = serde_json::to_value(next.receipt()).unwrap();
    for field in [
        "definitions",
        "registry",
        "rules",
        "compiled_rules",
        "routing",
        "mapping",
        "roles",
        "normalization",
        "rewards",
        "items",
        "item_source",
        "source",
        "query_sets",
        "query_rows",
        "query_policy_bytes",
    ] {
        assert_eq!(before[field], after[field], "unchanged {field}");
    }
}
