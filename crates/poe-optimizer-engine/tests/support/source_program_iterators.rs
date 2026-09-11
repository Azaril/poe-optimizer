//! Generic iterator triples use ordinary closure/callback calls on the same heap.
use super::*;

fn nil() -> ParserProgramExpr {
    e(ParserProgramExprKind::Literal {
        value: ParserFactoryLiteral::Nil,
    })
}
fn boolean(value: bool) -> ParserProgramExpr {
    e(ParserProgramExprKind::Literal {
        value: ParserFactoryLiteral::Boolean(value),
    })
}
fn set(table: ParserProgramExpr, key: &str, value: ParserProgramExpr) -> ParserProgramStatement {
    s(ParserProgramStatementKind::TableSet {
        table,
        key: b(key),
        value,
    })
}
fn increment(key: &str) -> ParserProgramStatement {
    set(
        l(0),
        key,
        binary(ParserProgramBinary::Add, get(l(0), b(key)), n(1.0)),
    )
}
fn branch(
    condition: ParserProgramExpr,
    body: Vec<ParserProgramStatement>,
) -> ParserProgramStatement {
    s(ParserProgramStatementKind::If {
        branches: vec![ParserProgramBranch { condition, body }],
        otherwise: vec![],
    })
}
fn eq(left: ParserProgramExpr, right: ParserProgramExpr) -> ParserProgramExpr {
    binary(ParserProgramBinary::Equal, left, right)
}
fn foreach(
    values: ParserProgramValueList,
    body: Vec<ParserProgramStatement>,
) -> ParserProgramStatement {
    s(ParserProgramStatementKind::ForEach {
        locals: vec![3, 4, 5, 6],
        iterator: ParserProgramIterator::Generic { values },
        body,
    })
}
fn initial_state() -> ProgramTable {
    ProgramTable {
        entries: ["init", "extra", "calls", "body"]
            .into_iter()
            .map(|k| (text(k), ProgramValue::Number(0.0)))
            .collect(),
    }
}
fn state_field(session: &mut ProgramSession, state: &SessionValue, key: &str) -> ProgramValue {
    session
        .snapshot(std::slice::from_ref(state))
        .unwrap()
        .graph()
        .tables[0]
        .entries
        .iter()
        .find(|(k, _)| *k == text(key))
        .map_or(ProgramValue::Nil, |(_, v)| v.clone())
}
fn fixture(
    zero_stop: bool,
    break_early: bool,
    fail_second: bool,
) -> (CompiledSourcePrograms, SourceSessionInput) {
    let loop_body = vec![
        increment("body"),
        branch(
            eq(l(3), boolean(false)),
            vec![set(l(0), "first", l(4)), set(l(0), "surplus", l(5))],
        ),
        branch(
            eq(l(3), n(0.0)),
            vec![set(l(0), "second", l(4)), set(l(0), "missing", l(5))],
        ),
        set(l(0), "padded", l(6)),
        s(ParserProgramStatementKind::Assign {
            locals: vec![3, 4],
            values: pack(vec![n(999.0), boolean(true)]),
        }),
        branch(l(2), vec![s(ParserProgramStatementKind::Break)]),
    ];
    let iterator = vec![
        increment("calls"),
        store(2, vec![binary(ParserProgramBinary::Add, cap(2), n(1.0))]),
        set(l(0), "captured_calls", cap(2)),
        set(l(0), "state_alias", eq(l(0), cap(0))),
        branch(
            eq(l(1), nil()),
            vec![ret(vec![boolean(false), b("first"), b("surplus")])],
        ),
        branch(
            cap(3),
            vec![ret(vec![get(boolean(false), b("source_error"))])],
        ),
        branch(
            eq(l(1), boolean(false)),
            vec![ret(vec![n(0.0), b("second")])],
        ),
        branch(cap(1), vec![ret(vec![])]),
        ret(vec![nil(), b("not visited")]),
    ];
    let lib = library_with_locals(
        vec![
            (
                1,
                vec![
                    foreach(
                        ParserProgramValueList {
                            values: vec![],
                            tail: Some(Box::new(ParserProgramPack::Call {
                                call: call(cap(0), vec![l(0)]),
                            })),
                        },
                        loop_body,
                    ),
                    ret(vec![l(0)]),
                ],
            ),
            (4, iterator),
            (
                2,
                vec![
                    increment("init"),
                    ret(vec![cap(0), l(0), nil(), invoke(cap(1), vec![l(0)])]),
                ],
            ),
            (0, vec![increment("extra"), ret(vec![n(99.0)])]),
        ],
        7,
    );
    let state = ProgramValueGraph {
        values: vec![
            cl(1),
            tab(1),
            ProgramValue::Boolean(zero_stop),
            ProgramValue::Boolean(break_early),
        ],
        tables: vec![initial_state()],
    };
    let artifact = input(
        &lib,
        &[
            (1, vec![1]),
            (2, vec![2, 3, 4, 5]),
            (3, vec![6, 7]),
            (4, vec![]),
        ],
        vec![
            cl(3),
            tab(1),
            ProgramValue::Boolean(zero_stop),
            ProgramValue::Number(0.0),
            ProgramValue::Boolean(fail_second),
            cl(2),
            cl(4),
        ],
        state,
    );
    (lib, artifact)
}

