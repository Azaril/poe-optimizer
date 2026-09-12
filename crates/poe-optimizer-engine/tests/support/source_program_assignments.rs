//! Structural IR tests; complete lowered-source comparisons live in the PoB crate.
use super::*;
type Operand = ParserProgramAssignmentOperand;
type Target = ParserProgramAssignmentTarget;
type Kind = ParserProgramAssignmentTargetKind;
fn live(local: u16) -> Operand {
    Operand::LocalRegister { local }
}
fn eager(value: ParserProgramExpr) -> Operand {
    Operand::Evaluated { value }
}
fn target(operation: Kind, at: u32) -> Target {
    Target {
        location: ParserProgramLocation {
            start: at,
            end: at + 1,
        },
        operation,
    }
}
fn slot(local: u16) -> Target {
    target(Kind::Local { local }, 1)
}
fn cell(upvalue: u16) -> Target {
    target(Kind::Capture { upvalue }, 2)
}
fn index(table: Operand, key: Operand) -> Target {
    target(Kind::Indexed { table, key }, 3)
}
fn mixed(targets: Vec<Target>, values: ParserProgramValueList) -> ParserProgramStatement {
    s(ParserProgramStatementKind::MixedAssign { targets, values })
}
fn tail(target: ParserProgramExpr, args: Vec<ParserProgramExpr>) -> ParserProgramValueList {
    ParserProgramValueList {
        values: vec![],
        tail: Some(Box::new(ParserProgramPack::Call {
            call: call(target, args),
        })),
    }
}
fn table_state(
    values: Vec<ProgramValue>,
    entries: Vec<(ProgramValue, ProgramValue)>,
) -> ProgramValueGraph {
    ProgramValueGraph {
        values,
        tables: vec![ProgramTable { entries }],
    }
}
fn result(
    session: &mut ProgramSession,
    closure: &SessionValue,
    args: &[SessionValue],
) -> Vec<ProgramValue> {
    let values = session.invoke_callable(closure, args).unwrap();
    session.snapshot(&values).unwrap().graph().values.clone()
}
fn field(session: &mut ProgramSession, table: &SessionValue, key: &str) -> ProgramValue {
    let out = session.snapshot(std::slice::from_ref(table)).unwrap();
    out.graph().tables[0]
        .entries
        .iter()
        .find(|(k, _)| *k == text(key))
        .map(|(_, v)| v.clone())
        .unwrap_or(ProgramValue::Nil)
}

#[test]
fn later_local_targets_preserve_conflicting_index_registers() {
    let lib = library(vec![(
        0,
        vec![
            mixed(
                vec![index(live(0), live(1)), slot(1)],
                pack(vec![n(11.0), n(2.0)]),
            ),
            ret(vec![get(l(0), n(1.0)), get(l(0), n(2.0)), l(1)]),
        ],
    )]);
    let artifact = input(
        &lib,
        &[(1, vec![])],
        vec![],
        table_state(vec![cl(1), tab(1), ProgramValue::Number(1.0)], vec![]),
    );
    let (mut session, roots) = lib
        .session_from_input(&artifact, ProgramLimits::default())
        .unwrap();
    assert_eq!(
        result(&mut session, &roots[0], &roots[1..]),
        vec![
            ProgramValue::Number(11.0),
            ProgramValue::Nil,
            ProgramValue::Number(2.0)
        ]
    );
    let source: (f64, Option<f64>, f64) = Lua::new()
        .load("local t,i={},1; t[i],i=11,2; return t[1],t[2],i")
        .eval()
        .unwrap();
    assert_eq!(source, (11.0, None, 2.0));
}

#[test]
fn duplicate_index_and_local_stores_run_right_to_left() {
    let lib = library(vec![(
        0,
        vec![
            mixed(
                vec![
                    index(live(0), eager(n(1.0))),
                    index(live(0), eager(n(1.0))),
                    slot(1),
                    slot(1),
                ],
                pack(vec![n(1.0), n(2.0), n(3.0), n(4.0)]),
            ),
            ret(vec![get(l(0), n(1.0)), l(1)]),
        ],
    )]);
    let artifact = input(
        &lib,
        &[(1, vec![])],
        vec![],
        table_state(vec![cl(1), tab(1)], vec![]),
    );
    let (mut session, roots) = lib
        .session_from_input(&artifact, ProgramLimits::default())
        .unwrap();
    assert_eq!(
        result(&mut session, &roots[0], &roots[1..]),
        vec![ProgramValue::Number(1.0), ProgramValue::Number(3.0)]
    );
    let source: (f64, f64) = Lua::new()
        .load("local t,i={},0; t[1],t[1],i,i=1,2,3,4;return t[1],i")
        .eval()
        .unwrap();
    assert_eq!(source, (1.0, 3.0));
}

