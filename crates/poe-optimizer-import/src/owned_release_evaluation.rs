//! Offline packaging of immutable evaluator artifacts. Validation establishes
//! exact dependencies, not active contributors or numerical build coverage.
use crate::owned_recipe::StagedOwnedRecipe;
use poe_optimizer_core::{
    owned_content::OwnedContentDigest,
    owned_metrics::MetricMappingInput,
    owned_schema::{SchemaClosure, SchemaState},
    owned_stages::EvaluationStagesInput,
    owned_support_inputs::SupportInputBindingsInput,
    owned_support_outputs::SupportOutputBindingsInput,
    owned_support_receiving::{SupportAdmissionContext, SupportReceivingInput},
    owned_supports::{SupportPreparationInput, SupportTypePredicate},
};
use poe_optimizer_data::{
    owned_metrics::{MetricMappingError, MetricMappingLimits, OwnedMetricMapping},
    owned_stages::{OwnedEvaluationStages, StageStorageError, StageStorageLimits},
    owned_support_inputs::{
        OwnedSupportInputBindings, SupportInputStorageError, SupportInputStorageLimits,
    },
    owned_support_outputs::{
        OwnedSupportOutputBindings, SupportOutputDependencies, SupportOutputStorageError,
        SupportOutputStorageLimits,
    },
    owned_support_receiving::{
        OwnedSupportReceiving, SupportReceivingStorageError, SupportReceivingStorageLimits,
    },
    owned_supports::{OwnedSupportPreparation, SupportStorageError, SupportStorageLimits},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedReleaseEvaluationInput {
    pub metrics: MetricMappingInput,
    /// Omission selects ordinary evaluation. A support bundle is indivisible;
    /// every dependency except final type publication is explicitly supplied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support: Option<OwnedReleaseSupportInput>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedReleaseSupportInput {
    pub stages: EvaluationStagesInput,
    pub preparation: SupportPreparationInput,
    pub inputs: SupportInputBindingsInput,
    pub receiving: SupportReceivingInput,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outputs: Option<SupportOutputBindingsInput>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedReleaseEvaluationReceipt {
    pub metrics: OwnedContentDigest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support: Option<OwnedReleaseSupportReceipt>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedReleaseSupportReceipt {
    pub stages: OwnedContentDigest,
    pub preparation: OwnedContentDigest,
    pub inputs: OwnedContentDigest,
    pub receiving: OwnedContentDigest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outputs: Option<OwnedContentDigest>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct OwnedReleaseEvaluationLimits {
    pub metrics: MetricMappingLimits,
    pub stages: StageStorageLimits,
    pub preparation: SupportStorageLimits,
    pub inputs: SupportInputStorageLimits,
    pub receiving: SupportReceivingStorageLimits,
    pub outputs: SupportOutputStorageLimits,
}
impl OwnedReleaseEvaluationLimits {
    pub fn validate(self) -> Result<(), OwnedReleaseEvaluationError> {
        self.metrics.validate()?;
        self.stages.validate()?;
        self.preparation.validate()?;
        self.inputs.validate()?;
        self.receiving.validate()?;
        self.outputs.validate()?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OwnedReleaseEvaluationError {
    #[error(transparent)]
    Metrics(#[from] MetricMappingError),
    #[error(transparent)]
    Stages(#[from] StageStorageError),
    #[error(transparent)]
    Preparation(#[from] SupportStorageError),
    #[error(transparent)]
    Inputs(#[from] SupportInputStorageError),
    #[error(transparent)]
    Receiving(#[from] SupportReceivingStorageError),
    #[error(transparent)]
    Outputs(#[from] SupportOutputStorageError),
}

/// Borrowed typed packages are reusable without another filesystem read. Their
/// private constructors already checked every exact package identity.
pub struct StagedOwnedReleaseEvaluation {
    input: OwnedReleaseEvaluationInput,
    metrics: OwnedMetricMapping,
    support: Option<StagedOwnedReleaseSupport>,
}
pub struct StagedOwnedReleaseSupport {
    stages: OwnedEvaluationStages,
    preparation: OwnedSupportPreparation,
    inputs: OwnedSupportInputBindings,
    receiving: OwnedSupportReceiving,
    outputs: Option<OwnedSupportOutputBindings>,
}
impl StagedOwnedReleaseEvaluation {
    pub fn input(&self) -> &OwnedReleaseEvaluationInput {
        &self.input
    }
    pub fn metrics(&self) -> &OwnedMetricMapping {
        &self.metrics
    }
    pub fn support(&self) -> Option<&StagedOwnedReleaseSupport> {
        self.support.as_ref()
    }
    pub fn receipt(&self) -> OwnedReleaseEvaluationReceipt {
        OwnedReleaseEvaluationReceipt {
            metrics: *self.metrics.identity(),
            support: self
                .support
                .as_ref()
                .map(|support| OwnedReleaseSupportReceipt {
                    stages: *support.stages.identity(),
                    preparation: *support.preparation.identity(),
                    inputs: *support.inputs.identity(),
                    receiving: *support.receiving.identity(),
                    outputs: support.outputs.as_ref().map(|p| *p.identity()),
                }),
        }
    }
    pub(crate) fn assemble(
        input: OwnedReleaseEvaluationInput,
        recipe: &StagedOwnedRecipe,
        limits: OwnedReleaseEvaluationLimits,
    ) -> Result<Self, OwnedReleaseEvaluationError> {
        limits.validate()?;
        let metrics = OwnedMetricMapping::new(input.metrics, recipe.schema(), limits.metrics)?;
        let support = input
            .support
            .map(|s| {
                let stages = OwnedEvaluationStages::new(
                    s.stages,
                    recipe.schema(),
                    recipe.rules(),
                    recipe.routing(),
                    limits.stages,
                )?;
                let preparation = OwnedSupportPreparation::new(
                    s.preparation,
                    recipe.schema(),
                    recipe.rules(),
                    limits.preparation,
                )?;
                let inputs = OwnedSupportInputBindings::new(
                    s.inputs,
                    recipe.schema(),
                    recipe.rules(),
                    &preparation,
                    &stages,
                    limits.inputs,
                )?;
                let receiving = OwnedSupportReceiving::new(
                    s.receiving,
                    recipe.schema(),
                    recipe.rules(),
                    &preparation,
                    &inputs,
                    &stages,
                    limits.receiving,
                )?;
                let outputs = s
                    .outputs
                    .map(|o| {
                        OwnedSupportOutputBindings::new(
                            o,
                            &SupportOutputDependencies {
                                definitions: recipe.schema(),
                                rules: recipe.rules(),
                                preparation: &preparation,
                                inputs: &inputs,
                                receiving: &receiving,
                                stages: &stages,
                            },
                            limits.outputs,
                        )
                    })
                    .transpose()?;
                Ok::<_, OwnedReleaseEvaluationError>(StagedOwnedReleaseSupport {
                    stages,
                    preparation,
                    inputs,
                    receiving,
                    outputs,
                })
            })
            .transpose()?;
        // Canonicalize only after all supplied dependency identities validated.
        let input = OwnedReleaseEvaluationInput {
            metrics: metrics.input().clone(),
            support: support.as_ref().map(StagedOwnedReleaseSupport::input),
        };
        Ok(Self {
            input,
            metrics,
            support,
        })
    }
}
impl StagedOwnedReleaseSupport {
    pub fn stages(&self) -> &OwnedEvaluationStages {
        &self.stages
    }
    pub fn preparation(&self) -> &OwnedSupportPreparation {
        &self.preparation
    }
    pub fn inputs(&self) -> &OwnedSupportInputBindings {
        &self.inputs
    }
    pub fn receiving(&self) -> &OwnedSupportReceiving {
        &self.receiving
    }
    pub fn outputs(&self) -> Option<&OwnedSupportOutputBindings> {
        self.outputs.as_ref()
    }
    fn input(&self) -> OwnedReleaseSupportInput {
        OwnedReleaseSupportInput {
            stages: self.stages.input().clone(),
            preparation: self.preparation.input().clone(),
            inputs: self.inputs.input().clone(),
            receiving: self.receiving.input().clone(),
            outputs: self.outputs.as_ref().map(|p| p.input().clone()),
        }
    }
}

/// Charge nested membership and semantic nodes before constituent cloning. The
/// shared counter includes the ordinary release; no per-bundle budget reset.
pub(crate) fn preflight(
    input: &OwnedReleaseEvaluationInput,
    limits: OwnedReleaseEvaluationLimits,
    left: &mut usize,
) -> Result<(), crate::owned_release::OwnedReleaseError> {
    use crate::owned_release::OwnedReleaseError;
    fn charge(left: &mut usize, n: usize) -> Result<(), OwnedReleaseError> {
        *left = left
            .checked_sub(n)
            .ok_or(OwnedReleaseError::Limit("validation entries"))?;
        Ok(())
    }
    fn closure(left: &mut usize, value: &SchemaClosure) -> Result<(), OwnedReleaseError> {
        if let SchemaClosure::Partial { gaps } = value {
            charge(left, gaps.len())?;
        }
        Ok(())
    }
    fn predicate(
        left: &mut usize,
        value: &SupportTypePredicate,
        depth: usize,
        maximum: usize,
    ) -> Result<(), OwnedReleaseError> {
        // Bounded recursion avoids an unbounded traversal or stack before the
        // component constructor's own predicate validation and serialization.
        charge(left, 1)?;
        if depth > maximum {
            return Err(OwnedReleaseError::Limit("support predicate depth"));
        }
        match value {
            SupportTypePredicate::Type(_) => {}
            SupportTypePredicate::Not(value) => predicate(left, value, depth + 1, maximum)?,
            SupportTypePredicate::Any(values) | SupportTypePredicate::All(values) => {
                // Precharge the frontier before iterating, including empty nodes.
                charge(left, values.len())?;
                for value in values {
                    predicate(left, value, depth + 1, maximum)?;
                }
            }
        }
        Ok(())
    }
    charge(left, 1)?;
    charge(left, input.metrics.bindings.len())?;
    let Some(s) = &input.support else {
        return Ok(());
    };
    charge(left, 1)?;
    for count in [
        s.stages.stages.len(),
        s.stages.programs.members.len(),
        s.stages.frozen_channels.len(),
    ] {
        charge(left, count)?;
    }
    closure(left, &s.stages.programs.closure)?;
    for stage in &s.stages.stages {
        charge(left, stage.predecessors.len())?;
    }
    for count in [
        s.preparation.types.len(),
        s.preparation.effects.len(),
        s.preparation.families.len(),
        s.preparation.supports.len(),
    ] {
        charge(left, count)?;
    }
    for support in &s.preparation.supports {
        match &support.preparation {
            SchemaState::Unmapped { gaps } => charge(left, gaps.len())?,
            SchemaState::Known(p) => {
                charge(left, p.families.as_ref().map_or(0, Vec::len))?;
                charge(left, p.added_types.len())?;
                for p in [&p.requires, &p.excludes].into_iter().flatten() {
                    predicate(left, p, 1, limits.preparation.max_predicate_depth)?;
                }
            }
        }
    }
    charge(left, 9)?;
    for rows in [
        &s.inputs.target.skill_types,
        &s.inputs.target.minion_types.members,
        &s.inputs.target.summoner.skill_types,
        &s.inputs.target.summoner.minion_types.members,
    ] {
        charge(left, rows.len())?;
    }
    for count in [
        s.receiving.roles.len(),
        s.receiving.targets.len(),
        s.receiving.supports.len(),
    ] {
        charge(left, count)?;
    }
    for target in &s.receiving.targets {
        charge(left, target.roles.members.len())?;
        closure(left, &target.roles.closure)?;
        for role in &target.roles.members {
            charge(left, role.endpoints.members.len())?;
            closure(left, &role.endpoints.closure)?;
            for endpoint in &role.endpoints.members {
                charge(left, endpoint.path().len())?;
                if endpoint.path().len() > limits.receiving.max_path_depth {
                    return Err(OwnedReleaseError::Limit("support receiver path depth"));
                }
                if let SupportAdmissionContext::ReceivingSkill {
                    summoner_path: Some(path),
                } = endpoint.admission()
                {
                    charge(left, path.len())?;
                    if path.len() > limits.receiving.max_path_depth {
                        return Err(OwnedReleaseError::Limit("support summoner path depth"));
                    }
                }
            }
        }
    }
    for support in &s.receiving.supports {
        charge(left, support.receivers.members.len())?;
        closure(left, &support.receivers.closure)?;
        for role in &support.receivers.members {
            charge(left, role.delivery.len())?;
        }
    }
    if let Some(outputs) = &s.outputs {
        charge(left, outputs.final_skill_types.len())?;
    }
    Ok(())
}