#[test]
fn generic_session_iterators_preserve_hidden_control_result_adjustment_and_aliases() {
    let lua = Lua::new();
    lua.load("jit.off(); jit.flush()").exec().unwrap();
    let source: mlua::Function = lua
        .load(
            r#"return function(zeroStop,breakEarly)
        local state={init=0,extra=0,calls=0,body=0}; local calls=0
        local function iterator(s,control)
            s.calls=s.calls+1; calls=calls+1; s.captured_calls=calls; s.state_alias=s==state
            if control==nil then return false,'first','surplus' end
            if control==false then return 0,'second' end
            if zeroStop then return end
            return nil,'not visited'
        end
        local function extra(s) s.extra=s.extra+1; return 99 end
        local function initializer(s) s.init=s.init+1; return iterator,s,nil,extra(s) end
        for key,value,surplus,padded in initializer(state) do
            state.body=state.body+1
            if key==false then state.first=value; state.surplus=surplus end
            if key==0 then state.second=value; state.missing=surplus end
            state.padded=padded; key,value=999,true
            if breakEarly then break end
        end
        return state
    end"#,
        )
        .eval()
        .unwrap();
    for zero_stop in [false, true] {
        for break_early in [false, true] {
            let expected: mlua::Table = source.call((zero_stop, break_early)).unwrap();
            let (lib, artifact) = fixture(zero_stop, break_early, false);
            let (mut session, roots) = lib
                .session_from_input(&artifact, ProgramLimits::default())
                .unwrap();
            let result = session.invoke_callable(&roots[0], &roots[1..]).unwrap();
            assert_eq!(result.len(), 1);
            for key in [
                "init",
                "extra",
                "calls",
                "body",
                "captured_calls",
                "state_alias",
                "first",
                "second",
                "surplus",
                "missing",
                "padded",
            ] {
                let expected = match expected.get::<mlua::Value>(key).unwrap() {
                    mlua::Value::Nil => ProgramValue::Nil,
                    mlua::Value::Boolean(v) => ProgramValue::Boolean(v),
                    mlua::Value::Integer(v) => ProgramValue::Number(v as f64),
                    mlua::Value::Number(v) => ProgramValue::Number(v),
                    mlua::Value::String(v) => ProgramValue::Bytes(v.as_bytes().to_vec()),
                    v => panic!("{v:?}"),
                };
                assert_eq!(
                    state_field(&mut session, &roots[1], key),
                    expected,
                    "{key},zero={zero_stop},break={break_early}"
                );
            }
            assert_eq!(
                state_field(&mut session, &result[0], "body"),
                ProgramValue::Number(if break_early { 1.0 } else { 2.0 })
            );
            let (mut other, _) = lib
                .session_from_input(&artifact, ProgramLimits::default())
                .unwrap();
            error(
                other.invoke_callable(&roots[0], &roots[1..]),
                ProgramRuntimeErrorKind::InvalidInput,
            );
        }
    }
}

