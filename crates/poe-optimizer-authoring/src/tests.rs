use crate::*;

fn parse(source: &str) -> ParsedExpression<'_> {
    parse_expression(source, ParseLimits::default()).unwrap()
}

fn kind<'a>(parsed: &'a ParsedExpression<'_>, id: ExprId) -> &'a ExprKind {
    &parsed.node(id).unwrap().kind
}

fn binary(parsed: &ParsedExpression<'_>, id: ExprId, expected: BinaryOperator) -> (ExprId, ExprId) {
    let ExprKind::Binary {
        operator,
        left,
        right,
        ..
    } = kind(parsed, id)
    else {
        panic!("expected binary expression: {:?}", kind(parsed, id));
    };
    assert_eq!(*operator, expected);
    (*left, *right)
}

fn path(parsed: &ParsedExpression<'_>, id: ExprId) -> String {
    let ExprKind::Path { segments } = kind(parsed, id) else {
        panic!("expected symbolic path");
    };
    segments
        .iter()
        .map(|span| parsed.text(*span).unwrap())
        .collect::<Vec<_>>()
        .join(".")
}

fn call<'a>(parsed: &'a ParsedExpression<'_>, id: ExprId, expected: &str) -> &'a [ExprId] {
    let ExprKind::Call { callee, arguments } = kind(parsed, id) else {
        panic!("expected call");
    };
    assert_eq!(path(parsed, *callee), expected);
    arguments
}

fn number<'a>(parsed: &'a ParsedExpression<'_>, id: ExprId) -> &'a NumericLiteral {
    let ExprKind::Number(number) = kind(parsed, id) else {
        panic!("expected numeric literal");
    };
    number
}

#[test]
fn life_formula_preserves_rounding_call_and_ordered_arithmetic() {
    let source =
        "round((base_life + flat_life) * (1factor + pct(increased_life)) * more_life, 1life)";
    let parsed = parse(source);
    let arguments = call(&parsed, parsed.root(), "round");
    assert_eq!(arguments.len(), 2);
    let (left, more) = binary(&parsed, arguments[0], BinaryOperator::Multiply);
    assert_eq!(path(&parsed, more), "more_life");
    let (base, factor) = binary(&parsed, left, BinaryOperator::Multiply);
    let ExprKind::Group { inner: base } = kind(&parsed, base) else {
        panic!()
    };
    let (base, flat) = binary(&parsed, *base, BinaryOperator::Add);
    assert_eq!(path(&parsed, base), "base_life");
    assert_eq!(path(&parsed, flat), "flat_life");
    let ExprKind::Group { inner: factor } = kind(&parsed, factor) else {
        panic!()
    };
    let (_, increased) = binary(&parsed, *factor, BinaryOperator::Add);
    assert_eq!(
        path(&parsed, call(&parsed, increased, "pct")[0]),
        "increased_life"
    );
    let unit = number(&parsed, arguments[1]).unit.unwrap();
    let UnitSuffix::Named(unit) = unit else {
        panic!()
    };
    assert_eq!(parsed.text(unit), Some("life"));
    assert_eq!(
        parsed.text(parsed.node(parsed.root()).unwrap().span),
        Some(source)
    );
}

#[test]
fn critical_formula_keeps_ordered_arguments_and_explicit_percent_bounds() {
    let parsed = parse(
        "clamp(cfg_chance * (1 + pct(self_inc) + pct(enemy_inc)) * (1 - pct(evade)), 0%, 100%)",
    );
    let arguments = call(&parsed, parsed.root(), "clamp");
    assert_eq!(arguments.len(), 3);
    let (chance, evade) = binary(&parsed, arguments[0], BinaryOperator::Multiply);
    let (chance, increased) = binary(&parsed, chance, BinaryOperator::Multiply);
    assert_eq!(path(&parsed, chance), "cfg_chance");
    let ExprKind::Group { inner } = kind(&parsed, increased) else {
        panic!()
    };
    let (prior, enemy) = binary(&parsed, *inner, BinaryOperator::Add);
    let (_, own) = binary(&parsed, prior, BinaryOperator::Add);
    assert_eq!(path(&parsed, call(&parsed, own, "pct")[0]), "self_inc");
    assert_eq!(path(&parsed, call(&parsed, enemy, "pct")[0]), "enemy_inc");
    let ExprKind::Group { inner } = kind(&parsed, evade) else {
        panic!()
    };
    let (_, evade) = binary(&parsed, *inner, BinaryOperator::Subtract);
    assert_eq!(path(&parsed, call(&parsed, evade, "pct")[0]), "evade");
    for (id, spelling) in [(arguments[1], "0"), (arguments[2], "100")] {
        let value = number(&parsed, id);
        assert_eq!(parsed.text(value.spelling), Some(spelling));
        assert!(matches!(value.unit, Some(UnitSuffix::Percent(_))));
    }
}

