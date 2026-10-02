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
mod support_effects;
mod support_metrics;
mod support_outputs;
mod supports;
pub use metrics::{MetricPlanIdentity, OwnedMetricPlan, OwnedMetricReport, OwnedMetricResult};
pub use support_effects::{
    OwnedSupportEffectPlan, SupportEffectPlanInputs, SupportEffectsOutcome, SupportEffectsReport,
};
pub use support_metrics::{OwnedSupportMetricPlan, OwnedSupportMetricReport, SupportMetricStatus};
pub use support_outputs::SupportPreparationContextKey;
pub use supports::{
    ComputedSupportOutcome, ComputedSupportReport, OwnedSupportPreparationPlan,
    SupportPreparationPlanInputs,
};

#[derive(Clone, Copy, Debug)]
pub struct PlanLimits {
    pub binding: BindingLimits,
    pub max_providers: usize,
    /// Discovered owners, including owners with empty program collections.
    /// This inventory is bounded independently from actual rule invocations.
    pub max_owner_bindings: usize,
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
            max_owner_bindings: 16384,
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
            (
                "owner bindings",
                self.max_owner_bindings,
                hard.max_owner_bindings,
            ),
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
    match work.checked_sub(n) {
        Some(remaining) => {
            *work = remaining;
            Ok(())
        }
        None => {
            *work = 0;
            Err(PlanError::Limit("work"))
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ConcreteEntity {
    Actor(ActorKey),
    Action(Box<ActionSelection>),
    EquipmentUse(ItemSlotUseId),
    /// One exact modifier on one receiving equipment use. Repeated definitions
    /// and repeated uses of a backing item never share intermediate values.
    Modifier(ProviderKey),
    /// Physical support assignment, independent of any receiving action.
    SupportOrigin(SupportOrigin),
    /// Exact authored or generated skill occurrence, not its actor or output.
    Skill(Box<SkillTarget>),
    Enemy,
    Environment,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuleOrigin {
    EffectApplication {
        application: OwnedDefinitionKey,
        source: ConcreteEntity,
        recipient: ConcreteEntity,
    },
    EffectApplicationGroup {
        family: OwnedDefinitionKey,
        modifier: OwnedDefinitionKey,
        recipient: ConcreteEntity,
    },
    SupportPreparation {
        context: Box<SupportPreparationContextKey>,
    },
    SupportApplication {
        application: Box<SupportApplicationKey>,
    },
    Provider {
        provider: ProviderKey,
    },
    Encounter,
    Receiver {
        receiver: OwnedDefinitionKey,
        actor: ActorKey,
    },
    /// A producer effect projected into one exact sibling modifier occurrence.
    ModifierTransform {
        producer: ProviderKey,
        recipient: ProviderKey,
    },
    EquipmentReceiver {
        receiver: OwnedDefinitionKey,
        equipment_use: ItemSlotUseId,
    },
    Usage {
        index: usize,
    },
    SourceSelection {
        action: Box<ActionSelection>,
        selector: OwnedDefinitionKey,
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
    SupportApplicability {
        application: Box<SupportApplicationKey>,
    },
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
    ApplicationCandidate {
        family: OwnedDefinitionKey,
        modifier: OwnedDefinitionKey,
        key: ContributionKey,
    },
    Value {
        key: PlanValueKey,
    },
    Contribution {
        key: ContributionKey,
    },
    ModifierTransform {
        key: PlanValueKey,
        operation: ModifierTransformOperation,
        order: BoundedInteger,
    },
    Requirement {
        code: OwnedDefinitionKey,
    },
    Applicability,
    SourceSelection {
        action: Box<ActionSelection>,
        selector: OwnedDefinitionKey,
    },
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
    PartialEffectApplications,
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub application_groups: Vec<EffectApplicationGroupResult>,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct EffectApplicationGroupResult {
    pub key: EffectOccurrenceKey,
    pub candidates: Vec<EffectOccurrenceKey>,
    pub co_winners: Vec<EffectOccurrenceKey>,
    pub value: EffectValue,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PlanIdentity {
    pub request: OwnedContentDigest,
    pub definitions: DataIdentity,
    pub rules: OwnedContentDigest,
    pub routing: OwnedContentDigest,
}

#[derive(Clone, Debug, PartialEq)]
enum ReadBinding {
    Constant(Option<ParameterValue>),
    Inactive,
    Select {
        decision: usize,
        when_true: Box<ReadBinding>,
        when_false: Box<ReadBinding>,
    },
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
    ModifierTransforms {
        initial: Box<ReadBinding>,
        steps: Vec<BoundModifierTransform>,
        complete: bool,
    },
    Missing(PlanGapReason),
}
#[derive(Clone, Debug, PartialEq)]
struct BoundModifierTransform {
    effect: usize,
    operation: ModifierTransformOperation,
}
#[derive(Clone)]
struct Invocation {
    key: ProgramOccurrenceKey,
    program: PreparedRuleProgram,
    reads: Vec<ReadBinding>,
    read_ids: Vec<OwnedDefinitionKey>,
}
#[derive(Clone, PartialEq)]
enum EffectOperation {
    ApplicationMaximum {
        candidates: Vec<usize>,
        applications: Vec<OwnedDefinitionKey>,
        complete: bool,
    },
    /// Created only from native preparation inside the current private attempt.
    /// None denotes established inactive preparation, never unknown membership.
    PreparedSupportType {
        stage: OwnedDefinitionKey,
        member: Option<bool>,
    },
    Program {
        invocation: usize,
        effect: usize,
    },
    SupportApplicability {
        invocation: usize,
        effect: usize,
        eligible: bool,
    },
    Route {
        source: ReadBinding,
    },
    SelectSource {
        source: ReadBinding,
    },
}
#[derive(Clone)]
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
    binding_report: DefinitionBindingReport,
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
    preparation_gates: BTreeMap<SkillTarget, Vec<ReadBinding>>,
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
    /// Exact diagnostics from the bounded whole-request binding performed during
    /// compilation. This report is immutable evidence, not evaluation authority.
    pub fn binding_report(&self) -> &DefinitionBindingReport {
        &self.binding_report
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
            attempt_plan: None,
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
    // Private execution slices may only continue the attempt begun for this plan.
    attempt_plan: Option<OwnedContentDigest>,
}
