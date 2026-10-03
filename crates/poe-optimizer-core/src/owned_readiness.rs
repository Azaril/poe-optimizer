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
    /// Only these exact Skill definitions receive earlier required-input gates.
    pub skills: Vec<GeneratedSkillReadiness>,
    /// Complete classification of every owner-qualified rule program.
    pub programs: DeclaredSet<ReadinessProgram>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratedSkillReadiness {
    pub skill: SkillDefId,
    /// Exact complete partition of the Skill's RequiredOnce input declarations.
    pub parameters: DeclaredSet<ParameterReadiness>,
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
    Execution,
}
