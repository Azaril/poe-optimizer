use poe_optimizer_data::{
    game_data::bundled_snapshot,
    item_loading::{ItemLoadingSource, ItemSourceSpan},
    modifier_parser::{ParserFactoryLiteral, ParserProgramCatalog, ParserProgramDefinitionRoot},
    source_program::*,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};

fn span() -> ItemSourceSpan {
    ItemSourceSpan {
        path: "fixtures/source.lua".into(),
        line: 1,
        end_line: 10,
        sha256: "b".repeat(64),
    }
}
fn callback(upvalues: Vec<SourceUpvalue>) -> SourceCallback {
    SourceCallback {
        kind: SourceCallbackKind::Lua { source: span() },
        upvalues,
        environment: SourceEnvironment::OriginalGlobals,
    }
}
fn definitions() -> SourceProgramDefinitions {
    SourceProgramDefinitions {
        schema_version: SOURCE_PROGRAM_DEFINITIONS_SCHEMA_VERSION,
        source: ItemLoadingSource {
            upstream_revision: "a".repeat(40),
            files: BTreeMap::from([("fixtures/source.lua".into(), "c".repeat(64))]),
            construction_spans: BTreeMap::new(),
            module_order: vec!["fixtures/source.lua".into()],
        },
        tables: vec![
            SourceTable {
                fields: BTreeMap::from([
                    ("self".into(), SourceValue::Table(SourceTableId(1))),
                    ("peer".into(), SourceValue::Table(SourceTableId(2))),
                    ("first".into(), SourceValue::Callback(SourceCallbackId(1))),
                    ("second".into(), SourceValue::Callback(SourceCallbackId(2))),
                ]),
                indexed: BTreeMap::new(),
            },
            SourceTable {
                fields: BTreeMap::from([("parent".into(), SourceValue::Table(SourceTableId(1)))]),
                indexed: BTreeMap::new(),
            },
        ],
        callbacks: vec![
            callback(vec![SourceUpvalue {
                name: "state".into(),
                value: SourceValue::Table(SourceTableId(1)),
            }]),
            callback(vec![SourceUpvalue {
                name: "state".into(),
                value: SourceValue::Table(SourceTableId(2)),
            }]),
        ],
        roots: vec![
            SourceProgramRoot {
                name: "definitions".into(),
                table: SourceTableId(1),
            },
            SourceProgramRoot {
                name: "alias".into(),
                table: SourceTableId(1),
            },
        ],
        intrinsics: BTreeMap::new(),
    }
}
fn expression(operation: SourceProgramExprKind) -> SourceProgramExpr {
    SourceProgramExpr {
        location: SourceProgramLocation { start: 0, end: 1 },
        operation,
    }
}
fn program(callback: SourceCallbackId, value: SourceProgramExprKind) -> SourceProgram {
    SourceProgram {
        callback,
        parameter_count: 0,
        variadic: false,
        local_count: 0,
        bindings: vec![],
        body: vec![SourceProgramStatement {
            location: SourceProgramLocation { start: 0, end: 1 },
            operation: SourceProgramStatementKind::Return {
                values: SourceProgramValueList {
                    values: vec![expression(value)],
                    tail: None,
                },
            },
        }],
        provenance: SourceProgramProvenance {
            source: span(),
            function_start: 0,
            function_end: 100,
            function_sha256: "d".repeat(64),
        },
    }
}
fn programs(programs: Vec<SourceProgram>) -> SourceProgramData {
    SourceProgramData {
        schema_version: SOURCE_PROGRAM_SCHEMA_VERSION,
        callbacks: programs
            .iter()
            .enumerate()
            .map(|(i, p)| (p.callback, SourceProgramId(i as u32 + 1)))
            .collect(),
        programs,
    }
}
#[test]
fn standalone_graph_retains_aliases_cycles_and_distinct_closures_without_parser_data() {
    let data = definitions();
    let owner = SourceProgramOwner::new(data.clone()).unwrap();
    assert!(owner.parser().is_none());
    assert_eq!(owner.definitions(), Some(&data));
    let cloned = owner.clone();
    assert!(cloned.is_same_owner(&owner));
    assert!(std::ptr::eq(cloned.tables(), owner.tables()));
    assert_eq!(
        owner.table(SourceTableId(1)).unwrap().fields["self"],
        SourceValue::Table(SourceTableId(1))
    );
    assert_eq!(
        owner.table(SourceTableId(2)).unwrap().fields["parent"],
        SourceValue::Table(SourceTableId(1))
    );
    assert_eq!(
        owner.callback(SourceCallbackId(1)).unwrap().kind,
        owner.callback(SourceCallbackId(2)).unwrap().kind
    );
    assert_ne!(
        owner.callback(SourceCallbackId(1)).unwrap().upvalues,
        owner.callback(SourceCallbackId(2)).unwrap().upvalues
    );
    let first = owner
        .bind_root(SourceProgramDefinitionRoot::Named(
            owner.root_id("definitions").unwrap(),
        ))
        .unwrap();
    let alias = owner
        .bind_root(SourceProgramDefinitionRoot::Named(
            owner.root_id("alias").unwrap(),
        ))
        .unwrap();
    assert_ne!(first.root(), alias.root());
    assert!(std::ptr::eq(first.table(), alias.table()));
    assert!(std::ptr::eq(
        owner.resolve_root(&first).unwrap(),
        cloned.resolve_root(&alias).unwrap()
    ));
    let independent = SourceProgramOwner::new(data).unwrap();
    assert!(!independent.is_same_owner(&owner));
    assert_eq!(
        independent.resolve_root(&first).unwrap_err().kind,
        SourceProgramErrorKind::Binding
    );
    assert!(
        owner
            .bind_root(SourceProgramDefinitionRoot::Named(SourceProgramRootId(0)))
            .is_err()
    );
    assert!(
        owner
            .bind_root(SourceProgramDefinitionRoot::Named(SourceProgramRootId(3)))
            .is_err()
    );
    assert!(owner.table(SourceTableId(0)).is_none());
    assert!(owner.callback(SourceCallbackId(0)).is_none());
}
#[test]
fn programs_validate_named_roots_against_their_actual_owner_and_keep_structural_admission_separate()
{
    let owner = SourceProgramOwner::new(definitions()).unwrap();
    let p = program(
        SourceCallbackId(1),
        SourceProgramExprKind::NamedDefinition {
            root: SourceProgramRootId(1),
        },
    );
    let data = programs(vec![p]);
    let catalog = SourceProgramCatalog::new(data.clone(), owner.clone()).unwrap();
    assert!(catalog.is_bound_to(&owner));
    assert!(!catalog.is_bound_to(&SourceProgramOwner::new(definitions()).unwrap()));
    assert!(std::ptr::eq(catalog.clone().data(), catalog.data()));
    assert_eq!(
        catalog.required_capabilities(),
        &BTreeSet::from([SourceProgramCapability::Core])
    );
    assert_eq!(
        catalog
            .check_capabilities(&BTreeSet::new())
            .unwrap_err()
            .kind,
        SourceProgramErrorKind::UnsupportedCapability
    );
    let encoded = serde_json::to_vec(&data).unwrap();
    assert_eq!(
        SourceProgramCatalog::from_bytes(&encoded, owner.clone())
            .unwrap()
            .data(),
        &data
    );
    assert!(std::ptr::eq(
        catalog
            .definition(SourceProgramDefinitionRoot::Named(SourceProgramRootId(1)))
            .unwrap(),
        owner.table(SourceTableId(1)).unwrap()
    ));
    for root in [
        SourceProgramDefinitionRoot::Named(SourceProgramRootId(3)),
        SourceProgramDefinitionRoot::ModFlags,
    ] {
        let bad = programs(vec![program(
            SourceCallbackId(1),
            match root {
                SourceProgramDefinitionRoot::Named(root) => {
                    SourceProgramExprKind::NamedDefinition { root }
                }
                root => SourceProgramExprKind::Definition {
                    root: root.legacy().unwrap(),
                },
            },
        )]);
        assert_eq!(
            SourceProgramCatalog::new(bad, owner.clone())
                .unwrap_err()
                .kind,
            SourceProgramErrorKind::Binding
        );
    }
}
#[test]
fn common_graph_verifier_rejects_dangling_nil_invalid_source_and_duplicate_names() {
    type Mutation = fn(&mut SourceProgramDefinitions);
    let variants: Vec<Mutation> = vec![
        |d| {
            d.tables[0]
                .fields
                .insert("bad".into(), SourceValue::Table(SourceTableId(3)));
        },
        |d| {
            d.callbacks[0].upvalues[0].value = SourceValue::Callback(SourceCallbackId(3));
        },
        |d| {
            d.tables[0].fields.insert("bad".into(), SourceValue::Nil);
        },
        |d| {
            d.tables[0]
                .fields
                .insert("bad".into(), SourceValue::Number(f64::NAN));
        },
        |d| {
            d.roots[1].name = d.roots[0].name.clone();
        },
        |d| {
            d.roots[1].table = SourceTableId(0);
        },
        |d| {
            d.source.upstream_revision = "invalid".into();
        },
        |d| {
            d.source.module_order.push("missing.lua".into());
        },
        |d| {
            d.source.files = BTreeMap::from([("../escape.lua".into(), "c".repeat(64))]);
        },
    ];
    for change in variants {
        let mut data = definitions();
        change(&mut data);
        assert!(SourceProgramOwner::new(data).is_err());
    }
    let mut nullable = definitions();
    nullable.callbacks[0].upvalues[0].value = SourceValue::Nil;
    assert!(SourceProgramOwner::new(nullable).is_ok());
}
#[test]
fn standalone_definition_limits_are_cumulative_and_checked_before_arc_construction() {
    let mut data = definitions();
    data.roots = (0..4097)
        .map(|i| SourceProgramRoot {
            name: format!("root{i}"),
            table: SourceTableId(1),
        })
        .collect();
    assert_eq!(
        SourceProgramOwner::new(data).unwrap_err().kind,
        SourceProgramErrorKind::ResourceLimit
    );
    let mut data = definitions();
    data.tables[0].fields = (0..4200)
        .map(|i| (format!("field{i}"), SourceValue::Text("x".repeat(4096))))
        .collect();
    let error = SourceProgramOwner::new(data).unwrap_err();
    assert_eq!(error.kind, SourceProgramErrorKind::ResourceLimit);
    assert!(error.message.contains("aggregate text"));
    let mut data = definitions();
    data.callbacks[0].upvalues = (0..129)
        .map(|i| SourceUpvalue {
            name: format!("v{i}"),
            value: SourceValue::Nil,
        })
        .collect();
    assert_eq!(
        SourceProgramOwner::new(data).unwrap_err().kind,
        SourceProgramErrorKind::ResourceLimit
    );
}
#[test]
fn explicit_builtin_bindings_match_captured_closure_identity_and_are_not_automatic_admission() {
    let mut data = definitions();
    data.callbacks.push(SourceCallback {
        kind: SourceCallbackKind::Builtin {
            symbol: "table.insert".into(),
        },
        upvalues: vec![],
        environment: SourceEnvironment::OriginalGlobals,
    });
    data.callbacks[0].upvalues = vec![SourceUpvalue {
        name: "insert".into(),
        value: SourceValue::Callback(SourceCallbackId(3)),
    }];
    let mut p = program(
        SourceCallbackId(1),
        SourceProgramExprKind::Literal {
            value: ParserFactoryLiteral::Nil,
        },
    );
    p.bindings.push(SourceProgramBinding::Intrinsic {
        operation: SourceProgramIntrinsic::TableInsert,
        source: SourceProgramIntrinsicSource::Captured {
            upvalue: 0,
            callback: SourceCallbackId(3),
        },
    });
    assert_eq!(
        SourceProgramCatalog::new(
            programs(vec![p.clone()]),
            SourceProgramOwner::new(data.clone()).unwrap()
        )
        .unwrap_err()
        .kind,
        SourceProgramErrorKind::UnsupportedCapability
    );
    data.intrinsics
        .insert(SourceCallbackId(3), SourceProgramIntrinsic::TableInsert);
    assert!(
        SourceProgramCatalog::new(
            programs(vec![p.clone()]),
            SourceProgramOwner::new(data.clone()).unwrap()
        )
        .is_ok()
    );
    let mut wrong = data.clone();
    wrong
        .intrinsics
        .insert(SourceCallbackId(3), SourceProgramIntrinsic::Ipairs);
    assert_eq!(
        SourceProgramOwner::new(wrong).unwrap_err().kind,
        SourceProgramErrorKind::Binding
    );
    data.callbacks[0].upvalues[0].value = SourceValue::Callback(SourceCallbackId(2));
    assert_eq!(
        SourceProgramCatalog::new(programs(vec![p]), SourceProgramOwner::new(data).unwrap())
            .unwrap_err()
            .kind,
        SourceProgramErrorKind::Binding
    );
}
#[test]
fn shared_verifier_retains_captured_helper_cycles_and_source_provenance_checks() {
    let mut data = definitions();
    for i in 0..2 {
        data.callbacks[i].upvalues = vec![SourceUpvalue {
            name: "helper".into(),
            value: SourceValue::Callback(SourceCallbackId(2 - i as u32)),
        }];
    }
    let body = (1..=2)
        .map(|i| {
            let mut p = program(
                SourceCallbackId(i),
                SourceProgramExprKind::Call {
                    call: Box::new(SourceProgramCall {
                        binding: 0,
                        receiver: None,
                        arguments: SourceProgramValueList::default(),
                    }),
                },
            );
            p.bindings = vec![SourceProgramBinding::CapturedCallback {
                upvalue: 0,
                callback: SourceCallbackId(3 - i),
            }];
            p
        })
        .collect();
    let library = programs(body);
    let owner = SourceProgramOwner::new(data).unwrap();
    let catalog = SourceProgramCatalog::new(library.clone(), owner.clone()).unwrap();
    assert!(
        catalog
            .required_capabilities()
            .contains(&SourceProgramCapability::RecursiveCalls)
    );
    let mut wrong = library;
    wrong.programs[0].provenance.source.line += 1;
    assert_eq!(
        SourceProgramCatalog::new(wrong, owner).unwrap_err().kind,
        SourceProgramErrorKind::Binding
    );
}
#[test]
fn parser_adapter_retains_the_exact_graph_arc_and_legacy_wire_without_a_schema_change() {
    static SNAPSHOT: OnceLock<poe_optimizer_data::game_data::GameDataSnapshot> = OnceLock::new();
    let snapshot = SNAPSHOT.get_or_init(|| bundled_snapshot().unwrap());
    let original = snapshot.modifier_parser();
    assert!(std::ptr::eq(
        snapshot.parser_programs().programs().data(),
        &original.data().programs.data
    ));

    let generic = SourceProgramOwner::from_parser(original.clone());
    assert!(generic.parser().unwrap().is_same_owner(original));
    assert!(std::ptr::eq(
        generic.tables(),
        original.data().tables.as_slice()
    ));
    assert!(std::ptr::eq(
        generic.callbacks(),
        original.data().callbacks.as_slice()
    ));
    let catalog = ParserProgramCatalog::new(
        SourceProgramData {
            schema_version: SOURCE_PROGRAM_SCHEMA_VERSION,
            programs: vec![],
            callbacks: BTreeMap::new(),
        },
        original.clone(),
    )
    .unwrap();
    assert!(catalog.is_bound_to(original));
    assert!(catalog.source_programs().owner().is_same_owner(&generic));
    for (root, wire) in [
        (ParserProgramDefinitionRoot::ModFlags, "\"mod_flags\""),
        (
            ParserProgramDefinitionRoot::KeywordFlags,
            "\"keyword_flags\"",
        ),
        (ParserProgramDefinitionRoot::SkillTypes, "\"skill_types\""),
        (
            ParserProgramDefinitionRoot::GemIdLookup,
            "\"gem_id_lookup\"",
        ),
    ] {
        assert_eq!(serde_json::to_string(&root).unwrap(), wire);
        assert!(std::ptr::eq(
            catalog.definition(root),
            generic.definition(root.into()).unwrap()
        ));
    }
    assert!(
        generic
            .bind_root(SourceProgramDefinitionRoot::Named(SourceProgramRootId(1)))
            .is_err()
    );
}
#[test]
fn definition_json_keeps_duplicate_intrinsic_keys_and_unknown_fields_invalid() {
    let json = serde_json::to_string(&definitions()).unwrap();
    assert_eq!(
        SourceProgramOwner::from_bytes(json.as_bytes())
            .unwrap()
            .definitions(),
        Some(&definitions())
    );
    let duplicate = json.replace(
        "\"intrinsics\":{}",
        "\"intrinsics\":{\"1\":\"ipairs\",\"1\":\"ipairs\"}",
    );
    assert!(
        SourceProgramOwner::from_bytes(duplicate.as_bytes())
            .unwrap_err()
            .message
            .contains("duplicate")
    );
    let unknown = json.replacen('{', "{\"unknown\":true,", 1);
    assert!(SourceProgramOwner::from_bytes(unknown.as_bytes()).is_err());
}

