//! Offline expression lowering to an unchecked fragment of the existing rule IR.
//!
//! These frontend checks do not establish read, definition, owner or execution
//! authority. The surrounding compiler must remap local keys transactionally and
//! submit the complete program to the existing `CompiledRulePackage` checker.
use crate::{
    BinaryOperator, Diagnostic, DiagnosticCode, ExprId, ExprKind, ExprNode, ExpressionBindings,
    LimitKind, NumericLiteral, ParsedBlock, ParsedExpression, Span, UnaryOperator, UnitSuffix,
};
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_definitions::{BoundedInteger, FiniteQuantity, OwnedDefinitionKey, UnitDefId},
    owned_rules::{RuleComparison, RuleExpression, RuleNode, RuleRounding},
    owned_schema::{ComputedValueType, UnitDimension},
};
use std::collections::BTreeMap;

/// Independent whole-input budgets for syntax, emitted nodes and declarations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LowerLimits {
    pub max_syntax_nodes: usize,
    pub max_nodes: usize,
    pub max_statements: usize,
}

impl Default for LowerLimits {
    fn default() -> Self {
        Self {
            max_syntax_nodes: 16 * 1024,
            max_nodes: 16 * 1024,
            max_statements: 4096,
        }
    }
}

/// An unchecked, expression-local fragment, never executable authority.
///
/// Keys have the form `expr.N`. `spans[i]` describes `nodes[i]`; syntax-only
/// arguments and grouping do not create nodes. Core intentionally normalizes
/// both floating-point zero encodings when constructing quantity literals.
#[derive(Clone, Debug, PartialEq)]
pub struct LoweredExpression {
    pub nodes: Vec<RuleNode>,
    pub root: OwnedDefinitionKey,
    pub value_type: ComputedValueType,
    pub spans: Vec<Span>,
}

/// One authored declaration pointing to its shared value in an unchecked graph.
/// An alias has its own declaration span, but does not create another rule node.
#[derive(Clone, Debug, PartialEq)]
pub struct LoweredBinding {
    pub name: String,
    pub value: OwnedDefinitionKey,
    pub value_type: ComputedValueType,
    pub span: Span,
}

/// An unchecked formula graph with declarations in authored order.
///
/// All declared calculations remain in `nodes`, including unused declarations.
/// Reusing a declaration reuses its key; repeated direct reads remain distinct.
/// The surrounding compiler must still supply and check read/effect authority.
#[derive(Clone, Debug, PartialEq)]
pub struct LoweredBlock {
    pub nodes: Vec<RuleNode>,
    pub spans: Vec<Span>,
    pub bindings: Vec<LoweredBinding>,
}

/// Borrowed syntax shared by expression and block lowering. The parser owns the
/// flat arena, so lowering does not copy it when advancing between statements.
pub(crate) struct Syntax<'a> {
    pub(crate) source: &'a str,
    pub(crate) nodes: &'a [ExprNode],
}

impl Syntax<'_> {
    fn nodes(&self) -> &[ExprNode] {
        self.nodes
    }

    pub(crate) fn node(&self, id: ExprId) -> Option<&ExprNode> {
        self.nodes.get(id.index())
    }

    pub(crate) fn text(&self, span: Span) -> Option<&str> {
        span.text(self.source)
    }
}

#[derive(Clone)]
struct Value {
    id: OwnedDefinitionKey,
    ty: ComputedValueType,
}

#[derive(Clone)]
enum Call {
    Minimum,
    Maximum,
    Scale,
    ScaleInteger,
    DivideFactor,
    Ratio(UnitDefId),
    Percent(UnitDefId),
    Round(FiniteQuantity, RuleRounding),
    QuantizeInteger(FiniteQuantity, RuleRounding),
    Quantity(FiniteQuantity),
    All,
    Any,
}

impl Call {
    fn value_arguments(&self, total: usize) -> usize {
        match self {
            Self::Minimum
            | Self::Maximum
            | Self::Scale
            | Self::ScaleInteger
            | Self::DivideFactor
            | Self::Ratio(_) => 2,
            Self::Percent(_) | Self::Round(_, _) | Self::QuantizeInteger(_, _) => 1,
            Self::Quantity(_) => 0,
            Self::All | Self::Any => total,
        }
    }
}

fn error(code: DiagnosticCode, span: Span, message: &'static str) -> Diagnostic {
    Diagnostic::new(code, span, message)
}

fn type_error(span: Span, message: &'static str) -> Diagnostic {
    error(DiagnosticCode::TypeMismatch, span, message)
}

