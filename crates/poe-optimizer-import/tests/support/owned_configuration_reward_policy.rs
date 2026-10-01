//! Exact shipped-rule census for publication tests, not source runtime proof.
use poe_optimizer_import::{
    owned_mapping::OwnedMappingIndex,
    owned_normalize::{ConfigurationRewardControl, ConfigurationRewardInventoryPolicy},
    owned_reward_policy::OwnedRewardPolicy,
};

pub fn policy(
    mapping: &OwnedMappingIndex,
    rewards: &OwnedRewardPolicy,
) -> ConfigurationRewardInventoryPolicy {
    let controls = rewards
        .rules()
        .map(|rule| {
            assert_eq!(rule.recipe.tiers.len(), 1);
            assert_eq!(rule.recipe.tiers[0].selectors.len(), 1);
            ConfigurationRewardControl {
                recipe: rule.recipe.id.clone(),
                selector: rule.recipe.tiers[0].selectors[0].clone(),
            }
        })
        .collect::<Vec<_>>();
    assert!(!controls.is_empty());
    ConfigurationRewardInventoryPolicy::PobFreshGeneratedControlsV1 {
        mapping_source: *mapping.source_identity(),
        reward_policy: *rewards.identity(),
        controls,
    }
}

pub fn assert_rebound(
    before: &ConfigurationRewardInventoryPolicy,
    after: &ConfigurationRewardInventoryPolicy,
    rewards: &OwnedRewardPolicy,
) {
    let mut expected = before.clone();
    let ConfigurationRewardInventoryPolicy::PobFreshGeneratedControlsV1 { reward_policy, .. } =
        &mut expected;
    *reward_policy = *rewards.identity();
    assert_eq!(
        after, &expected,
        "only the dependent reward digest may change"
    );
}
