//! Optional checked completion of the configuration reward inventory.
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::OwnedDefinitionKey};
use poe_optimizer_import::{
    owned_normalize::ConfigurationRewardInventoryPolicy,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_tree_policy::OwnedTreeNormalizationPolicy,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{fs, path::PathBuf};

pub fn directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/configuration-reward-inventory")
}
fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(directory().join(name)).unwrap()).unwrap()
}
fn key(text: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(text).unwrap()
}
pub fn policy() -> ConfigurationRewardInventoryPolicy {
    read("policy.json")
}
pub fn authoring() -> Value {
    read("authoring.json")
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    let authored = authoring();
    let seed: Value = serde_json::from_slice(
        &fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("data/owned/poe2/3887ae68/import/reward-policy-seed.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(
        serde_json::to_value(&prior.input().rewards.rules).unwrap() == seed["rules"],
        "the complete-source witness must cover the exact unchanged reward recipes"
    );
    assert_eq!(
        authored["before"],
        serde_json::to_value(prior.receipt().input).unwrap()
    );
    assert_eq!(
        authored["definitions"],
        serde_json::to_value(&prior.receipt().definitions).unwrap()
    );
    assert_eq!(
        authored["registry"],
        serde_json::to_value(prior.receipt().registry).unwrap()
    );
    assert_eq!(
        authored["source_validation"]["status"], "passed",
        "source witness required before publication"
    );
    let mut full = prior.input().clone();
    assert!(full.normalization.configuration_reward_inventory.is_none());
    full.normalization.configuration_reward_inventory = Some(policy());
    full.normalization.version = key("selected-configuration-reward-inventory-v1");
    full.tree = Some(
        OwnedTreeNormalizationPolicy::bind_new(
            full.tree.as_ref().unwrap().content.clone(),
            prior.assembled().registry(),
            prior.assembled().schema(),
            prior.mapping(),
            &full.normalization,
            Default::default(),
        )
        .unwrap()
        .input()
        .clone(),
    );
    full.provenance.push(OwnedReleaseProvenance {
        kind: key("explicit-configuration-reward-inventory"),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-configuration-reward-inventory-authoring-v1",
            &(authored, policy()),
            2 * 1024 * 1024,
        )
        .unwrap(),
    });
    let staged = assemble_owned_release(full, Default::default()).unwrap();
    preservation(prior, &staged);
    staged
}
pub fn preservation(prior: &StagedOwnedRelease, next: &StagedOwnedRelease) {
    let old = prior.input();
    let mut restored = next.input().clone();
    assert_eq!(
        restored.normalization.configuration_reward_inventory,
        Some(policy())
    );
    assert_eq!(
        restored.normalization.version,
        key("selected-configuration-reward-inventory-v1")
    );
    restored.normalization.configuration_reward_inventory =
        old.normalization.configuration_reward_inventory.clone();
    restored.normalization.version = old.normalization.version.clone();
    assert!(
        restored.normalization == old.normalization,
        "unrelated normalization policy changed"
    );
    assert_eq!(
        restored.tree.as_ref().unwrap().content,
        old.tree.as_ref().unwrap().content
    );
    restored.tree = old.tree.clone();
    assert_eq!(restored.provenance.len(), old.provenance.len() + 1);
    restored.provenance.pop();
    assert!(
        restored == *old,
        "only optional source admission and its explicit commitments change"
    );
    assert_eq!(next.receipt().definitions, prior.receipt().definitions);
    assert_eq!(next.receipt().registry, prior.receipt().registry);
}
