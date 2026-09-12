use poe_optimizer_data::{
    item_loading::{ItemLoadingSource, ItemSourceSpan},
    source_program::*,
};
use std::collections::{BTreeMap, BTreeSet};
fn span(line: u32, end_line: u32) -> ItemSourceSpan {
    ItemSourceSpan {
        path: "fixtures/classes.lua".into(),
        line,
        end_line,
        sha256: "b".repeat(64),
    }
}
fn callback(source: ItemSourceSpan) -> SourceCallback {
    SourceCallback {
        kind: SourceCallbackKind::Lua { source },
        upvalues: vec![],
        environment: SourceEnvironment::OriginalGlobals,
    }
}
fn definitions() -> (SourceProgramDefinitions, SourceClassDefinitions) {
    let mut callbacks = vec![
        callback(span(1, 5)),
        callback(span(6, 10)),
        callback(span(11, 15)),
        callback(span(16, 20)),
        callback(span(22, 28)),
    ];
    callbacks[4].upvalues = vec![
        SourceUpvalue {
            name: "original".into(),
            value: SourceValue::Callback(SourceCallbackId(4)),
        },
        SourceUpvalue {
            name: "class".into(),
            value: SourceValue::Table(SourceTableId(2)),
        },
        SourceUpvalue {
            name: "name".into(),
            value: SourceValue::Text("Child".into()),
        },
    ];
    callbacks.push(SourceCallback {
        kind: SourceCallbackKind::Builtin {
            symbol: "pairs".into(),
        },
        upvalues: vec![],
        environment: SourceEnvironment::OriginalGlobals,
    });
    callbacks.push(SourceCallback {
        kind: SourceCallbackKind::Builtin {
            symbol: "string.format".into(),
        },
        upvalues: vec![],
        environment: SourceEnvironment::OriginalGlobals,
    });
    callbacks[4].upvalues.push(SourceUpvalue {
        name: "pairs".into(),
        value: SourceValue::Callback(SourceCallbackId(6)),
    });
    callbacks[0].upvalues.push(SourceUpvalue {
        name: "s_format".into(),
        value: SourceValue::Callback(SourceCallbackId(7)),
    });
    let mut data = SourceProgramDefinitions {
        schema_version: SOURCE_PROGRAM_DEFINITIONS_SCHEMA_VERSION,
        source: ItemLoadingSource {
            upstream_revision: "a".repeat(40),
            files: BTreeMap::from([("fixtures/classes.lua".into(), "c".repeat(64))]),
            construction_spans: BTreeMap::new(),
            module_order: vec!["fixtures/classes.lua".into()],
        },
        tables: vec![
            SourceTable {
                fields: BTreeMap::from([
                    ("method".into(), SourceValue::Callback(SourceCallbackId(3))),
                    ("__index".into(), SourceValue::Table(SourceTableId(1))),
                ]),
                indexed: BTreeMap::new(),
            },
            SourceTable {
                fields: BTreeMap::from([
                    ("method".into(), SourceValue::Callback(SourceCallbackId(3))),
                    ("Child".into(), SourceValue::Callback(SourceCallbackId(5))),
                    ("__index".into(), SourceValue::Table(SourceTableId(2))),
                ]),
                indexed: BTreeMap::new(),
            },
        ],
        callbacks,
        roots: vec![
            SourceProgramRoot {
                name: "class".into(),
                table: SourceTableId(2),
            },
            SourceProgramRoot {
                name: "class_alias".into(),
                table: SourceTableId(2),
            },
        ],
        intrinsics: BTreeMap::new(),
    };
    let classes = SourceClassDefinitions {
        schema_version: SOURCE_CLASS_DEFINITIONS_SCHEMA_VERSION,
        source: SourceClassConstructionPolicy {
            allocation: span(30, 40),
            parent_call: span(1, 5),
            parent_index: span(6, 10),
            wrap_constructor: span(21, 29),
            parent_call_callback: SourceCallbackId(1),
            parent_call_format_upvalue: 0,
            parent_index_callback: SourceCallbackId(2),
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
                table: SourceTableId(1),
                parents: vec![],
                super_parents: None,
                unsupported_fields: BTreeSet::new(),
                methods: BTreeMap::from([(
                    "method".into(),
                    SourceClassMethod {
                        callback: SourceCallbackId(3),
                        declared_by: SourceClassId(1),
                    },
                )]),
                constructor: None,
            },
            SourceClassDefinition {
                name: "Child".into(),
                table: SourceTableId(2),
                parents: vec![SourceClassId(1)],
                super_parents: Some(vec![SourceClassId(1)]),
                unsupported_fields: BTreeSet::from(["_superParents".into()]),
                methods: BTreeMap::from([(
                    "method".into(),
                    SourceClassMethod {
                        callback: SourceCallbackId(3),
                        declared_by: SourceClassId(1),
                    },
                )]),
                constructor: Some(SourceClassConstructor {
                    callback: SourceCallbackId(4),
                    wrapper: Some(SourceClassConstructorWrapper {
                        callback: SourceCallbackId(5),
                        original_upvalue: 0,
                        class_upvalue: 1,
                        class_name_upvalue: 2,
                        pairs_upvalue: 3,
                    }),
                }),
            },
        ],
    };
    data.tables[0]
        .fields
        .insert("_className".into(), SourceValue::Text("Base".into()));
    data.tables[1]
        .fields
        .insert("_className".into(), SourceValue::Text("Child".into()));
    data.tables[1]
        .fields
        .insert("_parents".into(), SourceValue::Table(SourceTableId(3)));
    data.tables.push(SourceTable {
        fields: BTreeMap::new(),
        indexed: BTreeMap::from([(1, SourceValue::Table(SourceTableId(1)))]),
    });
    (data, classes)
}
#[test]
fn retains_actual_copied_inheritance_class_aliases_and_bound_identity() {
    let (data, classes) = definitions();
    let owner = SourceProgramOwner::new_with_classes(data.clone(), classes.clone()).unwrap();
    assert_eq!(owner.classes(), Some(&classes));
    assert_eq!(owner.class_id("Child"), Some(SourceClassId(2)));
    assert_eq!(owner.class_id("unknown"), None);
    let handle = owner.bind_class(SourceClassId(2)).unwrap();
    assert!(std::ptr::eq(
        owner.resolve_class(&handle).unwrap(),
        owner.clone().resolve_class(&handle).unwrap()
    ));
    assert!(std::ptr::eq(
        handle.definition(),
        owner.class(handle.id()).unwrap()
    ));
    assert!(handle.owner().is_same_owner(&owner));
    let class = owner
        .bind_root(SourceProgramDefinitionRoot::Named(SourceProgramRootId(1)))
        .unwrap();
    let alias = owner
        .bind_root(SourceProgramDefinitionRoot::Named(SourceProgramRootId(2)))
        .unwrap();
    assert!(std::ptr::eq(class.table(), alias.table()));
    assert_eq!(handle.definition().table, class.table_id());
    let foreign = SourceProgramOwner::new_with_classes(data, classes).unwrap();
    assert_eq!(
        foreign.resolve_class(&handle).unwrap_err().kind,
        SourceProgramErrorKind::Binding
    );
    assert!(owner.bind_class(SourceClassId(0)).is_err());
    assert!(owner.bind_class(SourceClassId(3)).is_err());
}
#[test]
fn rejects_inconsistent_callbacks_overrides_origins_and_projections() {
    type Mutation = fn(&mut SourceProgramDefinitions, &mut SourceClassDefinitions);
    let variants: Vec<Mutation> = vec![
        |_, c| c.classes[1].methods.get_mut("method").unwrap().callback = SourceCallbackId(4),
        |d, _| {
            d.tables[1]
                .fields
                .insert("method".into(), SourceValue::Callback(SourceCallbackId(4)));
        },
        |_, c| c.classes[0].methods.get_mut("method").unwrap().declared_by = SourceClassId(2),
        |_, c| c.classes[1].methods.get_mut("method").unwrap().declared_by = SourceClassId(3),
        |_, c| {
            c.classes[1].unsupported_fields.insert("method".into());
        },
        |_, c| c.classes[1].table = SourceTableId(1),
        |_, c| c.classes[1].name = "Base".into(),
        |_, c| c.classes[0].table = SourceTableId(999),
        |_, c| c.classes[1].parents.push(SourceClassId(1)),
        |_, c| c.classes[1].parents.push(SourceClassId(999)),
        |_, c| c.classes[0].parents.push(SourceClassId(2)),
        |_, c| c.source.proxy_object = c.source.object_alias.clone(),
        |_, c| c.source.proxy_parent = "__index".into(),
        |_, c| c.source.parent_index_callback = SourceCallbackId(3),
    ];
    for (i, mutate) in variants.into_iter().enumerate() {
        let (mut d, mut c) = definitions();
        mutate(&mut d, &mut c);
        assert!(
            SourceProgramOwner::new_with_classes(d, c).is_err(),
            "variant {i}"
        );
    }
    // A source-declared local override is structurally legitimate when its
    // callback exactly matches the class table; this is not parity admission.
    let (mut d, mut c) = definitions();
    d.tables[1]
        .fields
        .insert("method".into(), SourceValue::Callback(SourceCallbackId(4)));
    c.classes[1].methods.insert(
        "method".into(),
        SourceClassMethod {
            callback: SourceCallbackId(4),
            declared_by: SourceClassId(2),
        },
    );
    SourceProgramOwner::new_with_classes(d, c).unwrap();
}
#[test]
fn first_parent_raw_field_blocks_later_parent_callback_copy() {
    let (mut d, mut c) = definitions();
    d.tables.push(SourceTable {
        fields: BTreeMap::from([("method".into(), SourceValue::Boolean(false))]),
        indexed: BTreeMap::new(),
    });
    c.classes.push(SourceClassDefinition {
        name: "Blocker".into(),
        table: SourceTableId(4),
        parents: vec![],
        super_parents: None,
        unsupported_fields: BTreeSet::new(),
        methods: BTreeMap::new(),
        constructor: None,
    });
    d.tables[3]
        .fields
        .insert("_className".into(), SourceValue::Text("Blocker".into()));
    d.tables[2].indexed = BTreeMap::from([
        (1, SourceValue::Table(SourceTableId(4))),
        (2, SourceValue::Table(SourceTableId(1))),
    ]);
    c.classes[1].parents = vec![SourceClassId(3), SourceClassId(1)];
    assert!(SourceProgramOwner::new_with_classes(d, c).is_err());
}
#[test]
fn constructor_wrapper_requires_actual_original_class_and_name_captures() {
    type Mutation = fn(&mut SourceProgramDefinitions, &mut SourceClassDefinitions);
    let variants: Vec<Mutation> = vec![
        |d, _| d.callbacks[4].upvalues.clear(),
        |d, _| d.callbacks[4].upvalues[0].value = SourceValue::Callback(SourceCallbackId(3)),
        |d, _| d.callbacks[4].upvalues[1].value = SourceValue::Table(SourceTableId(1)),
        |d, _| d.callbacks[4].upvalues[2].value = SourceValue::Text("Base".into()),
        |_, c| {
            c.classes[1]
                .constructor
                .as_mut()
                .unwrap()
                .wrapper
                .as_mut()
                .unwrap()
                .original_upvalue = 99
        },
        |_, c| c.source.wrap_constructor = span(50, 60),
        |d, _| {
            d.tables[1]
                .fields
                .insert("Child".into(), SourceValue::Callback(SourceCallbackId(4)));
        },
    ];
    for (i, mutate) in variants.into_iter().enumerate() {
        let (mut d, mut c) = definitions();
        mutate(&mut d, &mut c);
        assert!(
            SourceProgramOwner::new_with_classes(d, c).is_err(),
            "variant {i}"
        );
    }
}
#[test]
fn class_json_is_bounded_and_duplicate_method_entries_are_rejected() {
    let (d, c) = definitions();
    let bytes = serde_json::to_vec(&c).unwrap();
    assert_eq!(SourceClassDefinitions::from_bytes(&bytes, &d).unwrap(), c);
    let mut missing = serde_json::to_value(&c).unwrap();
    missing["classes"][0]
        .as_object_mut()
        .unwrap()
        .remove("constructor");
    assert!(
        SourceClassDefinitions::from_bytes(&serde_json::to_vec(&missing).unwrap(), &d).is_err()
    );
    let mut missing = serde_json::to_value(&c).unwrap();
    missing["classes"][1]["constructor"]
        .as_object_mut()
        .unwrap()
        .remove("wrapper");
    assert!(
        SourceClassDefinitions::from_bytes(&serde_json::to_vec(&missing).unwrap(), &d).is_err()
    );
    let text=String::from_utf8(bytes).unwrap().replace("\"methods\":{\"method\":{\"callback\":3,\"declared_by\":1}}", "\"methods\":{\"method\":{\"callback\":3,\"declared_by\":1},\"method\":{\"callback\":3,\"declared_by\":1}}");
    assert!(SourceClassDefinitions::from_bytes(text.as_bytes(), &d).is_err());
    assert_eq!(
        SourceClassDefinitions::from_bytes(&vec![b' '; 16 * 1024 * 1024 + 1], &d)
            .unwrap_err()
            .kind,
        SourceProgramErrorKind::ResourceLimit
    );
    let (mut d, mut c) = definitions();
    for i in 2..65 {
        let table = SourceTableId(d.tables.len() as u32 + 1);
        let parents = SourceTableId(table.0 + 1);
        d.tables.push(SourceTable {
            fields: BTreeMap::from([
                ("_className".into(), SourceValue::Text(format!("Depth{i}"))),
                ("_parents".into(), SourceValue::Table(parents)),
            ]),
            indexed: BTreeMap::new(),
        });
        d.tables.push(SourceTable {
            fields: BTreeMap::new(),
            indexed: BTreeMap::from([(1, SourceValue::Table(c.classes[i as usize - 1].table))]),
        });
        c.classes.push(SourceClassDefinition {
            name: format!("Depth{i}"),
            table,
            parents: vec![SourceClassId(i)],
            super_parents: Some((1..=i).map(SourceClassId).collect()),
            unsupported_fields: BTreeSet::from(["_superParents".into()]),
            methods: BTreeMap::new(),
            constructor: None,
        });
    }
    assert_eq!(
        SourceProgramOwner::new_with_classes(d, c).unwrap_err().kind,
        SourceProgramErrorKind::ResourceLimit
    );
}

