//! Formatting changes structural whitespace only. Compare syntax and comment
//! identity independently of lowering, which intentionally ignores some spelling.
use crate::{
    lexer::{Token, TokenKind, lex},
    lowering_tests as native, program_tests, *,
};
use poe_optimizer_core::{owned_definitions::OwnedDefinitionKey, owned_rules::RuleProgram};

#[derive(Debug, PartialEq, Eq)]
struct ProgramShape {
    id: Span,
    context: Span,
    nodes: Vec<ExprNode>,
    reads: Vec<ReadDeclaration>,
    statements: Vec<LetStatement>,
    effects: Vec<EffectDeclaration>,
}

// Map every source location to a common, exact-token coordinate space. This
// keeps real AST variants/edges and redundant Groups instead of serializing a
// second syntax model or ignoring source association altogether.
fn shape(parsed: &ParsedProgram<'_>, tokens: &[Token]) -> ProgramShape {
    let mut next = 0usize;
    let positions: Vec<_> = tokens
        .iter()
        .map(|token| {
            let position = next;
            next += token.span.end - token.span.start + 1;
            position
        })
        .collect();
    let remap = |span: &mut Span| {
        let first = tokens.partition_point(|token| token.span.end <= span.start);
        let last = tokens
            .partition_point(|token| token.span.start < span.end)
            .checked_sub(1)
            .unwrap();
        assert!(tokens[first].span.start <= span.start && span.start < tokens[first].span.end);
        assert!(tokens[last].span.start < span.end && span.end <= tokens[last].span.end);
        *span = Span {
            start: positions[first] + span.start - tokens[first].span.start,
            end: positions[last] + span.end - tokens[last].span.start,
        };
    };
    let mut nodes = parsed.nodes().to_vec();
    for node in &mut nodes {
        remap(&mut node.span);
        match &mut node.kind {
            ExprKind::Number(number) => {
                remap(&mut number.spelling);
                if let Some(UnitSuffix::Percent(span) | UnitSuffix::Named(span)) = &mut number.unit
                {
                    remap(span);
                }
            }
            ExprKind::Key(span) => remap(span),
            ExprKind::Path { segments } => {
                for span in segments {
                    remap(span);
                }
            }
            ExprKind::Unary { operator_span, .. } | ExprKind::Binary { operator_span, .. } => {
                remap(operator_span)
            }
            ExprKind::Boolean(_)
            | ExprKind::Call { .. }
            | ExprKind::Group { .. }
            | ExprKind::If { .. } => {}
        }
    }
    let mut reads = parsed.reads().to_vec();
    for read in &mut reads {
        remap(&mut read.id);
        remap(&mut read.name);
        remap(&mut read.span);
    }
    let mut statements = parsed.statements().to_vec();
    for statement in &mut statements {
        remap(&mut statement.name);
        remap(&mut statement.span);
    }
    let mut effects = parsed.effects().to_vec();
    for effect in &mut effects {
        remap(&mut effect.id);
        remap(&mut effect.span);
    }
    let mut id = parsed.id();
    let mut context = parsed.context();
    remap(&mut id);
    remap(&mut context);
    ProgramShape {
        id,
        context,
        nodes,
        reads,
        statements,
        effects,
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Comment<'source> {
    body: &'source str,
    preceding_tokens: usize,
    inline: bool,
}

// Successful program syntax cannot contain // inside a quoted Core key. A
// physical LF ends a comment; CR and trailing spaces remain part of its body,
// exactly as in the existing lexer (including a bare CR at end of file).
fn comments<'source>(source: &'source str, tokens: &[Token]) -> Vec<Comment<'source>> {
    let mut offset = 0;
    let mut result = vec![];
    for line in source.split_inclusive('\n') {
        if let Some(start) = line.find("//") {
            result.push(Comment {
                body: line[start..].strip_suffix('\n').unwrap_or(&line[start..]),
                preceding_tokens: tokens.partition_point(|token| token.span.end <= offset + start),
                inline: line[..start]
                    .chars()
                    .any(|character| !character.is_whitespace()),
            });
        }
        offset += line.len();
    }
    result
}

