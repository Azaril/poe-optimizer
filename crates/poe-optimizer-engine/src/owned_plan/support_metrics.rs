//! Measurements collected inside the same sealed retained-support attempt.
use super::metrics::MetricProjection;
use super::support_effects::SupportAttempt;
use super::*;
use crate::owned_supports::SupportPreparationGap;
use poe_optimizer_data::owned_metrics::OwnedMetricMapping;

/// Preparation diagnostics accompany every requested measurement. An evaluated
/// attempt can still contain unavailable metrics; it does not certify parity.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SupportMetricStatus {
    Evaluated,
    Unavailable {
        cause: EffectValue,
        input: Option<Box<PlanValueKey>>,
    },
    PreparationUnresolved {
        target: SkillTarget,
        reason: SupportPreparationGap,
        origin_index: Option<usize>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OwnedSupportMetricReport {
    pub evaluation: OwnedMetricReport,
    pub support: SupportMetricStatus,
}

/// Immutable query projection over the shared native support executor. Workers
/// own scratch; no effect report is materialized or accepted as execution proof.
pub struct OwnedSupportMetricPlan<I> {
    effects: Arc<OwnedSupportEffectPlan<I>>,
    projection: MetricProjection,
    bindings: MetricPlanIdentity,
    identity: OwnedContentDigest,
}

impl<I: DefinitionSchemaIndex> OwnedSupportMetricPlan<I> {
    pub fn compile(
        effects: Arc<OwnedSupportEffectPlan<I>>,
        mapping: Arc<OwnedMetricMapping>,
    ) -> Result<Self> {
        let base = effects.base_plan();
        if base.request().queries().input().requests.len() != base.query_gates.len() {
            return Err(PlanError::Invalid(
                "query activation bindings differ".into(),
            ));
        }
        let projection = MetricProjection::compile(
            Arc::clone(&base.request),
            effects.definitions(),
            mapping,
            base.limits,
        )?;
        let bindings = MetricPlanIdentity {
            effects: effects.identity(),
            mapping: *projection.mapping().identity(),
        };
        let identity = digest_owned(
            "owned-metric-plan-v1",
            &bindings,
            base.limits.max_wire_bytes,
        )?;
        Ok(Self {
            effects,
            projection,
            bindings,
            identity,
        })
    }

    pub fn identity(&self) -> OwnedContentDigest {
        self.identity
    }
    pub fn bindings(&self) -> &MetricPlanIdentity {
        &self.bindings
    }
    pub fn effect_plan(&self) -> &OwnedSupportEffectPlan<I> {
        &self.effects
    }
    pub fn mapping(&self) -> &OwnedMetricMapping {
        self.projection.mapping()
    }
    pub fn definitions(&self) -> &I {
        self.effects.definitions()
    }
    pub fn request(&self) -> &OwnedEvaluationRequest {
        self.effects.request()
    }
    pub fn binding_report(&self) -> &DefinitionBindingReport {
        self.effects.binding_report()
    }
    pub fn new_scratch(&self) -> OwnedPlanScratch {
        self.effects.new_scratch()
    }

    pub fn evaluate(&self, scratch: &mut OwnedPlanScratch) -> Result<OwnedSupportMetricReport> {
        let mut work = self.effects.base_plan().limits.max_work;
        self.evaluate_with_budget(scratch, &mut work)
    }

    pub fn evaluate_with_budget(
        &self,
        scratch: &mut OwnedPlanScratch,
        work: &mut usize,
    ) -> Result<OwnedSupportMetricReport> {
        let base = self.effects.base_plan();
        let outcome = self
            .effects
            .evaluate_projected(scratch, work, |suffix, scratch, work| {
                let rows = self
                    .projection
                    .bind_values(&suffix.values, base.complete, work)?;
                self.projection.collect(
                    self.identity,
                    &base.gaps,
                    &suffix.query_gates,
                    &rows,
                    scratch,
                    work,
                )
            })?;
        let (evaluation, support) = match outcome {
            SupportAttempt::Evaluated(evaluation) => (evaluation, SupportMetricStatus::Evaluated),
            SupportAttempt::Unavailable { cause, input } => (
                self.projection
                    .unavailable(self.identity, &base.gaps, &cause, work)?,
                SupportMetricStatus::Unavailable { cause, input },
            ),
            SupportAttempt::PreparationUnresolved {
                target,
                reason,
                origin_index,
            } => (
                self.projection.unavailable(
                    self.identity,
                    &base.gaps,
                    &EffectValue::unresolved(PlanGapReason::UpstreamUnavailable),
                    work,
                )?,
                SupportMetricStatus::PreparationUnresolved {
                    target,
                    reason,
                    origin_index,
                },
            ),
        };
        Ok(OwnedSupportMetricReport {
            evaluation,
            support,
        })
    }
}
