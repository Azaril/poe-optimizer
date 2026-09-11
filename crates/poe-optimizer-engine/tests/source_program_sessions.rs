//! Structural VM tests use authored IR; complete original-source parity lives in PoB tests.
use poe_optimizer_data::{
    item_loading::{ItemLoadingSource, ItemSourceSpan},
    modifier_parser::*,
    source_program::*,
};
use poe_optimizer_engine::{lua_pattern::MatchLimits, source_program::*};
use std::collections::BTreeMap;

fn expr(operation: ParserProgramExprKind) -> ParserProgramExpr {
    ParserProgramExpr {
        location: ParserProgramLocation { start: 0, end: 1 },
        operation,
    }
}
fn local(local: u16) -> ParserProgramExpr {
    expr(ParserProgramExprKind::Local { local })
}
fn number(value: f64) -> ParserProgramExpr {
    expr(ParserProgramExprKind::Literal {
        value: ParserFactoryLiteral::Number(value),
    })
}
fn bytes(value: &str) -> ParserProgramExpr {
    expr(ParserProgramExprKind::Bytes {
        value: value.as_bytes().to_vec(),
    })
}
fn stmt(operation: ParserProgramStatementKind) -> ParserProgramStatement {
    ParserProgramStatement {
        location: ParserProgramLocation { start: 0, end: 1 },
        operation,
    }
}
fn ret(values: Vec<ParserProgramExpr>) -> ParserProgramStatement {
    stmt(ParserProgramStatementKind::Return {
        values: ParserProgramValueList { values, tail: None },
    })
}
fn get(table: ParserProgramExpr, key: &str) -> ParserProgramExpr {
    expr(ParserProgramExprKind::Get {
        table: Box::new(table),
        key: Box::new(bytes(key)),
    })
}
fn graph(values: Vec<ProgramValue>) -> ProgramValueGraph {
    ProgramValueGraph {
        values,
        tables: vec![],
    }
}
fn table(id: u32) -> ProgramValue {
    ProgramValue::Table(ProgramTableId(id))
}
fn state() -> ProgramValueGraph {
    ProgramValueGraph {
        values: vec![table(1), table(1)],
        tables: vec![ProgramTable {
            entries: vec![
                (
                    ProgramValue::Bytes(b"count".to_vec()),
                    ProgramValue::Number(0.0),
                ),
                (ProgramValue::Bytes(b"self".to_vec()), table(1)),
            ],
        }],
    }
}
fn fixture(
    value: f64,
    body: Vec<ParserProgramStatement>,
    bindings: Vec<ParserProgramBinding>,
) -> (SourceProgramOwner, CompiledSourcePrograms) {
    let path = "src/Classes/Standalone.lua".to_owned();
    let sha = "a".repeat(64);
    let span = ItemSourceSpan {
        path: path.clone(),
        line: 1,
        end_line: 2,
        sha256: sha.clone(),
    };
    let owner = SourceProgramOwner::new(SourceProgramDefinitions {
        schema_version: SOURCE_PROGRAM_DEFINITIONS_SCHEMA_VERSION,
        source: ItemLoadingSource {
            upstream_revision: "b".repeat(40),
            files: BTreeMap::from([(path.clone(), sha)]),
            construction_spans: BTreeMap::new(),
            module_order: vec![path],
        },
        tables: vec![ParserTable {
            fields: BTreeMap::from([("value".to_owned(), ParserValue::Number(value))]),
            indexed: BTreeMap::new(),
        }],
        callbacks: vec![ParserCallback {
            kind: ParserCallbackKind::Lua {
                source: span.clone(),
            },
            environment: ParserEnvironment::OriginalGlobals,
            upvalues: vec![ParserUpvalue {
                name: "data".to_owned(),
                value: ParserValue::Table(ParserTableId(1)),
            }],
        }],
        roots: vec![SourceProgramRoot {
            name: "data".to_owned(),
            table: ParserTableId(1),
        }],
        intrinsics: BTreeMap::new(),
    })
    .unwrap();
    let data = ParserProgramData {
        schema_version: PARSER_PROGRAM_SCHEMA_VERSION,
        programs: vec![ParserProgram {
            callback: ParserCallbackId(1),
            parameter_count: 2,
            local_count: 2,
            variadic: false,
            bindings,
            body,
            provenance: ParserProgramProvenance {
                source: span,
                function_start: 0,
                function_end: 16,
                function_sha256: "c".repeat(64),
            },
        }],
        callbacks: BTreeMap::from([(ParserCallbackId(1), ParserProgramId(1))]),
    };
    let compiled =
        CompiledSourcePrograms::new(&SourceProgramCatalog::new(data, owner.clone()).unwrap())
            .unwrap();
    (owner, compiled)
}
fn increment() -> Vec<ParserProgramStatement> {
    vec![
        stmt(ParserProgramStatementKind::TableSet {
            table: local(0),
            key: bytes("count"),
            value: expr(ParserProgramExprKind::Binary {
                operation: ParserProgramBinary::Add,
                left: Box::new(get(local(0), "count")),
                right: Box::new(number(1.0)),
            }),
        }),
        ret(vec![local(0)]),
    ]
}
fn field(output: &SourceProgramOutput, root: usize, key: &str) -> ProgramValue {
    let ProgramValue::Table(id) = output.graph().values[root] else {
        panic!("expected table")
    };
    output.graph().tables[id.0 as usize - 1]
        .entries
        .iter()
        .find(|(k, _)| *k == ProgramValue::Bytes(key.as_bytes().to_vec()))
        .unwrap()
        .1
        .clone()
}

