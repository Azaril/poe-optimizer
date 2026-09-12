use super::*;

fn location() -> ParserProgramLocation {
    ParserProgramLocation { start: 0, end: 1 }
}
fn literal(value: ParserFactoryLiteral) -> ParserProgramExpr {
    ParserProgramExpr {
        location: location(),
        operation: ParserProgramExprKind::Literal { value },
    }
}
fn statement(operation: ParserProgramStatementKind) -> ParserProgramStatement {
    ParserProgramStatement {
        location: location(),
        operation,
    }
}
fn returned(number: f64) -> ParserProgramStatement {
    statement(ParserProgramStatementKind::Return {
        values: ParserProgramValueList {
            values: vec![literal(ParserFactoryLiteral::Number(number))],
            tail: None,
        },
    })
}
fn lower(body: &[ParserProgramStatement]) -> Vec<ProgramInstruction> {
    let mut remaining = 100;
    let mut lowerer = Lowerer {
        code: Vec::new(),
        loop_states: 0,
        breaks: Vec::new(),
        remaining: &mut remaining,
    };
    lowerer.block(body).unwrap();
    lowerer
        .emit(location(), ProgramOperation::Fallthrough)
        .unwrap();
    lowerer.verify_targets().unwrap();
    lowerer.code
}
fn numeric(local: u16, body: Vec<ParserProgramStatement>) -> ParserProgramStatement {
    statement(ParserProgramStatementKind::ForNumeric {
        local,
        start: literal(ParserFactoryLiteral::Number(1.0)),
        limit: literal(ParserFactoryLiteral::Number(2.0)),
        step: literal(ParserFactoryLiteral::Number(1.0)),
        body,
    })
}

#[test]
fn elseif_false_paths_select_next_condition_and_success_skips_remaining_branches() {
    let code = lower(&[
        statement(ParserProgramStatementKind::If {
            branches: vec![
                ParserProgramBranch {
                    condition: literal(ParserFactoryLiteral::Boolean(false)),
                    body: vec![returned(10.0)],
                },
                ParserProgramBranch {
                    condition: literal(ParserFactoryLiteral::Boolean(true)),
                    body: vec![returned(20.0)],
                },
            ],
            otherwise: vec![returned(30.0)],
        }),
        returned(40.0),
    ]);
    assert!(matches!(
        code[0].operation,
        ProgramOperation::JumpIfFalse { target: 3, .. }
    ));
    assert!(matches!(
        code[2].operation,
        ProgramOperation::Jump { target: 7 }
    ));
    assert!(matches!(
        code[3].operation,
        ProgramOperation::JumpIfFalse { target: 6, .. }
    ));
    assert!(matches!(
        code[5].operation,
        ProgramOperation::Jump { target: 7 }
    ));
    assert!(matches!(code[6].operation, ProgramOperation::Return { .. }));
    assert!(matches!(code[7].operation, ProgramOperation::Return { .. }));
    assert!(matches!(code[8].operation, ProgramOperation::Fallthrough));
}

#[test]
fn nested_breaks_exit_only_their_own_loop() {
    let code = lower(&[
        numeric(
            0,
            vec![
                numeric(1, vec![statement(ParserProgramStatementKind::Break)]),
                statement(ParserProgramStatementKind::Break),
            ],
        ),
        returned(7.0),
    ]);
    assert!(matches!(
        code[0].operation,
        ProgramOperation::NumericInit {
            state: 0,
            exit: 6,
            ..
        }
    ));
    assert!(matches!(
        code[1].operation,
        ProgramOperation::NumericInit {
            state: 1,
            exit: 4,
            ..
        }
    ));
    assert!(matches!(
        code[2].operation,
        ProgramOperation::Jump { target: 4 }
    ));
    assert!(matches!(
        code[3].operation,
        ProgramOperation::NumericNext {
            state: 1,
            body: 2,
            exit: 4,
            ..
        }
    ));
    assert!(matches!(
        code[4].operation,
        ProgramOperation::Jump { target: 6 }
    ));
    assert!(matches!(
        code[5].operation,
        ProgramOperation::NumericNext {
            state: 0,
            body: 1,
            exit: 6,
            ..
        }
    ));
}

