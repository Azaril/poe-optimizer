//! Bounded, deterministic authoring source for an existing owned rule graph.
//!
//! Raising checks graph structure so it can order declarations safely. It does
//! not validate operation types, schema ownership or gameplay authority. The
//! existing complete-package Engine compiler remains the semantic authority.
use crate::SymbolRegistry;
use poe_optimizer_core::{
    owned_definitions::OwnedDefinitionKey,
    owned_rules::{RuleEffectKind, RuleExpression, RuleProgram},
    owned_schema::RuleEntityKind,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

mod emit;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RaiseLimits {
    pub max_reads: usize,
    pub max_nodes: usize,
    pub max_effects: usize,
    /// Operand occurrences, read/table references, effect guards/values, and
    /// modifier-transform target definitions and optional eligibility stats.
    /// Duplicated operands count separately; fixed read metadata has bounded arity.
    pub max_references: usize,
    /// All registry aliases, including aliases for the same exact address.
    pub max_symbols: usize,
    pub max_source_bytes: usize,
}

impl Default for RaiseLimits {
    fn default() -> Self {
        Self {
            max_reads: 4096,
            max_nodes: 16 * 1024,
            max_effects: 4096,
            max_references: 64 * 1024,
            max_symbols: 8192,
            max_source_bytes: 1024 * 1024,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RaiseLimit {
    Reads,
    Nodes,
    Effects,
    References,
    Symbols,
    SourceBytes,
}

/// Locations refer to the caller's input vector, never a reordered index.
/// Symbol indices cover definition aliases followed by slot aliases.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RaiseLocation {
    Program,
    Read(usize),
    Node(usize),
    Effect(usize),
    Symbol(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RaiseErrorKind {
    InvalidLimits,
    InvalidContext,
    LimitExceeded(RaiseLimit),
    DuplicateRead,
    DuplicateNode,
    DuplicateEffect,
    MissingRead,
    MissingNode,
    Cycle,
    MissingDefinitionSymbol,
    MissingSlotSymbol,
    InvalidLiteral,
}

/// A bounded diagnostic; it never owns an untrusted key, source or symbol name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RaiseError {
    pub kind: RaiseErrorKind,
    pub location: RaiseLocation,
}

impl fmt::Display for RaiseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} at {:?}", self.kind, self.location)
    }
}

impl std::error::Error for RaiseError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RaisedNode {
    pub original: OwnedDefinitionKey,
    pub name: String,
}

/// Source and a complete node map in emitted topological order. Lowering the
/// source supplies the second half of the original-node -> let -> IR mapping.
/// A successful raise is not a substitute for complete-package compilation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RaisedProgram {
    pub source: String,
    pub nodes: Vec<RaisedNode>,
}

fn error(kind: RaiseErrorKind, location: RaiseLocation) -> RaiseError {
    RaiseError { kind, location }
}

fn limit(which: RaiseLimit, location: RaiseLocation) -> RaiseError {
    error(RaiseErrorKind::LimitExceeded(which), location)
}

/// Emit one let per original node, including unused and repeated calculations.
/// Reads are sorted by ID; lets use a stable topological ordering by original
/// ID. Effects and operand lists retain their authored order exactly.
pub fn raise_program(
    program: &RuleProgram,
    symbols: &SymbolRegistry,
    limits: RaiseLimits,
) -> Result<RaisedProgram, RaiseError> {
    if [
        limits.max_reads,
        limits.max_nodes,
        limits.max_effects,
        limits.max_references,
        limits.max_symbols,
        limits.max_source_bytes,
    ]
    .contains(&0)
    {
        return Err(error(RaiseErrorKind::InvalidLimits, RaiseLocation::Program));
    }
    if program.context == RuleEntityKind::Modifier {
        return Err(error(
            RaiseErrorKind::InvalidContext,
            RaiseLocation::Program,
        ));
    }
    for (actual, maximum, kind) in [
        (program.reads.len(), limits.max_reads, RaiseLimit::Reads),
        (program.nodes.len(), limits.max_nodes, RaiseLimit::Nodes),
        (
            program.effects.len(),
            limits.max_effects,
            RaiseLimit::Effects,
        ),
    ] {
        if actual > maximum {
            return Err(limit(kind, RaiseLocation::Program));
        }
    }
    let prepared = prepare(program, limits)?;
    let source = emit::program(program, symbols, &prepared, limits)?;
    let nodes = prepared
        .order
        .iter()
        .enumerate()
        .map(|(position, original)| RaisedNode {
            original: program.nodes[*original].id.clone(),
            // Only a bounded machine index is formatted, never an input name.
            name: format!("n_{position}"),
        })
        .collect();
    Ok(RaisedProgram { source, nodes })
}

struct Prepared<'input> {
    reads: BTreeMap<&'input OwnedDefinitionKey, usize>,
    nodes: BTreeMap<&'input OwnedDefinitionKey, usize>,
    read_names: Vec<usize>,
    node_names: Vec<usize>,
    order: Vec<usize>,
}

