//! Structural class/dispatch tests. Original source differential tests live in PoB.
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
fn b(s: &str) -> ParserProgramExpr {
    e(ParserProgramExprKind::Bytes {
        value: s.as_bytes().to_vec(),
    })
}
fn n(n: f64) -> ParserProgramExpr {
    e(ParserProgramExprKind::Literal {
        value: ParserFactoryLiteral::Number(n),
    })
}
fn get(t: ParserProgramExpr, k: &str) -> ParserProgramExpr {
    e(ParserProgramExprKind::Get {
        table: Box::new(t),
        key: Box::new(b(k)),
    })
}
fn st(operation: ParserProgramStatementKind) -> ParserProgramStatement {
    ParserProgramStatement {
        location: ParserProgramLocation { start: 0, end: 1 },
        operation,
    }
}
fn list(values: Vec<ParserProgramExpr>) -> ParserProgramValueList {
    ParserProgramValueList { values, tail: None }
}
fn ret(values: Vec<ParserProgramExpr>) -> ParserProgramStatement {
    st(ParserProgramStatementKind::Return {
        values: list(values),
    })
}
fn set(key: &str, value: ParserProgramExpr) -> ParserProgramStatement {
    st(ParserProgramStatementKind::TableSet {
        table: l(0),
        key: b(key),
        value,
    })
}
fn call(
    binding: u16,
    receiver: Option<ParserProgramExpr>,
    values: Vec<ParserProgramExpr>,
) -> ParserProgramCall {
    ParserProgramCall {
        binding,
        receiver: receiver.map(Box::new),
        arguments: list(values),
    }
}
fn eq(a: ParserProgramExpr, b: ParserProgramExpr) -> ParserProgramExpr {
    e(ParserProgramExprKind::Binary {
        operation: ParserProgramBinary::Equal,
        left: Box::new(a),
        right: Box::new(b),
    })
}
fn fixture() -> (SourceProgramOwner, CompiledSourcePrograms) {
    let path = "src/Classes/Fixture.lua".to_owned();
    let sha = "a".repeat(64);
    let span = ItemSourceSpan {
        path: path.clone(),
        line: 1,
        end_line: 20,
        sha256: sha.clone(),
    };
    let mut callbacks = (0..41)
        .map(|_| ParserCallback {
            kind: ParserCallbackKind::Lua {
                source: span.clone(),
            },
            environment: ParserEnvironment::OriginalGlobals,
            upvalues: vec![],
        })
        .collect::<Vec<_>>();
    callbacks[2].upvalues = vec![
        ParserUpvalue {
            name: "original".into(),
            value: ParserValue::Callback(ParserCallbackId(2)),
        },
        ParserUpvalue {
            name: "class".into(),
            value: ParserValue::Table(ParserTableId(2)),
        },
        ParserUpvalue {
            name: "name".into(),
            value: ParserValue::Text("Child".into()),
        },
    ];
    callbacks[2].upvalues.push(ParserUpvalue {
        name: "pairs".into(),
        value: ParserValue::Callback(ParserCallbackId(40)),
    });
    callbacks[9].upvalues = vec![ParserUpvalue {
        name: "s_format".into(),
        value: ParserValue::Callback(ParserCallbackId(41)),
    }];
    callbacks[39].kind = ParserCallbackKind::Builtin {
        symbol: "pairs".into(),
    };
    callbacks[40].kind = ParserCallbackKind::Builtin {
        symbol: "string.format".into(),
    };
    callbacks[5].upvalues = vec![ParserUpvalue {
        name: "mutate".into(),
        value: ParserValue::Callback(ParserCallbackId(5)),
    }];
    let base_methods = BTreeMap::from([(
        "Record".into(),
        SourceClassMethod {
            callback: ParserCallbackId(4),
            declared_by: SourceClassId(1),
        },
    )]);
    let definitions = SourceProgramDefinitions {
        schema_version: SOURCE_PROGRAM_DEFINITIONS_SCHEMA_VERSION,
        source: ItemLoadingSource {
            upstream_revision: "b".repeat(40),
            files: BTreeMap::from([(path.clone(), sha)]),
            construction_spans: BTreeMap::new(),
            module_order: vec![path],
        },
        tables: vec![
            ParserTable {
                fields: BTreeMap::from([
                    ("_className".into(), ParserValue::Text("Base".into())),
                    ("__index".into(), ParserValue::Table(ParserTableId(1))),
                    ("Base".into(), ParserValue::Callback(ParserCallbackId(1))),
                    ("Record".into(), ParserValue::Callback(ParserCallbackId(4))),
                ]),
                indexed: BTreeMap::new(),
            },
            ParserTable {
                fields: BTreeMap::from([
                    ("_className".into(), ParserValue::Text("Child".into())),
                    ("__index".into(), ParserValue::Table(ParserTableId(2))),
                    (
                        "_unconstructedMeta".into(),
                        ParserValue::Table(ParserTableId(2)),
                    ),
                    ("_constructorInitialised".into(), ParserValue::Boolean(true)),
                    ("Child".into(), ParserValue::Callback(ParserCallbackId(3))),
                    ("_parents".into(), ParserValue::Table(ParserTableId(3))),
                    ("Record".into(), ParserValue::Callback(ParserCallbackId(4))),
                ]),
                indexed: BTreeMap::new(),
            },
            ParserTable {
                fields: BTreeMap::new(),
                indexed: BTreeMap::from([(1, ParserValue::Table(ParserTableId(1)))]),
            },
        ],
        callbacks,
        roots: vec![SourceProgramRoot {
            name: "ChildDefinition".into(),
            table: ParserTableId(2),
        }],
        intrinsics: BTreeMap::new(),
    };
    let classes = SourceClassDefinitions {
        schema_version: SOURCE_CLASS_DEFINITIONS_SCHEMA_VERSION,
        source: SourceClassConstructionPolicy {
            allocation: span.clone(),
            parent_call: span.clone(),
            parent_index: span.clone(),
            wrap_constructor: span.clone(),
            parent_call_callback: ParserCallbackId(10),
            parent_index_callback: ParserCallbackId(11),
            parent_call_format_upvalue: 0,
            object_alias: "Object".into(),
            parent_init: "_parentInit".into(),
            proxy_parent: "_parent".into(),
            proxy_object: "_object".into(),
            proxy_class_name: "_className".into(),
            class_name_field: "_className".into(),
            parent_classes_field: "_parents".into(),
            super_parents_field: "_superParents".into(),
            unconstructed_meta_field: "_unconstructedMeta".into(),
            constructor_initialized_field: "_constructorInitialised".into(),
        },
        classes: vec![
            SourceClassDefinition {
                name: "Base".into(),
                table: ParserTableId(1),
                parents: vec![],
                super_parents: None,
                methods: base_methods.clone(),
                constructor: Some(SourceClassConstructor {
                    callback: ParserCallbackId(1),
                    wrapper: None,
                }),
                unsupported_fields: BTreeSet::new(),
            },
            SourceClassDefinition {
                name: "Child".into(),
                table: ParserTableId(2),
                parents: vec![SourceClassId(1)],
                super_parents: Some(vec![SourceClassId(1)]),
                methods: base_methods,
                constructor: Some(SourceClassConstructor {
                    callback: ParserCallbackId(2),
                    wrapper: Some(SourceClassConstructorWrapper {
                        callback: ParserCallbackId(3),
                        original_upvalue: 0,
                        class_upvalue: 1,
                        class_name_upvalue: 2,
                        pairs_upvalue: 3,
                    }),
                }),
                unsupported_fields: BTreeSet::from(["_superParents".into()]),
            },
        ],
    };
    let owner = SourceProgramOwner::new_with_classes(definitions, classes).unwrap();
    let bodies = vec![
        (
            1,
            vec![],
            vec![set("parent", l(1)), set("last", n(0.0)), ret(vec![l(0)])],
        ),
        (
            2,
            vec![ParserProgramBinding::DynamicMethod { key: "Base".into() }],
            vec![
                st(ParserProgramStatementKind::Call {
                    call: call(0, Some(l(0)), vec![l(1)]),
                }),
                ret(vec![l(0)]),
            ],
        ),
        (4, vec![], vec![set("last", l(1)), ret(vec![b("base")])]),
        (
            5,
            vec![],
            vec![
                set("Record", l(1)),
                set("mutated", n(1.0)),
                ret(vec![n(17.0)]),
            ],
        ),
        (
            6,
            vec![
                ParserProgramBinding::DynamicMethod {
                    key: "Record".into(),
                },
                ParserProgramBinding::CapturedCallback {
                    upvalue: 0,
                    callback: ParserCallbackId(5),
                },
            ],
            vec![ret(vec![e(ParserProgramExprKind::Call {
                call: Box::new(call(
                    0,
                    Some(l(0)),
                    vec![e(ParserProgramExprKind::Call {
                        call: Box::new(call(1, None, vec![l(0), l(1)])),
                    })],
                )),
            })])],
        ),
        (
            7,
            vec![],
            vec![set("through", n(23.0)), ret(vec![get(l(0), "last")])],
        ),
        (8, vec![], vec![ret(vec![b("override")])]),
        (
            9,
            vec![],
            vec![ret(vec![
                get(l(0), "last"),
                get(l(0), "mutated"),
                eq(get(l(0), "Object"), l(0)),
                eq(get(get(l(0), "Base"), "Object"), l(0)),
                get(l(0), "through"),
            ])],
        ),
        (12, vec![], vec![ret(vec![get(l(0), "Base")])]),
        (
            13,
            vec![ParserProgramBinding::Intrinsic {
                operation: ParserProgramIntrinsic::TableInsert,
                source: ParserProgramIntrinsicSource::OriginalGlobal,
            }],
            vec![
                st(ParserProgramStatementKind::Call {
                    call: call(0, None, vec![l(0), l(1)]),
                }),
                ret(vec![
                    e(ParserProgramExprKind::Get {
                        table: Box::new(l(0)),
                        key: Box::new(n(1.0)),
                    }),
                    e(ParserProgramExprKind::Get {
                        table: Box::new(get(l(0), "Object")),
                        key: Box::new(n(1.0)),
                    }),
                ]),
            ],
        ),
        (
            14,
            vec![ParserProgramBinding::Intrinsic {
                operation: ParserProgramIntrinsic::Ipairs,
                source: ParserProgramIntrinsicSource::OriginalGlobal,
            }],
            vec![
                st(ParserProgramStatementKind::ForEach {
                    locals: vec![2, 3],
                    iterator: ParserProgramIterator::Dense {
                        table: l(0),
                        binding: 0,
                    },
                    body: vec![ret(vec![l(3)])],
                }),
                ret(vec![b("empty")]),
            ],
        ),
        (
            15,
            vec![],
            vec![ret(vec![e(ParserProgramExprKind::Unary {
                operation: ParserProgramUnary::Length,
                value: Box::new(get(l(0), "_parentInit")),
            })])],
        ),
        (
            16,
            vec![],
            vec![
                st(ParserProgramStatementKind::TableSet {
                    table: l(0),
                    key: l(1),
                    value: l(2),
                }),
                ret(vec![]),
            ],
        ),
        (17, vec![], vec![ret(vec![get(l(0), "_superParents")])]),
    ];
    let programs = bodies
        .into_iter()
        .map(|(id, bindings, body)| ParserProgram {
            callback: ParserCallbackId(id),
            parameter_count: if id == 16 { 3 } else { 2 },
            local_count: 4,
            variadic: false,
            bindings,
            body,
            provenance: ParserProgramProvenance {
                source: span.clone(),
                function_start: 0,
                function_end: 128,
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
    let library =
        CompiledSourcePrograms::new(&SourceProgramCatalog::new(data, owner.clone()).unwrap())
            .unwrap();
    (owner, library)
}
fn args(session: &mut ProgramSession, values: Vec<ProgramValue>) -> Vec<SessionValue> {
    session
        .borrow(&ProgramValueGraph {
            values,
            tables: vec![],
        })
        .unwrap()
}
fn observe(session: &mut ProgramSession, object: &SessionValue) -> Vec<ProgramValue> {
    let result = session
        .invoke(ParserCallbackId(9), std::slice::from_ref(object))
        .unwrap();
    session.snapshot(&result).unwrap().graph().values.clone()
}
fn allocate(owner: &SourceProgramOwner, session: &mut ProgramSession) -> SessionValue {
    session
        .allocate_instance(&owner.bind_class(SourceClassId(2)).unwrap())
        .unwrap()
}
#[test]
fn constructed_instance_keeps_self_parent_proxy_aliases_and_initialization_checks() {
    let (owner, library) = fixture();
    let (mut session, _) = library
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    let object = allocate(&owner, &mut session);
    let result = session.invoke_method(&object, "Child", &[]).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(
        observe(&mut session, &object),
        vec![
            ProgramValue::Number(0.0),
            ProgramValue::Nil,
            ProgramValue::Boolean(true),
            ProgramValue::Boolean(true),
            ProgramValue::Nil
        ]
    );
    assert_eq!(
        session
            .invoke_method(&object, "Base", &[])
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::Source
    );
    assert_eq!(
        session
            .invoke_method(&object, "Child", &[])
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::Source
    );
    assert_eq!(
        session.snapshot(&[object]).unwrap_err().kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
}
#[test]
fn dynamic_lookup_keeps_target_before_argument_mutation_and_raw_nil_restores_inheritance() {
    let (owner, library) = fixture();
    let (mut session, _) = library
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    let object = allocate(&owner, &mut session);
    session.invoke_method(&object, "Child", &[]).unwrap();
    let replacement = args(
        &mut session,
        vec![ProgramValue::Callback(ParserCallbackId(8))],
    );
    let result = session
        .invoke(
            ParserCallbackId(6),
            &[object.clone(), replacement[0].clone()],
        )
        .unwrap();
    assert_eq!(
        session.snapshot(&result).unwrap().graph().values,
        vec![ProgramValue::Bytes(b"base".to_vec())]
    );
    assert_eq!(
        observe(&mut session, &object)[..2],
        [ProgramValue::Number(17.0), ProgramValue::Number(1.0)]
    );
    let result = session.invoke_method(&object, "Record", &[]).unwrap();
    assert_eq!(
        session.snapshot(&result).unwrap().graph().values,
        vec![ProgramValue::Bytes(b"override".to_vec())]
    );
    let nil = args(&mut session, vec![ProgramValue::Nil]);
    session
        .invoke(ParserCallbackId(5), &[object.clone(), nil[0].clone()])
        .unwrap();
    let result = session.invoke_method(&object, "Record", &[]).unwrap();
    assert_eq!(
        session.snapshot(&result).unwrap().graph().values,
        vec![ProgramValue::Bytes(b"base".to_vec())]
    );
}
#[test]
fn failing_noncallable_method_still_evaluates_arguments_and_retains_writes() {
    let (owner, library) = fixture();
    let (mut session, _) = library
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    let object = allocate(&owner, &mut session);
    session.invoke_method(&object, "Child", &[]).unwrap();
    let disabled = args(&mut session, vec![ProgramValue::Boolean(false)]);
    session
        .invoke(ParserCallbackId(5), &[object.clone(), disabled[0].clone()])
        .unwrap();
    let replacement = args(
        &mut session,
        vec![ProgramValue::Callback(ParserCallbackId(8))],
    );
    let before = session.steps();
    let error = session
        .invoke(
            ParserCallbackId(6),
            &[object.clone(), replacement[0].clone()],
        )
        .unwrap_err();
    assert_eq!(error.kind, ProgramRuntimeErrorKind::Source);
    assert!(session.steps() > before);
    let result = session.invoke_method(&object, "Record", &[]).unwrap();
    assert_eq!(
        session.snapshot(&result).unwrap().graph().values,
        vec![ProgramValue::Bytes(b"override".to_vec())]
    );
}
#[test]
fn class_and_value_handles_cannot_cross_owner_or_session_and_budgets_are_cumulative() {
    let (owner, library) = fixture();
    let (other, _) = fixture();
    let (mut session, _) = library
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    assert_eq!(
        session
            .allocate_instance(&other.bind_class(SourceClassId(2)).unwrap())
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::InvalidInput
    );
    let object = allocate(&owner, &mut session);
    let (mut second, _) = library
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    assert_eq!(
        second
            .invoke_method(&object, "Record", &[])
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::InvalidInput
    );
    let (mut limited, _) = library
        .session(
            &ProgramValueGraph::default(),
            ProgramLimits {
                max_tables: 3,
                ..ProgramLimits::default()
            },
        )
        .unwrap();
    allocate(&owner, &mut limited);
    assert_eq!(
        limited
            .allocate_instance(&owner.bind_class(SourceClassId(2)).unwrap())
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
    assert_eq!(limited.allocations().tables, 3);
}

#[test]
fn parent_proxy_reads_raw_object_and_forwards_missing_writes_without_erasing_identity() {
    let (owner, library) = fixture();
    let (mut session, _) = library
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    let object = allocate(&owner, &mut session);
    session.invoke_method(&object, "Child", &[]).unwrap();
    let proxy = session
        .invoke(ParserCallbackId(12), std::slice::from_ref(&object))
        .unwrap()
        .remove(0);
    let result = session
        .invoke(ParserCallbackId(7), std::slice::from_ref(&proxy))
        .unwrap();
    assert_eq!(
        session.snapshot(&result).unwrap().graph().values,
        vec![ProgramValue::Number(0.0)]
    );
    assert_eq!(
        observe(&mut session, &object)[4],
        ProgramValue::Number(23.0)
    );
    assert_eq!(
        session
            .snapshot(std::slice::from_ref(&proxy))
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    // Inherited method receives the proxy; writes are forwarded through __newindex.
    let arguments = args(&mut session, vec![ProgramValue::Number(29.0)]);
    session.invoke_method(&proxy, "Record", &arguments).unwrap();
    assert_eq!(
        observe(&mut session, &object)[0],
        ProgramValue::Number(29.0)
    );
}

#[test]
fn native_array_primitives_bypass_proxy_index_and_newindex_metamethods() {
    let (owner, library) = fixture();
    let (mut session, _) = library
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    let object = allocate(&owner, &mut session);
    session.invoke_method(&object, "Child", &[]).unwrap();
    let proxy = session
        .invoke(ParserCallbackId(12), std::slice::from_ref(&object))
        .unwrap()
        .remove(0);
    let input = args(&mut session, vec![ProgramValue::Number(31.0)]);
    session
        .invoke(ParserCallbackId(13), &[object.clone(), input[0].clone()])
        .unwrap();
    let result = session
        .invoke(ParserCallbackId(14), std::slice::from_ref(&proxy))
        .unwrap();
    assert_eq!(
        session.snapshot(&result).unwrap().graph().values,
        vec![ProgramValue::Bytes(b"empty".to_vec())]
    );
    let input = args(&mut session, vec![ProgramValue::Number(37.0)]);
    let result = session
        .invoke(ParserCallbackId(13), &[proxy.clone(), input[0].clone()])
        .unwrap();
    assert_eq!(
        session.snapshot(&result).unwrap().graph().values,
        vec![ProgramValue::Number(37.0), ProgramValue::Number(31.0)]
    );
    let result = session.invoke(ParserCallbackId(14), &[proxy]).unwrap();
    assert_eq!(
        session.snapshot(&result).unwrap().graph().values,
        vec![ProgramValue::Number(37.0)]
    );
}

#[test]
fn unmodeled_class_metamethods_and_traversal_allocation_are_explicit_frontiers() {
    let (owner, library) = fixture();
    let mut classes = owner.classes().unwrap().clone();
    classes.classes[1].unsupported_fields.insert("__eq".into());
    let owner = SourceProgramOwner::new_with_classes(owner.definitions().unwrap().clone(), classes)
        .unwrap();
    let library = CompiledSourcePrograms::new(
        &SourceProgramCatalog::new(library.catalog().data().clone(), owner.clone()).unwrap(),
    )
    .unwrap();
    let (mut session, _) = library
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    assert_eq!(
        session
            .allocate_instance(&owner.bind_class(SourceClassId(2)).unwrap())
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    assert_eq!(session.allocations().tables, 0);
    let (owner, library) = fixture();
    let (mut session, _) = library
        .session(
            &ProgramValueGraph::default(),
            ProgramLimits {
                max_values: 2,
                ..ProgramLimits::default()
            },
        )
        .unwrap();
    assert_eq!(
        session
            .allocate_instance(&owner.bind_class(SourceClassId(2)).unwrap())
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::ResourceBound
    );
    assert_eq!(session.allocations().tables, 0);
}

#[test]
fn constructor_prototype_guards_and_empty_present_parent_table_preserve_source_state() {
    let (owner, library) = fixture();
    let mut definitions = owner.definitions().unwrap().clone();
    let mut classes = owner.classes().unwrap().clone();
    classes.classes[1].parents.clear();
    classes.classes[1].super_parents = Some(vec![]);
    classes.classes[1]
        .methods
        .get_mut("Record")
        .unwrap()
        .declared_by = SourceClassId(2);
    definitions.tables[2].indexed.clear();
    let empty_owner = SourceProgramOwner::new_with_classes(definitions, classes).unwrap();
    let empty_library = CompiledSourcePrograms::new(
        &SourceProgramCatalog::new(library.catalog().data().clone(), empty_owner.clone()).unwrap(),
    )
    .unwrap();
    let (mut session, _) = empty_library
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    let object = allocate(&empty_owner, &mut session);
    let result = session.invoke(ParserCallbackId(15), &[object]).unwrap();
    assert_eq!(
        session.snapshot(&result).unwrap().graph().values,
        vec![ProgramValue::Number(0.0)]
    );
    for (key, value) in [
        ("__index", None),
        ("_unconstructedMeta", None),
        ("_unconstructedMeta", Some(ParserValue::Boolean(false))),
        ("__index", Some(ParserValue::Table(ParserTableId(3)))),
        (
            "_unconstructedMeta",
            Some(ParserValue::Table(ParserTableId(3))),
        ),
        ("_constructorInitialised", Some(ParserValue::Boolean(false))),
    ] {
        let mut definitions = owner.definitions().unwrap().clone();
        if let Some(value) = value {
            definitions.tables[1].fields.insert(key.into(), value);
        } else {
            definitions.tables[1].fields.remove(key);
        }
        let owner =
            SourceProgramOwner::new_with_classes(definitions, owner.classes().unwrap().clone())
                .unwrap();
        let library = CompiledSourcePrograms::new(
            &SourceProgramCatalog::new(library.catalog().data().clone(), owner.clone()).unwrap(),
        )
        .unwrap();
        let (mut session, _) = library
            .session(&ProgramValueGraph::default(), ProgramLimits::default())
            .unwrap();
        assert_eq!(
            session
                .allocate_instance(&owner.bind_class(SourceClassId(2)).unwrap())
                .unwrap_err()
                .kind,
            ProgramRuntimeErrorKind::UnsupportedCapability,
            "{key}"
        );
    }
}

#[test]
fn proxy_mutations_preserve_raw_omission_and_do_not_enable_unsupported_metabehavior() {
    let (owner, library) = fixture();
    let (mut session, _) = library
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    let object = allocate(&owner, &mut session);
    let proxy = session
        .invoke(ParserCallbackId(12), std::slice::from_ref(&object))
        .unwrap()
        .remove(0);
    let key = args(&mut session, vec![ProgramValue::Bytes(b"_object".to_vec())]).remove(0);
    let definition = session
        .definition(
            &owner
                .bind_root(SourceProgramDefinitionRoot::Named(SourceProgramRootId(1)))
                .unwrap(),
        )
        .unwrap();
    session
        .invoke(ParserCallbackId(16), &[proxy.clone(), key, definition])
        .unwrap();
    assert_eq!(
        session
            .invoke(ParserCallbackId(17), std::slice::from_ref(&proxy))
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    // __call must itself be a function, not another callable table.
    let key = args(&mut session, vec![ProgramValue::Bytes(b"__call".to_vec())]).remove(0);
    session
        .invoke(ParserCallbackId(16), &[proxy.clone(), key, proxy.clone()])
        .unwrap();
    assert_eq!(
        session
            .invoke_method(&object, "Base", &[])
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::Source
    );
    // Disable __newindex so the next field would be installed on the proxy itself.
    let values = args(
        &mut session,
        vec![
            ProgramValue::Bytes(b"__newindex".to_vec()),
            ProgramValue::Nil,
        ],
    );
    session
        .invoke(
            ParserCallbackId(16),
            &[proxy.clone(), values[0].clone(), values[1].clone()],
        )
        .unwrap();
    let values = args(
        &mut session,
        vec![
            ProgramValue::Bytes(b"__eq".to_vec()),
            ProgramValue::Callback(ParserCallbackId(8)),
        ],
    );
    assert_eq!(
        session
            .invoke(
                ParserCallbackId(16),
                &[proxy, values[0].clone(), values[1].clone()]
            )
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
}

#[path = "support/source_program_instances.rs"]
mod imported_instances;