#[test]
fn generic_iterator_source_error_preserves_previous_body_and_current_call_effects() {
    let (lib, artifact) = fixture(false, false, true);
    let (mut session, roots) = lib
        .session_from_input(&artifact, ProgramLimits::default())
        .unwrap();
    error(
        session.invoke_callable(&roots[0], &roots[1..]),
        ProgramRuntimeErrorKind::Source,
    );
    for (key, value) in [
        ("init", 1.0),
        ("extra", 1.0),
        ("calls", 2.0),
        ("body", 1.0),
        ("captured_calls", 2.0),
    ] {
        assert_eq!(
            state_field(&mut session, &roots[1], key),
            ProgramValue::Number(value)
        );
    }
    let lua = Lua::new();
    let state:mlua::Table=lua.load(r#"local s={calls=0,body=0}; local function iter(state,control)
        state.calls=state.calls+1; if control==nil then return false end; return (false).source_error
        end; local ok=pcall(function() for k in iter,s do s.body=s.body+1 end end); assert(not ok); return s"#).eval().unwrap();
    assert_eq!(state.get::<u32>("calls").unwrap(), 2);
    assert_eq!(state.get::<u32>("body").unwrap(), 1);
}

#[test]
fn generic_initializer_evaluates_discarded_expressions_before_noncallable_failure() {
    let lib = library_with_locals(
        vec![
            (
                1,
                vec![foreach(
                    pack(vec![
                        boolean(false),
                        l(0),
                        nil(),
                        invoke(cap(0), vec![l(0)]),
                    ]),
                    vec![increment("body")],
                )],
            ),
            (0, vec![increment("extra"), ret(vec![n(99.0)])]),
        ],
        7,
    );
    let artifact = input(
        &lib,
        &[(1, vec![1]), (2, vec![])],
        vec![cl(2)],
        ProgramValueGraph {
            values: vec![cl(1), tab(1)],
            tables: vec![initial_state()],
        },
    );
    let (mut session, roots) = lib
        .session_from_input(&artifact, ProgramLimits::default())
        .unwrap();
    error(
        session.invoke_callable(&roots[0], &roots[1..]),
        ProgramRuntimeErrorKind::Source,
    );
    assert_eq!(
        state_field(&mut session, &roots[1], "extra"),
        ProgramValue::Number(1.0)
    );
    assert_eq!(
        state_field(&mut session, &roots[1], "body"),
        ProgramValue::Number(0.0)
    );
    let lua = Lua::new();
    let observed:u32=lua.load("local n=0; local function extra() n=n+1 end; local ok=pcall(function() for k in false,{},nil,extra() do end end); assert(not ok); return n").eval().unwrap();
    assert_eq!(observed, 1);
}

#[test]
fn generic_loop_call_instruction_and_value_budgets_are_cumulative() {
    let (lib, artifact) = fixture(false, false, false);
    let (mut measured, roots) = lib
        .session_from_input(&artifact, ProgramLimits::default())
        .unwrap();
    measured.invoke_callable(&roots[0], &roots[1..]).unwrap();
    let steps = measured.steps();
    let (mut session, roots) = lib
        .session_from_input(
            &artifact,
            ProgramLimits {
                max_steps: steps + 1,
                ..ProgramLimits::default()
            },
        )
        .unwrap();
    session.invoke_callable(&roots[0], &roots[1..]).unwrap();
    error(
        session.invoke_callable(&roots[0], &roots[1..]),
        ProgramRuntimeErrorKind::ResourceBound,
    );
    error(
        session.invoke_callable(&roots[0], &roots[1..]),
        ProgramRuntimeErrorKind::ResourceBound,
    );
    let (mut session, roots) = lib
        .session_from_input(
            &artifact,
            ProgramLimits {
                max_call_depth: 1,
                ..ProgramLimits::default()
            },
        )
        .unwrap();
    error(
        session.invoke_callable(&roots[0], &roots[1..]),
        ProgramRuntimeErrorKind::ResourceBound,
    );
    let usage = measured.allocations();
    let (mut session, roots) = lib
        .session_from_input(
            &artifact,
            ProgramLimits {
                max_values: usage.values - 1,
                ..ProgramLimits::default()
            },
        )
        .unwrap();
    error(
        session.invoke_callable(&roots[0], &roots[1..]),
        ProgramRuntimeErrorKind::ResourceBound,
    );
}

#[test]
fn generic_for_invokes_immutable_callback_values_without_session_prototype_confusion() {
    let lib = library_with_locals(
        vec![
            (
                1,
                vec![
                    foreach(pack(vec![cap(0), l(0)]), vec![increment("body")]),
                    ret(vec![l(0)]),
                ],
            ),
            (
                0,
                vec![
                    increment("calls"),
                    branch(eq(l(1), nil()), vec![ret(vec![boolean(false)])]),
                    ret(vec![]),
                ],
            ),
        ],
        7,
    );
    let owner = SourceProgramOwner::new_with_closures(
        lib.catalog().owner().definitions().unwrap().clone(),
        None,
        None,
        SourceClosurePrototypes {
            schema_version: SOURCE_CLOSURE_PROTOTYPES_SCHEMA_VERSION,
            prototypes: vec![SourceClosurePrototype {
                callback: SourceCallbackId(1),
            }],
        },
    )
    .unwrap();
    let lib = CompiledSourcePrograms::new(
        &SourceProgramCatalog::new(lib.catalog().data().clone(), owner).unwrap(),
    )
    .unwrap();
    let artifact = input(
        &lib,
        &[(1, vec![1])],
        vec![ProgramValue::Callback(SourceCallbackId(2))],
        ProgramValueGraph {
            values: vec![cl(1), tab(1)],
            tables: vec![initial_state()],
        },
    );
    let (mut session, roots) = lib
        .session_from_input(&artifact, ProgramLimits::default())
        .unwrap();
    session.invoke_callable(&roots[0], &roots[1..]).unwrap();
    assert_eq!(
        state_field(&mut session, &roots[1], "calls"),
        ProgramValue::Number(2.0)
    );
    assert_eq!(
        state_field(&mut session, &roots[1], "body"),
        ProgramValue::Number(1.0)
    );
}
