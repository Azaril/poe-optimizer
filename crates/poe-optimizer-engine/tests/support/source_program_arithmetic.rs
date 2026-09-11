//! Authored-IR numeric contracts against actual interpreted LuaJIT operations.
use super::*;

fn binary(
    operation: ParserProgramBinary,
    left: ParserProgramExpr,
    right: ParserProgramExpr,
) -> ParserProgramExpr {
    e(ParserProgramExprKind::Binary {
        operation,
        left: Box::new(left),
        right: Box::new(right),
    })
}
fn arithmetic(operation: ParserProgramBinary) -> CompiledSourcePrograms {
    compile(
        vec![(
            1,
            2,
            false,
            vec![],
            vec![ret(vec![binary(operation, l(0), l(1))])],
        )],
        vec![vec![]],
    )
}
fn assert_number(actual: &ProgramValue, expected: &ProgramValue) {
    match (actual, expected) {
        (ProgramValue::Number(a), ProgramValue::Number(b)) if a.is_nan() && b.is_nan() => {}
        _ => assert_values(std::slice::from_ref(actual), std::slice::from_ref(expected)),
    }
}

#[test]
fn floor_preserves_luajit_coercion_signed_zero_nonfinite_and_one_result() {
    let mut cases = vec![
        vec![],
        vec![ProgramValue::Nil],
        vec![ProgramValue::Boolean(false)],
        vec![text("bad")],
        vec![text("1\0")],
        vec![text(" 0x1.8p1 ")],
        vec![text("0b101")],
        vec![text("-0")],
        vec![text("inf")],
        vec![text("-infinity")],
        vec![text("nan")],
        vec![
            ProgramValue::Number(2.75),
            text("ignored"),
            ProgramValue::Nil,
        ],
    ];
    for value in [
        0.0,
        -0.0,
        f64::from_bits(1),
        -f64::from_bits(1),
        0.5,
        -0.5,
        1.0,
        -1.0,
        f64::from_bits(1.0f64.to_bits() - 1),
        f64::from_bits(1.0f64.to_bits() + 1),
        4503599627370495.5,
        4503599627370496.0,
        -4503599627370495.5,
        f64::MAX,
        -f64::MAX,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::from_bits(0xfff8_0000_0000_0000),
    ] {
        cases.push(vec![ProgramValue::Number(value)]);
    }
    compare(ParserProgramIntrinsic::MathFloor, "math.floor", cases);
}

#[test]
fn power_matches_luajit_scalar_bits_coercion_overflow_underflow_and_domain_results() {
    let lua = Lua::new();
    lua.load("jit.off(); jit.flush(); assert(not jit.status())")
        .exec()
        .unwrap();
    let source: mlua::Function = lua
        .load("return function(a,b) return a ^ b end")
        .eval()
        .unwrap();
    let native = arithmetic(ParserProgramBinary::Power);
    let mut cases = Vec::new();
    for a in [
        0.0,
        -0.0,
        1.0,
        -1.0,
        2.0,
        -2.0,
        10.0,
        0.1,
        f64::from_bits(1),
        f64::MIN_POSITIVE,
        f64::MAX,
        f64::from_bits(1.0f64.to_bits() - 1),
        f64::from_bits(1.0f64.to_bits() + 1),
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
    ] {
        for b in [
            0.0,
            -0.0,
            1.0,
            -1.0,
            2.0,
            -2.0,
            3.0,
            -3.0,
            0.5,
            -0.5,
            308.0,
            309.0,
            -323.0,
            -324.0,
            9007199254740992.0,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
        ] {
            cases.push(vec![ProgramValue::Number(a), ProgramValue::Number(b)]);
        }
    }
    cases.extend([
        vec![],
        vec![ProgramValue::Number(2.0)],
        vec![text(" 0x2 "), text("0b11")],
        vec![text("-0"), text("-3")],
        vec![text("nan"), text("0")],
        vec![ProgramValue::Boolean(false), ProgramValue::Number(2.0)],
        vec![ProgramValue::Number(2.0), ProgramValue::Boolean(false)],
        vec![text("bad"), text("bad")],
        vec![text("2\0"), text("3")],
    ]);
    for values in cases {
        let expected = source.call::<MultiValue>(MultiValue::from_vec(
            values.iter().map(|v| lua_value(&lua, v)).collect(),
        ));
        let actual = native.execute(
            ParserCallbackId(1),
            &graph(values.clone()),
            ProgramLimits::default(),
        );
        match (expected, actual) {
            (Ok(expected), Ok(actual)) => {
                assert_eq!(expected.len(), 1);
                assert_eq!(actual.graph().values.len(), 1);
                let expected = scalar(expected.into_iter().next().unwrap());
                assert_number(&actual.graph().values[0], &expected);
            }
            (Err(_), Err(actual)) => {
                assert_eq!(actual.kind, ProgramRuntimeErrorKind::Source, "{values:?}")
            }
            (expected, actual) => panic!("{values:?}: source={expected:?}, native={actual:?}"),
        }
    }
}