#[test]
fn dynamic_method_binding_has_explicit_receiver_and_separate_execution_capabilities() {
    let call = SourceProgramCall {
        binding: 0,
        receiver: Some(Box::new(expression(SourceProgramExprKind::Literal {
            value: ParserFactoryLiteral::Nil,
        }))),
        arguments: SourceProgramValueList::default(),
    };
    let mut p = program(
        SourceCallbackId(1),
        SourceProgramExprKind::Call {
            call: Box::new(call.clone()),
        },
    );
    p.bindings = vec![SourceProgramBinding::DynamicMethod {
        key: "AddMod".into(),
    }];
    let owner = SourceProgramOwner::new(definitions()).unwrap();
    let catalog = SourceProgramCatalog::new(programs(vec![p.clone()]), owner.clone()).unwrap();
    assert!(
        catalog
            .required_capabilities()
            .contains(&SourceProgramCapability::DynamicMethods)
    );
    assert!(
        catalog
            .required_capabilities()
            .contains(&SourceProgramCapability::RecursiveCalls)
    );
    assert_eq!(
        catalog
            .check_capabilities(&BTreeSet::from([SourceProgramCapability::Core]))
            .unwrap_err()
            .kind,
        SourceProgramErrorKind::UnsupportedCapability
    );
    for key in ["".into(), "a\0b".into(), "x".repeat(257)] {
        let mut invalid = p.clone();
        invalid.bindings[0] = SourceProgramBinding::DynamicMethod { key };
        assert!(SourceProgramCatalog::new(programs(vec![invalid]), owner.clone()).is_err());
    }
    let mut no_receiver = call;
    no_receiver.receiver = None;
    p.body[0].operation = SourceProgramStatementKind::Return {
        values: SourceProgramValueList {
            values: vec![expression(SourceProgramExprKind::Call {
                call: Box::new(no_receiver),
            })],
            tail: None,
        },
    };
    assert_eq!(
        SourceProgramCatalog::new(programs(vec![p]), owner)
            .unwrap_err()
            .kind,
        SourceProgramErrorKind::Binding
    );
}
#[test]
fn parser_owner_cannot_admit_dynamic_method_ir() {
    let snapshot = bundled_snapshot().unwrap();
    let original = snapshot.modifier_parser();
    let mut p = original.data().programs.data.programs[0].clone();
    p.bindings = vec![SourceProgramBinding::DynamicMethod {
        key: "AddMod".into(),
    }];
    let error = ParserProgramCatalog::new(programs(vec![p]), original.clone()).unwrap_err();
    assert_eq!(error.kind, SourceProgramErrorKind::UnsupportedCapability);
    assert!(error.message.contains("parser owner"));
}
#[test]
fn type_and_select_keep_source_bound_builtin_identity() {
    for (operation, symbol) in [
        (SourceProgramIntrinsic::Type, "type"),
        (SourceProgramIntrinsic::Select, "select"),
    ] {
        assert_eq!(operation.global_path(), Some([symbol].as_slice()));
        let mut d = definitions();
        d.callbacks.push(SourceCallback {
            kind: SourceCallbackKind::Builtin {
                symbol: symbol.into(),
            },
            upvalues: vec![],
            environment: SourceEnvironment::OriginalGlobals,
        });
        d.callbacks[0].upvalues = vec![SourceUpvalue {
            name: "primitive".into(),
            value: SourceValue::Callback(SourceCallbackId(3)),
        }];
        d.intrinsics.insert(SourceCallbackId(3), operation);
        let mut p = program(
            SourceCallbackId(1),
            SourceProgramExprKind::Literal {
                value: ParserFactoryLiteral::Nil,
            },
        );
        p.bindings = vec![SourceProgramBinding::Intrinsic {
            operation,
            source: SourceProgramIntrinsicSource::Captured {
                upvalue: 0,
                callback: SourceCallbackId(3),
            },
        }];
        SourceProgramCatalog::new(
            programs(vec![p.clone()]),
            SourceProgramOwner::new(d.clone()).unwrap(),
        )
        .unwrap();
        d.intrinsics.clear();
        assert_eq!(
            SourceProgramCatalog::new(programs(vec![p]), SourceProgramOwner::new(d).unwrap())
                .unwrap_err()
                .kind,
            SourceProgramErrorKind::UnsupportedCapability
        );
    }
}

