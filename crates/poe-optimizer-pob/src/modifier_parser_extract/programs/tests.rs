//! Structural lowering fixtures are separate from unchanged-body source parity.
use super::*;
use poe_optimizer_data::game_data::bundled_snapshot;

fn parse(body: &str) -> LowerResult<ParserProgram> {
    let owner = bundled_snapshot().unwrap();
    let data = owner.modifier_parser().data();
    let callback = ParserCallback {
        kind: ParserCallbackKind::Lua { source: span(body) },
        upvalues: vec![],
        environment: ParserEnvironment::OriginalGlobals,
    };
    let lua = Lua::new();
    let mut budget = Budget::default();
    Lowerer::new(
        &lua,
        body,
        ParserCallbackId(1),
        &callback,
        &parser_bindings(constructor(data)),
        &mut budget,
    )?
    .program(&span(body))
}
fn span(body: &str) -> ItemSourceSpan {
    ItemSourceSpan {
        path: PARSER.into(),
        line: 1,
        end_line: body.lines().count().max(1) as u32,
        sha256: hash(body.as_bytes()),
    }
}
fn constructor(data: &ModifierParserData) -> ParserCallbackId {
    let expected = &data.source.construction_spans["create_mod"];
    ParserCallbackId(
        data.callbacks
            .iter()
            .position(|c| matches!(&c.kind,ParserCallbackKind::Lua{source} if source==expected))
            .unwrap() as u32
            + 1,
    )
}
fn returns(program: &ParserProgram) -> &ParserProgramValueList {
    let ParserProgramStatementKind::Return { values } = &program.body.last().unwrap().operation
    else {
        panic!("return expected")
    };
    values
}

#[test]
fn complete_original_targets_lower_with_exact_provenance_and_own_captures() {
    let owner = bundled_snapshot().unwrap();
    let catalog = owner.modifier_parser();
    let data = catalog.data();
    let source =
        include_str!("../../../../../vendor/path-of-building-poe2/src/Modules/ModParser.lua")
            .replace("\r\n", "\n");
    let lua = Lua::new();
    let mut budget = Budget::default();
    let mut programs = vec![];
    // Source selectors are fixture anchors only, never production dispatch.
    for (line, parameter_count, sha) in [
        (
            2193,
            3,
            "984da9d380a972f615ce9326e0cdc54f112f0ff433626c4236e012fe02ed1c98",
        ),
        (
            3526,
            5,
            "3678a21eb6f7c6485c0b9524327bc521e738e1cff96b95aac02c303ee3a6df11",
        ),
        (
            3589,
            2,
            "d5cb1aa1572261bccddeede66c5f0150aa352e9eacaa8c3241a5e66999f5a99d",
        ),
        (
            3590,
            3,
            "d0e6b410bc4b7e4a908c566a2f4d108883ce7ba95cacf7b40fe34069e3957e13",
        ),
    ] {
        let (index,callback)=data.callbacks.iter().enumerate().find(|(_,c)|matches!(&c.kind,ParserCallbackKind::Lua{source} if source.path==PARSER && source.line==line)).unwrap();
        let ParserCallbackKind::Lua { source: span } = &callback.kind else {
            unreachable!()
        };
        let body = source
            .split_inclusive('\n')
            .skip(span.line as usize - 1)
            .take((span.end_line - span.line + 1) as usize)
            .collect::<String>();
        assert_eq!(hash(body.as_bytes()), span.sha256);
        let program = Lowerer::new(
            &lua,
            &body,
            ParserCallbackId(index as u32 + 1),
            callback,
            &parser_bindings(constructor(data)),
            &mut budget,
        )
        .unwrap()
        .program(span)
        .unwrap();
        assert_eq!(program.parameter_count, parameter_count);
        assert_eq!(program.provenance.function_sha256, sha);
        programs.push(program);
    }
    let callbacks = programs
        .iter()
        .enumerate()
        .map(|(i, p)| (p.callback, ParserProgramId(i as u32 + 1)))
        .collect();
    let typed = ParserProgramCatalog::new(
        ParserProgramData {
            schema_version: PARSER_PROGRAM_SCHEMA_VERSION,
            programs,
            callbacks,
        },
        catalog.clone(),
    )
    .unwrap();
    assert_eq!(typed.data().programs.len(), 4);
    assert!(
        typed
            .required_capabilities()
            .contains(&ParserProgramCapability::PatternFor)
    );
    assert!(
        !typed
            .required_capabilities()
            .contains(&ParserProgramCapability::LegacyPureCalls)
    );
}

#[test]
fn lexical_shadowing_initializers_and_loop_scope_are_preserved() {
    let p = parse("function(a,a) local a=a if a then local hidden=1 end return a end").unwrap();
    let ParserProgramStatementKind::Declare { locals, values } = &p.body[0].operation else {
        panic!()
    };
    assert_eq!(locals, &[2]);
    assert!(matches!(
        values.values[0].operation,
        ParserProgramExprKind::Local { local: 1 }
    ));
    assert!(matches!(
        returns(&p).values[0].operation,
        ParserProgramExprKind::Local { local: 2 }
    ));
    for body in [
        "function() local x=x return x end",
        "function() if false then local x=1 end return x end",
        "function() for i=1,2 do local x=i end return x end",
    ] {
        assert!(parse(body).is_err(), "{body}");
    }
    let p = parse("function() local x=3 for i=1,2 do local x=i x=x+1 end return x end").unwrap();
    assert!(matches!(
        returns(&p).values[0].operation,
        ParserProgramExprKind::Local { local: 0 }
    ));
}