#[test]
fn inherited_constructor_keeps_original_identity_after_parent_was_wrapped() {
    let (mut d, mut c) = definitions();
    // Child is a constructed parent whose source wrapper was installed after
    // Leaf copied its original constructor. There is no dynamic rebinding.
    d.tables.push(SourceTable {
        fields: BTreeMap::from([
            ("_className".into(), SourceValue::Text("Leaf".into())),
            ("_parents".into(), SourceValue::Table(SourceTableId(5))),
            ("Child".into(), SourceValue::Callback(SourceCallbackId(4))),
        ]),
        indexed: BTreeMap::new(),
    });
    d.tables.push(SourceTable {
        fields: BTreeMap::new(),
        indexed: BTreeMap::from([(1, SourceValue::Table(SourceTableId(2)))]),
    });
    c.classes[1].methods.insert(
        "Child".into(),
        SourceClassMethod {
            callback: SourceCallbackId(5),
            declared_by: SourceClassId(2),
        },
    );
    c.classes.push(SourceClassDefinition {
        name: "Leaf".into(),
        table: SourceTableId(4),
        parents: vec![SourceClassId(2)],
        super_parents: Some(vec![SourceClassId(2), SourceClassId(1)]),
        unsupported_fields: BTreeSet::from(["_superParents".into()]),
        methods: BTreeMap::from([(
            "Child".into(),
            SourceClassMethod {
                callback: SourceCallbackId(4),
                declared_by: SourceClassId(2),
            },
        )]),
        constructor: None,
    });
    SourceProgramOwner::new_with_classes(d.clone(), c.clone()).unwrap();
    // An unexplained later mutation is an explicit unsupported frontier.
    d.tables[3]
        .fields
        .insert("Child".into(), SourceValue::Callback(SourceCallbackId(3)));
    c.classes[2].methods.get_mut("Child").unwrap().callback = SourceCallbackId(3);
    assert_eq!(
        SourceProgramOwner::new_with_classes(d, c).unwrap_err().kind,
        SourceProgramErrorKind::UnsupportedCapability
    );
}
#[test]
fn class_metadata_names_are_injected_and_observed_parent_order_is_binding() {
    let (mut d, mut c) = definitions();
    c.source.object_alias = "Self".into();
    c.source.parent_init = "Initialized".into();
    c.source.proxy_parent = "BaseClass".into();
    c.source.proxy_object = "Instance".into();
    c.source.proxy_class_name = "ProxyClass".into();
    c.source.class_name_field = "ClassName".into();
    c.source.parent_classes_field = "Parents".into();
    for table in &mut d.tables {
        if let Some(value) = table.fields.remove("_className") {
            table.fields.insert("ClassName".into(), value);
        }
        if let Some(value) = table.fields.remove("_parents") {
            table.fields.insert("Parents".into(), value);
        }
    }
    SourceProgramOwner::new_with_classes(d.clone(), c.clone()).unwrap();
    d.tables[2]
        .indexed
        .insert(1, SourceValue::Table(SourceTableId(2)));
    assert_eq!(
        SourceProgramOwner::new_with_classes(d, c).unwrap_err().kind,
        SourceProgramErrorKind::Binding
    );
}
#[test]
fn source_class_aggregate_method_budget_is_independent_of_each_class_limit() {
    let (mut d, mut c) = definitions();
    d.tables.clear();
    d.roots.clear();
    d.callbacks[4].upvalues.clear();
    c.classes.clear();
    for i in 1..=9 {
        let methods: BTreeMap<_, _> = (0..4096)
            .map(|n| {
                (
                    format!("method{n}"),
                    SourceClassMethod {
                        callback: SourceCallbackId(3),
                        declared_by: SourceClassId(i),
                    },
                )
            })
            .collect();
        let mut fields: BTreeMap<_, _> = methods
            .iter()
            .map(|(key, value)| (key.clone(), SourceValue::Callback(value.callback)))
            .collect();
        fields.insert("_className".into(), SourceValue::Text(format!("Class{i}")));
        d.tables.push(SourceTable {
            fields,
            indexed: BTreeMap::new(),
        });
        c.classes.push(SourceClassDefinition {
            name: format!("Class{i}"),
            table: SourceTableId(i),
            parents: vec![],
            super_parents: None,
            unsupported_fields: BTreeSet::new(),
            methods,
            constructor: None,
        });
    }
    let error = SourceProgramOwner::new_with_classes(d, c).unwrap_err();
    assert_eq!(error.kind, SourceProgramErrorKind::ResourceLimit);
    assert!(error.message.contains("aggregate source class method"));
}

