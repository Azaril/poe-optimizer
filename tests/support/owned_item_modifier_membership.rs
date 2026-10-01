//! Checked normalization-only publication; no item/template coverage is rewritten.
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::OwnedDefinitionKey};
use poe_optimizer_import::{
    owned_normalize::ItemModifierMembershipPolicy,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_successor::{SuccessorBundleInput, transition_owned_normalization_with_tree_compact},
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{fs, path::PathBuf};

pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/item-modifier-membership")
}
fn read<T: DeserializeOwned>(file: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(file)).unwrap()).unwrap()
}
pub fn policy() -> ItemModifierMembershipPolicy {
    read("policy.json")
}
pub fn authoring() -> Value {
    read("authoring.json")
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    let a = authoring();
    assert_eq!(
        serde_json::to_value(prior.receipt().input).unwrap(),
        a["before"]
    );
    assert_eq!(
        serde_json::to_value(prior.receipt().normalization).unwrap(),
        a["normalization"]
    );
    assert_eq!(
        serde_json::to_value(&prior.receipt().definitions).unwrap(),
        a["definitions"]
    );
    assert_eq!(
        serde_json::to_value(prior.receipt().items).unwrap(),
        a["items"]
    );
    assert_eq!(
        serde_json::to_value(prior.receipt().item_source).unwrap(),
        a["item_source"]
    );
    let p = policy();
    let b = prior.input();
    assert!(b.normalization.item_modifier_membership.is_none());
    let mut normalization = b.normalization.clone();
    normalization.item_modifier_membership = Some(p.clone());
    let transition = transition_owned_normalization_with_tree_compact(
        SuccessorBundleInput {
            schema_version: 1,
            prior: b.recipe.clone(),
            successor: b.recipe.clone(),
            mapping: b.mapping.clone(),
            roles: b.roles.clone(),
            normalization: b.normalization.clone(),
            rewards: b.rewards.clone(),
            query_sets: b.query_sets.clone(),
            items: b.items.clone(),
            item_source: b.item_source.clone(),
        },
        b.tree.clone().unwrap(),
        normalization.clone(),
        Default::default(),
    )
    .unwrap();
    let mut full = b.clone();
    full.normalization = transition.normalization().clone();
    full.tree = transition.tree().map(|v| v.input().clone());
    assert_eq!(full.normalization, normalization);
    assert_eq!(
        full.tree.as_ref().unwrap().content,
        b.tree.as_ref().unwrap().content
    );
    full.provenance.push(OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new("explicit-singleton-item-modifier-membership").unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-singleton-item-modifier-membership-v1",
            &(&a, &p),
            1024 * 1024,
        )
        .unwrap(),
    });
    let staged = assemble_owned_release(full.clone(), Default::default()).unwrap();
    full.normalization.item_modifier_membership = None;
    full.tree = b.tree.clone();
    full.provenance.pop();
    assert!(
        full == *b,
        "only authored policy, tree binding and provenance may change"
    );
    staged
}
