//! Authored structural table facts; source-observer differential tests live in PoB.
use poe_optimizer_data::{
    item_loading::{ItemLoadingSource, ItemSourceSpan},
    modifier_parser::*,
    source_program::*,
};
use poe_optimizer_engine::{lua_pattern::MatchLimits, source_program::*};
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
fn library() -> CompiledSourcePrograms {
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
    CompiledSourcePrograms::new(
        &SourceProgramCatalog::new(
            ParserProgramData {
                schema_version: PARSER_PROGRAM_SCHEMA_VERSION,
                programs,
                callbacks,
            },
            owner,
        )
        .unwrap(),
    )
    .unwrap()
}
fn text(s: &str) -> ProgramValue {
    ProgramValue::Bytes(s.as_bytes().to_vec())
}
fn table(id: u32) -> ProgramValue {
    ProgramValue::Table(ProgramTableId(id))
}
fn input(lib: &CompiledSourcePrograms) -> SourceSessionInput {
    SourceSessionInput {
        owner: lib.catalog().owner().clone(),
        state: ProgramValueGraph {
            values: vec![table(1), table(1)],
            tables: vec![ProgramTable {
                entries: vec![
                    (ProgramValue::Number(2.0), text("two")),
                    (text("z"), ProgramValue::Boolean(false)),
                ],
            }],
        },
        coverage: BTreeMap::new(),
        class_bindings: BTreeMap::new(),
        cells: vec![],
        closures: vec![],
        traversal: Some(SourceSessionTraversal {
            tables: BTreeMap::from([(
                ProgramTableId(1),
                SourceSessionTableTraversal {
                    order: vec![text("z"), ProgramValue::Number(2.0)],
                    raw_length: Some(2),
                },
            )]),
        }),
    }
}
fn scalar(session: &mut ProgramSession, values: Vec<ProgramValue>) -> Vec<SessionValue> {
    session
        .borrow(&ProgramValueGraph {
            values,
            tables: vec![],
        })
        .unwrap()
}
fn out(session: &mut ProgramSession, values: &[SessionValue]) -> Vec<ProgramValue> {
    session.snapshot(values).unwrap().graph().values.clone()
}
fn length(
    session: &mut ProgramSession,
    table: &SessionValue,
) -> Result<Vec<ProgramValue>, ProgramRuntimeError> {
    let values = session.invoke(SourceCallbackId(2), std::slice::from_ref(table))?;
    Ok(out(session, &values))
}
fn next(
    session: &mut ProgramSession,
    table: &SessionValue,
    control: Option<&SessionValue>,
) -> Result<Vec<SessionValue>, ProgramRuntimeError> {
    let mut args = vec![table.clone()];
    if let Some(control) = control {
        args.push(control.clone());
    }
    session.invoke(SourceCallbackId(1), &args)
}
fn set(session: &mut ProgramSession, table: &SessionValue, key: ProgramValue, value: ProgramValue) {
    let values = scalar(session, vec![key, value]);
    session
        .invoke(
            SourceCallbackId(3),
            &[table.clone(), values[0].clone(), values[1].clone()],
        )
        .unwrap();
}
#[test]
fn mutable_observed_order_reads_current_values_and_preserves_false_aliases() {
    let lib = library();
    let (mut session, roots) = lib
        .session_from_input(&input(&lib), ProgramLimits::default())
        .unwrap();
    assert_eq!(
        length(&mut session, &roots[0]).unwrap(),
        vec![ProgramValue::Number(2.0)]
    );
    let first = next(&mut session, &roots[0], None).unwrap();
    assert_eq!(
        out(&mut session, &first),
        vec![text("z"), ProgramValue::Boolean(false)]
    );
    set(
        &mut session,
        &roots[1],
        ProgramValue::Number(2.0),
        text("changed"),
    );
    let second = next(&mut session, &roots[0], Some(&first[0])).unwrap();
    assert_eq!(
        out(&mut session, &second),
        vec![ProgramValue::Number(2.0), text("changed")]
    );
    let terminal = next(&mut session, &roots[0], Some(&second[0])).unwrap();
    assert_eq!(out(&mut session, &terminal), vec![ProgramValue::Nil]);
    assert_eq!(
        length(&mut session, &roots[0]).unwrap(),
        vec![ProgramValue::Number(2.0)]
    );
    let unknown = scalar(&mut session, vec![text("missing")]);
    assert_eq!(
        next(&mut session, &roots[0], Some(&unknown[0]))
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
}
#[test]
fn every_structural_write_invalidates_order_and_observed_sparse_length() {
    let lib = library();
    for (key, value) in [
        (text("new"), text("v")),
        (text("absent"), ProgramValue::Nil),
        (text("z"), ProgramValue::Nil),
        (ProgramValue::Number(2.0), ProgramValue::Nil),
    ] {
        let (mut session, roots) = lib
            .session_from_input(&input(&lib), ProgramLimits::default())
            .unwrap();
        set(&mut session, &roots[0], key.clone(), value);
        assert_eq!(
            next(&mut session, &roots[0], None).unwrap_err().kind,
            ProgramRuntimeErrorKind::UnsupportedCapability
        );
        if key == ProgramValue::Number(2.0) {
            assert_eq!(
                length(&mut session, &roots[0]).unwrap(),
                vec![ProgramValue::Number(0.0)]
            );
        } else {
            assert_eq!(
                length(&mut session, &roots[0]).unwrap_err().kind,
                ProgramRuntimeErrorKind::UnsupportedCapability
            );
        }
    }
    let (mut session, roots) = lib
        .session_from_input(&input(&lib), ProgramLimits::default())
        .unwrap();
    let value = scalar(&mut session, vec![text("three")]);
    session
        .invoke(SourceCallbackId(4), &[roots[0].clone(), value[0].clone()])
        .unwrap();
    assert_eq!(
        next(&mut session, &roots[0], None).unwrap_err().kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    assert_eq!(
        length(&mut session, &roots[0]).unwrap_err().kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    // Once invalidated, recreating the old raw inventory does not restore proof.
    set(
        &mut session,
        &roots[0],
        ProgramValue::Number(3.0),
        ProgramValue::Nil,
    );
    assert_eq!(
        length(&mut session, &roots[0]).unwrap_err().kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
}
#[test]
fn absent_length_empty_order_and_failed_writes_keep_distinct_contracts() {
    let lib = library();
    let mut source = input(&lib);
    source
        .traversal
        .as_mut()
        .unwrap()
        .tables
        .get_mut(&ProgramTableId(1))
        .unwrap()
        .raw_length = None;
    let (mut session, roots) = lib
        .session_from_input(&source, ProgramLimits::default())
        .unwrap();
    assert!(next(&mut session, &roots[0], None).is_ok());
    assert_eq!(
        length(&mut session, &roots[0]).unwrap_err().kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    let (mut session, roots) = lib
        .session_from_input(&input(&lib), ProgramLimits::default())
        .unwrap();
    let values = scalar(&mut session, vec![ProgramValue::Nil, text("value")]);
    assert_eq!(
        session
            .invoke(
                SourceCallbackId(3),
                &[roots[0].clone(), values[0].clone(), values[1].clone()]
            )
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::Source
    );
    assert!(next(&mut session, &roots[0], None).is_ok());
    assert_eq!(
        length(&mut session, &roots[0]).unwrap(),
        vec![ProgramValue::Number(2.0)]
    );
    let mut empty = input(&lib);
    empty.state.tables[0].entries.clear();
    let observation = empty
        .traversal
        .as_mut()
        .unwrap()
        .tables
        .get_mut(&ProgramTableId(1))
        .unwrap();
    observation.order.clear();
    observation.raw_length = Some(0);
    let (mut session, roots) = lib
        .session_from_input(&empty, ProgramLimits::default())
        .unwrap();
    let terminal = next(&mut session, &roots[0], None).unwrap();
    assert_eq!(out(&mut session, &terminal), vec![ProgramValue::Nil]);
    set(
        &mut session,
        &roots[0],
        ProgramValue::Number(1.0),
        text("one"),
    );
    assert_eq!(
        next(&mut session, &roots[0], None).unwrap_err().kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    assert_eq!(
        length(&mut session, &roots[0]).unwrap(),
        vec![ProgramValue::Number(1.0)]
    );
}

#[test]
fn snapshots_are_raw_values_and_cannot_round_trip_behavioral_observations() {
    let lib = library();
    let (mut session, roots) = lib
        .session_from_input(&input(&lib), ProgramLimits::default())
        .unwrap();
    let snapshot = session.snapshot(&roots).unwrap();
    let imported = session
        .import_with_coverage(snapshot.graph(), &ProgramTableCoverage::new())
        .unwrap();
    assert_eq!(
        next(&mut session, &imported[0], None).unwrap_err().kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    assert_eq!(
        length(&mut session, &imported[0]).unwrap_err().kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    assert!(next(&mut session, &roots[0], None).is_ok());
    let borrowed = session.borrow(snapshot.graph()).unwrap();
    assert_eq!(
        next(&mut session, &borrowed[0], None).unwrap_err().kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    let values = scalar(&mut session, vec![text("z"), text("changed")]);
    assert_eq!(
        session
            .invoke(
                SourceCallbackId(3),
                &[borrowed[0].clone(), values[0].clone(), values[1].clone()]
            )
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
}
#[test]
fn coherent_imports_remap_table_and_closure_keys_and_isolate_metadata() {
    let lib = library();
    let mut source = input(&lib);
    source.state.tables.push(ProgramTable::default());
    source.closures.push(SourceSessionClosure {
        prototype: lib
            .catalog()
            .owner()
            .bind_closure_prototype(SourceClosurePrototypeId(1))
            .unwrap(),
        captures: vec![],
    });
    let keys = vec![
        ProgramValue::Closure(SourceSessionClosureId(1)),
        table(2),
        ProgramValue::DefinitionTable(SourceTableId(1)),
        ProgramValue::Boolean(false),
        ProgramValue::Number(-0.0),
    ];
    source.state.values.extend(keys.clone());
    source.state.tables[0].entries = keys
        .iter()
        .enumerate()
        .map(|(i, k)| (k.clone(), ProgramValue::Number(i as f64)))
        .collect();
    source
        .traversal
        .as_mut()
        .unwrap()
        .tables
        .get_mut(&ProgramTableId(1))
        .unwrap()
        .order = keys;
    source
        .traversal
        .as_mut()
        .unwrap()
        .tables
        .get_mut(&ProgramTableId(1))
        .unwrap()
        .raw_length = Some(0);
    let (mut session, first) = lib
        .session_from_input(&source, ProgramLimits::default())
        .unwrap();
    let second = session.import_session_input(&source).unwrap();
    for roots in [&first, &second] {
        let mut control = None;
        for key in &roots[2..] {
            let pair = next(&mut session, &roots[0], control.as_ref()).unwrap();
            let equality = session
                .invoke(SourceCallbackId(6), &[pair[0].clone(), key.clone()])
                .unwrap();
            assert_eq!(
                out(&mut session, &equality),
                vec![ProgramValue::Boolean(true)]
            );
            control = Some(pair[0].clone());
        }
        let result = session.invoke_callable(&roots[2], &[]).unwrap();
        assert_eq!(out(&mut session, &result), vec![ProgramValue::Number(13.0)]);
    }
    let equality = session
        .invoke(SourceCallbackId(6), &[first[2].clone(), second[2].clone()])
        .unwrap();
    assert_eq!(
        out(&mut session, &equality),
        vec![ProgramValue::Boolean(false)]
    );
    set(&mut session, &first[0], text("new"), text("value"));
    assert!(next(&mut session, &second[0], None).is_ok());
    let (mut other, _) = lib
        .session_from_input(&source, ProgramLimits::default())
        .unwrap();
    assert_eq!(
        next(&mut other, &first[0], None).unwrap_err().kind,
        ProgramRuntimeErrorKind::InvalidInput
    );
}
#[test]
fn malformed_or_partial_observations_fail_atomically_and_keep_budget_charges() {
    let lib = library();
    let (mut session, roots) = lib
        .session_from_input(&input(&lib), ProgramLimits::default())
        .unwrap();
    let original_alloc = session.allocations();
    let mut invalid = input(&lib);
    invalid
        .traversal
        .as_mut()
        .unwrap()
        .tables
        .get_mut(&ProgramTableId(1))
        .unwrap()
        .order = vec![text("z"), text("z")];
    assert_eq!(
        session.import_session_input(&invalid).unwrap_err().kind,
        ProgramRuntimeErrorKind::InvalidInput
    );
    assert!(session.allocations().values > original_alloc.values);
    assert!(next(&mut session, &roots[0], None).is_ok());
    let mut invalid = input(&lib);
    invalid.coverage.insert(
        ProgramTableId(1),
        SourceTableCoverage {
            inventory: SourceTableInventory::Complete,
            known_absent: Default::default(),
            unavailable: std::collections::BTreeSet::from([SourceTableKey::Text("omitted".into())]),
            index_fallback: SourceTableIndexFallback::Nil,
            call_fallback: SourceTableCallFallback::NonCallable,
        },
    );
    assert_eq!(
        session.import_session_input(&invalid).unwrap_err().kind,
        ProgramRuntimeErrorKind::InvalidInput
    );
    let mut invalid = input(&lib);
    invalid
        .traversal
        .as_mut()
        .unwrap()
        .tables
        .get_mut(&ProgramTableId(1))
        .unwrap()
        .raw_length = Some(1);
    assert_eq!(
        session.import_session_input(&invalid).unwrap_err().kind,
        ProgramRuntimeErrorKind::InvalidInput
    );
    let foreign = library();
    assert_eq!(
        foreign
            .session_from_input(&input(&lib), ProgramLimits::default())
            .err()
            .unwrap()
            .kind,
        ProgramRuntimeErrorKind::InvalidInput
    );
}
#[test]
fn observation_import_and_lookup_work_are_bounded_without_shared_mutable_facts() {
    let lib = library();
    let source = input(&lib);
    let (baseline, _) = lib
        .session_from_input(&source, ProgramLimits::default())
        .unwrap();
    let used = baseline.allocations();
    for limits in [
        ProgramLimits {
            max_values: used.values - 1,
            ..ProgramLimits::default()
        },
        ProgramLimits {
            max_bytes: used.bytes - 1,
            ..ProgramLimits::default()
        },
    ] {
        assert_eq!(
            lib.session_from_input(&source, limits).err().unwrap().kind,
            ProgramRuntimeErrorKind::ResourceBound
        );
    }
    let (mut session, roots) = lib
        .session_from_input(
            &source,
            ProgramLimits {
                pattern: MatchLimits {
                    max_steps: 1,
                    ..MatchLimits::default()
                },
                ..ProgramLimits::default()
            },
        )
        .unwrap();
    let key = scalar(&mut session, vec![text("z")]);
    assert_eq!(
        next(&mut session, &roots[0], Some(&key[0]))
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
    assert_eq!(
        next(&mut session, &roots[0], Some(&key[0]))
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
    let (mut first, a) = lib
        .session_from_input(&source, ProgramLimits::default())
        .unwrap();
    let (mut second, b) = lib
        .session_from_input(&source, ProgramLimits::default())
        .unwrap();
    set(&mut first, &a[0], text("absent"), ProgramValue::Nil);
    assert!(next(&mut second, &b[0], None).is_ok());
}
