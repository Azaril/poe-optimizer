//! Explicit readiness over the existing occurrence graph, independent of scheduling.
use crate::{
    owned_build::DeclaredSlot, owned_definitions::*, owned_schema::*, owned_stages::StageChannel,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessPhase {
    Structural,
    Preparation,
    Execution,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadinessInput {
    /// Only these exact Skill definitions receive declared readiness requirements.
    pub skills: Vec<SkillReadiness>,
    /// Complete classification of every owner-qualified rule program.
    pub programs: DeclaredSet<ReadinessProgram>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillReadiness {
    pub skill: SkillDefId,
    /// Exact complete partition of the Skill's RequiredOnce input declarations.
    pub parameters: DeclaredSet<ParameterReadiness>,
    /// Execution requires this exact Boolean on the concrete Skill and its
    /// descendants. Preparation retains mechanical gates without this implicit
    /// requirement. Omission declares no participation requirement; it is not a
    /// default value for a missing producer or usage preference.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::owned_build::non_null_extension"
    )]
    pub participation: Option<StatDefId>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParameterReadiness {
    pub parameter: DeclaredSlot<ParameterSlotDefId>,
    pub phase: ReadinessPhase,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadinessProgram {
    pub owner: SchemaSubject,
    pub program: OwnedDefinitionKey,
    pub phase: ReadinessPhase,
    pub role: ReadinessProgramRole,
    /// Exact potential writes for early programs; execution uses its ordinary
    /// checked rule authority and must leave this list empty.
    pub outputs: Vec<StageChannel>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessProgramRole {
    PreparationFacts,
    FinalInputAssembly,
    SupportPreparationApplicability,
    SupportedPreparationProperty,
    SourceSupportedProperty,
    SourceExternalProperty,
    SourceFinalInputAssembly,
    Execution,
}
