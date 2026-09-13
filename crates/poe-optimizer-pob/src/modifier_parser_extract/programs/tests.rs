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
        &parser_bindings(data, constructor(data)).map_err(|e| e.to_string())?,
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
            &parser_bindings(data, constructor(data)).unwrap(),
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

fn with_captures(
    body: &str,
    captures: Vec<ParserUpvalue>,
    authorization: &LoweringBindings,
) -> LowerResult<ParserProgram> {
    let callback = ParserCallback {
        kind: ParserCallbackKind::Lua { source: span(body) },
        upvalues: captures,
        environment: ParserEnvironment::OriginalGlobals,
    };
    Lowerer::new(
        &Lua::new(),
        body,
        ParserCallbackId(1),
        &callback,
        authorization,
        &mut Budget::default(),
    )?
    .program(&span(body))
}

#[test]
fn first_to_upper_requires_captured_identity_but_allows_lexical_aliases() {
    let owner = bundled_snapshot().unwrap();
    let data = owner.modifier_parser().data();
    let upper = data.helpers["firstToUpper"];
    let authorization = parser_bindings(data, constructor(data)).unwrap();
    for name in ["firstToUpper", "captured_alias"] {
        let body = format!("function(value) return {name}(value) end");
        let program = with_captures(
            &body,
            vec![
                ParserUpvalue {
                    name: "unrelated".into(),
                    value: ParserValue::Nil,
                },
                ParserUpvalue {
                    name: name.into(),
                    value: ParserValue::Callback(upper),
                },
            ],
            &authorization,
        )
        .unwrap();
        assert_eq!(
            program.bindings,
            vec![ParserProgramBinding::Intrinsic {
                operation: ParserProgramIntrinsic::FirstToUpper,
                source: ParserProgramIntrinsicSource::Captured {
                    upvalue: 1,
                    callback: upper
                },
            }]
        );
    }
    assert!(
        with_captures(
            "function(v) return firstToUpper(v) end",
            vec![],
            &authorization
        )
        .is_err()
    );
    assert!(
        with_captures(
            "function(v) return string.upper(v) end",
            vec![],
            &authorization
        )
        .is_err()
    );
    assert!(
        with_captures(
            "function(v) return string.upper end",
            vec![],
            &authorization
        )
        .is_err()
    );
    assert!(
        with_captures(
            "function(firstToUpper) return firstToUpper('a') end",
            vec![ParserUpvalue {
                name: "firstToUpper".into(),
                value: ParserValue::Callback(upper)
            },],
            &authorization
        )
        .is_err()
    );
    assert!(
        with_captures(
            "function(v) return firstToUpper(v) end",
            vec![ParserUpvalue {
                name: "firstToUpper".into(),
                value: ParserValue::Nil
            },],
            &authorization
        )
        .is_err()
    );
    let other = data.helpers["flag"];
    let program = with_captures(
        "function(v) return firstToUpper(v) end",
        vec![ParserUpvalue {
            name: "firstToUpper".into(),
            value: ParserValue::Callback(other),
        }],
        &authorization,
    )
    .unwrap();
    assert_eq!(
        program.bindings,
        vec![ParserProgramBinding::CapturedCallback {
            upvalue: 0,
            callback: other,
        }]
    );
}

