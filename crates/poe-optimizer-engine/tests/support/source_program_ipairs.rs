//! Exact retained ipairs auxiliary identity and stateless raw lookup.
use super::*;
use std::collections::BTreeSet;
const FACTORY: SourceCallbackId = SourceCallbackId(7);
const OTHER: SourceCallbackId = SourceCallbackId(8);
const AUX: SourceCallbackId = SourceCallbackId(10);
fn fixture() -> CompiledSourcePrograms {
    let base = compile(
        (1..=6).map(|id| (id, 0, false, vec![], vec![])).collect(),
        vec![vec![]; 6],
    );
    let mut definitions = base.catalog().owner().definitions().unwrap().clone();
    for operation in [
        ParserProgramIntrinsic::Ipairs,
        ParserProgramIntrinsic::Ipairs,
        ParserProgramIntrinsic::IpairsAux,
        ParserProgramIntrinsic::IpairsAux,
    ] {
        let id = SourceCallbackId(definitions.callbacks.len() as u32 + 1);
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
        name: "original".into(),
        value: SourceValue::Callback(FACTORY),
    });
    definitions.callbacks[5].upvalues.push(SourceUpvalue {
        name: "effect".into(),
        value: SourceValue::Callback(SourceCallbackId(5)),
    });
    let owner = SourceProgramOwner::new_with_context(
        definitions,
        None,
        SourceProgramContext {
            iteration: Some(SourceProgramIteration {
                ipairs_aux: BTreeMap::from([(FACTORY, AUX), (OTHER, SourceCallbackId(9))]),
                ..SourceProgramIteration::default()
            }),
            ..SourceProgramContext::default()
        },
    )
    .unwrap();
    let mut data = base.catalog().data().clone();
    let bindings = vec![ParserProgramBinding::Intrinsic {
        operation: ParserProgramIntrinsic::Ipairs,
        source: ParserProgramIntrinsicSource::Captured {
            upvalue: 0,
            callback: FACTORY,
        },
    }];
    data.programs[0].parameter_count = 1;
    data.programs[0].local_count = 1;
    data.programs[0].bindings = bindings;
    data.programs[0].body = vec![tail(call(0, None, list(vec![l(0)])))];
    data.programs[1].parameter_count = 1;
    data.programs[1].local_count = 1;
    data.programs[1].variadic = true;
    data.programs[1].bindings = vec![ParserProgramBinding::DynamicCall {}];
    data.programs[1].body = vec![tail(call(
        0,
        Some(l(0)),
        ParserProgramValueList {
            values: vec![],
            tail: Some(Box::new(ParserProgramPack::Varargs)),
        },
    ))];
    data.programs[2].parameter_count = 3;
    data.programs[2].local_count = 5;
    data.programs[2].bindings = vec![
        ParserProgramBinding::DynamicCall {},
        ParserProgramBinding::Intrinsic {
            operation: ParserProgramIntrinsic::TableInsert,
            source: ParserProgramIntrinsicSource::OriginalGlobal,
        },
    ];
    data.programs[2].body = vec![
        s(ParserProgramStatementKind::ForEach {
            locals: vec![3, 4],
            iterator: ParserProgramIterator::Generic {
                values: ParserProgramValueList {
                    values: vec![],
                    tail: Some(Box::new(ParserProgramPack::Call {
                        call: call(0, Some(l(0)), list(vec![l(1)])),
                    })),
                },
            },
            body: vec![s(ParserProgramStatementKind::TableAppend {
                binding: 1,
                table: l(2),
                value: l(4),
            })],
        }),
        ret(vec![l(2)]),
    ];
    data.programs[3].parameter_count = 3;
    data.programs[3].local_count = 3;
    data.programs[3].body = vec![
        s(ParserProgramStatementKind::TableSet {
            table: l(0),
            key: l(1),
            value: l(2),
        }),
        ret(vec![]),
    ];
    data.programs[4].parameter_count = 1;
    data.programs[4].local_count = 1;
    data.programs[4].body = vec![
        s(ParserProgramStatementKind::TableSet {
            table: l(0),
            key: b("mark"),
            value: n(1.0),
        }),
        ret(vec![n(99.0)]),
    ];
    data.programs[5].parameter_count = 4;
    data.programs[5].local_count = 4;
    data.programs[5].bindings = vec![
        ParserProgramBinding::DynamicCall {},
        ParserProgramBinding::CapturedCallback {
            upvalue: 0,
            callback: SourceCallbackId(5),
        },
    ];
    data.programs[5].body = vec![tail(call(
        0,
        Some(l(0)),
        list(vec![
            l(1),
            l(2),
            e(ParserProgramExprKind::Call {
                call: Box::new(call(1, None, list(vec![l(3)]))),
            }),
        ]),
    ))];
    CompiledSourcePrograms::new(&SourceProgramCatalog::new(data, owner).unwrap()).unwrap()
}
fn arguments(session: &mut ProgramSession, values: Vec<ProgramValue>) -> Vec<SessionValue> {
    session.borrow(&graph(values)).unwrap()
}
fn callback(session: &mut ProgramSession, id: SourceCallbackId) -> SessionValue {
    arguments(session, vec![ProgramValue::Callback(id)]).remove(0)
}
fn output(session: &mut ProgramSession, values: &[SessionValue]) -> Vec<ProgramValue> {
    session.snapshot(values).unwrap().graph().values.clone()
}
fn owned_table(
    session: &mut ProgramSession,
    entries: Vec<(ProgramValue, ProgramValue)>,
) -> SessionValue {
    session
        .import_with_coverage(
            &ProgramValueGraph {
                values: vec![ProgramValue::Table(ProgramTableId(1))],
                tables: vec![ProgramTable { entries }],
            },
            &ProgramTableCoverage::new(),
        )
        .unwrap()
        .remove(0)
}
fn step(
    session: &mut ProgramSession,
    aux: &SessionValue,
    table: &SessionValue,
    control: ProgramValue,
) -> Result<Vec<SessionValue>, ProgramRuntimeError> {
    let control = arguments(session, vec![control]).remove(0);
    session.invoke_callable(aux, &[table.clone(), control])
}
fn write(
    session: &mut ProgramSession,
    table: &SessionValue,
    key: ProgramValue,
    value: ProgramValue,
) {
    let args = arguments(session, vec![key, value]);
    session
        .invoke(
            SourceCallbackId(4),
            &[table.clone(), args[0].clone(), args[1].clone()],
        )
        .unwrap();
}
#[test]
fn exact_factory_edges_preserve_auxiliary_table_identity_without_operation_fallback() {
    let lib = fixture();
    let (mut session, _) = lib
        .session(&graph(vec![]), ProgramLimits::default())
        .unwrap();
    let table = owned_table(&mut session, vec![]);
    let factory = callback(&mut session, FACTORY);
    let first = session
        .invoke_callable(&factory, std::slice::from_ref(&table))
        .unwrap();
    let second = session
        .invoke(SourceCallbackId(1), std::slice::from_ref(&table))
        .unwrap();
    let other = callback(&mut session, OTHER);
    let alternate = session
        .invoke_callable(&other, std::slice::from_ref(&table))
        .unwrap();
    for result in [&first, &second] {
        assert_eq!(
            output(&mut session, result),
            vec![
                ProgramValue::Callback(AUX),
                ProgramValue::Table(ProgramTableId(1)),
                ProgramValue::Number(0.0)
            ]
        );
    }
    assert_eq!(
        output(&mut session, &alternate)[0],
        ProgramValue::Callback(SourceCallbackId(9))
    );
    let aliases = session
        .snapshot(&[table, first[1].clone(), second[1].clone()])
        .unwrap();
    assert!(
        aliases
            .graph()
            .values
            .iter()
            .all(|v| *v == ProgramValue::Table(ProgramTableId(1)))
    );
    assert!(
        session
            .invoke_callable(&first[0], &first[1..])
            .unwrap()
            .is_empty()
    );
    let keys = owned_table(&mut session, vec![]);
    session
        .invoke(
            SourceCallbackId(4),
            &[keys.clone(), first[0].clone(), second[0].clone()],
        )
        .unwrap();
    let out = session.snapshot(&[keys]).unwrap();
    assert_eq!(
        out.graph().tables[0].entries,
        vec![(ProgramValue::Callback(AUX), ProgramValue::Callback(AUX))]
    );
}
#[test]
fn auxiliary_reads_current_raw_values_stops_only_on_nil_and_never_holds_a_cursor() {
    let lib = fixture();
    let (mut session, _) = lib
        .session(&graph(vec![]), ProgramLimits::default())
        .unwrap();
    let aux = callback(&mut session, AUX);
    let input = owned_table(
        &mut session,
        vec![
            (ProgramValue::Number(1.0), ProgramValue::Boolean(false)),
            (ProgramValue::Number(3.0), text("third")),
        ],
    );
    let first = step(&mut session, &aux, &input, ProgramValue::Number(0.0)).unwrap();
    assert_eq!(
        output(&mut session, &first),
        vec![ProgramValue::Number(1.0), ProgramValue::Boolean(false)]
    );
    assert!(
        step(&mut session, &aux, &input, ProgramValue::Number(1.0))
            .unwrap()
            .is_empty()
    );
    write(
        &mut session,
        &input,
        ProgramValue::Number(2.0),
        text("filled"),
    );
    let result = step(&mut session, &aux, &input, ProgramValue::Number(1.0)).unwrap();
    assert_eq!(
        output(&mut session, &result),
        vec![ProgramValue::Number(2.0), text("filled")]
    );
    write(
        &mut session,
        &input,
        ProgramValue::Number(2.0),
        ProgramValue::Nil,
    );
    assert!(
        step(&mut session, &aux, &input, ProgramValue::Number(1.0))
            .unwrap()
            .is_empty()
    );
    let repeat = step(&mut session, &aux, &input, ProgramValue::Number(0.0)).unwrap();
    assert_eq!(output(&mut session, &repeat), output(&mut session, &first));
    let child = owned_table(&mut session, vec![]);
    let key = arguments(&mut session, vec![ProgramValue::Number(3.0)]).remove(0);
    session
        .invoke(SourceCallbackId(4), &[input.clone(), key, child.clone()])
        .unwrap();
    let returned = step(&mut session, &aux, &input, ProgramValue::Number(2.0)).unwrap();
    let alias = session.snapshot(&[child, returned[1].clone()]).unwrap();
    assert_eq!(alias.graph().values[0], alias.graph().values[1]);
    let factory = callback(&mut session, FACTORY);
    let out = owned_table(&mut session, vec![]);
    let result = session
        .invoke(SourceCallbackId(3), &[factory, input, out])
        .unwrap();
    let graph = session.snapshot(&result).unwrap();
    assert_eq!(
        graph.graph().tables[0].entries,
        vec![(ProgramValue::Number(1.0), ProgramValue::Boolean(false))]
    );
}
#[test]
fn auxiliary_integer_coercion_and_signed_increment_match_interpreted_original() {
    let lua = Lua::new();
    lua.load("jit.off();jit.flush()").exec().unwrap();
    let aux: mlua::Function = lua.load("return (ipairs({}))").eval().unwrap();
    let source = lua.create_table().unwrap();
    let mut entries = Vec::new();
    for key in [i32::MIN, -2, -1, 0, 1, 2, 3, i32::MAX] {
        source.raw_set(key, key).unwrap();
        entries.push((
            ProgramValue::Number(f64::from(key)),
            ProgramValue::Number(f64::from(key)),
        ));
    }
    let lib = fixture();
    let (mut session, _) = lib
        .session(&graph(vec![]), ProgramLimits::default())
        .unwrap();
    let native_aux = callback(&mut session, AUX);
    let input = owned_table(&mut session, entries);
    let controls = vec![
        ProgramValue::Number(0.0),
        ProgramValue::Number(-0.0),
        ProgramValue::Number(0.9),
        ProgramValue::Number(-0.9),
        ProgramValue::Number(-1.9),
        ProgramValue::Number(-2.9),
        ProgramValue::Number(f64::from(i32::MAX)),
        ProgramValue::Number(f64::from(i32::MIN)),
        ProgramValue::Number(f64::from(i32::MAX) + 0.75),
        ProgramValue::Number(f64::from(i32::MIN) - 0.75),
        text("0x1"),
        text(" 1.9 "),
        text("-2.9"),
        text("2147483647"),
    ];
    for control in controls {
        let actual = aux
            .call::<MultiValue>((source.clone(), lua_value(&lua, &control)))
            .unwrap()
            .into_vec()
            .into_iter()
            .map(scalar)
            .collect::<Vec<_>>();
        let native = step(&mut session, &native_aux, &input, control.clone()).unwrap();
        assert_values(&output(&mut session, &native), &actual);
    }
}
#[test]
fn auxiliary_required_arguments_check_in_source_order_and_nonportable_inputs_stay_frontiers() {
    let lua = Lua::new();
    let aux: mlua::Function = lua.load("return (ipairs({}))").eval().unwrap();
    let source = lua.create_table().unwrap();
    let lib = fixture();
    let (mut session, _) = lib
        .session(&graph(vec![]), ProgramLimits::default())
        .unwrap();
    let native_aux = callback(&mut session, AUX);
    let input = owned_table(&mut session, vec![]);
    for control in [
        ProgramValue::Nil,
        ProgramValue::Boolean(false),
        text("bad"),
        text("1\0tail"),
    ] {
        assert!(
            aux.call::<MultiValue>((source.clone(), lua_value(&lua, &control)))
                .is_err()
        );
        assert_eq!(
            step(&mut session, &native_aux, &input, control)
                .unwrap_err()
                .kind,
            ProgramRuntimeErrorKind::Source
        );
    }
    assert_eq!(
        session
            .invoke_callable(&native_aux, std::slice::from_ref(&input))
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::Source
    );
    let invalid = arguments(
        &mut session,
        vec![ProgramValue::Nil, ProgramValue::Number(f64::INFINITY)],
    );
    let error = session.invoke_callable(&native_aux, &invalid).unwrap_err();
    assert_eq!(error.kind, ProgramRuntimeErrorKind::Source);
    assert!(error.message.contains("table"));
    for control in [
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        f64::from(i32::MAX) + 1.0,
        f64::from(i32::MIN) - 1.0,
    ] {
        assert_eq!(
            step(
                &mut session,
                &native_aux,
                &input,
                ProgramValue::Number(control)
            )
            .unwrap_err()
            .kind,
            ProgramRuntimeErrorKind::UnsupportedCapability
        );
    }
}
#[test]
fn auxiliary_ignores_index_fallback_but_preserves_unknown_raw_coverage() {
    let lib = fixture();
    let (mut session, _) = lib
        .session(&graph(vec![]), ProgramLimits::default())
        .unwrap();
    let coverage = SourceTableCoverage {
        inventory: SourceTableInventory::Selective,
        known_absent: BTreeSet::from([SourceTableKey::Integer(1)]),
        unavailable: BTreeSet::new(),
        index_fallback: SourceTableIndexFallback::Unavailable,
        call_fallback: SourceTableCallFallback::NonCallable,
    };
    let input = session
        .import_with_coverage(
            &ProgramValueGraph {
                values: vec![ProgramValue::Table(ProgramTableId(1))],
                tables: vec![ProgramTable::default()],
            },
            &BTreeMap::from([(ProgramTableId(1), coverage)]),
        )
        .unwrap()
        .remove(0);
    let aux = callback(&mut session, AUX);
    assert!(
        step(&mut session, &aux, &input, ProgramValue::Number(0.0))
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        step(&mut session, &aux, &input, ProgramValue::Number(1.0))
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    write(
        &mut session,
        &input,
        ProgramValue::Number(2.0),
        ProgramValue::Boolean(false),
    );
    let values = step(&mut session, &aux, &input, ProgramValue::Number(1.0)).unwrap();
    assert_eq!(
        output(&mut session, &values),
        vec![ProgramValue::Number(2.0), ProgramValue::Boolean(false)]
    );
    write(
        &mut session,
        &input,
        ProgramValue::Number(2.0),
        ProgramValue::Nil,
    );
    assert!(
        step(&mut session, &aux, &input, ProgramValue::Number(1.0))
            .unwrap()
            .is_empty()
    );
}
#[test]
fn ignored_argument_effects_complete_before_auxiliary_or_factory_errors() {
    let lib = fixture();
    let (mut session, _) = lib
        .session(&graph(vec![]), ProgramLimits::default())
        .unwrap();
    let target = owned_table(
        &mut session,
        vec![(ProgramValue::Number(1.0), ProgramValue::Boolean(false))],
    );
    for (id, bad_table, bad_control) in [
        (AUX, false, true),
        (AUX, true, false),
        (FACTORY, true, false),
        (FACTORY, false, false),
    ] {
        let effect = owned_table(&mut session, vec![]);
        let callable = callback(&mut session, id);
        let nil = arguments(&mut session, vec![ProgramValue::Nil]).remove(0);
        let zero = arguments(&mut session, vec![ProgramValue::Number(0.0)]).remove(0);
        let result = session.invoke(
            SourceCallbackId(6),
            &[
                callable,
                if bad_table {
                    nil.clone()
                } else {
                    target.clone()
                },
                if bad_control { nil } else { zero },
                effect.clone(),
            ],
        );
        if bad_table || bad_control {
            assert_eq!(result.unwrap_err().kind, ProgramRuntimeErrorKind::Source);
        } else {
            assert_eq!(result.unwrap().len(), 3);
        }
        let observed = session.snapshot(&[effect]).unwrap();
        assert_eq!(
            observed.graph().tables[0].entries,
            vec![(text("mark"), ProgramValue::Number(1.0))]
        );
    }
}
#[test]
fn auxiliary_and_factory_enforce_result_conversion_and_cumulative_work_budgets() {
    let lib = fixture();
    let limits = ProgramLimits {
        max_results: 2,
        ..ProgramLimits::default()
    };
    let (mut session, _) = lib.session(&graph(vec![]), limits).unwrap();
    let input = owned_table(&mut session, vec![]);
    let factory = callback(&mut session, FACTORY);
    assert_eq!(
        session
            .invoke_callable(&factory, std::slice::from_ref(&input))
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
    let mut limits = ProgramLimits::default();
    limits.pattern.max_steps = 3;
    let (mut session, _) = lib.session(&graph(vec![]), limits).unwrap();
    let input = owned_table(
        &mut session,
        vec![(ProgramValue::Number(1.0), ProgramValue::Boolean(false))],
    );
    let aux = callback(&mut session, AUX);
    let out = step(&mut session, &aux, &input, text("0")).unwrap();
    assert_eq!(out.len(), 2);
    assert_eq!(session.pattern_steps(), 2);
    assert_eq!(
        step(&mut session, &aux, &input, text("0"))
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
    assert!(session.pattern_steps() > 3);
}
#[test]
fn shared_auxiliary_callback_has_no_cross_session_cursor_or_handle_authority() {
    let lib = fixture();
    let (mut first, _) = lib
        .session(&graph(vec![]), ProgramLimits::default())
        .unwrap();
    let foreign = callback(&mut first, AUX);
    std::thread::scope(|scope| {
        for worker in 0..4 {
            let lib = &lib;
            let foreign = &foreign;
            scope.spawn(move || {
                let (mut session, _) = lib
                    .session(&graph(vec![]), ProgramLimits::default())
                    .unwrap();
                let aux = callback(&mut session, AUX);
                let input = owned_table(
                    &mut session,
                    vec![(
                        ProgramValue::Number(1.0),
                        ProgramValue::Number(f64::from(worker)),
                    )],
                );
                assert_eq!(
                    session
                        .invoke_callable(foreign, std::slice::from_ref(&input))
                        .unwrap_err()
                        .kind,
                    ProgramRuntimeErrorKind::InvalidInput
                );
                for _ in 0..8 {
                    let out = step(&mut session, &aux, &input, ProgramValue::Number(0.0)).unwrap();
                    assert_eq!(
                        output(&mut session, &out),
                        vec![
                            ProgramValue::Number(1.0),
                            ProgramValue::Number(f64::from(worker))
                        ]
                    );
                }
            });
        }
    });
}

#[test]
fn unbound_implicit_ipairs_retains_its_escape_frontier() {
    let library = primitive(ParserProgramIntrinsic::Ipairs);
    let input = ProgramValueGraph {
        values: vec![ProgramValue::Table(ProgramTableId(1))],
        tables: vec![ProgramTable::default()],
    };
    let error = library
        .execute(SourceCallbackId(1), &input, ProgramLimits::default())
        .unwrap_err();
    assert_eq!(error.kind, ProgramRuntimeErrorKind::UnsupportedCapability);
    assert_eq!(error.message, "escaped ipairs iterator");
}
