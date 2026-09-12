use poe_optimizer_data::{
    game_data::bundled_snapshot, item_loading::ItemLoadingSource, source_program::*,
};
use std::collections::{BTreeMap, BTreeSet};
fn text(value: &str) -> SourceTableKey {
    SourceTableKey::Text(value.into())
}
fn definitions() -> SourceProgramDefinitions {
    SourceProgramDefinitions {
        schema_version: SOURCE_PROGRAM_DEFINITIONS_SCHEMA_VERSION,
        source: ItemLoadingSource {
            upstream_revision: "a".repeat(40),
            files: BTreeMap::from([("fixture.lua".into(), "b".repeat(64))]),
            construction_spans: BTreeMap::new(),
            module_order: vec!["fixture.lua".into()],
        },
        tables: vec![
            SourceTable {
                fields: BTreeMap::from([
                    ("z".into(), SourceValue::Boolean(false)),
                    ("1".into(), SourceValue::Table(SourceTableId(2))),
                ]),
                indexed: BTreeMap::from([
                    (1, SourceValue::Number(1.0)),
                    (4, SourceValue::Number(4.0)),
                ]),
            },
            SourceTable::default(),
            SourceTable::default(),
        ],
        callbacks: ["pairs", "next", "pairs", "next"]
            .into_iter()
            .map(|symbol| SourceCallback {
                kind: SourceCallbackKind::Builtin {
                    symbol: symbol.into(),
                },
                upvalues: vec![],
                environment: SourceEnvironment::OriginalGlobals,
            })
            .collect(),
        roots: vec![SourceProgramRoot {
            name: "data".into(),
            table: SourceTableId(1),
        }],
        intrinsics: BTreeMap::from([
            (SourceCallbackId(1), SourceProgramIntrinsic::Pairs),
            (SourceCallbackId(2), SourceProgramIntrinsic::Next),
            (SourceCallbackId(3), SourceProgramIntrinsic::Pairs),
            (SourceCallbackId(4), SourceProgramIntrinsic::Next),
        ]),
    }
}
fn context() -> SourceProgramContext {
    SourceProgramContext {
        iteration: Some(SourceProgramIteration {
            table_order: BTreeMap::from([
                (
                    SourceTableId(1),
                    vec![
                        text("z"),
                        SourceTableKey::Integer(4),
                        text("1"),
                        SourceTableKey::Integer(1),
                    ],
                ),
                (SourceTableId(2), vec![]),
            ]),
            // Equal builtin operation labels never imply equal callback identity.
            pairs_next: BTreeMap::from([
                (SourceCallbackId(1), SourceCallbackId(4)),
                (SourceCallbackId(3), SourceCallbackId(2)),
            ]),
            ipairs_aux: BTreeMap::new(),
        }),
        ..SourceProgramContext::default()
    }
}
fn owner(context: SourceProgramContext) -> SourceProgramResult<SourceProgramOwner> {
    SourceProgramOwner::new_with_context(definitions(), None, context)
}
#[test]
fn exact_order_and_hidden_builtin_links_remain_owner_bound_and_independent_of_map_order() {
    let context = context();
    let owner = owner(context.clone()).unwrap();
    assert_eq!(
        owner.table_iteration_order(SourceTableId(1)),
        Some(context.iteration.as_ref().unwrap().table_order[&SourceTableId(1)].as_slice())
    );
    assert_eq!(owner.table_iteration_order(SourceTableId(2)), Some(&[][..]));
    assert_eq!(
        owner.table_iteration_order(SourceTableId(3)),
        None,
        "missing proof is not empty proof"
    );
    assert_eq!(
        owner.pairs_next_callback(SourceCallbackId(1)),
        Some(SourceCallbackId(4))
    );
    assert_eq!(
        owner.pairs_next_callback(SourceCallbackId(3)),
        Some(SourceCallbackId(2))
    );
    assert_eq!(owner.pairs_next_callback(SourceCallbackId(2)), None);
    let handle = owner
        .bind_root(SourceProgramDefinitionRoot::Named(SourceProgramRootId(1)))
        .unwrap();
    let other = SourceProgramOwner::new_with_context(definitions(), None, context).unwrap();
    assert!(!owner.is_same_owner(&other));
    assert_eq!(
        other.resolve_root(&handle).unwrap_err().kind,
        SourceProgramErrorKind::Binding
    );
    assert_eq!(owner.callbacks()[0].upvalues, vec![]);
}
#[test]
fn traversal_requires_exact_unique_complete_plain_raw_inventory() {
    for edit in 0..5 {
        let mut c = context();
        let orders = &mut c.iteration.as_mut().unwrap().table_order;
        match edit {
            0 => {
                orders.get_mut(&SourceTableId(1)).unwrap().pop();
            }
            1 => {
                orders.get_mut(&SourceTableId(1)).unwrap()[0] = text("missing");
            }
            2 => {
                orders.get_mut(&SourceTableId(1)).unwrap()[0] = text("1");
            }
            3 => {
                orders.insert(SourceTableId(0), vec![]);
            }
            _ => {
                orders.insert(SourceTableId(100), vec![]);
            }
        }
        assert!(owner(c).is_err(), "edit {edit}");
    }
    let coverage = SourceTableCoverage {
        inventory: SourceTableInventory::Complete,
        known_absent: BTreeSet::from([text("absent")]),
        unavailable: BTreeSet::new(),
        index_fallback: SourceTableIndexFallback::Nil,
        call_fallback: SourceTableCallFallback::NonCallable,
    };
    let mut c = context();
    c.tables.insert(SourceTableId(1), coverage.clone());
    owner(c).unwrap();
    for edit in 0..4 {
        let mut coverage = coverage.clone();
        match edit {
            0 => coverage.inventory = SourceTableInventory::Selective,
            1 => {
                coverage.unavailable.insert(text("omitted"));
            }
            2 => coverage.index_fallback = SourceTableIndexFallback::Unavailable,
            _ => coverage.call_fallback = SourceTableCallFallback::Unavailable,
        }
        let mut c = context();
        c.tables.insert(SourceTableId(1), coverage);
        assert_eq!(
            owner(c).unwrap_err().kind,
            SourceProgramErrorKind::UnsupportedCapability
        );
    }
    let mut data = definitions();
    data.tables[0].fields.insert("z".into(), SourceValue::Nil);
    assert!(
        context()
            .validate(&data, None)
            .unwrap_err()
            .message
            .contains("nil value")
    );
}
#[test]
fn pairs_link_is_required_by_every_nonclass_owner_constructor_and_cannot_be_inferred() {
    let data = definitions();
    data.validate().unwrap();
    let prototypes = SourceClosurePrototypes {
        schema_version: SOURCE_CLOSURE_PROTOTYPES_SCHEMA_VERSION,
        prototypes: vec![],
    };
    for result in [
        SourceProgramOwner::new(data.clone()),
        SourceProgramOwner::new_with_context(data.clone(), None, SourceProgramContext::default()),
        SourceProgramOwner::new_with_closures(data.clone(), None, None, prototypes.clone()),
        SourceProgramOwner::new_with_closures(
            data.clone(),
            None,
            Some(SourceProgramContext::default()),
            prototypes.clone(),
        ),
    ] {
        let error = result.unwrap_err();
        assert_eq!(error.kind, SourceProgramErrorKind::Binding);
        assert!(error.message.contains("retained next"));
    }
    SourceProgramOwner::new_with_closures(data, None, Some(context()), prototypes).unwrap();
    for edit in 0..5 {
        let mut c = context();
        let links = &mut c.iteration.as_mut().unwrap().pairs_next;
        match edit {
            0 => {
                links.remove(&SourceCallbackId(1));
            }
            1 => {
                links.insert(SourceCallbackId(2), SourceCallbackId(4));
            }
            2 => {
                links.insert(SourceCallbackId(1), SourceCallbackId(3));
            }
            3 => {
                links.insert(SourceCallbackId(1), SourceCallbackId(99));
            }
            _ => {
                links.insert(SourceCallbackId(99), SourceCallbackId(2));
            }
        }
        assert_eq!(
            owner(c).unwrap_err().kind,
            SourceProgramErrorKind::Binding,
            "edit {edit}"
        );
    }
}
#[test]
fn iteration_json_preserves_absent_wire_and_rejects_duplicate_normalized_keys_and_ids() {
    let old = br#"{"schema_version":1,"environment":null,"tables":{}}"#;
    let c: SourceProgramContext = serde_json::from_slice(old).unwrap();
    assert!(c.iteration.is_none());
    assert_eq!(serde_json::to_vec(&c).unwrap(), old);
    let data = definitions();
    let before = serde_json::to_vec(&data).unwrap();
    let wire = serde_json::to_vec(&context()).unwrap();
    let decoded = SourceProgramContext::from_bytes(&wire, &data, None).unwrap();
    assert_eq!(decoded, context());
    assert_eq!(serde_json::to_vec(&data).unwrap(), before);
    for wire in [
        r#"{"table_order":{"1":[],"01":[]},"pairs_next":{}}"#,
        r#"{"table_order":{},"pairs_next":{"1":2,"01":4}}"#,
        r#"{"table_order":{"1":[{"kind":"integer","value":1},{"kind":"integer","value":1}]},"pairs_next":{}}"#,
        r#"{"table_order":{},"pairs_next":{},"extra":true}"#,
    ] {
        assert!(
            serde_json::from_str::<SourceProgramIteration>(wire).is_err(),
            "{wire}"
        );
    }
    let parser =
        SourceProgramOwner::from_parser(bundled_snapshot().unwrap().modifier_parser().clone());
    assert!(parser.iteration().is_none());
    assert_eq!(parser.table_iteration_order(SourceTableId(1)), None);
    assert_eq!(parser.pairs_next_callback(SourceCallbackId(1)), None);
}
#[test]
fn iteration_shape_and_decoder_bound_counts_text_and_exact_key_domain() {
    for key in [
        text(&"x".repeat(4097)),
        text("a\0b"),
        SourceTableKey::Integer(9_007_199_254_740_992),
        SourceTableKey::Integer(-9_007_199_254_740_992),
        SourceTableKey::Integer(i64::MIN),
    ] {
        let mut c = context();
        c.iteration
            .as_mut()
            .unwrap()
            .table_order
            .insert(SourceTableId(2), vec![key]);
        assert!(owner(c).is_err());
    }
    let mut metadata = SourceProgramIteration::default();
    metadata.table_order.insert(
        SourceTableId(1),
        vec![
            SourceTableKey::Integer(-9_007_199_254_740_991),
            SourceTableKey::Integer(9_007_199_254_740_991),
            SourceTableKey::Integer(0),
            text("0"),
        ],
    );
    metadata.validate_shape().unwrap();
    assert_eq!(
        serde_json::from_slice::<SourceProgramIteration>(&serde_json::to_vec(&metadata).unwrap())
            .unwrap(),
        metadata
    );
    metadata.table_order.insert(
        SourceTableId(1),
        (0..50_001).map(SourceTableKey::Integer).collect(),
    );
    assert_eq!(
        metadata.validate_shape().unwrap_err().kind,
        SourceProgramErrorKind::ResourceLimit
    );
    let wire = serde_json::to_vec(&metadata).unwrap();
    assert!(serde_json::from_slice::<SourceProgramIteration>(&wire).is_err());
    metadata.table_order.clear();
    metadata.pairs_next = (1..258)
        .map(|id| (SourceCallbackId(id), SourceCallbackId(2)))
        .collect();
    assert_eq!(
        metadata.validate_shape().unwrap_err().kind,
        SourceProgramErrorKind::ResourceLimit
    );
    assert!(
        serde_json::from_slice::<SourceProgramIteration>(&serde_json::to_vec(&metadata).unwrap())
            .is_err()
    );
    metadata.pairs_next.clear();
    for id in 1..=21 {
        metadata.table_order.insert(
            SourceTableId(id),
            (0..50_000).map(SourceTableKey::Integer).collect(),
        );
    }
    assert_eq!(
        metadata.validate_shape().unwrap_err().kind,
        SourceProgramErrorKind::ResourceLimit
    );
    assert!(
        serde_json::from_slice::<SourceProgramIteration>(&serde_json::to_vec(&metadata).unwrap())
            .is_err()
    );
}