fn value_call_program(callee: SourceProgramExpr) -> SourceProgram {
    let mut p = program(
        SourceCallbackId(1),
        SourceProgramExprKind::Call {
            call: Box::new(SourceProgramCall {
                binding: 0,
                receiver: Some(Box::new(callee)),
                arguments: SourceProgramValueList::default(),
            }),
        },
    );
    p.bindings = vec![SourceProgramBinding::DynamicCall {}];
    p
}
#[test]
fn function_value_calls_retain_explicit_callee_and_separate_capability_without_wire_changes() {
    let p = value_call_program(expression(SourceProgramExprKind::Get {
        table: Box::new(expression(SourceProgramExprKind::Capture { upvalue: 0 })),
        key: Box::new(expression(SourceProgramExprKind::Bytes {
            value: b"second".to_vec(),
        })),
    }));
    let owner = SourceProgramOwner::new(definitions()).unwrap();
    let data = programs(vec![p]);
    let catalog = SourceProgramCatalog::new(data.clone(), owner.clone()).unwrap();
    assert!(
        catalog
            .required_capabilities()
            .contains(&SourceProgramCapability::DynamicCalls)
    );
    assert!(
        catalog
            .required_capabilities()
            .contains(&SourceProgramCapability::RecursiveCalls)
    );
    assert!(
        !catalog
            .required_capabilities()
            .contains(&SourceProgramCapability::DynamicMethods)
    );
    let bytes = serde_json::to_vec(&data).unwrap();
    assert_eq!(
        SourceProgramCatalog::from_bytes(&bytes, owner.clone())
            .unwrap()
            .data(),
        &data
    );
    assert!(!catalog.is_bound_to(&SourceProgramOwner::new(definitions()).unwrap()));
    let mut supported = catalog.required_capabilities().clone();
    supported.remove(&SourceProgramCapability::DynamicCalls);
    assert_eq!(
        catalog.check_capabilities(&supported).unwrap_err().kind,
        SourceProgramErrorKind::UnsupportedCapability
    );
    assert_eq!(
        serde_json::to_string(&SourceProgramBinding::DynamicCall {}).unwrap(),
        "{\"kind\":\"dynamic_call\"}"
    );
    assert_eq!(
        serde_json::to_string(&SourceProgramBinding::CapturedCallback {
            upvalue: 0,
            callback: SourceCallbackId(2)
        })
        .unwrap(),
        "{\"kind\":\"captured_callback\",\"upvalue\":0,\"callback\":2}"
    );
    assert_eq!(
        serde_json::to_string(&SourceProgramCall {
            binding: 0,
            receiver: None,
            arguments: SourceProgramValueList::default()
        })
        .unwrap(),
        "{\"binding\":0,\"receiver\":null,\"arguments\":{\"values\":[],\"tail\":null}}"
    );
    assert!(
        serde_json::from_str::<SourceProgramBinding>(
            "{\"kind\":\"dynamic_call\",\"ignored\":true}"
        )
        .is_err()
    );
    let mut missing = serde_json::to_value(data).unwrap();
    missing["programs"][0]["body"][0]["operation"]["values"]["values"][0]["operation"]["call"]["receiver"] =
        serde_json::Value::Null;
    assert!(
        SourceProgramCatalog::from_bytes(&serde_json::to_vec(&missing).unwrap(), owner).is_err()
    );
}
#[test]
fn function_value_callee_keeps_scope_capture_and_depth_validation() {
    let owner = SourceProgramOwner::new(definitions()).unwrap();
    for (callee, kind) in [
        (
            expression(SourceProgramExprKind::Local { local: 0 }),
            SourceProgramErrorKind::InvalidData,
        ),
        (
            expression(SourceProgramExprKind::Capture { upvalue: 99 }),
            SourceProgramErrorKind::Binding,
        ),
        (
            expression(SourceProgramExprKind::NamedDefinition {
                root: SourceProgramRootId(99),
            }),
            SourceProgramErrorKind::Binding,
        ),
    ] {
        assert_eq!(
            SourceProgramCatalog::new(programs(vec![value_call_program(callee)]), owner.clone())
                .unwrap_err()
                .kind,
            kind
        );
    }
    let mut deep = expression(SourceProgramExprKind::Capture { upvalue: 0 });
    for _ in 0..60 {
        deep = expression(SourceProgramExprKind::Get {
            table: Box::new(deep),
            key: Box::new(expression(SourceProgramExprKind::Bytes {
                value: b"callee".to_vec(),
            })),
        });
    }
    assert_eq!(
        SourceProgramCatalog::new(programs(vec![value_call_program(deep)]), owner.clone())
            .unwrap_err()
            .kind,
        SourceProgramErrorKind::ResourceLimit
    );
    let mut p = value_call_program(expression(SourceProgramExprKind::Capture { upvalue: 0 }));
    if let SourceProgramStatementKind::Return { values } = &mut p.body[0].operation
        && let SourceProgramExprKind::Call { call } = &mut values.values[0].operation
    {
        call.receiver = None;
    }
    assert_eq!(
        SourceProgramCatalog::new(programs(vec![p]), owner)
            .unwrap_err()
            .kind,
        SourceProgramErrorKind::Binding
    );
}
#[test]
fn standalone_numeric_text_and_pattern_primitives_bind_original_identity_and_shadowing() {
    for (operation, path) in [
        (SourceProgramIntrinsic::Unpack, vec!["unpack"]),
        (SourceProgramIntrinsic::MathFloor, vec!["math", "floor"]),
        (SourceProgramIntrinsic::MathMin, vec!["math", "min"]),
        (SourceProgramIntrinsic::MathMax, vec!["math", "max"]),
        (SourceProgramIntrinsic::ToString, vec!["tostring"]),
        (SourceProgramIntrinsic::StringMatch, vec!["string", "match"]),
    ] {
        assert!(operation.is_standalone_only());
        assert_eq!(operation.global_path(), Some(path.as_slice()));
        let mut p = program(
            SourceCallbackId(1),
            SourceProgramExprKind::Literal {
                value: ParserFactoryLiteral::Nil,
            },
        );
        p.bindings = vec![SourceProgramBinding::Intrinsic {
            operation,
            source: SourceProgramIntrinsicSource::OriginalGlobal,
        }];
        SourceProgramCatalog::new(
            programs(vec![p.clone()]),
            SourceProgramOwner::new(definitions()).unwrap(),
        )
        .unwrap();
        let mut shadow = definitions();
        shadow.callbacks[0].upvalues[0].name = path[0].into();
        assert_eq!(
            SourceProgramCatalog::new(
                programs(vec![p.clone()]),
                SourceProgramOwner::new(shadow).unwrap()
            )
            .unwrap_err()
            .kind,
            SourceProgramErrorKind::Binding
        );
        let mut captured = definitions();
        captured.callbacks.push(SourceCallback {
            kind: SourceCallbackKind::Builtin {
                symbol: path.join("."),
            },
            upvalues: vec![],
            environment: SourceEnvironment::OriginalGlobals,
        });
        captured.callbacks[0].upvalues[0].value = SourceValue::Callback(SourceCallbackId(3));
        captured.intrinsics.insert(SourceCallbackId(3), operation);
        p.bindings = vec![SourceProgramBinding::Intrinsic {
            operation,
            source: SourceProgramIntrinsicSource::Captured {
                upvalue: 0,
                callback: SourceCallbackId(3),
            },
        }];
        SourceProgramCatalog::new(
            programs(vec![p.clone()]),
            SourceProgramOwner::new(captured.clone()).unwrap(),
        )
        .unwrap();
        captured.intrinsics.clear();
        assert_eq!(
            SourceProgramCatalog::new(
                programs(vec![p]),
                SourceProgramOwner::new(captured).unwrap()
            )
            .unwrap_err()
            .kind,
            SourceProgramErrorKind::UnsupportedCapability
        );
    }
    assert!(SourceProgramIntrinsic::StringMatch.is_string_method());
    assert!(!SourceProgramIntrinsic::MathFloor.is_string_method());
    assert!(!SourceProgramIntrinsic::MathMin.is_string_method());
    assert!(!SourceProgramIntrinsic::ToNumber.is_standalone_only());
}
#[test]
fn parser_owner_rejects_new_function_value_and_intrinsic_forms_without_changing_admission() {
    let snapshot = bundled_snapshot().unwrap();
    let owner = snapshot.modifier_parser();
    let base = &owner.data().programs.data.programs[0];
    let bindings = [
        SourceProgramBinding::DynamicCall {},
        SourceProgramBinding::Intrinsic {
            operation: SourceProgramIntrinsic::Unpack,
            source: SourceProgramIntrinsicSource::OriginalGlobal,
        },
        SourceProgramBinding::Intrinsic {
            operation: SourceProgramIntrinsic::MathFloor,
            source: SourceProgramIntrinsicSource::OriginalGlobal,
        },
        SourceProgramBinding::Intrinsic {
            operation: SourceProgramIntrinsic::MathMin,
            source: SourceProgramIntrinsicSource::OriginalGlobal,
        },
        SourceProgramBinding::Intrinsic {
            operation: SourceProgramIntrinsic::MathMax,
            source: SourceProgramIntrinsicSource::OriginalGlobal,
        },
        SourceProgramBinding::Intrinsic {
            operation: SourceProgramIntrinsic::ToString,
            source: SourceProgramIntrinsicSource::OriginalGlobal,
        },
        SourceProgramBinding::Intrinsic {
            operation: SourceProgramIntrinsic::StringMatch,
            source: SourceProgramIntrinsicSource::OriginalGlobal,
        },
    ];
    let original = serde_json::to_vec(&owner.data().programs).unwrap();
    for binding in bindings {
        let mut p = base.clone();
        p.bindings = vec![binding];
        let error = ParserProgramCatalog::new(programs(vec![p]), owner.clone()).unwrap_err();
        assert_eq!(error.kind, SourceProgramErrorKind::UnsupportedCapability);
        assert!(error.message.contains("parser owner"));
    }
    assert_eq!(
        serde_json::to_vec(&owner.data().programs).unwrap(),
        original
    );
}

