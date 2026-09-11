use poe_optimizer_data::{item_loading::ItemLoadingSource, source_program::*};
use std::collections::{BTreeMap, BTreeSet};

fn definitions() -> SourceProgramDefinitions {
    SourceProgramDefinitions {
        schema_version: SOURCE_PROGRAM_DEFINITIONS_SCHEMA_VERSION,
        source: ItemLoadingSource {
            upstream_revision: "a".repeat(40),
            files: BTreeMap::from([("fixture.lua".into(), "b".repeat(64))]),
            construction_spans: BTreeMap::new(),
            module_order: vec!["fixture.lua".into()],
        },
        tables: vec![SourceTable {
            fields: BTreeMap::from([
                ("self".into(), SourceValue::Table(SourceTableId(1))),
                ("disabled".into(), SourceValue::Boolean(false)),
                ("1".into(), SourceValue::Text("text key".into())),
            ]),
            indexed: BTreeMap::from([(1, SourceValue::Text("numeric key".into()))]),
        }],
        callbacks: vec![],
        roots: vec![
            SourceProgramRoot {
                name: "environment".into(),
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
fn coverage() -> SourceTableCoverage {
    SourceTableCoverage {
        inventory: SourceTableInventory::Selective,
        known_absent: BTreeSet::from([SourceTableKey::Text("absent".into())]),
        unavailable: BTreeSet::from([SourceTableKey::Text("unavailable".into())]),
        index_fallback: SourceTableIndexFallback::Nil,
        call_fallback: SourceTableCallFallback::NonCallable,
    }
}
fn context() -> SourceProgramContext {
    SourceProgramContext {
        schema_version: SOURCE_PROGRAM_CONTEXT_SCHEMA_VERSION,
        iteration: None,
        environment: Some(SourceProgramRootId(1)),
        tables: BTreeMap::from([(SourceTableId(1), coverage())]),
    }
}
#[test]
fn context_retains_presence_inventory_and_fallback_without_changing_definition_wire() {
    let data = definitions();
    let wire = serde_json::to_vec(&data).unwrap();
    let mut context = context();
    for inventory in [
        SourceTableInventory::Complete,
        SourceTableInventory::Selective,
    ] {
        for fallback in [
            SourceTableIndexFallback::Nil,
            SourceTableIndexFallback::Unavailable,
        ] {
            context.tables.get_mut(&SourceTableId(1)).unwrap().inventory = inventory;
            context
                .tables
                .get_mut(&SourceTableId(1))
                .unwrap()
                .index_fallback = fallback;
            for call_fallback in [
                SourceTableCallFallback::NonCallable,
                SourceTableCallFallback::Unavailable,
            ] {
                context
                    .tables
                    .get_mut(&SourceTableId(1))
                    .unwrap()
                    .call_fallback = call_fallback;
                let bytes = serde_json::to_vec(&context).unwrap();
                let restored = SourceProgramContext::from_bytes(&bytes, &data, None).unwrap();
                assert_eq!(restored, context);
                let owner =
                    SourceProgramOwner::new_with_context(data.clone(), None, restored).unwrap();
                assert_eq!(
                    owner.table_coverage(SourceTableId(1)),
                    context.tables.get(&SourceTableId(1))
                );
                assert_eq!(
                    owner.table(SourceTableId(1)).unwrap().fields["disabled"],
                    SourceValue::Boolean(false)
                );
                assert_eq!(
                    serde_json::to_vec(owner.definitions().unwrap()).unwrap(),
                    wire
                );
                assert!(owner.classes().is_none());
            }
        }
    }
    let plain = SourceProgramOwner::new(data).unwrap();
    assert!(plain.context().is_none());
    assert!(plain.environment_root().is_none());
    assert!(plain.bind_environment().unwrap().is_none());
    assert!(plain.table_coverage(SourceTableId(1)).is_none());
}
#[test]
fn context_and_environment_are_immutable_owner_bound_observations() {
    let data = definitions();
    let mut authored = context();
    let owner = SourceProgramOwner::new_with_context(data.clone(), None, authored.clone()).unwrap();
    let retained = owner.clone();
    let handle = owner.bind_environment().unwrap().unwrap();
    assert_eq!(
        handle.root(),
        SourceProgramDefinitionRoot::Named(SourceProgramRootId(1))
    );
    assert_eq!(handle.table_id(), SourceTableId(1));
    assert!(std::ptr::eq(
        owner.context().unwrap(),
        retained.context().unwrap()
    ));
    assert!(std::ptr::eq(
        owner.resolve_root(&handle).unwrap(),
        retained.resolve_root(&handle).unwrap()
    ));
    let alias = owner
        .bind_root(SourceProgramDefinitionRoot::Named(SourceProgramRootId(2)))
        .unwrap();
    assert!(std::ptr::eq(handle.table(), alias.table()));
    authored.tables.clear();
    authored.environment = None;
    assert_eq!(owner.context(), Some(&context()));
    for other in [
        SourceProgramOwner::new(data.clone()).unwrap(),
        SourceProgramOwner::new_with_context(data.clone(), None, context()).unwrap(),
        SourceProgramOwner::new_with_context(data, None, authored).unwrap(),
    ] {
        assert!(!other.is_same_owner(&owner));
        assert_eq!(
            other.resolve_root(&handle).unwrap_err().kind,
            SourceProgramErrorKind::Binding
        );
    }
}
#[test]
fn rejects_missing_roots_unknown_tables_and_conflicting_presence_claims() {
    let data = definitions();
    for id in [0, 3, u32::MAX] {
        let mut context = context();
        context.environment = Some(SourceProgramRootId(id));
        assert_eq!(
            context.validate(&data, None).unwrap_err().kind,
            SourceProgramErrorKind::Binding
        );
    }
    for id in [0, 2, u32::MAX] {
        let mut context = context();
        context.tables.insert(SourceTableId(id), coverage());
        assert_eq!(
            context.validate(&data, None).unwrap_err().kind,
            SourceProgramErrorKind::InvalidData
        );
    }
    for key in [
        SourceTableKey::Text("disabled".into()),
        SourceTableKey::Integer(1),
    ] {
        for unavailable in [false, true] {
            let mut coverage = coverage();
            if unavailable {
                coverage.unavailable.insert(key.clone());
            } else {
                coverage.known_absent.insert(key.clone());
            }
            coverage.validate_shape().unwrap();
            assert!(
                coverage
                    .validate_table(&data.tables[0])
                    .unwrap_err()
                    .message
                    .contains("represented")
            );
        }
    }
    let mut overlap = coverage();
    overlap.unavailable.extend(overlap.known_absent.clone());
    assert!(
        overlap
            .validate_shape()
            .unwrap_err()
            .message
            .contains("both absent and unavailable")
    );
}
#[test]
fn canonical_keys_distinguish_text_and_integer_and_reject_invalid_lua_keys() {
    let mut value = coverage();
    value.known_absent = BTreeSet::from([
        SourceTableKey::Text("0".into()),
        SourceTableKey::Integer(0),
        SourceTableKey::Text(String::new()),
        SourceTableKey::Integer(-9_007_199_254_740_991),
        SourceTableKey::Integer(9_007_199_254_740_991),
    ]);
    value.validate_shape().unwrap();
    let restored: SourceTableCoverage =
        serde_json::from_slice(&serde_json::to_vec(&value).unwrap()).unwrap();
    assert_eq!(restored, value);
    for key in [
        SourceTableKey::Text("x\0y".into()),
        SourceTableKey::Text("x".repeat(4097)),
        SourceTableKey::Integer(9_007_199_254_740_992),
        SourceTableKey::Integer(i64::MIN),
    ] {
        let mut bad = coverage();
        bad.known_absent.insert(key);
        assert_eq!(
            bad.validate_shape().unwrap_err().kind,
            SourceProgramErrorKind::InvalidData
        );
    }
}
#[test]
fn context_json_rejects_duplicate_normalized_keys_ids_and_omitted_contracts() {
    let data = definitions();
    let json = serde_json::to_string(&context()).unwrap();
    let raw_coverage = serde_json::to_string(&coverage()).unwrap();
    for duplicate in [
        format!(
            r#"{{"schema_version":1,"environment":1,"tables":{{"1":{raw_coverage},"1":{raw_coverage}}}}}"#
        ),
        format!(
            r#"{{"schema_version":1,"environment":1,"tables":{{"1":{raw_coverage},"01":{raw_coverage}}}}}"#
        ),
        json.replace(
            r#""known_absent":[{"kind":"text","value":"absent"}]"#,
            r#""known_absent":[{"kind":"integer","value":0},{"kind":"integer","value":-0}]"#,
        ),
        json.replace(
            r#""unavailable":[{"kind":"text","value":"unavailable"}]"#,
            r#""unavailable":[{"kind":"text","value":"same"},{"kind":"text","value":"same"}]"#,
        ),
    ] {
        assert!(
            SourceProgramContext::from_bytes(duplicate.as_bytes(), &data, None).is_err(),
            "{duplicate}"
        );
    }
    for modified in [
        json.replacen('{', r#"{"unknown":true,"#, 1),
        json.replace(r#""schema_version":1"#, r#""schema_version":2"#),
        json.replace(r#""environment":1,"#, ""),
        json.replace(r#","call_fallback":"non_callable""#, ""),
        json.replace(
            r#""call_fallback":"non_callable""#,
            r#""call_fallback":"nil""#,
        ),
        json.replace(
            r#""index_fallback":"nil""#,
            r#""index_fallback":"nil","extra":false"#,
        ),
        json.replace(
            r#"{"kind":"text","value":"absent"}"#,
            r#"{"kind":"text","value":"absent","extra":0}"#,
        ),
        json.replace(
            r#"{"kind":"text","value":"absent"}"#,
            r#"{"kind":"integer","value":1.5}"#,
        ),
    ] {
        assert!(
            SourceProgramContext::from_bytes(modified.as_bytes(), &data, None).is_err(),
            "{modified}"
        );
    }
}
#[test]
fn coverage_counts_and_text_are_bounded_before_owner_storage() {
    let mut value = coverage();
    value.known_absent = (0..50_000).map(SourceTableKey::Integer).collect();
    value.unavailable.clear();
    value.validate_shape().unwrap();
    assert_eq!(
        value
            .validate_table(&definitions().tables[0])
            .unwrap_err()
            .kind,
        SourceProgramErrorKind::ResourceLimit
    );
    value.unavailable.insert(SourceTableKey::Integer(-1));
    assert_eq!(
        value.validate_shape().unwrap_err().kind,
        SourceProgramErrorKind::ResourceLimit
    );
    // Per-table keys are each legal, but their aggregate text is not.
    value.known_absent = (0..4097)
        .map(|index| SourceTableKey::Text(format!("{index:04}{}", "x".repeat(4092))))
        .collect();
    value.unavailable.clear();
    assert_eq!(
        value.validate_shape().unwrap_err().kind,
        SourceProgramErrorKind::ResourceLimit
    );
    assert_eq!(
        SourceProgramContext::from_bytes(&vec![b' '; 16 * 1024 * 1024 + 1], &definitions(), None)
            .unwrap_err()
            .kind,
        SourceProgramErrorKind::ResourceLimit
    );
}
#[test]
fn context_enforces_aggregate_bounds_across_individually_valid_tables() {
    let mut data = definitions();
    data.tables = vec![SourceTable::default(); 21];
    let mut context = SourceProgramContext::default();
    let value = SourceTableCoverage {
        inventory: SourceTableInventory::Selective,
        known_absent: (0..50_000).map(SourceTableKey::Integer).collect(),
        unavailable: BTreeSet::new(),
        index_fallback: SourceTableIndexFallback::Nil,
        call_fallback: SourceTableCallFallback::NonCallable,
    };
    value.validate_shape().unwrap();
    for id in 1..=21 {
        context.tables.insert(SourceTableId(id), value.clone());
    }
    assert_eq!(
        context.validate(&data, None).unwrap_err().kind,
        SourceProgramErrorKind::ResourceLimit
    );
}
