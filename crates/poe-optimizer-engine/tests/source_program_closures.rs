//! Authored structural IR with independent Lua comparisons. Complete original
//! control/source callback admission is tested in the PoB adapter crate.
#![cfg(not(target_arch = "wasm32"))]
use mlua::Lua;
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
fn cap(upvalue: u16) -> ParserProgramExpr {
    e(ParserProgramExprKind::Capture { upvalue })
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
fn pack(values: Vec<ParserProgramExpr>) -> ParserProgramValueList {
    ParserProgramValueList { values, tail: None }
}
fn ret(values: Vec<ParserProgramExpr>) -> ParserProgramStatement {
    s(ParserProgramStatementKind::Return {
        values: pack(values),
    })
}
fn store(upvalue: u16, values: Vec<ParserProgramExpr>) -> ParserProgramStatement {
    s(ParserProgramStatementKind::CaptureSet {
        upvalue,
        values: pack(values),
    })
}
fn call(target: ParserProgramExpr, args: Vec<ParserProgramExpr>) -> ParserProgramCall {
    ParserProgramCall {
        binding: 0,
        receiver: Some(Box::new(target)),
        arguments: pack(args),
    }
}
fn invoke(target: ParserProgramExpr, args: Vec<ParserProgramExpr>) -> ParserProgramExpr {
    e(ParserProgramExprKind::Call {
        call: Box::new(call(target, args)),
    })
}
fn binary(
    operation: ParserProgramBinary,
    left: ParserProgramExpr,
    right: ParserProgramExpr,
) -> ParserProgramExpr {
    e(ParserProgramExprKind::Binary {
        operation,
        left: Box::new(left),
        right: Box::new(right),
    })
}
fn text(value: &str) -> ProgramValue {
    ProgramValue::Bytes(value.as_bytes().to_vec())
}
fn tab(id: u32) -> ProgramValue {
    ProgramValue::Table(ProgramTableId(id))
}
fn cl(id: u32) -> ProgramValue {
    ProgramValue::Closure(SourceSessionClosureId(id))
}
fn library(bodies: Vec<(usize, Vec<ParserProgramStatement>)>) -> CompiledSourcePrograms {
    let path = "src/Modules/ClosureFixture.lua".to_owned();
    let span = ItemSourceSpan {
        path: path.clone(),
        line: 1,
        end_line: 20,
        sha256: "a".repeat(64),
    };
    let mut callbacks: Vec<_> = bodies
        .iter()
        .map(|(count, _)| ParserCallback {
            kind: ParserCallbackKind::Lua {
                source: span.clone(),
            },
            environment: ParserEnvironment::OriginalGlobals,
            upvalues: (0..*count)
                .map(|i| ParserUpvalue {
                    name: format!("capture{i}"),
                    value: ParserValue::LiveCapture {},
                })
                .collect(),
        })
        .collect();
    let type_id = ParserCallbackId(callbacks.len() as u32 + 1);
    callbacks.push(ParserCallback {
        kind: ParserCallbackKind::Builtin {
            symbol: "type".into(),
        },
        environment: ParserEnvironment::OriginalGlobals,
        upvalues: vec![],
    });
    let owner = SourceProgramOwner::new_with_closures(
        SourceProgramDefinitions {
            schema_version: SOURCE_PROGRAM_DEFINITIONS_SCHEMA_VERSION,
            source: ItemLoadingSource {
                upstream_revision: "b".repeat(40),
                files: BTreeMap::from([(path.clone(), "a".repeat(64))]),
                construction_spans: BTreeMap::new(),
                module_order: vec![path],
            },
            tables: vec![SourceTable {
                fields: BTreeMap::from([("field".into(), SourceValue::Text("x".into()))]),
                indexed: BTreeMap::new(),
            }],
            callbacks,
            roots: vec![],
            intrinsics: BTreeMap::from([(type_id, ParserProgramIntrinsic::Type)]),
        },
        None,
        None,
        SourceClosurePrototypes {
            schema_version: SOURCE_CLOSURE_PROTOTYPES_SCHEMA_VERSION,
            prototypes: (0..bodies.len())
                .map(|i| SourceClosurePrototype {
                    callback: SourceCallbackId(i as u32 + 1),
                })
                .collect(),
        },
    )
    .unwrap();
    let programs: Vec<_> = bodies
        .into_iter()
        .enumerate()
        .map(|(i, (_, body))| ParserProgram {
            callback: ParserCallbackId(i as u32 + 1),
            parameter_count: 3,
            local_count: 3,
            variadic: false,
            bindings: vec![ParserProgramBinding::DynamicCall {}],
            body,
            provenance: ParserProgramProvenance {
                source: span.clone(),
                function_start: 0,
                function_end: 256,
                function_sha256: "c".repeat(64),
            },
        })
        .collect();
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
fn input(
    library: &CompiledSourcePrograms,
    instances: &[(u32, Vec<u32>)],
    cells: Vec<ProgramValue>,
    state: ProgramValueGraph,
) -> SourceSessionInput {
    let owner = library.catalog().owner().clone();
    let closures = instances
        .iter()
        .map(|(id, captures)| SourceSessionClosure {
            prototype: owner
                .bind_closure_prototype(SourceClosurePrototypeId(*id))
                .unwrap(),
            captures: captures.iter().map(|i| SourceSessionCellId(*i)).collect(),
        })
        .collect();
    SourceSessionInput {
        class_bindings: BTreeMap::new(),
        owner,
        state,
        coverage: BTreeMap::new(),
        cells,
        closures,
    }
}
fn graph(values: Vec<ProgramValue>) -> ProgramValueGraph {
    ProgramValueGraph {
        values,
        tables: vec![],
    }
}
fn call_scalar(
    session: &mut ProgramSession,
    closure: &SessionValue,
    args: &[SessionValue],
) -> RuntimeResult<ProgramValue> {
    let values = session.invoke_callable(closure, args)?;
    Ok(session.snapshot(&values)?.graph().values[0].clone())
}
fn scalars(session: &mut ProgramSession, values: Vec<ProgramValue>) -> Vec<SessionValue> {
    session.borrow(&graph(values)).unwrap()
}
fn error<T>(value: RuntimeResult<T>, expected: ProgramRuntimeErrorKind) {
    assert_eq!(value.err().expect("expected error").kind, expected)
}
const UNKNOWN: ProgramRuntimeErrorKind = ProgramRuntimeErrorKind::UnsupportedCapability;
fn counter() -> Vec<ParserProgramStatement> {
    vec![
        store(0, vec![binary(ParserProgramBinary::Add, cap(0), l(0))]),
        ret(vec![cap(0)]),
    ]
}

#[test]
fn source_counters_share_cells_but_keep_distinct_function_and_session_identity() {
    let lib = library(vec![
        (1, counter()),
        (1, vec![ret(vec![cap(0)])]),
        (
            0,
            vec![ret(vec![binary(ParserProgramBinary::Equal, l(0), l(1))])],
        ),
    ]);
    let artifact = input(
        &lib,
        &[
            (1, vec![1]),
            (2, vec![1]),
            (1, vec![2]),
            (1, vec![1]),
            (3, vec![]),
        ],
        vec![ProgramValue::Number(0.0), ProgramValue::Number(0.0)],
        graph(vec![cl(1), cl(2), cl(3), cl(1), cl(4), cl(5)]),
    );
    let (mut session, roots) = lib
        .session_from_input(&artifact, ProgramLimits::default())
        .unwrap();
    let one = scalars(&mut session, vec![ProgramValue::Number(1.0)]);
    let two = scalars(&mut session, vec![ProgramValue::Number(2.0)]);
    assert_eq!(
        call_scalar(&mut session, &roots[0], &one).unwrap(),
        ProgramValue::Number(1.0)
    );
    assert_eq!(
        call_scalar(&mut session, &roots[1], &[]).unwrap(),
        ProgramValue::Number(1.0)
    );
    assert_eq!(
        call_scalar(&mut session, &roots[2], &two).unwrap(),
        ProgramValue::Number(2.0)
    );
    assert_eq!(
        call_scalar(&mut session, &roots[4], &two).unwrap(),
        ProgramValue::Number(3.0)
    );
    assert_eq!(
        call_scalar(
            &mut session,
            &roots[5],
            &[roots[0].clone(), roots[3].clone()]
        )
        .unwrap(),
        ProgramValue::Boolean(true)
    );
    assert_eq!(
        call_scalar(
            &mut session,
            &roots[5],
            &[roots[0].clone(), roots[4].clone()]
        )
        .unwrap(),
        ProgramValue::Boolean(false)
    );
    let (mut other, other_roots) = lib
        .session_from_input(&artifact, ProgramLimits::default())
        .unwrap();
    assert_eq!(
        call_scalar(&mut other, &other_roots[1], &[]).unwrap(),
        ProgramValue::Number(0.0)
    );
    error(
        other.invoke_callable(&roots[0], &[]),
        ProgramRuntimeErrorKind::InvalidInput,
    );
    let lua = Lua::new();
    let result:(f64,f64,f64,f64)=lua.load("local x,y=0,0; local function a(n)x=x+n;return x end; local function get()return x end; local function b(n)y=y+n;return y end; local function c(n)x=x+n;return x end; return a(1),get(),b(2),c(2)").eval().unwrap();
    assert_eq!(result, (1.0, 1.0, 2.0, 3.0));
}

#[test]
fn function_table_keys_repeated_aliases_and_cyclic_captures_retain_identity() {
    let lib = library(vec![
        (1, vec![ret(vec![cap(0)])]),
        (0, vec![ret(vec![get(l(0), l(1))])]),
        (
            0,
            vec![ret(vec![binary(ParserProgramBinary::Equal, l(0), l(1))])],
        ),
        (1, vec![ret(vec![invoke(cap(0), vec![l(0)])])]),
    ]);
    let type_id = ParserCallbackId(5);
    let artifact = input(
        &lib,
        &[(1, vec![1]), (2, vec![]), (3, vec![]), (4, vec![2])],
        vec![cl(1), ProgramValue::Callback(type_id)],
        ProgramValueGraph {
            values: vec![cl(1), cl(2), cl(3), cl(4), tab(1)],
            tables: vec![ProgramTable {
                entries: vec![
                    (cl(1), ProgramValue::Number(17.0)),
                    (text("again"), cl(1)),
                    (text("self"), tab(1)),
                ],
            }],
        },
    );
    let (mut session, roots) = lib
        .session_from_input(&artifact, ProgramLimits::default())
        .unwrap();
    let returned = session.invoke_callable(&roots[0], &[]).unwrap();
    assert_eq!(
        call_scalar(
            &mut session,
            &roots[2],
            &[returned[0].clone(), roots[0].clone()]
        )
        .unwrap(),
        ProgramValue::Boolean(true)
    );
    assert_eq!(
        call_scalar(
            &mut session,
            &roots[1],
            &[roots[4].clone(), returned[0].clone()]
        )
        .unwrap(),
        ProgramValue::Number(17.0)
    );
    assert_eq!(
        call_scalar(&mut session, &roots[3], &roots[..1]).unwrap(),
        text("function")
    );
    error(session.snapshot(&returned), UNKNOWN);
    error(session.snapshot(&roots[4..]), UNKNOWN);
}

#[test]
fn captured_call_target_is_resolved_before_arguments_can_replace_its_shared_cell() {
    let lib = library(vec![
        (
            3,
            vec![ret(vec![invoke(
                cap(0),
                vec![invoke(cap(1), vec![cap(2)])],
            )])],
        ),
        (1, vec![store(0, vec![l(0)]), ret(vec![])]),
        (0, vec![ret(vec![n(11.0)])]),
        (0, vec![ret(vec![n(22.0)])]),
    ]);
    let artifact = input(
        &lib,
        &[(1, vec![1, 2, 3]), (2, vec![1]), (3, vec![]), (4, vec![])],
        vec![cl(3), cl(2), cl(4)],
        graph(vec![cl(1), cl(2)]),
    );
    let (mut session, roots) = lib
        .session_from_input(&artifact, ProgramLimits::default())
        .unwrap();
    assert_eq!(
        call_scalar(&mut session, &roots[0], &[]).unwrap(),
        ProgramValue::Number(11.0)
    );
    assert_eq!(
        call_scalar(&mut session, &roots[0], &[]).unwrap(),
        ProgramValue::Number(22.0)
    );
    let lua = Lua::new();
    let result:(f64,f64)=lua.load("local old=function()return 11 end;local new=function()return 22 end;local target=old;local function set(f)target=f end;local function call()return target(set(new))end;return call(),call()").eval().unwrap();
    assert_eq!(result, (11.0, 22.0));
    let mut invalid = artifact.clone();
    invalid.cells[0] = ProgramValue::Boolean(false);
    let (mut session, roots) = lib
        .session_from_input(&invalid, ProgramLimits::default())
        .unwrap();
    error(
        session.invoke_callable(&roots[0], &[]),
        ProgramRuntimeErrorKind::Source,
    );
    assert_eq!(
        call_scalar(&mut session, &roots[0], &[]).unwrap(),
        ProgramValue::Number(22.0)
    );
}

#[test]
fn capture_store_evaluates_every_rhs_then_uses_first_or_nil_and_preserves_prior_effects_on_failure()
{
    let tail = ParserProgramValueList {
        values: vec![],
        tail: Some(Box::new(ParserProgramPack::Call {
            call: call(cap(1), vec![]),
        })),
    };
    let lib = library(vec![
        (
            2,
            vec![
                store(0, vec![n(42.0), invoke(cap(1), vec![n(1.0)])]),
                ret(vec![cap(0)]),
            ],
        ),
        (1, counter()),
        (1, vec![ret(vec![cap(0)])]),
        (
            2,
            vec![
                s(ParserProgramStatementKind::CaptureSet {
                    upvalue: 0,
                    values: tail,
                }),
                ret(vec![cap(0)]),
            ],
        ),
        (0, vec![ret(vec![])]),
        (
            2,
            vec![
                store(
                    0,
                    vec![
                        n(99.0),
                        invoke(cap(1), vec![n(1.0)]),
                        get(l(0), b("missing")),
                    ],
                ),
                ret(vec![cap(0)]),
            ],
        ),
    ]);
    let artifact = input(
        &lib,
        &[
            (1, vec![1, 3]),
            (2, vec![2]),
            (3, vec![1]),
            (3, vec![2]),
            (4, vec![1, 4]),
            (5, vec![]),
            (6, vec![1, 3]),
        ],
        vec![
            ProgramValue::Number(0.0),
            ProgramValue::Number(0.0),
            cl(2),
            cl(6),
        ],
        graph(vec![cl(1), cl(3), cl(4), cl(5), cl(7)]),
    );
    let (mut session, roots) = lib
        .session_from_input(&artifact, ProgramLimits::default())
        .unwrap();
    assert_eq!(
        call_scalar(&mut session, &roots[0], &[]).unwrap(),
        ProgramValue::Number(42.0)
    );
    assert_eq!(
        call_scalar(&mut session, &roots[2], &[]).unwrap(),
        ProgramValue::Number(1.0)
    );
    error(
        session.invoke_callable(&roots[4], &[]),
        ProgramRuntimeErrorKind::Source,
    );
    assert_eq!(
        call_scalar(&mut session, &roots[1], &[]).unwrap(),
        ProgramValue::Number(42.0)
    );
    assert_eq!(
        call_scalar(&mut session, &roots[2], &[]).unwrap(),
        ProgramValue::Number(2.0)
    );
    assert_eq!(
        call_scalar(&mut session, &roots[3], &[]).unwrap(),
        ProgramValue::Nil
    );
    let lua = Lua::new();
    let result: (f64, f64) = lua
        .load("local x,y=0,0;local function next()y=y+1 end;x=42,next();return x,y")
        .eval()
        .unwrap();
    assert_eq!(result, (42.0, 1.0));
}

#[test]
fn live_tables_and_immutable_definition_references_share_correct_aliases() {
    let lib = library(vec![
        (
            2,
            vec![
                s(ParserProgramStatementKind::TableSet {
                    table: cap(0),
                    key: get(cap(1), b("field")),
                    value: l(0),
                }),
                ret(vec![get(cap(0), b("x"))]),
            ],
        ),
        (0, vec![ret(vec![get(l(0), b("x"))])]),
        (
            1,
            vec![
                s(ParserProgramStatementKind::TableSet {
                    table: cap(0),
                    key: b("field"),
                    value: l(0),
                }),
                ret(vec![]),
            ],
        ),
    ]);
    let artifact = input(
        &lib,
        &[(1, vec![1, 2]), (2, vec![]), (3, vec![2])],
        vec![tab(1), ProgramValue::DefinitionTable(SourceTableId(1))],
        ProgramValueGraph {
            values: vec![cl(1), cl(2), cl(3), tab(1), tab(1)],
            tables: vec![ProgramTable::default()],
        },
    );
    let (mut session, roots) = lib
        .session_from_input(&artifact, ProgramLimits::default())
        .unwrap();
    let value = scalars(&mut session, vec![ProgramValue::Number(-0.0)]);
    assert_eq!(
        call_scalar(&mut session, &roots[0], &value).unwrap(),
        ProgramValue::Number(-0.0)
    );
    assert_eq!(
        call_scalar(&mut session, &roots[1], &roots[4..]).unwrap(),
        ProgramValue::Number(-0.0)
    );
    let output = session.snapshot(&roots[3..]).unwrap();
    assert_eq!(output.graph().values[0], output.graph().values[1]);
    let ProgramValue::Number(number) = output.graph().tables[0].entries[0].1 else {
        panic!()
    };
    assert_eq!(number.to_bits(), (-0.0f64).to_bits());
    error(session.invoke_callable(&roots[2], &value), UNKNOWN);
}

#[test]
fn all_artifact_ids_are_scoped_and_foreign_owners_fail_even_without_closures() {
    let lib = library(vec![(1, vec![ret(vec![cap(0)])])]);
    let first = input(
        &lib,
        &[(1, vec![1])],
        vec![ProgramValue::Number(1.0)],
        graph(vec![cl(1)]),
    );
    let (mut session, roots) = lib
        .session_from_input(&first, ProgramLimits::default())
        .unwrap();
    let mut second = first.clone();
    second.cells[0] = ProgramValue::Number(2.0);
    let added = session.import_session_input(&second).unwrap();
    assert_eq!(
        call_scalar(&mut session, &roots[0], &[]).unwrap(),
        ProgramValue::Number(1.0)
    );
    assert_eq!(
        call_scalar(&mut session, &added[0], &[]).unwrap(),
        ProgramValue::Number(2.0)
    );
    let other = library(vec![(1, vec![ret(vec![cap(0)])])]);
    let mut foreign = input(
        &other,
        &[],
        vec![],
        graph(vec![ProgramValue::DefinitionTable(SourceTableId(1))]),
    );
    let before = session.allocations();
    error(
        session.import_session_input(&foreign),
        ProgramRuntimeErrorKind::InvalidInput,
    );
    assert_eq!(session.allocations(), before);
    foreign.owner = lib.catalog().owner().clone();
    foreign.closures = vec![SourceSessionClosure {
        prototype: other
            .catalog()
            .owner()
            .bind_closure_prototype(SourceClosurePrototypeId(1))
            .unwrap(),
        captures: vec![],
    }];
    error(
        session.import_session_input(&foreign),
        ProgramRuntimeErrorKind::InvalidInput,
    );
    assert_eq!(session.allocations(), before);
}

#[test]
fn malformed_graphs_do_not_publish_instances_and_failed_imports_consume_budgets() {
    let lib = library(vec![(1, vec![ret(vec![cap(0)])])]);
    let good = input(
        &lib,
        &[(1, vec![1])],
        vec![ProgramValue::Number(9.0)],
        graph(vec![cl(1)]),
    );
    let (mut session, roots) = lib
        .session_from_input(
            &good,
            ProgramLimits {
                max_values: 100,
                ..ProgramLimits::default()
            },
        )
        .unwrap();
    let mut bad = good.clone();
    bad.closures[0].captures[0] = SourceSessionCellId(2);
    let before = session.allocations();
    error(
        session.import_session_input(&bad),
        ProgramRuntimeErrorKind::InvalidInput,
    );
    assert!(session.allocations().values > before.values);
    let mut cases = vec![];
    let mut case = good.clone();
    case.state.values[0] = cl(2);
    cases.push(case);
    let mut case = good.clone();
    case.cells[0] = tab(1);
    cases.push(case);
    let mut case = good.clone();
    case.closures[0].captures.clear();
    cases.push(case);
    let mut case = good.clone();
    case.state.tables.push(ProgramTable {
        entries: vec![
            (cl(1), ProgramValue::Number(1.0)),
            (cl(1), ProgramValue::Number(2.0)),
        ],
    });
    cases.push(case);
    for case in cases {
        error(
            session.import_session_input(&case),
            ProgramRuntimeErrorKind::InvalidInput,
        )
    }
    let added = session.import_session_input(&good).unwrap();
    assert_eq!(
        call_scalar(&mut session, &roots[0], &[]).unwrap(),
        ProgramValue::Number(9.0)
    );
    assert_eq!(
        call_scalar(&mut session, &added[0], &[]).unwrap(),
        ProgramValue::Number(9.0)
    );
    let mut exhausted = false;
    for _ in 0..100 {
        if session.import_session_input(&bad).unwrap_err().kind
            == ProgramRuntimeErrorKind::ResourceBound
        {
            exhausted = true;
            break;
        }
    }
    assert!(exhausted);
}

#[test]
fn plain_graphs_and_bare_prototypes_cannot_fabricate_live_function_identity() {
    let lib = library(vec![(0, vec![ret(vec![n(1.0)])])]);
    let (mut session, _) = lib
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    for value in [
        cl(1),
        ProgramValue::DefinitionTable(SourceTableId(1)),
        ProgramValue::Callback(ParserCallbackId(1)),
    ] {
        error(
            session.borrow(&graph(vec![value])),
            ProgramRuntimeErrorKind::InvalidInput,
        );
    }
    error(session.invoke(ParserCallbackId(1), &[]), UNKNOWN);
    error(
        lib.execute(
            ParserCallbackId(1),
            &ProgramValueGraph::default(),
            ProgramLimits::default(),
        ),
        UNKNOWN,
    );
    let good = input(&lib, &[(1, vec![])], vec![], graph(vec![cl(1)]));
    let roots = session.import_session_input(&good).unwrap();
    assert_eq!(
        call_scalar(&mut session, &roots[0], &[]).unwrap(),
        ProgramValue::Number(1.0)
    );
}

#[test]
fn parallel_sessions_share_compiled_prototypes_without_sharing_capture_cells() {
    let lib = library(vec![(1, counter())]);
    let artifact = input(
        &lib,
        &[(1, vec![1])],
        vec![ProgramValue::Number(0.0)],
        graph(vec![cl(1)]),
    );
    std::thread::scope(|scope| {
        let workers: Vec<_> = (0..8)
            .map(|_| {
                scope.spawn(|| {
                    let (mut session, roots) = lib
                        .session_from_input(&artifact, ProgramLimits::default())
                        .unwrap();
                    let one = scalars(&mut session, vec![ProgramValue::Number(1.0)]);
                    for expected in 1..=100 {
                        assert_eq!(
                            call_scalar(&mut session, &roots[0], &one).unwrap(),
                            ProgramValue::Number(expected as f64)
                        );
                    }
                })
            })
            .collect();
        for worker in workers {
            worker.join().unwrap();
        }
    });
}
