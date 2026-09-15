//! Native occurrence binding and effect dependency execution over owned inputs.
//!
//! Import formats and reference engines do not participate. This layer exposes
//! semantic values and coverage; mapping them to requested metrics is separate.
use crate::owned_rules::{
    CompiledRulePackage, EffectDisposition, NumericalFailure, PreparedRuleProgram, RuleError,
    RuleScratch,
};
use poe_optimizer_core::{
    build_identity::*, data::DataIdentity, owned_binding::*, owned_build::*, owned_content::*,
    owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::owned_routing::OwnedActionRouting;
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    sync::Arc,
};
mod compile;
mod graph;
mod metrics;
pub use metrics::{MetricPlanIdentity, OwnedMetricPlan, OwnedMetricReport, OwnedMetricResult};

#[derive(Clone, Copy, Debug)]
pub struct PlanLimits {
    pub binding: BindingLimits,
    pub max_providers: usize,
    pub max_invocations: usize,
    pub max_effects: usize,
    pub max_edges: usize,
    pub max_work: usize,
    pub max_wire_bytes: usize,
}
impl Default for PlanLimits {
    fn default() -> Self {
        Self {
            binding: BindingLimits::default(),
            max_providers: 4096,
            max_invocations: 16384,
            max_effects: 65536,
            max_edges: 1048576,
            max_work: 16777216,
            max_wire_bytes: 16 * 1024 * 1024,
        }
    }
}
impl PlanLimits {
    fn validate(self) -> Result<()> {
        let hard = Self::default();
        for (n, v, max) in [
            ("providers", self.max_providers, hard.max_providers),
            ("invocations", self.max_invocations, hard.max_invocations),
            ("effects", self.max_effects, hard.max_effects),
            ("edges", self.max_edges, hard.max_edges),
            ("work", self.max_work, hard.max_work),
            ("wire", self.max_wire_bytes, hard.max_wire_bytes),
        ] {
            if v == 0 || v > max {
                return Err(PlanError::Invalid(format!("invalid {n} limit")));
            }
        }
        Ok(())
    }
}
#[derive(Debug)]
pub enum PlanError {
    Binding(BindingError),
    Rule(RuleError),
    Invalid(String),
    Limit(&'static str),
    Digest(ContentDigestError),
}
impl fmt::Display for PlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Binding(e) => e.fmt(f),
            Self::Rule(e) => e.fmt(f),
            Self::Invalid(s) => f.write_str(s),
            Self::Limit(s) => write!(f, "owned plan exceeds {s}"),
            Self::Digest(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for PlanError {}
impl From<BindingError> for PlanError {
    fn from(v: BindingError) -> Self {
        Self::Binding(v)
    }
}
impl From<RuleError> for PlanError {
    fn from(v: RuleError) -> Self {
        Self::Rule(v)
    }
}
impl From<ContentDigestError> for PlanError {
    fn from(v: ContentDigestError) -> Self {
        Self::Digest(v)
    }
}
pub type Result<T> = std::result::Result<T, PlanError>;
fn charge(work: &mut usize, n: usize) -> Result<()> {
    *work = work.checked_sub(n).ok_or(PlanError::Limit("work"))?;
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ConcreteEntity {
    Actor(ActorKey),
    Action(Box<ActionSelection>),
    EquipmentUse(ItemSlotUseId),
    Enemy,
    Environment,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuleOrigin {
    Provider {
        provider: ProviderKey,
    },
    Encounter,
    Receiver {
        receiver: OwnedDefinitionKey,
        actor: ActorKey,
    },
    Usage {
        index: usize,
    },
    Route {
        action: Box<ActionSelection>,
        route: OwnedDefinitionKey,
    },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProgramOccurrenceKey {
    pub origin: RuleOrigin,
    pub owner: SchemaSubject,
    pub program: OwnedDefinitionKey,
    pub entity: ConcreteEntity,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EffectOccurrenceKey {
    pub invocation: ProgramOccurrenceKey,
    pub effect: OwnedDefinitionKey,
}
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlanValueKey {
    Stat {
        entity: ConcreteEntity,
        stat: StatDefId,
    },
    Capability {
        entity: ConcreteEntity,
        capability: CapabilityDefId,
    },
    Grant {
        provider: ProviderKey,
        slot: DeclaredSlot<GrantSlotDefId>,
    },
    SkillParameter {
        skill: Box<GeneratedSkillKey>,
        parameter: DeclaredSlot<ParameterSlotDefId>,
    },
}
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
pub struct ContributionKey {
    pub entity: ConcreteEntity,
    pub stat: StatDefId,
    pub kind: ContributionKind,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BoundEffectTarget {
    Value { key: PlanValueKey },
    Contribution { key: ContributionKey },
    Requirement { code: OwnedDefinitionKey },
    Applicability,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PlanGap {
    pub provider: Option<ProviderKey>,
    pub subject: Option<SchemaSubject>,
    pub reason: PlanGapReason,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanGapReason {
    SchemaUnresolved,
    MissingPrograms,
    PartialPrograms,
    PartialReceivers,
    PartialDeclarations,
    UnresolvedTopology,
    UnsupportedContext,
    UnsupportedRelation,
    MissingRouting,
    PartialRouting,
    MissingInput,
    MissingProducer,
    MissingMetricBinding,
    IncompleteContributors,
    UpstreamUnavailable,
    UnresolvedActivation,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum EffectValue {
    Known {
        value: ParameterValue,
    },
    Inactive,
    Unresolved {
        reason: PlanGapReason,
        read: Option<OwnedDefinitionKey>,
    },
    UnsupportedValue {
        value: ParameterValue,
    },
    /// A demanded finite lookup key outside the package table's reviewed
    /// domain. Preserve its cause through downstream readiness and activation.
    UnsupportedDomain {
        node: OwnedDefinitionKey,
        table: OwnedDefinitionKey,
        key: BoundedInteger,
        minimum: BoundedInteger,
        maximum: BoundedInteger,
    },
    NumericalError {
        node: OwnedDefinitionKey,
        reason: NumericalFailure,
    },
}
impl EffectValue {
    fn unresolved(reason: PlanGapReason) -> Self {
        Self::Unresolved { reason, read: None }
    }
    fn value(&self) -> Option<&ParameterValue> {
        if let Self::Known { value } = self {
            Some(value)
        } else {
            None
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BoundEffectResult {
    pub key: EffectOccurrenceKey,
    pub target: BoundEffectTarget,
    pub value: EffectValue,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ResolvedPlanValue {
    pub key: PlanValueKey,
    pub value: EffectValue,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OwnedEffectsReport {
    pub identity: OwnedContentDigest,
    pub gaps: Vec<PlanGap>,
    pub effects: Vec<BoundEffectResult>,
    pub values: Vec<ResolvedPlanValue>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PlanIdentity {
    pub request: OwnedContentDigest,
    pub definitions: DataIdentity,
    pub rules: OwnedContentDigest,
    pub routing: OwnedContentDigest,
}

#[derive(Clone, Debug)]
enum ReadBinding {
    Constant(Option<ParameterValue>),
    Present {
        source: Box<ReadBinding>,
    },
    Final {
        effect: Option<usize>,
        complete: bool,
    },
    Reduction {
        effects: Vec<usize>,
        reduction: ContributionReduction,
        empty: ParameterValue,
        complete: bool,
    },
    Missing(PlanGapReason),
}
struct Invocation {
    key: ProgramOccurrenceKey,
    program: PreparedRuleProgram,
    reads: Vec<ReadBinding>,
    read_ids: Vec<OwnedDefinitionKey>,
}
enum EffectOperation {
    Program { invocation: usize, effect: usize },
    Route { source: ReadBinding },
}
struct EffectNode {
    key: EffectOccurrenceKey,
    target: BoundEffectTarget,
    operation: EffectOperation,
    gates: Vec<ReadBinding>,
    dependencies: Vec<usize>,
}
/// Immutable owned request and compact dependency indices; workers own their scratch.
/// This is effect resolution, not a complete game metric evaluator.
pub struct OwnedEffectPlan<I> {
    request: Arc<OwnedEvaluationRequest>,
    definitions: Arc<I>,
    rules: Arc<CompiledRulePackage>,
    routing: Arc<OwnedActionRouting>,
    identity: OwnedContentDigest,
    bindings: PlanIdentity,
    limits: PlanLimits,
    gaps: Vec<PlanGap>,
    complete: bool,
    invocations: Vec<Invocation>,
    effects: Vec<EffectNode>,
    order: Vec<usize>,
    values: BTreeMap<PlanValueKey, usize>,
    // Ordered exactly like the immutable request queries. Diagnostic projections
    // alone do not establish activation of an actor or action.
    query_gates: Vec<Vec<ReadBinding>>,
}
impl<I: DefinitionSchemaIndex> OwnedEffectPlan<I> {
    pub fn compile(
        request: Arc<OwnedEvaluationRequest>,
        definitions: Arc<I>,
        rules: Arc<CompiledRulePackage>,
        routing: Arc<OwnedActionRouting>,
        limits: PlanLimits,
    ) -> Result<Self> {
        compile::compile(request, definitions, rules, routing, limits)
    }
    pub fn identity(&self) -> OwnedContentDigest {
        self.identity
    }
    pub fn bindings(&self) -> &PlanIdentity {
        &self.bindings
    }
    pub fn definitions(&self) -> &I {
        &self.definitions
    }
    pub fn routing(&self) -> &OwnedActionRouting {
        &self.routing
    }
    pub fn gaps(&self) -> &[PlanGap] {
        &self.gaps
    }
    pub fn request(&self) -> &OwnedEvaluationRequest {
        &self.request
    }
    pub fn new_scratch(&self) -> OwnedPlanScratch {
        OwnedPlanScratch {
            rule: self.rules.new_scratch(),
            values: Vec::with_capacity(self.effects.len()),
            facts: Vec::new(),
        }
    }
    pub fn evaluate(&self, scratch: &mut OwnedPlanScratch) -> Result<OwnedEffectsReport> {
        graph::evaluate(self, scratch)
    }
}
#[derive(Default)]
pub struct OwnedPlanScratch {
    rule: RuleScratch,
    values: Vec<Option<EffectValue>>,
    facts: Vec<Option<ParameterValue>>,
}
