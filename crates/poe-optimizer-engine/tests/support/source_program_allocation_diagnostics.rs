//! Native allocation origins are diagnostic evidence, not original Lua producers.
use super::*;

fn located(start: u32, end: u32, operation: ParserProgramExprKind) -> ParserProgramExpr {
    ParserProgramExpr {
        location: ParserProgramLocation { start, end },
        operation,
    }
}
fn table(start: u32, end: u32, fields: Vec<ParserProgramField>) -> ParserProgramExpr {
    located(start, end, ParserProgramExprKind::Table { fields })
}
fn nil() -> ParserProgramExpr {
    e(ParserProgramExprKind::Literal {
        value: ParserFactoryLiteral::Nil,
    })
}
fn child_call() -> ParserProgramExpr {
    e(ParserProgramExprKind::Call {
        call: Box::new(call(0, None, list(vec![]))),
    })
}
fn child_binding() -> Vec<ParserProgramBinding> {
    vec![ParserProgramBinding::CapturedCallback {
        upvalue: 0,
        callback: ParserCallbackId(2),
    }]
}
fn child_capture() -> Vec<ParserUpvalue> {
    vec![ParserUpvalue {
        name: "child".into(),
        value: ParserValue::Callback(ParserCallbackId(2)),
    }]
}
fn fixture() -> CompiledSourcePrograms {
    compile(
        vec![
            (
                1,
                0,
                false,
                child_binding(),
                vec![ret(vec![table(
                    10,
                    30,
                    vec![ParserProgramField::Named {
                        key: "child".into(),
                        value: child_call(),
                    }],
                )])],
            ),
            (
                2,
                0,
                false,
                vec![],
                vec![ret(vec![table(
                    40,
                    60,
                    vec![ParserProgramField::Named {
                        key: "value".into(),
                        value: n(7.0),
                    }],
                )])],
            ),
            (
                3,
                1,
                false,
                vec![],
                vec![
                    s(ParserProgramStatementKind::TableSet {
                        table: l(0),
                        key: b("self"),
                        value: l(0),
                    }),
                    ret(vec![l(0), l(0)]),
                ],
            ),
            (
                4,
                1,
                false,
                vec![ParserProgramBinding::Intrinsic {
                    operation: ParserProgramIntrinsic::Next,
                    source: ParserProgramIntrinsicSource::OriginalGlobal,
                }],
                vec![tail(call(0, None, list(vec![l(0)])))],
            ),
            (
                5,
                1,
                false,
                child_binding(),
                vec![
                    s(ParserProgramStatementKind::TableSet {
                        table: l(0),
                        key: b("saved"),
                        value: table(
                            70,
                            90,
                            vec![ParserProgramField::Named {
                                key: "child".into(),
                                value: child_call(),
                            }],
                        ),
                    }),
                    ret(vec![get(nil(), "failure")]),
                ],
            ),
            (
                6,
                2,
                false,
                vec![],
                vec![ret(vec![e(ParserProgramExprKind::Get {
                    table: Box::new(l(0)),
                    key: Box::new(l(1)),
                })])],
            ),
            (
                7,
                1,
                false,
                vec![ParserProgramBinding::CapturedCallback {
                    upvalue: 0,
                    callback: ParserCallbackId(8),
                }],
                vec![ret(vec![table(
                    100,
                    110,
                    vec![
                        ParserProgramField::Named {
                            key: "first".into(),
                            value: e(ParserProgramExprKind::Call {
                                call: Box::new(call(0, None, list(vec![l(0)]))),
                            }),
                        },
                        ParserProgramField::Named {
                            key: "second".into(),
                            value: get(nil(), "failure"),
                        },
                    ],
                )])],
            ),
            (
                8,
                1,
                false,
                vec![],
                vec![
                    s(ParserProgramStatementKind::TableSet {
                        table: l(0),
                        key: b("effect"),
                        value: n(1.0),
                    }),
                    ret(vec![n(7.0)]),
                ],
            ),
            (9, 0, false, vec![], vec![ret(vec![table(90, 100, vec![])])]),
        ],
        vec![
            child_capture(),
            vec![],
            vec![],
            vec![],
            child_capture(),
            vec![],
            vec![ParserUpvalue {
                name: "effect".into(),
                value: ParserValue::Callback(ParserCallbackId(8)),
            }],
            vec![],
            vec![],
        ],
    )
}
fn enable(session: &mut ProgramSession, max_records: usize) {
    session
        .enable_allocation_diagnostics(AllocationDiagnosticLimits { max_records })
        .unwrap();
}
fn origin(session: &ProgramSession, value: &SessionValue) -> TableExpressionOrigin {
    let TableAllocationOrigin::Expression(origin) = session.table_allocation_origin(value).unwrap()
    else {
        panic!("native allocation must be observed")
    };
    origin
}
fn missing(session: &ProgramSession, value: &SessionValue) {
    assert!(matches!(
        session.table_allocation_origin(value).unwrap(),
        TableAllocationOrigin::NotObserved
    ));
}
fn field(session: &mut ProgramSession, table: &SessionValue, key: &str) -> SessionValue {
    let key = session.borrow(&graph(vec![text(key)])).unwrap().remove(0);
    session
        .invoke(ParserCallbackId(6), &[table.clone(), key])
        .unwrap()
        .remove(0)
}
fn initial_state() -> ProgramValueGraph {
    ProgramValueGraph {
        values: vec![ProgramValue::Table(ProgramTableId(1))],
        tables: vec![ProgramTable::default()],
    }
}

