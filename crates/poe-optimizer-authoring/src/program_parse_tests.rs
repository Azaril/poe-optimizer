//! Program declarations preserve metadata and expression syntax in one arena.
use crate::*;
use poe_optimizer_core::owned_definitions::MAX_OWNED_DEFINITION_KEY_BYTES;

fn parse(source: &str) -> ParsedProgram<'_> {
    parse_program(source, ProgramParseLimits::default()).unwrap()
}

#[test]
fn program_declarations_preserve_identity_order_and_original_byte_spans() {
    let source = r#"// ⚡ original source
program "inherent-life" context actor {
    read "read-amount" as amount: quantity(unit.life) = stat(current, stat.life);
    read "read-permitted" as permitted: boolean = stat(current, stat.permitted);
    let total = if permitted { amount + 2life } else { 0life };
    let alias = total;
    effect "delivery" when permitted = contribute(current, stat.life, add, alias);
    effect "diagnostic" = require(permitted);
}"#;
    let parsed = parse(source);
    assert_eq!(parsed.source(), source);
    assert_eq!(parsed.text(parsed.id()), Some("inherent-life"));
    assert_eq!(parsed.text(parsed.context()), Some("actor"));
    assert_eq!(parsed.reads().len(), 2);
    assert_eq!(parsed.statements().len(), 2);
    assert_eq!(parsed.effects().len(), 2);
    assert_eq!(parsed.text(parsed.reads()[0].id), Some("read-amount"));
    assert_eq!(parsed.text(parsed.reads()[1].id), Some("read-permitted"));
    assert_eq!(parsed.text(parsed.reads()[0].name), Some("amount"));
    assert_eq!(
        parsed.reads()[0].name.start,
        source.find("as amount").unwrap() + 3
    );
    assert_eq!(
        parsed.text(parsed.node(parsed.reads()[0].value_type).unwrap().span),
        Some("quantity(unit.life)")
    );
    assert_eq!(
        parsed.text(parsed.node(parsed.reads()[0].source).unwrap().span),
        Some("stat(current, stat.life)")
    );
    assert!(matches!(
        parsed.node(parsed.statements()[0].value).unwrap().kind,
        ExprKind::If { .. }
    ));
    assert_eq!(parsed.text(parsed.effects()[0].id), Some("delivery"));
    assert_eq!(
        parsed.text(parsed.node(parsed.effects()[0].when.unwrap()).unwrap().span),
        Some("permitted")
    );
    assert!(parsed.effects()[1].when.is_none());
    assert!(parsed.reads()[1].source.index() < parsed.statements()[0].value.index());
    assert!(parsed.statements()[1].value.index() < parsed.effects()[0].effect.index());
    for span in parsed
        .reads()
        .iter()
        .map(|declaration| declaration.span)
        .chain(parsed.statements().iter().map(|statement| statement.span))
        .chain(parsed.effects().iter().map(|declaration| declaration.span))
    {
        assert!(parsed.text(span).unwrap().ends_with(';'));
    }
    for node in parsed.nodes() {
        assert!(parsed.text(node.span).is_some());
    }
}

