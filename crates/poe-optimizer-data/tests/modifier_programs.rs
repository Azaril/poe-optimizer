use poe_optimizer_data::{game_data::bundled_snapshot, modifier_parser::*};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};
fn owner() -> ModifierParserCatalog {
    static OWNER: OnceLock<ModifierParserCatalog> = OnceLock::new();
    OWNER
        .get_or_init(|| {
            let mut data = bundled_snapshot().unwrap().modifier_parser().data().clone();
            // Standalone G1 authoring tests do not retain packaged programs/admissions.
            data.programs = ParserProgramPayload::default();
            ModifierParserCatalog::new(data).unwrap()
        })
        .clone()
}
fn location() -> ParserProgramLocation {
    ParserProgramLocation { start: 0, end: 1 }
}
fn expr(operation: ParserProgramExprKind) -> ParserProgramExpr {
    ParserProgramExpr {
        location: location(),
        operation,
    }
}
fn nil() -> ParserProgramExpr {
    expr(ParserProgramExprKind::Literal {
        value: ParserFactoryLiteral::Nil,
    })
}
fn local(local: u16) -> ParserProgramExpr {
    expr(ParserProgramExprKind::Local { local })
}
fn stmt(operation: ParserProgramStatementKind) -> ParserProgramStatement {
    ParserProgramStatement {
        location: location(),
        operation,
    }
}
fn values(values: Vec<ParserProgramExpr>) -> ParserProgramValueList {
    ParserProgramValueList { values, tail: None }
}
fn program(owner: &ModifierParserCatalog, callback: ParserCallbackId) -> ParserProgram {
    let ParserCallbackKind::Lua { source } = &owner.callback(callback).unwrap().kind else {
        panic!("Lua fixture")
    };
    // These are authored structural programs, not claims of source translation.
    ParserProgram {
        callback,
        parameter_count: 0,
        variadic: false,
        local_count: 0,
        bindings: vec![],
        body: vec![],
        provenance: ParserProgramProvenance {
            source: source.clone(),
            function_start: 0,
            function_end: 1,
            function_sha256: source.sha256.clone(),
        },
    }
}
fn callback(owner: &ModifierParserCatalog) -> ParserCallbackId {
    owner
        .data()
        .callbacks
        .iter()
        .enumerate()
        .find_map(|(i, c)| {
            let id = ParserCallbackId(i as u32 + 1);
            (matches!(c.kind, ParserCallbackKind::Lua { .. })
                && matches!(
                    owner.factory(id),
                    Some(ParserFactoryDisposition::Unsupported { .. })
                ))
            .then_some(id)
        })
        .unwrap()
}
fn data(program: ParserProgram) -> ParserProgramData {
    ParserProgramData {
        schema_version: PARSER_PROGRAM_SCHEMA_VERSION,
        callbacks: BTreeMap::from([(program.callback, ParserProgramId(1))]),
        programs: vec![program],
    }
}
#[test]
fn catalog_retains_exact_owner_identity_and_has_no_package_dispatch_side_effect() {
    let original = owner();
    let before = original.data().clone();
    let p = program(&original, callback(&original));
    let catalog = ParserProgramCatalog::new(data(p.clone()), original.clone()).unwrap();
    assert!(catalog.is_bound_to(&original));
    assert!(catalog.is_bound_to(catalog.owner()));
    let independent = ModifierParserCatalog::new(before.clone()).unwrap();
    assert!(!catalog.is_bound_to(&independent));
    assert_eq!(catalog.program_id(p.callback), Some(ParserProgramId(1)));
    assert_eq!(catalog.for_callback(p.callback), Some(&p));
    assert!(catalog.program(ParserProgramId(0)).is_none());
    assert_eq!(original.data(), &before);
    assert!(std::ptr::eq(
        catalog.definition(ParserProgramDefinitionRoot::GemIdLookup),
        original.dictionary(ParserDictionary::GemIdLookup)
    ));
}
#[test]
fn writable_parameters_nil_initialization_and_explicit_return_packs_roundtrip() {
    let owner = owner();
    let mut p = program(&owner, callback(&owner));
    p.parameter_count = 1;
    p.local_count = 2;
    p.body = vec![
        stmt(ParserProgramStatementKind::Assign {
            locals: vec![0],
            values: values(vec![nil()]),
        }),
        stmt(ParserProgramStatementKind::Declare {
            locals: vec![1],
            values: ParserProgramValueList::default(),
        }),
        stmt(ParserProgramStatementKind::Return {
            values: values(vec![
                local(1),
                expr(ParserProgramExprKind::Bytes {
                    value: vec![0, 255, 128],
                }),
            ]),
        }),
    ];
    let data = data(p);
    let bytes = serde_json::to_vec(&data).unwrap();
    let loaded = ParserProgramCatalog::from_bytes(&bytes, owner.clone()).unwrap();
    assert_eq!(loaded.data(), &data);
    let mut zero = data.clone();
    zero.programs[0].body = vec![stmt(ParserProgramStatementKind::Return {
        values: ParserProgramValueList::default(),
    })];
    let mut one = zero.clone();
    one.programs[0].body = vec![stmt(ParserProgramStatementKind::Return {
        values: values(vec![nil()]),
    })];
    assert_ne!(
        serde_json::to_vec(&zero).unwrap(),
        serde_json::to_vec(&one).unwrap()
    );
    ParserProgramCatalog::new(zero, owner.clone()).unwrap();
    ParserProgramCatalog::new(one, owner).unwrap();
}
#[test]
fn pattern_loop_binding_and_staged_capabilities_are_explicit() {
    let owner = owner();
    let mut p = program(&owner, callback(&owner));
    p.parameter_count = 1;
    p.local_count = 3;
    p.bindings = vec![ParserProgramBinding::Intrinsic {
        operation: ParserProgramIntrinsic::StringGmatch,
        source: ParserProgramIntrinsicSource::OriginalGlobal,
    }];
    p.bindings.push(ParserProgramBinding::Intrinsic {
        operation: ParserProgramIntrinsic::TableInsert,
        source: ParserProgramIntrinsicSource::OriginalGlobal,
    });
    p.body = vec![
        stmt(ParserProgramStatementKind::Declare {
            locals: vec![1],
            values: values(vec![expr(ParserProgramExprKind::Table { fields: vec![] })]),
        }),
        stmt(ParserProgramStatementKind::ForEach {
            locals: vec![2],
            iterator: ParserProgramIterator::Pattern {
                call: ParserProgramCall {
                    binding: 0,
                    receiver: Some(Box::new(local(0))),
                    arguments: values(vec![expr(ParserProgramExprKind::Bytes {
                        value: b"%w+".to_vec(),
                    })]),
                },
            },
            body: vec![stmt(ParserProgramStatementKind::TableAppend {
                binding: 1,
                table: local(1),
                value: local(2),
            })],
        }),
        stmt(ParserProgramStatementKind::Return {
            values: values(vec![local(1)]),
        }),
    ];
    let mut wrong = p.clone();
    let ParserProgramStatementKind::ForEach { body, .. } = &mut wrong.body[1].operation else {
        unreachable!()
    };
    let ParserProgramStatementKind::TableAppend { binding, .. } = &mut body[0].operation else {
        unreachable!()
    };
    *binding = 0;
    assert_eq!(
        ParserProgramCatalog::new(data(wrong), owner.clone())
            .unwrap_err()
            .kind,
        ParserProgramErrorKind::Binding
    );
    let c = ParserProgramCatalog::new(data(p), owner).unwrap();
    assert_eq!(
        c.required_capabilities(),
        &BTreeSet::from([
            ParserProgramCapability::Core,
            ParserProgramCapability::PatternFor
        ])
    );
    assert_eq!(
        c.check_capabilities(&BTreeSet::from([ParserProgramCapability::Core]))
            .unwrap_err()
            .kind,
        ParserProgramErrorKind::UnsupportedCapability
    );
    c.check_capabilities(c.required_capabilities()).unwrap();
}
#[test]
fn constructor_binding_uses_actual_capture_and_keeps_arity_and_nil_holes() {
    let owner = owner();
    let expected = owner
        .data()
        .source
        .construction_spans
        .get("create_mod")
        .unwrap();
    let (id, slot, target) = owner
        .data()
        .callbacks
        .iter()
        .enumerate()
        .find_map(|(i, c)| {
            let id = ParserCallbackId(i as u32 + 1);
            if !matches!(
                owner.factory(id),
                Some(ParserFactoryDisposition::Unsupported { .. })
            ) {
                return None;
            }
            c.upvalues.iter().enumerate().find_map(|(slot, u)| {
                let ParserValue::Callback(target) = u.value else {
                    return None;
                };
                (owner.callback(target).unwrap().kind
                    == ParserCallbackKind::Lua {
                        source: expected.clone(),
                    })
                .then_some((id, slot as u16, target))
            })
        })
        .unwrap();
    let mut p = program(&owner, id);
    p.variadic = true;
    p.bindings.push(ParserProgramBinding::Intrinsic {
        operation: ParserProgramIntrinsic::CreateMod,
        source: ParserProgramIntrinsicSource::Captured {
            upvalue: slot,
            callback: target,
        },
    });
    p.body.push(stmt(ParserProgramStatementKind::Return {
        values: ParserProgramValueList {
            values: vec![],
            tail: Some(Box::new(ParserProgramPack::Call {
                call: ParserProgramCall {
                    binding: 0,
                    receiver: None,
                    arguments: ParserProgramValueList {
                        values: vec![nil(), nil(), nil(), nil()],
                        tail: Some(Box::new(ParserProgramPack::Varargs)),
                    },
                },
            })),
        },
    }));
    let c = ParserProgramCatalog::new(data(p), owner).unwrap();
    assert!(
        c.required_capabilities()
            .contains(&ParserProgramCapability::Varargs)
    );
    let bytes = serde_json::to_vec(c.data()).unwrap();
    assert_eq!(
        ParserProgramCatalog::from_bytes(&bytes, c.owner().clone())
            .unwrap()
            .data(),
        c.data()
    );
}
#[test]
fn final_constructor_pack_expansion_is_distinct_from_literal_nil_fields() {
    let owner = owner();
    let mut p = program(&owner, callback(&owner));
    p.variadic = true;
    p.body = vec![stmt(ParserProgramStatementKind::Return {
        values: values(vec![expr(ParserProgramExprKind::Table {
            fields: vec![
                ParserProgramField::List { value: nil() },
                ParserProgramField::Tail {
                    values: ParserProgramPack::Varargs,
                },
            ],
        })]),
    })];
    ParserProgramCatalog::new(data(p.clone()), owner.clone()).unwrap();
    let ParserProgramStatementKind::Return { values } = &mut p.body[0].operation else {
        unreachable!()
    };
    let ParserProgramExprKind::Table { fields } = &mut values.values[0].operation else {
        unreachable!()
    };
    fields.swap(0, 1);
    assert_eq!(
        ParserProgramCatalog::new(data(p), owner).unwrap_err().kind,
        ParserProgramErrorKind::InvalidData
    );
}
#[test]
fn generic_helper_calls_use_the_callees_own_source_and_scope() {
    let original = owner();
    let mut raw = original.data().clone();
    let id = callback(&original);
    let other = raw
        .callbacks
        .iter()
        .enumerate()
        .find_map(|(i, c)| {
            let candidate = ParserCallbackId(i as u32 + 1);
            (candidate != id
                && matches!(c.kind, ParserCallbackKind::Lua { .. })
                && matches!(
                    original.factory(candidate),
                    Some(ParserFactoryDisposition::Unsupported { .. })
                ))
            .then_some(candidate)
        })
        .unwrap();
    let slot = raw.callbacks[id.0 as usize - 1].upvalues.len() as u16;
    raw.callbacks[id.0 as usize - 1]
        .upvalues
        .push(ParserUpvalue {
            name: "authored_helper".into(),
            value: ParserValue::Callback(other),
        });
    let owner = ModifierParserCatalog::new(raw).unwrap();
    let mut p = program(&owner, id);
    p.bindings.push(ParserProgramBinding::CapturedCallback {
        upvalue: slot,
        callback: other,
    });
    p.body.push(stmt(ParserProgramStatementKind::Return {
        values: ParserProgramValueList {
            values: vec![],
            tail: Some(Box::new(ParserProgramPack::Call {
                call: ParserProgramCall {
                    binding: 0,
                    receiver: None,
                    arguments: ParserProgramValueList::default(),
                },
            })),
        },
    }));
    let helper = program(&owner, other);
    let data = ParserProgramData {
        schema_version: 1,
        programs: vec![p, helper.clone()],
        callbacks: BTreeMap::from([(id, ParserProgramId(1)), (other, ParserProgramId(2))]),
    };
    let c = ParserProgramCatalog::new(data, owner).unwrap();
    assert_eq!(c.program(ParserProgramId(2)), Some(&helper));
    assert!(
        !c.required_capabilities()
            .contains(&ParserProgramCapability::LegacyPureCalls)
    );
}
#[test]
fn recursive_calls_are_detected_iteratively_and_require_separate_capability() {
    let original = owner();
    let id = callback(&original);
    let mut raw = original.data().clone();
    let slot = raw.callbacks[id.0 as usize - 1].upvalues.len() as u16;
    raw.callbacks[id.0 as usize - 1]
        .upvalues
        .push(ParserUpvalue {
            name: "authored_self".into(),
            value: ParserValue::Callback(id),
        });
    let owner = ModifierParserCatalog::new(raw).unwrap();
    let mut p = program(&owner, id);
    p.bindings.push(ParserProgramBinding::CapturedCallback {
        upvalue: slot,
        callback: id,
    });
    p.body.push(stmt(ParserProgramStatementKind::Call {
        call: ParserProgramCall {
            binding: 0,
            receiver: None,
            arguments: ParserProgramValueList::default(),
        },
    }));
    let c = ParserProgramCatalog::new(data(p), owner).unwrap();
    assert!(
        c.required_capabilities()
            .contains(&ParserProgramCapability::RecursiveCalls)
    );
    assert_eq!(
        c.check_capabilities(&BTreeSet::from([ParserProgramCapability::Core]))
            .unwrap_err()
            .kind,
        ParserProgramErrorKind::UnsupportedCapability
    );
}
#[test]
fn decoding_rejects_duplicate_mappings_unknown_fields_and_omitted_pack_adjustment() {
    let owner = owner();
    let mut p = program(&owner, callback(&owner));
    p.body.push(stmt(ParserProgramStatementKind::Return {
        values: ParserProgramValueList::default(),
    }));
    let d = data(p);
    let mut v = serde_json::to_value(&d).unwrap();
    v["programs"][0]["body"][0]["operation"]["values"]
        .as_object_mut()
        .unwrap()
        .remove("tail");
    assert!(
        ParserProgramCatalog::from_bytes(&serde_json::to_vec(&v).unwrap(), owner.clone()).is_err()
    );
    let mut v = serde_json::to_value(&d).unwrap();
    v["programs"][0]["invented"] = true.into();
    assert!(
        ParserProgramCatalog::from_bytes(&serde_json::to_vec(&v).unwrap(), owner.clone()).is_err()
    );
    let raw = serde_json::to_string(&d).unwrap();
    let id = d.programs[0].callback.0;
    let key = format!("\"callbacks\":{{\"{id}\":1}}");
    let duplicate = raw.replace(&key, &format!("\"callbacks\":{{\"{id}\":1,\"{id}\":1}}"));
    assert_ne!(raw, duplicate);
    assert!(ParserProgramCatalog::from_bytes(duplicate.as_bytes(), owner).is_err());
}
