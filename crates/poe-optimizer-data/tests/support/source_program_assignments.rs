use super::*;
use SourceProgramAssignmentOperand as Operand;
use SourceProgramAssignmentTargetKind as Target;
fn target(operation: Target) -> SourceProgramAssignmentTarget {
    SourceProgramAssignmentTarget {
        location: SourceProgramLocation { start: 2, end: 4 },
        operation,
    }
}
fn mixed_owner(live: bool) -> SourceProgramOwner {
    let mut data = definitions();
    if !live {
        return SourceProgramOwner::new(data).unwrap();
    }
    data.callbacks[0].upvalues[0].value = SourceValue::LiveCapture {};
    SourceProgramOwner::new_with_closures(
        data,
        None,
        None,
        SourceClosurePrototypes {
            schema_version: SOURCE_CLOSURE_PROTOTYPES_SCHEMA_VERSION,
            prototypes: vec![SourceClosurePrototype {
                callback: SourceCallbackId(1),
            }],
        },
    )
    .unwrap()
}
fn mixed(targets: Vec<SourceProgramAssignmentTarget>) -> SourceProgram {
    let mut p = program(
        SourceCallbackId(1),
        SourceProgramExprKind::Literal {
            value: ParserFactoryLiteral::Nil,
        },
    );
    p.parameter_count = 2;
    p.local_count = 3;
    p.variadic = true;
    p.body[0].location = SourceProgramLocation { start: 0, end: 90 };
    p.body[0].operation = SourceProgramStatementKind::MixedAssign {
        targets,
        values: SourceProgramValueList {
            values: vec![expression(SourceProgramExprKind::Local { local: 1 })],
            tail: Some(Box::new(SourceProgramPack::Varargs)),
        },
    };
    p
}
fn bind(p: SourceProgram) -> SourceProgramResult<SourceProgramCatalog> {
    SourceProgramCatalog::new(programs(vec![p]), mixed_owner(true))
}
fn indexed(table: Operand, key: Operand) -> SourceProgramAssignmentTarget {
    target(Target::Indexed { table, key })
}
fn eager(operation: SourceProgramExprKind) -> Operand {
    Operand::Evaluated {
        value: expression(operation),
    }
}
#[test]
fn explicit_register_and_evaluated_operands_retain_target_order_and_full_rhs_wire() {
    let targets = vec![
        indexed(
            Operand::LocalRegister { local: 0 },
            Operand::LocalRegister { local: 1 },
        ),
        target(Target::Local { local: 0 }),
        indexed(
            eager(SourceProgramExprKind::Capture { upvalue: 0 }),
            eager(SourceProgramExprKind::Local { local: 1 }),
        ),
        target(Target::Capture { upvalue: 0 }),
        target(Target::Local { local: 0 }),
        target(Target::Capture { upvalue: 0 }),
    ];
    let p = mixed(targets.clone());
    let owner = mixed_owner(true);
    let data = programs(vec![p]);
    let bytes = serde_json::to_vec(&data).unwrap();
    let catalog = SourceProgramCatalog::from_bytes(&bytes, owner.clone()).unwrap();
    assert_eq!(catalog.data(), &data);
    assert!(
        catalog
            .required_capabilities()
            .contains(&SourceProgramCapability::MixedAssignment)
    );
    assert!(
        catalog
            .required_capabilities()
            .contains(&SourceProgramCapability::SessionClosures)
    );
    assert!(
        catalog
            .required_capabilities()
            .contains(&SourceProgramCapability::Varargs)
    );
    let mut supported = catalog.required_capabilities().clone();
    supported.remove(&SourceProgramCapability::MixedAssignment);
    assert!(catalog.check_capabilities(&supported).is_err());
    let decoded: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let operation = &decoded["programs"][0]["body"][0]["operation"];
    assert_eq!(operation["kind"], "mixed_assign");
    assert_eq!(
        operation["targets"][0]["operation"]["table"]["kind"],
        "local_register"
    );
    assert_eq!(
        operation["targets"][2]["operation"]["key"]["kind"],
        "evaluated"
    );
    assert_eq!(operation["values"]["tail"]["kind"], "varargs");
    let mut unknown = decoded;
    unknown["programs"][0]["body"][0]["operation"]["targets"][0]["unexpected"] = true.into();
    assert!(
        SourceProgramCatalog::from_bytes(&serde_json::to_vec(&unknown).unwrap(), owner).is_err()
    );
}
#[test]
fn mixed_targets_and_operands_require_visible_locals_and_declared_live_capture_slots() {
    for bad in [
        target(Target::Local { local: 2 }),
        indexed(
            Operand::LocalRegister { local: 2 },
            Operand::LocalRegister { local: 1 },
        ),
        indexed(
            Operand::LocalRegister { local: 0 },
            Operand::LocalRegister { local: u16::MAX },
        ),
        indexed(
            eager(SourceProgramExprKind::Local { local: 2 }),
            Operand::LocalRegister { local: 1 },
        ),
    ] {
        let error = bind(mixed(vec![bad])).unwrap_err();
        assert_eq!(error.kind, SourceProgramErrorKind::InvalidData);
        assert!(
            error.message.contains("lexical declaration scope"),
            "{error}"
        );
    }
    let bad = target(Target::Capture { upvalue: 1 });
    let error = bind(mixed(vec![bad.clone()])).unwrap_err();
    assert_eq!(error.kind, SourceProgramErrorKind::Binding);
    assert_eq!(error.location, Some(bad.location));
    assert!(error.message.contains("declared live capture"));
    let p = mixed(vec![target(Target::Capture { upvalue: 0 })]);
    let error = SourceProgramCatalog::new(programs(vec![p]), mixed_owner(false)).unwrap_err();
    assert_eq!(error.kind, SourceProgramErrorKind::UnsupportedCapability);
    assert!(error.message.contains("declared session closure"));
    let ordinary = mixed(vec![target(Target::Local { local: 0 })]);
    SourceProgramCatalog::new(programs(vec![ordinary]), mixed_owner(false)).unwrap();
}
#[test]
fn mixed_assignment_target_counts_ranges_expression_depth_and_rhs_are_bounded() {
    assert_eq!(
        bind(mixed(vec![])).unwrap_err().kind,
        SourceProgramErrorKind::InvalidData
    );
    let repeated = target(Target::Local { local: 0 });
    bind(mixed(vec![repeated.clone(); 128])).unwrap();
    assert_eq!(
        bind(mixed(vec![repeated; 129])).unwrap_err().kind,
        SourceProgramErrorKind::ResourceLimit
    );
    for location in [
        SourceProgramLocation { start: 2, end: 2 },
        SourceProgramLocation {
            start: 99,
            end: 101,
        },
    ] {
        let mut bad = target(Target::Local { local: 0 });
        bad.location = location;
        let error = bind(mixed(vec![bad])).unwrap_err();
        assert_eq!(error.kind, SourceProgramErrorKind::InvalidData);
        assert_eq!(error.location, Some(location));
    }
    let mut deep = expression(SourceProgramExprKind::Local { local: 0 });
    for _ in 0..50 {
        deep = expression(SourceProgramExprKind::Unary {
            operation: SourceProgramUnary::Not,
            value: Box::new(deep),
        });
    }
    assert_eq!(
        bind(mixed(vec![indexed(
            Operand::Evaluated { value: deep },
            Operand::LocalRegister { local: 1 }
        )]))
        .unwrap_err()
        .kind,
        SourceProgramErrorKind::ResourceLimit
    );
    let mut invalid_rhs = mixed(vec![target(Target::Local { local: 0 })]);
    invalid_rhs.variadic = false;
    assert!(bind(invalid_rhs).unwrap_err().message.contains("vararg"));
    let mut nil_rhs = mixed(vec![target(Target::Local { local: 0 })]);
    if let SourceProgramStatementKind::MixedAssign { values, .. } = &mut nil_rhs.body[0].operation {
        *values = SourceProgramValueList::default();
    }
    bind(nil_rhs).unwrap();
}
#[test]
fn mixed_assignment_cannot_escape_a_branch_local_scope_or_its_owner_callback() {
    let mut p = mixed(vec![target(Target::Local { local: 2 })]);
    let assignment = p.body.remove(0);
    let declaration = SourceProgramStatement {
        location: SourceProgramLocation { start: 5, end: 8 },
        operation: SourceProgramStatementKind::Declare {
            locals: vec![2],
            values: SourceProgramValueList::default(),
        },
    };
    p.body = vec![SourceProgramStatement {
        location: SourceProgramLocation { start: 1, end: 40 },
        operation: SourceProgramStatementKind::If {
            branches: vec![SourceProgramBranch {
                condition: expression(SourceProgramExprKind::Literal {
                    value: ParserFactoryLiteral::Boolean(true),
                }),
                body: vec![declaration, assignment.clone()],
            }],
            otherwise: vec![],
        },
    }];
    bind(p.clone()).unwrap();
    p.body.push(assignment);
    assert!(
        bind(p)
            .unwrap_err()
            .message
            .contains("lexical declaration scope")
    );
    let mut foreign = mixed(vec![target(Target::Capture { upvalue: 0 })]);
    foreign.callback = SourceCallbackId(2);
    assert!(
        bind(foreign)
            .unwrap_err()
            .message
            .contains("declared session closure")
    );
}
#[test]
fn parser_facade_rejects_even_local_only_mixed_assignment_without_changing_packaged_bytes() {
    let snapshot = bundled_snapshot().unwrap();
    let original = snapshot.modifier_parser();
    let bytes = serde_json::to_vec(&original.data().programs).unwrap();
    let (index, callback) = original
        .data()
        .callbacks
        .iter()
        .enumerate()
        .find(|(_, c)| matches!(c.kind, SourceCallbackKind::Lua { .. }))
        .unwrap();
    let SourceCallbackKind::Lua { source } = &callback.kind else {
        unreachable!()
    };
    let mut p = mixed(vec![target(Target::Local { local: 0 })]);
    p.callback = SourceCallbackId(index as u32 + 1);
    p.provenance.source = source.clone();
    let data = programs(vec![p]);
    let direct = ParserProgramCatalog::new(data.clone(), original.clone()).unwrap_err();
    let facade = SourceProgramCatalog::new(data, SourceProgramOwner::from_parser(original.clone()))
        .unwrap_err();
    for error in [direct, facade] {
        assert_eq!(error.kind, SourceProgramErrorKind::UnsupportedCapability);
        assert!(
            error
                .message
                .contains("parser owner does not admit mixed assignment")
        );
    }
    assert_eq!(
        serde_json::to_vec(&original.data().programs).unwrap(),
        bytes
    );
}

