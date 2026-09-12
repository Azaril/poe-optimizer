//! Opt-in failure evidence retains session identity without admitting traversal.
use super::*;
fn at(start: u32, end: u32, operation: ParserProgramExprKind) -> ParserProgramExpr {
    ParserProgramExpr {
        location: ParserProgramLocation { start, end },
        operation,
    }
}
fn fixture() -> CompiledSourcePrograms {
    let native = compile(
        vec![
            (
                1,
                3,
                false,
                vec![
                    ParserProgramBinding::CapturedCallback {
                        upvalue: 0,
                        callback: ParserCallbackId(2),
                    },
                    ParserProgramBinding::CapturedCallback {
                        upvalue: 1,
                        callback: ParserCallbackId(3),
                    },
                ],
                vec![
                    s(ParserProgramStatementKind::Call {
                        call: call(1, None, list(vec![l(0)])),
                    }),
                    ret(vec![at(
                        10,
                        30,
                        ParserProgramExprKind::Call {
                            call: Box::new(call(
                                0,
                                None,
                                list(vec![
                                    e(ParserProgramExprKind::Get {
                                        table: Box::new(l(0)),
                                        key: Box::new(l(1)),
                                    }),
                                    l(2),
                                ]),
                            )),
                        },
                    )]),
                ],
            ),
            (
                2,
                2,
                false,
                vec![ParserProgramBinding::Intrinsic {
                    operation: ParserProgramIntrinsic::Next,
                    source: ParserProgramIntrinsicSource::OriginalGlobal,
                }],
                vec![ret(vec![at(
                    40,
                    60,
                    ParserProgramExprKind::Call {
                        call: Box::new(call(0, None, list(vec![l(0), l(1)]))),
                    },
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
                        key: b("seen"),
                        value: n(1.0),
                    }),
                    ret(vec![]),
                ],
            ),
            (
                4,
                2,
                false,
                vec![],
                vec![ret(vec![e(ParserProgramExprKind::Binary {
                    operation: ParserProgramBinary::Equal,
                    left: Box::new(l(0)),
                    right: Box::new(l(1)),
                })])],
            ),
            (5, 0, false, vec![], vec![ret(vec![n(7.0)])]),
        ],
        vec![
            vec![
                ParserUpvalue {
                    name: "inner".into(),
                    value: ParserValue::Callback(ParserCallbackId(2)),
                },
                ParserUpvalue {
                    name: "effect".into(),
                    value: ParserValue::Callback(ParserCallbackId(3)),
                },
            ],
            vec![],
            vec![],
            vec![],
            vec![],
        ],
    );
    let mut definitions = native.catalog().owner().definitions().unwrap().clone();
    definitions.callbacks.push(SourceCallback {
        kind: SourceCallbackKind::Builtin {
            symbol: "next".into(),
        },
        environment: SourceEnvironment::OriginalGlobals,
        upvalues: vec![],
    });
    definitions
        .intrinsics
        .insert(SourceCallbackId(6), SourceProgramIntrinsic::Next);
    CompiledSourcePrograms::new(
        &SourceProgramCatalog::new(
            native.catalog().data().clone(),
            SourceProgramOwner::new(definitions).unwrap(),
        )
        .unwrap(),
    )
    .unwrap()
}
fn input(key: &str, control: ProgramValue) -> ProgramValueGraph {
    ProgramValueGraph {
        values: vec![
            ProgramValue::Table(ProgramTableId(1)),
            text(key),
            control,
            ProgramValue::Callback(ParserCallbackId(6)),
            ProgramValue::Table(ProgramTableId(2)),
            ProgramValue::Table(ProgramTableId(3)),
        ],
        tables: vec![
            ProgramTable {
                entries: vec![
                    (text("a"), ProgramValue::Table(ProgramTableId(2))),
                    (text("alias"), ProgramValue::Table(ProgramTableId(2))),
                    (text("distinct"), ProgramValue::Table(ProgramTableId(3))),
                ],
            },
            ProgramTable {
                entries: vec![(text("value"), ProgramValue::Number(1.0))],
            },
            ProgramTable {
                entries: vec![(text("value"), ProgramValue::Number(1.0))],
            },
        ],
    }
}
fn limits() -> TraversalDiagnosticLimits {
    TraversalDiagnosticLimits {
        max_frames: 2,
        max_activations: 3,
    }
}
fn same(session: &mut ProgramSession, left: SessionValue, right: SessionValue) -> bool {
    let output = session.invoke(ParserCallbackId(4), &[left, right]).unwrap();
    session.snapshot(&output).unwrap().graph().values == [ProgramValue::Boolean(true)]
}
#[test]
fn witness_preserves_shared_and_equal_distinct_inputs_after_disable() {
    let library = fixture();
    for key in ["a", "alias", "distinct"] {
        let (mut session, values) = library
            .session(
                &input(key, text("non_nil_control")),
                ProgramLimits::default(),
            )
            .unwrap();
        session.enable_traversal_diagnostics(limits()).unwrap();
        let error = session
            .invoke(ParserCallbackId(1), &values[..3])
            .unwrap_err();
        assert_eq!(error.kind, ProgramRuntimeErrorKind::UnsupportedCapability);
        let witness = session.take_traversal_failure().unwrap();
        assert!(witness.owner().is_same_owner(library.catalog().owner()));
        assert_eq!(witness.callback, error.callback);
        assert_eq!(witness.location, error.location);
        assert_eq!(witness.callback, Some(ParserCallbackId(2)));
        assert_eq!(
            witness.location,
            Some(ParserProgramLocation { start: 40, end: 60 })
        );
        assert_eq!(
            witness.frames,
            vec![
                TraversalActivation {
                    activation: 1,
                    parent_activation: None,
                    callback: ParserCallbackId(1),
                    location: Some(ParserProgramLocation { start: 10, end: 30 })
                },
                TraversalActivation {
                    activation: 3,
                    parent_activation: Some(1),
                    callback: ParserCallbackId(2),
                    location: Some(ParserProgramLocation { start: 40, end: 60 })
                },
            ]
        );
        assert!(session.take_traversal_failure().is_none());
        session.disable_traversal_diagnostics();
        let joint = session
            .snapshot(&[
                values[0].clone(),
                witness.table.clone(),
                witness.control.clone(),
            ])
            .unwrap();
        assert_eq!(joint.graph().tables.len(), 3);
        assert_eq!(joint.graph().values[2], text("non_nil_control"));
        let child = if key == "distinct" { 5 } else { 4 };
        assert!(same(
            &mut session,
            witness.table.clone(),
            values[child].clone()
        ));
        assert!(!same(
            &mut session,
            witness.table.clone(),
            values[if child == 4 { 5 } else { 4 }].clone()
        ));
        let (mut foreign, _) = library
            .session(&ProgramValueGraph::default(), ProgramLimits::default())
            .unwrap();
        assert_eq!(
            foreign.snapshot(&[witness.table]).unwrap_err().kind,
            ProgramRuntimeErrorKind::InvalidInput
        );
    }
}
#[test]
fn disabled_diagnostics_do_not_change_errors_steps_or_invocation_allocations() {
    let library = fixture();
    let (mut plain, values) = library
        .session(&input("a", ProgramValue::Nil), ProgramLimits::default())
        .unwrap();
    let before = plain.allocations();
    let steps = plain.steps();
    let work = plain.pattern_steps();
    let expected = plain.invoke(ParserCallbackId(1), &values[..3]).unwrap_err();
    let after = plain.allocations();
    assert!(plain.traversal_failure().is_none());
    assert!(plain.take_traversal_failure().is_none());
    let (mut disabled, other) = library
        .session(&input("a", ProgramValue::Nil), ProgramLimits::default())
        .unwrap();
    disabled.enable_traversal_diagnostics(limits()).unwrap();
    disabled.disable_traversal_diagnostics();
    let other_before = disabled.allocations();
    let other_steps = disabled.steps();
    let other_work = disabled.pattern_steps();
    assert_eq!(
        disabled
            .invoke(ParserCallbackId(1), &other[..3])
            .unwrap_err(),
        expected
    );
    assert!(disabled.traversal_failure().is_none());
    let other_after = disabled.allocations();
    assert_eq!(
        after.values - before.values,
        other_after.values - other_before.values
    );
    assert_eq!(
        after.bytes - before.bytes,
        other_after.bytes - other_before.bytes
    );
    assert_eq!(
        after.tables - before.tables,
        other_after.tables - other_before.tables
    );
    assert_eq!(plain.steps() - steps, disabled.steps() - other_steps);
    assert_eq!(
        plain.pattern_steps() - work,
        disabled.pattern_steps() - other_work
    );
}
#[test]
fn enabled_diagnostics_preserve_the_error_and_prior_effects_with_explicit_costs() {
    let library = fixture();
    let (mut plain, values) = library
        .session(&input("a", ProgramValue::Nil), ProgramLimits::default())
        .unwrap();
    let original = plain.invoke(ParserCallbackId(1), &values[..3]).unwrap_err();
    let (mut traced, other) = library
        .session(&input("a", ProgramValue::Nil), ProgramLimits::default())
        .unwrap();
    traced.enable_traversal_diagnostics(limits()).unwrap();
    let observed = traced.invoke(ParserCallbackId(1), &other[..3]).unwrap_err();
    assert_eq!(observed, original);
    assert_eq!(traced.steps(), plain.steps());
    assert_eq!(traced.pattern_steps() - plain.pattern_steps(), 7); //3entries+2frames+2handles
    let witness = traced.take_traversal_failure().unwrap();
    assert_eq!(
        traced.snapshot(&[witness.control]).unwrap().graph().values,
        [ProgramValue::Nil]
    );
    assert_eq!(
        plain.snapshot(&values[..1]).unwrap().graph(),
        traced.snapshot(&other[..1]).unwrap().graph()
    );
}
#[test]
fn each_public_invocation_clears_stale_witness_and_resets_activation_bounds() {
    let library = fixture();
    let (mut session, values) = library
        .session(&input("a", ProgramValue::Nil), ProgramLimits::default())
        .unwrap();
    session.enable_traversal_diagnostics(limits()).unwrap();
    for _ in 0..3 {
        assert_eq!(
            session
                .invoke(ParserCallbackId(1), &values[..3])
                .unwrap_err()
                .kind,
            ProgramRuntimeErrorKind::UnsupportedCapability
        );
        assert_eq!(session.traversal_failure().unwrap().frames[1].activation, 3);
    }
    session.invoke(ParserCallbackId(5), &[]).unwrap();
    assert!(session.traversal_failure().is_none());
    session
        .invoke(ParserCallbackId(1), &values[..3])
        .unwrap_err();
    assert!(session.traversal_failure().is_some());
    let (mut foreign, foreign_values) = library
        .session(
            &graph(vec![ProgramValue::Number(1.0)]),
            ProgramLimits::default(),
        )
        .unwrap();
    foreign.invoke(ParserCallbackId(5), &[]).unwrap();
    assert_eq!(
        session
            .invoke(ParserCallbackId(2), &foreign_values)
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::InvalidInput
    );
    assert!(session.traversal_failure().is_none());
    session
        .invoke(ParserCallbackId(1), &values[..3])
        .unwrap_err();
    assert_eq!(
        session
            .invoke_callable(&values[3], &[values[1].clone()])
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::Source
    );
    assert!(session.traversal_failure().is_none());
    session
        .invoke(ParserCallbackId(1), &values[..3])
        .unwrap_err();
    assert_eq!(
        session
            .invoke_method(&values[4], "missing", &[])
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::Source
    );
    assert!(session.traversal_failure().is_none());
}
#[test]
fn direct_original_next_keeps_exact_arguments_without_inventing_source_frames() {
    let library = fixture();
    let (mut session, values) = library
        .session(
            &input("a", ProgramValue::Table(ProgramTableId(3))),
            ProgramLimits::default(),
        )
        .unwrap();
    session.enable_traversal_diagnostics(limits()).unwrap();
    let error = session
        .invoke_callable(&values[3], &[values[4].clone(), values[2].clone()])
        .unwrap_err();
    assert_eq!(error.kind, ProgramRuntimeErrorKind::UnsupportedCapability);
    let witness = session.take_traversal_failure().unwrap();
    assert_eq!(witness.callback, None);
    assert_eq!(witness.location, None);
    assert!(witness.frames.is_empty());
    assert!(same(&mut session, witness.table, values[4].clone()));
    assert!(same(&mut session, witness.control, values[5].clone()));
}
#[test]
fn diagnostic_overruns_are_explicit_and_do_not_publish_partial_witnesses() {
    let library = fixture();
    for diagnostic_limits in [
        TraversalDiagnosticLimits {
            max_frames: 1,
            max_activations: 3,
        },
        TraversalDiagnosticLimits {
            max_frames: 2,
            max_activations: 2,
        },
    ] {
        let (mut session, values) = library
            .session(&input("a", ProgramValue::Nil), ProgramLimits::default())
            .unwrap();
        session
            .enable_traversal_diagnostics(diagnostic_limits)
            .unwrap();
        let error = session
            .invoke(ParserCallbackId(1), &values[..3])
            .unwrap_err();
        assert_eq!(error.kind, ProgramRuntimeErrorKind::ResourceBound);
        assert!(error.message.contains("traversal diagnostic"));
        assert!(session.traversal_failure().is_none());
        // A later invocation is valid: no stale active stack survives unwinding.
        session.invoke(ParserCallbackId(5), &[]).unwrap();
    }
    let (mut reference, values) = library
        .session(&input("a", ProgramValue::Nil), ProgramLimits::default())
        .unwrap();
    reference.enable_traversal_diagnostics(limits()).unwrap();
    reference
        .invoke(ParserCallbackId(1), &values[..3])
        .unwrap_err();
    let used = reference.allocations();
    for tight in [
        ProgramLimits {
            max_bytes: used.bytes - 1,
            ..ProgramLimits::default()
        },
        ProgramLimits {
            max_values: used.values - 1,
            ..ProgramLimits::default()
        },
    ] {
        let (mut session, values) = library
            .session(&input("a", ProgramValue::Nil), tight)
            .unwrap();
        session.enable_traversal_diagnostics(limits()).unwrap();
        let error = session
            .invoke(ParserCallbackId(1), &values[..3])
            .unwrap_err();
        assert_eq!(error.kind, ProgramRuntimeErrorKind::ResourceBound);
        assert!(session.traversal_failure().is_none());
        assert!(session.take_traversal_failure().is_none());
    }
    let (mut session, _) = library
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    for invalid in [
        TraversalDiagnosticLimits {
            max_frames: 0,
            max_activations: 1,
        },
        TraversalDiagnosticLimits {
            max_frames: 129,
            max_activations: 1,
        },
        TraversalDiagnosticLimits {
            max_frames: 1,
            max_activations: 0,
        },
        TraversalDiagnosticLimits {
            max_frames: 1,
            max_activations: 1_000_001,
        },
    ] {
        let before = session.allocations();
        assert_eq!(
            session
                .enable_traversal_diagnostics(invalid)
                .unwrap_err()
                .kind,
            ProgramRuntimeErrorKind::InvalidInput
        );
        assert_eq!(session.allocations(), before);
    }
}
#[test]
fn generic_loop_failure_records_the_actual_next_call_and_loop_location() {
    let base = fixture();
    let mut definitions = base.catalog().owner().definitions().unwrap().clone();
    definitions.callbacks[4].upvalues.push(SourceUpvalue {
        name: "iterator".into(),
        value: SourceValue::Callback(SourceCallbackId(6)),
    });
    let mut data = base.catalog().data().clone();
    let plan = &mut data.programs[4];
    plan.parameter_count = 1;
    plan.local_count = 3;
    plan.body = vec![
        ParserProgramStatement {
            location: ParserProgramLocation {
                start: 100,
                end: 120,
            },
            operation: ParserProgramStatementKind::ForEach {
                locals: vec![1, 2],
                iterator: ParserProgramIterator::Generic {
                    values: list(vec![
                        e(ParserProgramExprKind::Capture { upvalue: 0 }),
                        l(0),
                        e(ParserProgramExprKind::Literal {
                            value: ParserFactoryLiteral::Nil,
                        }),
                    ]),
                },
                body: vec![],
            },
        },
        ret(vec![]),
    ];
    let library = CompiledSourcePrograms::new(
        &SourceProgramCatalog::new(data, SourceProgramOwner::new(definitions).unwrap()).unwrap(),
    )
    .unwrap();
    let (mut session, values) = library
        .session(&input("a", ProgramValue::Nil), ProgramLimits::default())
        .unwrap();
    session.enable_traversal_diagnostics(limits()).unwrap();
    let error = session
        .invoke(ParserCallbackId(5), &values[4..5])
        .unwrap_err();
    assert_eq!(error.kind, ProgramRuntimeErrorKind::UnsupportedCapability);
    let witness = session.take_traversal_failure().unwrap();
    assert_eq!(witness.frames.len(), 1);
    assert_eq!(witness.callback, Some(ParserCallbackId(5)));
    assert_eq!(
        witness.location,
        Some(ParserProgramLocation {
            start: 100,
            end: 120
        })
    );
    assert_eq!(witness.location, error.location);
    assert!(same(&mut session, witness.table, values[4].clone()));
    assert_eq!(
        session.snapshot(&[witness.control]).unwrap().graph().values,
        [ProgramValue::Nil]
    );
}
#[test]
fn actual_session_closure_and_raw_method_entries_retain_source_activations() {
    let base = fixture();
    let mut definitions = base.catalog().owner().definitions().unwrap().clone();
    for capture in &mut definitions.callbacks[0].upvalues {
        capture.value = SourceValue::LiveCapture {};
    }
    let owner = SourceProgramOwner::new_with_closures(
        definitions,
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
    let mut data = base.catalog().data().clone();
    data.programs[0].bindings = vec![
        ParserProgramBinding::DynamicCall {},
        ParserProgramBinding::DynamicCall {},
    ];
    let ParserProgramStatementKind::Call { call } = &mut data.programs[0].body[0].operation else {
        unreachable!()
    };
    call.receiver = Some(Box::new(e(ParserProgramExprKind::Capture { upvalue: 1 })));
    let ParserProgramStatementKind::Return { values } = &mut data.programs[0].body[1].operation
    else {
        unreachable!()
    };
    let ParserProgramExprKind::Call { call } = &mut values.values[0].operation else {
        unreachable!()
    };
    call.receiver = Some(Box::new(e(ParserProgramExprKind::Capture { upvalue: 0 })));
    let library =
        CompiledSourcePrograms::new(&SourceProgramCatalog::new(data, owner.clone()).unwrap())
            .unwrap();
    let mut state = input("a", ProgramValue::Nil);
    state
        .values
        .insert(0, ProgramValue::Closure(SourceSessionClosureId(1)));
    state.tables[0].entries.push((
        text("walk"),
        ProgramValue::Closure(SourceSessionClosureId(1)),
    ));
    let input = SourceSessionInput {
        owner: owner.clone(),
        state,
        coverage: Default::default(),
        traversal: None,
        class_bindings: Default::default(),
        cells: vec![
            ProgramValue::Callback(SourceCallbackId(2)),
            ProgramValue::Callback(SourceCallbackId(3)),
        ],
        closures: vec![SourceSessionClosure {
            prototype: owner
                .bind_closure_prototype(SourceClosurePrototypeId(1))
                .unwrap(),
            captures: vec![SourceSessionCellId(1), SourceSessionCellId(2)],
        }],
    };
    let (mut session, values) = library
        .session_from_input(&input, ProgramLimits::default())
        .unwrap();
    session.enable_traversal_diagnostics(limits()).unwrap();
    for method in [false, true] {
        let error = if method {
            session.invoke_method(&values[1], "walk", &values[2..4])
        } else {
            session.invoke_callable(&values[0], &values[1..4])
        }
        .unwrap_err();
        assert_eq!(error.kind, ProgramRuntimeErrorKind::UnsupportedCapability);
        let witness = session.take_traversal_failure().unwrap();
        assert_eq!(
            witness
                .frames
                .iter()
                .map(|frame| frame.callback)
                .collect::<Vec<_>>(),
            vec![SourceCallbackId(1), SourceCallbackId(2)]
        );
        assert_eq!(witness.callback, error.callback);
        assert_eq!(witness.location, error.location);
        assert!(same(&mut session, witness.table, values[5].clone()));
    }
}