#[test]
fn first_to_upper_binding_rejects_missing_or_unclosed_helper_records() {
    let owner = bundled_snapshot().unwrap();
    let data = owner.modifier_parser().data();
    let ctor = constructor(data);
    let upper = data.helpers["firstToUpper"];
    for case in 0..7 {
        let mut changed = data.clone();
        match case {
            0 => {
                changed.helpers.remove("firstToUpper");
            }
            1 => {
                changed
                    .helpers
                    .insert("firstToUpper".into(), ParserCallbackId(u32::MAX));
            }
            2 => {
                changed
                    .source
                    .construction_spans
                    .remove("first_to_upper_primitive");
            }
            3 => {
                changed
                    .source
                    .construction_spans
                    .get_mut("first_to_upper_primitive")
                    .unwrap()
                    .end_line += 1;
            }
            4 => {
                changed.callbacks[upper.0 as usize - 1].kind = ParserCallbackKind::Builtin {
                    symbol: "string.upper".into(),
                };
            }
            5 => {
                changed.callbacks[upper.0 as usize - 1]
                    .upvalues
                    .push(ParserUpvalue {
                        name: "string".into(),
                        value: ParserValue::Nil,
                    });
            }
            6 => {
                changed.helpers.insert("firstToUpper".into(), ctor);
            }
            _ => unreachable!(),
        }
        assert!(parser_bindings(&changed, ctor).is_err(), "case {case}");
    }
    assert!(parser_bindings(data, upper).is_err());
}