#[test]
fn quoted_keys_are_validated_metadata_with_separate_content_spans() {
    let source = r#"program "p" context actor {
        read "r" as value: integer = query("query-id.2");
        let value_copy = value;
        effect "e" = require(value_copy, "requirement-id");
    }"#;
    let parsed = parse(source);
    let keys = parsed
        .nodes()
        .iter()
        .filter_map(|node| match node.kind {
            ExprKind::Key(contents) => Some((node.span, contents)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(keys.len(), 2);
    assert_eq!(parsed.text(keys[0].0), Some("\"query-id.2\""));
    assert_eq!(parsed.text(keys[0].1), Some("query-id.2"));
    assert_eq!(parsed.text(keys[1].1), Some("requirement-id"));
    for (whole, contents) in keys {
        assert_eq!(contents.start, whole.start + 1);
        assert_eq!(contents.end + 1, whole.end);
    }
    // A key is syntactically representable in any expression position. The
    // value lowerer must reject it rather than introducing a scalar string type.
    let expression = parse_expression("\"meta-key\"", ParseLimits::default()).unwrap();
    assert!(matches!(
        expression.node(expression.root()).unwrap().kind,
        ExprKind::Key(_)
    ));
    let maximal = "a".repeat(MAX_OWNED_DEFINITION_KEY_BYTES);
    parse(&format!("program \"{maximal}\" context actor {{}}"));
    let oversized = format!("{maximal}a");
    for contents in [
        "",
        "Upper",
        "_prefix",
        ".prefix",
        "-prefix",
        "white space",
        "a\\b",
        "é",
        "a\nb",
        oversized.as_str(),
    ] {
        let source = format!("program \"{contents}\" context actor {{}}");
        let error = parse_program(&source, ProgramParseLimits::default()).unwrap_err();
        assert_eq!(error.code, DiagnosticCode::InvalidKey, "{contents:?}");
        assert!(error.span.text(&source).is_some());
    }
    for source in ["program \"unterminated", "\"a\\\"b\""] {
        let error = parse_program(source, ProgramParseLimits::default()).unwrap_err();
        assert_eq!(error.code, DiagnosticCode::InvalidKey);
        assert!(error.span.text(source).is_some());
    }
}

#[test]
fn program_sections_are_ordered_without_reserving_contextual_words() {
    let parsed = parse(
        r#"program "p" context actor {
            read "r" as read: integer = source(context);
            let program = read;
            let context = program;
            let effect = context;
            let when = effect;
            let as = when;
            effect "e" when when = constructor(as);
        }"#,
    );
    assert_eq!(parsed.statements().len(), 5);
    for source in [
        r#"program "p" context actor { let a = 1; read "r" as b: integer = source(); }"#,
        r#"program "p" context actor { effect "e" = require(a); let a = true; }"#,
        r#"program "p" context actor { effect "e" = require(a); read "r" as a: boolean = source(); }"#,
    ] {
        let error = parse_program(source, ProgramParseLimits::default()).unwrap_err();
        assert_eq!(error.code, DiagnosticCode::ExpectedToken);
        assert!(error.span.text(source).is_some());
    }
    for word in ["program", "context", "read", "as", "effect", "when"] {
        let expression = parse_expression(word, ParseLimits::default()).unwrap();
        assert!(matches!(
            expression.node(expression.root()).unwrap().kind,
            ExprKind::Path { .. }
        ));
    }
}

#[test]
fn parser_accepts_empty_and_semantically_unchecked_programs() {
    let empty = parse_program(
        "program \"p\" context actor {}",
        ProgramParseLimits {
            expressions: ParseLimits {
                max_tokens: 6,
                max_nodes: 0,
                max_depth: 0,
                ..ParseLimits::default()
            },
            max_reads: 0,
            max_statements: 0,
            max_effects: 0,
        },
    )
    .unwrap();
    assert!(empty.nodes().is_empty());
    assert!(empty.reads().is_empty());
    assert!(empty.statements().is_empty());
    assert!(empty.effects().is_empty());
    // Type, context, ownership, duplicate identities and guard/value bindings
    // are semantic checks, not a second schema authority in the parser.
    parse(
        r#"program "p" context unresolved {
            read "duplicate" as x: unfamiliar(foo) = source();
            read "duplicate" as x: 1 + 2 = false;
            let x = true;
            effect "same" when x and true = unknown(1 + 2);
            effect "same" = false;
        }"#,
    );
}

