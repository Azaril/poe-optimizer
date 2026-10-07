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

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SourcePropertyOccurrence {
    /// Admit only authored SkillUses of the relation's input definition.
    /// An authored physical Gem remains a container, not a Skill effect.
    AuthoredSkillUse {},
    /// Admit only generated Skills from this exact supply declaration. The
    /// relation owner must equal the supplied Skill definition. Concrete
    /// providers are discovered from the candidate's existing occurrence graph;
    /// the same-definition authored Skill is not implicitly admitted.
    GeneratedSkill {
        skill_supply: DeclaredSlot<SkillGrantSlotDefId>,
    },
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
    /// Each program's definition owner follows its explicit binding. Concrete
    /// source ownership remains the exact existing SkillTarget; this field
    /// grants no additional raw-input storage or parameter-write authority.
    pub assembly: DeclaredSet<SourcePropertyAssemblyProgram>,
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
    /// The source's exact concrete Skill effect, authored or generated as
    /// admitted by the relation. A physical Gem is not its supplied effect.
    OwnerSkill {},
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

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourcePropertyAssemblyBinding {
    /// Use the relation's input definition and its exact concrete occurrence.
    /// Existing Gem child projection and Direct Skill facts retain their scope.
    InputOwner,
    /// Valid only for a generated source. Infer the program owner from its
    /// exact supply declaration and bind the occurrence to the DECLARING
    /// provider (GeneratedSkillKey.provider), not the entered child provider.
    /// Existing projection authority may write only this exact supplied Skill;
    /// property reads remain sealed to the separately bound source Skill.
    ExactSupplyingProvider,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourcePropertyAssemblyProgram {
    pub program: OwnedDefinitionKey,
    /// Determines the program's owner; callers cannot supply an unrelated owner
    /// or concrete provider identity alongside the relation's declared binding.
    pub binding: SourcePropertyAssemblyBinding,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourcePropertySupportPrograms {
    pub gem: GemDefId,
    pub counted: bool,
    pub programs: DeclaredSet<OwnedDefinitionKey>,
}
