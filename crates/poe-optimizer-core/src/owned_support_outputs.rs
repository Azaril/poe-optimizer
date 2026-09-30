//! Injected names for final support-prepared type membership, not caller values.
use crate::{
    data::DataIdentity, owned_content::OwnedContentDigest, owned_definitions::*,
    owned_support_inputs::SupportTypeStat,
};
use serde::{Deserialize, Serialize};

pub const OWNED_SUPPORT_OUTPUT_BINDINGS_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportOutputBindingsInput {
    pub schema_version: u32,
    pub namespace: GameVersionNamespace,
    pub release: OwnedDefinitionKey,
    pub definitions: DataIdentity,
    pub rules: OwnedContentDigest,
    pub preparation: OwnedContentDigest,
    pub inputs: OwnedContentDigest,
    pub receiving: OwnedContentDigest,
    pub stages: OwnedContentDigest,
    /// Native preparation publishes these channels at this explicit stage.
    pub output_stage: OwnedDefinitionKey,
    /// Exact finite vocabulary, mapped to distinct Boolean Skill-only stats.
    /// Initial input channels are never overwritten. Runtime export additionally
    /// requires one authoritative preparation context for the exact SkillTarget.
    pub final_skill_types: Vec<SupportTypeStat>,
}
