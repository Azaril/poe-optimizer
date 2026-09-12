//! Authored structural closure-creation evidence, not an original-source capture.
//! Actual FNEW observation and complete source comparisons belong to PoB tests.
use super::*;
type Origin = SourceProgramCaptureOrigin;
#[derive(Default)]
struct FactoryFixture {
    sites: Vec<(u32, ParserProgramExpr)>,
}
impl FactoryFixture {
    fn create(&mut self, parent: u32, child: u32, captures: Vec<Origin>) -> ParserProgramExpr {
        let at = 10 + self.sites.len() as u32;
        let expression = ParserProgramExpr {
            location: ParserProgramLocation {
                start: at,
                end: at + 1,
            },
            operation: ParserProgramExprKind::CreateClosure {
                prototype: SourceClosurePrototypeId(child),
                captures,
            },
        };
        self.sites.push((parent, expression.clone()));
        expression
    }
    fn catalog(self, bodies: Vec<(usize, Vec<ParserProgramStatement>)>) -> SourceProgramCatalog {
        let (data, owner) = fixture_parts(bodies, 16);
        let sites = self
            .sites
            .into_iter()
            .enumerate()
            .map(|(pc, (parent, expression))| {
                let ParserProgramExprKind::CreateClosure {
                    prototype,
                    captures,
                } = expression.operation
                else {
                    unreachable!()
                };
                let parent_program = &data.programs[parent as usize - 1];
                let mut locals = std::collections::BTreeSet::new();
                let local_bindings = captures
                    .iter()
                    .filter_map(|origin| match *origin {
                        Origin::Local { local } if locals.insert(local) => {
                            let (kind, declaration) = if local < 3 {
                                (
                                    SourceProgramClosureLocalKind::Parameter,
                                    ParserProgramLocation { start: 0, end: 1 },
                                )
                            } else {
                                declaration(&parent_program.body, local)
                                    .expect("declared test local")
                            };
                            Some(SourceProgramClosureLocalBinding {
                                local,
                                register: local as u8,
                                kind,
                                declaration,
                            })
                        }
                        _ => None,
                    })
                    .collect();
                let capture_descriptors = captures
                    .iter()
                    .map(|origin| match *origin {
                        Origin::Local { local } => 0x8000 | local,
                        Origin::ParentCapture { upvalue } => upvalue,
                    })
                    .collect();
                SourceProgramClosureCreation {
                    callback: SourceCallbackId(parent),
                    provenance: parent_program.provenance.clone(),
                    expression: expression.location,
                    prototype,
                    child_provenance: data.programs[prototype.0 as usize - 1].provenance.clone(),
                    bytecode_sha256: "d".repeat(64),
                    bytecode_pc: pc as u32 + 1,
                    instruction: 51 | (prototype.0 << 16),
                    child_bytecode_sha256: "d".repeat(64),
                    captures,
                    capture_descriptors,
                    local_bindings,
                }
            })
            .collect();
        SourceProgramCatalog::new_with_closure_creations(
            data,
            owner,
            None,
            SourceProgramClosureCreations {
                schema_version: SOURCE_PROGRAM_CLOSURE_CREATIONS_SCHEMA_VERSION,
                profile: SourceTableRuntimeProfile::luajit21_x64_single(),
                sites,
            },
        )
        .unwrap()
    }
    fn library(self, bodies: Vec<(usize, Vec<ParserProgramStatement>)>) -> CompiledSourcePrograms {
        CompiledSourcePrograms::new(&self.catalog(bodies)).unwrap()
    }
}
fn declaration(
    body: &[ParserProgramStatement],
    local: u16,
) -> Option<(SourceProgramClosureLocalKind, ParserProgramLocation)> {
    use ParserProgramStatementKind as S;
    for statement in body {
        let kind = match &statement.operation {
            S::Declare { locals, .. } if locals.contains(&local) => {
                Some(SourceProgramClosureLocalKind::Declare)
            }
            S::ForNumeric { local: found, .. } if *found == local => {
                Some(SourceProgramClosureLocalKind::NumericFor)
            }
            S::ForEach { locals, .. } if locals.contains(&local) => {
                Some(SourceProgramClosureLocalKind::GenericFor)
            }
            _ => None,
        };
        if let Some(kind) = kind {
            return Some((kind, statement.location));
        }
        match &statement.operation {
            S::ForNumeric { body, .. } | S::ForEach { body, .. } => {
                if let Some(found) = declaration(body, local) {
                    return Some(found);
                }
            }
            S::If {
                branches,
                otherwise,
            } => {
                for branch in branches {
                    if let Some(found) = declaration(&branch.body, local) {
                        return Some(found);
                    }
                }
                if let Some(found) = declaration(otherwise, local) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}
fn local(local: u16) -> Origin {
    Origin::Local { local }
}
fn inherited(upvalue: u16) -> Origin {
    Origin::ParentCapture { upvalue }
}
fn declare(local: u16, value: ParserProgramExpr) -> ParserProgramStatement {
    s(ParserProgramStatementKind::Declare {
        locals: vec![local],
        values: pack(vec![value]),
    })
}
fn assign(local: u16, value: ParserProgramExpr) -> ParserProgramStatement {
    s(ParserProgramStatementKind::Assign {
        locals: vec![local],
        values: pack(vec![value]),
    })
}
fn set(
    table: ParserProgramExpr,
    key: ParserProgramExpr,
    value: ParserProgramExpr,
) -> ParserProgramStatement {
    s(ParserProgramStatementKind::TableSet { table, key, value })
}
fn run(call: ParserProgramCall) -> ParserProgramStatement {
    s(ParserProgramStatementKind::Call { call })
}
fn add(left: ParserProgramExpr, right: ParserProgramExpr) -> ParserProgramExpr {
    binary(ParserProgramBinary::Add, left, right)
}
fn main_input(lib: &CompiledSourcePrograms) -> SourceSessionInput {
    input(lib, &[(1, vec![])], vec![], graph(vec![cl(1)]))
}
fn get_results(
    session: &mut ProgramSession,
    target: &SessionValue,
    args: &[SessionValue],
) -> Vec<ProgramValue> {
    let values = session.invoke_callable(target, args).unwrap();
    session.snapshot(&values).unwrap().graph().values.clone()
}

#[test]
fn created_siblings_share_promoted_parameters_and_keep_independent_factory_cells() {
    let mut f = FactoryFixture::default();
    let inc = f.create(1, 2, vec![local(0)]);
    let read = f.create(1, 3, vec![local(0)]);
    let lib = f.library(vec![
        (
            0,
            vec![
                declare(3, inc),
                declare(4, read),
                assign(0, add(l(0), n(1.0))),
                ret(vec![l(3), l(4)]),
            ],
        ),
        (1, counter()),
        (1, vec![ret(vec![cap(0)])]),
    ]);
    let (mut session, roots) = lib
        .session_from_input(&main_input(&lib), ProgramLimits::default())
        .unwrap();
    let args = scalars(&mut session, vec![ProgramValue::Number(6.0)]);
    let first = session.invoke_callable(&roots[0], &args).unwrap();
    let second = session.invoke_callable(&roots[0], &args).unwrap();
    let one = scalars(&mut session, vec![ProgramValue::Number(1.0)]);
    assert_eq!(
        call_scalar(&mut session, &first[0], &one).unwrap(),
        ProgramValue::Number(8.0)
    );
    assert_eq!(
        call_scalar(&mut session, &first[1], &[]).unwrap(),
        ProgramValue::Number(8.0)
    );
    assert_eq!(
        call_scalar(&mut session, &second[1], &[]).unwrap(),
        ProgramValue::Number(7.0)
    );
    error(session.snapshot(&first), UNKNOWN);
    let source: (i64, i64, i64) = Lua::new().load("local function f(x) local a=function(n)x=x+n return x end; local b=function()return x end;x=x+1;return a,b end;local a,b=f(6);local _,c=f(6);return a(1),b(),c()").eval().unwrap();
    assert_eq!(source, (8, 8, 7));
}

#[test]
fn nested_factory_forwards_the_same_parent_cell_and_preserves_nil_false() {
    let mut f = FactoryFixture::default();
    let middle = f.create(1, 2, vec![local(0), local(1)]);
    let child = f.create(2, 3, vec![inherited(0), inherited(1)]);
    let lib = f.library(vec![
        (0, vec![ret(vec![middle])]),
        (2, vec![ret(vec![child])]),
        (
            2,
            vec![store(0, vec![l(0)]), ret(vec![cap(0), cap(1), cap(0)])],
        ),
    ]);
    let (mut session, roots) = lib
        .session_from_input(&main_input(&lib), ProgramLimits::default())
        .unwrap();
    let args = scalars(
        &mut session,
        vec![ProgramValue::Number(1.0), ProgramValue::Boolean(false)],
    );
    let middle = session.invoke_callable(&roots[0], &args).unwrap();
    let a = session.invoke_callable(&middle[0], &[]).unwrap();
    let b = session.invoke_callable(&middle[0], &[]).unwrap();
    assert_eq!(
        get_results(&mut session, &a[0], &[]),
        vec![
            ProgramValue::Nil,
            ProgramValue::Boolean(false),
            ProgramValue::Nil
        ]
    );
    let four = scalars(&mut session, vec![ProgramValue::Number(4.0)]);
    assert_eq!(
        get_results(&mut session, &b[0], &four),
        vec![
            ProgramValue::Number(4.0),
            ProgramValue::Boolean(false),
            ProgramValue::Number(4.0)
        ]
    );
}

#[test]
fn recursive_local_captures_its_own_binding_before_assignment() {
    let mut f = FactoryFixture::default();
    let child = f.create(1, 2, vec![local(3)]);
    let lib = f.library(vec![
        (
            0,
            vec![
                declare(
                    3,
                    e(ParserProgramExprKind::Literal {
                        value: ParserFactoryLiteral::Nil,
                    }),
                ),
                assign(3, child),
                ret(vec![l(3)]),
            ],
        ),
        (
            1,
            vec![
                s(ParserProgramStatementKind::If {
                    branches: vec![ParserProgramBranch {
                        condition: binary(ParserProgramBinary::LessEqual, l(0), n(1.0)),
                        body: vec![ret(vec![n(1.0)])],
                    }],
                    otherwise: vec![],
                }),
                ret(vec![binary(
                    ParserProgramBinary::Multiply,
                    l(0),
                    invoke(
                        cap(0),
                        vec![binary(ParserProgramBinary::Subtract, l(0), n(1.0))],
                    ),
                )]),
            ],
        ),
    ]);
    let (mut session, roots) = lib
        .session_from_input(&main_input(&lib), ProgramLimits::default())
        .unwrap();
    let factorial = session.invoke_callable(&roots[0], &[]).unwrap();
    let five = scalars(&mut session, vec![ProgramValue::Number(5.0)]);
    assert_eq!(
        call_scalar(&mut session, &factorial[0], &five).unwrap(),
        ProgramValue::Number(120.0)
    );
}

#[test]
fn numeric_loop_bindings_and_body_declarations_have_fresh_generations() {
    let mut f = FactoryFixture::default();
    let child = f.create(1, 2, vec![local(3), local(4), local(1)]);
    let lib = f.library(vec![
        (
            0,
            vec![
                assign(1, n(0.0)),
                s(ParserProgramStatementKind::ForNumeric {
                    local: 3,
                    start: n(1.0),
                    limit: n(3.0),
                    step: n(1.0),
                    body: vec![
                        declare(4, l(3)),
                        set(l(0), l(3), child),
                        assign(3, add(l(3), n(10.0))),
                        assign(1, add(l(1), n(1.0))),
                    ],
                }),
                ret(vec![
                    get(l(0), n(1.0)),
                    get(l(0), n(2.0)),
                    get(l(0), n(3.0)),
                ]),
            ],
        ),
        (3, vec![ret(vec![cap(0), cap(1), cap(2)])]),
    ]);
    let artifact = input(
        &lib,
        &[(1, vec![])],
        vec![],
        ProgramValueGraph {
            values: vec![cl(1), tab(1)],
            tables: vec![ProgramTable { entries: vec![] }],
        },
    );
    let (mut session, roots) = lib
        .session_from_input(&artifact, ProgramLimits::default())
        .unwrap();
    let children = session.invoke_callable(&roots[0], &roots[1..]).unwrap();
    for (i, child) in children.iter().enumerate() {
        assert_eq!(
            get_results(&mut session, child, &[]),
            vec![
                ProgramValue::Number(i as f64 + 11.0),
                ProgramValue::Number(i as f64 + 1.0),
                ProgramValue::Number(3.0)
            ]
        );
    }
}

#[test]
fn generic_loop_visible_cells_do_not_replace_hidden_iterator_control() {
    let mut f = FactoryFixture::default();
    let child = f.create(1, 3, vec![local(3), local(4)]);
    let lib = f.library(vec![
        (
            0,
            vec![
                s(ParserProgramStatementKind::ForEach {
                    locals: vec![3, 4],
                    iterator: ParserProgramIterator::Generic {
                        values: pack(vec![l(1), n(0.0), n(0.0)]),
                    },
                    body: vec![
                        set(l(0), l(3), child),
                        assign(4, add(l(4), n(10.0))),
                        assign(3, n(99.0)),
                    ],
                }),
                ret(vec![
                    get(l(0), n(1.0)),
                    get(l(0), n(2.0)),
                    get(l(0), n(3.0)),
                ]),
            ],
        ),
        (
            0,
            vec![
                s(ParserProgramStatementKind::If {
                    branches: vec![ParserProgramBranch {
                        condition: binary(ParserProgramBinary::GreaterEqual, l(1), n(3.0)),
                        body: vec![ret(vec![])],
                    }],
                    otherwise: vec![],
                }),
                ret(vec![add(l(1), n(1.0)), add(l(1), n(4.0))]),
            ],
        ),
        (2, vec![ret(vec![cap(0), cap(1)])]),
    ]);
    let artifact = input(
        &lib,
        &[(1, vec![]), (2, vec![])],
        vec![],
        ProgramValueGraph {
            values: vec![cl(1), tab(1), cl(2)],
            tables: vec![ProgramTable { entries: vec![] }],
        },
    );
    let (mut session, roots) = lib
        .session_from_input(&artifact, ProgramLimits::default())
        .unwrap();
    let children = session.invoke_callable(&roots[0], &roots[1..]).unwrap();
    assert_eq!(children.len(), 3);
    for (i, child) in children.iter().enumerate() {
        assert_eq!(
            get_results(&mut session, child, &[]),
            vec![
                ProgramValue::Number(99.0),
                ProgramValue::Number(i as f64 + 14.0)
            ]
        );
    }
}

#[test]
fn closures_escaped_before_a_source_failure_keep_writable_cells() {
    let mut f = FactoryFixture::default();
    let child = f.create(1, 2, vec![local(1)]);
    let lib = f.library(vec![
        (
            0,
            vec![
                assign(1, n(7.0)),
                set(l(0), b("escaped"), child),
                assign(1, n(9.0)),
                run(call(n(0.0), vec![])),
            ],
        ),
        (1, counter()),
        (0, vec![ret(vec![get(l(0), b("escaped"))])]),
    ]);
    let artifact = input(
        &lib,
        &[(1, vec![]), (3, vec![])],
        vec![],
        ProgramValueGraph {
            values: vec![cl(1), cl(2), tab(1)],
            tables: vec![ProgramTable { entries: vec![] }],
        },
    );
    let (mut session, roots) = lib
        .session_from_input(&artifact, ProgramLimits::default())
        .unwrap();
    error(
        session.invoke_callable(&roots[0], &roots[2..]),
        ProgramRuntimeErrorKind::Source,
    );
    let escaped = session.invoke_callable(&roots[1], &roots[2..]).unwrap();
    let two = scalars(&mut session, vec![ProgramValue::Number(2.0)]);
    assert_eq!(
        call_scalar(&mut session, &escaped[0], &two).unwrap(),
        ProgramValue::Number(11.0)
    );
}

fn source_binary(
    operation: ParserProgramBinary,
    left: ParserProgramAssignmentOperand,
    right: ParserProgramExpr,
) -> ParserProgramExpr {
    e(ParserProgramExprKind::SourceBinary {
        operation,
        left: Box::new(left),
        right: Box::new(right),
    })
}
#[test]
fn source_binary_reads_promoted_local_after_rhs_but_computed_values_and_packs_are_eager() {
    for eager in [false, true] {
        let mut f = FactoryFixture::default();
        let child = f.create(1, 2, vec![local(0)]);
        let left = if eager {
            ParserProgramAssignmentOperand::Evaluated {
                value: add(l(0), n(0.0)),
            }
        } else {
            ParserProgramAssignmentOperand::LocalRegister { local: 0 }
        };
        let lib = f.library(vec![
            (
                0,
                vec![
                    declare(3, child),
                    ret(vec![
                        source_binary(ParserProgramBinary::Add, left, invoke(l(3), vec![])),
                        l(0),
                    ]),
                ],
            ),
            (1, vec![store(0, vec![n(10.0)]), ret(vec![n(2.0)])]),
        ]);
        let (mut session, roots) = lib
            .session_from_input(&main_input(&lib), ProgramLimits::default())
            .unwrap();
        let two = scalars(&mut session, vec![ProgramValue::Number(2.0)]);
        assert_eq!(
            get_results(&mut session, &roots[0], &two),
            vec![
                ProgramValue::Number(if eager { 4.0 } else { 12.0 }),
                ProgramValue::Number(10.0)
            ]
        );
    }
    let mut f = FactoryFixture::default();
    let child = f.create(1, 2, vec![local(0)]);
    let lib = f.library(vec![
        (
            0,
            vec![
                declare(3, child),
                ret(vec![l(0), invoke(l(3), vec![]), l(0)]),
            ],
        ),
        (1, vec![store(0, vec![n(10.0)]), ret(vec![n(2.0)])]),
    ]);
    let (mut session, roots) = lib
        .session_from_input(&main_input(&lib), ProgramLimits::default())
        .unwrap();
    let two = scalars(&mut session, vec![ProgramValue::Number(2.0)]);
    assert_eq!(
        get_results(&mut session, &roots[0], &two),
        vec![
            ProgramValue::Number(2.0),
            ProgramValue::Number(2.0),
            ProgramValue::Number(10.0)
        ]
    );
}

#[test]
fn mixed_assignment_hazards_copy_cells_only_at_conflicting_lhs_targets() {
    use ParserProgramAssignmentOperand as O;
    use ParserProgramAssignmentTargetKind as T;
    for freeze_table in [false, true] {
        for freeze_key in [false, true] {
            let mut f = FactoryFixture::default();
            let child = f.create(1, 2, vec![local(0), local(1), local(2)]);
            let target = |operation| ParserProgramAssignmentTarget {
                location: ParserProgramLocation { start: 0, end: 1 },
                operation,
            };
            let mut targets = vec![target(T::Indexed {
                table: O::LocalRegister { local: 0 },
                key: O::LocalRegister { local: 1 },
            })];
            let mut rhs = vec![invoke(l(3), vec![])];
            if freeze_table {
                targets.push(target(T::Local { local: 0 }));
                rhs.push(l(2));
            }
            if freeze_key {
                targets.push(target(T::Local { local: 1 }));
                rhs.push(n(1.0));
            }
            let lib = f.library(vec![
                (
                    0,
                    vec![
                        declare(3, child),
                        declare(4, l(0)),
                        s(ParserProgramStatementKind::MixedAssign {
                            targets,
                            values: pack(rhs),
                        }),
                        ret(vec![
                            get(l(4), n(1.0)),
                            get(l(4), n(2.0)),
                            get(l(2), n(1.0)),
                            get(l(2), n(2.0)),
                        ]),
                    ],
                ),
                (
                    3,
                    vec![
                        store(0, vec![cap(2)]),
                        store(1, vec![n(2.0)]),
                        ret(vec![n(99.0)]),
                    ],
                ),
            ]);
            let artifact = input(
                &lib,
                &[(1, vec![])],
                vec![],
                ProgramValueGraph {
                    values: vec![cl(1), tab(1), ProgramValue::Number(1.0), tab(2)],
                    tables: vec![
                        ProgramTable { entries: vec![] },
                        ProgramTable { entries: vec![] },
                    ],
                },
            );
            let (mut session, roots) = lib
                .session_from_input(&artifact, ProgramLimits::default())
                .unwrap();
            let mut expected = vec![ProgramValue::Nil; 4];
            expected[if freeze_table { 0 } else { 2 } + if freeze_key { 0 } else { 1 }] =
                ProgramValue::Number(99.0);
            assert_eq!(
                get_results(&mut session, &roots[0], &roots[1..]),
                expected,
                "table={freeze_table}, key={freeze_key}"
            );
        }
    }
}

#[test]
fn indexed_read_live_base_follows_mutation_during_key_evaluation() {
    let mut f = FactoryFixture::default();
    let child = f.create(1, 2, vec![local(0), local(1)]);
    let lib = f.library(vec![
        (
            0,
            vec![
                declare(3, child),
                ret(vec![e(ParserProgramExprKind::IndexedRead {
                    table: Box::new(ParserProgramAssignmentOperand::LocalRegister { local: 0 }),
                    key: Box::new(invoke(l(3), vec![])),
                })]),
            ],
        ),
        (2, vec![store(0, vec![cap(1)]), ret(vec![b("value")])]),
    ]);
    let artifact = input(
        &lib,
        &[(1, vec![])],
        vec![],
        ProgramValueGraph {
            values: vec![cl(1), tab(1), tab(2)],
            tables: vec![
                ProgramTable {
                    entries: vec![(text("value"), ProgramValue::Number(11.0))],
                },
                ProgramTable {
                    entries: vec![(text("value"), ProgramValue::Number(22.0))],
                },
            ],
        },
    );
    let (mut session, roots) = lib
        .session_from_input(&artifact, ProgramLimits::default())
        .unwrap();
    assert_eq!(
        get_results(&mut session, &roots[0], &roots[1..]),
        vec![ProgramValue::Number(22.0)]
    );
}

#[test]
fn zero_capture_factories_have_fresh_identity_and_function_keys() {
    let mut f = FactoryFixture::default();
    let a = f.create(1, 2, vec![]);
    let b = f.create(1, 2, vec![]);
    let lib = f.library(vec![
        (
            0,
            vec![
                declare(3, a),
                declare(4, b),
                set(l(0), l(3), n(7.0)),
                set(l(0), l(4), n(11.0)),
                ret(vec![
                    binary(ParserProgramBinary::Equal, l(3), l(4)),
                    get(l(0), l(3)),
                    get(l(0), l(4)),
                    l(3),
                ]),
            ],
        ),
        (0, vec![ret(vec![n(1.0)])]),
    ]);
    let artifact = input(
        &lib,
        &[(1, vec![])],
        vec![],
        ProgramValueGraph {
            values: vec![cl(1), tab(1)],
            tables: vec![ProgramTable { entries: vec![] }],
        },
    );
    let (mut session, roots) = lib
        .session_from_input(&artifact, ProgramLimits::default())
        .unwrap();
    let values = session.invoke_callable(&roots[0], &roots[1..]).unwrap();
    assert_eq!(
        session.snapshot(&values[..3]).unwrap().graph().values,
        vec![
            ProgramValue::Boolean(false),
            ProgramValue::Number(7.0),
            ProgramValue::Number(11.0)
        ]
    );
    error(session.snapshot(&values[3..]), UNKNOWN);
    let (mut other, other_roots) = lib
        .session_from_input(&artifact, ProgramLimits::default())
        .unwrap();
    error(
        other.invoke_callable(&values[3], &[]),
        ProgramRuntimeErrorKind::InvalidInput,
    );
    assert!(
        other
            .invoke_callable(&other_roots[0], &other_roots[1..])
            .is_ok()
    );
}

#[test]
fn break_and_return_keep_current_generation_without_rebinding_escaped_cells() {
    for early_return in [false, true] {
        let mut f = FactoryFixture::default();
        let child = f.create(1, 2, vec![local(3), local(1)]);
        let exit = if early_return {
            ret(vec![get(l(0), n(2.0))])
        } else {
            s(ParserProgramStatementKind::Break)
        };
        let lib = f.library(vec![
            (
                0,
                vec![
                    assign(1, n(4.0)),
                    s(ParserProgramStatementKind::ForNumeric {
                        local: 3,
                        start: n(1.0),
                        limit: n(3.0),
                        step: n(1.0),
                        body: vec![
                            set(l(0), l(3), child),
                            s(ParserProgramStatementKind::If {
                                branches: vec![ParserProgramBranch {
                                    condition: binary(ParserProgramBinary::Equal, l(3), n(2.0)),
                                    body: vec![exit],
                                }],
                                otherwise: vec![],
                            }),
                        ],
                    }),
                    assign(1, n(10.0)),
                    ret(vec![get(l(0), n(2.0))]),
                ],
            ),
            (2, vec![ret(vec![cap(0), cap(1)])]),
        ]);
        let artifact = input(
            &lib,
            &[(1, vec![])],
            vec![],
            ProgramValueGraph {
                values: vec![cl(1), tab(1)],
                tables: vec![ProgramTable { entries: vec![] }],
            },
        );
        let (mut session, roots) = lib
            .session_from_input(&artifact, ProgramLimits::default())
            .unwrap();
        let escaped = session.invoke_callable(&roots[0], &roots[1..]).unwrap();
        assert_eq!(
            get_results(&mut session, &escaped[0], &[]),
            vec![
                ProgramValue::Number(2.0),
                ProgramValue::Number(if early_return { 4.0 } else { 10.0 })
            ]
        );
    }
}

#[test]
fn one_compiled_factory_library_runs_on_independent_parallel_session_cells() {
    let mut f = FactoryFixture::default();
    let child = f.create(1, 2, vec![local(0)]);
    let lib = f.library(vec![(0, vec![ret(vec![child])]), (1, counter())]);
    let artifact = main_input(&lib);
    std::thread::scope(|scope| {
        let workers: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    let (mut session, roots) = lib
                        .session_from_input(&artifact, ProgramLimits::default())
                        .unwrap();
                    let zero = scalars(&mut session, vec![ProgramValue::Number(0.0)]);
                    let one = scalars(&mut session, vec![ProgramValue::Number(1.0)]);
                    let child = session.invoke_callable(&roots[0], &zero).unwrap();
                    for i in 1..=32 {
                        assert_eq!(
                            call_scalar(&mut session, &child[0], &one).unwrap(),
                            ProgramValue::Number(f64::from(i))
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

#[test]
fn unsupported_creation_profile_remains_a_reached_frontier() {
    let mut f = FactoryFixture::default();
    let child = f.create(1, 2, vec![]);
    let catalog = f.catalog(vec![(0, vec![ret(vec![child])]), (0, vec![ret(vec![])])]);
    let mut claims = catalog.closure_creations().unwrap().clone();
    claims.profile.gc64 = false;
    let catalog = SourceProgramCatalog::new_with_closure_creations(
        catalog.data().clone(),
        catalog.owner().clone(),
        None,
        claims,
    )
    .unwrap();
    let lib = CompiledSourcePrograms::new(&catalog).unwrap();
    let (mut session, roots) = lib
        .session_from_input(&main_input(&lib), ProgramLimits::default())
        .unwrap();
    error(session.invoke_callable(&roots[0], &[]), UNKNOWN);
}

#[test]
fn created_rhs_rebinding_does_not_change_already_resolved_call_target() {
    let mut f = FactoryFixture::default();
    let replace = f.create(1, 4, vec![local(0), local(1)]);
    let lib = f.library(vec![
        (
            0,
            vec![
                declare(3, replace),
                ret(vec![
                    invoke(l(0), vec![invoke(l(3), vec![])]),
                    invoke(l(0), vec![]),
                ]),
            ],
        ),
        (0, vec![ret(vec![n(1.0)])]),
        (0, vec![ret(vec![n(2.0)])]),
        (2, vec![store(0, vec![cap(1)]), ret(vec![n(0.0)])]),
    ]);
    let artifact = input(
        &lib,
        &[(1, vec![]), (2, vec![]), (3, vec![])],
        vec![],
        graph(vec![cl(1), cl(2), cl(3)]),
    );
    let (mut session, roots) = lib
        .session_from_input(&artifact, ProgramLimits::default())
        .unwrap();
    assert_eq!(
        get_results(&mut session, &roots[0], &roots[1..]),
        vec![ProgramValue::Number(1.0), ProgramValue::Number(2.0)]
    );
}

#[test]
fn promoted_capture_retains_borrowed_table_readonly_behavior_and_owned_aliases() {
    let mut f = FactoryFixture::default();
    let setter = f.create(1, 2, vec![local(0)]);
    let lib = f.library(vec![
        (0, vec![ret(vec![setter])]),
        (1, vec![set(cap(0), b("value"), n(9.0)), ret(vec![cap(0)])]),
    ]);
    let (mut session, roots) = lib
        .session_from_input(&main_input(&lib), ProgramLimits::default())
        .unwrap();
    let table = ProgramValueGraph {
        values: vec![tab(1)],
        tables: vec![ProgramTable {
            entries: vec![(text("value"), ProgramValue::Number(3.0))],
        }],
    };
    let borrowed = session.borrow(&table).unwrap();
    let setter = session.invoke_callable(&roots[0], &borrowed).unwrap();
    error(session.invoke_callable(&setter[0], &[]), UNKNOWN);
    assert_eq!(session.snapshot(&borrowed).unwrap().graph(), &table);
    let owned = session
        .import_with_coverage(&table, &ProgramTableCoverage::new())
        .unwrap();
    let setter = session.invoke_callable(&roots[0], &owned).unwrap();
    let returned = session.invoke_callable(&setter[0], &[]).unwrap();
    let projection = session
        .snapshot(&[owned[0].clone(), returned[0].clone()])
        .unwrap();
    assert_eq!(projection.graph().values, vec![tab(1), tab(1)]);
    assert_eq!(
        projection.graph().tables[0].entries,
        vec![(text("value"), ProgramValue::Number(9.0))]
    );
}