#[test]
fn original_explosion_and_caller_lower_without_changing_flag_or_constructor() {
    let owner = bundled_snapshot().unwrap();
    let catalog = owner.modifier_parser();
    let data = catalog.data();
    let upper = data.helpers["firstToUpper"];
    let authorization = parser_bindings(data, constructor(data)).unwrap();
    let mut previous = parser_bindings(data, constructor(data)).unwrap();
    previous.intrinsics.remove(&upper);
    assert_eq!(
        authorization.intrinsics.get(&constructor(data)),
        Some(&ParserProgramIntrinsic::CreateMod)
    );
    assert_eq!(
        authorization.intrinsics.get(&constructor(data)),
        previous.intrinsics.get(&constructor(data))
    );
    let sources: BTreeMap<String, String> = BTreeMap::from([
        (
            PARSER.into(),
            include_str!("../../../../../vendor/path-of-building-poe2/src/Modules/ModParser.lua")
                .replace("\r\n", "\n"),
        ),
        (
            TOOLS.into(),
            include_str!("../../../../../vendor/path-of-building-poe2/src/Modules/ModTools.lua")
                .replace("\r\n", "\n"),
        ),
    ]);
    let by_line = |line| {
        ParserCallbackId(data.callbacks.iter().position(|c|
        matches!(&c.kind, ParserCallbackKind::Lua { source } if source.path == PARSER && source.line == line)
    ).unwrap() as u32 + 1)
    };
    let explosion = by_line(2255);
    let caller = by_line(2336);
    let mut programs = vec![];
    for id in [constructor(data), data.helpers["flag"], explosion, caller] {
        let callback = &data.callbacks[id.0 as usize - 1];
        let ParserCallbackKind::Lua { source: origin } = &callback.kind else {
            panic!()
        };
        let body = sources[&origin.path]
            .split_inclusive('\n')
            .skip(origin.line as usize - 1)
            .take((origin.end_line - origin.line + 1) as usize)
            .collect::<String>();
        assert_eq!(hash(body.as_bytes()), origin.sha256);
        let lua = Lua::new();
        let lower = |bindings| {
            Lowerer::new(&lua, &body, id, callback, bindings, &mut Budget::default())
                .and_then(|lowerer| lowerer.program(origin))
        };
        if id == constructor(data) {
            // CreateMod is an existing authenticated captured intrinsic, not a
            // standalone source program. Adding FirstToUpper must not admit its
            // ordinary global select call or change that constructor contract.
            assert_eq!(
                lower(&authorization).unwrap_err(),
                "unsupported global call select"
            );
            assert_eq!(lower(&authorization), lower(&previous));
            continue;
        }
        let program = lower(&authorization).unwrap();
        let old = lower(&previous).unwrap();
        if id == data.helpers["flag"] || id == explosion {
            assert!(program.bindings.iter().any(|binding| matches!(binding,
                ParserProgramBinding::Intrinsic {
                    operation: ParserProgramIntrinsic::CreateMod,
                    source: ParserProgramIntrinsicSource::Captured { callback, .. },
                } if *callback == constructor(data)
            )));
        }
        if id == explosion {
            assert!(program.bindings.iter().any(|b| matches!(b,
                ParserProgramBinding::Intrinsic { operation: ParserProgramIntrinsic::FirstToUpper,
                    source: ParserProgramIntrinsicSource::Captured { callback, .. } } if *callback == upper
            )));
            assert!(old.bindings.iter().any(|b| matches!(b,
                ParserProgramBinding::CapturedCallback { callback, .. } if *callback == upper
            )));
        } else {
            assert_eq!(program, old, "unrelated original program changed: {id:?}");
        }
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
    assert_eq!(typed.data().programs.len(), 3);
    assert!(
        !typed
            .required_capabilities()
            .contains(&ParserProgramCapability::LegacyPureCalls)
    );
}

#[test]
fn acquisition_first_to_upper_source_and_original_primitives_stay_guarded() {
    let text =
        include_str!("../../../../../vendor/path-of-building-poe2/src/Modules/ModParser.lua")
            .replace("\r\n", "\n");
    for (from, to) in [
        (
            r#"return (str:gsub("^%l", string.upper))"#,
            r#"return str:gsub("^%l", string.upper)"#,
        ),
        ("string.upper))", "string.lower))"),
        ("firstToUpper(str)", "firstToUpper(str, extra)"),
    ] {
        let changed = text.replacen(from, to, 1);
        assert_ne!(changed, text);
        assert!(
            strings::source_policy(
                &Lua::new(),
                &BTreeMap::from([(PARSER.into(), changed)]),
                &mut BTreeMap::new()
            )
            .is_err()
        );
    }
    for code in [
        "string = {}",
        "string.upper = string.lower",
        "string.gsub = string.upper",
        "getmetatable('').__index = {}",
    ] {
        let lua = Lua::new();
        let original = strings::StringLibrary::capture(&lua).unwrap();
        lua.load(code).exec().unwrap();
        assert!(original.verify(&lua).is_err(), "{code}");
    }
}

#[test]
fn acquisition_first_to_upper_live_function_and_environment_stay_guarded() {
    let owner = bundled_snapshot().unwrap();
    let data = owner.modifier_parser().data();
    let upper = data.helpers["firstToUpper"];
    let descriptor = &data.callbacks[upper.0 as usize - 1];
    let origin = &data.source.construction_spans["first_to_upper_primitive"];
    let source =
        include_str!("../../../../../vendor/path-of-building-poe2/src/Modules/ModParser.lua")
            .replace("\r\n", "\n");
    let body = source
        .split_inclusive('\n')
        .skip(origin.line as usize - 1)
        .take((origin.end_line - origin.line + 1) as usize)
        .collect::<String>();
    assert_eq!(hash(body.as_bytes()), origin.sha256);
    let lua = Lua::new();
    let chunk = format!(
        "{}{}\nreturn firstToUpper",
        "\n".repeat(origin.line as usize - 1),
        body
    );
    // Only constructs the fixture's function; its body is not invoked.
    let function: Function = lua
        .load(&chunk)
        .set_name(format!("@{PARSER}"))
        .eval()
        .unwrap();
    let observed = BTreeMap::from([(function.to_pointer() as usize, upper)]);
    strings::verify_helper(&lua, &function, upper, descriptor, &observed, origin).unwrap();
    assert!(parser_bindings(data, constructor(data)).is_ok());
    let other: Function = lua
        .load(&chunk)
        .set_name(format!("@{PARSER}"))
        .eval()
        .unwrap();
    assert_ne!(function, other);
    assert!(strings::verify_helper(&lua, &other, upper, descriptor, &observed, origin).is_err());
    function
        .set_environment(lua.create_table().unwrap())
        .unwrap();
    assert!(strings::verify_helper(&lua, &function, upper, descriptor, &observed, origin).is_err());
}
