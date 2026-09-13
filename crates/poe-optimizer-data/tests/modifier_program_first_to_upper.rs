//! Authored structural binding tests, not source-translation or runtime parity.
use poe_optimizer_data::{
    game_data::bundled_snapshot,
    modifier_parser::*,
    source_program::{
        SOURCE_PROGRAM_DEFINITIONS_SCHEMA_VERSION, SourceProgramCatalog, SourceProgramDefinitions,
        SourceProgramOwner,
    },
};
use std::{collections::BTreeMap, sync::OnceLock};

fn definitions() -> ModifierParserData {
    static DATA: OnceLock<ModifierParserData> = OnceLock::new();
    DATA.get_or_init(|| {
        let mut data = bundled_snapshot().unwrap().modifier_parser().data().clone();
        data.programs = ParserProgramPayload::default();
        // Keep negative helper proofs independent of legacy factory validation.
        for factory in data.factories.values_mut() {
            *factory = ParserFactoryDisposition::Unsupported {
                reason: "authored binding fixture".into(),
            };
        }
        data
    })
    .clone()
}

fn program(data: &ModifierParserData) -> ParserProgram {
    let helper = data.helpers["firstToUpper"];
    let (caller, slot) = data
        .callbacks
        .iter()
        .enumerate()
        .find_map(|(index, callback)| {
            callback
                .upvalues
                .iter()
                .position(|u| u.value == ParserValue::Callback(helper))
                .map(|slot| (ParserCallbackId(index as u32 + 1), slot as u16))
        })
        .unwrap();
    let ParserCallbackKind::Lua { source } = &data.callbacks[caller.0 as usize - 1].kind else {
        panic!("Lua caller");
    };
    ParserProgram {
        callback: caller,
        parameter_count: 0,
        variadic: true,
        local_count: 0,
        bindings: vec![ParserProgramBinding::Intrinsic {
            operation: ParserProgramIntrinsic::FirstToUpper,
            source: ParserProgramIntrinsicSource::Captured {
                upvalue: slot,
                callback: helper,
            },
        }],
        body: vec![ParserProgramStatement {
            location: ParserProgramLocation { start: 0, end: 1 },
            operation: ParserProgramStatementKind::Return {
                values: ParserProgramValueList {
                    values: vec![],
                    tail: Some(Box::new(ParserProgramPack::Call {
                        call: ParserProgramCall {
                            binding: 0,
                            receiver: None,
                            arguments: ParserProgramValueList {
                                values: vec![],
                                tail: Some(Box::new(ParserProgramPack::Varargs)),
                            },
                        },
                    })),
                },
            },
        }],
        provenance: ParserProgramProvenance {
            source: source.clone(),
            function_start: 0,
            function_end: 1,
            function_sha256: source.sha256.clone(),
        },
    }
}

fn programs(program: ParserProgram) -> ParserProgramData {
    ParserProgramData {
        schema_version: PARSER_PROGRAM_SCHEMA_VERSION,
        callbacks: BTreeMap::from([(program.callback, ParserProgramId(1))]),
        programs: vec![program],
    }
}

#[test]
fn closed_helper_borrows_injected_pattern_and_roundtrips_without_admission() {
    let baseline = bundled_snapshot().unwrap().modifier_parser().clone();
    let admissions = baseline.data().programs.admissions.clone();
    for pattern in ["^%l", "", "[", "(.)"] {
        let mut data = definitions();
        data.policy.first_to_upper_pattern = pattern.into();
        let helper = data.helpers["firstToUpper"];
        let p = program(&data);
        let owner = ModifierParserCatalog::new(data).unwrap();
        let neutral = SourceProgramOwner::from_parser(owner.clone());
        let borrowed = neutral.first_to_upper_pattern(helper).unwrap();
        assert_eq!(borrowed, pattern);
        assert!(std::ptr::eq(
            borrowed,
            owner.data().policy.first_to_upper_pattern.as_str()
        ));
        assert_eq!(neutral.first_to_upper_pattern(ParserCallbackId(0)), None);
        let c = ParserProgramCatalog::new(programs(p), owner.clone()).unwrap();
        let encoded = serde_json::to_vec(c.data()).unwrap();
        assert!(
            std::str::from_utf8(&encoded)
                .unwrap()
                .contains("\"first_to_upper\"")
        );
        assert_eq!(
            ParserProgramCatalog::from_bytes(&encoded, owner.clone())
                .unwrap()
                .data(),
            c.data()
        );
        assert!(
            !c.required_capabilities()
                .contains(&ParserProgramCapability::LegacyPureCalls)
        );
        // Exercise the borrowed owner view used before its catalog Arc exists.
        let mut embedded = owner.data().clone();
        embedded.programs.data = c.data().clone();
        let embedded = ModifierParserCatalog::new(embedded).unwrap();
        assert!(embedded.data().programs.admissions.is_empty());
    }
    assert_eq!(baseline.data().programs.admissions, admissions);
}

