use super::*;
use poe_optimizer_core::{
    owned_definitions::{BoundedInteger, FiniteQuantity, UnitDefId},
    owned_rules::{RuleComparison, RuleRounding},
};

fn charge(s: &mut RuleScratch, max: usize) -> Result<(), RuleError> {
    s.work = s
        .work
        .checked_add(1)
        .filter(|v| *v <= max)
        .ok_or_else(|| RuleError::new("evaluation", "work limit exceeded"))?;
    Ok(())
}
fn quantity(value: f64, unit: &UnitDefId, node: usize) -> NodeValue {
    FiniteQuantity::new(value, unit.clone())
        .map(ParameterValue::Quantity)
        .map_err(|_| NodeFailure::Numerical(node, NumericalFailure::NonFinite))
}
fn integer(value: Option<i64>, node: usize) -> NodeValue {
    value
        .and_then(|v| BoundedInteger::new(v).ok())
        .map(ParameterValue::Integer)
        .ok_or(NodeFailure::Numerical(
            node,
            NumericalFailure::IntegerOverflow,
        ))
}
fn number(v: &ParameterValue) -> f64 {
    match v {
        ParameterValue::Quantity(v) => v.value(),
        ParameterValue::Integer(v) => v.get() as f64,
        _ => unreachable!("compiled numeric type"),
    }
}
fn boolean(v: &ParameterValue) -> bool {
    match v {
        ParameterValue::Boolean(v) => *v,
        _ => unreachable!("compiled boolean type"),
    }
}
fn cached(s: &RuleScratch, i: usize) -> &NodeValue {
    s.values[i].as_ref().expect("evaluated dependency")
}
fn val(s: &RuleScratch, i: usize) -> &ParameterValue {
    cached(s, i)
        .as_ref()
        .expect("failures propagated before operation")
}
fn failure(s: &RuleScratch, i: usize) -> Option<NodeFailure> {
    cached(s, i).as_ref().err().cloned()
}
fn comparison(kind: RuleComparison, a: &ParameterValue, b: &ParameterValue) -> bool {
    if kind == RuleComparison::Equal {
        return a == b;
    }
    let ord = match (a, b) {
        (ParameterValue::Integer(a), ParameterValue::Integer(b)) => a.cmp(b),
        (ParameterValue::Quantity(a), ParameterValue::Quantity(b)) => {
            a.value().partial_cmp(&b.value()).expect("finite")
        }
        _ => unreachable!("compiled ordered comparison"),
    };
    match kind {
        RuleComparison::Equal => ord.is_eq(),
        RuleComparison::Less => ord.is_lt(),
        RuleComparison::LessOrEqual => ord.is_le(),
        RuleComparison::Greater => ord.is_gt(),
        RuleComparison::GreaterOrEqual => ord.is_ge(),
    }
}
fn binary(kind: Binary, a: &ParameterValue, b: &ParameterValue, node: usize) -> NodeValue {
    if let ParameterValue::Integer(a) = a {
        let ParameterValue::Integer(b) = b else {
            unreachable!("integer operation type");
        };
        return integer(
            match kind {
                Binary::Add => a.get().checked_add(b.get()),
                Binary::Subtract => a.get().checked_sub(b.get()),
                Binary::Minimum => Some(a.get().min(b.get())),
                Binary::Maximum => Some(a.get().max(b.get())),
                Binary::ScaleInteger => a.get().checked_mul(b.get()),
                _ => unreachable!("quantity operation"),
            },
            node,
        );
    }
    let ParameterValue::Quantity(a) = a else {
        unreachable!("numeric operation");
    };
    let b = number(b);
    if matches!(kind, Binary::DivideFactor) && b == 0.0 {
        return Err(NodeFailure::Numerical(
            node,
            NumericalFailure::DivisionByZero,
        ));
    }
    quantity(
        match kind {
            Binary::Add => a.value() + b,
            Binary::Subtract => a.value() - b,
            Binary::Minimum => a.value().min(b),
            Binary::Maximum => a.value().max(b),
            Binary::Scale | Binary::ScaleInteger => a.value() * b,
            Binary::DivideFactor => a.value() / b,
        },
        a.unit(),
        node,
    )
}
fn compute(op: &Op, s: &RuleScratch, node: usize) -> NodeValue {
    match op {
        Op::Literal(v) => Ok(v.clone()),
        Op::Read(i) => s.facts[*i].clone().ok_or(NodeFailure::Missing(*i)),
        Op::Binary(kind, a, b) => binary(*kind, val(s, *a), val(s, *b), node),
        Op::Ratio(a, b, unit) => {
            let denominator = number(val(s, *b));
            if denominator == 0.0 {
                Err(NodeFailure::Numerical(
                    node,
                    NumericalFailure::DivisionByZero,
                ))
            } else {
                quantity(number(val(s, *a)) / denominator, unit, node)
            }
        }
        Op::Percent(a, unit) => quantity(number(val(s, *a)) / 100.0, unit, node),
        Op::Round(a, q, mode) => {
            let original = number(val(s, *a));
            let scaled = original / q.value();
            if scaled == 0.0 && original != 0.0 {
                // Division can underflow even though the original value still
                // lies strictly on one side of zero. Preserve directional modes.
                let rounded = match mode {
                    RuleRounding::Floor if original < 0.0 => -q.value(),
                    RuleRounding::Ceiling if original > 0.0 => q.value(),
                    _ => 0.0,
                };
                return quantity(rounded, q.unit(), node);
            }
            if !scaled.is_finite() {
                return Err(NodeFailure::Numerical(node, NumericalFailure::NonFinite));
            }
            let rounded = match mode {
                RuleRounding::Floor => scaled.floor(),
                RuleRounding::Ceiling => scaled.ceil(),
                RuleRounding::Truncate => scaled.trunc(),
                RuleRounding::NearestTiesPositive => {
                    // Unlike the legacy source helper, do not add 0.5 to the
                    // original value: that can change large integral values or
                    // turn an adjacent-below-half input into a tie.
                    let floor = scaled.floor();
                    if scaled - floor >= 0.5 {
                        floor + 1.0
                    } else {
                        floor
                    }
                }
            };
            quantity(rounded * q.value(), q.unit(), node)
        }
        Op::Compare(kind, a, b) => Ok(ParameterValue::Boolean(comparison(
            *kind,
            val(s, *a),
            val(s, *b),
        ))),
        Op::Not(a) => Ok(ParameterValue::Boolean(!boolean(val(s, *a)))),
        Op::All(_) | Op::Any(_) | Op::Select(..) => {
            unreachable!("lazy operations handled by stack")
        }
    }
}
fn dependency(op: &Op, next: usize) -> Option<usize> {
    match op {
        Op::Binary(_, a, b) | Op::Ratio(a, b, _) | Op::Compare(_, a, b) => match next {
            0 => Some(*a),
            1 => Some(*b),
            _ => None,
        },
        Op::Percent(a, _) | Op::Round(a, _, _) | Op::Not(a) => (next == 0).then_some(*a),
        Op::Literal(_) | Op::Read(_) => None,
        _ => unreachable!("lazy dependencies"),
    }
}
fn push(s: &mut RuleScratch, node: usize) {
    if s.values[node].is_none() {
        s.stack.push(Frame { node, next: 0 });
    }
}
fn finish(s: &mut RuleScratch, node: usize, value: NodeValue) {
    s.values[node] = Some(value);
    s.stack.pop();
}
fn node(
    p: &CompiledProgram,
    root: usize,
    s: &mut RuleScratch,
    max: usize,
) -> Result<NodeValue, RuleError> {
    push(s, root);
    while let Some(frame) = s.stack.last().copied() {
        charge(s, max)?;
        let op = &p.nodes[frame.node].expression;
        match op {
            Op::All(values) | Op::Any(values) => {
                let all = matches!(op, Op::All(_));
                if frame.next > 0 {
                    let previous = values[frame.next - 1];
                    if let Some(error) = failure(s, previous) {
                        finish(s, frame.node, Err(error));
                        continue;
                    }
                    if boolean(val(s, previous)) != all {
                        finish(s, frame.node, Ok(ParameterValue::Boolean(!all)));
                        continue;
                    }
                }
                if frame.next == values.len() {
                    finish(s, frame.node, Ok(ParameterValue::Boolean(all)));
                } else {
                    s.stack.last_mut().expect("frame").next += 1;
                    push(s, values[frame.next]);
                }
            }
            Op::Select(c, a, b) => {
                if frame.next == 0 {
                    s.stack.last_mut().expect("frame").next = 1;
                    push(s, *c);
                    continue;
                }
                if let Some(error) = failure(s, *c) {
                    finish(s, frame.node, Err(error));
                    continue;
                }
                let selected = if boolean(val(s, *c)) { *a } else { *b };
                if frame.next == 1 {
                    s.stack.last_mut().expect("frame").next = 2;
                    push(s, selected);
                } else {
                    finish(s, frame.node, cached(s, selected).clone());
                }
            }
            _ => {
                if frame.next > 0
                    && let Some(previous) = dependency(op, frame.next - 1)
                    && let Some(error) = failure(s, previous)
                {
                    finish(s, frame.node, Err(error));
                    continue;
                }
                if let Some(next) = dependency(op, frame.next) {
                    s.stack.last_mut().expect("frame").next += 1;
                    push(s, next);
                } else {
                    finish(s, frame.node, compute(op, s, frame.node));
                }
            }
        }
    }
    Ok(cached(s, root).clone())
}
fn disposition(p: &CompiledProgram, value: NodeValue) -> EffectDisposition {
    match value {
        Ok(value) => EffectDisposition::Applied { value },
        Err(NodeFailure::Missing(i)) => EffectDisposition::Unresolved {
            input: p.reads[i].id.clone(),
        },
        Err(NodeFailure::Numerical(i, reason)) => EffectDisposition::NumericalError {
            node: p.nodes[i].id.clone(),
            reason,
        },
    }
}
fn validate_fact(
    read: &CompiledRead,
    value: &ParameterValue,
    scratch: &mut RuleScratch,
    max_work: usize,
) -> Result<(), RuleError> {
    if !value_matches_type(value, &read.ty) {
        return Err(RuleError::new(
            format!("facts.{}", read.id),
            "fact type/unit mismatch",
        ));
    }
    if let Some(schema) = &read.schema {
        charge_schema(schema, scratch, max_work, "facts")?;
        if !value_in_schema(value, schema) {
            return Err(RuleError::new(
                format!("facts.{}", read.id),
                "fact violates declared value schema/membership",
            ));
        }
    }
    Ok(())
}
fn charge_schema(
    schema: &ValueSchema,
    scratch: &mut RuleScratch,
    max_work: usize,
    path: &str,
) -> Result<(), RuleError> {
    if let ValueSchema::Option { allowed } = schema {
        scratch.work = scratch
            .work
            .checked_add(allowed.members.len())
            .filter(|v| *v <= max_work)
            .ok_or_else(|| RuleError::new(path, "work limit exceeded"))?;
    }
    Ok(())
}
/// The sole effect evaluator, shared by checked reporting and prepared plans.
fn evaluate_effect(
    p: &CompiledProgram,
    effect: &CompiledEffect,
    s: &mut RuleScratch,
    max_work: usize,
) -> Result<EffectDisposition, RuleError> {
    charge(s, max_work)?;
    let guard = effect.when.map(|i| node(p, i, s, max_work)).transpose()?;
    Ok(match guard {
        Some(Ok(ParameterValue::Boolean(false))) => EffectDisposition::Inactive,
        Some(Err(error)) => disposition(p, Err(error)),
        Some(Ok(ParameterValue::Boolean(true))) | None => {
            let value = node(p, effect.value, s, max_work)?;
            if let (Ok(value), Some(schema)) = (&value, &effect.value_schema) {
                charge_schema(schema, s, max_work, "effect")?;
                if value_in_schema(value, schema) {
                    EffectDisposition::Applied {
                        value: value.clone(),
                    }
                } else {
                    EffectDisposition::UnsupportedValue {
                        value: value.clone(),
                    }
                }
            } else {
                disposition(p, value)
            }
        }
        _ => unreachable!("compiled boolean guard"),
    })
}
fn finish_attempt<T>(
    scratch: &mut RuleScratch,
    result: Result<T, RuleError>,
) -> Result<T, RuleError> {
    if result.is_err() {
        scratch.clear_caches();
    }
    result
}
pub(super) fn evaluate<I: DefinitionSchemaIndex>(
    package: &CompiledRulePackage,
    owner: &SchemaSubject,
    program: &OwnedDefinitionKey,
    facts: &[RuleFact],
    definitions: &I,
    s: &mut RuleScratch,
) -> Result<ProgramEvaluation, RuleError> {
    s.reset();
    let result = (|| {
        if definitions.identity() != &package.input.definitions
            || definitions.namespace() != &package.input.namespace
        {
            return Err(RuleError::new(
                "definitions",
                "definition identity/namespace mismatch",
            ));
        }
        let p = package
            .programs
            .get(&(SubjectKey::from(owner), program.clone()))
            .ok_or_else(|| RuleError::new("program", "unknown owner/program"))?;
        if facts.len() > package.limits.max_reads || facts.len() > p.reads.len() {
            return Err(RuleError::new(
                "facts",
                "fact count exceeds declared reads or resource limit",
            ));
        }
        digest_owned("owned-rule-facts-v1", &facts, package.limits.max_wire_bytes)
            .map_err(|e| RuleError::new("facts", e.to_string()))?;
        s.values.resize_with(p.nodes.len(), || None);
        s.facts.resize_with(p.reads.len(), || None);
        for fact in facts {
            charge(s, package.limits.max_work)?;
            let i = *p
                .read_index
                .get(&fact.read)
                .ok_or_else(|| RuleError::new("facts", "unknown fact read ID"))?;
            if s.facts[i].is_some() {
                return Err(RuleError::new("facts", "duplicate fact read ID"));
            }
            compile::validate_value(&fact.value, definitions, &format!("facts.{}", fact.read))?;
            validate_fact(&p.reads[i], &fact.value, s, package.limits.max_work)?;
            s.facts[i] = Some(fact.value.clone());
        }
        let mut effects = Vec::with_capacity(p.effects.len());
        for effect in &p.effects {
            effects.push(EffectEvaluation {
                id: effect.id.clone(),
                effect: effect.kind.clone(),
                disposition: evaluate_effect(p, effect, s, package.limits.max_work)?,
            });
        }
        let result = ProgramEvaluation {
            owner: p.owner.clone(),
            program: p.id.clone(),
            owner_programs_closure: p.closure.clone(),
            effects,
        };
        digest_owned(
            "owned-rule-result-v1",
            &result,
            package.limits.max_wire_bytes,
        )
        .map_err(|e| RuleError::new("result", e.to_string()))?;
        Ok(result)
    })();
    finish_attempt(s, result)
}
fn load_indexed(
    prepared: &PreparedRuleProgram,
    facts: &[Option<ParameterValue>],
    s: &mut RuleScratch,
    max_work: usize,
) -> Result<(), RuleError> {
    if max_work == 0 {
        return Err(RuleError::new("limits", "work allowance must be positive"));
    }
    let p = &prepared.program;
    if facts.len() != p.reads.len() {
        return Err(RuleError::new(
            "facts",
            "indexed fact count must equal declared reads",
        ));
    }
    s.values.resize_with(p.nodes.len(), || None);
    s.facts.resize_with(p.reads.len(), || None);
    for (i, value) in facts.iter().enumerate() {
        // None positions also cost work: the supplied vector must be checked in
        // full, including reads not demanded by the selected effect or branch.
        charge(s, max_work)?;
        if let Some(value) = value {
            if let ParameterValue::Option(id) = value
                && id.namespace() != &prepared.namespace
            {
                return Err(RuleError::new("facts", "foreign option namespace"));
            }
            validate_fact(&p.reads[i], value, s, max_work)?;
            s.facts[i] = Some(value.clone());
        }
    }
    Ok(())
}
pub(super) fn evaluate_effect_indexed(
    prepared: &PreparedRuleProgram,
    effect_index: usize,
    facts: &[Option<ParameterValue>],
    s: &mut RuleScratch,
    max_work: usize,
) -> Result<EffectDisposition, RuleError> {
    let max_work = max_work.min(prepared.max_work);
    s.reset();
    let result = (|| {
        let effect = prepared
            .program
            .effects
            .get(effect_index)
            .ok_or_else(|| RuleError::new("effect", "unknown effect index"))?;
        load_indexed(prepared, facts, s, max_work)?;
        evaluate_effect(&prepared.program, effect, s, max_work)
    })();
    finish_attempt(s, result)
}