#[test]
fn aliases_cycles_and_equal_distinct_allocations_keep_origins_across_invocations() {
    let library = fixture();
    let (mut session, imported) = library
        .session(&initial_state(), ProgramLimits::default())
        .unwrap();
    enable(&mut session, 8);
    missing(&session, &imported[0]);
    let first = session.invoke(ParserCallbackId(2), &[]).unwrap().remove(0);
    let second = session.invoke(ParserCallbackId(2), &[]).unwrap().remove(0);
    assert_eq!(
        session
            .snapshot(std::slice::from_ref(&first))
            .unwrap()
            .graph(),
        session
            .snapshot(std::slice::from_ref(&second))
            .unwrap()
            .graph()
    );
    let a = origin(&session, &first);
    let b = origin(&session, &second);
    assert_eq!((a.ordinal, b.ordinal), (1, 2));
    assert_eq!(a.callback, ParserCallbackId(2));
    assert_eq!(a.location, ParserProgramLocation { start: 40, end: 60 });
    assert_eq!(a.location, b.location);
    assert!(a.is_bound_to(&library.clone()));
    assert!(std::ptr::eq(a.catalog().data(), library.catalog().data()));
    assert!(a.catalog().owner().is_same_owner(library.catalog().owner()));
    let aliases = session
        .invoke(ParserCallbackId(3), std::slice::from_ref(&first))
        .unwrap();
    for alias in &aliases {
        assert_eq!(origin(&session, alias).ordinal, 1);
    }
    let cycle = field(&mut session, &first, "self");
    assert_eq!(origin(&session, &cycle).ordinal, 1);
    let used = session.allocations();
    let _ = origin(&session, &first);
    assert_eq!(session.allocations(), used);
    session.disable_allocation_diagnostics();
    missing(&session, &first);
    let joint = session.snapshot(&[a.table().clone(), first]).unwrap();
    assert_eq!(joint.graph().values[0], joint.graph().values[1]);
    assert_eq!(joint.graph().tables.len(), 1);
    enable(&mut session, 2);
    missing(&session, &second);
    let next = session.invoke(ParserCallbackId(2), &[]).unwrap().remove(0);
    assert_eq!(origin(&session, &next).ordinal, 1);
}

#[test]
fn outer_allocation_precedes_nested_fields_and_survives_later_source_failure() {
    let library = fixture();
    let (mut session, state) = library
        .session(&initial_state(), ProgramLimits::default())
        .unwrap();
    enable(&mut session, 8);
    let outer = session.invoke(ParserCallbackId(1), &[]).unwrap().remove(0);
    let child = field(&mut session, &outer, "child");
    let a = origin(&session, &outer);
    let b = origin(&session, &child);
    assert_eq!((a.ordinal, a.callback), (1, ParserCallbackId(1)));
    assert_eq!(a.location, ParserProgramLocation { start: 10, end: 30 });
    assert_eq!((b.ordinal, b.callback), (2, ParserCallbackId(2)));
    assert_eq!(b.location, ParserProgramLocation { start: 40, end: 60 });
    assert_eq!(
        session
            .invoke(ParserCallbackId(5), &state)
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::Source
    );
    let escaped = field(&mut session, &state[0], "saved");
    let child = field(&mut session, &escaped, "child");
    let a = origin(&session, &escaped);
    assert_eq!((a.ordinal, a.callback), (3, ParserCallbackId(5)));
    assert_eq!(a.location, ParserProgramLocation { start: 70, end: 90 });
    assert_eq!(origin(&session, &child).ordinal, 4);
    assert_eq!(origin(&session, &outer).ordinal, 1);
}

