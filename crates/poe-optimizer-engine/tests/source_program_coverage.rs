//! Partial-table evidence, session identity and whole-heap failure contracts.
#![cfg(not(target_arch = "wasm32"))]
use mlua::Lua;
use poe_optimizer_data::{
    item_loading::{ItemLoadingSource, ItemSourceSpan},
    modifier_parser::*,
    source_program::*,
};
use poe_optimizer_engine::source_program::*;
use std::collections::{BTreeMap, BTreeSet};
fn e(operation: ParserProgramExprKind) -> ParserProgramExpr {
    ParserProgramExpr {
        location: ParserProgramLocation { start: 0, end: 1 },
        operation,
    }
}
fn l(local: u16) -> ParserProgramExpr {
    e(ParserProgramExprKind::Local { local })
}
fn n(value: f64) -> ParserProgramExpr {
    e(ParserProgramExprKind::Literal {
        value: ParserFactoryLiteral::Number(value),
    })
}
fn b(value: &str) -> ParserProgramExpr {
    e(ParserProgramExprKind::Bytes {
        value: value.as_bytes().to_vec(),
    })
}
fn get(table: ParserProgramExpr, key: ParserProgramExpr) -> ParserProgramExpr {
    e(ParserProgramExprKind::Get {
        table: Box::new(table),
        key: Box::new(key),
    })
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
fn set(
    table: ParserProgramExpr,
    key: ParserProgramExpr,
    value: ParserProgramExpr,
) -> ParserProgramStatement {
    s(ParserProgramStatementKind::TableSet { table, key, value })
}
fn coverage(inventory: SourceTableInventory) -> SourceTableCoverage {
    SourceTableCoverage {
        inventory,
        known_absent: BTreeSet::new(),
        unavailable: BTreeSet::new(),
        index_fallback: SourceTableIndexFallback::Nil,
        call_fallback: SourceTableCallFallback::NonCallable,
    }
}
fn key(value: &str) -> SourceTableKey {
    SourceTableKey::Text(value.into())
}
fn text(value: &str) -> ProgramValue {
    ProgramValue::Bytes(value.as_bytes().to_vec())
}
fn table(id: u32) -> ProgramValue {
    ProgramValue::Table(ProgramTableId(id))
}
fn graph(entries: Vec<(ProgramValue, ProgramValue)>) -> ProgramValueGraph {
    ProgramValueGraph {
        values: vec![table(1), table(1)],
        tables: vec![ProgramTable { entries }],
    }
}
fn meta(value: SourceTableCoverage) -> ProgramTableCoverage {
    BTreeMap::from([(ProgramTableId(1), value)])
}
fn compile() -> CompiledSourcePrograms {
    let path = "src/Modules/CoverageFixture.lua".to_owned();
    let sha = "a".repeat(64);
    let span = ItemSourceSpan {
        path: path.clone(),
        line: 1,
        end_line: 20,
        sha256: sha.clone(),
    };
    let mut root_coverage = coverage(SourceTableInventory::Complete);
    root_coverage.unavailable.insert(key("hidden"));
    let mut callbacks: Vec<_> = (1..=7)
        .map(|id| ParserCallback {
            kind: ParserCallbackKind::Lua {
                source: span.clone(),
            },
            environment: ParserEnvironment::OriginalGlobals,
            upvalues: if id == 4 {
                vec![ParserUpvalue {
                    name: "ipairs".into(),
                    value: ParserValue::Callback(ParserCallbackId(8)),
                }]
            } else if id == 7 {
                vec![ParserUpvalue {
                    name: "write".into(),
                    value: ParserValue::Callback(ParserCallbackId(2)),
                }]
            } else {
                vec![]
            },
        })
        .collect();
    callbacks.push(ParserCallback {
        kind: ParserCallbackKind::Builtin {
            symbol: "ipairs".into(),
        },
        environment: ParserEnvironment::OriginalGlobals,
        upvalues: vec![],
    });
    let owner = SourceProgramOwner::new_with_context(
        SourceProgramDefinitions {
            schema_version: SOURCE_PROGRAM_DEFINITIONS_SCHEMA_VERSION,
            source: ItemLoadingSource {
                upstream_revision: "b".repeat(40),
                files: BTreeMap::from([(path.clone(), sha)]),
                construction_spans: BTreeMap::new(),
                module_order: vec![path],
            },
            tables: vec![SourceTable {
                fields: BTreeMap::from([("value".into(), SourceValue::Number(42.0))]),
                indexed: BTreeMap::new(),
            }],
            callbacks,
            roots: vec![SourceProgramRoot {
                name: "environment".into(),
                table: SourceTableId(1),
            }],
            intrinsics: BTreeMap::from([(ParserCallbackId(8), ParserProgramIntrinsic::Ipairs)]),
        },
        None,
        SourceProgramContext {
            environment: Some(SourceProgramRootId(1)),
            tables: BTreeMap::from([(SourceTableId(1), root_coverage)]),
            ..SourceProgramContext::default()
        },
    )
    .unwrap();
    let bodies = vec![
        (vec![], vec![ret(vec![get(l(0), l(1))])]),
        (vec![], vec![set(l(0), l(1), l(2)), ret(vec![])]),
        (
            vec![],
            vec![ret(vec![e(ParserProgramExprKind::Unary {
                operation: ParserProgramUnary::Length,
                value: Box::new(l(0)),
            })])],
        ),
        (
            vec![ParserProgramBinding::Intrinsic {
                operation: ParserProgramIntrinsic::Ipairs,
                source: ParserProgramIntrinsicSource::Captured {
                    upvalue: 0,
                    callback: ParserCallbackId(8),
                },
            }],
            vec![
                s(ParserProgramStatementKind::Assign {
                    locals: vec![2],
                    values: list(vec![n(0.0)]),
                }),
                s(ParserProgramStatementKind::ForEach {
                    locals: vec![3, 4],
                    iterator: ParserProgramIterator::Dense {
                        table: l(0),
                        binding: 0,
                    },
                    body: vec![s(ParserProgramStatementKind::Assign {
                        locals: vec![2],
                        values: list(vec![e(ParserProgramExprKind::Binary {
                            operation: ParserProgramBinary::Add,
                            left: Box::new(l(2)),
                            right: Box::new(l(4)),
                        })]),
                    })],
                }),
                ret(vec![l(2)]),
            ],
        ),
        (
            vec![],
            vec![ret(vec![get(
                e(ParserProgramExprKind::NamedDefinition {
                    root: SourceProgramRootId(1),
                }),
                l(1),
            )])],
        ),
        (
            vec![],
            vec![set(l(0), b("earlier"), n(1.0)), ret(vec![get(l(0), l(1))])],
        ),
        (
            vec![
                ParserProgramBinding::DynamicMethod {
                    key: "missing".into(),
                },
                ParserProgramBinding::CapturedCallback {
                    upvalue: 0,
                    callback: ParserCallbackId(2),
                },
            ],
            vec![ret(vec![e(ParserProgramExprKind::Call {
                call: Box::new(ParserProgramCall {
                    binding: 0,
                    receiver: Some(Box::new(l(0))),
                    arguments: list(vec![e(ParserProgramExprKind::Call {
                        call: Box::new(ParserProgramCall {
                            binding: 1,
                            receiver: None,
                            arguments: list(vec![l(1), b("ran"), n(1.0)]),
                        }),
                    })]),
                }),
            })])],
        ),
    ];
    let programs: Vec<_> = bodies
        .into_iter()
        .enumerate()
        .map(|(i, (bindings, body))| ParserProgram {
            callback: ParserCallbackId(i as u32 + 1),
            parameter_count: 3,
            local_count: 5,
            variadic: false,
            bindings,
            body,
            provenance: ParserProgramProvenance {
                source: span.clone(),
                function_start: 0,
                function_end: 64,
                function_sha256: "c".repeat(64),
            },
        })
        .collect();
    let callback_map = programs
        .iter()
        .enumerate()
        .map(|(i, p)| (p.callback, ParserProgramId(i as u32 + 1)))
        .collect();
    CompiledSourcePrograms::new(
        &SourceProgramCatalog::new(
            ParserProgramData {
                schema_version: PARSER_PROGRAM_SCHEMA_VERSION,
                programs,
                callbacks: callback_map,
            },
            owner,
        )
        .unwrap(),
    )
    .unwrap()
}
fn scalar(
    session: &mut ProgramSession,
    callback: u32,
    args: &[SessionValue],
) -> RuntimeResult<ProgramValue> {
    let result = session.invoke(ParserCallbackId(callback), args)?;
    Ok(session.snapshot(&result)?.graph().values[0].clone())
}
fn read(
    session: &mut ProgramSession,
    object: &SessionValue,
    key: ProgramValue,
) -> RuntimeResult<ProgramValue> {
    let mut args = vec![object.clone()];
    args.extend(session.borrow(&ProgramValueGraph {
        values: vec![key],
        tables: vec![],
    })?);
    scalar(session, 1, &args)
}
fn write(
    session: &mut ProgramSession,
    object: &SessionValue,
    key: ProgramValue,
    value: ProgramValue,
) -> RuntimeResult<()> {
    let mut args = vec![object.clone()];
    args.extend(session.borrow(&ProgramValueGraph {
        values: vec![key, value],
        tables: vec![],
    })?);
    session.invoke(ParserCallbackId(2), &args).map(|_| ())
}
fn kind<T>(result: RuntimeResult<T>, expected: ProgramRuntimeErrorKind) {
    assert_eq!(result.err().expect("expected error").kind, expected);
}
const UNKNOWN: ProgramRuntimeErrorKind = ProgramRuntimeErrorKind::UnsupportedCapability;

#[test]
fn present_false_proven_absence_and_unavailable_are_distinct() {
    let mut coverage = coverage(SourceTableInventory::Selective);
    coverage.known_absent.insert(key("absent"));
    coverage.unavailable.insert(key("hidden"));
    let (mut session, roots) = compile()
        .session_with_coverage(
            &graph(vec![(text("flag"), ProgramValue::Boolean(false))]),
            &meta(coverage),
            ProgramLimits::default(),
        )
        .unwrap();
    assert_eq!(
        read(&mut session, &roots[0], text("flag")).unwrap(),
        ProgramValue::Boolean(false)
    );
    assert_eq!(
        read(&mut session, &roots[0], text("absent")).unwrap(),
        ProgramValue::Nil
    );
    for key in [
        text("hidden"),
        text("unknown"),
        ProgramValue::Boolean(true),
        ProgramValue::Number(0.5),
        ProgramValue::Bytes(vec![0xff]),
        ProgramValue::Number(f64::INFINITY),
    ] {
        kind(read(&mut session, &roots[0], key), UNKNOWN);
    }
    assert_eq!(
        read(&mut session, &roots[0], ProgramValue::Number(f64::NAN)).unwrap(),
        ProgramValue::Nil
    );
    assert_eq!(
        read(&mut session, &roots[0], ProgramValue::Nil).unwrap(),
        ProgramValue::Nil
    );
    kind(session.snapshot(&roots), UNKNOWN);
}

#[test]
fn writes_resolve_private_coverage_without_reading_unknown_prior_values() {
    let mut coverage = coverage(SourceTableInventory::Selective);
    coverage.unavailable.insert(key("hidden"));
    let (mut session, roots) = compile()
        .session_with_coverage(&graph(vec![]), &meta(coverage), ProgramLimits::default())
        .unwrap();
    write(
        &mut session,
        &roots[0],
        text("hidden"),
        ProgramValue::Boolean(false),
    )
    .unwrap();
    assert_eq!(
        read(&mut session, &roots[1], text("hidden")).unwrap(),
        ProgramValue::Boolean(false)
    );
    write(
        &mut session,
        &roots[0],
        text("never_observed"),
        ProgramValue::Nil,
    )
    .unwrap();
    assert_eq!(
        read(&mut session, &roots[1], text("never_observed")).unwrap(),
        ProgramValue::Nil
    );
    for key in [
        ProgramValue::Boolean(true),
        ProgramValue::Number(0.5),
        ProgramValue::Bytes(vec![0xff]),
    ] {
        write(
            &mut session,
            &roots[0],
            key.clone(),
            ProgramValue::Number(9.0),
        )
        .unwrap();
        assert_eq!(
            read(&mut session, &roots[1], key.clone()).unwrap(),
            ProgramValue::Number(9.0)
        );
        write(&mut session, &roots[0], key.clone(), ProgramValue::Nil).unwrap();
        assert_eq!(
            read(&mut session, &roots[1], key).unwrap(),
            ProgramValue::Nil
        );
    }
    let missing = session
        .borrow(&ProgramValueGraph {
            values: vec![text("later")],
            tables: vec![],
        })
        .unwrap();
    kind(
        session.invoke(ParserCallbackId(6), &[roots[0].clone(), missing[0].clone()]),
        UNKNOWN,
    );
    assert_eq!(
        read(&mut session, &roots[1], text("earlier")).unwrap(),
        ProgramValue::Number(1.0)
    );
    kind(session.snapshot(&roots), UNKNOWN);
}

#[test]
fn index_fallback_is_distinct_from_raw_absence_and_does_not_block_plain_writes() {
    let mut coverage = coverage(SourceTableInventory::Selective);
    coverage.known_absent.insert(SourceTableKey::Integer(2));
    coverage.known_absent.insert(key("absent"));
    coverage.index_fallback = SourceTableIndexFallback::Unavailable;
    let (mut session, roots) = compile()
        .session_with_coverage(
            &graph(vec![(ProgramValue::Number(1.0), ProgramValue::Number(7.0))]),
            &meta(coverage),
            ProgramLimits::default(),
        )
        .unwrap();
    kind(read(&mut session, &roots[0], text("absent")), UNKNOWN);
    assert_eq!(
        scalar(&mut session, 4, &roots[..1]).unwrap(),
        ProgramValue::Number(7.0)
    );
    write(
        &mut session,
        &roots[0],
        text("unknown"),
        ProgramValue::Number(3.0),
    )
    .unwrap();
    assert_eq!(
        read(&mut session, &roots[1], text("unknown")).unwrap(),
        ProgramValue::Number(3.0)
    );
    write(&mut session, &roots[0], text("unknown"), ProgramValue::Nil).unwrap();
    kind(read(&mut session, &roots[0], text("unknown")), UNKNOWN);
    kind(session.snapshot(&roots), UNKNOWN);
    let lua = Lua::new();
    let (sum,present): (f64,f64) = lua.load("local t=setmetatable({7},{__index=function()return 99 end}); local n=0; for _,v in ipairs(t)do n=n+v end; t.unknown=3; return n,t.unknown").eval().unwrap();
    assert_eq!((sum, present), (7.0, 3.0));
}

#[test]
fn length_requires_integer_inventory_but_ipairs_can_stop_at_an_exact_absence() {
    let library = compile();
    let g = graph(vec![(ProgramValue::Number(1.0), ProgramValue::Number(7.0))]);
    let mut selective = coverage(SourceTableInventory::Selective);
    selective.known_absent.insert(SourceTableKey::Integer(2));
    selective.unavailable.insert(SourceTableKey::Integer(3));
    let (mut session, roots) = library
        .session_with_coverage(&g, &meta(selective), ProgramLimits::default())
        .unwrap();
    assert_eq!(
        scalar(&mut session, 4, &roots[..1]).unwrap(),
        ProgramValue::Number(7.0)
    );
    kind(scalar(&mut session, 3, &roots[..1]), UNKNOWN);
    let mut complete = coverage(SourceTableInventory::Complete);
    complete.unavailable.insert(key("hidden_hash"));
    let (mut session, roots) = library
        .session_with_coverage(&g, &meta(complete.clone()), ProgramLimits::default())
        .unwrap();
    assert_eq!(
        scalar(&mut session, 3, &roots[..1]).unwrap(),
        ProgramValue::Number(1.0)
    );
    complete.unavailable.insert(SourceTableKey::Integer(2));
    let (mut session, roots) = library
        .session_with_coverage(&g, &meta(complete), ProgramLimits::default())
        .unwrap();
    kind(scalar(&mut session, 3, &roots[..1]), UNKNOWN);
    kind(scalar(&mut session, 4, &roots[..1]), UNKNOWN);
}

#[test]
fn resolved_complete_inventory_can_snapshot_but_unknown_index_behavior_cannot() {
    let library = compile();
    let mut coverage = coverage(SourceTableInventory::Complete);
    coverage.unavailable.insert(key("hidden"));
    let (mut session, roots) = library
        .session_with_coverage(
            &graph(vec![]),
            &meta(coverage.clone()),
            ProgramLimits::default(),
        )
        .unwrap();
    kind(session.snapshot(&roots), UNKNOWN);
    write(&mut session, &roots[0], text("hidden"), ProgramValue::Nil).unwrap();
    let output = session.snapshot(&roots).unwrap();
    assert_eq!(output.graph().values[0], output.graph().values[1]);
    assert!(output.graph().tables[0].entries.is_empty());
    coverage.index_fallback = SourceTableIndexFallback::Unavailable;
    let (mut session, roots) = library
        .session_with_coverage(&graph(vec![]), &meta(coverage), ProgramLimits::default())
        .unwrap();
    write(
        &mut session,
        &roots[0],
        text("hidden"),
        ProgramValue::Number(2.0),
    )
    .unwrap();
    kind(session.snapshot(&roots), UNKNOWN);
}

#[test]
fn imports_are_scoped_borrowed_tables_stay_readonly_and_foreign_handles_fail() {
    let library = compile();
    let (mut session, _) = library
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    let first = session
        .import_with_coverage(
            &graph(vec![(text("value"), ProgramValue::Number(1.0))]),
            &meta(coverage(SourceTableInventory::Selective)),
        )
        .unwrap();
    let second = session
        .import_with_coverage(
            &graph(vec![(text("value"), ProgramValue::Number(2.0))]),
            &meta(coverage(SourceTableInventory::Selective)),
        )
        .unwrap();
    write(
        &mut session,
        &first[0],
        text("value"),
        ProgramValue::Number(3.0),
    )
    .unwrap();
    assert_eq!(
        read(&mut session, &first[1], text("value")).unwrap(),
        ProgramValue::Number(3.0)
    );
    assert_eq!(
        read(&mut session, &second[0], text("value")).unwrap(),
        ProgramValue::Number(2.0)
    );
    let borrowed = session
        .borrow_with_coverage(
            &graph(vec![]),
            &meta(coverage(SourceTableInventory::Selective)),
        )
        .unwrap();
    kind(
        write(
            &mut session,
            &borrowed[0],
            text("x"),
            ProgramValue::Number(1.0),
        ),
        UNKNOWN,
    );
    let (_, foreign) = library
        .session_with_coverage(
            &graph(vec![]),
            &meta(coverage(SourceTableInventory::Selective)),
            ProgramLimits::default(),
        )
        .unwrap();
    kind(
        read(&mut session, &foreign[0], text("value")),
        ProgramRuntimeErrorKind::InvalidInput,
    );
}

#[test]
fn definition_environment_coverage_is_owner_bound_and_readonly() {
    let library = compile();
    let root = library
        .catalog()
        .owner()
        .bind_environment()
        .unwrap()
        .unwrap();
    let (_, foreign) = {
        let owner = compile();
        let root = owner.catalog().owner().bind_environment().unwrap().unwrap();
        (owner, root)
    };
    let (mut session, _) = library
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    let value = session.definition(&root).unwrap();
    assert_eq!(
        read(&mut session, &value, text("value")).unwrap(),
        ProgramValue::Number(42.0)
    );
    assert_eq!(
        read(&mut session, &value, text("missing")).unwrap(),
        ProgramValue::Nil
    );
    kind(read(&mut session, &value, text("hidden")), UNKNOWN);
    kind(
        write(&mut session, &value, text("new"), ProgramValue::Number(1.0)),
        UNKNOWN,
    );
    kind(
        session.definition(&foreign),
        ProgramRuntimeErrorKind::InvalidInput,
    );
    kind(session.snapshot(&[value]), UNKNOWN);
}

#[test]
fn failed_metadata_imports_charge_budget_without_publishing_partial_tables() {
    let library = compile();
    let limits = ProgramLimits {
        max_values: 48,
        ..ProgramLimits::default()
    };
    let (mut session, _) = library
        .session(&ProgramValueGraph::default(), limits)
        .unwrap();
    let mut conflicting = coverage(SourceTableInventory::Selective);
    conflicting.known_absent.insert(key("present"));
    let g = graph(vec![(text("present"), ProgramValue::Number(1.0))]);
    let before = session.allocations();
    kind(
        session.import_with_coverage(&g, &meta(conflicting.clone())),
        ProgramRuntimeErrorKind::InvalidInput,
    );
    assert!(session.allocations().values > before.values);
    assert!(session.allocations().bytes > before.bytes);
    let good = session
        .import_with_coverage(
            &graph(vec![]),
            &meta(coverage(SourceTableInventory::Complete)),
        )
        .unwrap();
    assert_eq!(session.snapshot(&good).unwrap().graph().tables.len(), 1);
    let mut exhausted = false;
    for _ in 0..16 {
        if session
            .import_with_coverage(&g, &meta(conflicting.clone()))
            .unwrap_err()
            .kind
            == ProgramRuntimeErrorKind::ResourceBound
        {
            exhausted = true;
            break;
        }
    }
    assert!(exhausted);
    let (mut session, _) = library
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    let invalid_id =
        BTreeMap::from([(ProgramTableId(2), coverage(SourceTableInventory::Selective))]);
    kind(
        session.borrow_with_coverage(&graph(vec![]), &invalid_id),
        ProgramRuntimeErrorKind::InvalidInput,
    );
    let mut collision = coverage(SourceTableInventory::Selective);
    collision.known_absent.insert(SourceTableKey::Integer(0));
    kind(
        session.borrow_with_coverage(
            &graph(vec![(
                ProgramValue::Number(-0.0),
                ProgramValue::Number(1.0),
            )]),
            &meta(collision),
        ),
        ProgramRuntimeErrorKind::InvalidInput,
    );
}

#[test]
fn missing_method_lookup_preserves_effect_order_for_unknown_and_proven_absence() {
    let library = compile();
    for inventory in [
        SourceTableInventory::Selective,
        SourceTableInventory::Complete,
    ] {
        let (mut session, roots) = library
            .session_with_coverage(
                &graph(vec![]),
                &meta(coverage(inventory)),
                ProgramLimits::default(),
            )
            .unwrap();
        let marker = session
            .import_with_coverage(&graph(vec![]), &ProgramTableCoverage::new())
            .unwrap();
        let expected = if inventory == SourceTableInventory::Selective {
            UNKNOWN
        } else {
            ProgramRuntimeErrorKind::Source
        };
        kind(
            session.invoke(ParserCallbackId(7), &[roots[0].clone(), marker[0].clone()]),
            expected,
        );
        assert_eq!(
            read(&mut session, &marker[0], text("ran")).unwrap(),
            if inventory == SourceTableInventory::Selective {
                ProgramValue::Nil
            } else {
                ProgramValue::Number(1.0)
            }
        );
    }
}

#[test]
fn unrepresented_call_behavior_fails_after_arguments_and_cannot_be_snapshotted() {
    let library = compile();
    for fallback in [
        SourceTableCallFallback::Unavailable,
        SourceTableCallFallback::NonCallable,
    ] {
        let mut callable = coverage(SourceTableInventory::Complete);
        callable.call_fallback = fallback;
        let input = ProgramValueGraph {
            values: vec![table(1), table(2)],
            tables: vec![
                ProgramTable {
                    entries: vec![(text("missing"), table(2))],
                },
                ProgramTable::default(),
            ],
        };
        let (mut session, roots) = library
            .session_with_coverage(
                &input,
                &BTreeMap::from([(ProgramTableId(2), callable)]),
                ProgramLimits::default(),
            )
            .unwrap();
        let marker = session
            .import_with_coverage(&graph(vec![]), &ProgramTableCoverage::new())
            .unwrap();
        let expected = if fallback == SourceTableCallFallback::Unavailable {
            UNKNOWN
        } else {
            ProgramRuntimeErrorKind::Source
        };
        kind(
            session.invoke(ParserCallbackId(7), &[roots[0].clone(), marker[0].clone()]),
            expected,
        );
        assert_eq!(
            read(&mut session, &marker[0], text("ran")).unwrap(),
            ProgramValue::Number(1.0)
        );
        kind(session.invoke_callable(&roots[1], &[]), expected);
        write(
            &mut session,
            &roots[1],
            text("ordinary"),
            ProgramValue::Number(3.0),
        )
        .unwrap();
        assert_eq!(
            read(&mut session, &roots[1], text("ordinary")).unwrap(),
            ProgramValue::Number(3.0)
        );
        if fallback == SourceTableCallFallback::Unavailable {
            kind(session.snapshot(&roots), UNKNOWN);
        } else {
            session.snapshot(&roots).unwrap();
        }
    }
    let lua = Lua::new();
    let ran: f64 = lua.load("local marker={}; local t={missing=setmetatable({},{__call=function()error('call reached')end})}; local function arg()marker.ran=1 end; pcall(function()t:missing(arg())end); return marker.ran").eval().unwrap();
    assert_eq!(ran, 1.0);
}

#[test]
fn coverage_import_charges_exact_copied_text_without_recopying_input_keys() {
    let mut coverage = coverage(SourceTableInventory::Selective);
    coverage.known_absent.insert(key("missing"));
    let g = graph(vec![(text(&"x".repeat(100)), ProgramValue::Number(1.0))]);
    let (session, _) = compile()
        .session_with_coverage(&g, &meta(coverage), ProgramLimits::default())
        .unwrap();
    assert_eq!(session.allocations().bytes, 107);
}