#[test]
fn captured_address_values_are_eager_while_rhs_changes_shared_cells() {
    let lib = library(vec![
        (
            3,
            vec![
                mixed(
                    vec![index(eager(cap(0)), eager(cap(1))), slot(0)],
                    tail(cap(2), vec![]),
                ),
                ret(vec![l(0), get(cap(0), cap(1))]),
            ],
        ),
        (
            3,
            vec![
                store(0, vec![cap(2)]),
                store(1, vec![b("new")]),
                ret(vec![n(11.0), n(22.0)]),
            ],
        ),
    ]);
    let artifact = input(
        &lib,
        &[(1, vec![1, 2, 3]), (2, vec![1, 2, 4])],
        vec![tab(1), text("old"), cl(2), tab(2)],
        ProgramValueGraph {
            values: vec![cl(1), tab(1), tab(2)],
            tables: vec![
                ProgramTable { entries: vec![] },
                ProgramTable { entries: vec![] },
            ],
        },
    );
    let (mut session, roots) = lib
        .session_from_input(&artifact, ProgramLimits::default())
        .unwrap();
    assert_eq!(
        result(&mut session, &roots[0], &[]),
        vec![ProgramValue::Number(22.0), ProgramValue::Nil]
    );
    assert_eq!(
        field(&mut session, &roots[1], "old"),
        ProgramValue::Number(11.0)
    );
    assert_eq!(field(&mut session, &roots[2], "new"), ProgramValue::Nil);
    let source: (f64, Option<f64>, f64) = Lua::new().load("local old,new={},{}; local t,k=old,'old'; local function rhs()t,k=new,'new';return 11,22 end; local function assign()local x;t[k],x=rhs();return x,t[k] end;local x,v=assign();return x,v,old.old").eval().unwrap();
    assert_eq!(source, (22.0, None, 11.0));
}

fn trace_helper() -> Vec<ParserProgramStatement> {
    vec![
        s(ParserProgramStatementKind::TableSet {
            table: cap(0),
            key: b("trace"),
            value: binary(
                ParserProgramBinary::Add,
                binary(
                    ParserProgramBinary::Multiply,
                    get(cap(0), b("trace")),
                    n(10.0),
                ),
                l(0),
            ),
        }),
        ret(vec![l(1)]),
    ]
}
#[test]
fn address_and_rhs_effects_are_ordered_and_excess_rhs_still_runs() {
    let effect = |digit, value| invoke(l(1), vec![n(digit), value]);
    let lib = library(vec![
        (
            0,
            vec![
                mixed(
                    vec![
                        index(eager(effect(1.0, l(0))), eager(effect(2.0, b("a")))),
                        index(live(0), eager(effect(3.0, b("b")))),
                    ],
                    pack(vec![
                        effect(4.0, n(10.0)),
                        effect(5.0, n(20.0)),
                        effect(6.0, n(30.0)),
                    ]),
                ),
                ret(vec![
                    get(l(0), b("trace")),
                    get(l(0), b("a")),
                    get(l(0), b("b")),
                ]),
            ],
        ),
        (1, trace_helper()),
    ]);
    let artifact = input(
        &lib,
        &[(1, vec![]), (2, vec![1])],
        vec![tab(1)],
        table_state(
            vec![cl(1), tab(1), cl(2)],
            vec![(text("trace"), ProgramValue::Number(0.0))],
        ),
    );
    let (mut session, roots) = lib
        .session_from_input(&artifact, ProgramLimits::default())
        .unwrap();
    assert_eq!(
        result(&mut session, &roots[0], &roots[1..]),
        vec![
            ProgramValue::Number(123456.0),
            ProgramValue::Number(10.0),
            ProgramValue::Number(20.0)
        ]
    );
}

