use poe_optimizer_data::{
    game_data::bundled_snapshot,
    item_loading::{ItemLoadingSource, ItemSourceSpan},
    modifier_parser::ParserFactoryLiteral,
    source_program::*,
};
use std::collections::BTreeMap;
fn span() -> ItemSourceSpan {
    ItemSourceSpan {
        path: "fixture.lua".into(),
        line: 1,
        end_line: 20,
        sha256: "b".repeat(64),
    }
}
fn owner() -> SourceProgramOwner {
    SourceProgramOwner::new(SourceProgramDefinitions {
        schema_version: SOURCE_PROGRAM_DEFINITIONS_SCHEMA_VERSION,
        source: ItemLoadingSource {
            upstream_revision: "a".repeat(40),
            files: [("fixture.lua".into(), "c".repeat(64))].into(),
            construction_spans: BTreeMap::new(),
            module_order: vec!["fixture.lua".into()],
        },
        tables: vec![],
        callbacks: vec![SourceCallback {
            kind: SourceCallbackKind::Lua { source: span() },
            upvalues: vec![],
            environment: SourceEnvironment::OriginalGlobals,
        }],
        roots: vec![],
        intrinsics: BTreeMap::new(),
    })
    .unwrap()
}
fn provenance() -> SourceProgramProvenance {
    SourceProgramProvenance {
        source: span(),
        function_start: 0,
        function_end: 100,
        function_sha256: "d".repeat(64),
    }
}
fn e(operation: SourceProgramExprKind, start: u32, end: u32) -> SourceProgramExpr {
    SourceProgramExpr {
        location: SourceProgramLocation { start, end },
        operation,
    }
}
fn table() -> SourceProgramExpr {
    e(SourceProgramExprKind::Table { fields: vec![] }, 5, 7)
}
fn nil() -> SourceProgramExpr {
    e(
        SourceProgramExprKind::Literal {
            value: ParserFactoryLiteral::Nil,
        },
        1,
        2,
    )
}
fn values(value: SourceProgramExpr) -> SourceProgramValueList {
    SourceProgramValueList {
        values: vec![value],
        tail: None,
    }
}
fn statement(operation: SourceProgramStatementKind) -> SourceProgramStatement {
    SourceProgramStatement {
        location: SourceProgramLocation { start: 0, end: 90 },
        operation,
    }
}
fn program(body: Vec<SourceProgramStatement>) -> SourceProgramData {
    SourceProgramData {
        schema_version: SOURCE_PROGRAM_SCHEMA_VERSION,
        programs: vec![SourceProgram {
            callback: SourceCallbackId(1),
            parameter_count: 0,
            variadic: true,
            local_count: 3,
            bindings: vec![
                SourceProgramBinding::DynamicCall {},
                SourceProgramBinding::Intrinsic {
                    operation: SourceProgramIntrinsic::StringGmatch,
                    source: SourceProgramIntrinsicSource::OriginalGlobal,
                },
                SourceProgramBinding::Intrinsic {
                    operation: SourceProgramIntrinsic::Ipairs,
                    source: SourceProgramIntrinsicSource::OriginalGlobal,
                },
            ],
            body,
            provenance: provenance(),
        }],
        callbacks: [(SourceCallbackId(1), SourceProgramId(1))].into(),
    }
}
fn data() -> SourceProgramData {
    program(vec![statement(SourceProgramStatementKind::Return {
        values: values(table()),
    })])
}
fn metadata() -> SourceProgramConstructors {
    SourceProgramConstructors {
        schema_version: SOURCE_PROGRAM_CONSTRUCTORS_SCHEMA_VERSION,
        profile: SourceTableRuntimeProfile::luajit21_x64_single(),
        sites: vec![SourceProgramConstructor {
            callback: SourceCallbackId(1),
            provenance: provenance(),
            expression: SourceProgramLocation { start: 5, end: 7 },
            bytecode_sha256: "e".repeat(64),
            bytecode_pc: 1,
            instruction: 52,
            allocation: SourceTableAllocation::New {
                array_slots: 0,
                hash_bits: 0,
            },
        }],
    }
}
fn bind(
    data: SourceProgramData,
    meta: SourceProgramConstructors,
) -> SourceProgramResult<SourceProgramCatalog> {
    SourceProgramCatalog::new_with_constructors(data, owner(), meta)
}
#[test]
fn constructor_sidecar_binds_fresh_catalog_without_changing_owner_or_legacy_wire() {
    let owner = owner();
    let input = data();
    let before = serde_json::to_vec(&input).unwrap();
    let defs = serde_json::to_vec(owner.definitions().unwrap()).unwrap();
    let ordinary = SourceProgramCatalog::new(input.clone(), owner.clone()).unwrap();
    assert!(ordinary.constructors().is_none());
    let meta = metadata();
    let bound =
        SourceProgramCatalog::new_with_constructors(input, owner.clone(), meta.clone()).unwrap();
    assert_eq!(bound.constructors(), Some(&meta));
    assert!(bound.owner().is_same_owner(&owner));
    assert_eq!(serde_json::to_vec(bound.data()).unwrap(), before);
    assert_eq!(
        serde_json::to_vec(owner.definitions().unwrap()).unwrap(),
        defs
    );
    assert!(ordinary.constructors().is_none());
    assert!(
        !std::ptr::eq(ordinary.data(), bound.data()),
        "binding owns a fresh IR Arc"
    );
    assert!(std::ptr::eq(bound.data(), bound.clone().data()));
    let reloaded = SourceProgramCatalog::from_bytes(&before, owner).unwrap();
    assert!(reloaded.constructors().is_none());
}
#[test]
fn empty_sidecar_adds_no_sites_and_profiles_are_explicit_claims() {
    let mut meta = metadata();
    meta.sites.clear();
    assert!(
        bind(data(), meta.clone())
            .unwrap()
            .constructors()
            .unwrap()
            .sites
            .is_empty()
    );
    let supported = SourceTableRuntimeProfile::luajit21_x64_single();
    assert!(supported.is_supported_array_profile());
    let variants = [
        {
            let mut p = supported.clone();
            p.architecture = SourceRuntimeArchitecture::Arm64;
            p
        },
        {
            let mut p = supported.clone();
            p.number_mode = SourceNumberMode::Dual;
            p
        },
        {
            let mut p = supported.clone();
            p.endian = SourceEndianness::Big;
            p
        },
        {
            let mut p = supported.clone();
            p.gc64 = false;
            p
        },
        {
            let mut p = supported.clone();
            p.frame_slots = 1;
            p
        },
        {
            let mut p = supported.clone();
            p.table_bump = true;
            p
        },
        {
            let mut p = supported.clone();
            p.bytecode_version = 3;
            p
        },
        {
            let mut p = supported.clone();
            p.lua52_compat = true;
            p
        },
        {
            let mut p = supported.clone();
            p.source_revision = "other-runtime-revision".into();
            p
        },
    ];
    for profile in variants {
        assert!(!profile.is_supported_array_profile());
        meta.profile = profile;
        bind(data(), meta.clone()).unwrap();
    }
    for revision in ["", "bad revision", "bad\0revision"] {
        meta.profile = supported.clone();
        meta.profile.source_revision = revision.into();
        assert!(bind(data(), meta.clone()).is_err());
    }
    meta.profile = supported;
    meta.profile.frame_slots = 0;
    assert!(bind(data(), meta.clone()).is_err());
    meta.profile.frame_slots = 3;
    assert!(bind(data(), meta.clone()).is_err());
    meta.profile.frame_slots = 2;
    meta.profile.bytecode_version = 0;
    assert!(bind(data(), meta).is_err());
}
#[test]
fn rejects_foreign_callback_provenance_and_non_table_or_ambiguous_ranges() {
    for callback in [0, 2] {
        let mut m = metadata();
        m.sites[0].callback = SourceCallbackId(callback);
        assert!(bind(data(), m).is_err());
    }
    let mut m = metadata();
    m.sites[0].provenance.function_sha256 = "f".repeat(64);
    assert!(bind(data(), m).is_err());
    let mut m = metadata();
    m.sites[0].provenance.source.sha256 = "f".repeat(64);
    assert!(bind(data(), m).is_err());
    for location in [
        SourceProgramLocation { start: 0, end: 90 },
        SourceProgramLocation { start: 5, end: 6 },
        SourceProgramLocation { start: 5, end: 5 },
        SourceProgramLocation {
            start: 100,
            end: 102,
        },
    ] {
        let mut m = metadata();
        m.sites[0].expression = location;
        assert!(bind(data(), m).is_err());
    }
    let mut d = data();
    d.programs[0].body = vec![statement(SourceProgramStatementKind::Return {
        values: values(e(
            SourceProgramExprKind::Literal {
                value: ParserFactoryLiteral::Nil,
            },
            5,
            7,
        )),
    })];
    assert!(bind(d, metadata()).is_err());
    let mut d = data();
    d.programs[0].body = vec![statement(SourceProgramStatementKind::Return {
        values: SourceProgramValueList {
            values: vec![table(), table()],
            tail: None,
        },
    })];
    assert!(bind(d, metadata()).is_err());
    let populated = e(
        SourceProgramExprKind::Table {
            fields: vec![SourceProgramField::List { value: nil() }],
        },
        5,
        7,
    );
    assert!(
        bind(
            program(vec![statement(SourceProgramStatementKind::Return {
                values: values(populated)
            })]),
            metadata()
        )
        .is_err()
    );
}
#[test]
fn instruction_and_duplicate_bindings_are_consistent_with_empty_tnew() {
    for instruction in [53, 52 | (1 << 16), 52 | (1 << 27)] {
        let mut m = metadata();
        m.sites[0].instruction = instruction;
        assert!(bind(data(), m).is_err());
    }
    let mut m = metadata();
    m.sites[0].instruction = 52 | (1 << 16);
    m.sites[0].allocation = SourceTableAllocation::New {
        array_slots: 1,
        hash_bits: 0,
    };
    assert_eq!(
        bind(data(), m).unwrap_err().kind,
        SourceProgramErrorKind::UnsupportedCapability
    );
    for pc in [0, 1_000_001] {
        let mut m = metadata();
        m.sites[0].bytecode_pc = pc;
        assert!(bind(data(), m).is_err());
    }
    let mut m = metadata();
    m.sites.push(m.sites[0].clone());
    assert!(bind(data(), m).is_err());
    let second = e(SourceProgramExprKind::Table { fields: vec![] }, 8, 10);
    let d = program(vec![statement(SourceProgramStatementKind::Return {
        values: SourceProgramValueList {
            values: vec![table(), second],
            tail: None,
        },
    })]);
    let mut m = metadata();
    let mut site = m.sites[0].clone();
    site.expression = SourceProgramLocation { start: 8, end: 10 };
    m.sites.push(site);
    assert!(bind(d.clone(), m.clone()).is_err());
    m.sites[1].bytecode_pc = 2;
    bind(d.clone(), m.clone()).unwrap();
    m.sites[1].bytecode_sha256 = "f".repeat(64);
    assert!(bind(d, m).is_err());
    let mut m = metadata();
    m.sites[0].instruction = 52 | (255 << 8);
    bind(data(), m).unwrap(); // RegisterA does not change allocation.
}
fn call(value: SourceProgramExpr) -> SourceProgramCall {
    SourceProgramCall {
        binding: 0,
        receiver: Some(Box::new(nil())),
        arguments: values(value),
    }
}
#[test]
fn constructor_matching_follows_nested_expression_call_and_pack_containers() {
    let expressions = vec![
        e(
            SourceProgramExprKind::Get {
                table: Box::new(table()),
                key: Box::new(nil()),
            },
            1,
            90,
        ),
        e(
            SourceProgramExprKind::Get {
                table: Box::new(nil()),
                key: Box::new(table()),
            },
            1,
            90,
        ),
        e(
            SourceProgramExprKind::Unary {
                operation: SourceProgramUnary::Not,
                value: Box::new(table()),
            },
            1,
            90,
        ),
        e(
            SourceProgramExprKind::Binary {
                operation: SourceProgramBinary::And,
                left: Box::new(nil()),
                right: Box::new(table()),
            },
            1,
            90,
        ),
        e(
            SourceProgramExprKind::Table {
                fields: vec![SourceProgramField::Named {
                    key: "nested".into(),
                    value: table(),
                }],
            },
            1,
            90,
        ),
        e(
            SourceProgramExprKind::Table {
                fields: vec![SourceProgramField::Keyed {
                    key: table(),
                    value: nil(),
                }],
            },
            1,
            90,
        ),
        e(
            SourceProgramExprKind::Table {
                fields: vec![SourceProgramField::List { value: table() }],
            },
            1,
            90,
        ),
        e(
            SourceProgramExprKind::Table {
                fields: vec![SourceProgramField::Tail {
                    values: SourceProgramPack::Call {
                        call: call(table()),
                    },
                }],
            },
            1,
            90,
        ),
        e(
            SourceProgramExprKind::Call {
                call: Box::new(call(table())),
            },
            1,
            90,
        ),
        e(
            SourceProgramExprKind::Call {
                call: Box::new(SourceProgramCall {
                    binding: 0,
                    receiver: Some(Box::new(table())),
                    arguments: SourceProgramValueList::default(),
                }),
            },
            1,
            90,
        ),
    ];
    for e in expressions {
        bind(
            program(vec![statement(SourceProgramStatementKind::Return {
                values: values(e),
            })]),
            metadata(),
        )
        .unwrap();
    }
    bind(
        program(vec![statement(SourceProgramStatementKind::Return {
            values: SourceProgramValueList {
                values: vec![],
                tail: Some(Box::new(SourceProgramPack::Call {
                    call: call(table()),
                })),
            },
        })]),
        metadata(),
    )
    .unwrap();
}
#[test]
fn constructor_matching_follows_control_flow_and_write_operands() {
    use SourceProgramStatementKind as S;
    let statements = vec![
        S::Declare {
            locals: vec![0],
            values: values(table()),
        },
        S::If {
            branches: vec![SourceProgramBranch {
                condition: table(),
                body: vec![],
            }],
            otherwise: vec![],
        },
        S::If {
            branches: vec![SourceProgramBranch {
                condition: nil(),
                body: vec![statement(S::Return {
                    values: values(table()),
                })],
            }],
            otherwise: vec![],
        },
        S::If {
            branches: vec![SourceProgramBranch {
                condition: nil(),
                body: vec![],
            }],
            otherwise: vec![statement(S::Return {
                values: values(table()),
            })],
        },
        S::ForNumeric {
            local: 0,
            start: table(),
            limit: nil(),
            step: nil(),
            body: vec![],
        },
        S::ForEach {
            locals: vec![0],
            iterator: SourceProgramIterator::Generic {
                values: values(table()),
            },
            body: vec![],
        },
        S::ForEach {
            locals: vec![0],
            iterator: SourceProgramIterator::Dense {
                table: table(),
                binding: 2,
            },
            body: vec![],
        },
        S::ForEach {
            locals: vec![0],
            iterator: SourceProgramIterator::Pattern {
                call: SourceProgramCall {
                    binding: 1,
                    receiver: Some(Box::new(table())),
                    arguments: values(nil()),
                },
            },
            body: vec![],
        },
        S::TableSet {
            table: table(),
            key: nil(),
            value: nil(),
        },
        S::TableSet {
            table: nil(),
            key: table(),
            value: nil(),
        },
        S::TableSet {
            table: nil(),
            key: nil(),
            value: table(),
        },
        S::Call {
            call: call(table()),
        },
    ];
    for operation in statements {
        bind(program(vec![statement(operation)]), metadata()).unwrap();
    }
}
#[test]
fn parser_facade_rejects_sidecar_and_preserves_packaged_bytes() {
    let snapshot = bundled_snapshot().unwrap();
    let owner = snapshot.modifier_parser();
    let bytes = serde_json::to_vec(&owner.data().programs).unwrap();
    let data = owner.data().programs.data.clone();
    let mut m = metadata();
    m.sites.clear();
    assert_eq!(
        SourceProgramCatalog::new_with_constructors(
            data,
            SourceProgramOwner::from_parser(owner.clone()),
            m
        )
        .unwrap_err()
        .kind,
        SourceProgramErrorKind::UnsupportedCapability
    );
    assert_eq!(serde_json::to_vec(&owner.data().programs).unwrap(), bytes);
}
#[test]
fn descriptor_json_and_resource_bounds_are_checked_without_schema_drift() {
    let m = metadata();
    let bytes = serde_json::to_vec(&m).unwrap();
    assert_eq!(SourceProgramConstructors::from_bytes(&bytes).unwrap(), m);
    let mut wire = serde_json::to_value(&m).unwrap();
    wire["unknown"] = true.into();
    assert!(SourceProgramConstructors::from_bytes(&serde_json::to_vec(&wire).unwrap()).is_err());
    let duplicate = String::from_utf8(bytes.clone()).unwrap().replacen(
        "\"schema_version\":1",
        "\"schema_version\":1,\"schema_version\":1",
        1,
    );
    assert!(SourceProgramConstructors::from_bytes(duplicate.as_bytes()).is_err());
    let mut m = m.clone();
    m.sites[0].provenance.source.path = "x".repeat(4097);
    assert_eq!(
        bind(data(), m).unwrap_err().kind,
        SourceProgramErrorKind::ResourceLimit
    );
    assert_eq!(
        SourceProgramConstructors::from_bytes(&vec![b' '; 16 * 1024 * 1024 + 1])
            .unwrap_err()
            .kind,
        SourceProgramErrorKind::ResourceLimit
    );
    let mut m = metadata();
    m.sites[0].bytecode_sha256 = "not-a-hash".into();
    assert!(bind(data(), m).is_err());
    let mut m = metadata();
    m.schema_version = 2;
    assert!(bind(data(), m).is_err());
}

#[test]
fn constructor_count_and_aggregate_text_preflight_precede_binding_indices() {
    let mut m = metadata();
    let mut cheap = m.sites[0].clone();
    cheap.provenance.source.path.clear();
    cheap.provenance.source.sha256.clear();
    cheap.provenance.function_sha256.clear();
    cheap.bytecode_sha256.clear();
    m.sites = vec![cheap; 100_001];
    assert_eq!(
        bind(data(), m).unwrap_err().kind,
        SourceProgramErrorKind::ResourceLimit
    );
    let mut m = metadata();
    let mut long = m.sites[0].clone();
    long.provenance.source.path = "p".repeat(2000);
    m.sites = vec![long; 4500];
    assert_eq!(
        bind(data(), m).unwrap_err().kind,
        SourceProgramErrorKind::ResourceLimit
    );
}
