//! Structural authored claims only; actual template identity, self-marker rows
//! and source lifetime are authenticated by the PoB adapter, not these fixtures.
use super::*;

fn local() -> SourceProgramExpr {
    e(SourceProgramExprKind::Local { local: 0 }, 10, 11)
}
fn named(key: &str) -> SourceProgramField {
    SourceProgramField::Named {
        key: key.into(),
        value: local(),
    }
}
fn keyed(key: SourceProgramExprKind) -> SourceProgramField {
    SourceProgramField::Keyed {
        key: e(key, 12, 13),
        value: local(),
    }
}
fn bytes(key: &[u8]) -> SourceProgramField {
    keyed(SourceProgramExprKind::Bytes {
        value: key.to_vec(),
    })
}
fn fixture(
    fields: Vec<SourceProgramField>,
    keys: Vec<Vec<u8>>,
) -> (SourceProgramData, SourceProgramConstructors) {
    let (d, mut m) = list_fixture(fields);
    m.sites[0].instruction = 53 | (34 << 8) | (92 << 16);
    m.sites[0].allocation = SourceTableAllocation::DuplicateReservedStrings { keys };
    (d, m)
}
fn standard() -> (SourceProgramData, SourceProgramConstructors) {
    fixture(
        vec![named("first"), named("second")],
        vec![b"second".to_vec(), b"first".to_vec()],
    )
}
fn keys_mut(meta: &mut SourceProgramConstructors) -> &mut Vec<Vec<u8>> {
    let SourceTableAllocation::DuplicateReservedStrings { keys } = &mut meta.sites[0].allocation
    else {
        panic!("reserved fixture");
    };
    keys
}

#[test]
fn reserved_tdup_is_distinct_from_tnew_and_preserves_prior_wire_and_catalog_identity() {
    assert_eq!(
        serde_json::to_string(&SourceTableAllocation::New {
            array_slots: 0,
            hash_bits: 0
        })
        .unwrap(),
        r#"{"kind":"new","array_slots":0,"hash_bits":0}"#
    );
    let (d, m) = standard();
    let original = serde_json::to_vec(&d).unwrap();
    let owner = owner();
    let ordinary = SourceProgramCatalog::new(d.clone(), owner.clone()).unwrap();
    let wire = serde_json::to_vec(&m).unwrap();
    assert_eq!(SourceProgramConstructors::from_bytes(&wire).unwrap(), m);
    let catalog = SourceProgramCatalog::new_with_constructors(d, owner.clone(), m.clone()).unwrap();
    assert!(catalog.is_bound_to(&owner));
    assert_eq!(catalog.constructors(), Some(&m));
    assert_eq!(serde_json::to_vec(catalog.data()).unwrap(), original);
    assert!(!std::ptr::eq(catalog.data(), ordinary.data()));
    assert!(ordinary.constructors().is_none());
    assert!(
        SourceProgramCatalog::from_bytes(&original, owner)
            .unwrap()
            .constructors()
            .is_none()
    );
    assert_eq!(
        serde_json::to_value(&m.sites[0].allocation).unwrap(),
        serde_json::json!({
            "kind": "duplicate_reserved_strings", "keys": [b"second".to_vec(), b"first".to_vec()]
        })
    );
}

#[test]
fn reserved_tdup_accepts_exact_static_text_and_binary_keys_in_observed_order_with_final_pack() {
    let fields = vec![
        named("named"),
        keyed(SourceProgramExprKind::Literal {
            value: ParserFactoryLiteral::Text("quoted key \"λ".into()),
        }),
        bytes(&[0, 255, 128]),
        bytes(b""),
    ];
    let keys = vec![
        vec![],
        vec![0, 255, 128],
        b"named".to_vec(),
        "quoted key \"λ".as_bytes().to_vec(),
    ];
    for tail in [
        None,
        Some(SourceProgramPack::Varargs),
        Some(SourceProgramPack::Call {
            call: call(local()),
        }),
    ] {
        let mut fields = fields.clone();
        if let Some(values) = tail {
            fields.push(SourceProgramField::Tail { values });
        }
        let (d, m) = fixture(fields, keys.clone());
        let catalog = bind(d, m).unwrap();
        assert_eq!(
            catalog.constructors().unwrap().sites[0].allocation,
            SourceTableAllocation::DuplicateReservedStrings { keys: keys.clone() }
        );
    }
}

#[test]
fn reserved_tdup_rejects_empty_missing_extra_and_duplicate_keys_including_literal_aliases() {
    let (d, m) = standard();
    for keys in [
        vec![],
        vec![b"first".to_vec()],
        vec![b"first".to_vec(), b"second".to_vec(), b"extra".to_vec()],
        vec![b"first".to_vec(), b"first".to_vec()],
    ] {
        let mut bad = m.clone();
        *keys_mut(&mut bad) = keys;
        assert_eq!(
            bind(d.clone(), bad).unwrap_err().kind,
            SourceProgramErrorKind::Binding
        );
    }
    for duplicate in [
        named("same"),
        bytes(b"same"),
        keyed(SourceProgramExprKind::Literal {
            value: ParserFactoryLiteral::Text("same".into()),
        }),
    ] {
        let (d, m) = fixture(vec![named("same"), duplicate], vec![b"same".to_vec()]);
        assert_eq!(
            bind(d, m).unwrap_err().kind,
            SourceProgramErrorKind::Binding
        );
    }
}

