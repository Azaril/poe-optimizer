//! Standalone value-call contracts and interpreted LuaJIT primitive comparisons.
#![cfg(not(target_arch = "wasm32"))]
use mlua::{Lua, MultiValue, Value};
use poe_optimizer_data::{
    item_loading::{ItemLoadingSource, ItemSourceSpan},
    modifier_parser::*,
    source_program::*,
};
use poe_optimizer_engine::{lua_pattern::MatchLimits, source_program::*};
use std::collections::BTreeMap;
fn e(operation: ParserProgramExprKind) -> ParserProgramExpr {
    ParserProgramExpr {
        location: ParserProgramLocation { start: 0, end: 1 },
        operation,
    }
}
fn l(local: u16) -> ParserProgramExpr {
    e(ParserProgramExprKind::Local { local })
}
fn b(value: &str) -> ParserProgramExpr {
    e(ParserProgramExprKind::Bytes {
        value: value.as_bytes().to_vec(),
    })
}
fn n(value: f64) -> ParserProgramExpr {
    e(ParserProgramExprKind::Literal {
        value: ParserFactoryLiteral::Number(value),
    })
}
fn get(table: ParserProgramExpr, key: &str) -> ParserProgramExpr {
    e(ParserProgramExprKind::Get {
        table: Box::new(table),
        key: Box::new(b(key)),
    })
}
fn s(operation: ParserProgramStatementKind) -> ParserProgramStatement {
    ParserProgramStatement {
        location: ParserProgramLocation { start: 0, end: 1 },
        operation,
    }
}
fn list(values: Vec<ParserProgramExpr>) -> ParserProgramValueList {
    ParserProgramValueList { values, tail: None }
}
fn ret(values: Vec<ParserProgramExpr>) -> ParserProgramStatement {
    s(ParserProgramStatementKind::Return {
        values: list(values),
    })
}
fn call(
    binding: u16,
    receiver: Option<ParserProgramExpr>,
    arguments: ParserProgramValueList,
) -> ParserProgramCall {
    ParserProgramCall {
        binding,
        receiver: receiver.map(Box::new),
        arguments,
    }
}
fn tail(call: ParserProgramCall) -> ParserProgramStatement {
    s(ParserProgramStatementKind::Return {
        values: ParserProgramValueList {
            values: vec![],
            tail: Some(Box::new(ParserProgramPack::Call { call })),
        },
    })
}
type Body = (
    u32,
    u16,
    bool,
    Vec<ParserProgramBinding>,
    Vec<ParserProgramStatement>,
);
fn compile(bodies: Vec<Body>, captures: Vec<Vec<ParserUpvalue>>) -> CompiledSourcePrograms {
    let path = "src/Modules/ValueFixture.lua".to_owned();
    let sha = "a".repeat(64);
    let span = ItemSourceSpan {
        path: path.clone(),
        line: 1,
        end_line: 10,
        sha256: sha.clone(),
    };
    let callbacks = captures
        .into_iter()
        .map(|upvalues| ParserCallback {
            kind: ParserCallbackKind::Lua {
                source: span.clone(),
            },
            environment: ParserEnvironment::OriginalGlobals,
            upvalues,
        })
        .collect();
    let owner = SourceProgramOwner::new(SourceProgramDefinitions {
        schema_version: SOURCE_PROGRAM_DEFINITIONS_SCHEMA_VERSION,
        source: ItemLoadingSource {
            upstream_revision: "b".repeat(40),
            files: BTreeMap::from([(path.clone(), sha)]),
            construction_spans: BTreeMap::new(),
            module_order: vec![path],
        },
        tables: vec![],
        callbacks,
        roots: vec![],
        intrinsics: BTreeMap::new(),
    })
    .unwrap();
    let programs = bodies
        .into_iter()
        .map(|(id, parameters, variadic, bindings, body)| ParserProgram {
            callback: ParserCallbackId(id),
            parameter_count: parameters,
            local_count: parameters,
            variadic,
            bindings,
            body,
            provenance: ParserProgramProvenance {
                source: span.clone(),
                function_start: 0,
                function_end: 128,
                function_sha256: "c".repeat(64),
            },
        })
        .collect::<Vec<_>>();
    let callbacks = programs
        .iter()
        .enumerate()
        .map(|(i, p)| (p.callback, ParserProgramId(i as u32 + 1)))
        .collect();
    CompiledSourcePrograms::new(
        &SourceProgramCatalog::new(
            ParserProgramData {
                schema_version: PARSER_PROGRAM_SCHEMA_VERSION,
                programs,
                callbacks,
            },
            owner,
        )
        .unwrap(),
    )
    .unwrap()
}
fn primitive(operation: ParserProgramIntrinsic) -> CompiledSourcePrograms {
    compile(
        vec![(
            1,
            0,
            true,
            vec![ParserProgramBinding::Intrinsic {
                operation,
                source: ParserProgramIntrinsicSource::OriginalGlobal,
            }],
            vec![tail(call(
                0,
                None,
                ParserProgramValueList {
                    values: vec![],
                    tail: Some(Box::new(ParserProgramPack::Varargs)),
                },
            ))],
        )],
        vec![vec![]],
    )
}
fn graph(values: Vec<ProgramValue>) -> ProgramValueGraph {
    ProgramValueGraph {
        values,
        tables: vec![],
    }
}
fn text(value: &str) -> ProgramValue {
    ProgramValue::Bytes(value.as_bytes().to_vec())
}
fn lua_value(lua: &Lua, value: &ProgramValue) -> Value {
    match value {
        ProgramValue::Nil => Value::Nil,
        ProgramValue::Boolean(v) => Value::Boolean(*v),
        ProgramValue::Number(v) => Value::Number(*v),
        ProgramValue::Bytes(v) => Value::String(lua.create_string(v).unwrap()),
        _ => panic!("scalar source input"),
    }
}
fn scalar(value: Value) -> ProgramValue {
    match value {
        Value::Nil => ProgramValue::Nil,
        Value::Boolean(v) => ProgramValue::Boolean(v),
        Value::Integer(v) => ProgramValue::Number(v as f64),
        Value::Number(v) => ProgramValue::Number(v),
        Value::String(v) => ProgramValue::Bytes(v.as_bytes().to_vec()),
        _ => panic!("scalar source output"),
    }
}
fn assert_values(actual: &[ProgramValue], expected: &[ProgramValue]) {
    assert_eq!(actual.len(), expected.len());
    for (a, e) in actual.iter().zip(expected) {
        match (a, e) {
            (ProgramValue::Number(a), ProgramValue::Number(e)) => {
                assert_eq!(a.to_bits(), e.to_bits())
            }
            _ => assert_eq!(a, e),
        }
    }
}
fn compare(operation: ParserProgramIntrinsic, path: &str, cases: Vec<Vec<ProgramValue>>) {
    let lua = Lua::new();
    lua.load("jit.off(); jit.flush(); assert(not jit.status())")
        .exec()
        .unwrap();
    let source: mlua::Function = lua
        .load(format!("return function(...) return {path}(...) end"))
        .eval()
        .unwrap();
    let native = primitive(operation);
    for values in cases {
        let result = source.call::<MultiValue>(MultiValue::from_vec(
            values.iter().map(|v| lua_value(&lua, v)).collect(),
        ));
        let actual = native.execute(
            ParserCallbackId(1),
            &graph(values.clone()),
            ProgramLimits::default(),
        );
        match (result, actual) {
            (Ok(expected), Ok(actual)) => assert_values(
                &actual.graph().values,
                &expected.into_iter().map(scalar).collect::<Vec<_>>(),
            ),
            (Err(_), Err(actual)) => assert_eq!(
                actual.kind,
                ProgramRuntimeErrorKind::Source,
                "{path}: {values:?}"
            ),
            (source, native) => panic!("{path} {values:?}: source={source:?} native={native:?}"),
        }
    }
}
#[test]
fn math_extrema_match_interpreted_luajit_coercion_nan_and_signed_zero() {
    let nan = f64::from_bits(0xfff8_0000_0000_0000);
    let cases = vec![
        vec![],
        vec![ProgramValue::Nil],
        vec![ProgramValue::Boolean(false)],
        vec![text("bad")],
        vec![text(" 0x10 "), ProgramValue::Number(2.5), text("-4")],
        vec![ProgramValue::Number(1.0), text("bad"), ProgramValue::Nil],
        vec![ProgramValue::Number(-0.0), ProgramValue::Number(0.0)],
        vec![ProgramValue::Number(0.0), ProgramValue::Number(-0.0)],
        vec![ProgramValue::Number(nan)],
        vec![ProgramValue::Number(nan), ProgramValue::Number(7.0)],
        vec![ProgramValue::Number(7.0), ProgramValue::Number(nan)],
        vec![
            ProgramValue::Number(f64::NEG_INFINITY),
            ProgramValue::Number(f64::INFINITY),
        ],
        vec![
            ProgramValue::Number(-1.0),
            text("1e3"),
            ProgramValue::Number(2.0),
        ],
    ];
    compare(ParserProgramIntrinsic::MathMin, "math.min", cases.clone());
    compare(ParserProgramIntrinsic::MathMax, "math.max", cases);
}
#[test]
fn tostring_matches_scalar_luajit_outputs_and_ignores_additional_values() {
    let mut cases = vec![
        vec![],
        vec![ProgramValue::Nil],
        vec![ProgramValue::Boolean(false)],
        vec![ProgramValue::Boolean(true)],
        vec![ProgramValue::Bytes(vec![0, 255, b'a'])],
        vec![ProgramValue::Number(13.0), ProgramValue::Boolean(false)],
    ];
    for n in [
        0.0,
        -0.0,
        1.2345678901234567,
        1e-4,
        1e-5,
        1e13,
        1e14,
        f64::MIN_POSITIVE,
        f64::MAX,
        f64::from_bits(1),
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::from_bits(0xfff8_0000_0000_0000),
    ] {
        cases.push(vec![ProgramValue::Number(n)]);
    }
    compare(ParserProgramIntrinsic::ToString, "tostring", cases);
    let native = primitive(ParserProgramIntrinsic::ToString);
    let input = ProgramValueGraph {
        values: vec![ProgramValue::Table(ProgramTableId(1))],
        tables: vec![ProgramTable::default()],
    };
    assert_eq!(
        native
            .execute(ParserCallbackId(1), &input, ProgramLimits::default())
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
}
#[test]
fn string_match_preserves_result_packs_byte_captures_initial_position_and_error_order() {
    let mut cases = vec![
        vec![],
        vec![text("abc")],
        vec![text("a42b"), text("(%a)(%d+)()")],
        vec![text("abc"), text("z")],
        vec![text("abc"), text(".")],
        vec![text("abc"), text("^."), ProgramValue::Number(2.0)],
        vec![text("abc"), text(".$"), ProgramValue::Number(-1.0)],
        vec![text("abc"), text("()$"), ProgramValue::Number(99.0)],
        vec![text("abc"), text("()"), ProgramValue::Number(-99.0)],
        vec![text("abc"), text("[%"), ProgramValue::Boolean(false)],
        vec![text("abc"), text("[%")],
        vec![text("abc"), text("(.)"), text("2.9")],
        vec![ProgramValue::Number(1234.0), ProgramValue::Number(23.0)],
        vec![ProgramValue::Boolean(false), text("[%")],
    ];
    cases.push(vec![
        ProgramValue::Bytes(vec![0, 255, b'a']),
        text("(..)()"),
    ]);
    compare(ParserProgramIntrinsic::StringMatch, "string.match", cases);
    let native = primitive(ParserProgramIntrinsic::StringMatch);
    let input = graph(vec![text("abc"), text("()()()")]);
    assert_eq!(
        native
            .execute(
                ParserCallbackId(1),
                &input,
                ProgramLimits {
                    max_results: 2,
                    ..ProgramLimits::default()
                }
            )
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
}
fn dynamic() -> CompiledSourcePrograms {
    let mutate = call(1, None, list(vec![l(1)]));
    let dynamic = call(
        0,
        Some(get(l(0), "apply")),
        list(vec![e(ParserProgramExprKind::Call {
            call: Box::new(mutate),
        })]),
    );
    compile(
        vec![
            (
                1,
                2,
                false,
                vec![
                    ParserProgramBinding::DynamicCall {},
                    ParserProgramBinding::CapturedCallback {
                        upvalue: 0,
                        callback: ParserCallbackId(3),
                    },
                ],
                vec![tail(dynamic)],
            ),
            (
                2,
                1,
                false,
                vec![],
                vec![ret(vec![
                    l(0),
                    e(ParserProgramExprKind::Literal {
                        value: ParserFactoryLiteral::Nil,
                    }),
                ])],
            ),
            (
                3,
                1,
                false,
                vec![],
                vec![
                    s(ParserProgramStatementKind::TableSet {
                        table: l(0),
                        key: b("flag"),
                        value: n(1.0),
                    }),
                    s(ParserProgramStatementKind::TableSet {
                        table: get(l(0), "target"),
                        key: b("apply"),
                        value: e(ParserProgramExprKind::Capture { upvalue: 0 }),
                    }),
                    ret(vec![n(17.0)]),
                ],
            ),
            (4, 0, false, vec![], vec![ret(vec![n(99.0)])]),
        ],
        vec![
            vec![ParserUpvalue {
                name: "mutate".into(),
                value: ParserValue::Callback(ParserCallbackId(3)),
            }],
            vec![],
            vec![ParserUpvalue {
                name: "replacement".into(),
                value: ParserValue::Callback(ParserCallbackId(4)),
            }],
            vec![],
        ],
    )
}
fn state(target: ProgramValue) -> ProgramValueGraph {
    ProgramValueGraph {
        values: vec![
            ProgramValue::Table(ProgramTableId(1)),
            ProgramValue::Table(ProgramTableId(2)),
        ],
        tables: vec![
            ProgramTable {
                entries: vec![(text("apply"), target)],
            },
            ProgramTable {
                entries: vec![
                    (text("flag"), ProgramValue::Number(0.0)),
                    (text("target"), ProgramValue::Table(ProgramTableId(1))),
                ],
            },
        ],
    }
}
fn flag(session: &mut ProgramSession, values: &[SessionValue]) -> ProgramValue {
    let out = session.snapshot(&values[1..]).unwrap();
    out.graph().tables[0]
        .entries
        .iter()
        .find(|(k, _)| *k == text("flag"))
        .unwrap()
        .1
        .clone()
}
#[test]
fn dynamic_value_calls_resolve_before_arguments_without_implicit_self_or_pack_loss() {
    let library = dynamic();
    let (mut session, values) = library
        .session(
            &state(ProgramValue::Callback(ParserCallbackId(2))),
            ProgramLimits::default(),
        )
        .unwrap();
    let result = session.invoke(ParserCallbackId(1), &values).unwrap();
    assert_values(
        &session.snapshot(&result).unwrap().graph().values,
        &[ProgramValue::Number(17.0), ProgramValue::Nil],
    );
    assert_eq!(flag(&mut session, &values), ProgramValue::Number(1.0));
    let result = session.invoke(ParserCallbackId(1), &values).unwrap();
    assert_values(
        &session.snapshot(&result).unwrap().graph().values,
        &[ProgramValue::Number(99.0)],
    );
    let lua = Lua::new();
    lua.load("jit.off(); jit.flush()").exec().unwrap();
    let result:MultiValue=lua.load("local target={apply=function(x) return x,nil end}; local function mutate() target.apply=function() return 99 end; return 17 end; return target.apply(mutate())").eval().unwrap();
    assert_values(
        &result.into_iter().map(scalar).collect::<Vec<_>>(),
        &[ProgramValue::Number(17.0), ProgramValue::Nil],
    );
}
#[test]
fn dynamic_noncallable_failure_follows_argument_effects_but_index_failure_precedes_them() {
    let library = dynamic();
    let (mut session, values) = library
        .session(
            &state(ProgramValue::Boolean(false)),
            ProgramLimits::default(),
        )
        .unwrap();
    assert_eq!(
        session
            .invoke(ParserCallbackId(1), &values)
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::Source
    );
    assert_eq!(flag(&mut session, &values), ProgramValue::Number(1.0));
    let steps = session.steps();
    let (mut session, values) = library
        .session(
            &state(ProgramValue::Callback(ParserCallbackId(2))),
            ProgramLimits::default(),
        )
        .unwrap();
    let nil = session
        .borrow(&graph(vec![ProgramValue::Nil]))
        .unwrap()
        .remove(0);
    assert_eq!(
        session
            .invoke(ParserCallbackId(1), &[nil, values[1].clone()])
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::Source
    );
    assert_eq!(flag(&mut session, &values), ProgramValue::Number(0.0));
    assert!(session.steps() < steps);
}
#[test]
fn callable_handles_preserve_session_ownership_and_cumulative_pattern_limits() {
    let library = dynamic();
    let (mut session, _) = library
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    let callback = session
        .borrow(&graph(vec![
            ProgramValue::Callback(ParserCallbackId(2)),
            ProgramValue::Number(5.0),
        ]))
        .unwrap();
    let result = session
        .invoke_callable(&callback[0], &callback[1..])
        .unwrap();
    assert_values(
        &session.snapshot(&result).unwrap().graph().values,
        &[ProgramValue::Number(5.0), ProgramValue::Nil],
    );
    let (mut foreign, _) = library
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    assert_eq!(
        foreign.invoke_callable(&callback[0], &[]).unwrap_err().kind,
        ProgramRuntimeErrorKind::InvalidInput
    );
    let library = primitive(ParserProgramIntrinsic::MathMin);
    let (mut session, _) = library
        .session(
            &ProgramValueGraph::default(),
            ProgramLimits {
                pattern: MatchLimits {
                    max_steps: 4,
                    ..MatchLimits::default()
                },
                ..ProgramLimits::default()
            },
        )
        .unwrap();
    let values = session.borrow(&graph(vec![text("123")])).unwrap();
    session.invoke(ParserCallbackId(1), &values).unwrap();
    assert_eq!(
        session
            .invoke(ParserCallbackId(1), &values)
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
}

#[test]
fn dynamic_callable_values_dispatch_registered_original_intrinsics() {
    let library = dynamic();
    let mut definitions = library.catalog().owner().definitions().unwrap().clone();
    definitions.callbacks.push(ParserCallback {
        kind: ParserCallbackKind::Builtin {
            symbol: "math.min".into(),
        },
        environment: ParserEnvironment::OriginalGlobals,
        upvalues: vec![],
    });
    definitions
        .intrinsics
        .insert(ParserCallbackId(5), ParserProgramIntrinsic::MathMin);
    let owner = SourceProgramOwner::new(definitions).unwrap();
    let library = CompiledSourcePrograms::new(
        &SourceProgramCatalog::new(library.catalog().data().clone(), owner).unwrap(),
    )
    .unwrap();
    let (mut session, _) = library
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    let values = session
        .borrow(&graph(vec![
            ProgramValue::Callback(ParserCallbackId(5)),
            text("12"),
            ProgramValue::Number(9.0),
        ]))
        .unwrap();
    let result = session.invoke_callable(&values[0], &values[1..]).unwrap();
    assert_values(
        &session.snapshot(&result).unwrap().graph().values,
        &[ProgramValue::Number(9.0)],
    );
}
#[test]
fn string_match_method_uses_string_primitive_or_actual_table_override() {
    let library = compile(
        vec![
            (
                1,
                2,
                false,
                vec![ParserProgramBinding::DynamicMethod {
                    key: "match".into(),
                }],
                vec![tail(call(0, Some(l(0)), list(vec![l(1)])))],
            ),
            (2, 2, false, vec![], vec![ret(vec![l(1)])]),
        ],
        vec![vec![], vec![]],
    );
    let result = library
        .execute(
            ParserCallbackId(1),
            &graph(vec![text("a23b"), text("(%d+)")]),
            ProgramLimits::default(),
        )
        .unwrap();
    assert_values(&result.graph().values, &[text("23")]);
    let input = ProgramValueGraph {
        values: vec![
            ProgramValue::Table(ProgramTableId(1)),
            text("literal pattern"),
        ],
        tables: vec![ProgramTable {
            entries: vec![(text("match"), ProgramValue::Callback(ParserCallbackId(2)))],
        }],
    };
    let result = library
        .execute(ParserCallbackId(1), &input, ProgramLimits::default())
        .unwrap();
    assert_values(&result.graph().values, &[text("literal pattern")]);
}

#[path = "support/source_program_arithmetic.rs"]
mod arithmetic;
