//! Structural verification is not execution or proof of source lowering.
use poe_optimizer_data::{game_data::bundled_snapshot, modifier_parser::*};
use std::collections::BTreeMap;

fn location() -> ParserProgramLocation {
    ParserProgramLocation { start: 0, end: 1 }
}
fn expr(operation: ParserProgramExprKind) -> ParserProgramExpr {
    ParserProgramExpr {
        location: location(),
        operation,
    }
}
fn literal(value: ParserFactoryLiteral) -> ParserProgramExpr {
    expr(ParserProgramExprKind::Literal { value })
}
fn nil() -> ParserProgramExpr {
    literal(ParserFactoryLiteral::Nil)
}
fn local(local: u16) -> ParserProgramExpr {
    expr(ParserProgramExprKind::Local { local })
}
fn number(value: f64) -> ParserProgramExpr {
    literal(ParserFactoryLiteral::Number(value))
}
fn values(values: Vec<ParserProgramExpr>) -> ParserProgramValueList {
    ParserProgramValueList { values, tail: None }
}
fn statement(operation: ParserProgramStatementKind) -> ParserProgramStatement {
    ParserProgramStatement {
        location: location(),
        operation,
    }
}
fn returning(value: ParserProgramExpr) -> ParserProgramStatement {
    statement(ParserProgramStatementKind::Return {
        values: values(vec![value]),
    })
}
fn declare(slot: u16, value: Option<ParserProgramExpr>) -> ParserProgramStatement {
    statement(ParserProgramStatementKind::Declare {
        locals: vec![slot],
        values: values(value.into_iter().collect()),
    })
}
fn dead_branch(body: Vec<ParserProgramStatement>) -> ParserProgramStatement {
    statement(ParserProgramStatementKind::If {
        branches: vec![ParserProgramBranch {
            condition: literal(ParserFactoryLiteral::Boolean(false)),
            body,
        }],
        otherwise: vec![],
    })
}
fn numeric_loop(body: Vec<ParserProgramStatement>) -> ParserProgramStatement {
    statement(ParserProgramStatementKind::ForNumeric {
        local: 0,
        start: number(1.0),
        limit: number(3.0),
        step: number(1.0),
        body,
    })
}

fn fixture() -> (ModifierParserCatalog, ParserProgramData) {
    let owner = bundled_snapshot().unwrap().modifier_parser().clone();
    let callback = owner
        .data()
        .factories
        .iter()
        .find_map(|(id, disposition)| {
            let descriptor = owner.callback(*id)?;
            (matches!(disposition, ParserFactoryDisposition::Unsupported { .. })
                && matches!(descriptor.kind, ParserCallbackKind::Lua { .. })
                && !descriptor
                    .upvalues
                    .iter()
                    .any(|u| matches!(u.name.as_str(), "string" | "tonumber" | "table" | "ipairs")))
            .then_some(*id)
        })
        .expect("a source-owned Unsupported callback without shadowed test intrinsics");
    let ParserCallbackKind::Lua { source } = &owner.callback(callback).unwrap().kind else {
        unreachable!()
    };
    // This is authored test data linked to a real source owner. G1 checks linkage
    // and well-formed provenance; it does not certify these instructions as a
    // lowering of the original function. G3 supplies that independent proof.
    let provenance = ParserProgramProvenance {
        source: source.clone(),
        function_start: 0,
        function_end: 1,
        function_sha256: source.sha256.clone(),
    };
    let data = ParserProgramData {
        schema_version: PARSER_PROGRAM_SCHEMA_VERSION,
        callbacks: BTreeMap::from([(callback, ParserProgramId(1))]),
        programs: vec![ParserProgram {
            callback,
            parameter_count: 0,
            variadic: false,
            local_count: 4,
            bindings: vec![],
            body: vec![returning(nil())],
            provenance,
        }],
    };
    (owner, data)
}

fn rejects_structure(data: ParserProgramData, owner: ModifierParserCatalog) {
    let error = ParserProgramCatalog::new(data, owner).unwrap_err();
    assert!(matches!(
        error.kind,
        ParserProgramErrorKind::InvalidData | ParserProgramErrorKind::Binding
    ));
}

#[test]
fn unselected_branches_still_validate_references_and_control_scope() {
    let (owner, baseline) = fixture();
    let bad_call = ParserProgramCall {
        binding: 0,
        receiver: None,
        arguments: values(vec![]),
    };
    for hidden in [
        returning(local(u16::MAX)),
        statement(ParserProgramStatementKind::Call { call: bad_call }),
        statement(ParserProgramStatementKind::Break),
    ] {
        let mut data = baseline.clone();
        data.programs[0].body = vec![dead_branch(vec![hidden]), returning(nil())];
        rejects_structure(data, owner.clone());
    }
    let mut data = baseline;
    data.programs[0].body = vec![returning(expr(ParserProgramExprKind::Binary {
        operation: ParserProgramBinary::And,
        left: Box::new(literal(ParserFactoryLiteral::Boolean(false))),
        right: Box::new(local(u16::MAX)),
    }))];
    rejects_structure(data, owner);
}