#[test]
fn standalone_owner_roots_and_captures_survive_clone_drop_without_parser_data() {
    let body = vec![ret(vec![
        expr(ParserProgramExprKind::NamedDefinition {
            root: SourceProgramRootId(1),
        }),
        expr(ParserProgramExprKind::Capture { upvalue: 0 }),
    ])];
    let (owner, first) = fixture(17.0, body.clone(), vec![]);
    let (_, second) = fixture(29.0, body, vec![]);
    assert!(owner.parser().is_none());
    let cloned = first.clone();
    drop(first);
    let output = cloned
        .execute(
            ParserCallbackId(1),
            &ProgramValueGraph::default(),
            ProgramLimits::default(),
        )
        .unwrap();
    assert!(output.owner().is_same_owner(&owner));
    drop(cloned);
    assert_eq!(output.graph().values[0], output.graph().values[1]);
    assert_eq!(field(&output, 0, "value"), ProgramValue::Number(17.0));
    let other = second
        .execute(
            ParserCallbackId(1),
            &ProgramValueGraph::default(),
            ProgramLimits::default(),
        )
        .unwrap();
    assert!(!output.owner().is_same_owner(other.owner()));
    assert_eq!(field(&other, 0, "value"), ProgramValue::Number(29.0));
}

#[test]
fn sequential_callbacks_preserve_owned_aliases_cycles_and_snapshot_independence() {
    let (owner, compiled) = fixture(0.0, increment(), vec![]);
    let (mut session, roots) = compiled
        .session(&state(), ProgramLimits::default())
        .unwrap();
    assert!(session.is_bound_to(&owner));
    let first = session.invoke(ParserCallbackId(1), &roots[..1]).unwrap();
    let snapshot = session.snapshot(&first).unwrap();
    session.invoke(ParserCallbackId(1), &roots[1..]).unwrap();
    let final_state = session.snapshot(&roots).unwrap();
    assert_eq!(field(&snapshot, 0, "count"), ProgramValue::Number(1.0));
    assert_eq!(field(&final_state, 0, "count"), ProgramValue::Number(2.0));
    assert_eq!(final_state.graph().values[0], final_state.graph().values[1]);
    assert_eq!(
        field(&final_state, 0, "self"),
        final_state.graph().values[0]
    );
    assert!(final_state.steps() > snapshot.steps());
    assert!(final_state.allocations().values > snapshot.allocations().values);
    assert_eq!(state().tables[0].entries[0].1, ProgramValue::Number(0.0));
}