fn assert_preserved(original: &str, formatted: &str) {
    let limits = ProgramParseLimits::default();
    let before = parse_program(original, limits).unwrap();
    let after = parse_program(formatted, limits).unwrap();
    let before_tokens = lex(original, limits.expressions).unwrap();
    let after_tokens = lex(formatted, limits.expressions).unwrap();
    let token_spelling = |source: &str, tokens: &[Token]| {
        tokens
            .iter()
            .map(|token| (token.kind, token.span.text(source).unwrap().to_owned()))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        token_spelling(original, &before_tokens),
        token_spelling(formatted, &after_tokens)
    );
    assert_eq!(shape(&before, &before_tokens), shape(&after, &after_tokens));
    assert_eq!(
        comments(original, &before_tokens),
        comments(formatted, &after_tokens)
    );
    assert_eq!(
        format_program(formatted, FormatLimits::default()).unwrap(),
        formatted,
        "formatting is idempotent"
    );
}

fn format_checked(source: &str) -> String {
    let formatted = format_program(source, FormatLimits::default()).unwrap();
    assert_preserved(source, &formatted);
    formatted
}

#[test]
fn golden_layout_indents_declarations_lazy_branches_calls_and_groups() {
    let original = r#"program "sample" context actor{read "permit" as permitted:boolean=capability(current,capability.allowed);let amount=if permitted{(1damage+2damage)}else if false{3damage}else{-0damage};effect "result"=derive(current,stat.damage,amount);}"#;
    let expected = concat!(
        "program \"sample\" context actor {\n",
        "    read \"permit\" as permitted: boolean = capability(current, capability.allowed);\n",
        "    let amount = if permitted {\n",
        "        (1damage + 2damage)\n",
        "    } else if false {\n",
        "        3damage\n",
        "    } else {\n",
        "        -0damage\n",
        "    };\n",
        "    effect \"result\" = derive(current, stat.damage, amount);\n",
        "}\n",
    );
    assert_eq!(format_checked(original), expected);
    assert_eq!(
        format_checked("program \"empty\" context actor{}"),
        "program \"empty\" context actor {\n}\n"
    );
}

#[test]
fn golden_comments_keep_body_whitespace_and_inline_affinity() {
    let original = "// lead  \nprogram \"p\" context actor { // open \n // body\nlet x=1; // value\t \n} // end\n// eof";
    let expected = "// lead  \nprogram \"p\" context actor { // open \n    // body\n    let x = 1; // value\t \n} // end\n// eof\n";
    assert_eq!(format_checked(original), expected);
}

#[test]
fn formatting_preserves_numeric_spelling_suffix_adjacency_names_and_unary_structure() {
    let source = r#"program "spellings" context future_mode {
let grouped=(((.5000E-02damage)));
let signed= -0 + -0.0damage - -value;
let percent=12.00% + 1e+2% - .5%;
let named=2energy + 2e_unit + 1E02factor;
let paths=unknown.func((catalog.unit), quantity(-1.00e+03,unit.e2));
let booleans=not not left and (middle or right);
let minus=--value + -(other - -last);
let lazy=if flag{a}else if other{b}else{c}+d;
effect "future"=unknown.effect(paths);
}"#;
    let formatted = format_checked(source);
    for exact in [
        ".5000E-02damage",
        "-0",
        "-0.0damage",
        "12.00%",
        "1e+2%",
        ".5%",
        "2energy",
        "2e_unit",
        "1E02factor",
        "-1.00e+03",
        "unit.e2",
    ] {
        assert!(formatted.contains(exact), "preserve {exact}");
    }
    // Formatting is syntax-only: unresolved context, functions and symbols above
    // remain unresolved instead of being renamed, lowered or rejected here.
}