#[test]
fn explicit_environment_forbids_unobserved_global_intrinsic_bypasses() {
    let mut p = program(
        SourceCallbackId(1),
        SourceProgramExprKind::Literal {
            value: ParserFactoryLiteral::Nil,
        },
    );
    p.bindings.push(SourceProgramBinding::Intrinsic {
        operation: SourceProgramIntrinsic::ToNumber,
        source: SourceProgramIntrinsicSource::OriginalGlobal,
    });
    // Existing authored owners, with or without unrelated coverage, stay valid.
    SourceProgramCatalog::new(
        programs(vec![p.clone()]),
        SourceProgramOwner::new(definitions()).unwrap(),
    )
    .unwrap();
    SourceProgramCatalog::new(
        programs(vec![p.clone()]),
        SourceProgramOwner::new_with_context(definitions(), None, SourceProgramContext::default())
            .unwrap(),
    )
    .unwrap();
    let owner = SourceProgramOwner::new_with_context(
        definitions(),
        None,
        SourceProgramContext {
            environment: Some(SourceProgramRootId(1)),
            ..SourceProgramContext::default()
        },
    )
    .unwrap();
    assert!(
        SourceProgramCatalog::new(programs(vec![p]), owner.clone())
            .unwrap_err()
            .message
            .contains("bypasses explicit")
    );
    let p = program(
        SourceCallbackId(1),
        SourceProgramExprKind::NamedDefinition {
            root: SourceProgramRootId(1),
        },
    );
    SourceProgramCatalog::new(programs(vec![p]), owner).unwrap();

    let original = bundled_snapshot().unwrap().modifier_parser().clone();
    let parser = SourceProgramOwner::from_parser(original.clone());
    assert!(parser.context().is_none());
    assert!(parser.environment_root().is_none());
    assert!(parser.bind_environment().unwrap().is_none());
    assert!(parser.table_coverage(SourceTableId(1)).is_none());
    let p = program(
        SourceCallbackId(1),
        SourceProgramExprKind::NamedDefinition {
            root: SourceProgramRootId(1),
        },
    );
    assert!(ParserProgramCatalog::new(programs(vec![p]), original).is_err());
}

