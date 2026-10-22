//! Ordered declaration blocks share IR nodes without adding execution semantics.
use crate::{lowering_tests as native, *};
use poe_optimizer_core::{
    owned_definitions::{OwnedDefinitionKey, UnitDefId},
    owned_rules::{RuleEffectKind, RuleExpression},
    owned_schema::ComputedValueType,
};
use poe_optimizer_engine::owned_rules::{
    CompiledRulePackage, EffectDisposition, NumericalFailure, RuleFact, RuleLimits,
};

fn parse(source: &str) -> ParsedBlock<'_> {
    parse_block(source, BlockParseLimits::default()).unwrap()
}

fn lower(source: &str, bindings: &ExpressionBindings) -> LoweredBlock {
    lower_block(&parse(source), bindings, LowerLimits::default()).unwrap()
}

fn named<'a>(block: &'a LoweredBlock, name: &str) -> &'a LoweredBinding {
    block
        .bindings
        .iter()
        .find(|binding| binding.name == name)
        .unwrap()
}

#[test]
fn multiline_blocks_preserve_source_byte_spans_and_conditional_structure() {
    let source = "// ⚡ source\nlet amount = if has_quality {\n flat + 2damage\n} else { 0damage };\n\n// alias\nlet alias = amount;";
    let parsed = parse(source);
    assert_eq!(parsed.source(), source);
    assert_eq!(parsed.statements().len(), 2);
    assert_eq!(parsed.statements()[0].name.text(source), Some("amount"));
    assert_eq!(parsed.statements()[1].name.text(source), Some("alias"));
    assert_eq!(
        parsed.statements()[0].name.start,
        source.find("amount").unwrap()
    );
    assert_eq!(
        parsed.statements()[1].name.start,
        source.rfind("alias").unwrap()
    );
    assert!(matches!(
        parsed.nodes()[parsed.statements()[0].value.index()].kind,
        ExprKind::If { .. }
    ));
    for statement in parsed.statements() {
        assert!(statement.span.text(source).unwrap().starts_with("let "));
        assert!(statement.span.text(source).unwrap().ends_with(';'));
    }
    for node in parsed.nodes() {
        assert!(node.span.text(source).is_some());
    }
}

#[test]
fn empty_blocks_are_valid_but_output_is_not_a_statement() {
    let f = native::fixture::fixture();
    for source in ["", " \n// no declarations ⚡\n"] {
        let parsed = parse(source);
        assert!(parsed.nodes().is_empty());
        assert!(parsed.statements().is_empty());
        let lowered = lower_block(&parsed, &native::bindings(&f), LowerLimits::default()).unwrap();
        assert!(lowered.nodes.is_empty());
        assert!(lowered.bindings.is_empty());
    }
    assert!(parse_block("output result = 1;", BlockParseLimits::default()).is_err());
    // "output" is an ordinary name, not a temporary export construct.
    parse("let output = 1;");
}

#[test]
fn malformed_block_statements_are_rejected_at_a_valid_source_span() {
    for source in [
        "1;",
        "{ let a = 1; }",
        "let a 1;",
        "let a = ;",
        "let a = 1",
        "let a.b = 1;",
        "let let = 1;",
        "let if = 1;",
        "let a = 1; garbage",
        "let a = if true { 1 } else { 2 }; let b = ;",
    ] {
        let error = parse_block(source, BlockParseLimits::default()).unwrap_err();
        assert!(error.span.text(source).is_some(), "{source}: {error}");
    }
    let source = "// ⚡\nlet a = 1;\nlet b = 💥;";
    let error = parse_block(source, BlockParseLimits::default()).unwrap_err();
    assert_eq!(error.code, DiagnosticCode::UnexpectedCharacter);
    assert_eq!(error.span.text(source), Some("💥"));
}

