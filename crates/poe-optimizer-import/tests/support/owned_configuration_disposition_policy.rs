//! Injected partial responsibility rows for publication binding regressions.
use poe_optimizer_import::{
    owned_mapping::OwnedMappingIndex, owned_normalize::*, owned_reward_policy::OwnedRewardPolicy,
};

pub fn policy(
    normalization: &NormalizationPolicy,
    mapping: &OwnedMappingIndex,
    rewards: &OwnedRewardPolicy,
) -> ConfigurationDispositionPolicy {
    ConfigurationDispositionPolicy {
        inventory: None,
        mapping_source: *mapping.source_identity(),
        catalogue_evidence: *mapping.identity(),
        constructor_evidence: *rewards.identity(),
        dependencies: configuration_disposition_dependencies(
            normalization,
            rewards,
            Default::default(),
        )
        .unwrap(),
        rules: rewards
            .rules()
            .map(|r| ConfigurationDispositionRule {
                id: r.recipe.id.clone(),
                selector: r.recipe.tiers[0].selectors[0].clone(),
                destinations: vec![ConfigurationDestination::Adapter {
                    control: ConfigurationAdapterControl::Reward {
                        recipe: r.recipe.id.clone(),
                    },
                }],
            })
            .collect(),
    }
}

pub fn assert_rebound(
    before: &ConfigurationDispositionPolicy,
    normalization: &NormalizationPolicy,
    rewards: &OwnedRewardPolicy,
) {
    let mut expected = before.clone();
    expected.dependencies =
        configuration_disposition_dependencies(normalization, rewards, Default::default()).unwrap();
    assert_eq!(
        normalization.configuration_dispositions.as_ref(),
        Some(&expected)
    );
    assert_ne!(expected.dependencies, before.dependencies);
}
