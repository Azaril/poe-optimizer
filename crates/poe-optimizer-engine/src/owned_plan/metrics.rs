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
    value: ReadBinding,
}
/// Immutable plan shared across workers. This API requires the same complete
/// owned request and global contributor closure as OwnedEffectPlan. It does not
/// finalize partial drafts, certify legality or establish agreement with an oracle.
pub struct OwnedMetricPlan<I> {
    effects: Arc<OwnedEffectPlan<I>>,
    mapping: Arc<OwnedMetricMapping>,
    bindings: MetricPlanIdentity,
    identity: OwnedContentDigest,
    rows: Vec<Option<BoundMetric>>,
}
impl<I: DefinitionSchemaIndex> OwnedMetricPlan<I> {
    pub fn compile(
        effects: Arc<OwnedEffectPlan<I>>,
        mapping: Arc<OwnedMetricMapping>,
    ) -> Result<Self> {
        mapping
            .verify_bindings(effects.definitions())
            .map_err(|e| PlanError::Invalid(e.to_string()))?;
        let requests = &effects.request().queries().input().requests;
        if requests.len() != effects.query_gates.len() {
            return Err(PlanError::Invalid(
                "query activation bindings differ".into(),
            ));
        }
        let mut work = effects.limits.max_work;
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
            let SchemaLookup::Known(schema) = effects.definitions().definition(stat) else {
                return Err(PlanError::Invalid("mapped stat schema changed".into()));
            };
            let ComputedValueType::Quantity { unit } = &schema.value else {
                return Err(PlanError::Invalid("mapped stat is not a quantity".into()));
            };
            let value = ReadBinding::Final {
                effect: effects
                    .values
                    .get(&PlanValueKey::Stat {
                        entity,
                        stat: stat.clone(),
                    })
                    .copied(),
                complete: effects.complete,
            };
            rows.push(Some(BoundMetric {
                stat: stat.clone(),
                unit: unit.clone(),
                value,
            }));
        }
        let bindings = MetricPlanIdentity {
            effects: effects.identity(),
            mapping: *mapping.identity(),
        };
        let identity = digest_owned(
            "owned-metric-plan-v1",
            &bindings,
            effects.limits.max_wire_bytes,
        )?;
        Ok(Self {
            effects,
            mapping,
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
        &self.mapping
    }
    pub fn new_scratch(&self) -> OwnedPlanScratch {
        self.effects.new_scratch()
    }
    pub(super) fn collect(
        &self,
        scratch: &OwnedPlanScratch,
        work: &mut usize,
    ) -> Result<OwnedMetricReport> {
        charge(work, self.rows.len() + self.effects.gaps.len())?;
        let requests = &self.effects.request().queries().input().requests;
        let mut results = Vec::with_capacity(self.rows.len());
        for (index, row) in self.rows.iter().enumerate() {
            let request = &requests[index];
            let value = if let Some(blocked) = graph::gate_result(
                &self.effects.query_gates[index],
                &scratch.values,
                request.metric.key(),
                work,
            )? {
                blocked
            } else if let Some(row) = row {
                graph::read(&row.value, &scratch.values, request.metric.key(), work)?
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
            identity: self.identity,
            gaps: self.effects.gaps.clone(),
            results,
        })
    }
    pub fn evaluate(&self, scratch: &mut OwnedPlanScratch) -> Result<OwnedMetricReport> {
        let mut work = graph::execute(&self.effects, scratch)?;
        let result = self.collect(scratch, &mut work);
        if result.is_err() {
            scratch.values.clear();
            scratch.facts.clear();
        }
        result
    }
}
