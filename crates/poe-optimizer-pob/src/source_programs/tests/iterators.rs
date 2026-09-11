use super::*;
fn lower(text: &str) -> SourceProgramExtraction {
    let owner =
        SourceProgramOwner::new(definitions(text, vec![callback(text, 1, 1, vec![])])).unwrap();
    let lowered = lower_from_sources(&sources(text), &owner).unwrap();
    assert!(
        lowered.unsupported().is_empty(),
        "{:?}",
        lowered.unsupported()
    );
    lowered
}
#[test]
fn generic_initializer_keeps_complete_lists_packs_and_lexical_scope() {
    let text =
        "function(k,state,control) for k,v in k(state),state,control do return k,v end end\n";
    let lowered = lower(text);
    let program = &lowered.catalog().data().programs[0];
    let ParserProgramStatementKind::ForEach {
        locals,
        iterator: ParserProgramIterator::Generic { values },
        body,
    } = &program.body[0].operation
    else {
        panic!("generic loop")
    };
    assert_eq!(locals, &[3, 4]);
    assert_eq!(values.values.len(), 3);
    assert!(values.tail.is_none());
    let ParserProgramExprKind::Call { call } = &values.values[0].operation else {
        panic!("initializer call")
    };
    assert!(matches!(
        call.receiver.as_deref().unwrap().operation,
        ParserProgramExprKind::Local { local: 0 }
    ));
    let ParserProgramStatementKind::Return { values } = &body[0].operation else {
        panic!("return")
    };
    assert!(matches!(
        values.values[0].operation,
        ParserProgramExprKind::Local { local: 3 }
    ));
    for (initializer, fixed, tail) in [
        ("factory()", 0, true),
        ("(factory())", 1, false),
        ("factory(),factory()", 1, true),
        ("...", 0, true),
    ] {
        let text = format!("function(factory,...) for key in {initializer} do end end\n");
        let lowered = lower(&text);
        let ParserProgramStatementKind::ForEach {
            iterator: ParserProgramIterator::Generic { values },
            ..
        } = &lowered.catalog().data().programs[0].body[0].operation
        else {
            panic!("generic initializer")
        };
        assert_eq!(values.values.len(), fixed, "{initializer}");
        assert_eq!(values.tail.is_some(), tail, "{initializer}");
        if initializer == "..." {
            assert!(matches!(
                values.tail.as_deref(),
                Some(ParserProgramPack::Varargs)
            ));
        }
    }
}
#[test]
fn unchanged_static_dense_and_pattern_forms_match_legacy_programs() {
    for text in [
        "function(items) for k,v in ipairs(items) do end end\n",
        "function(value) for word in string.gmatch(value,'%a+') do end end\n",
    ] {
        let lowered = lower(text);
        let lua = Lua::new();
        let callback = callback(text, 1, 1, vec![]);
        let mut budget = Budget::default();
        let bindings = LoweringBindings::default();
        let legacy = Lowerer::new(
            &lua,
            text,
            ParserCallbackId(1),
            &callback,
            &bindings,
            &mut budget,
        )
        .unwrap()
        .program(&span(text, 1, 1))
        .unwrap();
        assert_eq!(lowered.catalog().data().programs[0], legacy);
        assert!(matches!(
            legacy.body[0].operation,
            ParserProgramStatementKind::ForEach {
                iterator: ParserProgramIterator::Dense { .. }
                    | ParserProgramIterator::Pattern { .. },
                ..
            }
        ));
    }
    let lowered = lower("function(items,effect) for k in ipairs(items),effect() do end end\n");
    assert!(matches!(
        lowered.catalog().data().programs[0].body[0].operation,
        ParserProgramStatementKind::ForEach {
            iterator: ParserProgramIterator::Generic { .. },
            ..
        }
    ));
}
#[test]
fn new_generic_forms_do_not_expand_legacy_parser_allowlist() {
    for text in [
        "function(step,state) for k in step,state do end end\n",
        "function(factory) for k in factory() do end end\n",
        "function(items) for k in (ipairs(items)) do end end\n",
        "function(...) for k in ... do end end\n",
    ] {
        let lua = Lua::new();
        let callback = callback(text, 1, 1, vec![]);
        let mut budget = Budget::default();
        let bindings = LoweringBindings::default();
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
            "{text}"
        );
    }
    let text = "function(items) for k in pairs(items) do end end\n";
    let owner =
        SourceProgramOwner::new(definitions(text, vec![callback(text, 1, 1, vec![])])).unwrap();
    assert!(
        lower_from_sources(&sources(text), &owner)
            .unwrap()
            .catalog()
            .data()
            .programs
            .is_empty(),
        "unbound global pairs cannot invent iterator identity"
    );
}