#[test]
fn rhs_results_adjust_nil_without_losing_false_or_zero() {
    for returned in [
        vec![],
        vec![
            n(0.0),
            e(ParserProgramExprKind::Literal {
                value: ParserFactoryLiteral::Boolean(false),
            }),
            n(99.0),
        ],
    ] {
        let lib = library(vec![
            (
                0,
                vec![
                    mixed(
                        vec![index(live(0), eager(b("a"))), slot(2)],
                        tail(l(1), vec![]),
                    ),
                    ret(vec![get(l(0), b("a")), l(2)]),
                ],
            ),
            (0, vec![ret(returned.clone())]),
        ]);
        let artifact = input(
            &lib,
            &[(1, vec![]), (2, vec![])],
            vec![],
            table_state(
                vec![cl(1), tab(1), cl(2)],
                vec![(text("a"), ProgramValue::Number(7.0))],
            ),
        );
        let (mut session, roots) = lib
            .session_from_input(&artifact, ProgramLimits::default())
            .unwrap();
        let expected = if returned.is_empty() {
            vec![ProgramValue::Nil, ProgramValue::Nil]
        } else {
            vec![ProgramValue::Number(0.0), ProgramValue::Boolean(false)]
        };
        assert_eq!(result(&mut session, &roots[0], &roots[1..]), expected);
    }
}

#[test]
fn store_errors_happen_after_rhs_and_preserve_only_completed_reverse_prefix() {
    for invalid_first in [true, false] {
        let bad = target(
            Kind::Indexed {
                table: eager(n(7.0)),
                key: eager(b("bad")),
            },
            50,
        );
        let good = index(live(0), eager(b("stored")));
        let targets = if invalid_first {
            vec![bad, good]
        } else {
            vec![good, bad]
        };
        let lib = library(vec![
            (
                0,
                vec![mixed(
                    targets,
                    pack(vec![invoke(l(1), vec![n(4.0), n(10.0)]), n(20.0)]),
                )],
            ),
            (1, trace_helper()),
        ]);
        let artifact = input(
            &lib,
            &[(1, vec![]), (2, vec![1])],
            vec![tab(1)],
            table_state(
                vec![cl(1), tab(1), cl(2)],
                vec![(text("trace"), ProgramValue::Number(0.0))],
            ),
        );
        let (mut session, roots) = lib
            .session_from_input(&artifact, ProgramLimits::default())
            .unwrap();
        let before = session.allocations();
        let failure = session.invoke_callable(&roots[0], &roots[1..]).unwrap_err();
        assert_eq!(failure.kind, ProgramRuntimeErrorKind::Source);
        assert_eq!(
            failure.location,
            Some(ParserProgramLocation { start: 50, end: 51 })
        );
        assert_eq!(
            field(&mut session, &roots[1], "trace"),
            ProgramValue::Number(4.0)
        );
        assert_eq!(
            field(&mut session, &roots[1], "stored"),
            if invalid_first {
                ProgramValue::Number(20.0)
            } else {
                ProgramValue::Nil
            }
        );
        assert!(session.allocations().values > before.values);
        assert!(session.allocations().bytes > before.bytes);
    }
}

#[test]
fn address_failure_prevents_rhs_but_retains_prior_address_effects() {
    let lib = library(vec![
        (
            0,
            vec![mixed(
                vec![index(
                    eager(invoke(l(1), vec![n(1.0), l(0)])),
                    eager(get(n(7.0), b("fail"))),
                )],
                pack(vec![invoke(l(1), vec![n(2.0), n(10.0)])]),
            )],
        ),
        (1, trace_helper()),
    ]);
    let artifact = input(
        &lib,
        &[(1, vec![]), (2, vec![1])],
        vec![tab(1)],
        table_state(
            vec![cl(1), tab(1), cl(2)],
            vec![(text("trace"), ProgramValue::Number(0.0))],
        ),
    );
    let (mut session, roots) = lib
        .session_from_input(&artifact, ProgramLimits::default())
        .unwrap();
    error(
        session.invoke_callable(&roots[0], &roots[1..]),
        ProgramRuntimeErrorKind::Source,
    );
    assert_eq!(
        field(&mut session, &roots[1], "trace"),
        ProgramValue::Number(1.0)
    );
}

#[test]
fn capture_targets_preserve_shared_cell_aliases_and_partial_failure_stores() {
    let lib = library(vec![
        (
            2,
            vec![mixed(
                vec![index(eager(n(7.0)), eager(b("bad"))), cell(0), cell(1)],
                pack(vec![n(1.0), n(2.0), n(3.0)]),
            )],
        ),
        (1, vec![ret(vec![cap(0)])]),
    ]);
    let artifact = input(
        &lib,
        &[(1, vec![1, 1]), (2, vec![1])],
        vec![ProgramValue::Number(0.0)],
        graph(vec![cl(1), cl(2)]),
    );
    let (mut session, roots) = lib
        .session_from_input(&artifact, ProgramLimits::default())
        .unwrap();
    error(
        session.invoke_callable(&roots[0], &[]),
        ProgramRuntimeErrorKind::Source,
    );
    assert_eq!(
        result(&mut session, &roots[1], &[]),
        vec![ProgramValue::Number(2.0)]
    );
    let (mut other, other_roots) = lib
        .session_from_input(&artifact, ProgramLimits::default())
        .unwrap();
    assert_eq!(
        result(&mut other, &other_roots[1], &[]),
        vec![ProgramValue::Number(0.0)]
    );
    error(
        other.invoke_callable(&roots[0], &[]),
        ProgramRuntimeErrorKind::InvalidInput,
    );
}