#[test]
fn precedence_and_left_association_preserve_noncommutative_order() {
    let parsed = parse("a - b - c / d * e + f");
    let (prior, f) = binary(&parsed, parsed.root(), BinaryOperator::Add);
    assert_eq!(path(&parsed, f), "f");
    let (prior, product) = binary(&parsed, prior, BinaryOperator::Subtract);
    let (a, b) = binary(&parsed, prior, BinaryOperator::Subtract);
    assert_eq!(
        (path(&parsed, a), path(&parsed, b)),
        ("a".into(), "b".into())
    );
    let (division, e) = binary(&parsed, product, BinaryOperator::Multiply);
    assert_eq!(path(&parsed, e), "e");
    let (c, d) = binary(&parsed, division, BinaryOperator::Divide);
    assert_eq!(
        (path(&parsed, c), path(&parsed, d)),
        ("c".into(), "d".into())
    );
}

#[test]
fn boolean_trees_are_ordered_and_left_associated() {
    let parsed = parse("a or b or c and not d and e == f");
    let (left, right) = binary(&parsed, parsed.root(), BinaryOperator::Or);
    let (a, b) = binary(&parsed, left, BinaryOperator::Or);
    assert_eq!(
        (path(&parsed, a), path(&parsed, b)),
        ("a".into(), "b".into())
    );
    let (left, comparison) = binary(&parsed, right, BinaryOperator::And);
    let (c, negation) = binary(&parsed, left, BinaryOperator::And);
    assert_eq!(path(&parsed, c), "c");
    let ExprKind::Unary {
        operator: UnaryOperator::Not,
        operand,
        ..
    } = kind(&parsed, negation)
    else {
        panic!()
    };
    assert_eq!(path(&parsed, *operand), "d");
    let (e, f) = binary(&parsed, comparison, BinaryOperator::Equal);
    assert_eq!(
        (path(&parsed, e), path(&parsed, f)),
        ("e".into(), "f".into())
    );
}

#[test]
fn conditional_branches_remain_distinct_lazy_syntax() {
    let parsed = parse("if never { 0% } else if always { 100% } else { unresolved(1 / 0) }");
    let ExprKind::If {
        condition,
        then_branch,
        else_branch,
    } = kind(&parsed, parsed.root())
    else {
        panic!()
    };
    assert_eq!(path(&parsed, *condition), "never");
    assert_eq!(
        parsed.text(number(&parsed, *then_branch).spelling),
        Some("0")
    );
    let ExprKind::If {
        condition,
        then_branch,
        else_branch,
    } = kind(&parsed, *else_branch)
    else {
        panic!()
    };
    assert_eq!(path(&parsed, *condition), "always");
    assert_eq!(
        parsed.text(number(&parsed, *then_branch).spelling),
        Some("100")
    );
    let arguments = call(&parsed, *else_branch, "unresolved");
    binary(&parsed, arguments[0], BinaryOperator::Divide);
    // Parsing neither resolves this call nor evaluates either branch.
    let parsed = parse("if a { b } else if c { d } else { e } + f");
    let (conditional, f) = binary(&parsed, parsed.root(), BinaryOperator::Add);
    assert!(matches!(kind(&parsed, conditional), ExprKind::If { .. }));
    assert_eq!(path(&parsed, f), "f");
}

#[test]
fn signs_remain_unary_including_negative_zero() {
    let parsed = parse("-0 + -1.25e-3factor");
    let (zero, fraction) = binary(&parsed, parsed.root(), BinaryOperator::Add);
    for (id, spelling) in [(zero, "0"), (fraction, "1.25e-3")] {
        let ExprKind::Unary {
            operator: UnaryOperator::Negate,
            operator_span,
            operand,
        } = kind(&parsed, id)
        else {
            panic!()
        };
        assert_eq!(parsed.text(*operator_span), Some("-"));
        assert_eq!(
            parsed.text(number(&parsed, *operand).spelling),
            Some(spelling)
        );
    }
}