#[test]
fn selected_runtime_type_and_pattern_errors_are_not_speculative_data_errors() {
    let (owner, mut data) = fixture();
    data.programs[0]
        .bindings
        .push(ParserProgramBinding::Intrinsic {
            operation: ParserProgramIntrinsic::StringGsub,
            source: ParserProgramIntrinsicSource::OriginalGlobal,
        });
    let bad_index = expr(ParserProgramExprKind::Get {
        table: Box::new(literal(ParserFactoryLiteral::Boolean(false))),
        key: Box::new(nil()),
    });
    let bad_pattern = expr(ParserProgramExprKind::Call {
        call: Box::new(ParserProgramCall {
            binding: 0,
            receiver: None,
            arguments: values(vec![
                literal(ParserFactoryLiteral::Text("subject".into())),
                literal(ParserFactoryLiteral::Text("[".into())),
                literal(ParserFactoryLiteral::Text("replacement".into())),
            ]),
        }),
    });
    data.programs[0].body = vec![
        dead_branch(vec![returning(bad_pattern)]),
        returning(expr(ParserProgramExprKind::Binary {
            operation: ParserProgramBinary::Or,
            left: Box::new(number(0.0)),
            right: Box::new(bad_index),
        })),
    ];
    ParserProgramCatalog::new(data, owner).unwrap();
    // No execution or assertion that an unselected source error has run.
}

#[test]
fn declaration_rhs_cannot_read_its_new_slot_but_missing_rhs_can_initialize_nil() {
    let (owner, mut data) = fixture();
    data.programs[0].body = vec![declare(0, None), returning(local(0))];
    ParserProgramCatalog::new(data.clone(), owner.clone()).unwrap();
    data.programs[0].body[0] = declare(0, Some(local(0)));
    rejects_structure(data, owner);
}

#[test]
fn branch_locals_cannot_escape_and_exclusive_branches_cannot_redeclare_a_slot() {
    let (owner, baseline) = fixture();
    let mut escaped = baseline.clone();
    escaped.programs[0].body = vec![dead_branch(vec![declare(0, None)]), returning(local(0))];
    rejects_structure(escaped, owner.clone());
    let mut duplicate = baseline;
    duplicate.programs[0].body = vec![statement(ParserProgramStatementKind::If {
        branches: vec![ParserProgramBranch {
            condition: literal(ParserFactoryLiteral::Boolean(false)),
            body: vec![declare(0, None)],
        }],
        otherwise: vec![declare(0, None)],
    })];
    rejects_structure(duplicate, owner);
}

#[test]
fn one_loop_declaration_can_execute_repeatedly_but_its_slots_do_not_escape() {
    let (owner, mut data) = fixture();
    data.programs[0].body = vec![
        numeric_loop(vec![
            declare(1, Some(local(0))),
            statement(ParserProgramStatementKind::Assign {
                locals: vec![1],
                values: values(vec![number(2.0)]),
            }),
            dead_branch(vec![statement(ParserProgramStatementKind::Break)]),
        ]),
        returning(nil()),
    ];
    let catalog = ParserProgramCatalog::new(data.clone(), owner.clone()).unwrap();
    assert!(
        catalog
            .required_capabilities()
            .contains(&ParserProgramCapability::NumericFor)
    );
    for slot in [0, 1] {
        let mut escaped = data.clone();
        *escaped.programs[0].body.last_mut().unwrap() = returning(local(slot));
        rejects_structure(escaped, owner.clone());
    }
    data.programs[0].body.push(declare(1, None));
    rejects_structure(data, owner);
}

#[test]
fn assignments_use_visible_parameters_and_cannot_create_new_locals() {
    let (owner, mut data) = fixture();
    data.programs[0].parameter_count = 1;
    data.programs[0].body = vec![
        statement(ParserProgramStatementKind::Assign {
            locals: vec![0],
            values: values(vec![]),
        }),
        returning(local(0)),
    ];
    ParserProgramCatalog::new(data.clone(), owner.clone()).unwrap();
    data.programs[0].body[0] = statement(ParserProgramStatementKind::Assign {
        locals: vec![1],
        values: values(vec![number(1.0)]),
    });
    rejects_structure(data, owner);
}