#[test]
fn empty_loop_has_a_real_charged_backedge_and_valid_zero_iteration_exit() {
    let code = lower(&[numeric(0, vec![])]);
    assert!(matches!(
        code[0].operation,
        ProgramOperation::NumericInit { exit: 2, .. }
    ));
    assert!(matches!(
        code[1].operation,
        ProgramOperation::NumericNext {
            body: 1,
            exit: 2,
            ..
        }
    ));
    assert!(matches!(code[2].operation, ProgramOperation::Fallthrough));
}

#[test]
fn iterator_state_and_nested_conditional_break_are_preserved() {
    let code = lower(&[statement(ParserProgramStatementKind::ForEach {
        locals: vec![0, 1],
        iterator: ParserProgramIterator::Dense {
            table: literal(ParserFactoryLiteral::Nil),
            binding: 0,
        },
        body: vec![statement(ParserProgramStatementKind::If {
            branches: vec![ParserProgramBranch {
                condition: literal(ParserFactoryLiteral::Boolean(true)),
                body: vec![statement(ParserProgramStatementKind::Break)],
            }],
            otherwise: vec![],
        })],
    })]);
    assert!(matches!(
        code[0].operation,
        ProgramOperation::IteratorInit {
            state: 0,
            exit: 5,
            ..
        }
    ));
    assert!(matches!(
        code[2].operation,
        ProgramOperation::Jump { target: 5 }
    ));
    assert!(matches!(
        code[4].operation,
        ProgramOperation::IteratorNext {
            state: 0,
            body: 1,
            exit: 5,
            ..
        }
    ));
}

#[test]
fn compiler_never_evaluates_unselected_source_errors_or_collapses_return_arity() {
    let erroneous = ParserProgramExpr {
        location: location(),
        operation: ParserProgramExprKind::Unary {
            operation: ParserProgramUnary::Negate,
            value: Box::new(literal(ParserFactoryLiteral::Boolean(false))),
        },
    };
    let values = ParserProgramValueList {
        values: vec![erroneous],
        tail: None,
    };
    let code = lower(&[statement(ParserProgramStatementKind::If {
        branches: vec![ParserProgramBranch {
            condition: literal(ParserFactoryLiteral::Boolean(false)),
            body: vec![statement(ParserProgramStatementKind::Return {
                values: values.clone(),
            })],
        }],
        otherwise: vec![statement(ParserProgramStatementKind::Return {
            values: ParserProgramValueList {
                values: vec![literal(ParserFactoryLiteral::Nil)],
                tail: None,
            },
        })],
    })]);
    assert_eq!(code[1].operation, ProgramOperation::Return { values });
    assert!(matches!(
        code.last().unwrap().operation,
        ProgramOperation::Fallthrough
    ));
    let ProgramOperation::Return { values } = &code[3].operation else {
        panic!("explicit Nil return")
    };
    assert_eq!(values.values.len(), 1);
}

#[test]
fn instruction_budget_includes_dead_branches_and_generated_jumps() {
    let mut remaining = 3;
    let mut lowerer = Lowerer {
        code: Vec::new(),
        loop_states: 0,
        breaks: Vec::new(),
        remaining: &mut remaining,
    };
    let body = [statement(ParserProgramStatementKind::If {
        branches: vec![ParserProgramBranch {
            condition: literal(ParserFactoryLiteral::Boolean(true)),
            body: vec![returned(1.0)],
        }],
        otherwise: vec![returned(2.0)],
    })];
    assert_eq!(
        lowerer.block(&body),
        Err(ProgramCompileError::ResourceBound("instructions"))
    );
    assert_eq!(lowerer.code.len(), 3);
}

#[test]
fn invalid_break_and_unpatched_targets_cannot_publish_a_plan() {
    let mut remaining = 10;
    let mut lowerer = Lowerer {
        code: Vec::new(),
        loop_states: 0,
        breaks: Vec::new(),
        remaining: &mut remaining,
    };
    assert_eq!(
        lowerer.block(&[statement(ParserProgramStatementKind::Break)]),
        Err(ProgramCompileError::InvalidControlFlow)
    );
    lowerer
        .emit(location(), ProgramOperation::Jump { target: usize::MAX })
        .unwrap();
    assert_eq!(
        lowerer.verify_targets(),
        Err(ProgramCompileError::InvalidControlFlow)
    );
}