#[test]
fn constructor_absence_cannot_hide_an_observed_or_unknown_constructor() {
    let (d, mut c) = definitions();
    c.classes[1].constructor = None;
    assert_eq!(
        SourceProgramOwner::new_with_classes(d, c).unwrap_err().kind,
        SourceProgramErrorKind::Binding
    );
    let (d, mut c) = definitions();
    c.classes[0].unsupported_fields.insert("Base".into());
    assert_eq!(
        SourceProgramOwner::new_with_classes(d, c).unwrap_err().kind,
        SourceProgramErrorKind::UnsupportedCapability
    );
    let (mut d, c) = definitions();
    d.tables[0]
        .fields
        .insert("Base".into(), SourceValue::Boolean(false));
    SourceProgramOwner::new_with_classes(d.clone(), c.clone()).unwrap();
    d.tables[0]
        .fields
        .insert("Base".into(), SourceValue::Boolean(true));
    assert_eq!(
        SourceProgramOwner::new_with_classes(d, c).unwrap_err().kind,
        SourceProgramErrorKind::UnsupportedCapability
    );
}

#[test]
fn repeated_direct_parents_preserve_source_order_without_duplicate_class_identity() {
    let (mut d, mut c) = definitions();
    c.classes[1].parents.push(SourceClassId(1));
    d.tables[2]
        .indexed
        .insert(2, SourceValue::Table(SourceTableId(1)));
    let owner = SourceProgramOwner::new_with_classes(d, c).unwrap();
    assert_eq!(
        owner.class(SourceClassId(2)).unwrap().parents,
        vec![SourceClassId(1), SourceClassId(1)]
    );
}

