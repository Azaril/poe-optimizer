//! Authored IR mechanics over actual injected captured slots. This target does
//! not authenticate source bodies, grant public permissions or claim source parity.
use poe_optimizer_data::{game_data::bundled_snapshot, modifier_parser::*};
use poe_optimizer_engine::parser_program::*;
use std::collections::BTreeMap;

fn loc() -> ParserProgramLocation {
    ParserProgramLocation { start: 0, end: 1 }
}
fn e(operation: ParserProgramExprKind) -> ParserProgramExpr {
    ParserProgramExpr {
        location: loc(),
        operation,
    }
}
fn l(local: u16) -> ParserProgramExpr {
    e(ParserProgramExprKind::Local { local })
}
fn nil() -> ParserProgramExpr {
    e(ParserProgramExprKind::Literal {
        value: ParserFactoryLiteral::Nil,
    })
}
fn number(value: f64) -> ParserProgramExpr {
    e(ParserProgramExprKind::Literal {
        value: ParserFactoryLiteral::Number(value),
    })
}
fn values(values: Vec<ParserProgramExpr>) -> ParserProgramValueList {
    ParserProgramValueList { values, tail: None }
}
fn s(operation: ParserProgramStatementKind) -> ParserProgramStatement {
    ParserProgramStatement {
        location: loc(),
        operation,
    }
}
fn call(arguments: ParserProgramValueList) -> ParserProgramCall {
    ParserProgramCall {
        binding: 0,
        receiver: None,
        arguments,
    }
}
fn insert(args: Vec<ParserProgramExpr>) -> ParserProgramStatement {
    s(ParserProgramStatementKind::Call {
        call: call(values(args)),
    })
}
fn declare(local: u16, fields: Vec<ParserProgramField>) -> ParserProgramStatement {
    s(ParserProgramStatementKind::Declare {
        locals: vec![local],
        values: values(vec![e(ParserProgramExprKind::Table { fields })]),
    })
}
fn ret(items: Vec<ParserProgramExpr>) -> ParserProgramStatement {
    s(ParserProgramStatementKind::Return {
        values: values(items),
    })
}
fn tail(arguments: ParserProgramValueList) -> ParserProgramStatement {
    s(ParserProgramStatementKind::Return {
        values: ParserProgramValueList {
            values: vec![],
            tail: Some(Box::new(ParserProgramPack::Call {
                call: call(arguments),
            })),
        },
    })
}
struct Fixture {
    plan: CompiledParserPrograms,
    caller: ParserCallbackId,
    insert: ParserCallbackId,
    slot: u16,
}
fn fixture(parameters: u16, body: Vec<ParserProgramStatement>) -> Fixture {
    let mut data = bundled_snapshot().unwrap().modifier_parser().data().clone();
    // Retain actual descriptors and capture identity, with an explicit closed
    // caller-supplied authorization. No inherited public permission enters this fixture.
    data.programs = ParserProgramPayload::default();
    let targets: Vec<_> = data.callbacks.iter().enumerate().filter(|(_, c)|
        matches!(&c.kind, ParserCallbackKind::Builtin { symbol } if symbol == "table.insert")
    ).map(|(i, _)| ParserCallbackId(u32::try_from(i + 1).unwrap())).collect();
    assert_eq!(targets.len(), 1);
    let insert = targets[0];
    let (caller, slot, source) = data
        .callbacks
        .iter()
        .enumerate()
        .find_map(|(i, c)| {
            let ParserCallbackKind::Lua { source } = &c.kind else {
                return None;
            };
            if c.environment != ParserEnvironment::OriginalGlobals {
                return None;
            }
            let slot = c
                .upvalues
                .iter()
                .position(|u| u.value == ParserValue::Callback(insert))?;
            Some((
                ParserCallbackId(u32::try_from(i + 1).unwrap()),
                u16::try_from(slot).unwrap(),
                source.clone(),
            ))
        })
        .expect("actual injected source closure captures table.insert");
    data.program_intrinsics = BTreeMap::from([(insert, ParserProgramIntrinsic::TableInsert)]);
    data.factories.insert(
        caller,
        ParserFactoryDisposition::Unsupported {
            reason: "authored captured-insert mechanics".into(),
        },
    );
    let owner = ModifierParserCatalog::new(data).unwrap();
    let program = ParserProgram {
        callback: caller,
        parameter_count: parameters,
        variadic: true,
        local_count: 8,
        bindings: vec![ParserProgramBinding::Intrinsic {
            operation: ParserProgramIntrinsic::TableInsert,
            source: ParserProgramIntrinsicSource::Captured {
                upvalue: slot,
                callback: insert,
            },
        }],
        body,
        // Authored instruction labels only. Actual body extraction and its hash
        // are independently tested in PoB; these bytes grant no source admission.
        provenance: ParserProgramProvenance {
            source,
            function_start: 0,
            function_end: 1,
            function_sha256: "a".repeat(64),
        },
    };
    let catalog = ParserProgramCatalog::new(
        ParserProgramData {
            schema_version: PARSER_PROGRAM_SCHEMA_VERSION,
            programs: vec![program],
            callbacks: BTreeMap::from([(caller, ParserProgramId(1))]),
        },
        owner,
    )
    .unwrap();
    Fixture {
        plan: CompiledParserPrograms::new(&catalog).unwrap(),
        caller,
        insert,
        slot,
    }
}
fn input(values: Vec<ProgramValue>, tables: Vec<ProgramTable>) -> ProgramValueGraph {
    ProgramValueGraph { values, tables }
}
fn table(graph: &ProgramValueGraph, value: &ProgramValue) -> ProgramTableId {
    let ProgramValue::Table(id) = value else {
        panic!("expected table, got {value:?}");
    };
    assert!((id.0 as usize) <= graph.tables.len());
    *id
}
fn field<'a>(
    graph: &'a ProgramValueGraph,
    id: ProgramTableId,
    key: &ProgramValue,
) -> Option<&'a ProgramValue> {
    graph.tables[id.0 as usize - 1]
        .entries
        .iter()
        .find_map(|(k, v)| (k == key).then_some(v))
}
fn execute(f: &Fixture, input: &ProgramValueGraph) -> ProgramOutput {
    f.plan
        .execute(f.caller, input, ProgramLimits::default())
        .unwrap()
}
fn assert_site(f: &Fixture, error: &ProgramRuntimeError, kind: ProgramRuntimeErrorKind) {
    assert_eq!(error.kind, kind);
    assert_eq!(error.callback, Some(f.caller));
    assert_ne!(f.caller, f.insert);
    assert_eq!(error.location, Some(loc()));
}

