//! Public execution contracts with authored IR. These are not whole-source parity tests.
use super::*;
use crate::parser_program::{CompiledParserPrograms, tests::program_fixture};
use poe_optimizer_data::modifier_parser::*;

fn expr(operation: ParserProgramExprKind) -> ParserProgramExpr {
    ParserProgramExpr {
        location: ParserProgramLocation { start: 0, end: 1 },
        operation,
    }
}
fn stmt(operation: ParserProgramStatementKind) -> ParserProgramStatement {
    ParserProgramStatement {
        location: ParserProgramLocation { start: 0, end: 1 },
        operation,
    }
}
fn num(value: f64) -> ParserProgramExpr {
    expr(ParserProgramExprKind::Literal {
        value: ParserFactoryLiteral::Number(value),
    })
}
fn local(local: u16) -> ParserProgramExpr {
    expr(ParserProgramExprKind::Local { local })
}
fn list(values: Vec<ParserProgramExpr>) -> ParserProgramValueList {
    ParserProgramValueList { values, tail: None }
}
fn ret(values: Vec<ParserProgramExpr>) -> ParserProgramStatement {
    stmt(ParserProgramStatementKind::Return {
        values: list(values),
    })
}
fn compile(owner: ModifierParserCatalog, data: ParserProgramData) -> CompiledParserPrograms {
    CompiledParserPrograms::new(&ParserProgramCatalog::new(data, owner).unwrap()).unwrap()
}
fn input(values: Vec<ProgramValue>) -> ProgramValueGraph {
    ProgramValueGraph {
        values,
        tables: vec![],
    }
}
fn execute(
    plan: &CompiledParserPrograms,
    values: &ProgramValueGraph,
    limits: ProgramLimits,
) -> RuntimeResult<ProgramOutput> {
    plan.execute(plan.catalog().data().programs[0].callback, values, limits)
}

#[test]
fn runtime_resource_failures_reset_and_report_instruction_context() {
    let (owner, mut data) = program_fixture(1);
    let callback = data.programs[0].callback;
    data.programs[0].body = vec![
        stmt(ParserProgramStatementKind::ForNumeric {
            local: 2,
            start: num(1.0),
            limit: local(0),
            step: num(0.0),
            body: vec![],
        }),
        ret(vec![num(19.0)]),
    ];
    let plan = compile(owner, data);
    let limits = ProgramLimits {
        max_steps: 32,
        ..ProgramLimits::default()
    };
    let error = execute(&plan, &input(vec![ProgramValue::Number(2.0)]), limits).unwrap_err();
    assert_eq!(error.kind, ProgramRuntimeErrorKind::ResourceBound);
    assert_eq!(error.callback, Some(callback));
    assert_eq!(
        error.location,
        Some(ParserProgramLocation { start: 0, end: 1 })
    );
    let good = input(vec![ProgramValue::Number(0.0)]);
    let first = execute(&plan, &good, limits).unwrap();
    let fresh = execute(&plan.clone(), &good, limits).unwrap();
    assert_eq!(first.graph().values, vec![ProgramValue::Number(19.0)]);
    assert_eq!(first.graph(), fresh.graph());
    assert_eq!(first.steps(), fresh.steps());
    assert_eq!(first.allocations(), fresh.allocations());
}

#[test]
fn runtime_allocation_counters_bound_complete_export_without_partial_success() {
    let (owner, mut data) = program_fixture(1);
    data.programs[0].body = vec![ret(vec![expr(ParserProgramExprKind::Table {
        fields: vec![ParserProgramField::List {
            value: expr(ParserProgramExprKind::Bytes {
                value: b"payload".to_vec(),
            }),
        }],
    })])];
    let plan = compile(owner, data);
    let graph = ProgramValueGraph::default();
    let output = execute(&plan, &graph, ProgramLimits::default()).unwrap();
    let usage = output.allocations();
    assert!(usage.values > 0 && usage.bytes > 0 && usage.tables > 0);
    let exact = ProgramLimits {
        max_values: usage.values,
        max_bytes: usage.bytes,
        max_tables: usage.tables,
        ..ProgramLimits::default()
    };
    assert_eq!(
        execute(&plan, &graph, exact).unwrap().graph(),
        output.graph()
    );
    for limits in [
        ProgramLimits {
            max_values: usage.values - 1,
            ..exact
        },
        ProgramLimits {
            max_bytes: usage.bytes - 1,
            ..exact
        },
        ProgramLimits {
            max_tables: usage.tables - 1,
            ..exact
        },
        ProgramLimits {
            max_results: 0,
            ..exact
        },
    ] {
        assert_eq!(
            execute(&plan, &graph, limits).unwrap_err().kind,
            ProgramRuntimeErrorKind::ResourceBound
        );
    }
    assert_eq!(
        execute(&plan, &graph, exact).unwrap().graph(),
        output.graph()
    );
}

