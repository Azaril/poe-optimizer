//! Injected finite scheduling declarations. These never establish rule coverage.
use crate::{
    data::DataIdentity, owned_build::DeclaredSlot, owned_content::OwnedContentDigest,
    owned_definitions::*, owned_readiness::ReadinessInput, owned_rules::ContributionKind,
    owned_schema::*,
};
use serde::{Deserialize, Serialize};

pub const OWNED_EVALUATION_STAGES_VERSION: u32 = 1;
pub const OWNED_EVALUATION_STAGES_V2: u32 = 2;
pub const OWNED_EVALUATION_STAGES_V3: u32 = 3;
/// Explicit early local item derivations; rule operations and defaults stay unchanged.
pub const OWNED_EVALUATION_STAGES_V4: u32 = 4;
fn readiness_non_null<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<ReadinessInput>, D::Error> {
    ReadinessInput::deserialize(d).map(Some)
}

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
    /// Required for V2–V4. Omission preserves the historical V1 wire bytes.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "readiness_non_null"
    )]
    pub readiness: Option<ReadinessInput>,
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