#[test]
fn observed_superclass_set_preserves_order_presence_and_validates_membership() {
    let (mut d, mut c) = definitions();
    d.tables.push(SourceTable {
        fields: BTreeMap::from([("_className".into(), SourceValue::Text("Other".into()))]),
        indexed: BTreeMap::new(),
    });
    c.classes.push(SourceClassDefinition {
        name: "Other".into(),
        table: SourceTableId(4),
        parents: vec![],
        super_parents: None,
        unsupported_fields: BTreeSet::new(),
        methods: BTreeMap::new(),
        constructor: None,
    });
    c.classes[1].parents.push(SourceClassId(3));
    d.tables[2]
        .indexed
        .insert(2, SourceValue::Table(SourceTableId(4)));
    c.classes[1].super_parents = Some(vec![SourceClassId(3), SourceClassId(1)]);
    let owner = SourceProgramOwner::new_with_classes(d.clone(), c.clone()).unwrap();
    assert_eq!(
        owner.class(SourceClassId(2)).unwrap().super_parents,
        Some(vec![SourceClassId(3), SourceClassId(1)])
    );
    for (set, kind) in [
        (
            Some(vec![SourceClassId(1), SourceClassId(1)]),
            SourceProgramErrorKind::Binding,
        ),
        (
            Some(vec![SourceClassId(1)]),
            SourceProgramErrorKind::UnsupportedCapability,
        ),
        (None, SourceProgramErrorKind::Binding),
        (
            Some(vec![SourceClassId(999)]),
            SourceProgramErrorKind::Binding,
        ),
    ] {
        let mut changed = c.clone();
        changed.classes[1].super_parents = set;
        assert_eq!(
            SourceProgramOwner::new_with_classes(d.clone(), changed)
                .unwrap_err()
                .kind,
            kind
        );
    }
    // Empty source tables are truthy and must retain presence independently
    // of the zero-sized ancestry vector.
    let (mut d, mut c) = definitions();
    d.tables[0]
        .fields
        .insert("_parents".into(), SourceValue::Table(SourceTableId(4)));
    d.tables[0]
        .fields
        .insert("_superParents".into(), SourceValue::Table(SourceTableId(5)));
    d.tables
        .extend([SourceTable::default(), SourceTable::default()]);
    c.classes[0].super_parents = Some(vec![]);
    let owner = SourceProgramOwner::new_with_classes(d, c).unwrap();
    assert_eq!(
        owner.class(SourceClassId(1)).unwrap().super_parents,
        Some(vec![])
    );
}
#[test]
fn modeled_parent_and_constructor_protocols_bind_the_complete_capture_shape() {
    type Mutation = fn(&mut SourceProgramDefinitions, &mut SourceClassDefinitions);
    let variants: Vec<Mutation> = vec![
        |d, _| {
            d.callbacks[0].upvalues.push(SourceUpvalue {
                name: "extra".into(),
                value: SourceValue::Nil,
            })
        },
        |d, _| {
            d.callbacks[1].upvalues.push(SourceUpvalue {
                name: "extra".into(),
                value: SourceValue::Nil,
            })
        },
        |d, _| {
            d.callbacks[4].upvalues.push(SourceUpvalue {
                name: "extra".into(),
                value: SourceValue::Nil,
            })
        },
        |d, _| d.callbacks[0].upvalues[0].value = SourceValue::Callback(SourceCallbackId(6)),
        |d, _| d.callbacks[4].upvalues[3].value = SourceValue::Callback(SourceCallbackId(7)),
        |_, c| c.source.parent_call_format_upvalue = 1,
        |_, c| {
            c.classes[1]
                .constructor
                .as_mut()
                .unwrap()
                .wrapper
                .as_mut()
                .unwrap()
                .pairs_upvalue = 2
        },
        |_, c| c.source.unconstructed_meta_field = c.source.parent_classes_field.clone(),
    ];
    for (index, change) in variants.into_iter().enumerate() {
        let (mut d, mut c) = definitions();
        change(&mut d, &mut c);
        assert!(
            SourceProgramOwner::new_with_classes(d, c).is_err(),
            "variant {index}"
        );
    }
}

