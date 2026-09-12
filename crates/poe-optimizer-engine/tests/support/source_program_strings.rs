//! Generic source string primitives: byte parity, exact packs and shared bounds.
use super::*;

#[test]
fn lower_matches_every_byte_and_source_scalar_coercion() {
    compare(
        ParserProgramIntrinsic::StringLower,
        "string.lower",
        vec![
            vec![],
            vec![ProgramValue::Nil],
            vec![ProgramValue::Boolean(false)],
            vec![text("")],
            vec![ProgramValue::Bytes((0..=255).collect())],
            vec![text("AZaz Ã„Ã– ÃœÄ° ðŸ¦€\0A"), ProgramValue::Boolean(false)],
            vec![ProgramValue::Number(1.25e20)],
            vec![ProgramValue::Number(-0.0)],
            vec![ProgramValue::Number(f64::INFINITY)],
            vec![ProgramValue::Number(f64::NEG_INFINITY)],
            vec![ProgramValue::Number(f64::NAN)],
        ],
    );
}

#[test]
fn find_matches_literal_pattern_capture_and_initial_position_packs() {
    let mut cases = vec![
        vec![],
        vec![text("abc")],
        vec![ProgramValue::Boolean(false), text("x")],
        vec![text("abc"), ProgramValue::Boolean(false)],
        vec![text("abc"), text("b"), ProgramValue::Boolean(false)],
        vec![ProgramValue::Number(12345.0), ProgramValue::Number(23.0)],
    ];
    for (subject, pattern) in [
        ("abcabc", "bc"),
        ("abc", ""),
        ("", ""),
        ("abc", "^b"),
        ("abc", "a$"),
        ("ab12cd", "()(%d+)()"),
        ("[a]", "["),
        ("a%b", "%"),
        ("a)b", ")"), // ')' does not trigger the fixed-string magic test.
        ("a\0b", "\0b"),
        ("a\0b", "\0%"), // magic after NUL chooses the pattern interpreter.
        ("aaa", "a("),
        ("abc", "%f[%a]a"),
        ("x(abc)y", "%b()"),
    ] {
        for init in [-20.0, -3.0, -1.0, -0.0, 1.0, 2.75, 4.0, 50.0] {
            for plain in [
                ProgramValue::Nil,
                ProgramValue::Boolean(false),
                ProgramValue::Boolean(true),
                ProgramValue::Number(0.0),
                text(""),
            ] {
                cases.push(vec![
                    text(subject),
                    text(pattern),
                    ProgramValue::Number(init),
                    plain,
                    text("ignored"),
                ]);
            }
        }
    }
    compare(ParserProgramIntrinsic::StringFind, "string.find", cases);
}

#[test]
fn sub_matches_required_start_optional_end_and_binary_clipping() {
    let mut cases = vec![
        vec![],
        vec![text("abc")],
        vec![text("abc"), ProgramValue::Nil],
        vec![text("abc"), ProgramValue::Boolean(false)],
        vec![ProgramValue::Boolean(false), ProgramValue::Number(1.0)],
        vec![ProgramValue::Number(12345.0), text("2.9"), text("-1.2")],
    ];
    for subject in ["", "abc", "a\0AZ", "Ã„ðŸ¦€"] {
        for start in [
            -2147483648.0,
            -20.0,
            -1.0,
            -0.0,
            1.0,
            2.8,
            20.0,
            2147483647.0,
        ] {
            cases.push(vec![text(subject), ProgramValue::Number(start)]);
            for end in [
                ProgramValue::Nil,
                ProgramValue::Boolean(false),
                text("-2"),
                ProgramValue::Number(-20.0),
                ProgramValue::Number(0.0),
                ProgramValue::Number(2.9),
                ProgramValue::Number(2147483647.0),
            ] {
                cases.push(vec![
                    text(subject),
                    ProgramValue::Number(start),
                    end,
                    text("ignored"),
                ]);
            }
        }
    }
    compare(ParserProgramIntrinsic::StringSub, "string.sub", cases);
}