/// Synthetic programs exercise the public compiler boundary. Their callback
/// descriptors come from a cloned package; these tests do not prove Lua parity.
pub(super) fn program_fixture(count: usize) -> (ModifierParserCatalog, ParserProgramData) {
    use poe_optimizer_data::game_data::bundled_snapshot;
    use std::sync::OnceLock;
    static BASE: OnceLock<ModifierParserData> = OnceLock::new();
    let mut owner = BASE
        .get_or_init(|| bundled_snapshot().unwrap().modifier_parser().data().clone())
        .clone();
    // This compiler fixture supplies its own separate programs.
    owner.programs = ParserProgramPayload::default();
    let originals: Vec<_> = owner
        .factories
        .iter()
        .filter_map(|(&id, disposition)| {
            if let ParserFactoryDisposition::Pure(factory) = disposition {
                Some((id, factory.provenance.clone()))
            } else {
                None
            }
        })
        .take(count)
        .collect();
    let mut programs = Vec::new();
    let mut callbacks = BTreeMap::new();
    for (index, (callback, original)) in originals.into_iter().enumerate() {
        owner.factories.insert(
            callback,
            ParserFactoryDisposition::Unsupported {
                reason: "synthetic program compiler fixture".into(),
            },
        );
        programs.push(ParserProgram {
            callback,
            parameter_count: 2,
            local_count: 8,
            variadic: false,
            bindings: vec![],
            body: vec![],
            provenance: ParserProgramProvenance {
                source: original.source,
                function_start: original.function_start,
                function_end: original.function_end,
                function_sha256: original.function_sha256,
            },
        });
        callbacks.insert(callback, ParserProgramId(index as u32 + 1));
    }
    (
        ModifierParserCatalog::new(owner).unwrap(),
        ParserProgramData {
            schema_version: PARSER_PROGRAM_SCHEMA_VERSION,
            programs,
            callbacks,
        },
    )
}

#[test]
fn public_compiler_retains_catalog_identity_parameters_and_injected_values() {
    let (owner, mut data) = program_fixture(1);
    let callback = data.programs[0].callback;
    data.programs[0].body = vec![returned(17.0)];
    let catalog = ParserProgramCatalog::new(data.clone(), owner.clone()).unwrap();
    let compiled = CompiledParserPrograms::new(&catalog).unwrap();
    assert!(compiled.catalog().is_bound_to(&owner));
    let plan = compiled.program(callback).unwrap();
    assert_eq!(plan.parameter_count(), 2);
    assert_eq!(plan.local_count(), 8);
    assert!(!plan.variadic());
    assert_eq!(plan.instructions().len(), 2);
    let ProgramOperation::Return { values } = &plan.instructions()[0].operation else {
        panic!("return")
    };
    assert_eq!(
        values.values[0],
        literal(ParserFactoryLiteral::Number(17.0))
    );
    assert!(compiled.program(ParserCallbackId(0)).is_none());

    // Same numeric IDs and source descriptors cannot retarget the retained owner.
    let other_owner = ModifierParserCatalog::new(owner.data().clone()).unwrap();
    data.programs[0].body = vec![returned(31.0)];
    let other =
        CompiledParserPrograms::new(&ParserProgramCatalog::new(data, other_owner.clone()).unwrap())
            .unwrap();
    assert!(!compiled.catalog().is_bound_to(&other_owner));
    assert!(other.catalog().is_bound_to(&other_owner));
    std::thread::scope(|scope| {
        for (library, expected) in [(compiled.clone(), 17.0), (other, 31.0)] {
            scope.spawn(move || {
                for _ in 0..16 {
                    let ProgramOperation::Return { values } =
                        &library.program(callback).unwrap().instructions()[0].operation
                    else {
                        panic!("return")
                    };
                    assert_eq!(
                        values.values[0],
                        literal(ParserFactoryLiteral::Number(expected))
                    );
                }
            });
        }
    });
}