#[test]
fn borrowed_definition_and_foreign_session_handles_cannot_be_mutated_or_rebound() {
    let (_, compiled) = fixture(0.0, increment(), vec![]);
    let (mut first, roots) = compiled
        .session(&state(), ProgramLimits::default())
        .unwrap();
    let (mut second, other_roots) = compiled
        .session(&state(), ProgramLimits::default())
        .unwrap();
    assert_eq!(
        first
            .invoke(ParserCallbackId(1), &other_roots[..1])
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::InvalidInput
    );
    assert_eq!(
        second.snapshot(&roots).unwrap_err().kind,
        ProgramRuntimeErrorKind::InvalidInput
    );
    let borrowed = first.borrow(&state()).unwrap();
    assert_eq!(
        first
            .invoke(ParserCallbackId(1), &borrowed[..1])
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    // Ordinary standalone/parser calls retain their stricter borrowed-input rule.
    assert_eq!(
        compiled
            .execute(ParserCallbackId(1), &state(), ProgramLimits::default())
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    let (_, writer) = fixture(
        0.0,
        vec![stmt(ParserProgramStatementKind::TableSet {
            table: local(0),
            key: bytes("value"),
            value: number(1.0),
        })],
        vec![],
    );
    let (mut session, _) = writer
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    let root = session
        .owner()
        .bind_root(SourceProgramDefinitionRoot::Named(SourceProgramRootId(1)))
        .unwrap();
    let definition = session.definition(&root).unwrap();
    let (other_owner, _) = fixture(0.0, increment(), vec![]);
    let foreign_root = other_owner
        .bind_root(SourceProgramDefinitionRoot::Named(SourceProgramRootId(1)))
        .unwrap();
    assert_eq!(
        session.definition(&foreign_root).unwrap_err().kind,
        ProgramRuntimeErrorKind::InvalidInput
    );
    assert_eq!(
        session
            .invoke(ParserCallbackId(1), &[definition])
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
}

#[test]
fn independent_sessions_share_code_but_not_mutable_state() {
    let (_, compiled) = fixture(0.0, increment(), vec![]);
    std::thread::scope(|scope| {
        let workers: Vec<_> = (1..=4)
            .map(|count| {
                let compiled = compiled.clone();
                scope.spawn(move || {
                    let (mut session, roots) = compiled
                        .session(&state(), ProgramLimits::default())
                        .unwrap();
                    for _ in 0..count {
                        session.invoke(ParserCallbackId(1), &roots[..1]).unwrap();
                    }
                    assert_eq!(
                        field(&session.snapshot(&roots).unwrap(), 0, "count"),
                        ProgramValue::Number(f64::from(count))
                    );
                })
            })
            .collect();
        for worker in workers {
            worker.join().unwrap();
        }
    });
}

#[test]
fn session_budgets_accumulate_across_calls_exports_and_failed_imports() {
    let (_, compiled) = fixture(0.0, increment(), vec![]);
    let (mut probe, roots) = compiled
        .session(&state(), ProgramLimits::default())
        .unwrap();
    probe.invoke(ParserCallbackId(1), &roots[..1]).unwrap();
    let one_call = probe.steps();
    let (mut limited, roots) = compiled
        .session(
            &state(),
            ProgramLimits {
                max_steps: one_call,
                ..ProgramLimits::default()
            },
        )
        .unwrap();
    limited.invoke(ParserCallbackId(1), &roots[..1]).unwrap();
    assert_eq!(
        limited
            .invoke(ParserCallbackId(1), &roots[..1])
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
    assert_eq!(limited.steps(), one_call);
    let (mut bytes_limited, _) = compiled
        .session(
            &ProgramValueGraph::default(),
            ProgramLimits {
                max_bytes: 3,
                ..ProgramLimits::default()
            },
        )
        .unwrap();
    let invalid = ProgramValueGraph {
        values: vec![ProgramValue::Bytes(b"ab".to_vec()), table(1)],
        tables: vec![],
    };
    assert_eq!(
        bytes_limited.borrow(&invalid).unwrap_err().kind,
        ProgramRuntimeErrorKind::InvalidInput
    );
    assert_eq!(bytes_limited.allocations().bytes, 2);
    assert_eq!(
        bytes_limited
            .borrow(&graph(vec![ProgramValue::Bytes(b"ab".to_vec())]))
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
    // Export allocations remain charged and cannot be reset by requesting another snapshot.
    let (mut counter, roots) = compiled
        .session(&state(), ProgramLimits::default())
        .unwrap();
    counter.snapshot(&roots).unwrap();
    let ceiling = counter.allocations().values;
    let (mut limited, roots) = compiled
        .session(
            &state(),
            ProgramLimits {
                max_values: ceiling,
                ..ProgramLimits::default()
            },
        )
        .unwrap();
    limited.snapshot(&roots).unwrap();
    assert_eq!(
        limited.snapshot(&roots).unwrap_err().kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
}

#[test]
fn session_pattern_work_is_cumulative_even_for_new_argument_imports() {
    let call = ParserProgramCall {
        binding: 0,
        receiver: None,
        arguments: ParserProgramValueList {
            values: vec![local(0)],
            tail: None,
        },
    };
    let (_, compiled) = fixture(
        0.0,
        vec![ret(vec![expr(ParserProgramExprKind::Call {
            call: Box::new(call),
        })])],
        vec![ParserProgramBinding::Intrinsic {
            operation: ParserProgramIntrinsic::ToNumber,
            source: ParserProgramIntrinsicSource::OriginalGlobal,
        }],
    );
    let (mut session, _) = compiled
        .session(
            &ProgramValueGraph::default(),
            ProgramLimits {
                pattern: MatchLimits {
                    max_steps: 4,
                    ..MatchLimits::default()
                },
                ..ProgramLimits::default()
            },
        )
        .unwrap();
    let arguments = session
        .borrow(&graph(vec![ProgramValue::Bytes(b"123".to_vec())]))
        .unwrap();
    session.invoke(ParserCallbackId(1), &arguments).unwrap();
    assert!(session.pattern_steps() >= 3);
    assert_eq!(
        session
            .invoke(ParserCallbackId(1), &arguments)
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
}

#[test]
fn source_failure_keeps_prior_session_writes_and_charges() {
    let mut body = increment();
    body.pop();
    body.push(ret(vec![get(local(1), "unreachable_field")]));
    let (_, compiled) = fixture(0.0, body, vec![]);
    let (mut session, roots) = compiled
        .session(&state(), ProgramLimits::default())
        .unwrap();
    let error = session
        .invoke(ParserCallbackId(1), &roots[..1])
        .unwrap_err();
    assert_eq!(error.kind, ProgramRuntimeErrorKind::Source);
    assert_eq!(error.callback, Some(ParserCallbackId(1)));
    let first = session.snapshot(&roots).unwrap();
    assert_eq!(field(&first, 0, "count"), ProgramValue::Number(1.0));
    assert!(first.steps() > 0);
    assert_eq!(
        session
            .invoke(ParserCallbackId(1), &roots[..1])
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::Source
    );
    let second = session.snapshot(&roots).unwrap();
    assert_eq!(field(&second, 0, "count"), ProgramValue::Number(2.0));
    assert!(second.steps() > first.steps());
    assert!(second.allocations().values > first.allocations().values);
}

#[test]
fn failed_duplicate_import_does_not_publish_partial_tables_or_shift_existing_aliases() {
    let (_, compiled) = fixture(0.0, increment(), vec![]);
    let (mut session, owned) = compiled
        .session(&state(), ProgramLimits::default())
        .unwrap();
    let first = session.borrow(&state()).unwrap();
    let mut invalid = state();
    invalid.tables.push(ProgramTable {
        entries: vec![
            (ProgramValue::Number(0.0), ProgramValue::Boolean(true)),
            (ProgramValue::Number(-0.0), ProgramValue::Boolean(false)),
        ],
    });
    let charged_before = session.allocations();
    assert_eq!(
        session.borrow(&invalid).unwrap_err().kind,
        ProgramRuntimeErrorKind::InvalidInput
    );
    assert!(session.allocations().tables > charged_before.tables);
    let second = session.borrow(&state()).unwrap();
    session.invoke(ParserCallbackId(1), &owned[..1]).unwrap();
    let selected = [
        owned[0].clone(),
        first[0].clone(),
        first[1].clone(),
        second[0].clone(),
        second[1].clone(),
    ];
    let output = session.snapshot(&selected).unwrap();
    assert_eq!(field(&output, 0, "count"), ProgramValue::Number(1.0));
    assert_eq!(field(&output, 1, "count"), ProgramValue::Number(0.0));
    assert_eq!(field(&output, 3, "count"), ProgramValue::Number(0.0));
    assert_eq!(output.graph().values[1], output.graph().values[2]);
    assert_eq!(output.graph().values[3], output.graph().values[4]);
    assert_ne!(output.graph().values[1], output.graph().values[3]);
    assert_eq!(field(&output, 1, "self"), output.graph().values[1]);
    assert_eq!(field(&output, 3, "self"), output.graph().values[3]);
}
