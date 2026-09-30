//! Requested native measurements over private, occurrence-bound effect plans.
//! Mapping is semantic data, not a source output field or an authorization to
//! publish intermediate effects. No caller-supplied result can bypass execution.
use super::*;
use poe_optimizer_core::owned_metrics::MetricBindingRole;
use poe_optimizer_data::owned_metrics::OwnedMetricMapping;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MetricPlanIdentity {
    pub effects: OwnedContentDigest,
    pub mapping: OwnedContentDigest,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OwnedMetricResult {
    /// Exact identity, target occurrence and order from the request.
    pub request: MetricRequest,
    pub stat: Option<StatDefId>,
    /// Known values are quantities in the metric's exact declared unit.
    pub value: EffectValue,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OwnedMetricReport {
    pub identity: OwnedContentDigest,
    pub gaps: Vec<PlanGap>,
    pub results: Vec<OwnedMetricResult>,
}
struct BoundMetric {
    stat: StatDefId,
    unit: UnitDefId,
    key: PlanValueKey,
}

/// Cold query projection shared by ordinary and retained-support graph execution.
/// Its rows name values; they never certify graph completeness or accept reports.
pub(super) struct MetricProjection {
    request: Arc<OwnedEvaluationRequest>,
    mapping: Arc<OwnedMetricMapping>,
    rows: Vec<Option<BoundMetric>>,
}
/// Bound only to the private value arena supplied by an executing graph driver.
pub(super) struct MetricRows {
    values: Vec<Option<ReadBinding>>,
}
impl MetricProjection {
    pub(super) fn compile<I: DefinitionSchemaIndex>(
        request: Arc<OwnedEvaluationRequest>,
        definitions: &I,
        mapping: Arc<OwnedMetricMapping>,
        limits: PlanLimits,
    ) -> Result<Self> {
        limits.validate()?;
        mapping
            .verify_bindings(definitions)
            .map_err(|e| PlanError::Invalid(e.to_string()))?;
        let requests = &request.queries().input().requests;
        let mut work = limits.max_work;
        charge(&mut work, requests.len())?;
        let mut rows = Vec::with_capacity(requests.len());
        for request in requests {
            let (role, entity) = match &request.target {
                MetricTarget::Actor(ActorKey::Player) => (
                    MetricBindingRole::PlayerActor,
                    ConcreteEntity::Actor(ActorKey::Player),
                ),
                MetricTarget::Actor(actor) => (
                    MetricBindingRole::OwnedActor,
                    ConcreteEntity::Actor(actor.clone()),
                ),
                MetricTarget::Action(action) => (
                    MetricBindingRole::Action,
                    ConcreteEntity::Action(action.clone()),
                ),
            };
            let Some(stat) = mapping.stat_for(&request.metric, role) else {
                rows.push(None);
                continue;
            };
            let SchemaLookup::Known(schema) = definitions.definition(stat) else {
                return Err(PlanError::Invalid("mapped stat schema changed".into()));
            };
            let ComputedValueType::Quantity { unit } = &schema.value else {
                return Err(PlanError::Invalid("mapped stat is not a quantity".into()));
            };
            rows.push(Some(BoundMetric {
                stat: stat.clone(),
                unit: unit.clone(),
                key: PlanValueKey::Stat {
                    entity,
                    stat: stat.clone(),
                },
            }));
        }
        Ok(Self {
            request,
            mapping,
            rows,
        })
    }
    pub(super) fn mapping(&self) -> &OwnedMetricMapping {
        &self.mapping
    }
    pub(super) fn bind_values(
        &self,
        values: &BTreeMap<PlanValueKey, usize>,
        complete: bool,
        work: &mut usize,
    ) -> Result<MetricRows> {
        charge(work, self.rows.len())?;
        let values = self
            .rows
            .iter()
            .map(|row| {
                row.as_ref().map(|row| ReadBinding::Final {
                    effect: values.get(&row.key).copied(),
                    complete,
                })
            })
            .collect();
        Ok(MetricRows { values })
    }
    pub(super) fn collect(
        &self,
        identity: OwnedContentDigest,
        gaps: &[PlanGap],
        query_gates: &[Vec<ReadBinding>],
        values: &MetricRows,
        scratch: &OwnedPlanScratch,
        work: &mut usize,
    ) -> Result<OwnedMetricReport> {
        if query_gates.len() != self.rows.len() || values.values.len() != self.rows.len() {
            return Err(PlanError::Invalid("metric query bindings differ".into()));
        }
        charge(work, self.rows.len() + gaps.len())?;
        let requests = &self.request.queries().input().requests;
        let mut results = Vec::with_capacity(self.rows.len());
        for (index, row) in self.rows.iter().enumerate() {
            let request = &requests[index];
            let value = if let Some(blocked) = graph::gate_result(
                &query_gates[index],
                &scratch.values,
                request.metric.key(),
                work,
            )? {
                blocked
            } else if let Some(binding) = &values.values[index] {
                graph::read(binding, &scratch.values, request.metric.key(), work)?
            } else {
                EffectValue::unresolved(PlanGapReason::MissingMetricBinding)
            };
            if let EffectValue::Known { value } = &value {
                match (row, value) {
                    (Some(row), ParameterValue::Quantity(quantity))
                        if quantity.unit() == &row.unit => {}
                    _ => {
                        return Err(PlanError::Invalid(
                            "metric value violates its bound type/unit".into(),
                        ));
                    }
                }
            }
            results.push(OwnedMetricResult {
                request: request.clone(),
                stat: row.as_ref().map(|row| row.stat.clone()),
                value,
            });
        }
        Ok(OwnedMetricReport {
            identity,
            gaps: gaps.to_vec(),
            results,
        })
    }

    /// Global preparation failure has no final arena to bind. Keep every query,
    /// including absent mappings, while reporting its exact common failure cause.
    pub(super) fn unavailable(
        &self,
        identity: OwnedContentDigest,
        gaps: &[PlanGap],
        cause: &EffectValue,
        work: &mut usize,
    ) -> Result<OwnedMetricReport> {
        if matches!(cause, EffectValue::Known { .. }) {
            return Err(PlanError::Invalid(
                "unavailable metric cause is a known value".into(),
            ));
        }
        charge(work, self.rows.len() + gaps.len())?;
        let results = self
            .request
            .queries()
            .input()
            .requests
            .iter()
            .zip(&self.rows)
            .map(|(request, row)| OwnedMetricResult {
                request: request.clone(),
                stat: row.as_ref().map(|row| row.stat.clone()),
                value: cause.clone(),
            })
            .collect();
        Ok(OwnedMetricReport {
            identity,
            gaps: gaps.to_vec(),
            results,
        })
    }
}

/// Immutable plan shared across workers. This API requires the same complete
/// owned request and global contributor closure as OwnedEffectPlan. It does not
/// finalize partial drafts, certify legality or establish agreement with an oracle.
pub struct OwnedMetricPlan<I> {
    effects: Arc<OwnedEffectPlan<I>>,
    projection: MetricProjection,
    bindings: MetricPlanIdentity,
    identity: OwnedContentDigest,
    rows: MetricRows,
}
impl<I: DefinitionSchemaIndex> OwnedMetricPlan<I> {
    pub fn compile(
        effects: Arc<OwnedEffectPlan<I>>,
        mapping: Arc<OwnedMetricMapping>,
    ) -> Result<Self> {
        if effects.request().queries().input().requests.len() != effects.query_gates.len() {
            return Err(PlanError::Invalid(
                "query activation bindings differ".into(),
            ));
        }
        let projection = MetricProjection::compile(
            Arc::clone(&effects.request),
            effects.definitions(),
            mapping,
            effects.limits,
        )?;
        let mut work = effects.limits.max_work;
        let rows = projection.bind_values(&effects.values, effects.complete, &mut work)?;
        let bindings = MetricPlanIdentity {
            effects: effects.identity(),
            mapping: *projection.mapping().identity(),
        };
        let identity = digest_owned(
            "owned-metric-plan-v1",
            &bindings,
            effects.limits.max_wire_bytes,
        )?;
        Ok(Self {
            effects,
            projection,
            bindings,
            identity,
            rows,
        })
    }
    pub fn identity(&self) -> OwnedContentDigest {
        self.identity
    }
    pub fn bindings(&self) -> &MetricPlanIdentity {
        &self.bindings
    }
    pub fn effect_plan(&self) -> &OwnedEffectPlan<I> {
        &self.effects
    }
    pub fn mapping(&self) -> &OwnedMetricMapping {
        self.projection.mapping()
    }
    pub fn new_scratch(&self) -> OwnedPlanScratch {
        self.effects.new_scratch()
    }
    pub(super) fn collect(
        &self,
        scratch: &OwnedPlanScratch,
        work: &mut usize,
    ) -> Result<OwnedMetricReport> {
        self.projection.collect(
            self.identity,
            &self.effects.gaps,
            &self.effects.query_gates,
            &self.rows,
            scratch,
            work,
        )
    }
    pub fn evaluate(&self, scratch: &mut OwnedPlanScratch) -> Result<OwnedMetricReport> {
        let mut work = graph::execute(&self.effects, scratch)?;
        let result = self.collect(scratch, &mut work);
        if result.is_err() {
            graph::clear_attempt(scratch);
        }
        result
    }
}
