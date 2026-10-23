//! Checked immutable evaluator artifacts. Validation establishes
//! exact dependencies, not active contributors or numerical build coverage.
use crate::owned_recipe::CheckedOwnedRecipe;
use poe_optimizer_core::{
    owned_content::OwnedContentDigest, owned_metrics::MetricMappingInput,
    owned_stages::EvaluationStagesInput, owned_support_inputs::SupportInputBindingsInput,
    owned_support_outputs::SupportOutputBindingsInput,
    owned_support_receiving::SupportReceivingInput, owned_supports::SupportPreparationInput,
};
use poe_optimizer_data::{
    owned_evaluation_budget::{
        EvaluationBudgetError, EvaluationBudgetLimits, EvaluationBudgetView, SupportBudgetView,
        charge_evaluation_entries,
    },
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
pub struct CheckedOwnedReleaseEvaluation {
    input: OwnedReleaseEvaluationInput,
    metrics: OwnedMetricMapping,
    support: Option<CheckedOwnedReleaseSupport>,
}
pub struct CheckedOwnedReleaseSupport {
    stages: OwnedEvaluationStages,
    preparation: OwnedSupportPreparation,
    inputs: OwnedSupportInputBindings,
    receiving: OwnedSupportReceiving,
    outputs: Option<OwnedSupportOutputBindings>,
}

/// Owned checked constituents, not a certificate for a reassembled release.
pub struct OwnedReleaseEvaluationParts {
    pub metrics: OwnedMetricMapping,
    pub support: Option<CheckedOwnedReleaseSupport>,
}

/// Consuming a support bundle retains each actual checked Data package.
pub struct OwnedReleaseSupportParts {
    pub stages: OwnedEvaluationStages,
    pub preparation: OwnedSupportPreparation,
    pub inputs: OwnedSupportInputBindings,
    pub receiving: OwnedSupportReceiving,
    pub outputs: Option<OwnedSupportOutputBindings>,
}
impl CheckedOwnedReleaseEvaluation {
    pub fn into_parts(self) -> OwnedReleaseEvaluationParts {
        OwnedReleaseEvaluationParts {
            metrics: self.metrics,
            support: self.support,
        }
    }
    pub fn input(&self) -> &OwnedReleaseEvaluationInput {
        &self.input
    }
    pub fn metrics(&self) -> &OwnedMetricMapping {
        &self.metrics
    }
    pub fn support(&self) -> Option<&CheckedOwnedReleaseSupport> {
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
    pub(crate) fn new(
        input: OwnedReleaseEvaluationInput,
        recipe: &CheckedOwnedRecipe,
        limits: OwnedReleaseEvaluationLimits,
    ) -> Result<Self, OwnedReleaseEvaluationError> {
        limits.validate()?;
        let metrics =
            OwnedMetricMapping::new(input.metrics, recipe.schema().as_ref(), limits.metrics)?;
        let support = input
            .support
            .map(|s| {
                let stages = OwnedEvaluationStages::new(
                    s.stages,
                    recipe.schema().as_ref(),
                    recipe.rules().as_ref(),
                    recipe.routing().as_ref(),
                    limits.stages,
                )?;
                let preparation = OwnedSupportPreparation::new(
                    s.preparation,
                    recipe.schema().as_ref(),
                    recipe.rules().as_ref(),
                    limits.preparation,
                )?;
                let inputs = OwnedSupportInputBindings::new(
                    s.inputs,
                    recipe.schema().as_ref(),
                    recipe.rules().as_ref(),
                    &preparation,
                    &stages,
                    limits.inputs,
                )?;
                let receiving = OwnedSupportReceiving::new(
                    s.receiving,
                    recipe.schema().as_ref(),
                    recipe.rules().as_ref(),
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
                                definitions: recipe.schema().as_ref(),
                                rules: recipe.rules().as_ref(),
                                preparation: &preparation,
                                inputs: &inputs,
                                receiving: &receiving,
                                stages: &stages,
                            },
                            limits.outputs,
                        )
                    })
                    .transpose()?;
                Ok::<_, OwnedReleaseEvaluationError>(CheckedOwnedReleaseSupport {
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
            support: support.as_ref().map(CheckedOwnedReleaseSupport::input),
        };
        Ok(Self {
            input,
            metrics,
            support,
        })
    }
}
impl CheckedOwnedReleaseSupport {
    pub fn into_parts(self) -> OwnedReleaseSupportParts {
        OwnedReleaseSupportParts {
            stages: self.stages,
            preparation: self.preparation,
            inputs: self.inputs,
            receiving: self.receiving,
            outputs: self.outputs,
        }
    }
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

impl<'a> From<&'a OwnedReleaseEvaluationInput> for EvaluationBudgetView<'a> {
    fn from(input: &'a OwnedReleaseEvaluationInput) -> Self {
        Self {
            metric_bindings: input.metrics.bindings.len(),
            support: input.support.as_ref().map(|s| SupportBudgetView {
                stages: (&s.stages).into(),
                preparation: (&s.preparation).into(),
                target: &s.inputs.target,
                receiving: (&s.receiving).into(),
                outputs: s.outputs.as_ref().map(|o| o.final_skill_types.len()),
            }),
        }
    }
}

impl From<EvaluationBudgetError> for crate::owned_release::OwnedReleaseError {
    fn from(error: EvaluationBudgetError) -> Self {
        match error {
            EvaluationBudgetError::Limit(name) => Self::Limit(name),
            EvaluationBudgetError::Preparation(error) => {
                Self::Evaluation(OwnedReleaseEvaluationError::Preparation(error))
            }
            EvaluationBudgetError::Receiving(error) => {
                Self::Evaluation(OwnedReleaseEvaluationError::Receiving(error))
            }
        }
    }
}

pub(crate) fn preflight(
    input: &OwnedReleaseEvaluationInput,
    limits: OwnedReleaseEvaluationLimits,
    left: &mut usize,
) -> Result<(), crate::owned_release::OwnedReleaseError> {
    let budget = EvaluationBudgetLimits::new(limits.preparation, limits.receiving)?;
    charge_evaluation_entries(input.into(), budget, left).map_err(Into::into)
}
