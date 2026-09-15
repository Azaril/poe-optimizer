//! Bounded owned expression compilation and component execution.
//!
//! Explicit facts are not build/provider-resolution authority. Every graph node
//! is validated at compilation; only demanded reads are evaluated at runtime.
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::OwnedDefinitionKey,
    owned_rules::{RuleEffectKind, RulePackageInput},
    owned_schema::{
        ComputedValueType, DefinitionAddress, DefinitionSchemaIndex, SchemaClosure, SchemaSubject,
        SlotAddress, ValueSchema,
    },
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt};
mod compile;
mod execute;

#[derive(Clone, Copy, Debug)]
pub struct RuleLimits {
    pub max_owners: usize,
    pub max_programs: usize,
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
            max_owners: 4096,
            max_programs: 8192,
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
            ("owners", self.max_owners, h.max_owners),
            ("programs", self.max_programs, h.max_programs),
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
enum Op {
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
    programs: BTreeMap<(SubjectKey, OwnedDefinitionKey), CompiledProgram>,
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
#[derive(Clone, Debug)]
enum NodeFailure {
    Missing(usize),
    Numerical(usize, NumericalFailure),
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