#[test]
fn legacy_parser_rejects_live_markers_and_capture_writes_without_wire_change() {
    let snapshot = bundled_snapshot().unwrap();
    let original = snapshot.modifier_parser();
    let wire = serde_json::to_vec(&original.data().programs).unwrap();
    let mut data = original.data().clone();
    let mut extra = data
        .callbacks
        .iter()
        .find(|value| matches!(value.kind, SourceCallbackKind::Lua { .. }))
        .unwrap()
        .clone();
    extra.upvalues = vec![SourceUpvalue {
        name: "live".into(),
        value: SourceValue::LiveCapture {},
    }];
    data.callbacks.push(extra);
    data.factories.insert(
        SourceCallbackId(data.callbacks.len() as u32),
        poe_optimizer_data::modifier_parser::ParserFactoryDisposition::Unsupported {
            reason: "test live capture".into(),
        },
    );
    assert!(
        data.validate()
            .unwrap_err()
            .to_string()
            .contains("live capture marker")
    );
    let (index, callback) = original
        .data()
        .callbacks
        .iter()
        .enumerate()
        .find(|(_, value)| matches!(value.kind, SourceCallbackKind::Lua { .. }))
        .unwrap();
    let mut p = program(
        SourceCallbackId(index as u32 + 1),
        SourceProgramExprKind::Literal {
            value: ParserFactoryLiteral::Nil,
        },
    );
    let SourceCallbackKind::Lua { source } = &callback.kind else {
        unreachable!()
    };
    p.provenance.source = source.clone();
    p.body[0].operation = SourceProgramStatementKind::CaptureSet {
        upvalue: 0,
        values: SourceProgramValueList::default(),
    };
    let error = ParserProgramCatalog::new(programs(vec![p]), original.clone()).unwrap_err();
    assert_eq!(error.kind, SourceProgramErrorKind::UnsupportedCapability);
    assert_eq!(serde_json::to_vec(&original.data().programs).unwrap(), wire);
    let facade = SourceProgramOwner::from_parser(original.clone());
    assert!(facade.closure_prototypes().is_none());
    assert!(
        facade
            .bind_closure_prototype(SourceClosurePrototypeId(1))
            .is_err()
    );
}