#[test]
fn string_indexes_keep_portable_frontier_and_source_check_order() {
    for operation in [
        ParserProgramIntrinsic::StringFind,
        ParserProgramIntrinsic::StringSub,
    ] {
        let lib = primitive(operation);
        for index in [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            2147483648.0,
            -2147483649.0,
        ] {
            let values = if operation == ParserProgramIntrinsic::StringFind {
                vec![
                    text("abc"),
                    text("%"),
                    ProgramValue::Number(index),
                    ProgramValue::Boolean(true),
                ]
            } else {
                vec![text("abc"), ProgramValue::Number(index)]
            };
            assert_eq!(
                lib.execute(
                    ParserCallbackId(1),
                    &graph(values),
                    ProgramLimits::default()
                )
                .unwrap_err()
                .kind,
                ProgramRuntimeErrorKind::UnsupportedCapability
            );
        }
        let invalid_subject = vec![
            ProgramValue::Boolean(false),
            ProgramValue::Number(f64::INFINITY),
            ProgramValue::Number(f64::INFINITY),
        ];
        assert_eq!(
            lib.execute(
                ParserCallbackId(1),
                &graph(invalid_subject),
                ProgramLimits::default()
            )
            .unwrap_err()
            .kind,
            ProgramRuntimeErrorKind::Source
        );
    }
    for (operation, args) in [
        (
            ParserProgramIntrinsic::StringFind,
            vec![
                text("abc"),
                ProgramValue::Boolean(false),
                ProgramValue::Number(f64::INFINITY),
            ],
        ),
        (
            ParserProgramIntrinsic::StringSub,
            vec![
                text("abc"),
                text("bad"),
                ProgramValue::Number(f64::INFINITY),
            ],
        ),
    ] {
        assert_eq!(
            primitive(operation)
                .execute(ParserCallbackId(1), &graph(args), ProgramLimits::default())
                .unwrap_err()
                .kind,
            ProgramRuntimeErrorKind::Source
        );
    }
}

