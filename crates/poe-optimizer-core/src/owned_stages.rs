//! Injected finite scheduling declarations. These never establish rule coverage.
use crate::{
    data::DataIdentity, owned_build::DeclaredSlot, owned_content::OwnedContentDigest,
    owned_definitions::*, owned_rules::ContributionKind, owned_schema::*,
};
use serde::{Deserialize, Serialize};

pub const OWNED_EVALUATION_STAGES_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationStagesInput {
    pub schema_version: u32,
    pub namespace: GameVersionNamespace,
    pub release: OwnedDefinitionKey,
    pub definitions: DataIdentity,
    pub rules: OwnedContentDigest,
    pub routing: OwnedContentDigest,
    pub stages: Vec<EvaluationStage>,
    /// IDs are local to their owner. Complete means classified, not executable.
    pub programs: DeclaredSet<StagedRuleProgram>,
    /// V15 effect applications have their own explicit classification, without
    /// fabricating a definition owner for their source/recipient invocation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect_applications: Option<DeclaredSet<StagedEffectApplication>>,
    pub routing_stage: OwnedDefinitionKey,
    pub frozen_channels: Vec<FrozenStageChannel>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationStage {
    pub id: OwnedDefinitionKey,
    pub predecessors: Vec<OwnedDefinitionKey>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StagedRuleProgram {
    pub owner: SchemaSubject,
    pub program: OwnedDefinitionKey,
    pub stage: OwnedDefinitionKey,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StagedEffectApplication {
    pub application: OwnedDefinitionKey,
    pub stage: OwnedDefinitionKey,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenStageChannel {
    pub channel: StageChannel,
    pub stage: OwnedDefinitionKey,
}
/// Static channel families cover every potential occurrence in the named scope.
/// Final values, contribution streams and ordered transforms are distinct.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum StageChannel {
    Stat {
        scope: RuleEntityKind,
        stat: StatDefId,
    },
    Capability {
        scope: RuleEntityKind,
        capability: CapabilityDefId,
    },
    Contributions {
        scope: RuleEntityKind,
        stat: StatDefId,
        contribution: ContributionKind,
    },
    ModifierTransforms {
        stat: StatDefId,
    },
    Grant {
        slot: DeclaredSlot<GrantSlotDefId>,
    },
    SkillParameter {
        parameter: DeclaredSlot<ParameterSlotDefId>,
    },
}
