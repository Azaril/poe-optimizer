//! A source-bound empty character reward inventory, independent of Config rewards.
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::OwnedDefinitionKey};
use poe_optimizer_import::{
    owned_normalize::CharacterRewardInventoryPolicy,
    owned_release::{
        OwnedReleaseError, OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release,
    },
    owned_tree_policy::OwnedTreeNormalizationPolicy,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{fs, path::PathBuf};

pub fn directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/character-reward-inventory")
}
fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(directory().join(name)).unwrap()).unwrap()
}
fn key(text: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(text).unwrap()
}
pub fn policy() -> CharacterRewardInventoryPolicy {
    read("policy.json")
}
pub fn authoring() -> Value {
    read("authoring.json")
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    let authored = authoring();
    let receipt = serde_json::to_value(prior.receipt()).unwrap();
    for field in ["definitions", "registry"] {
        assert_eq!(authored[field], receipt[field]);
    }
    assert_eq!(authored["before"], receipt["input"]);
    assert_eq!(authored["source_validation"]["status"], "passed");
    let mut full = prior.input().clone();
    assert!(full.normalization.character_reward_inventory.is_none());
    assert!(full.evaluation.is_none());
    full.normalization.character_reward_inventory = Some(policy());
    full.normalization.version = key("selected-character-reward-inventory-v1");
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
        kind: key("explicit-character-reward-inventory"),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-character-reward-inventory-authoring-v1",
            &(authored, policy()),
            2 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    preservation(prior, &next);
    next
}
pub fn preservation(prior: &StagedOwnedRelease, next: &StagedOwnedRelease) {
    let old = prior.input();
    let mut restored = next.input().clone();
    assert_eq!(
        restored.normalization.character_reward_inventory,
        Some(policy())
    );
    assert_eq!(
        restored.normalization.version,
        key("selected-character-reward-inventory-v1")
    );
    restored.normalization.character_reward_inventory =
        old.normalization.character_reward_inventory.clone();
    restored.normalization.version = old.normalization.version.clone();
    assert!(restored.normalization == old.normalization);
    assert_eq!(
        restored.tree.as_ref().unwrap().content,
        old.tree.as_ref().unwrap().content
    );
    restored.tree = old.tree.clone();
    assert_eq!(restored.provenance.len(), old.provenance.len() + 1);
    let added = restored.provenance.pop().unwrap();
    assert_eq!(added.kind, key("explicit-character-reward-inventory"));
    assert_eq!(added.prior_input, prior.receipt().input);
    assert!(
        restored == *old,
        "only optional character inventory, its tree binding and one provenance row change"
    );
    assert_eq!(next.receipt().definitions, prior.receipt().definitions);
    assert_eq!(next.receipt().registry, prior.receipt().registry);
}
pub fn assert_stale_source_rejected(next: &StagedOwnedRelease) {
    let mut stale = next.input().clone();
    let CharacterRewardInventoryPolicy::PobFreshCharacterOnlyEmptyV1 { mapping_source, .. } = stale
        .normalization
        .character_reward_inventory
        .as_mut()
        .unwrap();
    *mapping_source = digest_owned("stale-character-reward-source", &0, 128).unwrap();
    assert!(matches!(
        assemble_owned_release(stale, Default::default()),
        Err(OwnedReleaseError::Normalization(_))
    ));
}
