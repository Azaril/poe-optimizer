//! Injected computed-value bindings for support preparation, without value authority.
use crate::{data::DataIdentity, owned_content::OwnedContentDigest, owned_definitions::*};
use serde::{Deserialize, Serialize};

pub const OWNED_SUPPORT_INPUT_BINDINGS_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportInputBindingsInput {
    pub schema_version: u32,
    pub namespace: GameVersionNamespace,
    pub release: OwnedDefinitionKey,
    pub definitions: DataIdentity,
    pub rules: OwnedContentDigest,
    pub preparation: OwnedContentDigest,
    pub stages: OwnedContentDigest,
    pub preparation_stage: OwnedDefinitionKey,
    /// Integer computed for each exact support assignment.
    pub effective_level: StatDefId,
    /// Quantity computed for each exact support assignment in the preparation unit.
    pub effective_quality: StatDefId,
    /// Boolean facts computed for the exact assigned SkillTarget.
    pub target: SupportTargetInputBindings,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportTypeStat {
    pub support_type: OwnedDefinitionKey,
    pub stat: StatDefId,
}

/// Presence is independently computed: missing data never means an absent or
/// empty collection. Members cover the entire preparation type vocabulary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OptionalTypeInputs {
    pub present: StatDefId,
    pub members: Vec<SupportTypeStat>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OptionalTypeContextInputs {
    pub present: StatDefId,
    pub skill_types: Vec<SupportTypeStat>,
    pub minion_types: OptionalTypeInputs,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportTargetInputBindings {
    pub skill_types: Vec<SupportTypeStat>,
    pub minion_types: OptionalTypeInputs,
    pub summoner: OptionalTypeContextInputs,
    pub cannot_be_supported: StatDefId,
    pub has_gem: StatDefId,
    pub from_item: StatDefId,
    pub is_player_actor: StatDefId,
}