#[test]
fn function_keys_and_values_retain_exact_session_identity() {
    let lib = library(vec![
        (
            0,
            vec![
                mixed(
                    vec![index(live(0), live(1)), slot(2)],
                    pack(vec![l(1), l(1)]),
                ),
                ret(vec![binary(
                    ParserProgramBinary::Equal,
                    get(l(0), l(1)),
                    l(2),
                )]),
            ],
        ),
        (0, vec![ret(vec![])]),
    ]);
    let artifact = input(
        &lib,
        &[(1, vec![]), (2, vec![])],
        vec![],
        table_state(vec![cl(1), tab(1), cl(2)], vec![]),
    );
    let (mut session, roots) = lib
        .session_from_input(&artifact, ProgramLimits::default())
        .unwrap();
    assert_eq!(
        result(&mut session, &roots[0], &roots[1..]),
        vec![ProgramValue::Boolean(true)]
    );
    error(session.snapshot(std::slice::from_ref(&roots[1])), UNKNOWN);
}

#[test]
fn repeated_failed_assignments_exhaust_shared_budget_without_new_writes() {
    let lib = library(vec![(
        0,
        vec![mixed(
            vec![
                index(eager(n(7.0)), eager(b("bad"))),
                index(live(0), eager(b("stored"))),
            ],
            pack(vec![n(1.0), n(2.0)]),
        )],
    )]);
    let artifact = input(
        &lib,
        &[(1, vec![])],
        vec![],
        table_state(vec![cl(1), tab(1)], vec![]),
    );
    let limits = ProgramLimits {
        max_steps: 160,
        ..ProgramLimits::default()
    };
    let (mut session, roots) = lib.session_from_input(&artifact, limits).unwrap();
    let mut failures = 0;
    let mut previous = session.allocations();
    loop {
        let failure = session.invoke_callable(&roots[0], &roots[1..]).unwrap_err();
        assert!(session.allocations().values >= previous.values);
        assert!(session.allocations().bytes >= previous.bytes);
        previous = session.allocations();
        if failure.kind == ProgramRuntimeErrorKind::ResourceBound {
            break;
        }
        assert_eq!(failure.kind, ProgramRuntimeErrorKind::Source);
        failures += 1;
        assert!(failures < 20);
    }
    assert!(failures >= 2);
    assert_eq!(
        field(&mut session, &roots[1], "stored"),
        ProgramValue::Number(2.0)
    );
}

#[test]
fn assignment_scratch_is_bounded_before_any_address_effect() {
    let lib = library(vec![
        (
            0,
            vec![mixed(
                vec![index(
                    eager(invoke(l(1), vec![n(1.0), l(0)])),
                    eager(b("stored")),
                )],
                pack(vec![n(2.0)]),
            )],
        ),
        (1, trace_helper()),
        (2, vec![ret(vec![get(cap(0), cap(1))])]),
    ]);
    let artifact = input(
        &lib,
        &[(1, vec![]), (2, vec![1]), (3, vec![1, 2])],
        vec![tab(1), text("trace")],
        table_state(
            vec![cl(1), tab(1), cl(2), cl(3)],
            vec![(text("trace"), ProgramValue::Number(0.0))],
        ),
    );
    let (baseline, _) = lib
        .session_from_input(&artifact, ProgramLimits::default())
        .unwrap();
    let limits = ProgramLimits {
        max_bytes: baseline.allocations().bytes + 1,
        ..ProgramLimits::default()
    };
    let (mut session, roots) = lib.session_from_input(&artifact, limits).unwrap();
    error(
        session.invoke_callable(&roots[0], &roots[1..3]),
        ProgramRuntimeErrorKind::ResourceBound,
    );
    // This observer reuses the imported key and returns only a scalar, so it
    // needs no byte allocation after the deliberately failed reservation.
    assert_eq!(
        result(&mut session, &roots[3], &[]),
        vec![ProgramValue::Number(0.0)]
    );
    assert!(session.allocations().bytes <= limits.max_bytes);
    assert_eq!(session.allocations().tables, baseline.allocations().tables);
    assert!(session.allocations().values > baseline.allocations().values);
}