#[test]
fn inline_comments_preserve_every_token_boundary_except_indivisible_unit_suffixes() {
    let source = r#"program "boundaries" context actor { read "flag" as flag:boolean=known(); let x=if flag { (1.00damage + 2%) / -quantity(3,unit.x) } else if other { call(a,b) } else { -0 }; effect "e" when flag=emit(x); }"#;
    let tokens = lex(source, ParseLimits::default()).unwrap();
    for (position, token) in tokens.iter().enumerate() {
        let suffix = token.kind == TokenKind::Number
            && tokens.get(position + 1).is_some_and(|next| {
                next.span.start == token.span.end
                    && matches!(next.kind, TokenKind::Identifier | TokenKind::Percent)
            });
        if suffix {
            continue;
        }
        let mut commented = source[..token.span.end].to_owned();
        commented.push_str(&format!(" // boundary {position}: π 雪  \t\n"));
        commented.push_str(&source[token.span.end..]);
        format_checked(&commented);
    }
}

#[test]
fn standalone_repeated_crlf_and_eof_comments_preserve_all_non_lf_body_bytes() {
    for source in [
        "// π\t  \r\nprogram \"crlf\" context actor {\r\n// same\r\n// same\r\nlet x = 1; // inline\r\n}\r\n// tail \t\r\n",
        "program \"unicode\"\u{2003}context\u{a0}actor {\n// 雪 \r internal\t  \nlet x=1;\n// closing\n}\n// eof bare CR\r",
        "//\n//  \nprogram \"blank-comments\" context actor {\n//\nlet x=1; //\n//  \n}\n//\t ",
        "program // header\n\"split\" // identity\ncontext // keyword\nactor // context\n{\nlet // declaration\nx // name\n= // assignment\nif // condition\ntrue // opening branch\n{ // inside\n1 // first\n} // join\nelse // alternative\n{ // second branch\n2 // second\n} // semicolon\n; // end declaration\n} // program end",
    ] {
        let formatted = format_checked(source);
        assert!(formatted.ends_with('\n'));
    }
    let source = "program \"windows\" context actor {\r\nlet x=1;\r\n}\r\n";
    let formatted = format_checked(source);
    assert!(
        !formatted.contains('\r'),
        "structural CRLF becomes LF; comment CR remains content"
    );
}

#[test]
fn only_lf_in_trivia_changes_comment_affinity() {
    for separator in [
        "\r",
        "\u{85}",
        "\u{2028}",
        "\u{2029}",
        "\u{2003}",
        "\t",
        "\r\u{2028}",
    ] {
        let source = format!(
            "program \"affinity\" context actor {{let x=1;{separator}// attached  \r body\t\n{separator}// standalone \t\n}}{separator}// eof\r"
        );
        let formatted = format_checked(&source);
        assert!(formatted.contains("    let x = 1; // attached  \r body\t\n"));
        assert!(formatted.contains("\n    // standalone \t\n"));
        assert!(formatted.ends_with("} // eof\r\n"));
        let tokens = lex(&formatted, ParseLimits::default()).unwrap();
        assert_eq!(
            comments(&formatted, &tokens)
                .iter()
                .map(|comment| comment.inline)
                .collect::<Vec<_>>(),
            [true, false, true]
        );
    }
}

#[test]
fn malformed_programs_return_exact_parser_diagnostics_before_output_budget_errors() {
    for source in [
        "",
        "program \"p\" context actor { let x=1 }",
        "program \"p\" context actor { let x=1e+; }",
        "program \"p\" context actor { let x=1e2e3; }",
        "program \"p\" context actor { let x=1 < 2 < 3; }",
        "program \"p\" context actor { let x=if true {1}; }",
        "program \"p\" context actor { let x=1; read \"r\" as r:integer=read(); }",
        "program \"p\" context actor { let x=λ; }",
        "program \"escaped\\key\" context actor {}",
        "program \"p\" context actor {} trailing",
        "program \"p\" context actor { let x=1 // split suffix\n%; }",
    ] {
        let expected = parse_program(source, ProgramParseLimits::default()).unwrap_err();
        for max_output_bytes in [0, FormatLimits::default().max_output_bytes] {
            let actual = format_program(
                source,
                FormatLimits {
                    max_output_bytes,
                    ..FormatLimits::default()
                },
            )
            .unwrap_err();
            assert_eq!(actual, expected, "{source}");
            assert!(actual.span.text(source).is_some());
        }
    }
}