#[test]
fn reserved_tdup_rejects_dynamic_numeric_computed_keys_lists_and_nonfinal_packs() {
    let invalid_fields = [
        keyed(SourceProgramExprKind::Local { local: 0 }),
        keyed(SourceProgramExprKind::Literal {
            value: ParserFactoryLiteral::Number(1.0),
        }),
        keyed(SourceProgramExprKind::Binary {
            operation: SourceProgramBinary::Concat,
            left: Box::new(e(
                SourceProgramExprKind::Bytes {
                    value: b"first".to_vec(),
                },
                12,
                13,
            )),
            right: Box::new(e(SourceProgramExprKind::Bytes { value: vec![] }, 14, 15)),
        }),
        SourceProgramField::List { value: nil() },
    ];
    for field in invalid_fields {
        let (d, m) = fixture(vec![field], vec![b"first".to_vec()]);
        assert_eq!(
            bind(d, m).unwrap_err().kind,
            SourceProgramErrorKind::Binding
        );
    }
    for fields in [
        vec![
            SourceProgramField::Tail {
                values: SourceProgramPack::Varargs,
            },
            named("first"),
        ],
        vec![
            named("first"),
            SourceProgramField::Tail {
                values: SourceProgramPack::Varargs,
            },
            SourceProgramField::Tail {
                values: SourceProgramPack::Varargs,
            },
        ],
    ] {
        let (d, m) = fixture(fields, vec![b"first".to_vec()]);
        assert_eq!(
            bind(d, m).unwrap_err().kind,
            SourceProgramErrorKind::InvalidData
        );
    }
}

#[test]
fn reserved_tdup_requires_exact_opcode_callback_provenance_and_unique_table_range() {
    let (d, m) = standard();
    let mut invalid = vec![];
    for opcode in [0, 52, 54, 255] {
        let mut bad = m.clone();
        bad.sites[0].instruction = (bad.sites[0].instruction & !255) | opcode;
        invalid.push(bad);
    }
    for callback in [0, 2] {
        let mut bad = m.clone();
        bad.sites[0].callback = SourceCallbackId(callback);
        invalid.push(bad);
    }
    let mut bad = m.clone();
    bad.sites[0].provenance.function_sha256 = "f".repeat(64);
    invalid.push(bad);
    let mut bad = m.clone();
    bad.sites[0].expression.end -= 1;
    invalid.push(bad);
    let mut bad = m.clone();
    bad.sites.push(bad.sites[0].clone());
    invalid.push(bad);
    for bad in invalid {
        assert_eq!(
            bind(d.clone(), bad).unwrap_err().kind,
            SourceProgramErrorKind::Binding
        );
    }
    let mut ambiguous = d;
    let SourceProgramStatementKind::Return { values } =
        &mut ambiguous.programs[0].body[0].operation
    else {
        unreachable!()
    };
    values.values.push(values.values[0].clone());
    assert_eq!(
        bind(ambiguous, m).unwrap_err().kind,
        SourceProgramErrorKind::Binding
    );
}

#[test]
fn reserved_tdup_reused_constant_has_one_order_but_independent_constants_and_tnew_sites_do_not() {
    let (mut d, mut m) = standard();
    let SourceProgramStatementKind::Return { values } = &mut d.programs[0].body[0].operation else {
        unreachable!()
    };
    let mut second = values.values[0].clone();
    second.location = SourceProgramLocation { start: 40, end: 80 };
    values.values.push(second.clone());
    let mut site = m.sites[0].clone();
    site.expression = second.location;
    site.bytecode_pc += 1;
    site.instruction = (site.instruction & !(255 << 8)) | (100 << 8);
    m.sites.push(site);
    bind(d.clone(), m.clone()).unwrap();
    let SourceTableAllocation::DuplicateReservedStrings { keys } = &mut m.sites[1].allocation
    else {
        unreachable!()
    };
    keys.reverse();
    assert_eq!(
        bind(d.clone(), m.clone()).unwrap_err().kind,
        SourceProgramErrorKind::Binding
    );
    m.sites[1].instruction += 1 << 16;
    bind(d.clone(), m.clone()).unwrap();
    // Distinct allocation families still share the existing unique-site namespace.
    let SourceProgramStatementKind::Return { values } = &mut d.programs[0].body[0].operation else {
        unreachable!()
    };
    values.values.push(table());
    values.values[2].location = SourceProgramLocation { start: 90, end: 92 };
    let mut tnew = metadata().sites.remove(0);
    tnew.provenance = m.sites[0].provenance.clone();
    tnew.expression = values.values[2].location;
    tnew.bytecode_pc = 3;
    m.sites.push(tnew);
    bind(d.clone(), m.clone()).unwrap();
    m.sites[2].bytecode_pc = m.sites[0].bytecode_pc;
    assert_eq!(
        bind(d, m).unwrap_err().kind,
        SourceProgramErrorKind::Binding
    );
}