#[test]
fn numeric_spelling_and_suffixes_remain_unresolved() {
    for (source, spelling, suffix) in [
        (".5%", ".5", Some("%")),
        ("0001.2500life", "0001.2500", Some("life")),
        ("1E+9000factor", "1E+9000", Some("factor")),
        ("2energy", "2", Some("energy")),
        ("2e_unit", "2", Some("e_unit")),
        ("1e2", "1e2", None),
        ("1e2energy", "1e2", Some("energy")),
    ] {
        let parsed = parse(source);
        let value = number(&parsed, parsed.root());
        assert_eq!(parsed.text(value.spelling), Some(spelling));
        assert_eq!(
            value.unit.map(|unit| match unit {
                UnitSuffix::Percent(span) | UnitSuffix::Named(span) => parsed.text(span).unwrap(),
            }),
            suffix
        );
    }
    // Names are syntactically admitted, not classified as valid units or reads.
    assert!(matches!(
        kind(&parse("4unregistered_unit"), ExprId(0)),
        ExprKind::Number(_)
    ));
}

#[test]
fn numeric_and_suffix_boundaries_refuse_ambiguous_or_malformed_input() {
    for source in [
        "1.", "1e", "1e+", "1E-", "1.2.3", "1_000", "1e2e3", "1e2E-3",
    ] {
        assert_eq!(
            parse_expression(source, ParseLimits::default())
                .unwrap_err()
                .code,
            DiagnosticCode::InvalidNumber,
            "{source}"
        );
    }
    for source in ["1 life", "1 %", "1life%", "1true", "1life extra"] {
        assert_eq!(
            parse_expression(source, ParseLimits::default())
                .unwrap_err()
                .code,
            DiagnosticCode::TrailingInput,
            "{source}"
        );
    }
}

#[test]
fn symbolic_paths_and_calls_keep_every_segment_and_argument() {
    let source = "math.clamp(self.life.current, 0life, cap())";
    let parsed = parse(source);
    let arguments = call(&parsed, parsed.root(), "math.clamp");
    assert_eq!(arguments.len(), 3);
    assert_eq!(path(&parsed, arguments[0]), "self.life.current");
    assert!(call(&parsed, arguments[2], "cap").is_empty());
    let ExprKind::Path { segments } = kind(&parsed, arguments[0]) else {
        panic!()
    };
    assert_eq!(
        segments
            .iter()
            .map(|span| parsed.text(*span).unwrap())
            .collect::<Vec<_>>(),
        ["self", "life", "current"]
    );
    assert_eq!(parsed.source(), source);
}

#[test]
fn chained_comparisons_require_explicit_structure() {
    for operator in ["==", "!=", "<", "<=", ">", ">="] {
        let source = format!("a {operator} b {operator} c");
        let error = parse_expression(&source, ParseLimits::default()).unwrap_err();
        assert_eq!(error.code, DiagnosticCode::ChainedComparison, "{source}");
        assert_eq!(error.span.text(&source), Some(operator));
    }
    parse("a < b and b <= c");
    parse("(a < b) == true");
    parse("a == (b != c)");
}

#[test]
fn parentheses_remain_source_associated_without_reassociation() {
    let parsed = parse("a + (b + c)");
    let (_, group) = binary(&parsed, parsed.root(), BinaryOperator::Add);
    assert_eq!(
        parsed.text(parsed.node(group).unwrap().span),
        Some("(b + c)")
    );
    let ExprKind::Group { inner } = kind(&parsed, group) else {
        panic!()
    };
    binary(&parsed, *inner, BinaryOperator::Add);
}

#[test]
fn utf8_comments_whitespace_and_error_spans_use_bytes() {
    let source = "// ⚡ formula\n\u{2003}self.life + 2life";
    let parsed = parse(source);
    let (left, _) = binary(&parsed, parsed.root(), BinaryOperator::Add);
    let span = parsed.node(left).unwrap().span;
    assert_eq!(span.start, source.find("self").unwrap());
    assert_eq!(parsed.text(span), Some("self.life"));
    for node in parsed.nodes() {
        assert!(source.is_char_boundary(node.span.start));
        assert!(source.is_char_boundary(node.span.end));
    }
    let source = "// ⚡\nself.life + 💥";
    let error = parse_expression(source, ParseLimits::default()).unwrap_err();
    assert_eq!(error.code, DiagnosticCode::UnexpectedCharacter);
    assert_eq!(
        error.span,
        Span {
            start: source.find('💥').unwrap(),
            end: source.len()
        }
    );
    assert_eq!(error.span.text(source), Some("💥"));
    assert_eq!(Span { start: 4, end: 5 }.text("⚡"), None);
}