fn ipairs_definitions() -> SourceProgramDefinitions {
    let mut data = definitions();
    for (index, callback) in data.callbacks.iter_mut().enumerate() {
        let operation = if index % 2 == 0 {
            SourceProgramIntrinsic::Ipairs
        } else {
            SourceProgramIntrinsic::IpairsAux
        };
        callback.kind = SourceCallbackKind::Builtin {
            symbol: operation.builtin_symbol().unwrap(),
        };
        data.intrinsics
            .insert(SourceCallbackId(index as u32 + 1), operation);
    }
    data
}
fn ipairs_context() -> SourceProgramContext {
    let mut c = context();
    let iteration = c.iteration.as_mut().unwrap();
    iteration.ipairs_aux = std::mem::take(&mut iteration.pairs_next);
    c
}
#[test]
fn retained_ipairs_auxiliary_is_nonglobal_and_exact_to_each_owner_factory() {
    let data = ipairs_definitions();
    let before = serde_json::to_vec(&data).unwrap();
    let c = ipairs_context();
    let owner = SourceProgramOwner::new_with_context(data.clone(), None, c.clone()).unwrap();
    assert_eq!(
        owner.ipairs_aux_callback(SourceCallbackId(1)),
        Some(SourceCallbackId(4))
    );
    assert_eq!(
        owner.ipairs_aux_callback(SourceCallbackId(3)),
        Some(SourceCallbackId(2))
    );
    assert_eq!(owner.ipairs_aux_callback(SourceCallbackId(2)), None);
    assert_eq!(owner.ipairs_aux_callback(SourceCallbackId(0)), None);
    assert_eq!(owner.pairs_next_callback(SourceCallbackId(1)), None);
    let second = SourceProgramOwner::new_with_context(data.clone(), None, c).unwrap();
    assert!(!owner.is_same_owner(&second));
    let root = owner
        .bind_root(SourceProgramDefinitionRoot::Named(SourceProgramRootId(1)))
        .unwrap();
    assert_eq!(
        second.resolve_root(&root).unwrap_err().kind,
        SourceProgramErrorKind::Binding
    );
    assert_eq!(
        serde_json::to_vec(owner.definitions().unwrap()).unwrap(),
        before
    );
    assert!(
        owner
            .callbacks()
            .iter()
            .all(|callback| callback.upvalues.is_empty())
    );
    assert_eq!(SourceProgramIntrinsic::IpairsAux.global_path(), None);
    assert_eq!(
        SourceProgramIntrinsic::IpairsAux
            .builtin_symbol()
            .as_deref(),
        Some("ipairs_aux")
    );
    assert_eq!(SourceProgramIntrinsic::CreateMod.builtin_symbol(), None);
    assert!(SourceProgramIntrinsic::IpairsAux.is_standalone_only());
    assert!(!SourceProgramIntrinsic::IpairsAux.is_string_method());
    let parser =
        SourceProgramOwner::from_parser(bundled_snapshot().unwrap().modifier_parser().clone());
    assert_eq!(parser.ipairs_aux_callback(SourceCallbackId(1)), None);
}
#[test]
fn auxiliary_requires_context_link_but_old_ipairs_owners_remain_valid() {
    let data = ipairs_definitions();
    data.validate().unwrap(); // Graph consistency alone does not admit a retained relation.
    let prototypes = SourceClosurePrototypes {
        schema_version: SOURCE_CLOSURE_PROTOTYPES_SCHEMA_VERSION,
        prototypes: vec![],
    };
    for result in [
        SourceProgramOwner::new(data.clone()),
        SourceProgramOwner::new_with_context(data.clone(), None, SourceProgramContext::default()),
        SourceProgramOwner::new_with_closures(data.clone(), None, None, prototypes.clone()),
        SourceProgramOwner::new_with_closures(
            data.clone(),
            None,
            Some(SourceProgramContext::default()),
            prototypes.clone(),
        ),
    ] {
        let error = result.unwrap_err();
        assert_eq!(error.kind, SourceProgramErrorKind::Binding);
        assert!(error.message.contains("retaining factory"));
    }
    SourceProgramOwner::new_with_closures(
        data.clone(),
        None,
        Some(ipairs_context()),
        prototypes.clone(),
    )
    .unwrap();
    let mut old = data;
    old.callbacks.truncate(1);
    old.intrinsics.retain(|id, _| id.0 == 1);
    let wire = serde_json::to_vec(&old).unwrap();
    let owner = SourceProgramOwner::new(old.clone()).unwrap();
    assert_eq!(owner.ipairs_aux_callback(SourceCallbackId(1)), None);
    SourceProgramOwner::from_bytes(&wire).unwrap();
    SourceProgramOwner::new_with_context(old.clone(), None, SourceProgramContext::default())
        .unwrap();
    SourceProgramOwner::new_with_closures(old, None, None, prototypes).unwrap();
}
#[test]
fn ipairs_links_reject_missing_wrong_and_fabricated_identity_descriptors() {
    for edit in 0..14 {
        let mut data = ipairs_definitions();
        let mut c = ipairs_context();
        let links = &mut c.iteration.as_mut().unwrap().ipairs_aux;
        match edit {
            0 => {
                links.remove(&SourceCallbackId(1));
            }
            1 => {
                links.insert(SourceCallbackId(2), SourceCallbackId(4));
            }
            2 => {
                links.insert(SourceCallbackId(1), SourceCallbackId(3));
            }
            3 => {
                links.insert(SourceCallbackId(1), SourceCallbackId(99));
            }
            4 => {
                links.insert(SourceCallbackId(99), SourceCallbackId(2));
            }
            5 => {
                links.insert(SourceCallbackId(0), SourceCallbackId(2));
            }
            6 => {
                links.insert(SourceCallbackId(1), SourceCallbackId(0));
            }
            7 => {
                data.intrinsics.remove(&SourceCallbackId(4));
            }
            8 => {
                data.callbacks[3].kind = SourceCallbackKind::Builtin {
                    symbol: "not_ipairs_aux".into(),
                };
            }
            9 => {
                data.callbacks[3].kind = SourceCallbackKind::Lua {
                    source: poe_optimizer_data::item_loading::ItemSourceSpan {
                        path: "fixture.lua".into(),
                        line: 1,
                        end_line: 1,
                        sha256: "b".repeat(64),
                    },
                };
            }
            10 => {
                data.intrinsics.remove(&SourceCallbackId(1));
            }
            11 => {
                data.intrinsics
                    .insert(SourceCallbackId(1), SourceProgramIntrinsic::Next);
            }
            12 => {
                data.intrinsics
                    .insert(SourceCallbackId(4), SourceProgramIntrinsic::Ipairs);
            }
            _ => {
                data.callbacks[3].upvalues.push(SourceUpvalue {
                    name: "fake".into(),
                    value: SourceValue::Callback(SourceCallbackId(1)),
                });
            }
        }
        assert!(
            SourceProgramOwner::new_with_context(data, None, c).is_err(),
            "edit {edit}"
        );
    }
    // The same exact auxiliary may be retained by more than one original factory.
    let mut c = ipairs_context();
    c.iteration
        .as_mut()
        .unwrap()
        .ipairs_aux
        .insert(SourceCallbackId(3), SourceCallbackId(4));
    let mut data = ipairs_definitions();
    data.intrinsics.remove(&SourceCallbackId(2));
    SourceProgramOwner::new_with_context(data, None, c).unwrap();
}
#[test]
fn optional_ipairs_links_preserve_old_iteration_wire_and_bound_untrusted_maps() {
    let old = br#"{"table_order":{},"pairs_next":{}}"#;
    let decoded: SourceProgramIteration = serde_json::from_slice(old).unwrap();
    assert!(decoded.ipairs_aux.is_empty());
    assert_eq!(serde_json::to_vec(&decoded).unwrap(), old);
    let explicit_empty: SourceProgramIteration =
        serde_json::from_str(r#"{"table_order":{},"pairs_next":{},"ipairs_aux":{}}"#).unwrap();
    assert_eq!(serde_json::to_vec(&explicit_empty).unwrap(), old);
    let c = ipairs_context();
    let wire = serde_json::to_vec(&c).unwrap();
    assert_eq!(
        SourceProgramContext::from_bytes(&wire, &ipairs_definitions(), None).unwrap(),
        c
    );
    for wire in [
        r#"{"table_order":{},"pairs_next":{},"ipairs_aux":{"1":2,"01":4}}"#,
        r#"{"table_order":{},"pairs_next":{},"ipairs_aux":{"1":2,"1":4}}"#,
        r#"{"table_order":{},"pairs_next":{},"ipairs_aux":null}"#,
        r#"{"table_order":{},"pairs_next":{},"ipairs_aux":{},"unknown":true}"#,
        r#"{"table_order":{},"ipairs_aux":{}}"#,
    ] {
        assert!(
            serde_json::from_str::<SourceProgramIteration>(wire).is_err(),
            "{wire}"
        );
    }
    let mut metadata = SourceProgramIteration {
        ipairs_aux: (1..=256)
            .map(|id| (SourceCallbackId(id), SourceCallbackId(2)))
            .collect(),
        ..SourceProgramIteration::default()
    };
    metadata.validate_shape().unwrap();
    assert_eq!(
        serde_json::from_slice::<SourceProgramIteration>(&serde_json::to_vec(&metadata).unwrap())
            .unwrap(),
        metadata
    );
    metadata
        .ipairs_aux
        .insert(SourceCallbackId(257), SourceCallbackId(2));
    assert_eq!(
        metadata.validate_shape().unwrap_err().kind,
        SourceProgramErrorKind::ResourceLimit
    );
    assert!(
        serde_json::from_slice::<SourceProgramIteration>(&serde_json::to_vec(&metadata).unwrap())
            .is_err()
    );
}