#[test]
fn read_only_target_failure_keeps_prior_private_store() {
    let lib = library(vec![(
        0,
        vec![mixed(
            vec![
                index(live(1), eager(b("field"))),
                index(live(0), eager(b("stored"))),
            ],
            pack(vec![n(1.0), n(2.0)]),
        )],
    )]);
    let artifact = input(
        &lib,
        &[(1, vec![])],
        vec![],
        table_state(vec![cl(1), tab(1)], vec![]),
    );
    let (mut session, roots) = lib
        .session_from_input(&artifact, ProgramLimits::default())
        .unwrap();
    let borrowed = session
        .borrow(&table_state(
            vec![tab(1)],
            vec![(text("field"), ProgramValue::Number(8.0))],
        ))
        .unwrap();
    error(
        session.invoke_callable(&roots[0], &[roots[1].clone(), borrowed[0].clone()]),
        UNKNOWN,
    );
    assert_eq!(
        field(&mut session, &roots[1], "stored"),
        ProgramValue::Number(2.0)
    );
    assert_eq!(
        field(&mut session, &borrowed[0], "field"),
        ProgramValue::Number(8.0)
    );
}

fn indexed_read(table: Operand, key: ParserProgramExpr) -> ParserProgramExpr {
    e(ParserProgramExprKind::IndexedRead {
        table: Box::new(table),
        key: Box::new(key),
    })
}
#[test]
fn indexed_reads_keep_computed_bases_before_key_effects_and_type_errors_after() {
    let lib = library(vec![
        (
            0,
            vec![ret(vec![indexed_read(
                eager(invoke(l(1), vec![n(1.0), l(0)])),
                invoke(l(1), vec![n(2.0), b("value")]),
            )])],
        ),
        (1, trace_helper()),
        (
            0,
            vec![ret(vec![indexed_read(
                eager(n(7.0)),
                invoke(l(1), vec![n(3.0), b("value")]),
            )])],
        ),
    ]);
    let artifact = input(
        &lib,
        &[(1, vec![]), (2, vec![1]), (3, vec![])],
        vec![tab(1)],
        table_state(
            vec![cl(1), tab(1), cl(2), cl(3)],
            vec![
                (text("trace"), ProgramValue::Number(0.0)),
                (text("value"), ProgramValue::Number(9.0)),
            ],
        ),
    );
    let (mut session, roots) = lib
        .session_from_input(&artifact, ProgramLimits::default())
        .unwrap();
    assert_eq!(
        result(&mut session, &roots[0], &roots[1..3]),
        vec![ProgramValue::Number(9.0)]
    );
    assert_eq!(
        field(&mut session, &roots[1], "trace"),
        ProgramValue::Number(12.0)
    );
    error(
        session.invoke_callable(&roots[3], &roots[1..3]),
        ProgramRuntimeErrorKind::Source,
    );
    assert_eq!(
        field(&mut session, &roots[1], "trace"),
        ProgramValue::Number(123.0)
    );
}

#[test]
fn indexed_read_of_capture_keeps_base_while_key_rebinds_the_shared_cell() {
    let lib = library(vec![
        (
            2,
            vec![ret(vec![indexed_read(
                eager(cap(0)),
                invoke(cap(1), vec![]),
            )])],
        ),
        (2, vec![store(0, vec![cap(1)]), ret(vec![b("value")])]),
    ]);
    let artifact = input(
        &lib,
        &[(1, vec![1, 2]), (2, vec![1, 3])],
        vec![tab(1), cl(2), tab(2)],
        ProgramValueGraph {
            values: vec![cl(1)],
            tables: vec![
                ProgramTable {
                    entries: vec![(text("value"), ProgramValue::Number(1.0))],
                },
                ProgramTable {
                    entries: vec![(text("value"), ProgramValue::Number(2.0))],
                },
            ],
        },
    );
    let (mut session, roots) = lib
        .session_from_input(&artifact, ProgramLimits::default())
        .unwrap();
    assert_eq!(
        result(&mut session, &roots[0], &[]),
        vec![ProgramValue::Number(1.0)]
    );
    assert_eq!(
        result(&mut session, &roots[0], &[]),
        vec![ProgramValue::Number(2.0)]
    );
}
