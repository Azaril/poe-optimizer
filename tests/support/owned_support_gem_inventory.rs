//! Source inventory publication, preserving all scalar and static rule coverage.
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::OwnedDefinitionKey};
use poe_optimizer_import::{
    owned_normalize::{
        GemInventoryPolicy, SingleSupportGemInventory, gem_inventory_scalar_inputs_identity,
    },
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_successor::{SuccessorBundleInput, transition_owned_normalization_with_tree_compact},
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{fs, path::PathBuf};

pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/owned/poe2/3887ae68/support-gem-inventory")
}
fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
pub fn inventory() -> Vec<SingleSupportGemInventory> {
    read("inventory.json")
}
pub fn authoring() -> Value {
    read("authoring.json")
}
pub fn policy(prior: &StagedOwnedRelease) -> GemInventoryPolicy {
    let a = authoring();
    let r = serde_json::to_value(prior.receipt()).unwrap();
    assert_eq!(r["input"], a["before"]);
    for field in [
        "definitions",
        "registry",
        "mapping",
        "roles",
        "normalization",
    ] {
        assert_eq!(r[field], a[field], "exact predecessor {field}");
    }
    GemInventoryPolicy::PobFreshSingleSupportV1 {
        definitions: prior.receipt().definitions.clone(),
        roles: prior.receipt().roles,
        catalog: serde_json::from_value(a["catalog"].clone()).unwrap(),
        scalar_inputs: gem_inventory_scalar_inputs_identity(
            &prior.input().normalization,
            Default::default(),
        )
        .unwrap(),
        gems: inventory(),
    }
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    let b = prior.input();
    assert!(b.normalization.gem_inventory.is_none());
    assert!(b.evaluation.is_none());
    let p = policy(prior);
    let mut normalization = b.normalization.clone();
    normalization.gem_inventory = Some(p.clone());
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
    full.tree = transition.tree().map(|t| t.input().clone());
    assert_eq!(full.normalization, normalization);
    assert_eq!(
        full.tree.as_ref().unwrap().content,
        b.tree.as_ref().unwrap().content
    );
    full.provenance.push(OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new("explicit-single-support-physical-inventory").unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-single-support-physical-inventory-v1",
            &(authoring(), &p),
            2 * 1024 * 1024,
        )
        .unwrap(),
    });
    let result = assemble_owned_release(full, Default::default()).unwrap();
    preservation(prior, &result);
    result
}
pub fn preservation(prior: &StagedOwnedRelease, next: &StagedOwnedRelease) {
    let b = prior.input();
    let mut copy = next.input().clone();
    assert_eq!(copy.normalization.gem_inventory, Some(policy(prior)));
    copy.normalization.gem_inventory = None;
    assert_eq!(
        copy.tree.as_ref().unwrap().content,
        b.tree.as_ref().unwrap().content
    );
    copy.tree = b.tree.clone();
    assert_eq!(copy.provenance.len(), b.provenance.len() + 1);
    copy.provenance.pop();
    assert!(
        copy == *b,
        "only optional physical inventory proof, tree commitment and provenance may change"
    );
    assert_eq!(next.receipt().definitions, prior.receipt().definitions);
    assert_eq!(next.receipt().registry, prior.receipt().registry);
    assert_eq!(next.receipt().rules, prior.receipt().rules);
    assert_eq!(next.receipt().mapping, prior.receipt().mapping);
    assert_eq!(next.receipt().roles, prior.receipt().roles);
    assert!(next.input().evaluation.is_none());
}
