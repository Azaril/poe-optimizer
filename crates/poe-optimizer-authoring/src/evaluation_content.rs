//! Offline declarations, without runtime dependency commitments.
use poe_optimizer_core::owned_content::digest_owned;
use poe_optimizer_data::{
    owned_evaluation_budget::{
        EvaluationBudgetLimits, EvaluationBudgetView, SupportBudgetView, charge_evaluation_entries,
    },
    owned_metrics::{MetricMappingContent, OwnedMetricMapping},
    owned_stages::{EvaluationStagesContent, OwnedEvaluationStages},
    owned_support_inputs::{OwnedSupportInputBindings, SupportInputBindingsContent},
    owned_support_outputs::{
        OwnedSupportOutputBindings, SupportOutputBindingsContent, SupportOutputDependencies,
    },
    owned_support_receiving::{OwnedSupportReceiving, SupportReceivingContent},
    owned_supports::{OwnedSupportPreparation, SupportPreparationContent},
};
use poe_optimizer_import::{
    owned_recipe::CheckedOwnedRecipe,
    owned_release::{OwnedReleaseError, OwnedReleaseLimits},
    owned_release_evaluation::{
        CheckedOwnedReleaseEvaluation, OwnedReleaseEvaluationError, OwnedReleaseEvaluationInput,
        OwnedReleaseEvaluationLimits, OwnedReleaseSupportInput,
    },
};
use serde::{Deserialize, Serialize};

/// Author-supplied evaluation semantics. Dependency identities are derived from
/// checked final packages, never supplied or repaired as part of this format.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedReleaseEvaluationContent {
    pub metrics: MetricMappingContent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support: Option<OwnedReleaseSupportContent>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedReleaseSupportContent {
    pub stages: EvaluationStagesContent,
    pub preparation: SupportPreparationContent,
    pub inputs: SupportInputBindingsContent,
    pub receiving: SupportReceivingContent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outputs: Option<SupportOutputBindingsContent>,
}

impl OwnedReleaseEvaluationContent {
    /// Consume the actual checked owners, never a raw input with inherited seals.
    pub fn from_checked(checked: CheckedOwnedReleaseEvaluation) -> Self {
        let parts = checked.into_parts();
        Self {
            metrics: parts.metrics.into_authored_content(),
            support: parts.support.map(|support| {
                let parts = support.into_parts();
                OwnedReleaseSupportContent {
                    stages: parts.stages.into_authored_content(),
                    preparation: parts.preparation.into_authored_content(),
                    inputs: parts.inputs.into_authored_content(),
                    receiving: parts.receiving.into_authored_content(),
                    outputs: parts
                        .outputs
                        .map(OwnedSupportOutputBindings::into_authored_content),
                }
            }),
        }
    }
}

impl<'a> From<&'a OwnedReleaseEvaluationContent> for EvaluationBudgetView<'a> {
    fn from(input: &'a OwnedReleaseEvaluationContent) -> Self {
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

impl OwnedReleaseEvaluationContent {
    /// Charge the same nested semantic entries as runtime release validation.
    /// Authoring can reserve this cost before checking/cloning its other source
    /// dependencies. The remaining counter must belong to this release budget;
    /// a successful charge establishes no semantic or coverage authority.
    /// An error may leave a partial debit; discard the failed attempt's counter.
    pub fn charge_validation_entries(
        &self,
        limits: OwnedReleaseLimits,
        remaining: &mut usize,
    ) -> Result<(), OwnedReleaseError> {
        limits.validate()?;
        if *remaining > limits.max_validation_entries {
            return Err(OwnedReleaseError::InvalidLimit("validation entries"));
        }
        let budget = EvaluationBudgetLimits::new(
            limits.evaluation.preparation,
            limits.evaluation.receiving,
        )?;
        charge_evaluation_entries(self.into(), budget, remaining).map_err(Into::into)
    }

    /// Construct all dependencies in order. This returns raw canonical inputs;
    /// final ordinary release assembly remains mandatory before publication.
    pub fn into_input(
        self,
        recipe: &CheckedOwnedRecipe,
        limits: OwnedReleaseLimits,
    ) -> Result<OwnedReleaseEvaluationInput, OwnedReleaseError> {
        let mut remaining = limits.max_validation_entries;
        self.charge_validation_entries(limits, &mut remaining)?;
        digest_owned("owned-evaluation-content-v1", &self, limits.max_input_bytes)?;
        self.bind(recipe, limits.evaluation).map_err(Into::into)
    }

    fn bind(
        self,
        recipe: &CheckedOwnedRecipe,
        limits: OwnedReleaseEvaluationLimits,
    ) -> Result<OwnedReleaseEvaluationInput, OwnedReleaseEvaluationError> {
        let metrics =
            OwnedMetricMapping::bind_new(self.metrics, recipe.schema().as_ref(), limits.metrics)?;
        let support = self
            .support
            .map(|s| {
                let stages = OwnedEvaluationStages::bind_new(
                    s.stages,
                    recipe.schema().as_ref(),
                    recipe.rules().as_ref(),
                    recipe.routing().as_ref(),
                    limits.stages,
                )?;
                let preparation = OwnedSupportPreparation::bind_new(
                    s.preparation,
                    recipe.schema().as_ref(),
                    recipe.rules().as_ref(),
                    limits.preparation,
                )?;
                let inputs = OwnedSupportInputBindings::bind_new(
                    s.inputs,
                    recipe.schema().as_ref(),
                    recipe.rules().as_ref(),
                    &preparation,
                    &stages,
                    limits.inputs,
                )?;
                let receiving = OwnedSupportReceiving::bind_new(
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
                        OwnedSupportOutputBindings::bind_new(
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
                Ok::<_, OwnedReleaseEvaluationError>(OwnedReleaseSupportInput {
                    stages: stages.input().clone(),
                    preparation: preparation.input().clone(),
                    inputs: inputs.input().clone(),
                    receiving: receiving.input().clone(),
                    outputs: outputs.map(|o| o.input().clone()),
                })
            })
            .transpose()?;
        Ok(OwnedReleaseEvaluationInput {
            metrics: metrics.input().clone(),
            support,
        })
    }
}
