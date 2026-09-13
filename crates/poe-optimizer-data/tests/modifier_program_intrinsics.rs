//! Authored authority/binding contracts, not source acquisition or runtime parity.
use poe_optimizer_data::{
    game_data::bundled_snapshot,
    modifier_parser::*,
    source_program::{
        SOURCE_PROGRAM_DEFINITIONS_SCHEMA_VERSION, SourceProgramDefinitions, SourceProgramOwner,
    },
};
use std::{collections::BTreeMap, sync::OnceLock};

fn definitions() -> ModifierParserData {
    static DATA: OnceLock<ModifierParserData> = OnceLock::new();
    DATA.get_or_init(|| {
        let mut data = bundled_snapshot().unwrap().modifier_parser().data().clone();
        data.programs = ParserProgramPayload::default();
        data.program_intrinsics.clear();
        for factory in data.factories.values_mut() {
            *factory = ParserFactoryDisposition::Unsupported {
                reason: "authored primitive authority fixture".into(),
            };
        }
        data
    })
    .clone()
}

fn target(data: &ModifierParserData) -> ParserCallbackId {
    let matches: Vec<_> = data
        .callbacks
        .iter()
        .enumerate()
        .filter(|(_, callback)| {
            matches!(
                &callback.kind,
                ParserCallbackKind::Builtin { symbol } if symbol == "table.insert"
            )
        })
        .map(|(i, _)| ParserCallbackId(i as u32 + 1))
        .collect();
    assert_eq!(matches.len(), 1);
    matches[0]
}