fn arguments_error(span: Span, message: &'static str) -> Diagnostic {
    error(DiagnosticCode::InvalidArguments, span, message)
}

pub(crate) fn validate_limits(
    limits: LowerLimits,
    syntax_nodes: usize,
    span: Span,
) -> Result<(), Diagnostic> {
    if limits.max_syntax_nodes == 0 || limits.max_nodes == 0 || limits.max_statements == 0 {
        return Err(error(
            DiagnosticCode::InvalidLimits,
            span,
            "lowering limits must be positive",
        ));
    }
    if syntax_nodes > limits.max_syntax_nodes {
        return Err(error(
            DiagnosticCode::LimitExceeded(LimitKind::Nodes),
            span,
            "syntax node limit exceeded",
        ));
    }
    Ok(())
}

/// Lower only computational syntax, preserving order and association exactly.
/// The parser's postorder arena permits two linear passes without recursion.
pub fn lower_expression(
    parsed: &ParsedExpression<'_>,
    bindings: &ExpressionBindings,
    limits: LowerLimits,
) -> Result<LoweredExpression, Diagnostic> {
    let span = parsed
        .node(parsed.root())
        .map_or(Span { start: 0, end: 0 }, |node| node.span);
    validate_limits(limits, parsed.nodes().len(), span)?;
    let syntax = Syntax {
        source: parsed.source(),
        nodes: parsed.nodes(),
    };
    let mut emitter = Emitter::new(limits.max_nodes);
    let root = append_expression(
        &syntax,
        bindings,
        &BTreeMap::new(),
        0,
        parsed.root(),
        &mut emitter,
    )?;
    Ok(LoweredExpression {
        nodes: emitter.nodes,
        root: root.id,
        value_type: root.ty,
        spans: emitter.spans,
    })
}

/// Lower sequential declarations into a single shared, unchecked rule graph.
///
/// Only earlier declarations are visible. Each disjoint syntax range is scanned
/// once in each direction; no previous expression or emitted graph is cloned.
pub fn lower_block(
    parsed: &ParsedBlock<'_>,
    bindings: &ExpressionBindings,
    limits: LowerLimits,
) -> Result<LoweredBlock, Diagnostic> {
    let end = parsed
        .statements()
        .last()
        .map_or(0, |s| s.value.index() + 1);
    if end != parsed.nodes().len() {
        return Err(arguments_error(
            Span {
                start: 0,
                end: parsed.source().len(),
            },
            "block contains unassigned syntax",
        ));
    }
    lower_statements(
        &Syntax {
            source: parsed.source(),
            nodes: parsed.nodes(),
        },
        parsed.statements(),
        0,
        bindings,
        limits,
    )
}

/// Shared formula lowering inside a larger source arena. Read and effect
/// constructors are syntax metadata and are not executable formula nodes.
pub(crate) fn lower_statements(
    syntax: &Syntax<'_>,
    statements: &[crate::LetStatement],
    start: usize,
    bindings: &ExpressionBindings,
    limits: LowerLimits,
) -> Result<LoweredBlock, Diagnostic> {
    let span = Span {
        start: 0,
        end: syntax.source.len(),
    };
    validate_limits(limits, syntax.nodes().len(), span)?;
    if statements.len() > limits.max_statements {
        return Err(error(
            DiagnosticCode::LimitExceeded(LimitKind::Statements),
            statements[limits.max_statements].span,
            "lowering statement limit exceeded",
        ));
    }
    let mut emitter = Emitter::new(limits.max_nodes);
    let mut locals = BTreeMap::new();
    let mut declarations = Vec::with_capacity(statements.len());
    let mut start = start;
    for statement in statements {
        let name = syntax
            .text(statement.name)
            .ok_or_else(|| arguments_error(statement.name, "invalid declaration name span"))?;
        if bindings.value(name).is_some() || locals.contains_key(name) {
            return Err(error(
                DiagnosticCode::DuplicateSymbol,
                statement.name,
                "declaration duplicates an existing value symbol",
            ));
        }
        let value = append_expression(
            syntax,
            bindings,
            &locals,
            start,
            statement.value,
            &mut emitter,
        )?;
        start = statement.value.index() + 1;
        declarations.push(LoweredBinding {
            name: name.to_owned(),
            value: value.id.clone(),
            value_type: value.ty.clone(),
            span: statement.span,
        });
        locals.insert(name, value);
    }
    Ok(LoweredBlock {
        nodes: emitter.nodes,
        spans: emitter.spans,
        bindings: declarations,
    })
}

