use poe_optimizer_data::{
    game_data::bundled_snapshot,
    item_loading::{ItemLoadingSource, ItemSourceSpan},
    modifier_parser::{ParserFactoryLiteral, ParserProgramCatalog},
    source_program::*,
};
use std::collections::BTreeMap;
type Origin = SourceProgramCaptureOrigin;
type E = SourceProgramExprKind;
type S = SourceProgramStatementKind;
fn range(start: u32, end: u32) -> SourceProgramLocation {
    SourceProgramLocation { start, end }
}
fn span(child: bool) -> ItemSourceSpan {
    ItemSourceSpan {
        path: "fixture.lua".into(),
        line: if child { 4 } else { 1 },
        end_line: if child { 8 } else { 20 },
        sha256: if child { "e" } else { "b" }.repeat(64),
    }
}
fn provenance(child: bool) -> SourceProgramProvenance {
    SourceProgramProvenance {
        source: span(child),
        function_start: 0,
        function_end: if child { 40 } else { 100 },
        function_sha256: if child { "d" } else { "f" }.repeat(64),
    }
}
fn owner(static_parent: bool, child_captures: usize) -> SourceProgramOwner {
    let callbacks = [(false, 1), (true, child_captures)]
        .into_iter()
        .map(|(child, n)| SourceCallback {
            kind: SourceCallbackKind::Lua {
                source: span(child),
            },
            environment: SourceEnvironment::OriginalGlobals,
            upvalues: (0..n)
                .map(|i| SourceUpvalue {
                    name: format!("capture{i}"),
                    value: if static_parent && !child {
                        SourceValue::Number(2.0)
                    } else {
                        SourceValue::LiveCapture {}
                    },
                })
                .collect(),
        })
        .collect();
    SourceProgramOwner::new_with_closures(
        SourceProgramDefinitions {
            schema_version: SOURCE_PROGRAM_DEFINITIONS_SCHEMA_VERSION,
            source: ItemLoadingSource {
                upstream_revision: "a".repeat(40),
                files: [("fixture.lua".into(), "c".repeat(64))].into(),
                construction_spans: BTreeMap::new(),
                module_order: vec!["fixture.lua".into()],
            },
            tables: vec![],
            callbacks,
            roots: vec![],
            intrinsics: BTreeMap::new(),
        },
        None,
        None,
        SourceClosurePrototypes {
            schema_version: SOURCE_CLOSURE_PROTOTYPES_SCHEMA_VERSION,
            prototypes: if static_parent {
                vec![SourceClosurePrototype {
                    callback: SourceCallbackId(2),
                }]
            } else {
                vec![
                    SourceClosurePrototype {
                        callback: SourceCallbackId(1),
                    },
                    SourceClosurePrototype {
                        callback: SourceCallbackId(2),
                    },
                ]
            },
        },
    )
    .unwrap()
}
fn expr(operation: E, location: SourceProgramLocation) -> SourceProgramExpr {
    SourceProgramExpr {
        operation,
        location,
    }
}
fn nil() -> SourceProgramExpr {
    expr(
        E::Literal {
            value: ParserFactoryLiteral::Nil,
        },
        range(11, 12),
    )
}
fn values(value: SourceProgramExpr) -> SourceProgramValueList {
    SourceProgramValueList {
        values: vec![value],
        tail: None,
    }
}
fn stmt(operation: S, location: SourceProgramLocation) -> SourceProgramStatement {
    SourceProgramStatement {
        operation,
        location,
    }
}
fn fixture() -> (
    SourceProgramData,
    SourceProgramOwner,
    SourceProgramClosureCreations,
) {
    let captures = vec![
        Origin::Local { local: 0 },
        Origin::Local { local: 1 },
        Origin::ParentCapture { upvalue: 0 },
    ];
    let creation = expr(
        E::CreateClosure {
            prototype: SourceClosurePrototypeId(2),
            captures: captures.clone(),
        },
        range(30, 70),
    );
    let parent = SourceProgram {
        callback: SourceCallbackId(1),
        parameter_count: 1,
        variadic: false,
        local_count: 3,
        bindings: vec![],
        provenance: provenance(false),
        body: vec![
            stmt(
                S::Declare {
                    locals: vec![1],
                    values: values(nil()),
                },
                range(10, 20),
            ),
            stmt(
                S::Return {
                    values: values(creation),
                },
                range(25, 75),
            ),
        ],
    };
    let child = SourceProgram {
        callback: SourceCallbackId(2),
        parameter_count: 0,
        variadic: false,
        local_count: 0,
        bindings: vec![],
        provenance: provenance(true),
        body: vec![stmt(
            S::Return {
                values: values(expr(E::Capture { upvalue: 2 }, range(10, 11))),
            },
            range(8, 15),
        )],
    };
    let data = SourceProgramData {
        schema_version: SOURCE_PROGRAM_SCHEMA_VERSION,
        programs: vec![parent, child],
        callbacks: [
            (SourceCallbackId(1), SourceProgramId(1)),
            (SourceCallbackId(2), SourceProgramId(2)),
        ]
        .into(),
    };
    let metadata = SourceProgramClosureCreations {
        schema_version: SOURCE_PROGRAM_CLOSURE_CREATIONS_SCHEMA_VERSION,
        profile: SourceTableRuntimeProfile::luajit21_x64_single(),
        sites: vec![SourceProgramClosureCreation {
            callback: SourceCallbackId(1),
            provenance: provenance(false),
            expression: range(30, 70),
            prototype: SourceClosurePrototypeId(2),
            child_provenance: provenance(true),
            bytecode_sha256: "1".repeat(64),
            bytecode_pc: 7,
            instruction: 51,
            child_bytecode_sha256: "2".repeat(64),
            captures,
            capture_descriptors: vec![0xc000, 0x8003, 0],
            local_bindings: vec![
                SourceProgramClosureLocalBinding {
                    local: 0,
                    register: 0,
                    declaration: range(9, 10),
                    kind: SourceProgramClosureLocalKind::Parameter,
                },
                SourceProgramClosureLocalBinding {
                    local: 1,
                    register: 3,
                    declaration: range(10, 20),
                    kind: SourceProgramClosureLocalKind::Declare,
                },
            ],
        }],
    };
    (data, owner(false, 3), metadata)
}
fn bind(
    data: SourceProgramData,
    owner: SourceProgramOwner,
    metadata: SourceProgramClosureCreations,
) -> SourceProgramResult<SourceProgramCatalog> {
    SourceProgramCatalog::new_with_closure_creations(data, owner, None, metadata)
}
fn creation(data: &mut SourceProgramData) -> &mut SourceProgramExpr {
    let S::Return { values } = &mut data.programs[0].body[1].operation else {
        panic!()
    };
    &mut values.values[0]
}
#[test]
fn exact_creation_binds_parameters_locals_and_parent_cells_without_live_values() {
    let (data, owner, metadata) = fixture();
    let bytes = serde_json::to_vec(&metadata).unwrap();
    let decoded = SourceProgramClosureCreations::from_bytes(&bytes).unwrap();
    assert_eq!(decoded, metadata);
    let catalog = bind(data.clone(), owner.clone(), decoded).unwrap();
    assert_eq!(catalog.closure_creations(), Some(&metadata));
    assert!(catalog.is_bound_to(&owner));
    assert!(!catalog.is_bound_to(&fixture().1));
    assert!(
        catalog
            .required_capabilities()
            .contains(&SourceProgramCapability::ClosureCreation)
    );
    let mut caps = catalog.required_capabilities().clone();
    caps.remove(&SourceProgramCapability::ClosureCreation);
    assert!(catalog.check_capabilities(&caps).is_err());
    assert_eq!(
        serde_json::to_vec(catalog.data()).unwrap(),
        serde_json::to_vec(&data).unwrap()
    );
    assert!(
        SourceProgramCatalog::new(data, owner)
            .unwrap_err()
            .message
            .contains("creation metadata")
    );
}
#[test]
fn child_program_prototype_layout_and_live_parent_are_required() {
    let (data, _, metadata) = fixture();
    assert!(
        bind(data.clone(), owner(false, 2), metadata.clone())
            .unwrap_err()
            .message
            .contains("exact capture layout")
    );
    let mut missing = data.clone();
    missing.programs.pop();
    missing.callbacks.remove(&SourceCallbackId(2));
    assert!(
        bind(missing, owner(false, 3), metadata.clone())
            .unwrap_err()
            .message
            .contains("complete child program")
    );
    let mut foreign = data.clone();
    if let E::CreateClosure { prototype, .. } = &mut creation(&mut foreign).operation {
        *prototype = SourceClosurePrototypeId(99);
    }
    assert!(
        bind(foreign, owner(false, 3), metadata.clone())
            .unwrap_err()
            .message
            .contains("not declared")
    );
    let mut static_data = data;
    if let E::CreateClosure { prototype, .. } = &mut creation(&mut static_data).operation {
        *prototype = SourceClosurePrototypeId(1);
    }
    let mut static_meta = metadata;
    static_meta.sites[0].prototype = SourceClosurePrototypeId(1);
    assert!(
        bind(static_data, owner(true, 3), static_meta)
            .unwrap_err()
            .message
            .contains("declared session closure")
    );
}
#[test]
fn creation_scope_distinguishes_initializer_capture_and_recursive_local_declaration() {
    let (mut data, owner, metadata) = fixture();
    let created = creation(&mut data).clone();
    data.programs[0].body = vec![stmt(
        S::Declare {
            locals: vec![1],
            values: values(created.clone()),
        },
        range(10, 20),
    )];
    assert!(
        bind(data.clone(), owner.clone(), metadata.clone())
            .unwrap_err()
            .message
            .contains("lexical declaration scope")
    );
    data.programs[0].body = vec![
        stmt(
            S::Declare {
                locals: vec![1],
                values: SourceProgramValueList::default(),
            },
            range(10, 20),
        ),
        stmt(
            S::Assign {
                locals: vec![1],
                values: values(created),
            },
            range(25, 75),
        ),
    ];
    bind(data, owner, metadata).unwrap();
    let (mut data, owner, metadata) = fixture();
    let declared = data.programs[0].body.remove(0);
    data.programs[0].body.insert(
        0,
        stmt(
            S::If {
                branches: vec![SourceProgramBranch {
                    condition: expr(
                        E::Literal {
                            value: ParserFactoryLiteral::Boolean(true),
                        },
                        range(2, 3),
                    ),
                    body: vec![declared],
                }],
                otherwise: vec![],
            },
            range(1, 22),
        ),
    );
    assert!(
        bind(data, owner, metadata)
            .unwrap_err()
            .message
            .contains("lexical declaration scope")
    );
}
#[test]
fn declaration_proof_matches_loop_kind_and_exact_source_range() {
    for numeric in [true, false] {
        let (mut data, owner, mut metadata) = fixture();
        let returning = data.programs[0].body.pop().unwrap();
        let n = expr(
            E::Literal {
                value: ParserFactoryLiteral::Number(1.0),
            },
            range(2, 3),
        );
        let operation = if numeric {
            S::ForNumeric {
                local: 1,
                start: n.clone(),
                limit: n.clone(),
                step: n,
                body: vec![returning],
            }
        } else {
            S::ForEach {
                locals: vec![1],
                iterator: SourceProgramIterator::Generic { values: values(n) },
                body: vec![returning],
            }
        };
        data.programs[0].body = vec![stmt(operation, range(10, 80))];
        metadata.sites[0].local_bindings[1].declaration = range(10, 80);
        metadata.sites[0].local_bindings[1].kind = if numeric {
            SourceProgramClosureLocalKind::NumericFor
        } else {
            SourceProgramClosureLocalKind::GenericFor
        };
        bind(data.clone(), owner.clone(), metadata.clone()).unwrap();
        metadata.sites[0].local_bindings[1].declaration = range(11, 80);
        assert!(
            bind(data, owner, metadata)
                .unwrap_err()
                .message
                .contains("range/kind")
        );
    }
}
#[test]
fn creation_evidence_rejects_wrong_opcode_descriptor_declaration_and_child_provenance() {
    for case in 0..13 {
        let (data, owner, mut m) = fixture();
        let site = &mut m.sites[0];
        match case {
            0 => site.instruction = 52,
            1 => site.capture_descriptors[0] = 0,
            2 => site.capture_descriptors[1] = 0x8103,
            3 => site.capture_descriptors[2] = 1,
            4 => site.local_bindings[1].register = 4,
            5 => site.local_bindings[1].kind = SourceProgramClosureLocalKind::Parameter,
            6 => site.local_bindings[1].declaration = range(11, 20),
            7 => site.local_bindings.push(site.local_bindings[0].clone()),
            8 => site.local_bindings.pop().map(|_| ()).unwrap(),
            9 => site.child_provenance.function_sha256 = "3".repeat(64),
            10 => site.expression = range(31, 70),
            11 => site.captures.reverse(),
            12 => site.bytecode_pc = 0,
            _ => unreachable!(),
        }
        assert!(bind(data, owner, m).is_err(), "case {case}");
    }
}
#[test]
fn creation_sites_are_one_to_one_and_hashes_profiles_and_child_edges_are_coherent() {
    let (data, owner, mut meta) = fixture();
    meta.sites.push(meta.sites[0].clone());
    assert!(
        bind(data.clone(), owner.clone(), meta)
            .unwrap_err()
            .message
            .contains("duplicate")
    );
    let (_, _, mut missing) = fixture();
    missing.sites.clear();
    assert!(
        bind(data.clone(), owner.clone(), missing)
            .unwrap_err()
            .message
            .contains("every exact")
    );
    let tables = SourceProgramConstructors {
        schema_version: SOURCE_PROGRAM_CONSTRUCTORS_SCHEMA_VERSION,
        profile: SourceTableRuntimeProfile {
            number_mode: SourceNumberMode::Dual,
            ..SourceTableRuntimeProfile::luajit21_x64_single()
        },
        sites: vec![],
    };
    assert!(
        SourceProgramCatalog::new_with_closure_creations(data, owner, Some(tables), fixture().2)
            .unwrap_err()
            .message
            .contains("profiles differ")
    );
    let (mut data, owner, mut meta) = fixture();
    let extra = data.programs[0].body[1].clone();
    data.programs[0].body.push(extra);
    assert!(
        bind(data, owner, meta.clone())
            .unwrap_err()
            .message
            .contains("every exact")
    );
    meta.profile.source_revision = "unsupported-but-structural".into();
    let (data, owner, _) = fixture();
    let catalog = bind(data, owner, meta).unwrap();
    assert!(
        !catalog
            .closure_creations()
            .unwrap()
            .profile
            .is_supported_array_profile()
    );
}
#[test]
fn metadata_decoding_and_preflight_bound_all_lists_bytes_and_source_ranges() {
    let (_, _, meta) = fixture();
    let mut json = serde_json::to_value(&meta).unwrap();
    json["sites"][0]["unexpected"] = true.into();
    assert!(
        SourceProgramClosureCreations::from_bytes(&serde_json::to_vec(&json).unwrap()).is_err()
    );
    for field in ["capture_descriptors", "captures", "local_bindings"] {
        let mut json = serde_json::to_value(&meta).unwrap();
        let value = json["sites"][0][field][0].clone();
        json["sites"][0][field] = serde_json::Value::Array(vec![value; 129]);
        assert!(
            SourceProgramClosureCreations::from_bytes(&serde_json::to_vec(&json).unwrap()).is_err()
        );
    }
    let (data, owner, mut meta) = fixture();
    meta.sites[0].bytecode_sha256 = "0".repeat(4097);
    assert_eq!(
        bind(data, owner, meta).unwrap_err().kind,
        SourceProgramErrorKind::ResourceLimit
    );
    let (data, owner, mut meta) = fixture();
    meta.sites[0].local_bindings[0].declaration = range(100, 101);
    assert!(bind(data, owner, meta).is_err());
}
#[test]
fn standalone_source_binary_preserves_operand_kind_and_rejects_non_arithmetic_forms() {
    let (mut data, owner, _) = fixture();
    data.programs.truncate(1);
    data.callbacks.remove(&SourceCallbackId(2));
    let mut binary = expr(
        E::SourceBinary {
            operation: SourceProgramBinary::Add,
            left: Box::new(SourceProgramOperand::LocalRegister { local: 0 }),
            right: Box::new(nil()),
        },
        range(30, 70),
    );
    data.programs[0].body[1] = stmt(
        S::Return {
            values: values(binary.clone()),
        },
        range(25, 75),
    );
    let catalog = SourceProgramCatalog::new(data.clone(), owner.clone()).unwrap();
    assert!(
        catalog
            .required_capabilities()
            .contains(&SourceProgramCapability::RegisterOperands)
    );
    for operation in [
        SourceProgramBinary::And,
        SourceProgramBinary::Or,
        SourceProgramBinary::Concat,
    ] {
        if let E::SourceBinary { operation: op, .. } = &mut binary.operation {
            *op = operation;
        }
        data.programs[0].body[1] = stmt(
            S::Return {
                values: values(binary.clone()),
            },
            range(25, 75),
        );
        assert!(
            SourceProgramCatalog::new(data.clone(), owner.clone())
                .unwrap_err()
                .message
                .contains("exclude logical")
        );
    }
}
#[test]
fn parser_facade_rejects_creation_and_binary_register_capabilities() {
    let snapshot = bundled_snapshot().unwrap();
    let parser = snapshot.modifier_parser().clone();
    let source = SourceProgramOwner::from_parser(parser.clone());
    let (callback, original) = source
        .callbacks()
        .iter()
        .enumerate()
        .find(|(_, c)| matches!(c.kind, SourceCallbackKind::Lua { .. }))
        .unwrap();
    let SourceCallbackKind::Lua { source: span } = &original.kind else {
        unreachable!()
    };
    for operation in [
        E::CreateClosure {
            prototype: SourceClosurePrototypeId(1),
            captures: vec![],
        },
        E::SourceBinary {
            operation: SourceProgramBinary::Add,
            left: Box::new(SourceProgramOperand::LocalRegister { local: 0 }),
            right: Box::new(nil()),
        },
    ] {
        let id = SourceCallbackId(callback as u32 + 1);
        let p = SourceProgram {
            callback: id,
            parameter_count: 1,
            variadic: false,
            local_count: 1,
            bindings: vec![],
            body: vec![stmt(
                S::Return {
                    values: values(expr(operation, range(30, 70))),
                },
                range(25, 75),
            )],
            provenance: SourceProgramProvenance {
                source: span.clone(),
                function_start: 0,
                function_end: 100,
                function_sha256: "a".repeat(64),
            },
        };
        let data = SourceProgramData {
            schema_version: SOURCE_PROGRAM_SCHEMA_VERSION,
            programs: vec![p],
            callbacks: [(id, SourceProgramId(1))].into(),
        };
        let err = ParserProgramCatalog::new(data, parser.clone()).unwrap_err();
        assert_eq!(
            err.kind,
            SourceProgramErrorKind::UnsupportedCapability,
            "{err}"
        );
    }
}
#[test]
fn zero_capture_children_still_require_explicit_prototypes_and_complete_site_evidence() {
    let (mut data, _, mut meta) = fixture();
    if let E::CreateClosure { captures, .. } = &mut creation(&mut data).operation {
        captures.clear();
    }
    data.programs[1].body[0].operation = S::Return {
        values: values(expr(
            E::Literal {
                value: ParserFactoryLiteral::Nil,
            },
            range(10, 11),
        )),
    };
    meta.sites[0].captures.clear();
    meta.sites[0].capture_descriptors.clear();
    meta.sites[0].local_bindings.clear();
    bind(data.clone(), owner(false, 0), meta.clone()).unwrap();
    assert!(bind(data.clone(), owner(false, 1), meta.clone()).is_err());
    meta.sites[0].prototype = SourceClosurePrototypeId(99);
    assert!(bind(data, owner(false, 0), meta).is_err());
}
#[test]
fn source_child_prototype_cycles_reject_without_confusing_function_recursion() {
    let (mut data, old_owner, mut meta) = fixture();
    let mut defs = old_owner.definitions().unwrap().clone();
    defs.callbacks[1].kind = SourceCallbackKind::Lua {
        source: span(false),
    };
    let owner = SourceProgramOwner::new_with_closures(
        defs,
        None,
        None,
        old_owner.closure_prototypes().unwrap().clone(),
    )
    .unwrap();
    data.programs[1].provenance.source = span(false);
    meta.sites[0].child_provenance = data.programs[1].provenance.clone();
    let captures = vec![Origin::ParentCapture { upvalue: 0 }];
    data.programs[1].body = vec![stmt(
        S::Return {
            values: values(expr(
                E::CreateClosure {
                    prototype: SourceClosurePrototypeId(1),
                    captures: captures.clone(),
                },
                range(10, 30),
            )),
        },
        range(8, 35),
    )];
    meta.sites.push(SourceProgramClosureCreation {
        callback: SourceCallbackId(2),
        provenance: data.programs[1].provenance.clone(),
        expression: range(10, 30),
        prototype: SourceClosurePrototypeId(1),
        child_provenance: provenance(false),
        bytecode_sha256: "2".repeat(64),
        bytecode_pc: 3,
        instruction: 51,
        child_bytecode_sha256: "1".repeat(64),
        captures,
        capture_descriptors: vec![0],
        local_bindings: vec![],
    });
    assert!(
        bind(data, owner, meta)
            .unwrap_err()
            .message
            .contains("cyclic")
    );
}
#[test]
fn creation_walker_visits_nested_call_packs_binary_operands_and_assignment_targets() {
    for mode in 0..4 {
        let (mut data, owner, meta) = fixture();
        let created = creation(&mut data).clone();
        let operation = match mode {
            0 => S::Return {
                values: values(expr(
                    E::SourceBinary {
                        operation: SourceProgramBinary::Equal,
                        left: Box::new(SourceProgramOperand::Evaluated { value: created }),
                        right: Box::new(nil()),
                    },
                    range(25, 80),
                )),
            },
            1 => S::Return {
                values: values(expr(
                    E::SourceBinary {
                        operation: SourceProgramBinary::Equal,
                        left: Box::new(SourceProgramOperand::LocalRegister { local: 0 }),
                        right: Box::new(created),
                    },
                    range(25, 80),
                )),
            },
            2 => {
                data.programs[0]
                    .bindings
                    .push(SourceProgramBinding::DynamicCall {});
                S::Return {
                    values: SourceProgramValueList {
                        values: vec![],
                        tail: Some(Box::new(SourceProgramPack::Call {
                            call: SourceProgramCall {
                                binding: 0,
                                receiver: Some(Box::new(created)),
                                arguments: SourceProgramValueList::default(),
                            },
                        })),
                    },
                }
            }
            3 => S::MixedAssign {
                targets: vec![SourceProgramAssignmentTarget {
                    location: range(25, 80),
                    operation: SourceProgramAssignmentTargetKind::Indexed {
                        table: SourceProgramOperand::Evaluated { value: created },
                        key: SourceProgramOperand::Evaluated { value: nil() },
                    },
                }],
                values: SourceProgramValueList::default(),
            },
            _ => unreachable!(),
        };
        data.programs[0].body[1] = stmt(operation, range(25, 90));
        bind(data, owner, meta).unwrap();
    }
}

#[test]
fn source_register_aliases_and_reused_child_capture_descriptors_cannot_disagree() {
    let (data, owner, mut meta) = fixture();
    meta.sites[0].local_bindings[1].register = 0;
    meta.sites[0].capture_descriptors[1] = 0x8000;
    assert!(
        bind(data, owner, meta)
            .unwrap_err()
            .message
            .contains("share one source register")
    );
    let (mut data, owner, mut meta) = fixture();
    let mut created = creation(&mut data).clone();
    created.location = range(75, 85);
    data.programs[0].body.push(stmt(
        S::Return {
            values: values(created),
        },
        range(74, 90),
    ));
    let mut site = meta.sites[0].clone();
    site.expression = range(75, 85);
    site.bytecode_pc = 8;
    meta.sites.push(site);
    bind(data.clone(), owner.clone(), meta.clone()).unwrap();
    meta.sites[1].capture_descriptors[0] ^= 0x4000;
    assert!(
        bind(data, owner, meta)
            .unwrap_err()
            .message
            .contains("immutable child capture descriptors")
    );
}
