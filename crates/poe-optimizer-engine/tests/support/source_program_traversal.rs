//! Raw immutable traversal order and retained iterator callback identity.
use super::*;
fn fixture(order: Vec<SourceTableKey>) -> CompiledSourcePrograms {
    let base = compile(
        vec![
            (1, 6, false, vec![], vec![]),
            (2, 1, false, vec![], vec![]),
            (3, 2, false, vec![], vec![]),
        ],
        vec![vec![], vec![], vec![]],
    );
    let mut definitions = base.catalog().owner().definitions().unwrap().clone();
    definitions.tables = vec![
        SourceTable {
            fields: BTreeMap::from([
                ("z".into(), SourceValue::Number(1.0)),
                ("a".into(), SourceValue::Table(SourceTableId(2))),
                ("shared".into(), SourceValue::Table(SourceTableId(2))),
            ]),
            indexed: BTreeMap::from([
                (0, SourceValue::Number(4.0)),
                (-1, SourceValue::Number(3.0)),
                (2, SourceValue::Number(5.0)),
            ]),
        },
        SourceTable::default(),
        SourceTable::default(),
    ];
    definitions.roots = (1..=3)
        .map(|id| SourceProgramRoot {
            name: format!("table{id}"),
            table: SourceTableId(id),
        })
        .collect();
    for (id, symbol, operation) in [
        (4, "pairs", ParserProgramIntrinsic::Pairs),
        (5, "next", ParserProgramIntrinsic::Next),
        (6, "next", ParserProgramIntrinsic::Next),
    ] {
        definitions.callbacks.push(SourceCallback {
            kind: SourceCallbackKind::Builtin {
                symbol: symbol.into(),
            },
            environment: SourceEnvironment::OriginalGlobals,
            upvalues: vec![],
        });
        definitions
            .intrinsics
            .insert(SourceCallbackId(id), operation);
    }
    definitions.callbacks[1].upvalues.push(SourceUpvalue {
        name: "capturedPairs".into(),
        value: SourceValue::Callback(SourceCallbackId(4)),
    });
    let owner = SourceProgramOwner::new_with_context(
        definitions,
        None,
        SourceProgramContext {
            iteration: Some(SourceProgramIteration {
                ipairs_aux: BTreeMap::new(),
                table_order: BTreeMap::from([
                    (SourceTableId(1), order),
                    (SourceTableId(2), vec![]),
                ]),
                pairs_next: BTreeMap::from([(SourceCallbackId(4), SourceCallbackId(6))]),
            }),
            ..SourceProgramContext::default()
        },
    )
    .unwrap();
    let mut data = base.catalog().data().clone();
    data.programs[0].parameter_count = 3;
    data.programs[0].bindings = vec![
        ParserProgramBinding::DynamicCall {},
        ParserProgramBinding::Intrinsic {
            operation: ParserProgramIntrinsic::TableInsert,
            source: ParserProgramIntrinsicSource::OriginalGlobal,
        },
    ];
    data.programs[0].body = vec![
        s(ParserProgramStatementKind::ForEach {
            locals: vec![3, 4],
            iterator: ParserProgramIterator::Generic {
                values: ParserProgramValueList {
                    values: vec![],
                    tail: Some(Box::new(ParserProgramPack::Call {
                        call: call(0, Some(l(1)), list(vec![l(0)])),
                    })),
                },
            },
            body: vec![s(ParserProgramStatementKind::TableAppend {
                binding: 1,
                table: l(2),
                value: l(3),
            })],
        }),
        ret(vec![l(2)]),
    ];
    data.programs[1].bindings = vec![ParserProgramBinding::Intrinsic {
        operation: ParserProgramIntrinsic::Pairs,
        source: ParserProgramIntrinsicSource::Captured {
            upvalue: 0,
            callback: SourceCallbackId(4),
        },
    }];
    data.programs[1].body = vec![tail(call(0, None, list(vec![l(0)])))];
    data.programs[2].bindings = vec![ParserProgramBinding::Intrinsic {
        operation: ParserProgramIntrinsic::Next,
        source: ParserProgramIntrinsicSource::OriginalGlobal,
    }];
    data.programs[2].body = vec![tail(call(0, None, list(vec![l(0), l(1)])))];
    CompiledSourcePrograms::new(&SourceProgramCatalog::new(data, owner).unwrap()).unwrap()
}
fn order() -> Vec<SourceTableKey> {
    vec![
        SourceTableKey::Integer(0),
        SourceTableKey::Integer(2),
        SourceTableKey::Text("z".into()),
        SourceTableKey::Integer(-1),
        SourceTableKey::Text("shared".into()),
        SourceTableKey::Text("a".into()),
    ]
}
fn roots(lib: &CompiledSourcePrograms, session: &mut ProgramSession) -> Vec<SessionValue> {
    (1..=3)
        .map(|id| {
            session
                .definition(
                    &lib.catalog()
                        .owner()
                        .bind_root(SourceProgramDefinitionRoot::Named(SourceProgramRootId(id)))
                        .unwrap(),
                )
                .unwrap()
        })
        .collect()
}
fn scalar_out(session: &mut ProgramSession, result: &[SessionValue]) -> Vec<ProgramValue> {
    session.snapshot(result).unwrap().graph().values.clone()
}
fn key_value(key: &SourceTableKey) -> ProgramValue {
    match key {
        SourceTableKey::Text(v) => text(v),
        SourceTableKey::Integer(v) => ProgramValue::Number(*v as f64),
    }
}