/// Append one statement's contiguous syntax range using the shared allocator.
fn append_expression(
    parsed: &Syntax<'_>,
    bindings: &ExpressionBindings,
    locals: &BTreeMap<&str, Value>,
    start: usize,
    root: ExprId,
    emitter: &mut Emitter,
) -> Result<Value, Diagnostic> {
    let span = parsed
        .node(root)
        .map_or(Span { start: 0, end: 0 }, |node| node.span);
    if start > root.index() || parsed.node(root).is_none() {
        return Err(arguments_error(span, "expression has no root"));
    }
    let count = root.index() - start + 1;
    let mut needed = vec![false; count];
    let mut calls = vec![None; count];
    needed[count - 1] = true;
    for index in (start..=root.index()).rev() {
        let offset = index - start;
        if !needed[offset] {
            continue;
        }
        let node = &parsed.nodes()[index];
        let mut require = |child: ExprId| -> Result<(), Diagnostic> {
            if child.index() < start || child.index() >= index {
                return Err(arguments_error(
                    node.span,
                    "expression edges must precede their parent within the statement",
                ));
            }
            needed[child.index() - start] = true;
            Ok(())
        };
        match &node.kind {
            ExprKind::Call { callee, arguments } => {
                let call = resolve_call(parsed, bindings, *callee, arguments, node.span)?;
                for argument in arguments.iter().take(call.value_arguments(arguments.len())) {
                    require(*argument)?;
                }
                calls[offset] = Some(call);
            }
            ExprKind::Unary {
                operator: UnaryOperator::Negate,
                operand,
                ..
            } if matches!(
                parsed.node(*operand).map(|v| &v.kind),
                Some(ExprKind::Number(_))
            ) => {}
            ExprKind::Unary { operand, .. } => require(*operand)?,
            ExprKind::Binary { left, right, .. } => {
                require(*left)?;
                require(*right)?;
            }
            ExprKind::Group { inner } => require(*inner)?,
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                require(*condition)?;
                require(*then_branch)?;
                require(*else_branch)?;
            }
            ExprKind::Number(_)
            | ExprKind::Boolean(_)
            | ExprKind::Path { .. }
            | ExprKind::Key(_) => {}
        }
    }

    let mut values: Vec<Option<Value>> = vec![None; count];
    for (offset, node) in parsed.nodes()[start..=root.index()].iter().enumerate() {
        let index = start + offset;
        if !needed[offset] {
            continue;
        }
        let get = |id: ExprId| -> Result<&Value, Diagnostic> {
            id.index()
                .checked_sub(start)
                .and_then(|offset| values.get(offset))
                .and_then(Option::as_ref)
                .ok_or_else(|| arguments_error(node.span, "expression dependency was not lowered"))
        };
        let value = match &node.kind {
            ExprKind::Number(number) => emitter.literal(
                number_value(parsed, bindings, *number, false, node.span)?,
                node.span,
            )?,
            ExprKind::Boolean(value) => {
                emitter.literal(ParameterValue::Boolean(*value), node.span)?
            }
            ExprKind::Key(_) => {
                return Err(arguments_error(
                    node.span,
                    "owned key literals are metadata, not formula values",
                ));
            }
            ExprKind::Path { .. } => {
                let name = path_name(parsed, ExprId(index))?;
                if let Some(value) = locals.get(name.as_str()) {
                    value.clone()
                } else {
                    let binding = bindings.value(&name).ok_or_else(|| {
                        error(
                            DiagnosticCode::UnknownSymbol,
                            node.span,
                            "unknown value symbol",
                        )
                    })?;
                    emitter.push(
                        RuleExpression::Read {
                            input: binding.read.clone(),
                        },
                        binding.value_type.clone(),
                        node.span,
                    )?
                }
            }
            ExprKind::Group { inner } => get(*inner)?.clone(),
            ExprKind::Unary {
                operator,
                operand,
                operator_span,
            } => match operator {
                UnaryOperator::Negate => {
                    if let Some(ExprKind::Number(number)) = parsed.node(*operand).map(|n| &n.kind) {
                        emitter.literal(
                            number_value(parsed, bindings, *number, true, node.span)?,
                            node.span,
                        )?
                    } else {
                        let value = get(*operand)?;
                        numeric(&value.ty, *operator_span)?;
                        let minus_one = emitter.literal(
                            ParameterValue::Integer(
                                BoundedInteger::new(-1).expect("negative one is bounded"),
                            ),
                            *operator_span,
                        )?;
                        emitter.push(
                            RuleExpression::ScaleInteger {
                                value: value.id.clone(),
                                count: minus_one.id,
                            },
                            value.ty.clone(),
                            node.span,
                        )?
                    }
                }
                UnaryOperator::Not => {
                    let value = get(*operand)?;
                    boolean(&value.ty, *operator_span)?;
                    emitter.push(
                        RuleExpression::Not {
                            value: value.id.clone(),
                        },
                        ComputedValueType::Boolean,
                        node.span,
                    )?
                }
            },
            ExprKind::Binary {
                operator,
                operator_span,
                left,
                right,
            } => binary(
                emitter,
                bindings,
                *operator,
                get(*left)?,
                get(*right)?,
                *operator_span,
                node.span,
            )?,
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition = get(*condition)?;
                let yes = get(*then_branch)?;
                let no = get(*else_branch)?;
                boolean(&condition.ty, node.span)?;
                same(&yes.ty, &no.ty, node.span)?;
                emitter.push(
                    RuleExpression::Select {
                        condition: condition.id.clone(),
                        when_true: yes.id.clone(),
                        when_false: no.id.clone(),
                    },
                    yes.ty.clone(),
                    node.span,
                )?
            }
            ExprKind::Call { arguments, .. } => {
                let call = calls[offset]
                    .take()
                    .ok_or_else(|| arguments_error(node.span, "call was not prepared"))?;
                let inputs: Vec<_> = arguments
                    .iter()
                    .take(call.value_arguments(arguments.len()))
                    .map(|id| get(*id))
                    .collect::<Result<_, _>>()?;
                lower_call(emitter, bindings, call, &inputs, node.span)?
            }
        };
        values[offset] = Some(value);
    }
    values[count - 1]
        .take()
        .ok_or_else(|| arguments_error(span, "expression root was not lowered"))
}

