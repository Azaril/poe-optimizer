//! Generic scalar bit/modulo execution against the pinned interpreted source.
use super::*;
const BIT_OPERATIONS: [(ParserProgramIntrinsic, &str); 4] = [
    (ParserProgramIntrinsic::BitBand, "bit.band"),
    (ParserProgramIntrinsic::BitBor, "bit.bor"),
    (ParserProgramIntrinsic::BitBxor, "bit.bxor"),
    (ParserProgramIntrinsic::BitBnot, "bit.bnot"),
];
fn modulo(left: ParserProgramExpr, right: ParserProgramExpr) -> ParserProgramExpr {
    e(ParserProgramExprKind::SourceBinary {
        operation: ParserProgramBinary::Modulo,
        left: Box::new(SourceProgramAssignmentOperand::Evaluated { value: left }),
        right: Box::new(right),
    })
}
fn modulo_library() -> CompiledSourcePrograms {
    compile(
        vec![(1, 2, false, vec![], vec![ret(vec![modulo(l(0), l(1))])])],
        vec![vec![]],
    )
}
fn numbers() -> Vec<f64> {
    let mut values = vec![
        0.0,
        -0.0,
        f64::from_bits(1),
        -f64::from_bits(1),
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::MAX,
        -f64::MAX,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::from_bits(0xfff8_0000_0000_0000),
        0.1,
        -0.1,
        0.5,
        -0.5,
        1.5,
        -1.5,
        2.5,
        -2.5,
        3.5,
        -3.5,
    ];
    for exponent in [0, 1, 20, 21, 31, 32, 51, 52, 53, 63] {
        let value = 2.0f64.powi(exponent);
        for adjacent in [
            f64::from_bits(value.to_bits() - 1),
            value,
            f64::from_bits(value.to_bits() + 1),
        ] {
            values.extend([adjacent, -adjacent]);
        }
    }
    values
}
#[test]
fn bit_operations_match_source_rounding_word_wrap_53bit_extremes_and_nonfinite() {
    let values = numbers();
    for (operation, path) in BIT_OPERATIONS {
        let mut cases = Vec::new();
        for value in &values {
            cases.push(vec![ProgramValue::Number(*value)]);
            if operation != ParserProgramIntrinsic::BitBnot {
                for other in [-4294967295.0, -1.0, 0.0, 1.0, 2147483648.0, 4294967295.0] {
                    cases.push(vec![
                        ProgramValue::Number(*value),
                        ProgramValue::Number(other),
                        ProgramValue::Number(*value),
                    ]);
                }
            }
        }
        compare(operation, path, cases);
    }
}
#[test]
fn bit_arity_conversion_failures_and_ignored_unary_extras_match_source() {
    let cases = vec![
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
        vec![text("2147483647.5"), text("4294967295"), text("-1.5")],
        vec![ProgramValue::Number(5.5), ProgramValue::Nil],
        vec![
            ProgramValue::Number(5.5),
            text("bad"),
            ProgramValue::Boolean(false),
        ],
        vec![text("bad"), text("5")],
    ];
    for (operation, path) in BIT_OPERATIONS {
        compare(operation, path, cases.clone());
    }
}
#[test]
fn modulo_matches_cold_source_full_ieee_matrix_and_conversion_errors() {
    let lua = Lua::new();
    lua.load("jit.off(); jit.flush(); assert(not jit.status())")
        .exec()
        .unwrap();
    let source: mlua::Function = lua
        .load("return function(a,b) return a % b end")
        .eval()
        .unwrap();
    let native = modulo_library();
    let mut cases = Vec::new();
    let numbers = numbers();
    for a in &numbers {
        for b in &numbers {
            cases.push(vec![ProgramValue::Number(*a), ProgramValue::Number(*b)]);
        }
    }
    // A fixed pseudorandom bit corpus reaches cancellation/overflow regimes
    // independently of the human-selected thresholds, without a random oracle.
    let mut state = 0x7370_6563_7472_756du64;
    for _ in 0..2048 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let a = f64::from_bits(state);
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let b = f64::from_bits(state);
        cases.push(vec![ProgramValue::Number(a), ProgramValue::Number(b)]);
    }
    cases.extend([
        vec![],
        vec![ProgramValue::Number(2.0)],
        vec![text(" 0x15 "), text("0b11")],
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
                let expected = scalar(expected.into_iter().next().unwrap());
                match (&actual.graph().values[..], &expected) {
                    ([ProgramValue::Number(a)], ProgramValue::Number(b))
                        if a.is_nan() && b.is_nan() => {}
                    _ => assert_values(&actual.graph().values, &[expected]),
                }
            }
            (Err(_), Err(actual)) => {
                assert_eq!(actual.kind, ProgramRuntimeErrorKind::Source, "{values:?}")
            }
            (expected, actual) => panic!("{values:?}: source={expected:?}, native={actual:?}"),
        }
    }
}
fn ordered(operation: Option<ParserProgramIntrinsic>) -> CompiledSourcePrograms {
    let bindings = vec![ParserProgramBinding::CapturedCallback {
        upvalue: 0,
        callback: ParserCallbackId(2),
    }];
    let mark = |key: &str, value| {
        e(ParserProgramExprKind::Call {
            call: Box::new(call(0, None, list(vec![l(0), b(key), value]))),
        })
    };
    let left = mark("L", l(1));
    let right = mark("R", l(2));
    let mut bindings = bindings;
    let body = if let Some(operation) = operation {
        bindings.push(ParserProgramBinding::Intrinsic {
            operation,
            source: ParserProgramIntrinsicSource::OriginalGlobal,
        });
        tail(call(1, None, list(vec![left, right])))
    } else {
        ret(vec![modulo(left, right)])
    };
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
                        value: e(ParserProgramExprKind::Binary {
                            operation: ParserProgramBinary::Concat,
                            left: Box::new(get(l(0), "log")),
                            right: Box::new(l(1)),
                        }),
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
fn all_argument_effects_precede_bit_conversion_and_modulo_errors() {
    let lua = Lua::new();
    lua.load("jit.off(); jit.flush()").exec().unwrap();
    for (operation, path) in BIT_OPERATIONS
        .into_iter()
        .map(|(op, path)| (Some(op), path))
        .chain([(None, "modulo")])
    {
        let expr = if operation.is_some() {
            format!("{path}(mark('L',a),mark('R',b))")
        } else {
            "mark('L',a)%mark('R',b)".into()
        };
        let source:mlua::Function=lua.load(format!("return function(s,a,b) local function mark(k,v) s.log=s.log..k; return v end; return {expr} end")).eval().unwrap();
        let native = ordered(operation);
        for (a, b) in [
            (text("bad"), ProgramValue::Nil),
            (ProgramValue::Number(5.5), text("bad")),
            (text("5.5"), text("2")),
        ] {
            let state = lua.create_table().unwrap();
            state.set("log", "").unwrap();
            let expected = source.call::<MultiValue>((
                state.clone(),
                lua_value(&lua, &a),
                lua_value(&lua, &b),
            ));
            let input = ProgramValueGraph {
                values: vec![ProgramValue::Table(ProgramTableId(1)), a, b],
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
                (expected, actual) => panic!("{path}: {expected:?}, {actual:?}"),
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
fn bit_work_conversion_order_and_results_remain_cumulative_on_failure() {
    for (operation, _) in BIT_OPERATIONS {
        let native = primitive(operation);
        let input = graph(vec![text("5.5"), text("2")]);
        let steps = if operation == ParserProgramIntrinsic::BitBnot {
            4
        } else {
            6
        };
        let (mut session, values) = native
            .session(
                &input,
                ProgramLimits {
                    pattern: MatchLimits {
                        max_steps: steps,
                        ..MatchLimits::default()
                    },
                    ..ProgramLimits::default()
                },
            )
            .unwrap();
        session.invoke(ParserCallbackId(1), &values).unwrap();
        for _ in 0..2 {
            assert_eq!(
                session
                    .invoke(ParserCallbackId(1), &values)
                    .unwrap_err()
                    .kind,
                ProgramRuntimeErrorKind::ResourceBound
            );
        }
        let output = native
            .execute(ParserCallbackId(1), &input, ProgramLimits::default())
            .unwrap();
        for limits in [
            ProgramLimits {
                max_results: 0,
                ..ProgramLimits::default()
            },
            ProgramLimits {
                max_values: output.allocations().values - 1,
                ..ProgramLimits::default()
            },
        ] {
            assert_eq!(
                native
                    .execute(ParserCallbackId(1), &input, limits)
                    .unwrap_err()
                    .kind,
                ProgramRuntimeErrorKind::ResourceBound
            );
        }
    }
    let native = primitive(ParserProgramIntrinsic::BitBor);
    // Left-to-right validation stops before the long later conversion. Reversing
    // these operands consumes the work limit before it can report Source.
    for (values, kind) in [
        (
            vec![ProgramValue::Boolean(false), text(&"1".repeat(64))],
            ProgramRuntimeErrorKind::Source,
        ),
        (
            vec![text(&"1".repeat(64)), ProgramValue::Boolean(false)],
            ProgramRuntimeErrorKind::ResourceBound,
        ),
    ] {
        assert_eq!(
            native
                .execute(
                    ParserCallbackId(1),
                    &graph(values),
                    ProgramLimits {
                        pattern: MatchLimits {
                            max_steps: 8,
                            ..MatchLimits::default()
                        },
                        ..ProgramLimits::default()
                    }
                )
                .unwrap_err()
                .kind,
            kind
        );
    }
}
#[test]
fn numeric_tables_are_rejected_and_bit_libraries_share_no_session_state() {
    for (operation, _) in BIT_OPERATIONS {
        let library = primitive(operation);
        let input = ProgramValueGraph {
            values: vec![ProgramValue::Table(ProgramTableId(1))],
            tables: vec![ProgramTable::default()],
        };
        assert_eq!(
            library
                .execute(ParserCallbackId(1), &input, ProgramLimits::default())
                .unwrap_err()
                .kind,
            ProgramRuntimeErrorKind::Source
        );
        std::thread::scope(|scope| {
            for _ in 0..4 {
                let library = &library;
                scope.spawn(move || {
                    let input = graph(vec![
                        ProgramValue::Number(4294967295.0),
                        ProgramValue::Number(1.0),
                    ]);
                    let (mut session, values) =
                        library.session(&input, ProgramLimits::default()).unwrap();
                    let a = session.invoke(ParserCallbackId(1), &values).unwrap();
                    let b = session.invoke(ParserCallbackId(1), &values).unwrap();
                    assert_values(
                        &session.snapshot(&a).unwrap().graph().values,
                        &session.snapshot(&b).unwrap().graph().values,
                    );
                });
            }
        });
    }
}
#[test]
fn retained_bit_callbacks_use_exact_owner_identity_and_dynamic_dispatch() {
    for (operation, _) in BIT_OPERATIONS {
        let base = compile(
            vec![(1, 0, true, vec![], vec![]), (2, 1, true, vec![], vec![])],
            vec![vec![], vec![]],
        );
        let mut definitions = base.catalog().owner().definitions().unwrap().clone();
        for id in [SourceCallbackId(3), SourceCallbackId(4)] {
            definitions.callbacks.push(SourceCallback {
                kind: SourceCallbackKind::Builtin {
                    symbol: operation.builtin_symbol().unwrap(),
                },
                environment: SourceEnvironment::OriginalGlobals,
                upvalues: vec![],
            });
            definitions.intrinsics.insert(id, operation);
        }
        definitions.callbacks[0].upvalues.push(SourceUpvalue {
            name: "retained".into(),
            value: SourceValue::Callback(SourceCallbackId(3)),
        });
        let owner = SourceProgramOwner::new(definitions).unwrap();
        let mut data = base.catalog().data().clone();
        let arguments = ParserProgramValueList {
            values: vec![],
            tail: Some(Box::new(ParserProgramPack::Varargs)),
        };
        data.programs[0].bindings = vec![ParserProgramBinding::Intrinsic {
            operation,
            source: ParserProgramIntrinsicSource::Captured {
                upvalue: 0,
                callback: SourceCallbackId(3),
            },
        }];
        data.programs[0].body = vec![tail(call(0, None, arguments.clone()))];
        data.programs[1].bindings = vec![ParserProgramBinding::DynamicCall {}];
        data.programs[1].body = vec![tail(call(0, Some(l(0)), arguments))];
        let native =
            CompiledSourcePrograms::new(&SourceProgramCatalog::new(data, owner).unwrap()).unwrap();
        let input = graph(vec![
            ProgramValue::Callback(SourceCallbackId(3)),
            ProgramValue::Callback(SourceCallbackId(4)),
            ProgramValue::Number(4294967295.0),
            ProgramValue::Number(7.5),
        ]);
        let (mut session, values) = native.session(&input, ProgramLimits::default()).unwrap();
        let expected = session.invoke(SourceCallbackId(1), &values[2..]).unwrap();
        for target in &values[..2] {
            let args = [target.clone(), values[2].clone(), values[3].clone()];
            let actual = session.invoke(SourceCallbackId(2), &args).unwrap();
            assert_values(
                &session.snapshot(&actual).unwrap().graph().values,
                &session.snapshot(&expected).unwrap().graph().values,
            );
        }
        let (mut other, _) = native
            .session(&ProgramValueGraph::default(), ProgramLimits::default())
            .unwrap();
        assert!(other.invoke(SourceCallbackId(2), &values[..1]).is_err());
    }
}
#[test]
fn modulo_retains_cumulative_conversion_work_and_legacy_runtime_frontier() {
    let native = modulo_library();
    let (mut session, values) = native
        .session(
            &graph(vec![text("5.5"), text("2")]),
            ProgramLimits {
                pattern: MatchLimits {
                    max_steps: 4,
                    ..MatchLimits::default()
                },
                ..ProgramLimits::default()
            },
        )
        .unwrap();
    session.invoke(SourceCallbackId(1), &values).unwrap();
    for _ in 0..2 {
        assert_eq!(
            session
                .invoke(SourceCallbackId(1), &values)
                .unwrap_err()
                .kind,
            ProgramRuntimeErrorKind::ResourceBound
        );
    }
    let snapshot = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
    let parser = snapshot.modifier_parser();
    let mut program = parser.data().programs.data.programs[0].clone();
    let callback = program.callback;
    program.bindings.clear();
    program.parameter_count = 0;
    program.local_count = 0;
    program.variadic = false;
    // Invalid numeric operands prove the legacy unsupported gate still precedes
    // coercion; standalone's corresponding expression produces a Source error.
    program.body = vec![ret(vec![e(ParserProgramExprKind::Binary {
        operation: ParserProgramBinary::Modulo,
        left: Box::new(b("bad")),
        right: Box::new(n(2.0)),
    })])];
    let data = ParserProgramData {
        schema_version: PARSER_PROGRAM_SCHEMA_VERSION,
        callbacks: BTreeMap::from([(callback, ParserProgramId(1))]),
        programs: vec![program],
    };
    let catalog = ParserProgramCatalog::new(data, parser.clone()).unwrap();
    let library = CompiledSourcePrograms::new(catalog.source_programs()).unwrap();
    let error = library
        .execute(
            callback,
            &ProgramValueGraph::default(),
            ProgramLimits::default(),
        )
        .unwrap_err();
    assert_eq!(error.kind, ProgramRuntimeErrorKind::UnsupportedCapability);
    assert_eq!(
        error.message,
        "modulo operation requires source arithmetic proof"
    );
}
