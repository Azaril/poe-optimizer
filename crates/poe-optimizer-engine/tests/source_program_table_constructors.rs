//! Structural source-constructor binding; authentic bytecode/source oracles live in PoB.
use poe_optimizer_data::{
    item_loading::{ItemLoadingSource, ItemSourceSpan},
    modifier_parser::*,
    source_program::*,
};
use poe_optimizer_engine::source_program::*;
use std::collections::BTreeMap;
fn e(operation: ParserProgramExprKind) -> ParserProgramExpr {
    ParserProgramExpr {
        location: ParserProgramLocation { start: 0, end: 1 },
        operation,
    }
}
fn l(local: u16) -> ParserProgramExpr {
    e(ParserProgramExprKind::Local { local })
}
fn s(operation: ParserProgramStatementKind) -> ParserProgramStatement {
    ParserProgramStatement {
        location: ParserProgramLocation { start: 0, end: 1 },
        operation,
    }
}
fn list(values: Vec<ParserProgramExpr>) -> ParserProgramValueList {
    ParserProgramValueList { values, tail: None }
}
fn ret(values: Vec<ParserProgramExpr>) -> ParserProgramStatement {
    s(ParserProgramStatementKind::Return {
        values: list(values),
    })
}
fn call(
    operation: ParserProgramIntrinsic,
    values: Vec<ParserProgramExpr>,
) -> (Vec<ParserProgramBinding>, Vec<ParserProgramStatement>) {
    (
        vec![ParserProgramBinding::Intrinsic {
            operation,
            source: ParserProgramIntrinsicSource::OriginalGlobal,
        }],
        vec![s(ParserProgramStatementKind::Return {
            values: ParserProgramValueList {
                values: vec![],
                tail: Some(Box::new(ParserProgramPack::Call {
                    call: ParserProgramCall {
                        binding: 0,
                        receiver: None,
                        arguments: list(values),
                    },
                })),
            },
        })],
    )
}
fn library(profile: Option<SourceTableRuntimeProfile>) -> CompiledSourcePrograms {
    let path = "src/Modules/TableFacts.lua".to_string();
    let span = ItemSourceSpan {
        path: path.clone(),
        line: 1,
        end_line: 10,
        sha256: "a".repeat(64),
    };
    let mut bodies = vec![call(ParserProgramIntrinsic::Next, vec![l(0), l(1)])];
    bodies.push((
        vec![],
        vec![ret(vec![e(ParserProgramExprKind::Unary {
            operation: ParserProgramUnary::Length,
            value: Box::new(l(0)),
        })])],
    ));
    bodies.push((
        vec![],
        vec![
            s(ParserProgramStatementKind::TableSet {
                table: l(0),
                key: l(1),
                value: l(2),
            }),
            ret(vec![]),
        ],
    ));
    bodies.push(call(ParserProgramIntrinsic::TableInsert, vec![l(0), l(1)]));
    bodies.push((
        vec![],
        vec![ret(vec![e(ParserProgramExprKind::Get {
            table: Box::new(l(0)),
            key: Box::new(l(1)),
        })])],
    ));
    bodies.push((
        vec![],
        vec![ret(vec![e(ParserProgramExprKind::Binary {
            operation: ParserProgramBinary::Equal,
            left: Box::new(l(0)),
            right: Box::new(l(1)),
        })])],
    ));
    bodies.push((
        vec![],
        vec![ret(vec![e(ParserProgramExprKind::Literal {
            value: ParserFactoryLiteral::Number(13.0),
        })])],
    ));
    for _ in 0..2 {
        bodies.push((
            vec![],
            vec![ret(vec![ParserProgramExpr {
                location: ParserProgramLocation { start: 10, end: 12 },
                operation: ParserProgramExprKind::Table { fields: vec![] },
            }])],
        ));
    }
    bodies.push(call(ParserProgramIntrinsic::Unpack, vec![l(0)]));
    bodies.push(call(ParserProgramIntrinsic::Unpack, vec![l(0), l(1), l(2)]));
    let owner = SourceProgramOwner::new_with_closures(
        SourceProgramDefinitions {
            schema_version: SOURCE_PROGRAM_DEFINITIONS_SCHEMA_VERSION,
            source: ItemLoadingSource {
                upstream_revision: "b".repeat(40),
                files: BTreeMap::from([(path.clone(), "a".repeat(64))]),
                construction_spans: BTreeMap::new(),
                module_order: vec![path],
            },
            tables: vec![SourceTable::default()],
            callbacks: (0..bodies.len())
                .map(|_| SourceCallback {
                    kind: SourceCallbackKind::Lua {
                        source: span.clone(),
                    },
                    environment: SourceEnvironment::OriginalGlobals,
                    upvalues: vec![],
                })
                .collect(),
            roots: vec![],
            intrinsics: BTreeMap::new(),
        },
        None,
        None,
        SourceClosurePrototypes {
            schema_version: SOURCE_CLOSURE_PROTOTYPES_SCHEMA_VERSION,
            prototypes: vec![SourceClosurePrototype {
                callback: SourceCallbackId(7),
            }],
        },
    )
    .unwrap();
    let programs = bodies
        .into_iter()
        .enumerate()
        .map(|(i, (bindings, body))| ParserProgram {
            callback: SourceCallbackId(i as u32 + 1),
            parameter_count: 3,
            local_count: 3,
            variadic: false,
            bindings,
            body,
            provenance: ParserProgramProvenance {
                source: span.clone(),
                function_start: 0,
                function_end: 100,
                function_sha256: "c".repeat(64),
            },
        })
        .collect::<Vec<_>>();
    let callbacks = programs
        .iter()
        .enumerate()
        .map(|(i, p)| (p.callback, ParserProgramId(i as u32 + 1)))
        .collect();
    let data = ParserProgramData {
        schema_version: PARSER_PROGRAM_SCHEMA_VERSION,
        programs,
        callbacks,
    };
    let catalog = if let Some(profile) = profile {
        let provenance = data.programs[7].provenance.clone();
        SourceProgramCatalog::new_with_constructors(
            data,
            owner,
            SourceProgramConstructors {
                schema_version: SOURCE_PROGRAM_CONSTRUCTORS_SCHEMA_VERSION,
                profile,
                sites: vec![SourceProgramConstructor {
                    callback: SourceCallbackId(8),
                    provenance,
                    expression: SourceProgramLocation { start: 10, end: 12 },
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
        .unwrap()
    } else {
        SourceProgramCatalog::new(data, owner).unwrap()
    };
    CompiledSourcePrograms::new(&catalog).unwrap()
}
fn write(
    session: &mut ProgramSession,
    table: &SessionValue,
    key: ProgramValue,
    value: ProgramValue,
) {
    let args = session
        .borrow(&ProgramValueGraph {
            values: vec![key, value],
            tables: vec![],
        })
        .unwrap();
    session
        .invoke(
            SourceCallbackId(3),
            &[table.clone(), args[0].clone(), args[1].clone()],
        )
        .unwrap();
}
fn unpack(
    session: &mut ProgramSession,
    table: &SessionValue,
) -> Result<ProgramValueGraph, ProgramRuntimeError> {
    let values = session.invoke(SourceCallbackId(10), std::slice::from_ref(table))?;
    session
        .snapshot(&values)
        .map(|output| output.graph().clone())
}
#[test]
fn exact_callback_expression_seed_admits_sparse_unpack_without_promoting_other_constructors() {
    let lib = library(Some(SourceTableRuntimeProfile::luajit21_x64_single()));
    let (mut session, _) = lib
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    let first = session.invoke(SourceCallbackId(8), &[]).unwrap().remove(0);
    let second = session.invoke(SourceCallbackId(9), &[]).unwrap().remove(0);
    for table in [&first, &second] {
        write(
            &mut session,
            table,
            ProgramValue::Number(2.0),
            ProgramValue::Boolean(false),
        );
    }
    assert_eq!(
        unpack(&mut session, &first).unwrap().values,
        vec![ProgramValue::Nil, ProgramValue::Boolean(false)]
    );
    assert_eq!(
        unpack(&mut session, &second).unwrap_err().kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    assert_eq!(
        session
            .invoke(SourceCallbackId(2), std::slice::from_ref(&first))
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    let graph = session.snapshot(std::slice::from_ref(&first)).unwrap();
    let imported = session
        .import_with_coverage(graph.graph(), &ProgramTableCoverage::new())
        .unwrap();
    assert_eq!(
        unpack(&mut session, &imported[0]).unwrap_err().kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
}
#[test]
fn omitted_and_unsupported_profiles_do_not_grant_layout_execution() {
    let unbound = library(None);
    let (mut session, _) = unbound
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    let table = session.invoke(SourceCallbackId(8), &[]).unwrap().remove(0);
    write(
        &mut session,
        &table,
        ProgramValue::Number(2.0),
        ProgramValue::Boolean(true),
    );
    assert_eq!(
        unpack(&mut session, &table).unwrap_err().kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    let mut profile = SourceTableRuntimeProfile::luajit21_x64_single();
    profile.table_bump = true;
    let unsupported = library(Some(profile));
    let (mut session, _) = unsupported
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    let before = session.allocations().tables;
    let error = session.invoke(SourceCallbackId(8), &[]).unwrap_err();
    assert_eq!(error.kind, ProgramRuntimeErrorKind::UnsupportedCapability);
    assert_eq!(error.callback, Some(SourceCallbackId(8)));
    assert_eq!(session.allocations().tables, before);
}
#[test]
fn shared_library_sessions_keep_independent_histories_and_foreign_handles_reject() {
    let lib = library(Some(SourceTableRuntimeProfile::luajit21_x64_single()));
    let threads: Vec<_> = (0..4)
        .map(|index| {
            let lib = lib.clone();
            std::thread::spawn(move || {
                let (mut session, _) = lib
                    .session(&ProgramValueGraph::default(), ProgramLimits::default())
                    .unwrap();
                let table = session.invoke(SourceCallbackId(8), &[]).unwrap().remove(0);
                write(
                    &mut session,
                    &table,
                    ProgramValue::Number(2.0),
                    ProgramValue::Number(f64::from(index)),
                );
                assert_eq!(
                    unpack(&mut session, &table).unwrap().values,
                    vec![ProgramValue::Nil, ProgramValue::Number(f64::from(index))]
                );
                table
            })
        })
        .collect();
    let roots: Vec<_> = threads
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect();
    let (mut session, _) = lib
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    for root in roots {
        assert_eq!(
            session
                .invoke(SourceCallbackId(10), &[root])
                .unwrap_err()
                .kind,
            ProgramRuntimeErrorKind::InvalidInput
        );
    }
}

#[test]
fn ambiguous_native_array_hints_reject_default_unpack_but_keep_explicit_raw_range() {
    let lib = library(Some(SourceTableRuntimeProfile::luajit21_x64_single()));
    let (mut session, _) = lib
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    let table = session.invoke(SourceCallbackId(8), &[]).unwrap().remove(0);
    write(
        &mut session,
        &table,
        ProgramValue::Number(0.0),
        ProgramValue::Boolean(false),
    );
    write(
        &mut session,
        &table,
        ProgramValue::Number(2.0),
        ProgramValue::Boolean(false),
    );
    assert_eq!(
        unpack(&mut session, &table).unwrap_err().kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    let before = session.snapshot(std::slice::from_ref(&table)).unwrap();
    let append_value = session
        .borrow(&ProgramValueGraph {
            values: vec![ProgramValue::Boolean(true)],
            tables: vec![],
        })
        .unwrap();
    assert_eq!(
        session
            .invoke(
                SourceCallbackId(4),
                &[table.clone(), append_value[0].clone()]
            )
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    assert_eq!(
        session
            .snapshot(std::slice::from_ref(&table))
            .unwrap()
            .graph(),
        before.graph()
    );
    let range = session
        .borrow(&ProgramValueGraph {
            values: vec![ProgramValue::Number(1.0), ProgramValue::Number(2.0)],
            tables: vec![],
        })
        .unwrap();
    let values = session
        .invoke(
            SourceCallbackId(11),
            &[table.clone(), range[0].clone(), range[1].clone()],
        )
        .unwrap();
    assert_eq!(
        session.snapshot(&values).unwrap().graph().values,
        vec![ProgramValue::Nil, ProgramValue::Boolean(false)]
    );
    write(
        &mut session,
        &table,
        ProgramValue::Number(0.0),
        ProgramValue::Nil,
    );
    write(
        &mut session,
        &table,
        ProgramValue::Number(1.0),
        ProgramValue::Boolean(true),
    );
    write(
        &mut session,
        &table,
        ProgramValue::Number(3.0),
        ProgramValue::Boolean(true),
    );
    write(
        &mut session,
        &table,
        ProgramValue::Number(2.0),
        ProgramValue::Nil,
    );
    assert_eq!(
        unpack(&mut session, &table).unwrap_err().kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
}