fn program(data: &ModifierParserData) -> ParserProgram {
    let target = target(data);
    let (caller, slot) = data
        .callbacks
        .iter()
        .enumerate()
        .find_map(|(i, callback)| {
            callback
                .upvalues
                .iter()
                .position(|u| u.value == ParserValue::Callback(target))
                .map(|slot| (ParserCallbackId(i as u32 + 1), slot as u16))
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
            operation: ParserProgramIntrinsic::TableInsert,
            source: ParserProgramIntrinsicSource::Captured {
                upvalue: slot,
                callback: target,
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
fn explicit_authority_is_required_and_preserves_exact_captured_identity() {
    let mut data = definitions();
    let id = target(&data);
    let p = program(&data);
    let owner = ModifierParserCatalog::new(data.clone()).unwrap();
    assert_eq!(
        SourceProgramOwner::from_parser(owner.clone()).intrinsic(id),
        None
    );
    assert_eq!(
        ParserProgramCatalog::new(programs(p.clone()), owner)
            .unwrap_err()
            .kind,
        ParserProgramErrorKind::UnsupportedCapability
    );

    data.program_intrinsics
        .insert(id, ParserProgramIntrinsic::TableInsert);
    let ParserProgramBinding::Intrinsic {
        source: ParserProgramIntrinsicSource::Captured { upvalue, .. },
        ..
    } = p.bindings[0]
    else {
        panic!("captured primitive");
    };
    // An alias name does not change the exact callback identity in this slot.
    data.callbacks[p.callback.0 as usize - 1].upvalues[upvalue as usize].name =
        "caller_selected_alias".into();
    let owner = ModifierParserCatalog::new(data.clone()).unwrap();
    let neutral = SourceProgramOwner::from_parser(owner.clone());
    assert_eq!(
        neutral.intrinsic(id),
        Some(ParserProgramIntrinsic::TableInsert)
    );
    assert_eq!(neutral.intrinsic(ParserCallbackId(0)), None);
    let catalog = ParserProgramCatalog::new(programs(p.clone()), owner.clone()).unwrap();
    assert!(
        !catalog
            .required_capabilities()
            .contains(&ParserProgramCapability::LegacyPureCalls)
    );
    assert!(owner.data().programs.admissions.is_empty());

    // Exercise the private borrowed owner view before catalog Arc construction.
    data.programs.data = programs(p.clone());
    ModifierParserCatalog::new(data).unwrap();

    for source in [
        ParserProgramIntrinsicSource::Captured {
            upvalue: u16::MAX,
            callback: id,
        },
        ParserProgramIntrinsicSource::Captured {
            upvalue,
            callback: p.callback,
        },
    ] {
        let mut wrong = p.clone();
        wrong.bindings[0] = ParserProgramBinding::Intrinsic {
            operation: ParserProgramIntrinsic::TableInsert,
            source,
        };
        assert_eq!(
            ParserProgramCatalog::new(programs(wrong), owner.clone())
                .unwrap_err()
                .kind,
            ParserProgramErrorKind::Binding
        );
    }
}

#[test]
fn parser_map_rejects_wrong_operations_targets_and_ambiguous_builtin_identity() {
    let original = definitions();
    let id = target(&original);
    for operation in [
        ParserProgramIntrinsic::CreateMod,
        ParserProgramIntrinsic::FirstToUpper,
        ParserProgramIntrinsic::Ipairs,
        ParserProgramIntrinsic::StringGsub,
        ParserProgramIntrinsic::ToNumber,
    ] {
        let mut data = original.clone();
        data.program_intrinsics.insert(id, operation);
        assert!(data.validate().is_err());
    }
    for id in [
        ParserCallbackId(0),
        ParserCallbackId(u32::MAX),
        original.helpers["firstToUpper"],
    ] {
        let mut data = original.clone();
        data.program_intrinsics
            .insert(id, ParserProgramIntrinsic::TableInsert);
        assert!(data.validate().is_err());
    }
    for case in 0..4 {
        let mut data = original.clone();
        data.program_intrinsics
            .insert(id, ParserProgramIntrinsic::TableInsert);
        match case {
            0 => {
                data.callbacks[id.0 as usize - 1].kind = ParserCallbackKind::Builtin {
                    symbol: "table.remove".into(),
                }
            }
            1 => data.callbacks[id.0 as usize - 1]
                .upvalues
                .push(ParserUpvalue {
                    name: "captured".into(),
                    value: ParserValue::Nil,
                }),
            _ => {
                data.callbacks
                    .push(data.callbacks[id.0 as usize - 1].clone());
                let duplicate = ParserCallbackId(data.callbacks.len() as u32);
                data.factories.insert(
                    duplicate,
                    ParserFactoryDisposition::Unsupported {
                        reason: "duplicate original primitive claim".into(),
                    },
                );
                if case == 3 {
                    data.program_intrinsics
                        .insert(duplicate, ParserProgramIntrinsic::TableInsert);
                }
            }
        }
        assert!(data.validate().is_err(), "case {case}");
    }

    // The environment enum currently admits only OriginalGlobals.
    let mut encoded = serde_json::to_value(&original).unwrap();
    encoded["callbacks"][id.0 as usize - 1]["environment"] = "foreign_globals".into();
    assert!(serde_json::from_value::<ModifierParserData>(encoded).is_err());
}

#[test]
fn required_unique_map_roundtrips_and_enters_definition_and_admission_identity() {
    let mut data = definitions();
    let id = target(&data);
    let encoded = serde_json::to_string(&data).unwrap();
    assert!(encoded.contains("\"program_intrinsics\":{}"));
    for replacement in [
        "\"program_intrinsics\":null".to_owned(),
        format!(
            "\"program_intrinsics\":{{\"{}\":\"table_insert\",\"{}\":\"table_insert\"}}",
            id.0, id.0
        ),
    ] {
        let invalid = encoded.replacen("\"program_intrinsics\":{}", &replacement, 1);
        assert!(serde_json::from_str::<ModifierParserData>(&invalid).is_err());
    }
    let mut missing = serde_json::to_value(&data).unwrap();
    missing
        .as_object_mut()
        .unwrap()
        .remove("program_intrinsics");
    assert!(serde_json::from_value::<ModifierParserData>(missing).is_err());

    // Existing OriginalGlobal behavior remains independent of the captured map.
    let mut p = program(&data);
    p.bindings[0] = ParserProgramBinding::Intrinsic {
        operation: ParserProgramIntrinsic::TableInsert,
        source: ParserProgramIntrinsicSource::OriginalGlobal,
    };
    let admission = ParserProgramAdmission::bind(
        &data,
        &p,
        ParserProgramRole::Helper,
        "authored identity test",
    )
    .unwrap();
    data.programs.data = programs(p.clone());
    data.programs.admissions.insert(p.callback, admission);
    data.validate().unwrap();
    let before = data.definition_sha256().unwrap();
    data.program_intrinsics
        .insert(id, ParserProgramIntrinsic::TableInsert);
    assert_ne!(data.definition_sha256().unwrap(), before);
    assert!(
        data.validate()
            .unwrap_err()
            .to_string()
            .contains("stale admission")
    );

    data.programs.admissions.clear();
    data.validate().unwrap();
    let bytes = data.definition_bytes().unwrap();
    let projection: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        projection["program_intrinsics"][id.0.to_string()],
        "table_insert"
    );
    assert!(projection.get("programs").is_none());
    let encoded = serde_json::to_vec(&data).unwrap();
    let decoded: ModifierParserData = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(data, decoded);
    decoded.validate().unwrap();
}

#[test]
fn standalone_intrinsic_authority_remains_separate() {
    let data = definitions();
    let id = target(&data);
    let mut standalone = SourceProgramDefinitions {
        schema_version: SOURCE_PROGRAM_DEFINITIONS_SCHEMA_VERSION,
        source: data.source.clone(),
        tables: data.tables.clone(),
        callbacks: data.callbacks.clone(),
        roots: vec![],
        intrinsics: BTreeMap::new(),
    };
    let owner = SourceProgramOwner::new(standalone.clone()).unwrap();
    assert_eq!(owner.intrinsic(id), None);
    standalone
        .intrinsics
        .insert(id, ParserProgramIntrinsic::TableInsert);
    let owner = SourceProgramOwner::new(standalone).unwrap();
    assert_eq!(
        owner.intrinsic(id),
        Some(ParserProgramIntrinsic::TableInsert)
    );
    // Standalone authority does not mutate or confer authority on a parser owner.
    let parser = SourceProgramOwner::from_parser(ModifierParserCatalog::new(data).unwrap());
    assert_eq!(parser.intrinsic(id), None);
}