struct Emitter {
    nodes: Vec<RuleNode>,
    spans: Vec<Span>,
    max_nodes: usize,
}

impl Emitter {
    fn new(max_nodes: usize) -> Self {
        Self {
            nodes: Vec::new(),
            spans: Vec::new(),
            max_nodes,
        }
    }

    fn push(
        &mut self,
        expression: RuleExpression,
        ty: ComputedValueType,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if self.nodes.len() >= self.max_nodes {
            return Err(error(
                DiagnosticCode::LimitExceeded(LimitKind::LoweredNodes),
                span,
                "lowered node limit exceeded",
            ));
        }
        let id = OwnedDefinitionKey::new(format!("expr.{}", self.nodes.len()))
            .expect("generated local key is valid");
        self.nodes.push(RuleNode {
            id: id.clone(),
            expression,
        });
        self.spans.push(span);
        Ok(Value { id, ty })
    }

    fn literal(&mut self, value: ParameterValue, span: Span) -> Result<Value, Diagnostic> {
        let ty = match &value {
            ParameterValue::Boolean(_) => ComputedValueType::Boolean,
            ParameterValue::Integer(_) => ComputedValueType::Integer,
            ParameterValue::Quantity(quantity) => ComputedValueType::Quantity {
                unit: quantity.unit().clone(),
            },
            ParameterValue::Option(_) => ComputedValueType::Option,
        };
        self.push(RuleExpression::Literal { value }, ty, span)
    }
}

pub(crate) fn path_name(parsed: &Syntax<'_>, id: ExprId) -> Result<String, Diagnostic> {
    let node = parsed
        .node(id)
        .ok_or_else(|| arguments_error(Span { start: 0, end: 0 }, "missing symbolic path"))?;
    let ExprKind::Path { segments } = &node.kind else {
        return Err(arguments_error(node.span, "expected a symbolic path"));
    };
    let mut name = String::new();
    for (index, segment) in segments.iter().enumerate() {
        if index != 0 {
            name.push('.');
        }
        name.push_str(
            parsed
                .text(*segment)
                .ok_or_else(|| arguments_error(node.span, "invalid symbolic span"))?,
        );
    }
    Ok(name)
}

fn number_text(
    parsed: &Syntax<'_>,
    number: NumericLiteral,
    negative: bool,
    span: Span,
) -> Result<String, Diagnostic> {
    let spelling = parsed
        .text(number.spelling)
        .ok_or_else(|| error(DiagnosticCode::InvalidNumber, span, "invalid number span"))?;
    Ok(if negative {
        format!("-{spelling}")
    } else {
        spelling.to_owned()
    })
}

