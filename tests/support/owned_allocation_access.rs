//! Checked tree-only publication; the source policy adds no numerical authority.
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::OwnedDefinitionKey};
use poe_optimizer_import::{
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_tree_policy::AllocationAccessPolicy,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{fs, path::PathBuf};

pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/ordinary-allocation-access")
}
fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
pub fn authoring() -> Value {
    read("authoring.json")
}
pub fn policy() -> AllocationAccessPolicy {
    read("policy.json")
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
    assert!(tree.content.access.is_none());
    assert_eq!(
        serde_json::to_value(tree.content.catalog).unwrap(),
        a["catalog"]
    );
    tree.content.access = Some(policy());
    full.provenance.push(OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new("independent-saved-allocation-access").unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-independent-saved-allocation-access-v1",
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
    assert_eq!(tree.content.access.take(), Some(policy()));
    assert_eq!(
        restored.provenance.len(),
        prior.input().provenance.len() + 1
    );
    restored.provenance.pop();
    assert!(
        restored == *prior.input(),
        "only the tree access policy and explicit provenance may change"
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