#[test]
fn reserved_tdup_profile_claims_do_not_grant_capability_and_parser_owners_still_reject() {
    let (d, mut m) = standard();
    m.profile.table_bump = true;
    let catalog = bind(d, m.clone()).unwrap();
    assert!(
        !catalog
            .constructors()
            .unwrap()
            .profile
            .is_supported_array_profile()
    );
    let snapshot = bundled_snapshot().unwrap();
    let parser = snapshot.modifier_parser();
    let before = serde_json::to_vec(&parser.data().programs).unwrap();
    let error = SourceProgramCatalog::new_with_constructors(
        parser.data().programs.data.clone(),
        SourceProgramOwner::from_parser(parser.clone()),
        m,
    )
    .unwrap_err();
    assert_eq!(error.kind, SourceProgramErrorKind::UnsupportedCapability);
    assert_eq!(serde_json::to_vec(&parser.data().programs).unwrap(), before);
}

#[test]
fn reserved_tdup_json_rejects_missing_unknown_duplicate_and_oversized_members() {
    let (_, m) = standard();
    let wire = serde_json::to_value(&m).unwrap();
    for key in ["keys", "kind"] {
        let mut bad = wire.clone();
        bad["sites"][0]["allocation"]
            .as_object_mut()
            .unwrap()
            .remove(key);
        assert!(SourceProgramConstructors::from_bytes(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
    for key in ["capacity", "raw_length", "values", "template_constant"] {
        let mut bad = wire.clone();
        bad["sites"][0]["allocation"][key] = 1.into();
        assert!(SourceProgramConstructors::from_bytes(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
    let duplicate = serde_json::to_string(&m)
        .unwrap()
        .replace("\"keys\":", "\"keys\":[],\"keys\":");
    assert!(SourceProgramConstructors::from_bytes(duplicate.as_bytes()).is_err());
    for keys in [
        vec![vec![0; SOURCE_PROGRAM_CONSTRUCTORS_MAX_KEY_BYTES + 1]],
        vec![vec![]; SOURCE_PROGRAM_CONSTRUCTORS_MAX_KEYS_PER_TEMPLATE + 1],
    ] {
        let mut bad = m.clone();
        *keys_mut(&mut bad) = keys;
        assert!(SourceProgramConstructors::from_bytes(&serde_json::to_vec(&bad).unwrap()).is_err());
        assert_eq!(
            bind(standard().0, bad).unwrap_err().kind,
            SourceProgramErrorKind::ResourceLimit
        );
    }
}

#[test]
fn reserved_tdup_preflights_aggregate_key_records_and_bytes_before_duplicate_sets() {
    let (d, mut m) = standard();
    *keys_mut(&mut m) = vec![vec![b'k']; SOURCE_PROGRAM_CONSTRUCTORS_MAX_KEYS_PER_TEMPLATE];
    let repetitions = SOURCE_PROGRAM_CONSTRUCTORS_MAX_RESERVED_KEYS
        / SOURCE_PROGRAM_CONSTRUCTORS_MAX_KEYS_PER_TEMPLATE
        + 1;
    m.sites = vec![m.sites[0].clone(); repetitions];
    // Even though each template has duplicate keys, the entire facet is bounded
    // before allocating normalized sets or reporting those structural errors.
    assert_eq!(
        bind(d.clone(), m.clone()).unwrap_err().kind,
        SourceProgramErrorKind::ResourceLimit
    );
    assert!(SourceProgramConstructors::from_bytes(&serde_json::to_vec(&m).unwrap()).is_err());
    m.sites.truncate(1);
    *keys_mut(&mut m) = vec![
        vec![b'k'; SOURCE_PROGRAM_CONSTRUCTORS_MAX_KEY_BYTES];
        SOURCE_PROGRAM_CONSTRUCTORS_MAX_TEXT_BYTES
            / SOURCE_PROGRAM_CONSTRUCTORS_MAX_KEY_BYTES
    ];
    assert_eq!(
        bind(d, m.clone()).unwrap_err().kind,
        SourceProgramErrorKind::ResourceLimit
    );
    assert!(SourceProgramConstructors::from_bytes(&serde_json::to_vec(&m).unwrap()).is_err());
}

#[test]
fn reserved_tdup_accepts_key_and_field_bounds_without_losing_binary_identity() {
    let key = vec![255; SOURCE_PROGRAM_CONSTRUCTORS_MAX_KEY_BYTES];
    let (d, m) = fixture(vec![bytes(&key)], vec![key]);
    bind(d, m).unwrap();
    let keys: Vec<_> = (0..SOURCE_PROGRAM_CONSTRUCTORS_MAX_KEYS_PER_TEMPLATE)
        .map(|i| (i as u32).to_le_bytes().to_vec())
        .collect();
    let fields = keys.iter().map(|key| bytes(key)).collect();
    let (d, m) = fixture(fields, keys);
    bind(d, m).unwrap();
}