#[test]
fn context_coexists_with_classes_but_cannot_replace_class_projection_semantics() {
    let (data, classes) = definitions();
    let context = SourceProgramContext {
        schema_version: SOURCE_PROGRAM_CONTEXT_SCHEMA_VERSION,
        iteration: None,
        environment: Some(SourceProgramRootId(1)),
        tables: BTreeMap::from([(
            SourceTableId(3),
            SourceTableCoverage {
                inventory: SourceTableInventory::Complete,
                known_absent: BTreeSet::new(),
                unavailable: BTreeSet::new(),
                index_fallback: SourceTableIndexFallback::Nil,
                call_fallback: SourceTableCallFallback::NonCallable,
            },
        )]),
    };
    let owner =
        SourceProgramOwner::new_with_context(data.clone(), Some(classes.clone()), context.clone())
            .unwrap();
    assert_eq!(owner.classes(), Some(&classes));
    assert_eq!(owner.context(), Some(&context));
    let legacy = SourceProgramOwner::new_with_classes(data.clone(), classes.clone()).unwrap();
    assert!(legacy.context().is_none());
    assert!(!legacy.is_same_owner(&owner));
    let handle = legacy.bind_class(SourceClassId(1)).unwrap();
    assert_eq!(
        owner.resolve_class(&handle).unwrap_err().kind,
        SourceProgramErrorKind::Binding
    );
    for id in [SourceTableId(1), SourceTableId(2)] {
        let mut bad = context.clone();
        bad.tables
            .insert(id, context.tables[&SourceTableId(3)].clone());
        assert!(
            SourceProgramOwner::new_with_context(data.clone(), Some(classes.clone()), bad)
                .unwrap_err()
                .message
                .contains("overlaps a source class projection")
        );
    }
}

