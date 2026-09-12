use super::*;
fn lower(text: &str) -> SourceProgramExtraction {
    let owner =
        SourceProgramOwner::new(definitions(text, vec![callback(text, 1, 1, vec![])])).unwrap();
    let lowered = lower_from_sources(&sources(text), &owner).unwrap();
    assert!(
        lowered.unsupported().is_empty(),
        "{text}: {:?}",
        lowered.unsupported()
    );
    lowered
}
#[test]
fn standalone_modulo_is_left_associative_at_multiplicative_precedence() {
    for (expression, expected) in [
        ("a%b%c", "Modulo(Modulo(v0,v1),v2)"),
        ("a*b%c", "Modulo(Multiply(v0,v1),v2)"),
        ("a%b*c", "Multiply(Modulo(v0,v1),v2)"),
        ("a/b%c", "Modulo(Divide(v0,v1),v2)"),
        ("a%b/c", "Divide(Modulo(v0,v1),v2)"),
        ("a+b%c", "Add(v0,Modulo(v1,v2))"),
        ("a%b-c", "Subtract(Modulo(v0,v1),v2)"),
        ("-a%b", "Modulo(Negate(v0),v1)"),
        ("a%-b^c", "Modulo(v0,Negate(Power(v1,v2)))"),
        ("a^b%c", "Modulo(Power(v0,v1),v2)"),
        ("a() % b() % c()", "Modulo(Modulo(v0(),v1()),v2())"),
    ] {
        let text = format!("function(a,b,c) return {expression} end\n");
        let lowered = lower(&text);
        assert_eq!(
            super::power::returned_shape(&lowered.catalog().data().programs[0].body[0]),
            expected
        );
    }
}
#[test]
fn modulo_operand_descriptors_match_actual_cold_source_effects_and_fold_refusals() {
    // Test-only debug mutation observes original local-register timing.
    let lua = unsafe { Lua::unsafe_new() };
    let effect: mlua::Function = lua
        .load("return function() assert(debug.setlocal(2,1,11)=='a'); return 5 end")
        .eval()
        .unwrap();
    for (expression, live) in [
        ("a", true),
        ("(a)", true),
        ("true and a", true),
        ("a and a", false),
        ("a+0", false),
        ("a%13", false),
        ("(10%3) and a", true),
        ("(0%3) and a", true),
        ("(-10%3) and a", true),
        ("(1%0) and a", false),
        ("(-0%3) and a", false),
        ("((1/0)%3) and a", false),
    ] {
        let text = format!("function(a,effect) return ({expression}) % effect() end\n");
        let lowered = lower(&text);
        let ParserProgramStatementKind::Return { values } =
            &lowered.catalog().data().programs[0].body[0].operation
        else {
            panic!("return")
        };
        let ParserProgramExprKind::SourceBinary {
            operation: ParserProgramBinary::Modulo,
            left,
            ..
        } = &values.values[0].operation
        else {
            panic!("modulo")
        };
        assert_eq!(
            matches!(
                left.as_ref(),
                ParserProgramAssignmentOperand::LocalRegister { local: 0 }
            ),
            live,
            "{expression}"
        );
        // Test-only register mutation asks the original compiler whether this
        // descriptor remains live. It is not an executable native debug API.
        let source: mlua::Function = lua.load(format!("return {text}")).eval().unwrap();
        let actual: f64 = source.call((7.0, effect.clone())).unwrap();
        assert_eq!(actual, if live { 1.0 } else { 2.0 }, "{expression}");
    }
}
#[test]
fn literal_modulo_remains_a_runtime_expression_and_legacy_parser_does_not_expand() {
    let text = "function() return 10 % 3 end\n";
    let lowered = lower(text);
    let ParserProgramStatementKind::Return { values } =
        &lowered.catalog().data().programs[0].body[0].operation
    else {
        panic!("return")
    };
    assert!(matches!(
        values.values[0].operation,
        ParserProgramExprKind::SourceBinary {
            operation: ParserProgramBinary::Modulo,
            ..
        }
    ));
    for text in [
        text,
        "function() return bit.band(7,3) end\n",
        "function() return bit.bor(7,3) end\n",
        "function() return bit.bxor(7,3) end\n",
        "function() return bit.bnot(7) end\n",
    ] {
        let lua = Lua::new();
        let mut budget = Budget::default();
        let bindings = LoweringBindings::default();
        let callback = callback(text, 1, 1, vec![]);
        assert!(
            Lowerer::new(
                &lua,
                text,
                ParserCallbackId(1),
                &callback,
                &bindings,
                &mut budget
            )
            .unwrap()
            .program(&span(text, 1, 1))
            .is_err(),
            "legacy parser broadened: {text}"
        );
    }
}