fn finite_number(
    parsed: &Syntax<'_>,
    number: NumericLiteral,
    negative: bool,
    span: Span,
) -> Result<f64, Diagnostic> {
    number_text(parsed, number, negative, span)?
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
        .ok_or_else(|| {
            error(
                DiagnosticCode::InvalidNumber,
                span,
                "quantity literal must be finite",
            )
        })
}

fn number_value(
    parsed: &Syntax<'_>,
    bindings: &ExpressionBindings,
    number: NumericLiteral,
    negative: bool,
    span: Span,
) -> Result<ParameterValue, Diagnostic> {
    if let Some(suffix) = number.unit {
        let (name, unit_span) = match suffix {
            UnitSuffix::Percent(unit_span) => ("%", unit_span),
            UnitSuffix::Named(unit_span) => (
                parsed
                    .text(unit_span)
                    .ok_or_else(|| arguments_error(span, "invalid unit span"))?,
                unit_span,
            ),
        };
        let unit = bindings.unit(name).ok_or_else(|| {
            error(
                DiagnosticCode::UnknownUnit,
                unit_span,
                "unknown unit symbol",
            )
        })?;
        let quantity = FiniteQuantity::new(
            finite_number(parsed, number, negative, span)?,
            unit.unit.clone(),
        )
        .map_err(|_| {
            error(
                DiagnosticCode::InvalidNumber,
                span,
                "quantity literal must be finite",
            )
        })?;
        Ok(ParameterValue::Quantity(quantity))
    } else {
        let spelling = parsed
            .text(number.spelling)
            .ok_or_else(|| error(DiagnosticCode::InvalidNumber, span, "invalid number span"))?;
        if !spelling.bytes().all(|c| c.is_ascii_digit()) {
            return Err(error(
                DiagnosticCode::InvalidNumber,
                span,
                "fractional and exponent literals require an explicit unit",
            ));
        }
        let value = number_text(parsed, number, negative, span)?
            .parse::<i64>()
            .ok()
            .and_then(|value| BoundedInteger::new(value).ok())
            .ok_or_else(|| {
                error(
                    DiagnosticCode::InvalidNumber,
                    span,
                    "integer literal is outside the exact bounded range",
                )
            })?;
        Ok(ParameterValue::Integer(value))
    }
}

fn symbolic_unit(
    parsed: &Syntax<'_>,
    bindings: &ExpressionBindings,
    id: ExprId,
) -> Result<UnitDefId, Diagnostic> {
    let name = path_name(parsed, id)?;
    bindings
        .unit(&name)
        .map(|value| value.unit.clone())
        .ok_or_else(|| {
            error(
                DiagnosticCode::UnknownUnit,
                parsed.nodes()[id.index()].span,
                "unknown unit symbol",
            )
        })
}

fn ungroup(parsed: &Syntax<'_>, mut id: ExprId) -> Result<ExprId, Diagnostic> {
    loop {
        let node = parsed.node(id).ok_or_else(|| {
            arguments_error(Span { start: 0, end: 0 }, "missing literal argument")
        })?;
        if let ExprKind::Group { inner } = node.kind {
            if inner.index() >= id.index() {
                return Err(arguments_error(node.span, "invalid grouped literal"));
            }
            id = inner;
        } else {
            return Ok(id);
        }
    }
}

fn quantity_constructor(
    parsed: &Syntax<'_>,
    bindings: &ExpressionBindings,
    arguments: &[ExprId],
    span: Span,
) -> Result<FiniteQuantity, Diagnostic> {
    arity(arguments, 2, span)?;
    let mut number_id = ungroup(parsed, arguments[0])?;
    let negative = if let ExprKind::Unary {
        operator: UnaryOperator::Negate,
        operand,
        ..
    } = parsed.nodes()[number_id.index()].kind
    {
        number_id = ungroup(parsed, operand)?;
        true
    } else {
        false
    };
    let ExprKind::Number(number) = parsed.nodes()[number_id.index()].kind else {
        return Err(arguments_error(
            parsed.nodes()[arguments[0].index()].span,
            "quantity requires a numeric literal",
        ));
    };
    if number.unit.is_some() {
        return Err(arguments_error(
            parsed.nodes()[arguments[0].index()].span,
            "quantity literal must not already have a unit",
        ));
    }
    let unit = symbolic_unit(parsed, bindings, arguments[1])?;
    FiniteQuantity::new(finite_number(parsed, number, negative, span)?, unit).map_err(|_| {
        error(
            DiagnosticCode::InvalidNumber,
            span,
            "quantity literal must be finite",
        )
    })
}