fn prepare(program: &RuleProgram, limits: RaiseLimits) -> Result<Prepared<'_>, RaiseError> {
    let mut reads = BTreeMap::new();
    for (index, read) in program.reads.iter().enumerate() {
        if reads.insert(&read.id, index).is_some() {
            return Err(error(
                RaiseErrorKind::DuplicateRead,
                RaiseLocation::Read(index),
            ));
        }
    }
    let mut nodes = BTreeMap::new();
    for (index, node) in program.nodes.iter().enumerate() {
        if nodes.insert(&node.id, index).is_some() {
            return Err(error(
                RaiseErrorKind::DuplicateNode,
                RaiseLocation::Node(index),
            ));
        }
    }
    let mut references = 0usize;
    let mut charge = |location: RaiseLocation| -> Result<(), RaiseError> {
        references = references
            .checked_add(1)
            .filter(|count| *count <= limits.max_references)
            .ok_or_else(|| limit(RaiseLimit::References, location))?;
        Ok(())
    };
    let mut indegree = vec![0usize; program.nodes.len()];
    let mut consumers = vec![Vec::new(); program.nodes.len()];
    for (index, node) in program.nodes.iter().enumerate() {
        let location = RaiseLocation::Node(index);
        match &node.expression {
            RuleExpression::Read { input } => {
                charge(location)?;
                if !reads.contains_key(input) {
                    return Err(error(RaiseErrorKind::MissingRead, location));
                }
            }
            // The table belongs to the surrounding package. Raising preserves
            // its exact key; only the complete package checker validates it.
            RuleExpression::LookupIntegerTable { .. } => charge(location)?,
            _ => {}
        }
        dependencies(&node.expression, |key| {
            charge(location)?;
            let dependency = nodes
                .get(key)
                .copied()
                .ok_or_else(|| error(RaiseErrorKind::MissingNode, location))?;
            // Charge before storing each edge, including repeated operands.
            indegree[index] += 1;
            consumers[dependency].push(index);
            Ok(())
        })?;
    }
    let mut effect_ids = BTreeSet::new();
    for (index, effect) in program.effects.iter().enumerate() {
        let location = RaiseLocation::Effect(index);
        if !effect_ids.insert(&effect.id) {
            return Err(error(RaiseErrorKind::DuplicateEffect, location));
        }
        for key in effect
            .when
            .iter()
            .chain(std::iter::once(effect_value(&effect.effect)))
        {
            charge(location)?;
            if !nodes.contains_key(key) {
                return Err(error(RaiseErrorKind::MissingNode, location));
            }
        }
        if let RuleEffectKind::ProjectModifierTransform { targets, .. } = &effect.effect {
            for target in targets {
                charge(location)?;
                if target.when.is_some() {
                    charge(location)?;
                }
            }
        }
    }
    let mut ready = nodes
        .iter()
        .filter_map(|(key, index)| (indegree[*index] == 0).then_some((*key, *index)))
        .collect::<BTreeSet<_>>();
    let mut order = Vec::with_capacity(program.nodes.len());
    while let Some((_, index)) = ready.pop_first() {
        order.push(index);
        for consumer in &consumers[index] {
            indegree[*consumer] -= 1;
            if indegree[*consumer] == 0 {
                ready.insert((&program.nodes[*consumer].id, *consumer));
            }
        }
    }
    if order.len() != program.nodes.len() {
        let index = nodes
            .values()
            .copied()
            .find(|index| indegree[*index] != 0)
            .expect("a nonempty unvisited remainder has an incoming edge");
        return Err(error(RaiseErrorKind::Cycle, RaiseLocation::Node(index)));
    }
    let mut read_names = vec![0; program.reads.len()];
    for (position, index) in reads.values().enumerate() {
        read_names[*index] = position;
    }
    let mut node_names = vec![0; program.nodes.len()];
    for (position, index) in order.iter().enumerate() {
        node_names[*index] = position;
    }
    Ok(Prepared {
        reads,
        nodes,
        read_names,
        node_names,
        order,
    })
}

fn effect_value(effect: &RuleEffectKind) -> &OwnedDefinitionKey {
    match effect {
        RuleEffectKind::Contribute { value, .. }
        | RuleEffectKind::ProjectModifierTransform { value, .. }
        | RuleEffectKind::Derive { value, .. }
        | RuleEffectKind::ProjectSkillParameter { value, .. }
        | RuleEffectKind::ProjectActorStat { value, .. } => value,
        RuleEffectKind::Capability { enabled, .. }
        | RuleEffectKind::ActivateGrant { enabled, .. } => enabled,
        RuleEffectKind::SupportApplicability { applicable } => applicable,
        RuleEffectKind::Requirement { satisfied, .. } => satisfied,
    }
}

/// Structural traversal only. No inferred type, constant folding or execution
/// demand is calculated here. The order matches the Core operand contract.
fn dependencies(
    expression: &RuleExpression,
    mut visit: impl FnMut(&OwnedDefinitionKey) -> Result<(), RaiseError>,
) -> Result<(), RaiseError> {
    use RuleExpression::*;
    match expression {
        Literal { .. } | Read { .. } => {}
        CheckNumericDomain { value, .. }
        | Round { value, .. }
        | QuantizeInteger { value, .. }
        | Not { value } => visit(value)?,
        LookupIntegerTable { key, .. } => visit(key)?,
        OrdinaryTiming { recipe } => {
            for input in recipe.inputs() {
                visit(input)?;
            }
        }
        Add { left, right }
        | Subtract { left, right }
        | Minimum { left, right }
        | Maximum { left, right }
        | Compare { left, right, .. } => {
            visit(left)?;
            visit(right)?;
        }
        Scale { value, factor } => {
            visit(value)?;
            visit(factor)?;
        }
        ScaleInteger { value, count } => {
            visit(value)?;
            visit(count)?;
        }
        DivideFactor { value, divisor } => {
            visit(value)?;
            visit(divisor)?;
        }
        Ratio {
            numerator,
            denominator,
            ..
        } => {
            visit(numerator)?;
            visit(denominator)?;
        }
        PercentAsFactor { percent, .. } => visit(percent)?,
        All { values } | Any { values } => {
            for value in values {
                visit(value)?;
            }
        }
        Select {
            condition,
            when_true,
            when_false,
        } => {
            visit(condition)?;
            visit(when_true)?;
            visit(when_false)?;
        }
    }
    Ok(())
}