#[test]
fn captured_insert_appends_borrowed_values_without_copying_their_aliases() {
    let f = fixture(
        1,
        vec![
            declare(1, vec![]),
            insert(vec![l(1), l(0)]),
            insert(vec![l(1), l(0)]),
            ret(vec![l(1), l(0)]),
        ],
    );
    assert_eq!(
        f.plan.program(f.caller).unwrap().bindings(),
        &[CompiledProgramBinding::Intrinsic {
            operation: ParserProgramIntrinsic::TableInsert,
            source: ParserProgramIntrinsicSource::Captured {
                upvalue: f.slot,
                callback: f.insert
            },
        }]
    );
    let original = input(
        vec![ProgramValue::Table(ProgramTableId(1))],
        vec![ProgramTable {
            entries: vec![(
                ProgramValue::Bytes(b"payload".to_vec()),
                ProgramValue::Number(7.0),
            )],
        }],
    );
    let output = execute(&f, &original);
    let graph = output.graph();
    let list = table(graph, &graph.values[0]);
    let tag = table(graph, &graph.values[1]);
    assert_ne!(list, tag);
    assert_eq!(
        field(graph, list, &ProgramValue::Number(1.0)),
        Some(&ProgramValue::Table(tag))
    );
    assert_eq!(
        field(graph, list, &ProgramValue::Number(2.0)),
        Some(&ProgramValue::Table(tag))
    );
    assert_eq!(
        field(graph, tag, &ProgramValue::Bytes(b"payload".to_vec())),
        Some(&ProgramValue::Number(7.0))
    );
    assert_eq!(graph.tables.len(), 2);
    // A caller can edit an exported snapshot without modifying input or another invocation.
    let mut changed = graph.clone();
    changed.tables[list.0 as usize - 1].entries.clear();
    assert_ne!(changed, *graph);
    assert_eq!(execute(&f, &original).graph(), graph);
    assert_eq!(original.tables[0].entries.len(), 1);
}