fn session_with_class() -> SourceSessionInput {
    let (data, classes) = definitions();
    let owner = SourceProgramOwner::new_with_classes(data, classes).unwrap();
    let class = owner.bind_class(SourceClassId(2)).unwrap();
    SourceSessionInput {
        owner,
        traversal: None,
        state: SourceSessionValueGraph {
            values: vec![
                SourceSessionValue::Table(SourceSessionTableId(1)),
                SourceSessionValue::Table(SourceSessionTableId(1)),
            ],
            tables: vec![SourceSessionTable {
                entries: vec![(
                    SourceSessionValue::Bytes(b"Object".to_vec()),
                    SourceSessionValue::Table(SourceSessionTableId(1)),
                )],
            }],
        },
        coverage: BTreeMap::from([(
            SourceSessionTableId(1),
            SourceTableCoverage {
                inventory: SourceTableInventory::Complete,
                known_absent: BTreeSet::new(),
                unavailable: BTreeSet::from([SourceTableKey::Text("uncaptured".into())]),
                index_fallback: SourceTableIndexFallback::ClassResolved,
                call_fallback: SourceTableCallFallback::NonCallable,
            },
        )]),
        class_bindings: BTreeMap::from([(SourceSessionTableId(1), class)]),
        cells: vec![],
        closures: vec![],
    }
}
#[test]
fn coherent_session_class_associations_preserve_raw_state_and_owner_definitions() {
    let input = session_with_class();
    let raw = input.state.clone();
    let definitions = serde_json::to_vec(input.owner.definitions().unwrap()).unwrap();
    input.validate_class_bindings(1).unwrap();
    assert_eq!(input.state, raw);
    assert_eq!(input.state.tables[0].entries.len(), 1);
    assert_eq!(input.state.values[0], input.state.values[1]);
    assert_eq!(
        input.owner.class(SourceClassId(2)).unwrap().methods["method"].callback,
        SourceCallbackId(3)
    );
    assert_eq!(
        serde_json::to_vec(input.owner.definitions().unwrap()).unwrap(),
        definitions
    );
}
#[test]
fn session_class_associations_require_local_tables_and_exact_owner_identity() {
    for id in [SourceSessionTableId(0), SourceSessionTableId(2)] {
        let mut input = session_with_class();
        let class = input
            .class_bindings
            .remove(&SourceSessionTableId(1))
            .unwrap();
        input.class_bindings.insert(id, class);
        assert_eq!(
            input.validate_class_bindings(2).unwrap_err().kind,
            SourceProgramErrorKind::Binding
        );
    }
    let mut input = session_with_class();
    let foreign = session_with_class();
    input.class_bindings = foreign.class_bindings;
    assert!(
        input
            .validate_class_bindings(1)
            .unwrap_err()
            .message
            .contains("another owner")
    );
}
#[test]
fn resolved_class_coverage_and_binding_are_one_to_one() {
    let mut input = session_with_class();
    input.class_bindings.clear();
    assert!(
        input
            .validate_class_bindings(1)
            .unwrap_err()
            .message
            .contains("lacks a session class binding")
    );
    let mut input = session_with_class();
    input.coverage.clear();
    assert!(
        input
            .validate_class_bindings(1)
            .unwrap_err()
            .message
            .contains("lacks resolved-class index coverage")
    );
    for fallback in [
        SourceTableIndexFallback::Nil,
        SourceTableIndexFallback::Unavailable,
    ] {
        let mut input = session_with_class();
        input
            .coverage
            .get_mut(&SourceSessionTableId(1))
            .unwrap()
            .index_fallback = fallback;
        assert_eq!(
            input.validate_class_bindings(1).unwrap_err().kind,
            SourceProgramErrorKind::Binding
        );
    }
    let mut input = session_with_class();
    let coverage = input.coverage.remove(&SourceSessionTableId(1)).unwrap();
    input.class_bindings.clear();
    input.coverage.insert(SourceSessionTableId(0), coverage);
    assert_eq!(
        input.validate_class_bindings(1).unwrap_err().kind,
        SourceProgramErrorKind::Binding
    );
}
#[test]
fn class_association_limits_precede_metadata_walk_and_allow_ordinary_inputs() {
    let input = session_with_class();
    assert_eq!(
        input.validate_class_bindings(0).unwrap_err().kind,
        SourceProgramErrorKind::ResourceLimit
    );
    let mut ordinary = input.clone();
    ordinary.class_bindings.clear();
    ordinary
        .coverage
        .get_mut(&SourceSessionTableId(1))
        .unwrap()
        .index_fallback = SourceTableIndexFallback::Unavailable;
    ordinary.validate_class_bindings(1).unwrap();
    let mut oversized = input.clone();
    oversized.coverage.insert(
        SourceSessionTableId(2),
        input.coverage[&SourceSessionTableId(1)].clone(),
    );
    assert_eq!(
        oversized.validate_class_bindings(1).unwrap_err().kind,
        SourceProgramErrorKind::ResourceLimit
    );
    let mut oversized = input.clone();
    oversized.class_bindings.insert(
        SourceSessionTableId(2),
        input.class_bindings[&SourceSessionTableId(1)].clone(),
    );
    assert_eq!(
        oversized.validate_class_bindings(1).unwrap_err().kind,
        SourceProgramErrorKind::ResourceLimit
    );
}
#[test]
fn resolved_class_marker_cannot_enter_immutable_definition_context() {
    let (data, classes) = definitions();
    let coverage = session_with_class().coverage[&SourceSessionTableId(1)].clone();
    coverage.validate_shape().unwrap();
    assert_eq!(
        serde_json::to_value(coverage.index_fallback).unwrap(),
        "class_resolved"
    );
    let context = SourceProgramContext {
        schema_version: SOURCE_PROGRAM_CONTEXT_SCHEMA_VERSION,
        iteration: None,
        environment: None,
        tables: BTreeMap::from([(SourceTableId(3), coverage)]),
    };
    assert_eq!(
        SourceProgramOwner::new_with_context(data, Some(classes), context)
            .unwrap_err()
            .kind,
        SourceProgramErrorKind::UnsupportedCapability
    );
}
#[test]
fn shared_class_graph_rejects_live_and_zero_capture_prototypes() {
    // Ordinary method, original constructor and closed parent-index callback.
    for id in [
        SourceCallbackId(3),
        SourceCallbackId(4),
        SourceCallbackId(2),
    ] {
        let (data, classes) = definitions();
        let prototypes = SourceClosurePrototypes {
            schema_version: SOURCE_CLOSURE_PROTOTYPES_SCHEMA_VERSION,
            prototypes: vec![SourceClosurePrototype { callback: id }],
        };
        let error = SourceProgramOwner::new_with_closures(data, Some(classes), None, prototypes)
            .unwrap_err();
        assert_eq!(error.kind, SourceProgramErrorKind::UnsupportedCapability);
        assert!(
            error
                .message
                .contains("class graph reaches a session closure prototype")
        );
    }
    let (mut data, classes) = definitions();
    data.callbacks[2].upvalues.push(SourceUpvalue {
        name: "live".into(),
        value: SourceValue::LiveCapture {},
    });
    let prototypes = SourceClosurePrototypes {
        schema_version: SOURCE_CLOSURE_PROTOTYPES_SCHEMA_VERSION,
        prototypes: vec![SourceClosurePrototype {
            callback: SourceCallbackId(3),
        }],
    };
    assert_eq!(
        SourceProgramOwner::new_with_closures(data, Some(classes), None, prototypes)
            .unwrap_err()
            .kind,
        SourceProgramErrorKind::UnsupportedCapability
    );
}
#[test]
fn class_capture_graph_cannot_hide_a_live_helper_but_unrelated_closures_remain_valid() {
    let (mut data, classes) = definitions();
    data.callbacks.push(callback(span(42, 44)));
    let prototypes = SourceClosurePrototypes {
        schema_version: SOURCE_CLOSURE_PROTOTYPES_SCHEMA_VERSION,
        prototypes: vec![SourceClosurePrototype {
            callback: SourceCallbackId(8),
        }],
    };
    SourceProgramOwner::new_with_closures(
        data.clone(),
        Some(classes.clone()),
        None,
        prototypes.clone(),
    )
    .unwrap();
    data.tables.push(SourceTable {
        fields: BTreeMap::from([("helper".into(), SourceValue::Callback(SourceCallbackId(8)))]),
        indexed: BTreeMap::new(),
    });
    data.callbacks[2].upvalues.push(SourceUpvalue {
        name: "helpers".into(),
        value: SourceValue::Table(SourceTableId(4)),
    });
    let error =
        SourceProgramOwner::new_with_closures(data, Some(classes), None, prototypes).unwrap_err();
    assert_eq!(error.kind, SourceProgramErrorKind::UnsupportedCapability);
    assert!(
        error
            .message
            .contains("class graph reaches a session closure prototype")
    );
}