#[test]
fn malformed_expressions_have_specific_diagnostics() {
    for source in [
        "",
        "// comment",
        "a +",
        "f(,x)",
        "f(x,)",
        "if a {} else { b }",
    ] {
        let error = parse_expression(source, ParseLimits::default()).unwrap_err();
        assert_eq!(error.code, DiagnosticCode::ExpectedExpression, "{source}");
    }
    for source in [
        "(a",
        "f(a b)",
        "self.",
        "if a { b }",
        "if a b else { c }",
        "if a { b } else c",
    ] {
        assert_eq!(
            parse_expression(source, ParseLimits::default())
                .unwrap_err()
                .code,
            DiagnosticCode::ExpectedToken,
            "{source}"
        );
    }
    for source in ["a b", "f()()", "(f)(x)", "a)", "a = b", "a; b"] {
        assert_eq!(
            parse_expression(source, ParseLimits::default())
                .unwrap_err()
                .code,
            DiagnosticCode::TrailingInput,
            "{source}"
        );
    }
    for source in ["!true", "a && b", "a ^ b", "{\"raw\":true}"] {
        assert_eq!(
            parse_expression(source, ParseLimits::default())
                .unwrap_err()
                .code,
            DiagnosticCode::UnexpectedCharacter,
            "{source}"
        );
    }
    let error = parse_expression("a +", ParseLimits::default()).unwrap_err();
    assert_eq!(error.span, Span { start: 3, end: 3 });
    assert_eq!(error.to_string(), "expected expression at bytes 3..3");
}

#[test]
fn each_resource_limit_has_an_exact_boundary() {
    let source = "a + 1life";
    let exact = ParseLimits {
        max_source_bytes: source.len(),
        max_tokens: 4,
        max_depth: 2,
        max_nodes: 3,
    };
    assert_eq!(parse_expression(source, exact).unwrap().nodes().len(), 3);
    for (limits, expected) in [
        (
            ParseLimits {
                max_source_bytes: source.len() - 1,
                ..exact
            },
            LimitKind::SourceBytes,
        ),
        (
            ParseLimits {
                max_tokens: 3,
                ..exact
            },
            LimitKind::Tokens,
        ),
        (
            ParseLimits {
                max_depth: 1,
                ..exact
            },
            LimitKind::Nesting,
        ),
        (
            ParseLimits {
                max_nodes: 2,
                ..exact
            },
            LimitKind::Nodes,
        ),
    ] {
        assert_eq!(
            parse_expression(source, limits).unwrap_err().code,
            DiagnosticCode::LimitExceeded(expected)
        );
    }
    for (limits, expected) in [
        (
            ParseLimits {
                max_source_bytes: 0,
                ..exact
            },
            LimitKind::SourceBytes,
        ),
        (
            ParseLimits {
                max_tokens: 0,
                ..exact
            },
            LimitKind::Tokens,
        ),
        (
            ParseLimits {
                max_depth: 0,
                ..exact
            },
            LimitKind::Nesting,
        ),
        (
            ParseLimits {
                max_nodes: 0,
                ..exact
            },
            LimitKind::Nodes,
        ),
    ] {
        assert_eq!(
            parse_expression("x", limits).unwrap_err().code,
            DiagnosticCode::LimitExceeded(expected)
        );
    }
    assert_eq!(
        parse_expression(
            "x",
            ParseLimits {
                max_depth: MAX_PARSE_DEPTH + 1,
                ..exact
            }
        )
        .unwrap_err()
        .code,
        DiagnosticCode::InvalidLimits
    );
}

#[test]
fn token_limit_counts_suffixes_but_not_comments_or_whitespace() {
    let source = "// text ⚡\n 1life // ignored\n";
    let limits = ParseLimits {
        max_tokens: 2,
        max_nodes: 1,
        ..ParseLimits::default()
    };
    assert_eq!(parse_expression(source, limits).unwrap().nodes().len(), 1);
    let error = parse_expression(
        source,
        ParseLimits {
            max_tokens: 1,
            ..limits
        },
    )
    .unwrap_err();
    assert_eq!(error.code, DiagnosticCode::LimitExceeded(LimitKind::Tokens));
    assert_eq!(error.span.text(source), Some("l"));
}