fn static_quantum(
    parsed: &Syntax<'_>,
    bindings: &ExpressionBindings,
    id: ExprId,
) -> Result<FiniteQuantity, Diagnostic> {
    let literal = ungroup(parsed, id)?;
    let node = &parsed.nodes()[literal.index()];
    let quantity = match &node.kind {
        ExprKind::Number(number) => {
            match number_value(parsed, bindings, *number, false, node.span)? {
                ParameterValue::Quantity(value) => value,
                _ => {
                    return Err(arguments_error(
                        node.span,
                        "quantum requires an explicit unit",
                    ));
                }
            }
        }
        ExprKind::Call { callee, arguments } if path_name(parsed, *callee)? == "quantity" => {
            quantity_constructor(parsed, bindings, arguments, node.span)?
        }
        _ => {
            return Err(arguments_error(
                node.span,
                "quantum must be a positive quantity literal",
            ));
        }
    };
    if quantity.value() <= 0.0 {
        return Err(arguments_error(node.span, "quantum must be positive"));
    }
    Ok(quantity)
}

/// A literal constructor used in read metadata. It never emits or evaluates
/// a formula; arithmetic, named values and calls other than quantity fail.
pub(crate) fn static_literal(
    parsed: &Syntax<'_>,
    bindings: &ExpressionBindings,
    id: ExprId,
) -> Result<ParameterValue, Diagnostic> {
    let id = ungroup(parsed, id)?;
    let node = &parsed.nodes()[id.index()];
    match &node.kind {
        ExprKind::Number(number) => number_value(parsed, bindings, *number, false, node.span),
        ExprKind::Boolean(value) => Ok(ParameterValue::Boolean(*value)),
        ExprKind::Unary {
            operator: UnaryOperator::Negate,
            operand,
            ..
        } => {
            let operand = ungroup(parsed, *operand)?;
            let ExprKind::Number(number) = parsed.nodes()[operand.index()].kind else {
                return Err(arguments_error(
                    node.span,
                    "expected a signed numeric literal",
                ));
            };
            number_value(parsed, bindings, number, true, node.span)
        }
        ExprKind::Call { callee, arguments } if path_name(parsed, *callee)? == "quantity" => {
            quantity_constructor(parsed, bindings, arguments, node.span)
                .map(ParameterValue::Quantity)
        }
        _ => Err(arguments_error(node.span, "expected a static literal")),
    }
}

fn arity(arguments: &[ExprId], count: usize, span: Span) -> Result<(), Diagnostic> {
    if arguments.len() == count {
        Ok(())
    } else {
        Err(arguments_error(span, "wrong number of function arguments"))
    }
}

fn resolve_call(
    parsed: &Syntax<'_>,
    bindings: &ExpressionBindings,
    callee: ExprId,
    arguments: &[ExprId],
    span: Span,
) -> Result<Call, Diagnostic> {
    let name = path_name(parsed, callee)?;
    Ok(match name.as_str() {
        "all" => Call::All,
        "any" => Call::Any,
        "quantity" => Call::Quantity(quantity_constructor(parsed, bindings, arguments, span)?),
        "min" | "max" | "scale" | "scale_integer" | "divide_factor" => {
            arity(arguments, 2, span)?;
            match name.as_str() {
                "min" => Call::Minimum,
                "max" => Call::Maximum,
                "scale" => Call::Scale,
                "scale_integer" => Call::ScaleInteger,
                _ => Call::DivideFactor,
            }
        }
        "ratio" | "pct" => {
            let count = if name == "ratio" { 3 } else { 2 };
            arity(arguments, count, span)?;
            let unit = symbolic_unit(parsed, bindings, arguments[count - 1])?;
            dimension(
                bindings,
                &ComputedValueType::Quantity { unit: unit.clone() },
                UnitDimension::DimensionlessFactor,
                parsed.nodes()[arguments[count - 1].index()].span,
            )?;
            if name == "ratio" {
                Call::Ratio(unit)
            } else {
                Call::Percent(unit)
            }
        }
        "round" | "quantize_integer" => {
            arity(arguments, 3, span)?;
            let quantum = static_quantum(parsed, bindings, arguments[1])?;
            let mode = match path_name(parsed, arguments[2])?.as_str() {
                "floor" => RuleRounding::Floor,
                "ceiling" => RuleRounding::Ceiling,
                "truncate" => RuleRounding::Truncate,
                "nearest_ties_positive" => RuleRounding::NearestTiesPositive,
                _ => {
                    return Err(arguments_error(
                        parsed.nodes()[arguments[2].index()].span,
                        "unknown rounding mode",
                    ));
                }
            };
            if name == "round" {
                Call::Round(quantum, mode)
            } else {
                Call::QuantizeInteger(quantum, mode)
            }
        }
        _ => {
            return Err(error(
                DiagnosticCode::UnknownFunction,
                parsed.nodes()[callee.index()].span,
                "unknown function",
            ));
        }
    })
}