#[test]
fn borrowed_aliases_remain_explicit_reached_ownership_obligations() {
    let (owner, mut data) = fixture();
    data.programs[0].bindings = vec![ParserProgramBinding::Intrinsic {
        operation: ParserProgramIntrinsic::TableInsert,
        source: ParserProgramIntrinsicSource::OriginalGlobal,
    }];
    let borrowed = expr(ParserProgramExprKind::Definition {
        root: ParserProgramDefinitionRoot::GemIdLookup,
    });
    data.programs[0].body = vec![
        declare(
            0,
            Some(expr(ParserProgramExprKind::Table {
                fields: vec![ParserProgramField::Named {
                    key: "alias".into(),
                    value: borrowed,
                }],
            })),
        ),
        declare(
            1,
            Some(expr(ParserProgramExprKind::Get {
                table: Box::new(local(0)),
                key: Box::new(literal(ParserFactoryLiteral::Text("alias".into()))),
            })),
        ),
        statement(ParserProgramStatementKind::TableSet {
            table: local(1),
            key: number(1.0),
            value: nil(),
        }),
        statement(ParserProgramStatementKind::TableAppend {
            binding: 0,
            table: local(1),
            value: number(2.0),
        }),
    ];
    let catalog = ParserProgramCatalog::new(data.clone(), owner).unwrap();
    assert_eq!(
        catalog.program(ParserProgramId(1)).unwrap().body,
        data.programs[0].body
    );
    // G1 preserves the alias path. G2 must reject a reached borrowed-table write;
    // merely placing the reference in a fresh table cannot confer ownership.
}

#[test]
fn callback_maps_cannot_alias_programs_or_override_a_legacy_pure_factory() {
    let (owner, baseline) = fixture();
    let other = owner
        .data()
        .factories
        .keys()
        .copied()
        .find(|id| *id != baseline.programs[0].callback)
        .unwrap();
    let mut alias = baseline.clone();
    alias.callbacks.insert(other, ParserProgramId(1));
    rejects_structure(alias, owner.clone());
    let mut dangling = baseline.clone();
    dangling
        .callbacks
        .insert(baseline.programs[0].callback, ParserProgramId(2));
    rejects_structure(dangling, owner.clone());
    let mut duplicate = baseline.clone();
    duplicate.programs.push(duplicate.programs[0].clone());
    rejects_structure(duplicate, owner.clone());
    let pure = owner
        .data()
        .factories
        .iter()
        .find_map(|(id, value)| matches!(value, ParserFactoryDisposition::Pure(_)).then_some(*id))
        .unwrap();
    let ParserCallbackKind::Lua { source } = &owner.callback(pure).unwrap().kind else {
        unreachable!()
    };
    let mut overlap = baseline;
    overlap.programs[0].callback = pure;
    overlap.programs[0].provenance.source = source.clone();
    overlap.programs[0].provenance.function_sha256 = source.sha256.clone();
    overlap.callbacks = BTreeMap::from([(pure, ParserProgramId(1))]);
    rejects_structure(overlap, owner);
}

#[test]
fn invalid_capture_binding_is_rejected_even_when_its_call_is_unselected() {
    let (owner, mut data) = fixture();
    data.programs[0].bindings = vec![ParserProgramBinding::CapturedCallback {
        upvalue: u16::MAX,
        callback: data.programs[0].callback,
    }];
    data.programs[0].body = vec![dead_branch(vec![statement(
        ParserProgramStatementKind::Call {
            call: ParserProgramCall {
                binding: 0,
                receiver: None,
                arguments: values(vec![]),
            },
        },
    )])];
    rejects_structure(data, owner);
}

#[test]
fn nesting_and_local_limits_reject_without_executing_a_program() {
    let (owner, baseline) = fixture();
    for depth in [8, 80] {
        let mut value = nil();
        for _ in 0..depth {
            value = expr(ParserProgramExprKind::Unary {
                operation: ParserProgramUnary::Not,
                value: Box::new(value),
            });
        }
        let mut data = baseline.clone();
        data.programs[0].body = vec![returning(value)];
        let result = ParserProgramCatalog::new(data, owner.clone());
        if depth == 8 {
            result.unwrap();
        } else {
            assert_eq!(
                result.unwrap_err().kind,
                ParserProgramErrorKind::ResourceLimit
            );
        }
    }
    let mut data = baseline;
    data.programs[0].local_count = 1025;
    assert_eq!(
        ParserProgramCatalog::new(data, owner).unwrap_err().kind,
        ParserProgramErrorKind::ResourceLimit
    );
}

#[test]
fn aggregate_text_in_a_dead_branch_is_bounded_independently_of_each_literal() {
    let (owner, baseline) = fixture();
    // Both tables fit the 4096-field bound and every literal fits its 4096-byte
    // bound. Only the second program exceeds the aggregate eight-MiB budget.
    for count in [2000, 2050] {
        let mut data = baseline.clone();
        let table = expr(ParserProgramExprKind::Table {
            fields: (0..count)
                .map(|_| ParserProgramField::List {
                    value: literal(ParserFactoryLiteral::Text("x".repeat(4096))),
                })
                .collect(),
        });
        data.programs[0].body = vec![dead_branch(vec![returning(table)]), returning(nil())];
        let result = ParserProgramCatalog::new(data, owner.clone());
        if count == 2000 {
            result.unwrap();
        } else {
            assert_eq!(
                result.unwrap_err().kind,
                ParserProgramErrorKind::ResourceLimit
            );
        }
    }
}