fn ordered(floor: bool) -> CompiledSourcePrograms {
    let mark = |key: &str, value| {
        e(ParserProgramExprKind::Call {
            call: Box::new(call(
                u16::from(floor),
                None,
                list(vec![l(0), b(key), value]),
            )),
        })
    };
    let left = mark("L", l(1));
    let right = mark("R", l(2));
    let mut bindings = Vec::new();
    let body = if floor {
        bindings.push(ParserProgramBinding::Intrinsic {
            operation: ParserProgramIntrinsic::MathFloor,
            source: ParserProgramIntrinsicSource::OriginalGlobal,
        });
        tail(call(0, None, list(vec![left, right])))
    } else {
        ret(vec![binary(ParserProgramBinary::Power, left, right)])
    };
    bindings.push(ParserProgramBinding::CapturedCallback {
        upvalue: 0,
        callback: ParserCallbackId(2),
    });
    compile(
        vec![
            (1, 3, false, bindings, vec![body]),
            (
                2,
                3,
                false,
                vec![],
                vec![
                    s(ParserProgramStatementKind::TableSet {
                        table: l(0),
                        key: b("log"),
                        value: binary(ParserProgramBinary::Concat, get(l(0), "log"), l(1)),
                    }),
                    ret(vec![l(2)]),
                ],
            ),
        ],
        vec![
            vec![ParserUpvalue {
                name: "mark".into(),
                value: ParserValue::Callback(ParserCallbackId(2)),
            }],
            vec![],
        ],
    )
}

#[test]
fn numeric_operands_and_extra_floor_arguments_finish_before_numeric_errors() {
    let lua = Lua::new();
    lua.load("jit.off(); jit.flush()").exec().unwrap();
    for floor in [false, true] {
        let body = if floor {
            "math.floor(mark('L',left),mark('R',right))"
        } else {
            "mark('L',left)^mark('R',right)"
        };
        let source:mlua::Function=lua.load(format!("return function(state,left,right) local function mark(key,value) state.log=state.log..key; return value end; return {body} end")).eval().unwrap();
        let native = ordered(floor);
        for (left, right) in [
            (ProgramValue::Number(2.75), ProgramValue::Number(3.0)),
            (ProgramValue::Boolean(false), text("bad")),
            (ProgramValue::Number(2.75), ProgramValue::Boolean(false)),
        ] {
            let state = lua.create_table().unwrap();
            state.set("log", "").unwrap();
            let expected = source.call::<MultiValue>((
                state.clone(),
                lua_value(&lua, &left),
                lua_value(&lua, &right),
            ));
            let input = ProgramValueGraph {
                values: vec![ProgramValue::Table(ProgramTableId(1)), left, right],
                tables: vec![ProgramTable {
                    entries: vec![(text("log"), text(""))],
                }],
            };
            let (mut session, values) = native.session(&input, ProgramLimits::default()).unwrap();
            let actual = session.invoke(ParserCallbackId(1), &values);
            match (expected, actual) {
                (Ok(expected), Ok(actual)) => assert_values(
                    &session.snapshot(&actual).unwrap().graph().values,
                    &expected.into_iter().map(scalar).collect::<Vec<_>>(),
                ),
                (Err(_), Err(actual)) => assert_eq!(actual.kind, ProgramRuntimeErrorKind::Source),
                (expected, actual) => {
                    panic!("floor={floor}: source={expected:?} native={actual:?}")
                }
            }
            assert_eq!(state.get::<String>("log").unwrap(), "LR");
            assert_eq!(
                session.snapshot(&values[..1]).unwrap().graph().tables[0].entries,
                vec![(text("log"), text("LR"))]
            );
        }
    }
}