#[test]
fn class_owner_requires_pairs_links_and_omitted_class_fields_preclude_raw_order() {
    let (mut data, classes) = definitions();
    data.intrinsics
        .insert(SourceCallbackId(6), SourceProgramIntrinsic::Pairs);
    let error = SourceProgramOwner::new_with_classes(data, classes).unwrap_err();
    assert_eq!(error.kind, SourceProgramErrorKind::Binding);
    assert!(error.message.contains("retained next"));
    let (data, mut classes) = definitions();
    classes.classes[0]
        .unsupported_fields
        .insert("omitted".into());
    let table = classes.classes[0].table;
    let source = &data.tables[table.0 as usize - 1];
    let order = source
        .fields
        .keys()
        .cloned()
        .map(SourceTableKey::Text)
        .chain(source.indexed.keys().copied().map(SourceTableKey::Integer))
        .collect();
    let context = SourceProgramContext {
        iteration: Some(SourceProgramIteration {
            table_order: BTreeMap::from([(table, order)]),
            ..SourceProgramIteration::default()
        }),
        ..SourceProgramContext::default()
    };
    let error = SourceProgramOwner::new_with_context(data, Some(classes), context).unwrap_err();
    assert_eq!(error.kind, SourceProgramErrorKind::UnsupportedCapability);
    assert!(error.message.contains("complete class raw inventory"));
}