#[test]
fn floor_requires_the_exact_builtin_and_capture_slot_even_with_an_explicit_environment() {
    let mut data = definitions();
    data.callbacks.push(SourceCallback {
        kind: SourceCallbackKind::Builtin {
            symbol: "math.floor".into(),
        },
        upvalues: vec![],
        environment: SourceEnvironment::OriginalGlobals,
    });
    data.callbacks[0].upvalues[0].value = SourceValue::Callback(SourceCallbackId(3));
    data.intrinsics
        .insert(SourceCallbackId(3), SourceProgramIntrinsic::MathFloor);
    let mut p = program(
        SourceCallbackId(1),
        SourceProgramExprKind::Call {
            call: Box::new(SourceProgramCall {
                binding: 0,
                receiver: None,
                arguments: SourceProgramValueList {
                    values: vec![expression(SourceProgramExprKind::Literal {
                        value: ParserFactoryLiteral::Number(1.25),
                    })],
                    tail: None,
                },
            }),
        },
    );
    p.bindings = vec![SourceProgramBinding::Intrinsic {
        operation: SourceProgramIntrinsic::MathFloor,
        source: SourceProgramIntrinsicSource::Captured {
            upvalue: 0,
            callback: SourceCallbackId(3),
        },
    }];
    let owner = SourceProgramOwner::new_with_context(
        data.clone(),
        None,
        SourceProgramContext {
            environment: Some(SourceProgramRootId(1)),
            ..SourceProgramContext::default()
        },
    )
    .unwrap();
    SourceProgramCatalog::new(programs(vec![p.clone()]), owner.clone()).unwrap();
    for source in [
        SourceProgramIntrinsicSource::OriginalGlobal,
        SourceProgramIntrinsicSource::Captured {
            upvalue: 1,
            callback: SourceCallbackId(3),
        },
        SourceProgramIntrinsicSource::Captured {
            upvalue: 0,
            callback: SourceCallbackId(2),
        },
    ] {
        let mut invalid = p.clone();
        invalid.bindings = vec![SourceProgramBinding::Intrinsic {
            operation: SourceProgramIntrinsic::MathFloor,
            source,
        }];
        assert_eq!(
            SourceProgramCatalog::new(programs(vec![invalid]), owner.clone())
                .unwrap_err()
                .kind,
            SourceProgramErrorKind::Binding
        );
    }
    let mut wrong_symbol = data.clone();
    wrong_symbol.callbacks[2].kind = SourceCallbackKind::Builtin {
        symbol: "math.ceil".into(),
    };
    assert_eq!(
        SourceProgramOwner::new(wrong_symbol).unwrap_err().kind,
        SourceProgramErrorKind::Binding
    );
    let mut wrong_kind = data.clone();
    wrong_kind.callbacks[2].kind = SourceCallbackKind::Lua { source: span() };
    assert_eq!(
        SourceProgramOwner::new(wrong_kind).unwrap_err().kind,
        SourceProgramErrorKind::Binding
    );
    data.callbacks[2].upvalues.push(SourceUpvalue {
        name: "hidden".into(),
        value: SourceValue::Number(1.0),
    });
    let error = SourceProgramOwner::new(data).unwrap_err();
    assert_eq!(error.kind, SourceProgramErrorKind::InvalidData);
    assert!(error.message.contains("invalid builtin"));
}