#[test]
fn captured_identity_global_and_method_substitutions_are_rejected() {
    let data = definitions();
    let original = program(&data);
    let owner = ModifierParserCatalog::new(data).unwrap();
    for case in 0..3 {
        let mut p = original.clone();
        match &mut p.bindings[0] {
            ParserProgramBinding::Intrinsic { source, .. } => match case {
                0 => *source = ParserProgramIntrinsicSource::OriginalGlobal,
                1 => {
                    *source = ParserProgramIntrinsicSource::Captured {
                        upvalue: u16::MAX,
                        callback: owner.data().helpers["firstToUpper"],
                    }
                }
                _ => {
                    *source = ParserProgramIntrinsicSource::Captured {
                        upvalue: 0,
                        callback: ParserCallbackId(0),
                    }
                }
            },
            _ => unreachable!(),
        }
        assert_eq!(
            ParserProgramCatalog::new(programs(p), owner.clone())
                .unwrap_err()
                .kind,
            ParserProgramErrorKind::Binding
        );
    }
    let mut method = original;
    let ParserProgramStatementKind::Return { values } = &mut method.body[0].operation else {
        unreachable!()
    };
    let ParserProgramPack::Call { call } = values.tail.as_deref_mut().unwrap() else {
        unreachable!()
    };
    call.receiver = Some(Box::new(ParserProgramExpr {
        location: ParserProgramLocation { start: 0, end: 1 },
        operation: ParserProgramExprKind::Literal {
            value: ParserFactoryLiteral::Nil,
        },
    }));
    assert_eq!(
        ParserProgramCatalog::new(programs(method), owner)
            .unwrap_err()
            .kind,
        ParserProgramErrorKind::Binding
    );
    assert_eq!(ParserProgramIntrinsic::FirstToUpper.global_path(), None);
    assert_eq!(ParserProgramIntrinsic::FirstToUpper.builtin_symbol(), None);
    assert!(!ParserProgramIntrinsic::FirstToUpper.is_string_method());
}

#[test]
fn helper_registry_source_and_closed_capture_proofs_are_required() {
    let original = definitions();
    let helper = original.helpers["firstToUpper"];
    let p = program(&original);
    for case in 0..6 {
        let mut data = original.clone();
        match case {
            0 => {
                data.helpers.remove("firstToUpper");
            }
            1 => {
                data.helpers.insert("firstToUpper".into(), p.callback);
            }
            2 => {
                data.source
                    .construction_spans
                    .remove("first_to_upper_primitive");
            }
            3 => {
                data.source
                    .construction_spans
                    .get_mut("first_to_upper_primitive")
                    .unwrap()
                    .sha256 = "0".repeat(64);
            }
            4 => {
                data.callbacks[helper.0 as usize - 1].kind = ParserCallbackKind::Builtin {
                    symbol: "string.upper".into(),
                };
            }
            _ => {
                data.callbacks[helper.0 as usize - 1]
                    .upvalues
                    .push(ParserUpvalue {
                        name: "captured".into(),
                        value: ParserValue::Nil,
                    });
            }
        }
        let owner = ModifierParserCatalog::new(data).unwrap();
        assert_eq!(
            SourceProgramOwner::from_parser(owner.clone()).first_to_upper_pattern(helper),
            None
        );
        assert_eq!(
            ParserProgramCatalog::new(programs(p.clone()), owner)
                .unwrap_err()
                .kind,
            ParserProgramErrorKind::Binding
        );
    }
    let mut callback = serde_json::to_value(&original.callbacks[helper.0 as usize - 1]).unwrap();
    callback["environment"] = serde_json::json!("custom_globals");
    assert!(serde_json::from_value::<ParserCallback>(callback).is_err());
}

#[test]
fn copied_source_spans_cannot_grant_standalone_owner_a_parser_helper() {
    let data = definitions();
    let mut p = program(&data);
    let helper = data.helpers["firstToUpper"];
    let mut caller = data.callbacks[p.callback.0 as usize - 1].clone();
    caller.upvalues = vec![ParserUpvalue {
        name: "firstToUpper".into(),
        value: ParserValue::Callback(ParserCallbackId(2)),
    }];
    p.callback = ParserCallbackId(1);
    p.bindings[0] = ParserProgramBinding::Intrinsic {
        operation: ParserProgramIntrinsic::FirstToUpper,
        source: ParserProgramIntrinsicSource::Captured {
            upvalue: 0,
            callback: ParserCallbackId(2),
        },
    };
    let standalone = SourceProgramDefinitions {
        schema_version: SOURCE_PROGRAM_DEFINITIONS_SCHEMA_VERSION,
        source: data.source.clone(),
        tables: vec![ParserTable {
            fields: BTreeMap::new(),
            indexed: BTreeMap::new(),
        }],
        callbacks: vec![caller, data.callbacks[helper.0 as usize - 1].clone()],
        roots: vec![],
        intrinsics: BTreeMap::new(),
    };
    let owner = SourceProgramOwner::new(standalone.clone()).unwrap();
    assert_eq!(owner.first_to_upper_pattern(ParserCallbackId(2)), None);
    assert_eq!(
        SourceProgramCatalog::new(programs(p), owner)
            .unwrap_err()
            .kind,
        ParserProgramErrorKind::Binding
    );
    let mut claimed_builtin = standalone;
    claimed_builtin
        .intrinsics
        .insert(ParserCallbackId(2), ParserProgramIntrinsic::FirstToUpper);
    assert_eq!(
        SourceProgramOwner::new(claimed_builtin).unwrap_err().kind,
        ParserProgramErrorKind::Binding
    );
}
