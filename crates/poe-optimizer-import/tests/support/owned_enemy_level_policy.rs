//! A finite policy fixture for package binding tests, not source admission proof.
use poe_optimizer_core::owned_definitions::OwnedDefinitionKey;
use poe_optimizer_import::{
    owned_mapping::OwnedMappingIndex,
    owned_normalize::EnemyLevelPolicy,
    owned_value::{DecimalSyntax, ValueCodecInput, ValueCodecKind, WhitespacePolicy},
    owned_value_policy::{
        DuplicatePolicy, MissingValuePolicy, ValueLane, ValueRecipeInput, ValueSelector, ValueTier,
    },
};

pub fn policy(mapping: &OwnedMappingIndex) -> EnemyLevelPolicy {
    EnemyLevelPolicy::PobFreshDefaultConfigLevelV1 {
        mapping_source: *mapping.source_identity(),
        absent_input_names: vec!["enemyLevel".into(), "enemyIsBoss".into()],
        placeholder: ValueRecipeInput {
            id: OwnedDefinitionKey::new("reviewed-enemy-level-placeholder").unwrap(),
            codec: ValueCodecInput {
                namespace: mapping.input().namespace.clone(),
                whitespace: WhitespacePolicy::Exact,
                codec: ValueCodecKind::Integer {
                    syntax: DecimalSyntax::Integer,
                },
            },
            tiers: vec![ValueTier {
                selectors: vec![ValueSelector {
                    lane: ValueLane::PlaceholderNumber,
                    name: "enemyLevel".into(),
                }],
                duplicates: DuplicatePolicy::Reject,
            }],
            missing: MissingValuePolicy::Pending,
            numeric_aliases: vec![],
        },
        expected_level: 82,
    }
}