#[test]
fn failure_witness_queries_the_exact_table_without_admitting_map_traversal() {
    let library = fixture();
    let (mut plain, _) = library
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    let value = plain.invoke(ParserCallbackId(2), &[]).unwrap().remove(0);
    let expected = plain
        .invoke(ParserCallbackId(4), std::slice::from_ref(&value))
        .unwrap_err();
    let expected_graph = plain.snapshot(std::slice::from_ref(&value)).unwrap();
    let (mut observed, _) = library
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    enable(&mut observed, 4);
    let other = observed.invoke(ParserCallbackId(2), &[]).unwrap().remove(0);
    observed
        .enable_traversal_diagnostics(TraversalDiagnosticLimits::default())
        .unwrap();
    assert_eq!(
        observed
            .invoke(ParserCallbackId(4), std::slice::from_ref(&other))
            .unwrap_err(),
        expected
    );
    assert_eq!(
        expected.kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    let failure = observed.take_traversal_failure().unwrap();
    let origin = origin(&observed, &failure.table);
    observed.disable_traversal_diagnostics();
    observed.disable_allocation_diagnostics();
    assert_eq!(origin.callback, ParserCallbackId(2));
    let joint = observed
        .snapshot(&[other, failure.table, origin.table().clone()])
        .unwrap();
    assert!(joint.graph().values.windows(2).all(|x| x[0] == x[1]));
    assert_eq!(joint.graph().tables, expected_graph.graph().tables);
}

#[test]
fn disabled_allocation_diagnostics_add_no_records_or_invocation_cost() {
    let library = fixture();
    let mut results = Vec::new();
    for previously_enabled in [false, true] {
        let (mut session, _) = library
            .session(&ProgramValueGraph::default(), ProgramLimits::default())
            .unwrap();
        if previously_enabled {
            enable(&mut session, 4);
            session.disable_allocation_diagnostics();
        }
        let before = session.allocations();
        let steps = session.steps();
        let work = session.pattern_steps();
        let values = session.invoke(ParserCallbackId(1), &[]).unwrap();
        missing(&session, &values[0]);
        let after = session.allocations();
        results.push((
            after.values - before.values,
            after.bytes - before.bytes,
            after.tables - before.tables,
            session.steps() - steps,
            session.pattern_steps() - work,
            session.snapshot(&values).unwrap().graph().clone(),
        ));
    }
    assert_eq!(results[0], results[1]);
}

#[test]
fn record_bound_precedes_allocation_and_fields_and_keeps_failed_constructor_origin() {
    let library = fixture();
    let (mut session, state) = library
        .session(&initial_state(), ProgramLimits::default())
        .unwrap();
    enable(&mut session, 1);
    // The outer allocation succeeds and is recorded before field effects/error.
    assert_eq!(
        session
            .invoke(ParserCallbackId(7), &state)
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::Source
    );
    let effect = field(&mut session, &state[0], "effect");
    assert_eq!(
        session.snapshot(&[effect]).unwrap().graph().values,
        [ProgramValue::Number(1.0)]
    );
    let tables = session.allocations().tables;
    let error = session.invoke(ParserCallbackId(2), &[]).unwrap_err();
    assert_eq!(error.kind, ProgramRuntimeErrorKind::ResourceBound);
    assert!(error.message.contains("allocation diagnostic record bound"));
    assert_eq!(session.allocations().tables, tables);

    let (mut other, state) = library
        .session(&initial_state(), ProgramLimits::default())
        .unwrap();
    enable(&mut other, 1);
    let value = other.invoke(ParserCallbackId(2), &[]).unwrap().remove(0);
    assert_eq!(
        other.invoke(ParserCallbackId(7), &state).unwrap_err().kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
    assert!(
        other.snapshot(&state).unwrap().graph().tables[0]
            .entries
            .is_empty()
    );
    assert_eq!(origin(&other, &value).ordinal, 1);
}

#[test]
fn invalid_enable_and_foreign_handles_preserve_existing_evidence() {
    let library = fixture();
    let (mut session, imported) = library
        .session(&initial_state(), ProgramLimits::default())
        .unwrap();
    let scalar = session
        .borrow(&graph(vec![ProgramValue::Nil]))
        .unwrap()
        .remove(0);
    let (foreign, foreign_values) = library
        .session(&initial_state(), ProgramLimits::default())
        .unwrap();
    for enabled in [false, true] {
        if enabled {
            enable(&mut session, 2);
        }
        for bad in [&foreign_values[0], &scalar] {
            assert_eq!(
                session.table_allocation_origin(bad).unwrap_err().kind,
                ProgramRuntimeErrorKind::InvalidInput
            );
        }
        missing(&session, &imported[0]);
    }
    let value = session.invoke(ParserCallbackId(2), &[]).unwrap().remove(0);
    for max_records in [0, 1_000_001] {
        let before = session.allocations();
        assert_eq!(
            session
                .enable_allocation_diagnostics(AllocationDiagnosticLimits { max_records })
                .unwrap_err()
                .kind,
            ProgramRuntimeErrorKind::InvalidInput
        );
        assert_eq!(session.allocations(), before);
        assert_eq!(origin(&session, &value).ordinal, 1);
    }
    // Valid limits may still exceed the cumulative session allocation budget.
    assert_eq!(
        session
            .enable_allocation_diagnostics(AllocationDiagnosticLimits {
                max_records: 1_000_000
            })
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
    let retained = origin(&session, &value);
    assert_eq!(retained.ordinal, 1);
    assert_eq!(
        foreign
            .table_allocation_origin(retained.table())
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::InvalidInput
    );
    let mut data = library.catalog().data().clone();
    data.programs[1].body = vec![ret(vec![n(42.0)])];
    let other = CompiledSourcePrograms::new(
        &SourceProgramCatalog::new(data, library.catalog().owner().clone()).unwrap(),
    )
    .unwrap();
    assert!(
        other
            .catalog()
            .owner()
            .is_same_owner(retained.catalog().owner())
    );
    assert!(!retained.is_bound_to(&other));
    assert!(!std::ptr::eq(
        retained.catalog().data(),
        other.catalog().data()
    ));
}

fn with_seed(base: &CompiledSourcePrograms, supported: bool) -> CompiledSourcePrograms {
    let mut profile = SourceTableRuntimeProfile::luajit21_x64_single();
    profile.table_bump = !supported;
    let catalog = SourceProgramCatalog::new_with_constructors(
        base.catalog().data().clone(),
        base.catalog().owner().clone(),
        SourceProgramConstructors {
            schema_version: SOURCE_PROGRAM_CONSTRUCTORS_SCHEMA_VERSION,
            profile,
            sites: vec![SourceProgramConstructor {
                callback: ParserCallbackId(9),
                provenance: base.catalog().data().programs[8].provenance.clone(),
                expression: ParserProgramLocation {
                    start: 90,
                    end: 100,
                },
                bytecode_sha256: "d".repeat(64),
                bytecode_pc: 1,
                instruction: 52,
                allocation: SourceTableAllocation::New {
                    array_slots: 0,
                    hash_bits: 0,
                },
            }],
        },
    )
    .unwrap();
    CompiledSourcePrograms::new(&catalog).unwrap()
}

#[test]
fn exact_catalog_retains_seed_evidence_but_rejected_profiles_record_no_allocation() {
    let base = fixture();
    let library = with_seed(&base, true);
    let (mut session, _) = library
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    enable(&mut session, 2);
    let seeded = session.invoke(ParserCallbackId(9), &[]).unwrap().remove(0);
    let unseeded = session.invoke(ParserCallbackId(2), &[]).unwrap().remove(0);
    let witness = origin(&session, &seeded);
    let retained = witness.catalog().constructors().unwrap();
    assert!(std::ptr::eq(
        retained,
        library.catalog().constructors().unwrap()
    ));
    assert_eq!(retained.sites[0].callback, witness.callback);
    assert_eq!(retained.sites[0].expression, witness.location);
    assert!(witness.is_bound_to(&library));
    let unseeded = origin(&session, &unseeded);
    assert!(
        !retained
            .sites
            .iter()
            .any(|s| s.callback == unseeded.callback)
    );
    let unsupported = with_seed(&base, false);
    assert!(
        unsupported
            .catalog()
            .owner()
            .is_same_owner(witness.catalog().owner())
    );
    assert!(!witness.is_bound_to(&unsupported));
    let (mut other, _) = unsupported
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    enable(&mut other, 1);
    let value = other.invoke(ParserCallbackId(2), &[]).unwrap().remove(0);
    let tables = other.allocations().tables;
    let error = other.invoke(ParserCallbackId(9), &[]).unwrap_err();
    assert_eq!(error.kind, ProgramRuntimeErrorKind::UnsupportedCapability);
    assert!(
        error
            .message
            .contains("source table runtime profile is not admitted")
    );
    assert_eq!(other.allocations().tables, tables);
    assert_eq!(origin(&other, &value).ordinal, 1);
}
