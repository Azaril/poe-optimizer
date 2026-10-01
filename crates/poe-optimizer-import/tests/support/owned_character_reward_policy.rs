//! Source authority only; no definition or Config reward dependency to repair.
use poe_optimizer_import::{
    owned_mapping::OwnedMappingIndex, owned_normalize::CharacterRewardInventoryPolicy,
};

pub fn policy(mapping: &OwnedMappingIndex) -> CharacterRewardInventoryPolicy {
    CharacterRewardInventoryPolicy::PobFreshCharacterOnlyEmptyV1 {
        mapping_source: *mapping.source_identity(),
        target_version: "0_1".into(),
        tree_version: "0_5".into(),
    }
}