#[test]
fn pairs_returns_exact_retained_callback_state_and_nil_for_dynamic_and_captured_calls() {
    let lib = fixture(order());
    let (mut session, _) = lib
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    let tables = roots(&lib, &mut session);
    let pair = session
        .borrow(&graph(vec![ProgramValue::Callback(SourceCallbackId(4))]))
        .unwrap()
        .remove(0);
    let dynamic = session.invoke_callable(&pair, &tables[..1]).unwrap();
    let captured = session.invoke(SourceCallbackId(2), &tables[..1]).unwrap();
    for result in [dynamic, captured] {
        let out = scalar_out(&mut session, &result);
        assert_eq!(out.len(), 3);
        assert_eq!(out[0], ProgramValue::Callback(SourceCallbackId(6)));
        assert_eq!(out[2], ProgramValue::Nil);
        let next = session.invoke_callable(&result[0], &result[1..]).unwrap();
        assert_eq!(
            scalar_out(&mut session, &next),
            vec![ProgramValue::Number(0.0), ProgramValue::Number(4.0)]
        );
    }
}

#[test]
fn next_and_generic_pairs_preserve_observed_mixed_key_order_and_table_aliases() {
    let lib = fixture(order());
    let (mut session, _) = lib
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    let tables = roots(&lib, &mut session);
    let ids = session
        .borrow(&graph(vec![
            ProgramValue::Callback(SourceCallbackId(4)),
            ProgramValue::Callback(SourceCallbackId(6)),
            ProgramValue::Nil,
        ]))
        .unwrap();
    let mut control = ids[2].clone();
    let mut keys = Vec::new();
    let mut shared = Vec::new();
    loop {
        let result = session
            .invoke_callable(&ids[1], &[tables[0].clone(), control])
            .unwrap();
        if result.len() == 1 {
            assert_eq!(scalar_out(&mut session, &result), vec![ProgramValue::Nil]);
            break;
        }
        assert_eq!(result.len(), 2);
        let key = scalar_out(&mut session, &result[..1]).remove(0);
        if key == text("shared") || key == text("a") {
            shared.push(result[1].clone());
        }
        keys.push(key);
        control = result[0].clone();
    }
    assert_eq!(keys, order().iter().map(key_value).collect::<Vec<_>>());
    let alias = session.snapshot(&shared).unwrap();
    assert_eq!(alias.graph().values[0], alias.graph().values[1]);
    assert_eq!(alias.graph().tables.len(), 1);
    let output = session
        .import_with_coverage(
            &ProgramValueGraph {
                values: vec![ProgramValue::Table(ProgramTableId(1))],
                tables: vec![ProgramTable::default()],
            },
            &ProgramTableCoverage::new(),
        )
        .unwrap()
        .remove(0);
    let result = session
        .invoke(
            SourceCallbackId(1),
            &[tables[0].clone(), ids[0].clone(), output],
        )
        .unwrap();
    let output = session.snapshot(&result).unwrap();
    assert_eq!(
        output.graph().tables[0]
            .entries
            .iter()
            .map(|(_, v)| v.clone())
            .collect::<Vec<_>>(),
        keys
    );
    let negative_zero = session
        .borrow(&graph(vec![ProgramValue::Number(-0.0)]))
        .unwrap();
    let result = session
        .invoke(
            SourceCallbackId(3),
            &[tables[0].clone(), negative_zero[0].clone()],
        )
        .unwrap();
    assert_eq!(
        scalar_out(&mut session, &result),
        vec![ProgramValue::Number(2.0), ProgramValue::Number(5.0)]
    );
}