#[test]
fn exact_output_cap_and_all_whole_program_parser_budgets_are_enforced() {
    let source = "program \"limits\" context actor{read \"a\" as a:integer=first();read \"b\" as b:integer=second();let x=((a+b));let y=x;effect \"one\"=first(y);effect \"two\"=second(y);}";
    let formatted = format_checked(source);
    let exact_output = FormatLimits {
        max_output_bytes: formatted.len(),
        ..FormatLimits::default()
    };
    assert_eq!(format_program(source, exact_output).unwrap(), formatted);
    for maximum in [0, formatted.len() - 1] {
        let error = format_program(
            source,
            FormatLimits {
                max_output_bytes: maximum,
                ..exact_output
            },
        )
        .unwrap_err();
        assert_eq!(
            error.code,
            DiagnosticCode::LimitExceeded(LimitKind::OutputBytes)
        );
        assert!(error.span.text(source).is_some());
    }
    // Equal source/output capacity is tested on canonical input; a compact input
    // at its source cap cannot grant capacity for a larger generated program.
    let source = formatted.as_str();
    let parsed = parse_program(source, ProgramParseLimits::default()).unwrap();
    let tokens = lex(source, ParseLimits::default()).unwrap();
    let exact = ProgramParseLimits {
        expressions: ParseLimits {
            max_source_bytes: source.len(),
            max_tokens: tokens.len(),
            max_nodes: parsed.nodes().len(),
            ..ParseLimits::default()
        },
        max_reads: parsed.reads().len(),
        max_statements: parsed.statements().len(),
        max_effects: parsed.effects().len(),
    };
    assert_eq!(
        format_program(
            source,
            FormatLimits {
                parse: exact,
                ..FormatLimits::default()
            }
        )
        .unwrap(),
        formatted
    );
    for kind in [
        LimitKind::SourceBytes,
        LimitKind::Tokens,
        LimitKind::Nodes,
        LimitKind::Reads,
        LimitKind::Statements,
        LimitKind::Effects,
        LimitKind::Nesting,
    ] {
        let mut parse = exact;
        match kind {
            LimitKind::SourceBytes => parse.expressions.max_source_bytes -= 1,
            LimitKind::Tokens => parse.expressions.max_tokens -= 1,
            LimitKind::Nodes => parse.expressions.max_nodes -= 1,
            LimitKind::Reads => parse.max_reads -= 1,
            LimitKind::Statements => parse.max_statements -= 1,
            LimitKind::Effects => parse.max_effects -= 1,
            LimitKind::Nesting => parse.expressions.max_depth = 1,
            _ => unreachable!(),
        }
        let expected = parse_program(source, parse).unwrap_err();
        assert_eq!(expected.code, DiagnosticCode::LimitExceeded(kind));
        assert_eq!(
            format_program(
                source,
                FormatLimits {
                    parse,
                    ..FormatLimits::default()
                }
            )
            .unwrap_err(),
            expected
        );
    }
}

#[test]
fn generated_output_must_fit_the_same_source_limit_for_reformatting() {
    let source = "program \"growth\" context actor{let x=1;}";
    let formatted = format_checked(source);
    assert!(formatted.len() > source.len());
    let limits = |source_bytes, output_bytes| FormatLimits {
        parse: ProgramParseLimits {
            expressions: ParseLimits {
                max_source_bytes: source_bytes,
                ..ParseLimits::default()
            },
            ..ProgramParseLimits::default()
        },
        max_output_bytes: output_bytes,
    };
    for (source_bytes, output_bytes, expected) in [
        (source.len(), formatted.len(), LimitKind::SourceBytes),
        (formatted.len() - 1, formatted.len(), LimitKind::SourceBytes),
        (source.len(), source.len(), LimitKind::OutputBytes),
        (formatted.len(), formatted.len() - 1, LimitKind::OutputBytes),
    ] {
        let limits = limits(source_bytes, output_bytes);
        assert!(
            parse_program(source, limits.parse).is_ok(),
            "the input itself fits"
        );
        let error = format_program(source, limits).unwrap_err();
        assert_eq!(error.code, DiagnosticCode::LimitExceeded(expected));
        assert!(error.message.contains("formatted source"));
        assert!(error.span.text(source).is_some());
    }
    for output_bytes in [formatted.len(), formatted.len() + 1] {
        let limits = limits(formatted.len(), output_bytes);
        assert_eq!(format_program(source, limits).unwrap(), formatted);
        assert!(parse_program(&formatted, limits.parse).is_ok());
        assert_eq!(format_program(&formatted, limits).unwrap(), formatted);
    }
}