#[test]
fn source_bound_recursive_helpers_compile_without_inlining_or_losing_owners() {
    let (owner, mut data) = program_fixture(2);
    let mut definitions = owner.data().clone();
    let callbacks: Vec<_> = data.programs.iter().map(|p| p.callback).collect();
    for (index, program) in data.programs.iter_mut().enumerate() {
        let helper = callbacks[1 - index];
        let closure = &mut definitions.callbacks[program.callback.0 as usize - 1];
        let upvalue = closure.upvalues.len() as u16;
        closure.upvalues.push(ParserUpvalue {
            name: "synthetic_helper".into(),
            value: ParserValue::Callback(helper),
        });
        program
            .bindings
            .push(ParserProgramBinding::CapturedCallback {
                upvalue,
                callback: helper,
            });
        program
            .body
            .push(statement(ParserProgramStatementKind::Call {
                call: ParserProgramCall {
                    binding: 0,
                    receiver: None,
                    arguments: ParserProgramValueList::default(),
                },
            }));
    }
    let owner = ModifierParserCatalog::new(definitions).unwrap();
    let catalog = ParserProgramCatalog::new(data, owner).unwrap();
    assert!(
        catalog
            .required_capabilities()
            .contains(&ParserProgramCapability::RecursiveCalls)
    );
    let library = CompiledParserPrograms::new(&catalog).unwrap();
    assert_eq!(library.instruction_count(), 4);
    for (index, callback) in callbacks.iter().enumerate() {
        let program = library.program(*callback).unwrap();
        assert_eq!(
            program.bindings(),
            &[CompiledProgramBinding::Program {
                index: 1 - index,
                callback: callbacks[1 - index]
            }]
        );
    }
}

#[test]
fn compiled_traversal_indices_share_positions_without_per_session_key_or_table_copies() {
    use poe_optimizer_data::source_program::*;
    let (parser, mut data) = program_fixture(1);
    let mut callback = parser.data().callbacks[data.programs[0].callback.0 as usize - 1].clone();
    callback.upvalues.clear();
    data.programs.truncate(1);
    data.programs[0].callback = ParserCallbackId(1);
    data.programs[0].bindings.clear();
    data.programs[0].body = vec![returned(1.0)];
    data.callbacks = BTreeMap::from([(ParserCallbackId(1), ParserProgramId(1))]);
    let order = vec![
        SourceTableKey::Text("z".into()),
        SourceTableKey::Integer(2),
        SourceTableKey::Text("a".into()),
        SourceTableKey::Integer(1),
    ];
    let owner = SourceProgramOwner::new_with_context(
        SourceProgramDefinitions {
            schema_version: SOURCE_PROGRAM_DEFINITIONS_SCHEMA_VERSION,
            source: parser.data().source.clone(),
            tables: vec![SourceTable {
                fields: BTreeMap::from([
                    ("z".into(), SourceValue::Number(1.0)),
                    ("a".into(), SourceValue::Number(2.0)),
                ]),
                indexed: BTreeMap::from([
                    (1, SourceValue::Number(3.0)),
                    (2, SourceValue::Number(4.0)),
                ]),
            }],
            callbacks: vec![callback],
            roots: vec![],
            intrinsics: BTreeMap::new(),
        },
        None,
        SourceProgramContext {
            iteration: Some(SourceProgramIteration {
                ipairs_aux: BTreeMap::new(),
                table_order: BTreeMap::from([(SourceTableId(1), order)]),
                pairs_next: BTreeMap::new(),
            }),
            ..SourceProgramContext::default()
        },
    )
    .unwrap();
    let plan =
        CompiledSourcePrograms::new(&SourceProgramCatalog::new(data, owner).unwrap()).unwrap();
    let cloned = plan.clone();
    assert!(Arc::ptr_eq(&plan.0, &cloned.0));
    let index = &plan.0.traversal[&ParserTableId(1)];
    assert_eq!(index.text.as_ref(), &[2, 0]);
    assert_eq!(index.integers.as_ref(), &[3, 1]);
    assert_eq!(
        index.text.as_ptr(),
        cloned.0.traversal[&ParserTableId(1)].text.as_ptr()
    );
    for lib in [plan, cloned] {
        let (session, _) = lib
            .session(&ProgramValueGraph::default(), ProgramLimits::default())
            .unwrap();
        assert_eq!(session.allocations().tables, 0);
        assert_eq!(session.allocations().bytes, 0);
        assert_eq!(session.allocations().values, 0);
    }
}