#[test]
fn malformed_program_structure_reports_valid_source_boundaries() {
    for source in [
        "",
        "program p context actor {}",
        "program \"p\" actor {}",
        "program \"p\" context actor",
        "program \"p\" context actor {",
        "program \"p\" context actor {} trailing",
        "program \"p\" context actor {};",
        "program \"p\" context actor {} program \"q\" context actor {}",
        "program \"p\" context actor { read \"r\" x: integer = source(); }",
        "program \"p\" context actor { read \"r\" as x integer = source(); }",
        "program \"p\" context actor { read \"r\" as x: integer source(); }",
        "program \"p\" context actor { read \"r\" as x: integer = source() }",
        "program \"p\" context actor { effect \"e\" when = require(x); }",
        "program \"p\" context actor { effect \"e\" require(x); }",
        "program \"p\" context actor { effect \"e\" = require(x) }",
        "program \"p\" context actor { effect \"e\" = ; }",
        "program \"p\" context actor { let a = if true { 1 } else { 2 };",
    ] {
        let error = parse_program(source, ProgramParseLimits::default()).unwrap_err();
        assert!(error.span.text(source).is_some(), "{source}: {error}");
    }
    let source = "// ⚡\nprogram \"p\" context actor { let a = 💥; }";
    let error = parse_program(source, ProgramParseLimits::default()).unwrap_err();
    assert_eq!(error.code, DiagnosticCode::UnexpectedCharacter);
    assert_eq!(error.span.text(source), Some("💥"));
}

#[test]
fn all_program_sections_share_source_token_and_node_budgets() {
    let source = r#"program "p" context actor { read "r" as a: integer = source(); let b = a; effect "e" when b = require(b); }"#;
    // Header/body punctuation count, metadata calls and dynamic expressions all
    // consume one whole-source lexer/node budget. No section resets either.
    let exact = ProgramParseLimits {
        expressions: ParseLimits {
            max_source_bytes: source.len(),
            max_tokens: 32,
            max_depth: 2,
            max_nodes: 8,
        },
        max_reads: 1,
        max_statements: 1,
        max_effects: 1,
    };
    let parsed = parse_program(source, exact).unwrap();
    assert_eq!(parsed.nodes().len(), 8);
    for (limits, kind) in [
        (
            ProgramParseLimits {
                expressions: ParseLimits {
                    max_source_bytes: source.len() - 1,
                    ..exact.expressions
                },
                ..exact
            },
            LimitKind::SourceBytes,
        ),
        (
            ProgramParseLimits {
                expressions: ParseLimits {
                    max_tokens: 31,
                    ..exact.expressions
                },
                ..exact
            },
            LimitKind::Tokens,
        ),
        (
            ProgramParseLimits {
                expressions: ParseLimits {
                    max_nodes: 7,
                    ..exact.expressions
                },
                ..exact
            },
            LimitKind::Nodes,
        ),
        (
            ProgramParseLimits {
                max_reads: 0,
                ..exact
            },
            LimitKind::Reads,
        ),
        (
            ProgramParseLimits {
                max_statements: 0,
                ..exact
            },
            LimitKind::Statements,
        ),
        (
            ProgramParseLimits {
                max_effects: 0,
                ..exact
            },
            LimitKind::Effects,
        ),
    ] {
        let error = parse_program(source, limits).unwrap_err();
        assert_eq!(error.code, DiagnosticCode::LimitExceeded(kind));
        assert!(error.span.text(source).is_some());
    }
}

#[test]
fn metadata_and_formula_nesting_share_the_same_hard_stack_limit() {
    for source in [
        "program \"p\" context actor { read \"r\" as x: kind(inner()) = source(); }",
        "program \"p\" context actor { read \"r\" as x: integer = source(inner()); }",
        "program \"p\" context actor { let x = outer(inner()); }",
        "program \"p\" context actor { effect \"e\" when outer(inner()) = effect(); }",
        "program \"p\" context actor { effect \"e\" = effect(inner()); }",
    ] {
        let error = parse_program(
            source,
            ProgramParseLimits {
                expressions: ParseLimits {
                    max_depth: 1,
                    ..ParseLimits::default()
                },
                ..ProgramParseLimits::default()
            },
        )
        .unwrap_err();
        assert_eq!(
            error.code,
            DiagnosticCode::LimitExceeded(LimitKind::Nesting)
        );
    }
    let error = parse_program(
        "program \"p\" context actor {}",
        ProgramParseLimits {
            expressions: ParseLimits {
                max_depth: MAX_PARSE_DEPTH + 1,
                ..ParseLimits::default()
            },
            ..ProgramParseLimits::default()
        },
    )
    .unwrap_err();
    assert_eq!(error.code, DiagnosticCode::InvalidLimits);
}