fn same(left: &ComputedValueType, right: &ComputedValueType, span: Span) -> Result<(), Diagnostic> {
    if left == right {
        Ok(())
    } else {
        Err(type_error(span, "operand types or exact units differ"))
    }
}

fn numeric(ty: &ComputedValueType, span: Span) -> Result<(), Diagnostic> {
    if matches!(
        ty,
        ComputedValueType::Integer | ComputedValueType::Quantity { .. }
    ) {
        Ok(())
    } else {
        Err(type_error(span, "numeric operand required"))
    }
}

fn boolean(ty: &ComputedValueType, span: Span) -> Result<(), Diagnostic> {
    if *ty == ComputedValueType::Boolean {
        Ok(())
    } else {
        Err(type_error(span, "Boolean operand required"))
    }
}

fn dimension(
    bindings: &ExpressionBindings,
    ty: &ComputedValueType,
    wanted: UnitDimension,
    span: Span,
) -> Result<(), Diagnostic> {
    if let ComputedValueType::Quantity { unit } = ty
        && bindings.dimension(unit) == Some(wanted)
    {
        return Ok(());
    }
    Err(type_error(span, "quantity has the wrong dimension"))
}

fn scaling(
    bindings: &ExpressionBindings,
    value: &Value,
    factor: &Value,
    integer: bool,
    span: Span,
) -> Result<(), Diagnostic> {
    if integer {
        numeric(&value.ty, span)?;
        if factor.ty != ComputedValueType::Integer {
            return Err(type_error(span, "integer scale requires an Integer count"));
        }
    } else {
        if !matches!(value.ty, ComputedValueType::Quantity { .. }) {
            return Err(type_error(span, "factor scaling requires a quantity"));
        }
        dimension(
            bindings,
            &factor.ty,
            UnitDimension::DimensionlessFactor,
            span,
        )?;
    }
    Ok(())
}

fn binary(
    emitter: &mut Emitter,
    bindings: &ExpressionBindings,
    operator: BinaryOperator,
    left: &Value,
    right: &Value,
    operator_span: Span,
    span: Span,
) -> Result<Value, Diagnostic> {
    use BinaryOperator as B;
    let (expression, ty) = match operator {
        B::Add | B::Subtract => {
            same(&left.ty, &right.ty, operator_span)?;
            numeric(&left.ty, operator_span)?;
            (
                if operator == B::Add {
                    RuleExpression::Add {
                        left: left.id.clone(),
                        right: right.id.clone(),
                    }
                } else {
                    RuleExpression::Subtract {
                        left: left.id.clone(),
                        right: right.id.clone(),
                    }
                },
                left.ty.clone(),
            )
        }
        B::Multiply | B::Divide => {
            let integer = operator == B::Multiply && right.ty == ComputedValueType::Integer;
            scaling(bindings, left, right, integer, operator_span)?;
            let expression = if integer {
                RuleExpression::ScaleInteger {
                    value: left.id.clone(),
                    count: right.id.clone(),
                }
            } else if operator == B::Multiply {
                RuleExpression::Scale {
                    value: left.id.clone(),
                    factor: right.id.clone(),
                }
            } else {
                RuleExpression::DivideFactor {
                    value: left.id.clone(),
                    divisor: right.id.clone(),
                }
            };
            (expression, left.ty.clone())
        }
        B::And | B::Or => {
            boolean(&left.ty, operator_span)?;
            boolean(&right.ty, operator_span)?;
            let values = vec![left.id.clone(), right.id.clone()];
            (
                if operator == B::And {
                    RuleExpression::All { values }
                } else {
                    RuleExpression::Any { values }
                },
                ComputedValueType::Boolean,
            )
        }
        B::Equal | B::NotEqual | B::Less | B::LessEqual | B::Greater | B::GreaterEqual => {
            same(&left.ty, &right.ty, operator_span)?;
            if !matches!(operator, B::Equal | B::NotEqual) {
                numeric(&left.ty, operator_span)?;
            }
            let operation = match operator {
                B::Less => RuleComparison::Less,
                B::LessEqual => RuleComparison::LessOrEqual,
                B::Greater => RuleComparison::Greater,
                B::GreaterEqual => RuleComparison::GreaterOrEqual,
                _ => RuleComparison::Equal,
            };
            let expression = RuleExpression::Compare {
                operation,
                left: left.id.clone(),
                right: right.id.clone(),
            };
            if operator == B::NotEqual {
                let equal = emitter.push(expression, ComputedValueType::Boolean, span)?;
                return emitter.push(
                    RuleExpression::Not { value: equal.id },
                    ComputedValueType::Boolean,
                    span,
                );
            }
            (expression, ComputedValueType::Boolean)
        }
    };
    emitter.push(expression, ty, span)
}

