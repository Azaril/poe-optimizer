//! Test loader for shipped production files, using ordinary public consumers.
use poe_optimizer_data::{
    owned_schema::*,
    skill_identities::{SkillIdentityCatalog, SkillIdentityData},
};
use poe_optimizer_import::{
    owned_item_lines::*, owned_item_source::*, owned_mapping::*, owned_normalize::*,
    owned_reward_policy::*, owned_skill_catalog::*,
};
use serde::de::DeserializeOwned;
use std::{fs::File, io::Read, path::Path};
pub fn bytes(path: &Path) -> Vec<u8> {
    let mut out = vec![];
    File::open(path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut out)
        .unwrap();
    assert!(out.len() <= 16 * 1024 * 1024);
    out
}
pub fn json<T: DeserializeOwned>(path: &Path) -> T {
    serde_json::from_slice(&bytes(path)).unwrap()
}
pub struct ProductionArtifacts {
    pub catalog: SkillIdentityCatalog,
    pub registry: OwnedIdRegistry,
    pub definitions: OwnedDefinitionSchemaPackage,
    pub mappings: OwnedMappingIndex,
    pub roles: OwnedSkillRoleIndex,
    pub rewards: OwnedRewardPolicy,
    pub items: OwnedItemLinePolicy,
    pub item_source: ItemSourceLayoutPolicy,
    pub policy: NormalizationPolicy,
}
pub fn load(root: &Path) -> ProductionArtifacts {
    let dir = root.join("data/owned/poe2/3887ae68/import");
    let compiled = dir.join("compiled");
    let policy_dir = dir.join("policies");
    let catalog = SkillIdentityCatalog::new(json::<SkillIdentityData>(
        &dir.join("skill-identities.json"),
    ))
    .unwrap();
    let limits = OwnedMappingLimits::default();
    let registry = decode_registry(&bytes(&compiled.join("registry.json")), limits).unwrap();
    let definitions = decode_schema_package(
        &bytes(&compiled.join("schema.json")),
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    let mappings = decode_mapping_package(
        &bytes(&compiled.join("mapping.json")),
        &registry,
        &definitions,
        limits,
    )
    .unwrap();
    let roles = OwnedSkillRoleIndex::new(
        json(&compiled.join("roles.json")),
        &mappings,
        &definitions,
        SkillCatalogLimits::default(),
    )
    .unwrap();
    let rewards = decode_reward_policy(
        &bytes(&policy_dir.join("rewards.json")),
        &mappings,
        &definitions,
        RewardPolicyLimits::default(),
    )
    .unwrap();
    let items = decode_item_line_policy(
        &bytes(&policy_dir.join("items.json")),
        &definitions,
        ItemLineLimits::default(),
    )
    .unwrap();
    let item_source = decode_item_source_policy(
        &bytes(&policy_dir.join("item-source.json")),
        &items,
        &definitions,
        ItemSourceLimits::default(),
    )
    .unwrap();
    let policy = json(&policy_dir.join("normalization.json"));
    ProductionArtifacts {
        catalog,
        registry,
        definitions,
        mappings,
        roles,
        rewards,
        items,
        item_source,
        policy,
    }
}