fn read_program(table: Operand, key: SourceProgramExpr) -> SourceProgram {
    let mut p = mixed(vec![target(Target::Local { local: 0 })]);
    p.body[0].operation = SourceProgramStatementKind::Return {
        values: SourceProgramValueList {
            values: vec![expression(SourceProgramExprKind::IndexedRead {
                table: Box::new(table),
                key: Box::new(key),
            })],
            tail: None,
        },
    };
    p
}
#[test]
fn indexed_reads_have_independent_capability_and_bounded_scoped_operands() {
    let key = expression(SourceProgramExprKind::Local { local: 1 });
    let p = read_program(Operand::LocalRegister { local: 0 }, key.clone());
    let catalog = bind(p.clone()).unwrap();
    assert!(
        catalog
            .required_capabilities()
            .contains(&SourceProgramCapability::RegisterOperands)
    );
    assert!(
        !catalog
            .required_capabilities()
            .contains(&SourceProgramCapability::MixedAssignment)
    );
    let mut capabilities = catalog.required_capabilities().clone();
    capabilities.remove(&SourceProgramCapability::RegisterOperands);
    assert!(catalog.check_capabilities(&capabilities).is_err());
    let wire = serde_json::to_vec(&programs(vec![p])).unwrap();
    let restored = SourceProgramCatalog::from_bytes(&wire, mixed_owner(true)).unwrap();
    assert_eq!(restored.data(), catalog.data());
    bind(read_program(
        eager(SourceProgramExprKind::Capture { upvalue: 0 }),
        key.clone(),
    ))
    .unwrap();
    for operand in [
        Operand::LocalRegister { local: 2 },
        eager(SourceProgramExprKind::Local { local: 2 }),
    ] {
        assert!(
            bind(read_program(operand, key.clone()))
                .unwrap_err()
                .message
                .contains("lexical declaration scope")
        );
    }
    assert!(
        bind(read_program(
            Operand::LocalRegister { local: 0 },
            expression(SourceProgramExprKind::Local { local: 2 })
        ))
        .unwrap_err()
        .message
        .contains("lexical declaration scope")
    );
    let mut deep = key;
    for _ in 0..50 {
        deep = expression(SourceProgramExprKind::IndexedRead {
            table: Box::new(Operand::LocalRegister { local: 0 }),
            key: Box::new(deep),
        });
    }
    assert_eq!(
        bind(read_program(Operand::LocalRegister { local: 0 }, deep))
            .unwrap_err()
            .kind,
        SourceProgramErrorKind::ResourceLimit
    );
}
#[test]
fn indexed_read_is_rejected_by_both_legacy_parser_facades_without_wire_change() {
    let snapshot = bundled_snapshot().unwrap();
    let owner = snapshot.modifier_parser();
    let before = serde_json::to_vec(&owner.data().programs).unwrap();
    let (index, callback) = owner
        .data()
        .callbacks
        .iter()
        .enumerate()
        .find(|(_, c)| matches!(c.kind, SourceCallbackKind::Lua { .. }))
        .unwrap();
    let SourceCallbackKind::Lua { source } = &callback.kind else {
        unreachable!()
    };
    let mut p = read_program(
        Operand::LocalRegister { local: 0 },
        expression(SourceProgramExprKind::Local { local: 1 }),
    );
    p.callback = SourceCallbackId(index as u32 + 1);
    p.provenance.source = source.clone();
    let data = programs(vec![p]);
    for error in [
        ParserProgramCatalog::new(data.clone(), owner.clone()).unwrap_err(),
        SourceProgramCatalog::new(data, SourceProgramOwner::from_parser(owner.clone()))
            .unwrap_err(),
    ] {
        assert_eq!(error.kind, SourceProgramErrorKind::UnsupportedCapability);
        assert!(
            error
                .message
                .contains("parser owner does not admit live-register indexed reads")
        );
    }
    assert_eq!(serde_json::to_vec(&owner.data().programs).unwrap(), before);
}
#[test]
fn implementation_fingerprint_covers_every_neutral_source_program_file_once() {
    let directory = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let fingerprint = include_str!("../../src/lib.rs");
    let included = fingerprint
        .lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix("include_str!(\"")?
                .strip_suffix("\"),")
        })
        .filter(|path| *path == "source_program.rs" || path.starts_with("source_program/"))
        .collect::<Vec<_>>();
    let unique = included.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(
        included.len(),
        unique.len(),
        "duplicate source implementation hash input"
    );
    let mut actual = BTreeSet::from(["source_program.rs".to_owned()]);
    let mut pending = vec![directory.join("source_program")];
    while let Some(folder) = pending.pop() {
        for entry in std::fs::read_dir(folder).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                pending.push(entry.path());
            } else if entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "rs")
            {
                actual.insert(
                    entry
                        .path()
                        .strip_prefix(&directory)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
    }
    assert_eq!(
        unique,
        actual.iter().map(String::as_str).collect(),
        "new neutral source files must enter the implementation fingerprint"
    );
}