#[test]
fn precedence_and_right_associative_concatenation_are_structural() {
    let p = parse("function(a,b,c) return a or b and c, a .. b .. c, -a*b end").unwrap();
    let values = &returns(&p).values;
    let ParserProgramExprKind::Binary {
        operation: ParserProgramBinary::Or,
        right,
        ..
    } = &values[0].operation
    else {
        panic!()
    };
    assert!(matches!(
        right.operation,
        ParserProgramExprKind::Binary {
            operation: ParserProgramBinary::And,
            ..
        }
    ));
    let ParserProgramExprKind::Binary {
        operation: ParserProgramBinary::Concat,
        right,
        ..
    } = &values[1].operation
    else {
        panic!()
    };
    assert!(matches!(
        right.operation,
        ParserProgramExprKind::Binary {
            operation: ParserProgramBinary::Concat,
            ..
        }
    ));
    let ParserProgramExprKind::Binary {
        operation: ParserProgramBinary::Multiply,
        left,
        ..
    } = &values[2].operation
    else {
        panic!()
    };
    assert!(matches!(
        left.operation,
        ParserProgramExprKind::Unary {
            operation: ParserProgramUnary::Negate,
            ..
        }
    ));
    assert!(parse("function(a,b) return a > b end").is_err());
}

#[test]
fn raw_return_and_literal_tail_packs_keep_parenthesized_adjustment() {
    let p = parse("function(...) return ... end").unwrap();
    assert!(returns(&p).values.is_empty());
    assert!(matches!(
        returns(&p).tail.as_deref(),
        Some(ParserProgramPack::Varargs)
    ));
    let p = parse("function(a) return tonumber(a) end").unwrap();
    assert!(matches!(
        returns(&p).tail.as_deref(),
        Some(ParserProgramPack::Call { .. })
    ));
    let p = parse("function(a) return (tonumber(a)) end").unwrap();
    assert!(returns(&p).tail.is_none());
    assert_eq!(returns(&p).values.len(), 1);
    let p = parse("function(a,b) return {tonumber(a),tonumber(b)} end").unwrap();
    let ParserProgramExprKind::Table { fields } = &returns(&p).values[0].operation else {
        panic!()
    };
    assert!(matches!(fields[0], ParserProgramField::List { .. }));
    assert!(matches!(fields[1], ParserProgramField::Tail { .. }));
    for body in [
        "function(...) return (...) end",
        "function(...) return ...+1 end",
        "function(...) local x=...,1 end",
        "function(...) if ... then return 1 end end",
    ] {
        assert!(parse(body).is_err(), "{body}");
    }
}

#[test]
fn constructors_require_disjoint_static_keys_but_table_sets_stay_dynamic() {
    for body in [
        "function() return {x=1,x=2} end",
        r#"function() return {x=1,["x"]=2} end"#,
        "function(k) return {[k]=1} end",
        "function() return {[1]=2,3} end",
    ] {
        assert!(parse(body).is_err(), "{body}");
    }
    assert!(parse("function(k,v) local t={} t[k]=v return t end").is_ok());
    assert!(parse(r#"function(v) return {label="fixture",v,["other"]=false} end"#).is_ok());
}

#[test]
fn unsupported_syntax_in_dead_branches_rejects_the_whole_function() {
    for body in [
        "function() if false then while true do end end return nil end",
        "function() return nil; local x=1 end",
        "function() if false then local f=function()end end end",
        "function() return nil end return 2",
        "function() break end",
    ] {
        assert!(parse(body).is_err(), "{body}");
    }
    assert!(parse("function() if false then return 1 end return nil end").is_ok());
}

#[test]
fn primitive_bindings_do_not_override_visible_names() {
    assert!(parse("function(tonumber) return tonumber(1) end").is_err());
    assert!(parse("function() local table={} table.insert({},1) end").is_err());
    assert!(parse(r#"function(string,name) return name:gsub("a","b") end"#).is_ok());
    assert!(parse(r#"function(name) return string.gsub(name,"a","b") end"#).is_ok());
    assert!(parse("function(t) for k,v in ipairs(t,tonumber()) do end end").is_err());
}

#[test]
fn lexical_literals_preserve_bytes_and_reject_spaced_compound_tokens() {
    let p = parse(r#"function() return "\255\000",[=[function end]=],0x1.fp2 end"#).unwrap();
    assert!(
        matches!(&returns(&p).values[0].operation,ParserProgramExprKind::Bytes{value} if value==&[255,0])
    );
    assert!(
        matches!(&returns(&p).values[1].operation,ParserProgramExprKind::Literal{value:ParserFactoryLiteral::Text(s)} if s=="function end")
    );
    assert!(
        matches!(&returns(&p).values[2].operation,ParserProgramExprKind::Literal{value:ParserFactoryLiteral::Number(n)} if *n==7.75)
    );
    for body in [
        "function(a,b) return a = = b end",
        "function(a,b) return a . . b end",
        "function() return 1..2 end",
    ] {
        assert!(parse(body).is_err(), "{body}");
    }
    assert!(parse("function(a,b) return a==b, 1 .. 2 end").is_ok());
}

#[test]
fn bounded_source_and_expression_depth_fail_without_running_a_body() {
    let deep = format!("function() return {}true end", "not ".repeat(80));
    assert!(parse(&deep).is_err());
    let large = format!("function() return {:?} end", "x".repeat(4097));
    assert!(parse(&large).is_err());
    let large = format!("function() --{}\nend", "x".repeat(65536));
    assert!(parse(&large).is_err());
}