#[test]
fn floor_and_power_keep_cumulative_conversion_and_result_budgets() {
    for (native, values, conversion_steps) in [
        (
            primitive(ParserProgramIntrinsic::MathFloor),
            vec![text("2.5")],
            3,
        ),
        (
            arithmetic(ParserProgramBinary::Power),
            vec![text("2"), text("3")],
            2,
        ),
    ] {
        let (mut session, values) = native
            .session(
                &graph(values),
                ProgramLimits {
                    pattern: MatchLimits {
                        max_steps: conversion_steps + 1,
                        ..MatchLimits::default()
                    },
                    ..ProgramLimits::default()
                },
            )
            .unwrap();
        session.invoke(ParserCallbackId(1), &values).unwrap();
        assert_eq!(
            session
                .invoke(ParserCallbackId(1), &values)
                .unwrap_err()
                .kind,
            ProgramRuntimeErrorKind::ResourceBound
        );
        assert_eq!(
            session
                .invoke(ParserCallbackId(1), &values)
                .unwrap_err()
                .kind,
            ProgramRuntimeErrorKind::ResourceBound
        );
    }
    let floor = primitive(ParserProgramIntrinsic::MathFloor);
    let input = graph(vec![
        ProgramValue::Number(2.75),
        text("ignored invalid number"),
    ]);
    let output = floor
        .execute(
            ParserCallbackId(1),
            &input,
            ProgramLimits {
                pattern: MatchLimits {
                    max_steps: 1,
                    ..MatchLimits::default()
                },
                ..ProgramLimits::default()
            },
        )
        .unwrap();
    assert_values(&output.graph().values, &[ProgramValue::Number(2.0)]);
    let usage = output.allocations();
    for limits in [
        ProgramLimits {
            max_results: 0,
            ..ProgramLimits::default()
        },
        ProgramLimits {
            max_values: usage.values - 1,
            ..ProgramLimits::default()
        },
    ] {
        assert_eq!(
            floor
                .execute(ParserCallbackId(1), &input, limits)
                .unwrap_err()
                .kind,
            ProgramRuntimeErrorKind::ResourceBound
        );
    }
}

#[test]
fn plain_table_numeric_arguments_fail_and_standalone_modulo_remains_unavailable() {
    let lua = Lua::new();
    let floor: mlua::Function = lua
        .load("return function(a) return math.floor(a) end")
        .eval()
        .unwrap();
    let power: mlua::Function = lua
        .load("return function(a,b) return a^b end")
        .eval()
        .unwrap();
    let table = lua.create_table().unwrap();
    assert!(floor.call::<MultiValue>(table.clone()).is_err());
    assert!(power.call::<MultiValue>((table, 2)).is_err());
    let input = ProgramValueGraph {
        values: vec![
            ProgramValue::Table(ProgramTableId(1)),
            ProgramValue::Number(2.0),
        ],
        tables: vec![ProgramTable::default()],
    };
    for native in [
        primitive(ParserProgramIntrinsic::MathFloor),
        arithmetic(ParserProgramBinary::Power),
    ] {
        assert_eq!(
            native
                .execute(ParserCallbackId(1), &input, ProgramLimits::default())
                .unwrap_err()
                .kind,
            ProgramRuntimeErrorKind::Source
        );
    }
    assert_eq!(
        arithmetic(ParserProgramBinary::Modulo)
            .execute(
                ParserCallbackId(1),
                &graph(vec![ProgramValue::Number(5.0), ProgramValue::Number(2.0)]),
                ProgramLimits::default()
            )
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
}