#[test]
fn nesting_limit_applies_to_groups_unary_calls_and_else_if() {
    for source in ["(((x)))", "not not not x", "f(g(h(x)))"] {
        parse_expression(
            source,
            ParseLimits {
                max_depth: 4,
                ..ParseLimits::default()
            },
        )
        .unwrap();
        assert_eq!(
            parse_expression(
                source,
                ParseLimits {
                    max_depth: 3,
                    ..ParseLimits::default()
                }
            )
            .unwrap_err()
            .code,
            DiagnosticCode::LimitExceeded(LimitKind::Nesting),
            "{source}"
        );
    }
    let source = "if a { b } else if c { d } else { e }";
    parse_expression(
        source,
        ParseLimits {
            max_depth: 3,
            ..ParseLimits::default()
        },
    )
    .unwrap();
    assert_eq!(
        parse_expression(
            source,
            ParseLimits {
                max_depth: 2,
                ..ParseLimits::default()
            }
        )
        .unwrap_err()
        .code,
        DiagnosticCode::LimitExceeded(LimitKind::Nesting)
    );
}

#[test]
fn wide_calls_are_bounded_by_nodes_and_keep_argument_order() {
    let source = format!(
        "f({})",
        (0..100)
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
    let limits = ParseLimits {
        max_nodes: 102,
        ..ParseLimits::default()
    };
    let parsed = parse_expression(&source, limits).unwrap();
    let arguments = call(&parsed, parsed.root(), "f");
    assert_eq!(arguments.len(), 100);
    assert_eq!(
        parsed.text(number(&parsed, arguments[99]).spelling),
        Some("99")
    );
    assert_eq!(
        parse_expression(
            &source,
            ParseLimits {
                max_nodes: 101,
                ..limits
            }
        )
        .unwrap_err()
        .code,
        DiagnosticCode::LimitExceeded(LimitKind::Nodes)
    );
}

#[test]
fn arena_edges_are_backward_and_foreign_out_of_range_ids_are_checked() {
    let parsed = parse("if a and not b { f(c + d, e) } else { (g) }");
    for (index, node) in parsed.nodes().iter().enumerate() {
        let children = match &node.kind {
            ExprKind::Number(_) | ExprKind::Boolean(_) | ExprKind::Path { .. } => Vec::new(),
            ExprKind::Call { callee, arguments } => std::iter::once(*callee)
                .chain(arguments.iter().copied())
                .collect(),
            ExprKind::Unary { operand, .. } => vec![*operand],
            ExprKind::Binary { left, right, .. } => vec![*left, *right],
            ExprKind::Group { inner } => vec![*inner],
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => vec![*condition, *then_branch, *else_branch],
        };
        assert!(children.into_iter().all(|child| child.index() < index));
    }
    assert!(parsed.node(ExprId(usize::MAX)).is_none());
}

#[test]
fn small_stack_handles_maximum_nesting_and_flat_long_trees() {
    std::thread::Builder::new()
        .stack_size(1024 * 1024)
        .spawn(|| {
            let source = format!(
                "{}x{}",
                "(".repeat(MAX_PARSE_DEPTH - 1),
                ")".repeat(MAX_PARSE_DEPTH - 1)
            );
            let parsed = parse(&source);
            assert_eq!(parsed.nodes().len(), MAX_PARSE_DEPTH);
            drop(parsed);
            let excessive = format!("{}x{}", "(".repeat(4096), ")".repeat(4096));
            assert_eq!(
                parse_expression(&excessive, ParseLimits::default())
                    .unwrap_err()
                    .code,
                DiagnosticCode::LimitExceeded(LimitKind::Nesting)
            );
            let source = std::iter::repeat_n("x", 3000)
                .collect::<Vec<_>>()
                .join(" or ");
            let parsed = parse(&source);
            assert_eq!(parsed.nodes().len(), 5999);
            let mut id = parsed.root();
            for _ in 1..3000 {
                let (left, right) = binary(&parsed, id, BinaryOperator::Or);
                assert_eq!(path(&parsed, right), "x");
                id = left;
            }
            assert_eq!(path(&parsed, id), "x");
            // Drop traverses vectors of IDs, never recursively owned expression nodes.
            drop(parsed);
        })
        .unwrap()
        .join()
        .unwrap();
}