#[test]
fn captured_insert_has_zero_returns_scalar_nil_adjustment_and_nil_does_not_advance_length() {
    let raw = fixture(
        0,
        vec![declare(0, vec![]), tail(values(vec![l(0), number(7.0)]))],
    );
    assert!(
        execute(&raw, &ProgramValueGraph::default())
            .graph()
            .values
            .is_empty()
    );
    let scalar = fixture(
        0,
        vec![
            declare(0, vec![]),
            ret(vec![e(ParserProgramExprKind::Call {
                call: Box::new(call(values(vec![l(0), number(7.0)]))),
            })]),
        ],
    );
    assert_eq!(
        execute(&scalar, &ProgramValueGraph::default())
            .graph()
            .values,
        vec![ProgramValue::Nil]
    );
    let nil_then_value = fixture(
        0,
        vec![
            declare(0, vec![]),
            insert(vec![l(0), nil()]),
            insert(vec![l(0), number(9.0)]),
            ret(vec![l(0)]),
        ],
    );
    let result = execute(&nil_then_value, &ProgramValueGraph::default());
    let graph = result.graph();
    let id = table(graph, &graph.values[0]);
    assert_eq!(
        graph.tables[id.0 as usize - 1].entries,
        vec![(ProgramValue::Number(1.0), ProgramValue::Number(9.0))]
    );
    let recursive = fixture(
        0,
        vec![
            declare(0, vec![]),
            insert(vec![l(0), l(0)]),
            ret(vec![l(0)]),
        ],
    );
    let result = execute(&recursive, &ProgramValueGraph::default());
    let graph = result.graph();
    let id = table(graph, &graph.values[0]);
    assert_eq!(
        field(graph, id, &ProgramValue::Number(1.0)),
        Some(&ProgramValue::Table(id))
    );
}

#[test]
fn captured_insert_arity_and_receiver_errors_stay_at_the_calling_program() {
    let f = fixture(
        0,
        vec![tail(ParserProgramValueList {
            values: vec![],
            tail: Some(Box::new(ParserProgramPack::Varargs)),
        })],
    );
    let table = ProgramValue::Table(ProgramTableId(1));
    for args in [
        vec![],
        vec![ProgramValue::Nil],
        vec![ProgramValue::Boolean(false), ProgramValue::Number(1.0)],
        vec![ProgramValue::Number(2.0), ProgramValue::Number(1.0)],
        vec![table.clone()],
        vec![
            table.clone(),
            ProgramValue::Number(1.0),
            ProgramValue::Number(2.0),
            ProgramValue::Number(3.0),
        ],
        vec![table.clone(), ProgramValue::Nil, ProgramValue::Number(3.0)],
        vec![
            table.clone(),
            ProgramValue::Boolean(false),
            ProgramValue::Number(3.0),
        ],
    ] {
        let error = f
            .plan
            .execute(
                f.caller,
                &input(args, vec![ProgramTable { entries: vec![] }]),
                ProgramLimits::default(),
            )
            .unwrap_err();
        assert_site(&f, &error, ProgramRuntimeErrorKind::Source);
    }
    // Positional shifts are still explicit, including coercible numeric positions.
    for position in [
        ProgramValue::Number(1.0),
        ProgramValue::Bytes(b"1".to_vec()),
    ] {
        let error = f
            .plan
            .execute(
                f.caller,
                &input(
                    vec![table.clone(), position, ProgramValue::Number(3.0)],
                    vec![ProgramTable { entries: vec![] }],
                ),
                ProgramLimits::default(),
            )
            .unwrap_err();
        assert_site(&f, &error, ProgramRuntimeErrorKind::UnsupportedCapability);
        assert!(error.message.contains("positional shift"));
    }
}

