use SourceSessionValue as V;
use poe_optimizer_data::{
    item_loading::{ItemLoadingSource, ItemSourceSpan},
    source_program::*,
};
use std::collections::{BTreeMap, BTreeSet};
fn owner() -> SourceProgramOwner {
    let callback = SourceCallback {
        kind: SourceCallbackKind::Lua {
            source: ItemSourceSpan {
                path: "fixture.lua".into(),
                line: 1,
                end_line: 1,
                sha256: "b".repeat(64),
            },
        },
        environment: SourceEnvironment::OriginalGlobals,
        upvalues: vec![],
    };
    SourceProgramOwner::new_with_closures(
        SourceProgramDefinitions {
            schema_version: SOURCE_PROGRAM_DEFINITIONS_SCHEMA_VERSION,
            source: ItemLoadingSource {
                upstream_revision: "a".repeat(40),
                files: [("fixture.lua".into(), "c".repeat(64))].into(),
                construction_spans: BTreeMap::new(),
                module_order: vec!["fixture.lua".into()],
            },
            tables: vec![SourceTable::default()],
            callbacks: vec![callback.clone(), callback],
            roots: vec![],
            intrinsics: BTreeMap::new(),
        },
        None,
        None,
        SourceClosurePrototypes {
            schema_version: SOURCE_CLOSURE_PROTOTYPES_SCHEMA_VERSION,
            prototypes: vec![SourceClosurePrototype {
                callback: SourceCallbackId(2),
            }],
        },
    )
    .unwrap()
}
fn input(keys: Vec<V>, length: Option<u32>) -> SourceSessionInput {
    let owner = owner();
    let prototype = owner
        .bind_closure_prototype(SourceClosurePrototypeId(1))
        .unwrap();
    let mut order = keys.clone();
    order.reverse();
    SourceSessionInput {
        owner,
        state: SourceSessionValueGraph {
            values: vec![V::Table(SourceSessionTableId(1))],
            tables: vec![SourceSessionTable {
                entries: keys
                    .into_iter()
                    .map(|key| (key, V::Boolean(false)))
                    .collect(),
            }],
        },
        coverage: BTreeMap::new(),
        class_bindings: BTreeMap::new(),
        cells: vec![],
        closures: vec![SourceSessionClosure {
            prototype,
            captures: vec![],
        }],
        traversal: Some(SourceSessionTraversal {
            tables: [(
                SourceSessionTableId(1),
                SourceSessionTableTraversal {
                    order,
                    raw_length: length,
                },
            )]
            .into(),
        }),
    }
}
fn validate(input: &SourceSessionInput) -> SourceProgramResult<()> {
    input.validate_traversal(10, 100, 1000)
}
fn row(input: &mut SourceSessionInput) -> &mut SourceSessionTableTraversal {
    input
        .traversal
        .as_mut()
        .unwrap()
        .tables
        .get_mut(&SourceSessionTableId(1))
        .unwrap()
}
fn coverage() -> SourceTableCoverage {
    SourceTableCoverage {
        inventory: SourceTableInventory::Complete,
        known_absent: BTreeSet::new(),
        unavailable: BTreeSet::new(),
        index_fallback: SourceTableIndexFallback::Nil,
        call_fallback: SourceTableCallFallback::NonCallable,
    }
}
#[test]
fn arbitrary_raw_key_identity_and_unordered_entries_are_independent() {
    let mut input = input(
        vec![
            V::Boolean(false),
            V::Boolean(true),
            V::Number(-0.0),
            V::Number(1.5),
            V::Number(f64::INFINITY),
            V::Number(f64::NEG_INFINITY),
            V::Bytes(vec![0, 255]),
            V::Table(SourceSessionTableId(1)),
            V::DefinitionTable(SourceTableId(1)),
            V::Callback(SourceCallbackId(1)),
            V::Closure(SourceSessionClosureId(1)),
        ],
        Some(0),
    );
    let wire = serde_json::to_vec(input.owner.definitions().unwrap()).unwrap();
    validate(&input).unwrap();
    input.state.tables[0].entries.reverse();
    validate(&input).unwrap();
    assert_eq!(
        wire,
        serde_json::to_vec(input.owner.definitions().unwrap()).unwrap()
    );
    row(&mut input).order.iter_mut().for_each(|key| {
        if matches!(key,V::Number(value) if *value == 0.0) {
            *key = V::Number(0.0);
        }
    });
    validate(&input).unwrap();
}
#[test]
fn absence_of_observation_and_explicit_empty_proof_are_distinct() {
    let mut input = input(vec![], Some(0));
    validate(&input).unwrap();
    input.traversal = None;
    validate(&input).unwrap();
    input.traversal = Some(SourceSessionTraversal::default());
    validate(&input).unwrap();
    input.traversal.as_mut().unwrap().tables.insert(
        SourceSessionTableId(0),
        SourceSessionTableTraversal::default(),
    );
    assert!(validate(&input).is_err());
}
#[test]
fn permutation_rejects_duplicate_missing_nil_and_nan_keys() {
    for keys in [
        vec![V::Number(-0.0), V::Number(0.0)],
        vec![V::Nil],
        vec![V::Number(f64::NAN)],
    ] {
        assert!(validate(&input(keys, None)).is_err());
    }
    let base = input(vec![V::Number(1.0), V::Bytes(b"1".to_vec())], None);
    let mut bad = base.clone();
    row(&mut bad).order.pop();
    assert!(validate(&bad).is_err());
    let mut bad = base.clone();
    row(&mut bad).order[1] = row(&mut bad).order[0].clone();
    assert!(validate(&bad).is_err());
    let mut bad = base.clone();
    row(&mut bad).order[1] = V::Number(2.0);
    assert!(validate(&bad).is_err());
    let mut bad = base;
    bad.state.tables[0].entries[0].1 = V::Nil;
    assert!(validate(&bad).is_err());
}
#[test]
fn raw_length_is_an_observed_boundary_not_a_dense_or_max_key_guess() {
    let mut sparse = input(vec![V::Number(2.0)], Some(0));
    validate(&sparse).unwrap();
    row(&mut sparse).raw_length = Some(2);
    validate(&sparse).unwrap();
    row(&mut sparse).raw_length = Some(1);
    assert!(validate(&sparse).is_err());
    let mut dense = input(vec![V::Number(1.0), V::Number(2.0)], Some(2));
    validate(&dense).unwrap();
    row(&mut dense).raw_length = Some(0);
    assert!(validate(&dense).is_err());
    row(&mut dense).raw_length = Some(1);
    assert!(validate(&dense).is_err());
    validate(&input(vec![V::Number(f64::from(u32::MAX))], Some(u32::MAX))).unwrap();
    assert!(
        validate(&input(
            vec![
                V::Number(f64::from(u32::MAX)),
                V::Number(f64::from(u32::MAX) + 1.0)
            ],
            Some(u32::MAX)
        ))
        .is_err()
    );
}
#[test]
fn partial_or_contradictory_raw_coverage_cannot_certify_traversal() {
    let mut input = input(vec![V::Bytes(b"key".to_vec()), V::Number(-0.0)], Some(0));
    input.coverage.insert(SourceSessionTableId(1), coverage());
    validate(&input).unwrap();
    input
        .coverage
        .get_mut(&SourceSessionTableId(1))
        .unwrap()
        .inventory = SourceTableInventory::Selective;
    assert!(validate(&input).is_err());
    input
        .coverage
        .get_mut(&SourceSessionTableId(1))
        .unwrap()
        .inventory = SourceTableInventory::Complete;
    input
        .coverage
        .get_mut(&SourceSessionTableId(1))
        .unwrap()
        .unavailable
        .insert(SourceTableKey::Text("omitted".into()));
    assert!(validate(&input).is_err());
    input
        .coverage
        .get_mut(&SourceSessionTableId(1))
        .unwrap()
        .unavailable
        .clear();
    for key in [
        SourceTableKey::Text("key".into()),
        SourceTableKey::Integer(0),
    ] {
        input
            .coverage
            .get_mut(&SourceSessionTableId(1))
            .unwrap()
            .known_absent = BTreeSet::from([key]);
        assert!(validate(&input).is_err());
    }
    input
        .coverage
        .get_mut(&SourceSessionTableId(1))
        .unwrap()
        .known_absent = BTreeSet::from([SourceTableKey::Integer(1)]);
    input
        .coverage
        .get_mut(&SourceSessionTableId(1))
        .unwrap()
        .index_fallback = SourceTableIndexFallback::Unavailable;
    input
        .coverage
        .get_mut(&SourceSessionTableId(1))
        .unwrap()
        .call_fallback = SourceTableCallFallback::Unavailable;
    validate(&input).unwrap(); // __index/__call do not alter raw traversal/length.
}
#[test]
fn local_and_foreign_reference_keys_fail_closed() {
    for key in [
        V::Table(SourceSessionTableId(0)),
        V::Table(SourceSessionTableId(2)),
        V::Closure(SourceSessionClosureId(0)),
        V::Closure(SourceSessionClosureId(2)),
        V::Callback(SourceCallbackId(0)),
        V::Callback(SourceCallbackId(2)),
        V::DefinitionTable(SourceTableId(2)),
    ] {
        assert!(validate(&input(vec![key], None)).is_err());
    }
    let mut input = input(vec![V::Closure(SourceSessionClosureId(1))], None);
    input.closures[0].prototype = owner()
        .bind_closure_prototype(SourceClosurePrototypeId(1))
        .unwrap();
    assert!(validate(&input).is_err());
}
#[test]
fn facet_budgets_precede_key_normalization_and_allocation() {
    let input = input(vec![V::Bytes(vec![0; 8])], None);
    for (tables, keys, bytes) in [(0, 10, 100), (10, 0, 100), (10, 10, 15)] {
        assert_eq!(
            input
                .validate_traversal(tables, keys, bytes)
                .unwrap_err()
                .kind,
            SourceProgramErrorKind::ResourceLimit
        );
    }
    input.validate_traversal(1, 1, 16).unwrap();
    let mut oversized = input.clone();
    oversized.state.tables[0].entries[0].0 = V::Nil;
    assert_eq!(
        oversized.validate_traversal(1, 0, 0).unwrap_err().kind,
        SourceProgramErrorKind::ResourceLimit
    );
}
