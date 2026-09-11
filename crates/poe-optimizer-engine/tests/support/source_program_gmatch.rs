//! Native function instances over the shared matcher and private session heap.
use super::*;

fn library() -> CompiledSourcePrograms {
    compile(
        vec![
            (
                1,
                0,
                true,
                vec![ParserProgramBinding::Intrinsic {
                    operation: ParserProgramIntrinsic::StringGmatch,
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
            ),
            (
                2,
                2,
                false,
                vec![ParserProgramBinding::Intrinsic {
                    operation: ParserProgramIntrinsic::Type,
                    source: ParserProgramIntrinsicSource::OriginalGlobal,
                }],
                vec![ret(vec![
                    e(ParserProgramExprKind::Call {
                        call: Box::new(call(0, None, list(vec![l(0)]))),
                    }),
                    e(ParserProgramExprKind::Binary {
                        operation: ParserProgramBinary::Equal,
                        left: Box::new(l(0)),
                        right: Box::new(l(1)),
                    }),
                ])],
            ),
            (
                3,
                2,
                false,
                vec![],
                vec![ret(vec![e(ParserProgramExprKind::Get {
                    table: Box::new(l(0)),
                    key: Box::new(l(1)),
                })])],
            ),
            (
                4,
                3,
                false,
                vec![],
                vec![
                    s(ParserProgramStatementKind::TableSet {
                        table: l(0),
                        key: l(1),
                        value: l(2),
                    }),
                    ret(vec![]),
                ],
            ),
            (
                5,
                1,
                true,
                vec![ParserProgramBinding::DynamicCall {}],
                vec![tail(call(
                    0,
                    Some(l(0)),
                    ParserProgramValueList {
                        values: vec![],
                        tail: Some(Box::new(ParserProgramPack::Varargs)),
                    },
                ))],
            ),
        ],
        vec![vec![]; 5],
    )
}
fn session(lib: &CompiledSourcePrograms, limits: ProgramLimits) -> ProgramSession {
    lib.session(&ProgramValueGraph::default(), limits)
        .unwrap()
        .0
}
fn values(session: &mut ProgramSession, input: Vec<ProgramValue>) -> Vec<SessionValue> {
    session.borrow(&graph(input)).unwrap()
}
fn make(session: &mut ProgramSession, subject: &str, pattern: &str) -> SessionValue {
    let input = values(session, vec![text(subject), text(pattern)]);
    let result = session.invoke(SourceCallbackId(1), &input).unwrap();
    assert_eq!(result.len(), 1);
    result[0].clone()
}
fn output(session: &mut ProgramSession, result: &[SessionValue]) -> Vec<ProgramValue> {
    session.snapshot(result).unwrap().graph().values.clone()
}
fn next(session: &mut ProgramSession, iterator: &SessionValue) -> Vec<ProgramValue> {
    let result = session.invoke_callable(iterator, &[]).unwrap();
    output(session, &result)
}

#[test]
fn stateful_factory_and_capture_packs_match_interpreted_luajit() {
    let lua = Lua::new();
    lua.load("jit.off(); jit.flush(); assert(not jit.status())")
        .exec()
        .unwrap();
    let source: mlua::Function = lua.load("return string.gmatch").eval().unwrap();
    let lib = library();
    let cases = vec![
        vec![text("abc"), text(".")],
        vec![text("a1 b2"), text("(%a)(%d)")],
        vec![text("ab"), text("()")],
        vec![text(""), text("")],
        vec![text("ab"), text("a*")],
        vec![text("^a^b"), text("^.")],
        vec![ProgramValue::Bytes(vec![0, 255, b'a']), text(".")],
        vec![
            ProgramValue::Number(123.0),
            ProgramValue::Number(2.0),
            ProgramValue::Boolean(false),
        ],
        vec![text("a"), text("[")],
        vec![text("ab"), text("a*[")],
        vec![text("ab"), text("%")],
        vec![text("ab"), text("(.")],
    ];
    for args in cases {
        let iter: mlua::Function = source
            .call(MultiValue::from_vec(
                args.iter().map(|v| lua_value(&lua, v)).collect(),
            ))
            .unwrap();
        let mut session = session(&lib, ProgramLimits::default());
        let input = values(&mut session, args.clone());
        let iterator = session
            .invoke(SourceCallbackId(1), &input)
            .unwrap()
            .remove(0);
        // Keep invoking after both exhaustion and errors to check persistent state.
        for _ in 0..8 {
            let expected = iter.call::<MultiValue>((false, 73));
            let ignored = values(
                &mut session,
                vec![ProgramValue::Boolean(false), ProgramValue::Number(73.0)],
            );
            let actual = session.invoke_callable(&iterator, &ignored);
            match (expected, actual) {
                (Ok(expected), Ok(actual)) => assert_values(
                    &output(&mut session, &actual),
                    &expected.into_iter().map(scalar).collect::<Vec<_>>(),
                ),
                (Err(_), Err(actual)) => {
                    assert_eq!(actual.kind, ProgramRuntimeErrorKind::Source, "{args:?}")
                }
                (expected, actual) => panic!("{args:?}: source={expected:?}, native={actual:?}"),
            }
        }
    }
    for args in [
        vec![],
        vec![text("a")],
        vec![ProgramValue::Boolean(false), text("[")],
        vec![text("a"), ProgramValue::Boolean(false)],
    ] {
        assert!(
            source
                .call::<MultiValue>(MultiValue::from_vec(
                    args.iter().map(|v| lua_value(&lua, v)).collect()
                ))
                .is_err()
        );
        let mut session = session(&lib, ProgramLimits::default());
        let input = values(&mut session, args);
        assert_eq!(
            session
                .invoke(SourceCallbackId(1), &input)
                .unwrap_err()
                .kind,
            ProgramRuntimeErrorKind::Source
        );
    }
}

#[test]
fn iterator_identity_keys_aliases_method_overrides_and_lifetime_are_session_owned() {
    let lib = library();
    let mut session = session(&lib, ProgramLimits::default());
    let first = make(&mut session, "abc", ".");
    let alias = first.clone();
    let second = make(&mut session, "abc", ".");
    for (other, equal) in [(&alias, true), (&second, false)] {
        let result = session
            .invoke(SourceCallbackId(2), &[first.clone(), other.clone()])
            .unwrap();
        assert_eq!(
            output(&mut session, &result),
            vec![text("function"), ProgramValue::Boolean(equal)]
        );
    }
    let table = session
        .import_with_coverage(
            &ProgramValueGraph {
                values: vec![ProgramValue::Table(ProgramTableId(1))],
                tables: vec![ProgramTable::default()],
            },
            &ProgramTableCoverage::new(),
        )
        .unwrap()
        .remove(0);
    let key = values(&mut session, vec![text("next")]).remove(0);
    session
        .invoke(
            SourceCallbackId(4),
            &[table.clone(), first.clone(), second.clone()],
        )
        .unwrap();
    session
        .invoke(
            SourceCallbackId(4),
            &[table.clone(), key.clone(), first.clone()],
        )
        .unwrap();
    let retained = session
        .invoke(SourceCallbackId(3), &[table.clone(), first.clone()])
        .unwrap()
        .remove(0);
    drop(first);
    drop(second);
    assert_eq!(next(&mut session, &alias), vec![text("a")]);
    let result = session.invoke_method(&table, "next", &[]).unwrap();
    assert_eq!(output(&mut session, &result), vec![text("b")]);
    assert_eq!(next(&mut session, &retained), vec![text("a")]);
    assert_eq!(next(&mut session, &alias), vec![text("c")]);
    assert!(next(&mut session, &alias).is_empty());
    for root in [&alias, &table] {
        assert_eq!(
            session
                .snapshot(std::slice::from_ref(root))
                .unwrap_err()
                .kind,
            ProgramRuntimeErrorKind::UnsupportedCapability
        );
    }
    let nil = values(&mut session, vec![ProgramValue::Nil]).remove(0);
    session
        .invoke(SourceCallbackId(4), &[table.clone(), key.clone(), nil])
        .unwrap();
    let scalar = values(&mut session, vec![ProgramValue::Number(1.0)]).remove(0);
    session
        .invoke(SourceCallbackId(4), &[table.clone(), alias.clone(), scalar])
        .unwrap();
    assert_eq!(
        session.snapshot(&[table]).unwrap_err().kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    // Functions are not writable tables and cannot acquire arbitrary fields.
    assert_eq!(
        session
            .invoke(
                SourceCallbackId(4),
                &[alias.clone(), key.clone(), retained.clone()]
            )
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::Source
    );
    assert_eq!(
        session
            .invoke(SourceCallbackId(3), &[alias.clone(), key])
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::Source
    );
    let mut foreign = lib
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap()
        .0;
    assert_eq!(
        foreign.invoke_callable(&retained, &[]).unwrap_err().kind,
        ProgramRuntimeErrorKind::InvalidInput
    );
    assert_eq!(
        foreign
            .snapshot(std::slice::from_ref(&retained))
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::InvalidInput
    );
    assert_eq!(next(&mut session, &retained), vec![text("b")]);
}

#[test]
fn malformed_and_result_limit_errors_restore_the_iterator_slot() {
    let lib = library();
    let mut session = session(&lib, ProgramLimits::default());
    let bad = make(&mut session, "ab", "[");
    for _ in 0..3 {
        let before = session.steps();
        assert_eq!(
            session.invoke_callable(&bad, &[]).unwrap_err().kind,
            ProgramRuntimeErrorKind::Source
        );
        assert!(session.steps() > before);
    }
    // Gmatch commits successful match position before exporting capture results.
    let mut limited = lib
        .session(
            &ProgramValueGraph::default(),
            ProgramLimits {
                max_results: 2,
                ..ProgramLimits::default()
            },
        )
        .unwrap()
        .0;
    let iterator = make(&mut limited, "abc", "(a)(b)(c)");
    assert_eq!(
        limited.invoke_callable(&iterator, &[]).unwrap_err().kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
    assert!(limited.invoke_callable(&iterator, &[]).unwrap().is_empty());
}

#[test]
fn native_function_creation_and_calls_use_cumulative_session_budgets() {
    let lib = library();
    let mut measured = session(&lib, ProgramLimits::default());
    let input = values(&mut measured, vec![text("abc"), text(".")]);
    measured.invoke(SourceCallbackId(1), &input).unwrap();
    let after = measured.allocations();
    for limits in [
        ProgramLimits {
            max_values: after.values,
            ..ProgramLimits::default()
        },
        ProgramLimits {
            max_bytes: after.bytes,
            ..ProgramLimits::default()
        },
    ] {
        let mut limited = session(&lib, limits);
        let input = values(&mut limited, vec![text("abc"), text(".")]);
        let iterator = limited
            .invoke(SourceCallbackId(1), &input)
            .unwrap()
            .remove(0);
        assert_eq!(
            limited
                .invoke(SourceCallbackId(1), &input)
                .unwrap_err()
                .kind,
            ProgramRuntimeErrorKind::ResourceBound
        );
        assert!(limited.allocations().values <= limits.max_values);
        assert!(limited.allocations().bytes <= limits.max_bytes);
        // Failed creation never fabricates another visible instance.
        assert_eq!(
            limited.snapshot(&[iterator]).unwrap_err().kind,
            if limited.allocations().values == limits.max_values {
                ProgramRuntimeErrorKind::ResourceBound
            } else {
                ProgramRuntimeErrorKind::UnsupportedCapability
            }
        );
    }
    let mut measured = session(&lib, ProgramLimits::default());
    let _ = make(&mut measured, "ab", ".");
    let mut instructions = session(
        &lib,
        ProgramLimits {
            max_steps: measured.steps() + 1,
            ..ProgramLimits::default()
        },
    );
    let iterator = make(&mut instructions, "ab", ".");
    instructions.invoke_callable(&iterator, &[]).unwrap();
    assert_eq!(
        instructions
            .invoke_callable(&iterator, &[])
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
    assert_eq!(
        instructions
            .invoke_callable(&iterator, &[])
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
    let mut limited = session(
        &lib,
        ProgramLimits {
            pattern: MatchLimits {
                max_steps: 1,
                ..MatchLimits::default()
            },
            ..ProgramLimits::default()
        },
    );
    let iterator = make(&mut limited, "ab", ".");
    let error = limited.invoke_callable(&iterator, &[]).unwrap_err();
    assert_eq!(error.kind, ProgramRuntimeErrorKind::ResourceBound);
    assert_eq!(
        limited.invoke_callable(&iterator, &[]).unwrap_err().kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
    let mut depth = session(
        &lib,
        ProgramLimits {
            max_call_depth: 1,
            ..ProgramLimits::default()
        },
    );
    let iterator = make(&mut depth, "ab", ".");
    assert_eq!(
        depth
            .invoke(SourceCallbackId(5), std::slice::from_ref(&iterator))
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
    assert_eq!(next(&mut depth, &iterator), vec![text("a")]);
}

#[test]
fn concurrent_sessions_share_library_but_never_iterator_progress() {
    let library = library();
    std::thread::scope(|scope| {
        let jobs = (0..8)
            .map(|_| {
                let library = library.clone();
                scope.spawn(move || {
                    let mut session = session(&library, ProgramLimits::default());
                    let iterator = make(&mut session, "abcd", ".");
                    let other = make(&mut session, "xy", ".");
                    assert_eq!(next(&mut session, &iterator), vec![text("a")]);
                    assert_eq!(next(&mut session, &other), vec![text("x")]);
                    assert_eq!(next(&mut session, &iterator), vec![text("b")]);
                    assert_eq!(next(&mut session, &other), vec![text("y")]);
                    assert_eq!(next(&mut session, &iterator), vec![text("c")]);
                    assert_eq!(next(&mut session, &iterator), vec![text("d")]);
                    assert!(next(&mut session, &iterator).is_empty());
                })
            })
            .collect::<Vec<_>>();
        for job in jobs {
            job.join().unwrap();
        }
    });
}