#[test]
fn parsing_budgets_cover_the_entire_block_including_declaration_tokens() {
    let source = "let a = 1; let b = 2;";
    let exact = BlockParseLimits {
        expressions: ParseLimits {
            max_source_bytes: source.len(),
            max_tokens: 10,
            max_depth: 1,
            max_nodes: 2,
        },
        max_statements: 2,
    };
    assert_eq!(parse_block(source, exact).unwrap().statements().len(), 2);
    for limits in [
        BlockParseLimits {
            expressions: ParseLimits {
                max_source_bytes: source.len() - 1,
                ..exact.expressions
            },
            ..exact
        },
        BlockParseLimits {
            expressions: ParseLimits {
                max_tokens: 9,
                ..exact.expressions
            },
            ..exact
        },
        BlockParseLimits {
            expressions: ParseLimits {
                max_nodes: 1,
                ..exact.expressions
            },
            ..exact
        },
        BlockParseLimits {
            max_statements: 1,
            ..exact
        },
    ] {
        assert!(matches!(
            parse_block(source, limits).unwrap_err().code,
            DiagnosticCode::LimitExceeded(_)
        ));
    }
}

#[test]
fn intermediate_uses_share_one_node_and_aliases_emit_no_extra_nodes() {
    let f = native::fixture::fixture();
    let block = lower(
        "let base = flat; let doubled = base + base; let alias = doubled;",
        &native::bindings(&f),
    );
    assert_eq!(block.nodes.len(), 2);
    assert_eq!(block.bindings.len(), 3);
    let base = &named(&block, "base").value;
    let doubled = &named(&block, "doubled").value;
    assert_eq!(&named(&block, "alias").value, doubled);
    assert_eq!(
        block
            .nodes
            .iter()
            .find(|node| &node.id == doubled)
            .unwrap()
            .expression,
        RuleExpression::Add {
            left: base.clone(),
            right: base.clone()
        }
    );
    assert!(matches!(
        block
            .nodes
            .iter()
            .find(|node| &node.id == base)
            .unwrap()
            .expression,
        RuleExpression::Read { .. }
    ));
}

#[test]
fn separate_reads_and_unused_declarations_are_preserved() {
    let f = native::fixture::fixture();
    let block = lower(
        "let a = flat; let b = flat; let result = a + b; let unused = 1damage / 0factor;",
        &native::bindings(&f),
    );
    let a = &named(&block, "a").value;
    let b = &named(&block, "b").value;
    assert_ne!(a, b);
    let first = &block
        .nodes
        .iter()
        .find(|node| &node.id == a)
        .unwrap()
        .expression;
    let second = &block
        .nodes
        .iter()
        .find(|node| &node.id == b)
        .unwrap()
        .expression;
    assert_eq!(first, second);
    assert!(matches!(first, RuleExpression::Read { .. }));
    let unused = &named(&block, "unused").value;
    assert!(matches!(
        block
            .nodes
            .iter()
            .find(|node| &node.id == unused)
            .unwrap()
            .expression,
        RuleExpression::DivideFactor { .. }
    ));
    let keys: std::collections::BTreeSet<_> = block.nodes.iter().map(|node| &node.id).collect();
    assert_eq!(keys.len(), block.nodes.len());
}

#[test]
fn lexical_bindings_reject_duplicates_shadowing_forward_and_self_references() {
    let f = native::fixture::fixture();
    let names = native::bindings(&f);
    for source in [
        "let a = 1; let a = 2;",
        "let flat = 1;",
        "let a = b; let b = 1;",
        "let a = a;",
        "let a = unknown;",
        "let a = true; let b = a + 1damage;",
    ] {
        let parsed = parse(source);
        let error = lower_block(&parsed, &names, LowerLimits::default()).unwrap_err();
        assert!(error.span.text(source).is_some(), "{source}: {error}");
    }
    // Values do not shadow the separate function or unit symbol domains.
    let block = lower(
        "let min = 2; let damage = 1; let value = min(3, 4) + damage + min;",
        &names,
    );
    assert_eq!(
        named(&block, "value").value_type,
        ComputedValueType::Integer
    );
}

