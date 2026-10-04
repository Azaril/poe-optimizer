//! Finite source-property invocation authority over existing Skill occurrences.
//! Producer contexts retain their raw-input ownership; this is not a new entity.
use crate::{
    owned_build::DeclaredSlot,
    owned_definitions::*,
    owned_rules::ContributionKind,
    owned_schema::{DeclaredSet, SchemaSubject},
    owned_support_receiving::{SupportAdmissionContext, SupportTargetDefinition},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourcePropertyPreparationInput {
    pub relations: DeclaredSet<SourcePropertyRelation>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourcePropertyOccurrence {
    AuthoredSkillUseV1,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourcePropertyAliasPolicy {
    RejectSharedBackingGemV1,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourcePropertyContext {
    PlayerScenarioV1,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourcePropertyCensus {
    /// Coalesce admission of the exact same selected position across effects;
    /// distinct retained positions of one source remain distinct contributions.
    ExactSelectedPositionV1,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourcePropertyRelation {
    pub id: OwnedDefinitionKey,
    pub owner: SupportTargetDefinition,
    pub occurrence: SourcePropertyOccurrence,
    pub aliases: SourcePropertyAliasPolicy,
    pub context: SourcePropertyContext,
    pub census: SourcePropertyCensus,
    pub census_stage: OwnedDefinitionKey,
    pub effects: DeclaredSet<SourcePropertyEffect>,
    /// Computed facts of this exact input owner, never caller-supplied values.
    pub inputs: Vec<StatDefId>,
    pub channels: DeclaredSet<SourcePropertyChannel>,
    /// Complete independently of supports, including when none are admitted.
    pub external: DeclaredSet<SourcePropertyExternalProgram>,
    pub supports: DeclaredSet<SourcePropertySupportPrograms>,
    /// Programs owned by the relation's input declaration.
    pub assembly: DeclaredSet<OwnedDefinitionKey>,
    /// Integer Skill channel, produced only by the sealed selected census.
    pub non_hidden_count: StatDefId,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourcePropertyEffect {
    pub endpoint: SourcePropertyEffectEndpoint,
    pub admission: SupportAdmissionContext,
}
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SourcePropertyEffectEndpoint {
    /// Valid only for a Direct Skill. A physical Gem is not its supplied effect.
    DirectOwner {},
    Generated {
        path: Vec<DeclaredSlot<GrantSlotDefId>>,
        skill_supply: DeclaredSlot<SkillGrantSlotDefId>,
    },
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourcePropertyChannel {
    pub stat: StatDefId,
    pub contribution: ContributionKind,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourcePropertyExternalProgram {
    pub owner: SchemaSubject,
    pub program: OwnedDefinitionKey,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourcePropertySupportPrograms {
    pub gem: GemDefId,
    pub counted: bool,
    pub programs: DeclaredSet<OwnedDefinitionKey>,
}