#[test]
fn runtime_reached_borrowed_writes_fail_without_mutating_input() {
    let (owner, mut data) = program_fixture(1);
    data.programs[0].body = vec![
        stmt(ParserProgramStatementKind::TableSet {
            table: local(0),
            key: num(1.0),
            value: num(99.0),
        }),
        ret(vec![local(0)]),
    ];
    let plan = compile(owner, data);
    let graph = ProgramValueGraph {
        values: vec![ProgramValue::Table(ProgramTableId(1))],
        tables: vec![ProgramTable {
            entries: vec![(ProgramValue::Number(1.0), ProgramValue::Number(7.0))],
        }],
    };
    let before = graph.clone();
    assert_eq!(
        execute(&plan, &graph, ProgramLimits::default())
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    assert_eq!(graph, before);
    let bad_receiver = input(vec![ProgramValue::Boolean(false)]);
    assert_eq!(
        execute(&plan, &bad_receiver, ProgramLimits::default())
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::Source
    );
}

// Retain source-descriptor ownership while injecting synthetic program captures.
fn helper_fixture(captured: f64) -> CompiledParserPrograms {
    let (owner, mut data) = program_fixture(2);
    let mut definitions = owner.data().clone();
    let helper = data.programs[1].callback;
    let main_closure = &mut definitions.callbacks[data.programs[0].callback.0 as usize - 1];
    let binding = main_closure.upvalues.len() as u16;
    main_closure.upvalues.push(ParserUpvalue {
        name: "synthetic_helper".into(),
        value: ParserValue::Callback(helper),
    });
    main_closure.upvalues.push(ParserUpvalue {
        name: "synthetic_value".into(),
        value: ParserValue::Number(-999.0),
    });
    let helper_closure = &mut definitions.callbacks[helper.0 as usize - 1];
    let capture = helper_closure.upvalues.len() as u16;
    helper_closure.upvalues.push(ParserUpvalue {
        name: "synthetic_value".into(),
        value: ParserValue::Number(captured),
    });
    data.programs[0].bindings = vec![ParserProgramBinding::CapturedCallback {
        upvalue: binding,
        callback: helper,
    }];
    let call = ParserProgramCall {
        binding: 0,
        receiver: None,
        arguments: list(vec![local(2)]),
    };
    data.programs[0].body = vec![
        stmt(ParserProgramStatementKind::Declare {
            locals: vec![2],
            values: list(vec![expr(ParserProgramExprKind::Table { fields: vec![] })]),
        }),
        stmt(ParserProgramStatementKind::Return {
            values: ParserProgramValueList {
                values: vec![local(2)],
                tail: Some(Box::new(ParserProgramPack::Call { call })),
            },
        }),
    ];
    data.programs[1].body = vec![
        stmt(ParserProgramStatementKind::TableSet {
            table: local(0),
            key: num(1.0),
            value: expr(ParserProgramExprKind::Capture { upvalue: capture }),
        }),
        stmt(ParserProgramStatementKind::TableSet {
            table: local(0),
            key: num(2.0),
            value: local(0),
        }),
        ret(vec![
            local(0),
            expr(ParserProgramExprKind::Literal {
                value: ParserFactoryLiteral::Nil,
            }),
        ]),
    ];
    compile(ModifierParserCatalog::new(definitions).unwrap(), data)
}

#[test]
fn runtime_helpers_preserve_aliases_and_own_captures_across_parallel_catalogs() {
    let first = helper_fixture(17.0);
    let second = helper_fixture(31.0);
    assert!(!first.catalog().is_bound_to(second.catalog().owner()));
    std::thread::scope(|scope| {
        for worker in 0..4 {
            let first = &first;
            let second = &second;
            scope.spawn(move || {
                for step in 0..32 {
                    let index = worker + step;
                    let (plan, expected) = if index % 2 == 0 {
                        (&first, 17.0)
                    } else {
                        (&second, 31.0)
                    };
                    let result = execute(
                        plan,
                        &ProgramValueGraph::default(),
                        ProgramLimits::default(),
                    )
                    .unwrap();
                    assert!(plan.catalog().is_bound_to(result.owner()));
                    assert_eq!(
                        result.graph().values,
                        vec![
                            ProgramValue::Table(ProgramTableId(1)),
                            ProgramValue::Table(ProgramTableId(1)),
                            ProgramValue::Nil
                        ]
                    );
                    let entries = &result.graph().tables[0].entries;
                    assert!(
                        entries
                            .contains(&(ProgramValue::Number(1.0), ProgramValue::Number(expected)))
                    );
                    assert!(entries.contains(&(
                        ProgramValue::Number(2.0),
                        ProgramValue::Table(ProgramTableId(1))
                    )));
                }
            });
        }
    });
}

#[test]
fn runtime_recursive_helpers_are_bounded_even_with_unlimited_caller_depth() {
    let (owner, mut data) = program_fixture(1);
    let mut definitions = owner.data().clone();
    let callback = data.programs[0].callback;
    let closure = &mut definitions.callbacks[callback.0 as usize - 1];
    let upvalue = closure.upvalues.len() as u16;
    closure.upvalues.push(ParserUpvalue {
        name: "synthetic_self".into(),
        value: ParserValue::Callback(callback),
    });
    data.programs[0].bindings = vec![ParserProgramBinding::CapturedCallback { upvalue, callback }];
    data.programs[0].body = vec![stmt(ParserProgramStatementKind::Call {
        call: ParserProgramCall {
            binding: 0,
            receiver: None,
            arguments: ParserProgramValueList::default(),
        },
    })];
    let plan = compile(ModifierParserCatalog::new(definitions).unwrap(), data);
    for depth in [3, usize::MAX] {
        let limits = ProgramLimits {
            max_call_depth: depth,
            ..ProgramLimits::default()
        };
        let error = execute(&plan, &ProgramValueGraph::default(), limits).unwrap_err();
        assert_eq!(error.kind, ProgramRuntimeErrorKind::ResourceBound);
        assert!(
            error
                .message
                .contains(if depth == 3 { "call depth" } else { "nesting" })
        );
        assert_eq!(error.callback, Some(callback));
    }
}

#[test]
fn runtime_byte_comparisons_consume_shared_scan_budget() {
    let (owner, mut data) = program_fixture(1);
    for operation in [
        ParserProgramBinary::Equal,
        ParserProgramBinary::NotEqual,
        ParserProgramBinary::LessThan,
        ParserProgramBinary::LessEqual,
    ] {
        data.programs[0].body = vec![ret(vec![expr(ParserProgramExprKind::Binary {
            operation,
            left: Box::new(local(0)),
            right: Box::new(local(1)),
        })])];
        let plan = compile(owner.clone(), data.clone());
        let args = input(vec![
            ProgramValue::Bytes(vec![b'x'; 1024]),
            ProgramValue::Bytes(vec![b'x'; 1024]),
        ]);
        let low = ProgramLimits {
            pattern: MatchLimits {
                max_steps: 1023,
                ..MatchLimits::default()
            },
            ..ProgramLimits::default()
        };
        assert_eq!(
            execute(&plan, &args, low).unwrap_err().kind,
            ProgramRuntimeErrorKind::ResourceBound
        );
        let exact = ProgramLimits {
            pattern: MatchLimits {
                max_steps: 1024,
                ..MatchLimits::default()
            },
            ..low
        };
        let good = execute(&plan, &args, exact).unwrap();
        assert_eq!(good.pattern_steps(), 1024);
        assert_eq!(
            good.graph().values,
            vec![ProgramValue::Boolean(matches!(
                operation,
                ParserProgramBinary::Equal | ParserProgramBinary::LessEqual
            ))]
        );
    }
}

#[test]
fn shared_requests_accumulate_successes_without_changing_standalone_outputs() {
    let (owner, mut data) = program_fixture(1);
    data.programs[0].body = vec![ret(vec![expr(ParserProgramExprKind::Bytes {
        value: b"value".to_vec(),
    })])];
    let plan = compile(owner, data);
    let callback = plan.catalog().data().programs[0].callback;
    let input = ProgramValueGraph::default();
    let original = plan
        .execute(callback, &input, ProgramLimits::default())
        .unwrap();
    let limits = ProgramLimits {
        max_values: original.allocations().values * 2,
        ..ProgramLimits::default()
    };
    let mut account = ProgramRequestAccounting::new(limits);
    let mut patterns = crate::lua_pattern::MatchBudget::new(limits.pattern);
    for _ in 0..2 {
        let result = plan
            .execute_shared(callback, &input, &mut account, &mut patterns)
            .unwrap();
        assert_eq!(result.graph(), original.graph());
        assert_eq!(result.steps(), original.steps());
        assert_eq!(result.allocations(), original.allocations());
    }
    assert_eq!(account.steps(), original.steps() * 2);
    assert_eq!(
        account.allocation_usage().values,
        original.allocations().values * 2
    );
    assert_eq!(
        plan.execute_shared(callback, &input, &mut account, &mut patterns)
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
    assert!(account.steps() > original.steps() * 2);
    assert_eq!(
        plan.execute(callback, &input, limits).unwrap().graph(),
        original.graph()
    );
}

#[test]
fn shared_requests_keep_input_execution_and_export_failure_charges() {
    let (owner, mut data) = program_fixture(1);
    data.programs[0].body = vec![ret(vec![expr(ParserProgramExprKind::Bytes {
        value: b"abc".to_vec(),
    })])];
    let plan = compile(owner, data);
    let callback = plan.catalog().data().programs[0].callback;
    let limits = ProgramLimits {
        max_bytes: 5,
        ..ProgramLimits::default()
    };
    let mut patterns = crate::lua_pattern::MatchBudget::new(limits.pattern);
    let mut account = ProgramRequestAccounting::new(limits);
    let invalid = ProgramValueGraph {
        values: vec![
            ProgramValue::Bytes(b"abc".to_vec()),
            ProgramValue::Table(ProgramTableId(1)),
        ],
        tables: vec![],
    };
    assert_eq!(
        plan.execute_shared(callback, &invalid, &mut account, &mut patterns)
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::InvalidInput
    );
    assert_eq!(account.allocation_usage().bytes, 3);
    assert_eq!(account.allocation_usage().values, 2);
    assert_eq!(account.steps(), 0);
    let empty = ProgramValueGraph::default();
    assert_eq!(
        plan.execute_shared(callback, &empty, &mut account, &mut patterns)
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
    assert!(account.steps() > 0);
    assert_eq!(account.allocation_usage().bytes, 3);
    // A fresh request reaches export, where the second payload copy exceeds five
    // bytes. Both its executed work and the first allocation remain charged.
    let mut fresh = ProgramRequestAccounting::new(limits);
    let failure = plan
        .execute_shared(callback, &empty, &mut fresh, &mut patterns)
        .unwrap_err();
    assert_eq!(failure.kind, ProgramRuntimeErrorKind::ResourceBound);
    assert_eq!(fresh.allocation_usage().bytes, 3);
    assert!(fresh.allocation_usage().values > 0 && fresh.steps() > 0);
    let prior = fresh.steps();
    assert!(
        plan.execute_shared(callback, &empty, &mut fresh, &mut patterns)
            .is_err()
    );
    assert!(fresh.steps() > prior);
    assert_eq!(fresh.allocation_usage().bytes, 3);
}

#[test]
fn shared_request_steps_and_pattern_work_survive_source_and_resource_errors() {
    let (owner, mut data) = program_fixture(1);
    data.programs[0].body = vec![ret(vec![expr(ParserProgramExprKind::Unary {
        operation: ParserProgramUnary::Negate,
        value: Box::new(local(0)),
    })])];
    let plan = compile(owner, data);
    let callback = plan.catalog().data().programs[0].callback;
    let limits = ProgramLimits::default();
    let mut patterns = crate::lua_pattern::MatchBudget::new(MatchLimits {
        max_steps: 8,
        ..limits.pattern
    });
    patterns.charge(2).unwrap(); // Prior parser scan work.
    let mut account = ProgramRequestAccounting::new(limits);
    let bad = input(vec![ProgramValue::Bytes(b"bad".to_vec())]);
    let error = plan
        .execute_shared(callback, &bad, &mut account, &mut patterns)
        .unwrap_err();
    assert_eq!(error.kind, ProgramRuntimeErrorKind::Source);
    assert_eq!(error.callback, Some(callback));
    assert_eq!(patterns.steps_used(), 5);
    let failed_steps = account.steps();
    let good = input(vec![ProgramValue::Bytes(b"123".to_vec())]);
    assert_eq!(
        plan.execute_shared(callback, &good, &mut account, &mut patterns)
            .unwrap()
            .graph()
            .values,
        vec![ProgramValue::Number(-123.0)]
    );
    assert_eq!(patterns.steps_used(), 8);
    assert!(account.steps() > failed_steps);
    let used = account.allocation_usage();
    assert_eq!(
        plan.execute_shared(callback, &good, &mut account, &mut patterns)
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
    assert!(account.allocation_usage().bytes > used.bytes);
    let mut fresh_pattern = crate::lua_pattern::MatchBudget::new(limits.pattern);
    let mut tiny = ProgramRequestAccounting::new(ProgramLimits {
        max_steps: 2,
        ..limits
    });
    assert_eq!(
        plan.execute_shared(callback, &good, &mut tiny, &mut fresh_pattern)
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
    assert_eq!(tiny.steps(), 2);
    assert_eq!(
        plan.execute_shared(callback, &good, &mut tiny, &mut fresh_pattern)
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
    assert_eq!(tiny.steps(), 2);
}