#[test]
fn literal_find_never_compiles_needles_and_charges_repeated_comparisons() {
    let lib = primitive(ParserProgramIntrinsic::StringFind);
    for (subject, needle, plain, expected) in [
        (
            "a[b",
            "[",
            true,
            vec![ProgramValue::Number(2.0), ProgramValue::Number(2.0)],
        ),
        (
            "a)b",
            ")",
            false,
            vec![ProgramValue::Number(2.0), ProgramValue::Number(2.0)],
        ),
        (
            "a\0b",
            "\0b",
            false,
            vec![ProgramValue::Number(2.0), ProgramValue::Number(3.0)],
        ),
    ] {
        let result = lib
            .execute(
                ParserCallbackId(1),
                &graph(vec![
                    text(subject),
                    text(needle),
                    ProgramValue::Nil,
                    ProgramValue::Boolean(plain),
                ]),
                ProgramLimits {
                    max_bytes: subject.len() + needle.len(),
                    ..ProgramLimits::default()
                },
            )
            .unwrap();
        assert_values(&result.graph().values, &expected);
    }
    let input = graph(vec![
        text("aaaaaaaaa"),
        text("aaaab"),
        ProgramValue::Nil,
        ProgramValue::Boolean(true),
    ]);
    let (mut session, args) = lib
        .session(
            &input,
            ProgramLimits {
                pattern: MatchLimits {
                    max_steps: 26,
                    ..MatchLimits::default()
                },
                ..ProgramLimits::default()
            },
        )
        .unwrap();
    let result = session.invoke(ParserCallbackId(1), &args).unwrap();
    assert_values(
        &session.snapshot(&result).unwrap().graph().values,
        &[ProgramValue::Nil],
    );
    assert_eq!(session.pattern_steps(), 26);
    assert_eq!(
        session.invoke(ParserCallbackId(1), &args).unwrap_err().kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
    assert_eq!(
        session.invoke(ParserCallbackId(1), &args).unwrap_err().kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
}

#[test]
fn string_outputs_captures_and_transforms_share_cumulative_limits() {
    for (operation, args) in [
        (ParserProgramIntrinsic::StringLower, vec![text("ABC")]),
        (
            ParserProgramIntrinsic::StringSub,
            vec![text("ABC"), ProgramValue::Number(1.0)],
        ),
        (
            ParserProgramIntrinsic::StringFind,
            vec![text("abc12"), text("()(%d+)()")],
        ),
    ] {
        let lib = primitive(operation);
        let input = graph(args);
        let output = lib
            .execute(ParserCallbackId(1), &input, ProgramLimits::default())
            .unwrap();
        for limits in [
            ProgramLimits {
                max_bytes: output.allocations().bytes - 1,
                ..ProgramLimits::default()
            },
            ProgramLimits {
                max_values: output.allocations().values - 1,
                ..ProgramLimits::default()
            },
        ] {
            assert_eq!(
                lib.execute(ParserCallbackId(1), &input, limits)
                    .unwrap_err()
                    .kind,
                ProgramRuntimeErrorKind::ResourceBound
            );
        }
        let (mut session, args) = lib
            .session(
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
        assert_eq!(
            session.invoke(ParserCallbackId(1), &args).unwrap_err().kind,
            ProgramRuntimeErrorKind::ResourceBound
        );
    }
    let lib = primitive(ParserProgramIntrinsic::StringFind);
    assert_eq!(
        lib.execute(
            ParserCallbackId(1),
            &graph(vec![
                text("abc"),
                text("a"),
                ProgramValue::Nil,
                ProgramValue::Boolean(true)
            ]),
            ProgramLimits {
                pattern: MatchLimits {
                    max_subject_bytes: 2,
                    ..MatchLimits::default()
                },
                ..ProgramLimits::default()
            }
        )
        .unwrap_err()
        .kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
}

#[test]
fn dynamic_string_methods_keep_native_primitives_and_actual_raw_overrides() {
    for (name, subject, args, expected) in [
        ("lower", "ABC", vec![], vec![text("abc")]),
        (
            "find",
            "abc",
            vec![text("b")],
            vec![ProgramValue::Number(2.0), ProgramValue::Number(2.0)],
        ),
        (
            "sub",
            "abc",
            vec![ProgramValue::Number(2.0)],
            vec![text("bc")],
        ),
    ] {
        let lib = compile(
            vec![
                (
                    1,
                    1,
                    true,
                    vec![ParserProgramBinding::DynamicMethod { key: name.into() }],
                    vec![tail(call(
                        0,
                        Some(l(0)),
                        ParserProgramValueList {
                            values: vec![],
                            tail: Some(Box::new(ParserProgramPack::Varargs)),
                        },
                    ))],
                ),
                (2, 1, true, vec![], vec![ret(vec![b("override")])]),
            ],
            vec![vec![], vec![]],
        );
        let mut values = vec![text(subject)];
        values.extend(args.clone());
        let output = lib
            .execute(
                ParserCallbackId(1),
                &graph(values),
                ProgramLimits::default(),
            )
            .unwrap();
        assert_values(&output.graph().values, &expected);
        let mut values = vec![ProgramValue::Table(ProgramTableId(1))];
        values.extend(args);
        let output = lib
            .execute(
                ParserCallbackId(1),
                &ProgramValueGraph {
                    values,
                    tables: vec![ProgramTable {
                        entries: vec![(text(name), ProgramValue::Callback(ParserCallbackId(2)))],
                    }],
                },
                ProgramLimits::default(),
            )
            .unwrap();
        assert_values(&output.graph().values, &[text("override")]);
        assert_eq!(
            lib.execute(
                ParserCallbackId(1),
                &graph(vec![ProgramValue::Number(123.0)]),
                ProgramLimits::default()
            )
            .unwrap_err()
            .kind,
            ProgramRuntimeErrorKind::Source
        );
    }
}

#[test]
fn static_method_table_override_stops_after_argument_effects_without_false_source_error() {
    let marked = e(ParserProgramExprKind::Call {
        call: Box::new(call(1, None, list(vec![l(0)]))),
    });
    let lib = compile(
        vec![
            (
                1,
                1,
                false,
                vec![
                    ParserProgramBinding::Intrinsic {
                        operation: ParserProgramIntrinsic::StringLower,
                        source: ParserProgramIntrinsicSource::OriginalGlobal,
                    },
                    ParserProgramBinding::CapturedCallback {
                        upvalue: 0,
                        callback: ParserCallbackId(2),
                    },
                ],
                vec![tail(call(0, Some(l(0)), list(vec![marked])))],
            ),
            (
                2,
                1,
                false,
                vec![],
                vec![
                    s(ParserProgramStatementKind::TableSet {
                        table: l(0),
                        key: b("mark"),
                        value: n(1.0),
                    }),
                    ret(vec![b("ignored")]),
                ],
            ),
            (3, 1, false, vec![], vec![ret(vec![get(l(0), "mark")])]),
        ],
        vec![
            vec![ParserUpvalue {
                name: "mark".into(),
                value: ParserValue::Callback(ParserCallbackId(2)),
            }],
            vec![],
            vec![],
        ],
    );
    let input = ProgramValueGraph {
        values: vec![ProgramValue::Table(ProgramTableId(1))],
        tables: vec![
            ProgramTable {
                entries: vec![
                    (text("lower"), ProgramValue::Table(ProgramTableId(2))),
                    (text("mark"), ProgramValue::Number(0.0)),
                ],
            },
            ProgramTable::default(),
        ],
    };
    let coverage = ProgramTableCoverage::from([(
        ProgramTableId(2),
        SourceTableCoverage {
            inventory: SourceTableInventory::Complete,
            known_absent: Default::default(),
            unavailable: Default::default(),
            index_fallback: SourceTableIndexFallback::Nil,
            call_fallback: SourceTableCallFallback::Unavailable,
        },
    )]);
    let (mut session, values) = lib
        .session_with_coverage(&input, &coverage, ProgramLimits::default())
        .unwrap();
    assert_eq!(
        session
            .invoke(ParserCallbackId(1), &values)
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    let mark = session.invoke(ParserCallbackId(3), &values).unwrap();
    assert_values(
        &session.snapshot(&mark).unwrap().graph().values,
        &[ProgramValue::Number(1.0)],
    );
}
