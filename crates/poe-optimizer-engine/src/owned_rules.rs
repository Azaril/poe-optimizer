//! Bounded owned expression compilation and component execution.
//!
//! Explicit facts are not build/provider-resolution authority. Every graph node
//! is validated at compilation; only demanded reads are evaluated at runtime.
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::{BoundedInteger, GameVersionNamespace, OwnedDefinitionKey},
    owned_rules::{RuleEffectKind, RulePackageInput},
    owned_schema::{
        ComputedValueType, DefinitionAddress, DefinitionSchemaIndex, SchemaClosure, SchemaSubject,
        SlotAddress, ValueSchema,
    },
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt, sync::Arc};
mod compile;
mod execute;
#[cfg(test)]
mod prepared_tests;

#[derive(Clone, Copy, Debug)]
pub struct RuleLimits {
    pub max_receivers: usize,
    pub max_receiver_targets: usize,
    pub max_gaps: usize,
    pub max_owners: usize,
    pub max_programs: usize,
    pub max_tables: usize,
    pub max_table_cells: usize,
    pub max_reads: usize,
    pub max_nodes: usize,
    pub max_edges: usize,
    pub max_effects: usize,
    pub max_work: usize,
    pub max_wire_bytes: usize,
}
impl Default for RuleLimits {
    fn default() -> Self {
        Self {
            max_receivers: 8192,
            max_receiver_targets: 65536,
            max_gaps: 65536,
            max_owners: 4096,
            max_programs: 8192,
            max_tables: 8192,
            max_table_cells: 262144,
            max_reads: 65536,
            max_nodes: 262144,
            max_edges: 1048576,
            max_effects: 65536,
            max_work: 4194304,
            max_wire_bytes: 8 * 1024 * 1024,
        }
    }
}
impl RuleLimits {
    fn validate(self) -> Result<(), RuleError> {
        let h = Self::default();
        for (name, v, max) in [
            ("receivers", self.max_receivers, h.max_receivers),
            (
                "receiver_targets",
                self.max_receiver_targets,
                h.max_receiver_targets,
            ),
            ("gaps", self.max_gaps, h.max_gaps),
            ("owners", self.max_owners, h.max_owners),
            ("programs", self.max_programs, h.max_programs),
            ("tables", self.max_tables, h.max_tables),
            ("table_cells", self.max_table_cells, h.max_table_cells),
            ("reads", self.max_reads, h.max_reads),
            ("nodes", self.max_nodes, h.max_nodes),
            ("edges", self.max_edges, h.max_edges),
            ("effects", self.max_effects, h.max_effects),
            ("work", self.max_work, h.max_work),
            ("wire_bytes", self.max_wire_bytes, h.max_wire_bytes),
        ] {
            if v == 0 || v > max {
                return Err(RuleError::new(
                    "limits",
                    format!("{name} must be positive and <= {max}"),
                ));
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuleError {
    pub path: String,
    pub message: String,
}
impl RuleError {
    fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}
impl fmt::Display for RuleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}
impl std::error::Error for RuleError {}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleFact {
    pub read: OwnedDefinitionKey,
    pub value: ParameterValue,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ProgramEvaluation {
    pub owner: SchemaSubject,
    pub program: OwnedDefinitionKey,
    pub owner_programs_closure: SchemaClosure,
    pub effects: Vec<EffectEvaluation>,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct EffectEvaluation {
    pub id: OwnedDefinitionKey,
    pub effect: RuleEffectKind,
    pub disposition: EffectDisposition,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EffectDisposition {
    Applied {
        value: ParameterValue,
    },
    /// A finite computed projection outside the explicitly supported target
    /// parameter domain. The value is retained; this is not gameplay illegality,
    /// missing input, an arithmetic error, or permission to clamp.
    UnsupportedValue {
        value: ParameterValue,
    },
    /// A demanded lookup key lies outside the table's explicit supported domain.
    UnsupportedDomain {
        node: OwnedDefinitionKey,
        table: OwnedDefinitionKey,
        key: BoundedInteger,
        minimum: BoundedInteger,
        maximum: BoundedInteger,
    },
    Inactive,
    Unresolved {
        input: OwnedDefinitionKey,
    },
    NumericalError {
        node: OwnedDefinitionKey,
        reason: NumericalFailure,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NumericalFailure {
    DivisionByZero,
    NonFinite,
    IntegerOverflow,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum SubjectKey {
    Definition(DefinitionAddress),
    Slot(SlotAddress),
}
impl From<&SchemaSubject> for SubjectKey {
    fn from(v: &SchemaSubject) -> Self {
        match v {
            SchemaSubject::Definition(v) => Self::Definition(v.clone()),
            SchemaSubject::Slot(v) => Self::Slot(v.clone()),
        }
    }
}
#[derive(Clone, Debug)]
struct CompiledRead {
    id: OwnedDefinitionKey,
    ty: ComputedValueType,
    schema: Option<ValueSchema>,
}
#[derive(Clone, Debug)]
struct CompiledNode {
    id: OwnedDefinitionKey,
    expression: Op,
    ty: ComputedValueType,
}
#[derive(Clone, Debug)]
struct CompiledTiming {
    inputs: [usize; 8],
    precision: u32,
    units: poe_optimizer_core::owned_rules::ReciprocalTimingUnits,
    output: poe_optimizer_core::owned_rules::OrdinaryTimingChannel,
}
#[derive(Clone, Debug)]
enum Op {
    LookupIntegerTable {
        key: usize,
        table: Arc<poe_optimizer_core::owned_rules::IntegerRuleTable>,
    },
    OrdinaryTiming(Box<CompiledTiming>),
    Literal(ParameterValue),
    Read(usize),
    Binary(Binary, usize, usize),
    Ratio(
        usize,
        usize,
        poe_optimizer_core::owned_definitions::UnitDefId,
    ),
    Percent(usize, poe_optimizer_core::owned_definitions::UnitDefId),
    Round(
        usize,
        poe_optimizer_core::owned_definitions::FiniteQuantity,
        poe_optimizer_core::owned_rules::RuleRounding,
    ),
    Compare(
        poe_optimizer_core::owned_rules::RuleComparison,
        usize,
        usize,
    ),
    Not(usize),
    All(Vec<usize>),
    Any(Vec<usize>),
    Select(usize, usize, usize),
}
#[derive(Clone, Copy, Debug)]
enum Binary {
    Add,
    Subtract,
    Minimum,
    Maximum,
    Scale,
    ScaleInteger,
    DivideFactor,
}
impl Op {
    fn dependencies(&self) -> Vec<usize> {
        match self {
            Self::OrdinaryTiming(recipe) => recipe.inputs.to_vec(),
            Self::LookupIntegerTable { key, .. } => vec![*key],
            Self::Literal(_) | Self::Read(_) => vec![],
            Self::Binary(_, a, b) | Self::Ratio(a, b, _) | Self::Compare(_, a, b) => vec![*a, *b],
            Self::Percent(a, _) | Self::Round(a, _, _) | Self::Not(a) => vec![*a],
            Self::All(v) | Self::Any(v) => v.clone(),
            Self::Select(a, b, c) => vec![*a, *b, *c],
        }
    }
}
#[derive(Clone, Debug)]
struct CompiledEffect {
    id: OwnedDefinitionKey,
    kind: RuleEffectKind,
    when: Option<usize>,
    value: usize,
    value_schema: Option<ValueSchema>,
    read_indices: Vec<usize>,
}
#[derive(Clone, Debug)]
struct CompiledProgram {
    owner: SchemaSubject,
    id: OwnedDefinitionKey,
    closure: SchemaClosure,
    reads: Vec<CompiledRead>,
    read_index: BTreeMap<OwnedDefinitionKey, usize>,
    nodes: Vec<CompiledNode>,
    effects: Vec<CompiledEffect>,
}
/// Immutable private-index programs. Execution scratch is owned by each worker.
#[derive(Clone, Debug)]
pub struct CompiledRulePackage {
    input: RulePackageInput,
    identity: OwnedContentDigest,
    programs: BTreeMap<(SubjectKey, OwnedDefinitionKey), Arc<CompiledProgram>>,
    limits: RuleLimits,
}
impl CompiledRulePackage {
    pub fn compile<I: DefinitionSchemaIndex>(
        input: &RulePackageInput,
        definitions: &I,
        limits: RuleLimits,
    ) -> Result<Self, RuleError> {
        compile::compile(input, definitions, limits)
    }
    pub fn input(&self) -> &RulePackageInput {
        &self.input
    }
    pub fn identity(&self) -> OwnedContentDigest {
        self.identity
    }
    /// Resolve IDs once during plan construction. Reads use this program's
    /// canonical compiled order, exposed by PreparedRuleProgram::read_ids.
    pub(crate) fn prepare_program(
        &self,
        owner: &SchemaSubject,
        program: &OwnedDefinitionKey,
    ) -> Result<PreparedRuleProgram, RuleError> {
        let program = self
            .programs
            .get(&(SubjectKey::from(owner), program.clone()))
            .ok_or_else(|| RuleError::new("program", "unknown owner/program"))?;
        Ok(PreparedRuleProgram {
            program: Arc::clone(program),
            namespace: self.input.namespace.clone(),
            max_work: self.limits.max_work,
        })
    }
    pub fn new_scratch(&self) -> RuleScratch {
        let mut s = RuleScratch::default();
        let n = self
            .programs
            .values()
            .map(|p| p.nodes.len())
            .max()
            .unwrap_or(0);
        let r = self
            .programs
            .values()
            .map(|p| p.reads.len())
            .max()
            .unwrap_or(0);
        s.values.reserve(n);
        s.stack.reserve(n);
        s.facts.reserve(r);
        s
    }
    pub fn evaluate<I: DefinitionSchemaIndex>(
        &self,
        owner: &SchemaSubject,
        program: &OwnedDefinitionKey,
        facts: &[RuleFact],
        definitions: &I,
        scratch: &mut RuleScratch,
    ) -> Result<ProgramEvaluation, RuleError> {
        execute::evaluate(self, owner, program, facts, definitions, scratch)
    }
}
/// Cold-bound program handle for the owned planner, not an unchecked public
/// fact API. The planner validates arbitrary Option identity existence before
/// injecting values; execution checks exact kind/unit/namespace and declared
/// input schemas. No source/index lookup or serialization occurs in this path.
#[derive(Clone, Debug)]
pub(crate) struct PreparedRuleProgram {
    program: Arc<CompiledProgram>,
    namespace: GameVersionNamespace,
    max_work: usize,
}
impl PreparedRuleProgram {
    pub(crate) fn read_ids(&self) -> impl ExactSizeIterator<Item = &OwnedDefinitionKey> {
        self.program.reads.iter().map(|read| &read.id)
    }
    /// Conservative static dependencies, including both branches and guards.
    /// Activation still uses the shared lazy evaluator at execution time.
    pub(crate) fn effect_read_indices(&self, effect_index: usize) -> Result<&[usize], RuleError> {
        self.program
            .effects
            .get(effect_index)
            .map(|effect| effect.read_indices.as_slice())
            .ok_or_else(|| RuleError::new("effect", "unknown effect index"))
    }
    /// The caller's remaining allowance is capped by the package work limit.
    pub(crate) fn evaluate_effect_indexed(
        &self,
        effect_index: usize,
        facts: &[Option<ParameterValue>],
        scratch: &mut RuleScratch,
        max_work: usize,
    ) -> Result<EffectDisposition, RuleError> {
        execute::evaluate_effect_indexed(self, effect_index, facts, scratch, max_work)
    }
}
#[derive(Clone, Debug)]
enum NodeFailure {
    Missing(usize),
    Numerical(usize, NumericalFailure),
    UnsupportedDomain(usize, BoundedInteger),
}
type NodeValue = Result<ParameterValue, NodeFailure>;
#[derive(Clone, Copy, Debug)]
struct Frame {
    node: usize,
    next: usize,
}
/// Reset for every call, including errors; contains no package ownership authority.
#[derive(Debug, Default)]
pub struct RuleScratch {
    values: Vec<Option<NodeValue>>,
    facts: Vec<Option<ParameterValue>>,
    stack: Vec<Frame>,
    work: usize,
}
impl RuleScratch {
    /// Work consumed by the latest attempt, including a failed attempt. Caches
    /// are cleared after errors; telemetry survives for the caller's budget.
    pub(crate) fn work_used(&self) -> usize {
        self.work
    }
    fn clear_caches(&mut self) {
        self.values.clear();
        self.facts.clear();
        self.stack.clear();
    }
    fn reset(&mut self) {
        self.clear_caches();
        self.work = 0;
    }
}
fn value_matches_type(v: &ParameterValue, ty: &ComputedValueType) -> bool {
    match (v, ty) {
        (ParameterValue::Boolean(_), ComputedValueType::Boolean)
        | (ParameterValue::Integer(_), ComputedValueType::Integer)
        | (ParameterValue::Option(_), ComputedValueType::Option) => true,
        (ParameterValue::Quantity(value), ComputedValueType::Quantity { unit }) => {
            value.unit() == unit
        }
        _ => false,
    }
}
fn value_type(v: &ParameterValue) -> ComputedValueType {
    match v {
        ParameterValue::Boolean(_) => ComputedValueType::Boolean,
        ParameterValue::Integer(_) => ComputedValueType::Integer,
        ParameterValue::Quantity(v) => ComputedValueType::Quantity {
            unit: v.unit().clone(),
        },
        ParameterValue::Option(_) => ComputedValueType::Option,
    }
}
fn numeric(t: &ComputedValueType) -> bool {
    matches!(
        t,
        ComputedValueType::Integer | ComputedValueType::Quantity { .. }
    )
}
fn value_in_schema(v: &ParameterValue, s: &ValueSchema) -> bool {
    match (v, s) {
        (ParameterValue::Boolean(_), ValueSchema::Boolean) => true,
        (ParameterValue::Integer(v), ValueSchema::Integer(r)) => *v >= r.minimum && *v <= r.maximum,
        (ParameterValue::Quantity(v), ValueSchema::Quantity(r)) => {
            v.unit() == r.minimum.unit()
                && v.unit() == r.maximum.unit()
                && v.value() >= r.minimum.value()
                && v.value() <= r.maximum.value()
        }
        (ParameterValue::Option(v), ValueSchema::Option { allowed }) => allowed.members.contains(v),
        _ => false,
    }
}