fn lower(source: &str, f: &native::fixture::Fixture) -> LoweredProgram {
    lower_program(
        &parse_program(source, ProgramParseLimits::default()).unwrap(),
        &f.schema,
        &program_tests::symbols(f),
        &f.rules.tables,
        ProgramLowerLimits::default(),
    )
    .unwrap()
}

fn local(f: &mut native::fixture::Fixture) -> &mut RuleProgram {
    f.rules.owners[0]
        .programs
        .members
        .iter_mut()
        .find(|program| program.id == OwnedDefinitionKey::new("local-item").unwrap())
        .unwrap()
}

#[test]
fn formatting_preserves_exact_lowered_ir_bindings_and_independent_engine_results() {
    let source = r#"program "local-item" context equipment_use{
read "flat" as flat:quantity(unit.damage)=contributions(current,stat.local_damage,add,sum,quantity(0,unit.damage));
read "increase" as increase:quantity(unit.percent)=contributions(current,stat.local_damage,increase,sum,quantity(0,unit.percent));
read "has-quality" as has_quality:boolean=has_item_quality(quality.local);
read "quality" as quality:quantity(unit.percent)=item_quality_amount(quality.local);
// Keep lazy quality: absent quality is not a zero-valued supplied fact.
let result=round((quantity(100,unit.damage)+flat)*(quantity(1,unit.factor)+pct(increase,unit.factor))*(if has_quality{quantity(1,unit.factor)+pct(quality,unit.factor)}else{quantity(1,unit.factor)}),quantity(1,unit.damage),nearest_ties_positive);
effect "local-result"=derive(current,stat.local_damage,result);}"#;
    let mut f = native::fixture::fixture();
    let formatted = format_checked(source);
    let before = lower(source, &f);
    let after = lower(&formatted, &f);
    assert_eq!(
        before.program, after.program,
        "node identities and all semantic fields are unchanged"
    );
    let binding_values = |lowered: &LoweredProgram| {
        lowered
            .bindings
            .iter()
            .map(|binding| {
                (
                    binding.name.clone(),
                    binding.value.clone(),
                    binding.value_type.clone(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(binding_values(&before), binding_values(&after));
    assert_ne!(
        before.spans, after.spans,
        "locations refer to their actual input representation"
    );
    for program in [before.program, after.program] {
        *local(&mut f) = program;
        for (case, expected) in f.cases[..2].iter().zip([165.0, 132.0]) {
            assert_eq!(
                native::execute(&f, &case.facts),
                native::damage(&f, expected)
            );
        }
    }
}

#[test]
fn large_flat_program_formats_reparses_and_drops_on_a_one_mib_stack() {
    std::thread::Builder::new()
        .stack_size(1024 * 1024)
        .spawn(|| {
            let mut source = String::from("program \"flat\" context actor {\n");
            for position in 0..1500 {
                source.push_str(&format!(
                    "let value_{position}=({position}); // row {position}  \n"
                ));
            }
            source.push('}');
            let formatted = format_checked(&source);
            assert_eq!(
                parse_program(&formatted, ProgramParseLimits::default())
                    .unwrap()
                    .statements()
                    .len(),
                1500
            );
        })
        .unwrap()
        .join()
        .unwrap();
}
