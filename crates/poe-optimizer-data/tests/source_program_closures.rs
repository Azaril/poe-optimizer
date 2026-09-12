use poe_optimizer_data::{
    item_loading::{ItemLoadingSource, ItemSourceSpan},
    modifier_parser::ParserFactoryLiteral,
    source_program::*,
};
use std::collections::{BTreeMap, BTreeSet};
fn span() -> ItemSourceSpan {
    ItemSourceSpan {
        path: "fixture.lua".into(),
        line: 1,
        end_line: 8,
        sha256: "b".repeat(64),
    }
}
fn definitions() -> SourceProgramDefinitions {
    SourceProgramDefinitions {
        schema_version: SOURCE_PROGRAM_DEFINITIONS_SCHEMA_VERSION,
        source: ItemLoadingSource {
            upstream_revision: "a".repeat(40),
            files: BTreeMap::from([("fixture.lua".into(), "c".repeat(64))]),
            construction_spans: BTreeMap::new(),
            module_order: vec!["fixture.lua".into()],
        },
        tables: vec![SourceTable::default()],
        callbacks: vec![SourceCallback {
            kind: SourceCallbackKind::Lua { source: span() },
            environment: SourceEnvironment::OriginalGlobals,
            upvalues: vec![
                SourceUpvalue {
                    name: "self".into(),
                    value: SourceValue::LiveCapture {},
                },
                SourceUpvalue {
                    name: "varData".into(),
                    value: SourceValue::LiveCapture {},
                },
            ],
        }],
        roots: vec![SourceProgramRoot {
            name: "definitions".into(),
            table: SourceTableId(1),
        }],
        intrinsics: BTreeMap::new(),
    }
}
fn prototypes() -> SourceClosurePrototypes {
    SourceClosurePrototypes {
        schema_version: SOURCE_CLOSURE_PROTOTYPES_SCHEMA_VERSION,
        prototypes: vec![SourceClosurePrototype {
            callback: SourceCallbackId(1),
        }],
    }
}
fn owner() -> SourceProgramOwner {
    SourceProgramOwner::new_with_closures(definitions(), None, None, prototypes()).unwrap()
}
fn expr(operation: SourceProgramExprKind) -> SourceProgramExpr {
    SourceProgramExpr {
        location: SourceProgramLocation { start: 0, end: 1 },
        operation,
    }
}
fn program(operation: SourceProgramStatementKind) -> SourceProgram {
    SourceProgram {
        callback: SourceCallbackId(1),
        parameter_count: 0,
        variadic: false,
        local_count: 0,
        bindings: vec![],
        body: vec![SourceProgramStatement {
            location: SourceProgramLocation { start: 0, end: 1 },
            operation,
        }],
        provenance: SourceProgramProvenance {
            source: span(),
            function_start: 0,
            function_end: 100,
            function_sha256: "d".repeat(64),
        },
    }
}
fn catalog(
    program: SourceProgram,
    owner: SourceProgramOwner,
) -> SourceProgramResult<SourceProgramCatalog> {
    SourceProgramCatalog::new(
        SourceProgramData {
            schema_version: SOURCE_PROGRAM_SCHEMA_VERSION,
            callbacks: BTreeMap::from([(program.callback, SourceProgramId(1))]),
            programs: vec![program],
        },
        owner,
    )
}
#[test]
fn prototype_layout_is_source_ordered_and_bound_to_immutable_owner_identity() {
    let data = definitions();
    let owner = owner();
    assert_eq!(owner.definitions(), Some(&data));
    assert_eq!(owner.closure_prototypes(), Some(&prototypes()));
    assert_eq!(
        owner.closure_prototype_id(SourceCallbackId(1)),
        Some(SourceClosurePrototypeId(1))
    );
    assert_eq!(owner.closure_prototype_id(SourceCallbackId(2)), None);
    let handle = owner
        .bind_closure_prototype(SourceClosurePrototypeId(1))
        .unwrap();
    assert_eq!(handle.capture_count(), 2);
    assert_eq!(handle.definition().callback, SourceCallbackId(1));
    assert_eq!(
        owner
            .callback(handle.definition().callback)
            .unwrap()
            .upvalues
            .iter()
            .map(|v| v.name.as_str())
            .collect::<Vec<_>>(),
        vec!["self", "varData"]
    );
    let clone = owner.clone();
    assert!(std::ptr::eq(
        owner.resolve_closure_prototype(&handle).unwrap(),
        clone.resolve_closure_prototype(&handle).unwrap()
    ));
    assert!(handle.owner().is_same_owner(&owner));
    let foreign = SourceProgramOwner::new_with_closures(data, None, None, prototypes()).unwrap();
    assert_eq!(
        foreign.resolve_closure_prototype(&handle).unwrap_err().kind,
        SourceProgramErrorKind::Binding
    );
    for id in [0, 2, u32::MAX] {
        assert!(
            owner
                .bind_closure_prototype(SourceClosurePrototypeId(id))
                .is_err()
        );
    }
}
#[test]
fn live_markers_require_declared_lua_upvalues_and_are_never_plain_values() {
    let data = definitions();
    // Structural graph shape is valid; only a prototype-bearing owner can bind it.
    data.validate().unwrap();
    assert!(
        SourceProgramOwner::new(data.clone())
            .unwrap_err()
            .message
            .contains("no declared")
    );
    assert!(
        SourceProgramOwner::new_with_context(data.clone(), None, SourceProgramContext::default())
            .is_err()
    );
    assert!(SourceProgramOwner::from_bytes(&serde_json::to_vec(&data).unwrap()).is_err());
    for indexed in [false, true] {
        let mut bad = data.clone();
        if indexed {
            bad.tables[0].indexed.insert(1, SourceValue::LiveCapture {});
        } else {
            bad.tables[0]
                .fields
                .insert("self".into(), SourceValue::LiveCapture {});
        }
        assert!(
            SourceProgramOwner::new_with_closures(bad, None, None, prototypes())
                .unwrap_err()
                .message
                .contains("standalone Lua upvalue")
        );
    }
    let mut bad = data;
    bad.callbacks[0].kind = SourceCallbackKind::Builtin {
        symbol: "tonumber".into(),
    };
    assert!(SourceProgramOwner::new_with_closures(bad, None, None, prototypes()).is_err());
}
#[test]
fn declarations_are_complete_unique_and_never_substitute_immutable_capture_values() {
    let data = definitions();
    for value in [
        SourceValue::Nil,
        SourceValue::Boolean(false),
        SourceValue::Table(SourceTableId(1)),
        SourceValue::Number(0.0),
    ] {
        let mut bad = data.clone();
        bad.callbacks[0].upvalues[1].value = value;
        assert!(
            SourceProgramOwner::new_with_closures(bad, None, None, prototypes())
                .unwrap_err()
                .message
                .contains("every ordered capture")
        );
    }
    let mut bad = prototypes();
    bad.prototypes.clear();
    assert!(
        bad.validate(&data)
            .unwrap_err()
            .message
            .contains("no declared")
    );
    let mut bad = prototypes();
    bad.prototypes.push(bad.prototypes[0].clone());
    assert!(
        bad.validate(&data)
            .unwrap_err()
            .message
            .contains("duplicate")
    );
    for id in [0, 2, u32::MAX] {
        let mut bad = prototypes();
        bad.prototypes[0].callback = SourceCallbackId(id);
        assert_eq!(
            bad.validate(&data).unwrap_err().kind,
            SourceProgramErrorKind::Binding
        );
    }
    let mut empty = data;
    empty.callbacks[0].upvalues.clear();
    let empty = SourceProgramOwner::new_with_closures(empty, None, None, prototypes()).unwrap();
    assert_eq!(
        empty
            .bind_closure_prototype(SourceClosurePrototypeId(1))
            .unwrap()
            .capture_count(),
        0
    );
}
#[test]
fn prototype_json_and_live_marker_are_strict_and_bounded() {
    let wire = serde_json::to_string(&prototypes()).unwrap();
    assert_eq!(
        SourceClosurePrototypes::from_bytes(wire.as_bytes(), &definitions()).unwrap(),
        prototypes()
    );
    for text in [
        wire.replace("\"schema_version\":1", "\"schema_version\":2"),
        wire.replace("\"callback\":1", "\"callback\":1,\"slot\":0"),
        wire.replacen('{', "{\"build_state\":{},", 1),
    ] {
        assert!(SourceClosurePrototypes::from_bytes(text.as_bytes(), &definitions()).is_err());
    }
    let marker = serde_json::to_string(&SourceValue::LiveCapture {}).unwrap();
    assert_eq!(marker, r#"{"kind":"live_capture","value":{}}"#);
    assert!(serde_json::from_str::<SourceValue>(&marker.replace("{}", r#"{"value":0}"#)).is_err());
    let mut many = prototypes();
    many.prototypes.resize(
        20_001,
        SourceClosurePrototype {
            callback: SourceCallbackId(1),
        },
    );
    assert_eq!(
        many.validate(&definitions()).unwrap_err().kind,
        SourceProgramErrorKind::ResourceLimit
    );
    let mut many = definitions();
    many.callbacks[0].upvalues.resize(
        129,
        SourceUpvalue {
            name: "slot".into(),
            value: SourceValue::LiveCapture {},
        },
    );
    assert_eq!(
        prototypes().validate(&many).unwrap_err().kind,
        SourceProgramErrorKind::ResourceLimit
    );
    assert_eq!(
        SourceClosurePrototypes::from_bytes(&vec![b' '; 4 * 1024 * 1024 + 1], &definitions())
            .unwrap_err()
            .kind,
        SourceProgramErrorKind::ResourceLimit
    );
    assert_eq!(
        serde_json::to_string(&SourceValue::Nil).unwrap(),
        r#"{"kind":"nil"}"#
    );
}
#[test]
fn capture_write_checks_declared_live_slot_and_all_rhs_scope_and_budgets() {
    let valid = program(SourceProgramStatementKind::CaptureSet {
        upvalue: 0,
        values: SourceProgramValueList::default(),
    });
    let wire = serde_json::to_string(&valid).unwrap();
    assert_eq!(serde_json::from_str::<SourceProgram>(&wire).unwrap(), valid);
    assert!(
        serde_json::from_str::<SourceProgram>(
            &wire.replace("\"upvalue\":0", "\"upvalue\":0,\"override\":true")
        )
        .is_err()
    );
    let verified = catalog(valid.clone(), owner()).unwrap();
    assert!(
        verified
            .required_capabilities()
            .contains(&SourceProgramCapability::SessionClosures)
    );
    assert_eq!(
        verified
            .check_capabilities(&BTreeSet::from([SourceProgramCapability::Core]))
            .unwrap_err()
            .kind,
        SourceProgramErrorKind::UnsupportedCapability
    );
    let read_only = program(SourceProgramStatementKind::Return {
        values: SourceProgramValueList {
            values: vec![expr(SourceProgramExprKind::Capture { upvalue: 0 })],
            tail: None,
        },
    });
    assert!(
        catalog(read_only, owner())
            .unwrap()
            .required_capabilities()
            .contains(&SourceProgramCapability::SessionClosures)
    );
    for slot in [2, u16::MAX] {
        assert_eq!(
            catalog(
                program(SourceProgramStatementKind::CaptureSet {
                    upvalue: slot,
                    values: SourceProgramValueList::default()
                }),
                owner()
            )
            .unwrap_err()
            .kind,
            SourceProgramErrorKind::Binding
        );
    }
    let first = expr(SourceProgramExprKind::Literal {
        value: ParserFactoryLiteral::Number(1.0),
    });
    for second in [
        expr(SourceProgramExprKind::Local { local: 0 }),
        expr(SourceProgramExprKind::Capture { upvalue: 5 }),
    ] {
        assert!(
            catalog(
                program(SourceProgramStatementKind::CaptureSet {
                    upvalue: 0,
                    values: SourceProgramValueList {
                        values: vec![first.clone(), second],
                        tail: None
                    }
                }),
                owner()
            )
            .is_err()
        );
    }
    assert_eq!(
        catalog(
            program(SourceProgramStatementKind::CaptureSet {
                upvalue: 0,
                values: SourceProgramValueList {
                    values: vec![first; 4097],
                    tail: None
                }
            }),
            owner()
        )
        .unwrap_err()
        .kind,
        SourceProgramErrorKind::ResourceLimit
    );
    let varargs = program(SourceProgramStatementKind::CaptureSet {
        upvalue: 0,
        values: SourceProgramValueList {
            values: vec![],
            tail: Some(Box::new(SourceProgramPack::Varargs)),
        },
    });
    assert!(catalog(varargs.clone(), owner()).is_err());
    let mut variadic = varargs;
    variadic.variadic = true;
    catalog(variadic, owner()).unwrap();
    let mut plain = definitions();
    plain.callbacks[0].upvalues[0].value = SourceValue::Nil;
    plain.callbacks[0].upvalues[1].value = SourceValue::Nil;
    assert_eq!(
        catalog(valid, SourceProgramOwner::new(plain).unwrap())
            .unwrap_err()
            .kind,
        SourceProgramErrorKind::UnsupportedCapability
    );
}
#[test]
fn session_artifact_keeps_live_values_and_aliases_out_of_shared_prototype_data() {
    let owner = owner();
    let wire = serde_json::to_vec(owner.definitions().unwrap()).unwrap();
    let prototype = owner
        .bind_closure_prototype(SourceClosurePrototypeId(1))
        .unwrap();
    let mut input = SourceSessionInput {
        owner: owner.clone(),
        traversal: None,
        state: SourceSessionValueGraph {
            values: vec![SourceSessionValue::Table(SourceSessionTableId(1))],
            tables: vec![SourceSessionTable {
                entries: vec![
                    (
                        SourceSessionValue::Bytes(b"self".to_vec()),
                        SourceSessionValue::Table(SourceSessionTableId(1)),
                    ),
                    (
                        SourceSessionValue::Bytes(b"first".to_vec()),
                        SourceSessionValue::Closure(SourceSessionClosureId(1)),
                    ),
                    (
                        SourceSessionValue::Bytes(b"second".to_vec()),
                        SourceSessionValue::Closure(SourceSessionClosureId(2)),
                    ),
                ],
            }],
        },
        coverage: SourceSessionCoverage::new(),
        class_bindings: SourceSessionClassBindings::new(),
        cells: vec![
            SourceSessionValue::Table(SourceSessionTableId(1)),
            SourceSessionValue::DefinitionTable(SourceTableId(1)),
            SourceSessionValue::DefinitionTable(SourceTableId(1)),
        ],
        closures: vec![
            SourceSessionClosure {
                prototype: prototype.clone(),
                captures: vec![SourceSessionCellId(1), SourceSessionCellId(2)],
            },
            SourceSessionClosure {
                prototype,
                captures: vec![SourceSessionCellId(1), SourceSessionCellId(3)],
            },
        ],
    };
    assert!(input.owner.is_same_owner(&owner));
    assert_eq!(input.closures[0].captures[0], input.closures[1].captures[0]);
    assert_ne!(input.closures[0].captures[1], input.closures[1].captures[1]);
    assert_eq!(input.cells[1], input.cells[2]); // Equal values do not merge cells.
    input.cells[0] = SourceSessionValue::Number(17.0);
    assert_eq!(
        serde_json::to_vec(owner.definitions().unwrap()).unwrap(),
        wire
    );
}