fn lower_call(
    emitter: &mut Emitter,
    bindings: &ExpressionBindings,
    call: Call,
    inputs: &[&Value],
    span: Span,
) -> Result<Value, Diagnostic> {
    let (expression, ty) = match call {
        Call::Quantity(quantity) => {
            return emitter.literal(ParameterValue::Quantity(quantity), span);
        }
        Call::Minimum | Call::Maximum => {
            same(&inputs[0].ty, &inputs[1].ty, span)?;
            numeric(&inputs[0].ty, span)?;
            let left = inputs[0].id.clone();
            let right = inputs[1].id.clone();
            (
                if matches!(call, Call::Minimum) {
                    RuleExpression::Minimum { left, right }
                } else {
                    RuleExpression::Maximum { left, right }
                },
                inputs[0].ty.clone(),
            )
        }
        Call::Scale | Call::ScaleInteger | Call::DivideFactor => {
            scaling(
                bindings,
                inputs[0],
                inputs[1],
                matches!(call, Call::ScaleInteger),
                span,
            )?;
            let value = inputs[0].id.clone();
            let factor = inputs[1].id.clone();
            let expression = match call {
                Call::Scale => RuleExpression::Scale { value, factor },
                Call::ScaleInteger => RuleExpression::ScaleInteger {
                    value,
                    count: factor,
                },
                _ => RuleExpression::DivideFactor {
                    value,
                    divisor: factor,
                },
            };
            (expression, inputs[0].ty.clone())
        }
        Call::Ratio(unit) => {
            same(&inputs[0].ty, &inputs[1].ty, span)?;
            if !matches!(inputs[0].ty, ComputedValueType::Quantity { .. }) {
                return Err(type_error(span, "ratio requires quantities"));
            }
            (
                RuleExpression::Ratio {
                    numerator: inputs[0].id.clone(),
                    denominator: inputs[1].id.clone(),
                    unit: unit.clone(),
                },
                ComputedValueType::Quantity { unit },
            )
        }
        Call::Percent(unit) => {
            dimension(
                bindings,
                &inputs[0].ty,
                UnitDimension::PercentagePoints,
                span,
            )?;
            (
                RuleExpression::PercentAsFactor {
                    percent: inputs[0].id.clone(),
                    unit: unit.clone(),
                },
                ComputedValueType::Quantity { unit },
            )
        }
        Call::Round(quantum, mode) => {
            same(
                &inputs[0].ty,
                &ComputedValueType::Quantity {
                    unit: quantum.unit().clone(),
                },
                span,
            )?;
            (
                RuleExpression::Round {
                    value: inputs[0].id.clone(),
                    quantum,
                    mode,
                },
                inputs[0].ty.clone(),
            )
        }
        Call::QuantizeInteger(quantum, mode) => {
            same(
                &inputs[0].ty,
                &ComputedValueType::Quantity {
                    unit: quantum.unit().clone(),
                },
                span,
            )?;
            (
                RuleExpression::QuantizeInteger {
                    value: inputs[0].id.clone(),
                    quantum,
                    mode,
                },
                ComputedValueType::Integer,
            )
        }
        Call::All | Call::Any => {
            for input in inputs {
                boolean(&input.ty, span)?;
            }
            let values = inputs.iter().map(|value| value.id.clone()).collect();
            (
                if matches!(call, Call::All) {
                    RuleExpression::All { values }
                } else {
                    RuleExpression::Any { values }
                },
                ComputedValueType::Boolean,
            )
        }
    };
    emitter.push(expression, ty, span)
}