#[test]
fn block_lowering_budgets_apply_across_all_declarations() {
    let f = native::fixture::fixture();
    let names = native::bindings(&f);
    let parsed = parse("let a = flat; let b = a + a; let c = b;");
    let exact = LowerLimits {
        max_syntax_nodes: parsed.nodes().len(),
        max_nodes: 2,
        max_statements: 3,
    };
    assert_eq!(lower_block(&parsed, &names, exact).unwrap().nodes.len(), 2);
    for limits in [
        LowerLimits {
            max_syntax_nodes: exact.max_syntax_nodes - 1,
            ..exact
        },
        LowerLimits {
            max_nodes: 1,
            ..exact
        },
        LowerLimits {
            max_statements: 2,
            ..exact
        },
    ] {
        assert!(matches!(
            lower_block(&parsed, &names, limits).unwrap_err().code,
            DiagnosticCode::LimitExceeded(_)
        ));
    }
}

#[test]
fn long_alias_chain_uses_flat_storage_and_keeps_source_spans_on_a_small_stack() {
    let f = native::fixture::fixture();
    let names = native::bindings(&f);
    std::thread::Builder::new()
        .stack_size(1024 * 1024)
        .spawn(move || {
            let mut source = String::from("let n0 = 1;\n");
            for index in 1..1000 {
                source.push_str(&format!("let n{index} = n{};\n", index - 1));
            }
            let block = lower(&source, &names);
            assert_eq!(block.nodes.len(), 1);
            assert_eq!(block.bindings.len(), 1000);
            assert!(
                block
                    .bindings
                    .iter()
                    .all(|binding| binding.value == block.nodes[0].id)
            );
            assert!(block.spans.iter().all(|span| span.text(&source).is_some()));
            assert!(
                block
                    .bindings
                    .iter()
                    .all(|binding| binding.span.text(&source).is_some())
            );
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn complete_engine_program_keeps_declared_reads_effects_and_lazy_shared_nodes() {
    let mut f = native::fixture::fixture();
    let names = native::bindings(&f);
    let source = "let base = 100damage + flat;\n\
        let unavailable = quality;\n\
        let invalid = 1damage / 0factor;\n\
        let result = if has_quality { base * (1factor + pct(unavailable, factor)) } else { base };\n\
        let alias = result;";
    let block = lower(source, &names);
    let selected = named(&block, "alias");
    let original = f.rules.owners[0]
        .programs
        .members
        .iter()
        .find(|program| program.id.as_str() == "local-item")
        .unwrap()
        .clone();
    native::install(
        &mut f,
        LoweredExpression {
            nodes: block.nodes.clone(),
            root: selected.value.clone(),
            value_type: selected.value_type.clone(),
            spans: block.spans.clone(),
        },
    );
    let migrated = f.rules.owners[0]
        .programs
        .members
        .iter()
        .find(|program| program.id.as_str() == "local-item")
        .unwrap();
    assert_eq!(migrated.reads, original.reads);
    assert_eq!(migrated.context, original.context);
    assert_eq!(migrated.id, original.id);
    let mut expected_effects = original.effects;
    let poe_optimizer_core::owned_rules::RuleEffectKind::Derive { value, .. } =
        &mut expected_effects[0].effect
    else {
        panic!()
    };
    *value = named(&block, "alias").value.clone();
    assert_eq!(migrated.effects, expected_effects);
    assert_eq!(
        native::execute(&f, &f.cases[1].facts),
        native::damage(&f, 110.0)
    );
    assert_eq!(
        native::execute(&f, &f.cases[0].facts),
        native::damage(&f, 137.5)
    );
    let mut missing = f.cases[0].facts.clone();
    missing.retain(|(key, _)| key.as_str() != "quality");
    assert_eq!(
        native::execute(&f, &missing),
        EffectDisposition::Unresolved {
            input: OwnedDefinitionKey::new("quality").unwrap(),
        }
    );
    let selected = named(&block, "invalid");
    native::install(
        &mut f,
        LoweredExpression {
            nodes: block.nodes.clone(),
            root: selected.value.clone(),
            value_type: selected.value_type.clone(),
            spans: block.spans.clone(),
        },
    );
    assert!(matches!(
        native::execute(&f, &[]),
        EffectDisposition::NumericalError {
            reason: NumericalFailure::DivisionByZero,
            ..
        }
    ));
}

#[test]
fn shared_multi_effect_block_preserves_reference_ir_and_effect_dispositions() {
    let mut f = native::fixture::fixture();
    let original = f.rules.owners[2].programs.members[0].clone();
    let reference =
        CompiledRulePackage::compile(&f.rules, &f.schema, RuleLimits::default()).unwrap();
    let bindings = ExpressionBindings::from_inputs(
        &f.schema,
        &[(
            "factor",
            UnitDefId::parse(f.rules.namespace.clone(), "unit.factor").unwrap(),
        )],
        &[("capability", original.reads[0].clone())],
        &[],
        &[],
        BindingLimits::default(),
    )
    .unwrap();
    let block = lower(
        "let applicable = capability; let factor = 1.5factor;",
        &bindings,
    );
    let mapped: std::collections::BTreeMap<_, _> = original
        .nodes
        .iter()
        .map(|node| {
            (
                node.id.clone(),
                named(&block, node.id.as_str()).value.clone(),
            )
        })
        .collect();
    let mut remapped = original.clone();
    for node in &mut remapped.nodes {
        node.id = mapped[&node.id].clone();
    }
    // The original read and literal expressions are unchanged; only local node
    // names differ. No expression is cloned to supply the second effect.
    assert_eq!(block.nodes, remapped.nodes);
    for effect in &mut remapped.effects {
        effect.when = effect.when.as_ref().map(|id| mapped[id].clone());
        match &mut effect.effect {
            RuleEffectKind::SupportApplicability { applicable } => {
                *applicable = mapped[&*applicable].clone()
            }
            RuleEffectKind::Contribute { value, .. } => *value = mapped[&*value].clone(),
            other => panic!("fixture contains an unexpected effect: {other:?}"),
        }
    }
    let RuleEffectKind::SupportApplicability { applicable } = &remapped.effects[0].effect else {
        panic!()
    };
    assert_eq!(remapped.effects[1].when.as_ref(), Some(applicable));
    assert_eq!(remapped.reads, original.reads);
    assert_eq!(remapped.context, original.context);
    assert_eq!(remapped.id, original.id);
    f.rules.owners[2].programs.members[0] = remapped;
    let compiled =
        CompiledRulePackage::compile(&f.rules, &f.schema, RuleLimits::default()).unwrap();
    let mut saw_applied = false;
    let mut saw_inactive = false;
    for case in f.cases.iter().filter(|case| case.program == original.id) {
        let facts: Vec<_> = case
            .facts
            .iter()
            .map(|(read, value)| RuleFact {
                read: read.clone(),
                value: value.clone(),
            })
            .collect();
        let before = reference
            .evaluate(
                &case.owner,
                &case.program,
                &facts,
                &f.schema,
                &mut reference.new_scratch(),
            )
            .unwrap();
        let after = compiled
            .evaluate(
                &case.owner,
                &case.program,
                &facts,
                &f.schema,
                &mut compiled.new_scratch(),
            )
            .unwrap();
        assert_eq!(after.effects.len(), case.expected.len());
        assert_eq!(after.effects.len(), before.effects.len());
        for ((actual, prior), expected) in after
            .effects
            .iter()
            .zip(&before.effects)
            .zip(&case.expected)
        {
            assert_eq!(actual.id, expected.id);
            assert_eq!(actual.disposition, prior.disposition);
            assert_eq!(
                actual.disposition,
                match &expected.value {
                    Some(value) => EffectDisposition::Applied {
                        value: value.clone()
                    },
                    None => EffectDisposition::Inactive,
                }
            );
        }
        match after.effects[1].disposition {
            EffectDisposition::Applied { .. } => saw_applied = true,
            EffectDisposition::Inactive => saw_inactive = true,
            ref other => panic!("unexpected support contribution: {other:?}"),
        }
    }
    assert!(saw_applied && saw_inactive);
}