#[test]
fn captured_insert_refuses_borrowed_mutation_and_ambiguous_sparse_length() {
    let f = fixture(2, vec![tail(values(vec![l(0), l(1)]))]);
    let original = input(
        vec![
            ProgramValue::Table(ProgramTableId(1)),
            ProgramValue::Number(3.0),
        ],
        vec![ProgramTable { entries: vec![] }],
    );
    let before = original.clone();
    let error = f
        .plan
        .execute(f.caller, &original, ProgramLimits::default())
        .unwrap_err();
    assert_site(&f, &error, ProgramRuntimeErrorKind::UnsupportedCapability);
    assert!(error.message.contains("mutation of a borrowed table"));
    assert_eq!(original, before);
    let sparse = fixture(
        0,
        vec![
            declare(
                0,
                vec![ParserProgramField::Keyed {
                    key: number(2.0),
                    value: number(7.0),
                }],
            ),
            insert(vec![l(0), number(3.0)]),
            ret(vec![l(0)]),
        ],
    );
    let error = sparse
        .plan
        .execute(
            sparse.caller,
            &ProgramValueGraph::default(),
            ProgramLimits::default(),
        )
        .unwrap_err();
    assert_site(
        &sparse,
        &error,
        ProgramRuntimeErrorKind::UnsupportedCapability,
    );
    assert!(error.message.contains("integer holes"));
}

#[test]
fn captured_insert_keeps_reached_owned_alias_prefix_after_later_source_error() {
    let f = fixture(
        2,
        vec![insert(vec![l(0), l(1)]), insert(vec![l(0)]), ret(vec![])],
    );
    let initial = input(
        vec![
            ProgramValue::Table(ProgramTableId(1)),
            ProgramValue::Table(ProgramTableId(2)),
        ],
        vec![
            ProgramTable { entries: vec![] },
            ProgramTable {
                entries: vec![(
                    ProgramValue::Bytes(b"tag".to_vec()),
                    ProgramValue::Boolean(true),
                )],
            },
        ],
    );
    let (mut session, roots) = f
        .plan
        .source_programs()
        .session(&initial, ProgramLimits::default())
        .unwrap();
    let error = session.invoke(f.caller, &roots).unwrap_err();
    assert_site(&f, &error, ProgramRuntimeErrorKind::Source);
    let output = session.snapshot(&roots).unwrap();
    let graph = output.graph();
    let list = table(graph, &graph.values[0]);
    let tag = table(graph, &graph.values[1]);
    assert_eq!(
        field(graph, list, &ProgramValue::Number(1.0)),
        Some(&ProgramValue::Table(tag))
    );
    assert_eq!(graph.tables[list.0 as usize - 1].entries.len(), 1);
    assert!(initial.tables[0].entries.is_empty());
}

#[test]
fn captured_insert_limits_accumulate_across_owned_session_calls() {
    let f = fixture(2, vec![tail(values(vec![l(0), l(1)]))]);
    let initial = input(
        vec![
            ProgramValue::Table(ProgramTableId(1)),
            ProgramValue::Number(7.0),
        ],
        vec![ProgramTable { entries: vec![] }],
    );
    for limits in [
        ProgramLimits {
            max_values: 256,
            ..ProgramLimits::default()
        },
        ProgramLimits {
            max_steps: 40,
            ..ProgramLimits::default()
        },
    ] {
        let (mut session, roots) = f.plan.source_programs().session(&initial, limits).unwrap();
        let mut successes = 0;
        let mut error = None;
        let start = session.allocations();
        for _ in 0..128 {
            match session.invoke(f.caller, &roots) {
                Ok(result) => {
                    assert!(result.is_empty());
                    successes += 1;
                }
                Err(failed) => {
                    error = Some(failed);
                    break;
                }
            }
        }
        assert!(successes > 0);
        assert_eq!(
            error
                .expect("cumulative limit must stop repeated appends")
                .kind,
            ProgramRuntimeErrorKind::ResourceBound
        );
        assert!(session.allocations().values > start.values);
        assert!(session.allocations().values <= limits.max_values);
        assert!(session.steps() > 0);
    }
    // No implicit owned-table clone may hide an exhausted table allocation budget.
    let creates = fixture(
        0,
        vec![
            declare(0, vec![]),
            insert(vec![l(0), number(7.0)]),
            ret(vec![l(0)]),
        ],
    );
    let error = creates
        .plan
        .execute(
            creates.caller,
            &ProgramValueGraph::default(),
            ProgramLimits {
                max_tables: 0,
                ..ProgramLimits::default()
            },
        )
        .unwrap_err();
    assert_eq!(error.kind, ProgramRuntimeErrorKind::ResourceBound);
}