#[test]
fn existing_power_wire_form_retains_parser_structural_acceptance() {
    let snapshot = bundled_snapshot().unwrap();
    let owner = snapshot.modifier_parser();
    let original = serde_json::to_vec(&owner.data().programs).unwrap();
    let mut p = owner.data().programs.data.programs[0].clone();
    let location = p.body[0].location;
    p.bindings.clear();
    p.body = vec![SourceProgramStatement {
        location,
        operation: SourceProgramStatementKind::Return {
            values: SourceProgramValueList {
                values: vec![SourceProgramExpr {
                    location,
                    operation: SourceProgramExprKind::Binary {
                        operation: SourceProgramBinary::Power,
                        left: Box::new(SourceProgramExpr {
                            location,
                            operation: SourceProgramExprKind::Literal {
                                value: ParserFactoryLiteral::Number(2.0),
                            },
                        }),
                        right: Box::new(SourceProgramExpr {
                            location,
                            operation: SourceProgramExprKind::Literal {
                                value: ParserFactoryLiteral::Number(3.0),
                            },
                        }),
                    },
                }],
                tail: None,
            },
        },
    }];
    // The engine keeps parser Power unsupported at execution. Structural
    // acceptance and serialized enum spelling predate standalone admission.
    ParserProgramCatalog::new(programs(vec![p]), owner.clone()).unwrap();
    assert_eq!(
        serde_json::to_string(&SourceProgramBinary::Power).unwrap(),
        "\"power\""
    );
    assert_eq!(
        serde_json::to_vec(&owner.data().programs).unwrap(),
        original
    );
}