#[test]
fn explicit_empty_order_differs_from_missing_order_and_mutable_table_traversal() {
    let lib = fixture(order());
    let (mut session, _) = lib
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    let tables = roots(&lib, &mut session);
    let result = session.invoke(SourceCallbackId(3), &tables[1..2]).unwrap();
    assert_eq!(scalar_out(&mut session, &result), vec![ProgramValue::Nil]);
    assert_eq!(
        session
            .invoke(SourceCallbackId(3), &tables[2..3])
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    let mutable = session
        .import_with_coverage(
            &ProgramValueGraph {
                values: vec![ProgramValue::Table(ProgramTableId(1))],
                tables: vec![ProgramTable::default()],
            },
            &ProgramTableCoverage::new(),
        )
        .unwrap();
    let triple = session.invoke(SourceCallbackId(2), &mutable).unwrap();
    assert_eq!(triple.len(), 3);
    assert_eq!(
        session
            .invoke_callable(&triple[0], &triple[1..])
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    for value in [
        text("absent"),
        ProgramValue::Boolean(false),
        ProgramValue::Number(0.5),
    ] {
        let control = session.borrow(&graph(vec![value])).unwrap();
        assert_eq!(
            session
                .invoke(
                    SourceCallbackId(3),
                    &[tables[0].clone(), control[0].clone()]
                )
                .unwrap_err()
                .kind,
            ProgramRuntimeErrorKind::UnsupportedCapability
        );
    }
    let nan = session
        .borrow(&graph(vec![ProgramValue::Number(f64::NAN)]))
        .unwrap();
    assert_eq!(
        session
            .invoke(SourceCallbackId(3), &[tables[0].clone(), nan[0].clone()])
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::Source
    );
    let scalar = session
        .borrow(&graph(vec![ProgramValue::Boolean(false)]))
        .unwrap();
    for callback in [2, 3] {
        assert_eq!(
            session
                .invoke(SourceCallbackId(callback), &scalar)
                .unwrap_err()
                .kind,
            ProgramRuntimeErrorKind::Source
        );
    }
}

#[test]
fn next_comparison_work_and_pairs_result_capacity_remain_bounded() {
    let lib = fixture(order());
    let (mut session, _) = lib
        .session(
            &ProgramValueGraph::default(),
            ProgramLimits {
                pattern: MatchLimits {
                    max_steps: 7,
                    ..MatchLimits::default()
                },
                ..ProgramLimits::default()
            },
        )
        .unwrap();
    let tables = roots(&lib, &mut session);
    let key = session.borrow(&graph(vec![text("z")])).unwrap();
    session
        .invoke(SourceCallbackId(3), &[tables[0].clone(), key[0].clone()])
        .unwrap();
    assert_eq!(
        session
            .invoke(SourceCallbackId(3), &[tables[0].clone(), key[0].clone()])
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
    let (mut limited, _) = lib
        .session(
            &ProgramValueGraph::default(),
            ProgramLimits {
                max_results: 2,
                ..ProgramLimits::default()
            },
        )
        .unwrap();
    let table = limited
        .definition(
            &lib.catalog()
                .owner()
                .bind_root(SourceProgramDefinitionRoot::Named(SourceProgramRootId(1)))
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        limited
            .invoke(SourceCallbackId(2), &[table])
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
}

#[test]
fn missing_live_controls_do_not_fabricate_errors_for_array_holes_or_deleted_hash_keys() {
    let lua = Lua::new();
    let proof: (bool, i64, i64, bool, bool, bool) = lua
        .load(
            r#"
        jit.off()
        local t = {1, 2, 3}
        t[2] = nil
        local array_ok, key, value = pcall(next, t, 2)
        t.deleted = true
        t.deleted = nil
        local hash_ok = pcall(next, t, 'deleted')
        t[false] = true
        t[false] = nil
        local boolean_ok = pcall(next, t, false)
        local nan_ok = pcall(next, t, 0/0)
        return array_ok, key, value, hash_ok, boolean_ok, nan_ok
    "#,
        )
        .eval()
        .unwrap();
    assert_eq!(proof, (true, 3, 3, true, true, false));

    let lib = fixture(order());
    let (mut session, _) = lib
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    let tables = roots(&lib, &mut session);
    // Neither a complete current raw inventory nor its observed order includes
    // source allocation capacity or retained deleted hash keys.
    for key in [
        ProgramValue::Number(1.0),
        text("deleted"),
        ProgramValue::Boolean(false),
    ] {
        let control = session.borrow(&graph(vec![key])).unwrap();
        assert_eq!(
            session
                .invoke(
                    SourceCallbackId(3),
                    &[tables[0].clone(), control[0].clone()]
                )
                .unwrap_err()
                .kind,
            ProgramRuntimeErrorKind::UnsupportedCapability
        );
    }
}