#[test]
fn generic_iterator_initializers_have_ordinary_scope_pack_and_resource_validation() {
    let mut p = program(
        SourceCallbackId(1),
        SourceProgramExprKind::Literal {
            value: ParserFactoryLiteral::Nil,
        },
    );
    p.local_count = 1;
    p.body = vec![SourceProgramStatement {
        location: SourceProgramLocation { start: 0, end: 1 },
        operation: SourceProgramStatementKind::ForEach {
            locals: vec![0],
            iterator: SourceProgramIterator::Generic {
                values: SourceProgramValueList {
                    values: vec![expression(SourceProgramExprKind::Capture { upvalue: 0 })],
                    tail: None,
                },
            },
            body: vec![SourceProgramStatement {
                location: SourceProgramLocation { start: 0, end: 1 },
                operation: SourceProgramStatementKind::Break,
            }],
        },
    }];
    let owner = SourceProgramOwner::new(definitions()).unwrap();
    let catalog = SourceProgramCatalog::new(programs(vec![p.clone()]), owner.clone()).unwrap();
    assert!(
        catalog
            .required_capabilities()
            .contains(&SourceProgramCapability::GenericFor)
    );
    assert!(
        catalog
            .required_capabilities()
            .contains(&SourceProgramCapability::RecursiveCalls)
    );
    assert!(
        catalog
            .check_capabilities(&BTreeSet::from([
                SourceProgramCapability::Core,
                SourceProgramCapability::RecursiveCalls
            ]))
            .is_err()
    );
    for (value, expected) in [
        (
            expression(SourceProgramExprKind::Local { local: 0 }),
            SourceProgramErrorKind::InvalidData,
        ),
        (
            expression(SourceProgramExprKind::Capture { upvalue: 1 }),
            SourceProgramErrorKind::Binding,
        ),
    ] {
        let mut invalid = p.clone();
        let SourceProgramStatementKind::ForEach {
            iterator: SourceProgramIterator::Generic { values },
            ..
        } = &mut invalid.body[0].operation
        else {
            unreachable!()
        };
        values.values = vec![value];
        assert_eq!(
            SourceProgramCatalog::new(programs(vec![invalid]), owner.clone())
                .unwrap_err()
                .kind,
            expected
        );
    }
    let mut invalid = p.clone();
    let SourceProgramStatementKind::ForEach {
        iterator: SourceProgramIterator::Generic { values },
        ..
    } = &mut invalid.body[0].operation
    else {
        unreachable!()
    };
    values.tail = Some(Box::new(SourceProgramPack::Varargs));
    assert_eq!(
        SourceProgramCatalog::new(programs(vec![invalid]), owner.clone())
            .unwrap_err()
            .kind,
        SourceProgramErrorKind::InvalidData
    );
    let mut deep = expression(SourceProgramExprKind::Capture { upvalue: 0 });
    for _ in 0..60 {
        deep = expression(SourceProgramExprKind::Unary {
            operation: SourceProgramUnary::Not,
            value: Box::new(deep),
        });
    }
    let mut invalid = p.clone();
    let SourceProgramStatementKind::ForEach {
        iterator: SourceProgramIterator::Generic { values },
        ..
    } = &mut invalid.body[0].operation
    else {
        unreachable!()
    };
    values.values = vec![deep];
    assert_eq!(
        SourceProgramCatalog::new(programs(vec![invalid]), owner)
            .unwrap_err()
            .kind,
        SourceProgramErrorKind::ResourceLimit
    );
    let snapshot = bundled_snapshot().unwrap();
    let parser = snapshot.modifier_parser();
    let wire = serde_json::to_vec(&parser.data().programs).unwrap();
    let mut legacy = parser.data().programs.data.programs[0].clone();
    legacy.body = p.body;
    legacy.local_count = 1;
    legacy.parameter_count = 0;
    legacy.bindings.clear();
    assert_eq!(
        ParserProgramCatalog::new(programs(vec![legacy]), parser.clone())
            .unwrap_err()
            .kind,
        SourceProgramErrorKind::UnsupportedCapability
    );
    assert_eq!(serde_json::to_vec(&parser.data().programs).unwrap(), wire);
}
#[test]
fn pairs_requires_exact_capture_while_next_global_keeps_its_standalone_boundary() {
    let mut data = definitions();
    for (symbol, operation) in [
        ("pairs", SourceProgramIntrinsic::Pairs),
        ("next", SourceProgramIntrinsic::Next),
    ] {
        data.callbacks.push(SourceCallback {
            kind: SourceCallbackKind::Builtin {
                symbol: symbol.into(),
            },
            upvalues: vec![],
            environment: SourceEnvironment::OriginalGlobals,
        });
        data.intrinsics
            .insert(SourceCallbackId(data.callbacks.len() as u32), operation);
    }
    data.callbacks[0].upvalues[0].value = SourceValue::Callback(SourceCallbackId(3));
    let owner = SourceProgramOwner::new_with_context(
        data,
        None,
        SourceProgramContext {
            iteration: Some(SourceProgramIteration {
                pairs_next: BTreeMap::from([(SourceCallbackId(3), SourceCallbackId(4))]),
                ..SourceProgramIteration::default()
            }),
            ..SourceProgramContext::default()
        },
    )
    .unwrap();
    let mut p = program(
        SourceCallbackId(1),
        SourceProgramExprKind::Literal {
            value: ParserFactoryLiteral::Nil,
        },
    );
    p.bindings = vec![SourceProgramBinding::Intrinsic {
        operation: SourceProgramIntrinsic::Pairs,
        source: SourceProgramIntrinsicSource::Captured {
            upvalue: 0,
            callback: SourceCallbackId(3),
        },
    }];
    SourceProgramCatalog::new(programs(vec![p.clone()]), owner.clone()).unwrap();
    p.bindings = vec![SourceProgramBinding::Intrinsic {
        operation: SourceProgramIntrinsic::Pairs,
        source: SourceProgramIntrinsicSource::OriginalGlobal,
    }];
    let error = SourceProgramCatalog::new(programs(vec![p.clone()]), owner.clone()).unwrap_err();
    assert_eq!(error.kind, SourceProgramErrorKind::Binding);
    assert!(error.message.contains("exact captured"));
    p.bindings = vec![SourceProgramBinding::Intrinsic {
        operation: SourceProgramIntrinsic::Next,
        source: SourceProgramIntrinsicSource::OriginalGlobal,
    }];
    SourceProgramCatalog::new(programs(vec![p]), owner).unwrap();
    let snapshot = bundled_snapshot().unwrap();
    let parser = snapshot.modifier_parser();
    for operation in [SourceProgramIntrinsic::Pairs, SourceProgramIntrinsic::Next] {
        assert!(operation.is_standalone_only());
        assert!(!operation.is_string_method());
        let mut p = parser.data().programs.data.programs[0].clone();
        p.bindings = vec![SourceProgramBinding::Intrinsic {
            operation,
            source: SourceProgramIntrinsicSource::OriginalGlobal,
        }];
        assert_eq!(
            ParserProgramCatalog::new(programs(vec![p]), parser.clone())
                .unwrap_err()
                .kind,
            SourceProgramErrorKind::UnsupportedCapability
        );
    }
}
