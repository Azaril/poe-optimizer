use super::*;
use crate::parser_program::runtime::ProgramRuntimeErrorKind as Kind;
use poe_optimizer_data::game_data::bundled_snapshot;
use std::sync::OnceLock;

fn owner() -> ModifierParserCatalog {
    static OWNER: OnceLock<ModifierParserCatalog> = OnceLock::new();
    OWNER
        .get_or_init(|| bundled_snapshot().unwrap().modifier_parser().clone())
        .clone()
}
fn heap() -> Heap {
    Heap::new(
        &owner(),
        &ProgramValueGraph::default(),
        &ProgramLimits::default(),
    )
    .unwrap()
    .0
}
fn text(bytes: &[u8]) -> V {
    V::Bytes(Arc::from(bytes))
}
fn num(n: f64) -> ProgramValue {
    ProgramValue::Number(n)
}
fn table(id: u32) -> ProgramValue {
    ProgramValue::Table(ProgramTableId(id))
}
fn assert_kind<T>(result: Result<T>, expected: Kind) {
    assert_eq!(result.err().expect("expected failure").kind, expected);
}

#[test]
fn imported_graph_preserves_alias_cycles_table_keys_and_raw_scalar_bits() {
    let owner = owner();
    let callback = ParserCallbackId(1);
    let nan = f64::from_bits(0x7ff8_0000_0000_0042);
    let input = ProgramValueGraph {
        values: vec![
            table(1),
            table(1),
            num(nan),
            num(-0.0),
            ProgramValue::Bytes(vec![0xff, 0, b'a']),
        ],
        tables: vec![ProgramTable {
            entries: vec![
                (table(1), table(1)),
                (
                    ProgramValue::Callback(callback),
                    ProgramValue::Boolean(false),
                ),
            ],
        }],
    };
    let (mut heap, values) = Heap::new(&owner, &input, &ProgramLimits::default()).unwrap();
    assert!(values[0].lua_equal(&values[1]));
    assert!(
        heap.get(&values[0], &values[0])
            .unwrap()
            .lua_equal(&values[0])
    );
    assert!(matches!(
        heap.get(&values[0], &V::Callback(callback)).unwrap(),
        V::Boolean(false)
    ));
    let output = heap.freeze(&values).unwrap();
    assert_eq!(output.tables.len(), 1);
    assert_eq!(output.values[0], table(1));
    assert_eq!(output.values[1], table(1));
    let ProgramValue::Number(actual) = output.values[2] else {
        panic!()
    };
    assert_eq!(actual.to_bits(), nan.to_bits());
    let ProgramValue::Number(actual) = output.values[3] else {
        panic!()
    };
    assert_eq!(actual.to_bits(), (-0.0f64).to_bits());
    assert_eq!(output.values[4], input.values[4]);
    assert!(output.tables[0].entries.contains(&(table(1), table(1))));
}

#[test]
fn fresh_alias_writes_remain_shared_until_the_explicit_graph_boundary() {
    let mut heap = heap();
    let root = heap.new_table().unwrap();
    let alias = root.clone();
    let child = heap.new_table().unwrap();
    heap.set(&root, text(b"left"), child.clone()).unwrap();
    heap.set(&root, text(b"right"), child.clone()).unwrap();
    heap.set(&alias, V::Number(1.0), V::Boolean(true)).unwrap();
    heap.set(&child, text(b"parent"), root.clone()).unwrap();
    assert!(matches!(
        heap.get(&root, &V::Number(1.0)).unwrap(),
        V::Boolean(true)
    ));
    let output = heap.freeze(&[root, alias, child]).unwrap();
    assert_eq!(output.values, vec![table(1), table(1), table(2)]);
    assert_eq!(output.tables.len(), 2);
    let root_entries = &output.tables[0].entries;
    assert!(root_entries.contains(&(ProgramValue::Bytes(b"left".to_vec()), table(2))));
    assert!(root_entries.contains(&(ProgramValue::Bytes(b"right".to_vec()), table(2))));
    assert_eq!(
        output.tables[1].entries,
        vec![(ProgramValue::Bytes(b"parent".to_vec()), table(1))]
    );
}

#[test]
fn heap_argument_and_definition_id_domains_are_distinct_and_borrowed_writes_defer() {
    let input = ProgramValueGraph {
        values: vec![table(1)],
        tables: vec![ProgramTable::default()],
    };
    let (mut heap, values) = Heap::new(&owner(), &input, &ProgramLimits::default()).unwrap();
    let fresh = heap.new_table().unwrap();
    let definition = heap.definition(ParserTableId(1)).unwrap();
    assert!(!fresh.lua_equal(&values[0]));
    assert!(!fresh.lua_equal(&definition));
    assert!(!definition.lua_equal(&values[0]));
    assert_kind(
        heap.set(&values[0], text(b"new"), V::Boolean(true)),
        Kind::UnsupportedCapability,
    );
    assert_kind(
        heap.set(&definition, text(b"new"), V::Boolean(true)),
        Kind::UnsupportedCapability,
    );
    assert!(matches!(
        heap.get(&values[0], &text(b"new")).unwrap(),
        V::Nil
    ));
    heap.set(&fresh, values[0].clone(), V::Boolean(false))
        .unwrap();
    heap.set(&fresh, definition, V::Boolean(true)).unwrap();
    assert!(matches!(
        heap.get(&fresh, &values[0]).unwrap(),
        V::Boolean(false)
    ));
}

#[test]
fn lua_key_rules_normalize_zero_preserve_identity_and_distinguish_read_write_errors() {
    let mut heap = heap();
    let root = heap.new_table().unwrap();
    heap.set(&root, V::Number(-0.0), V::Number(1.0)).unwrap();
    heap.set(&root, V::Number(0.0), V::Number(2.0)).unwrap();
    heap.set(&root, V::Boolean(false), V::Number(3.0)).unwrap();
    heap.set(&root, V::Number(f64::INFINITY), V::Number(4.0))
        .unwrap();
    assert!(matches!(
        heap.get(&root, &V::Number(-0.0)).unwrap(),
        V::Number(2.0)
    ));
    assert!(matches!(
        heap.get(&root, &V::Boolean(false)).unwrap(),
        V::Number(3.0)
    ));
    assert!(matches!(
        heap.get(&root, &V::Number(f64::INFINITY)).unwrap(),
        V::Number(4.0)
    ));
    for key in [V::Nil, V::Number(f64::NAN)] {
        assert!(matches!(heap.get(&root, &key).unwrap(), V::Nil));
        assert_kind(heap.set(&root, key, V::Boolean(true)), Kind::Source);
    }
    for scalar in [
        V::Nil,
        V::Boolean(false),
        V::Number(0.0),
        V::Callback(ParserCallbackId(1)),
    ] {
        assert_kind(heap.get(&scalar, &V::Nil), Kind::Source);
        assert_kind(heap.set(&scalar, V::Nil, V::Nil), Kind::Source);
    }
    assert_kind(
        heap.get(&text(b"str"), &text(b"sub")),
        Kind::UnsupportedCapability,
    );
    assert_kind(
        heap.set(&text(b"str"), V::Number(1.0), V::Nil),
        Kind::Source,
    );
    heap.set(&root, V::Number(-0.0), V::Nil).unwrap();
    assert!(matches!(heap.get(&root, &V::Number(0.0)).unwrap(), V::Nil));
}

#[test]
fn truthiness_and_equality_use_lua_values_without_coercion() {
    for value in [
        V::Number(0.0),
        V::Number(f64::NAN),
        text(b""),
        V::Table(TableRef::Heap(1)),
    ] {
        assert!(value.truthy());
    }
    assert!(!V::Nil.truthy());
    assert!(!V::Boolean(false).truthy());
    assert!(!V::Number(f64::NAN).lua_equal(&V::Number(f64::NAN)));
    assert!(V::Number(-0.0).lua_equal(&V::Number(0.0)));
    assert!(!V::Number(1.0).lua_equal(&text(b"1")));
    assert!(text(&[0xff, 0]).lua_equal(&text(&[0xff, 0])));
    assert!(!V::Callback(ParserCallbackId(1)).lua_equal(&V::Callback(ParserCallbackId(2))));
}

#[test]
fn complete_input_validation_rejects_dead_invalid_nodes_and_normalized_duplicates() {
    let catalog = owner();
    let limits = ProgramLimits::default();
    let failures = [
        ProgramTable {
            entries: vec![(num(1.0), table(2))],
        },
        ProgramTable {
            entries: vec![(table(0), num(1.0))],
        },
        ProgramTable {
            entries: vec![(ProgramValue::Callback(ParserCallbackId(u32::MAX)), num(1.0))],
        },
        ProgramTable {
            entries: vec![(ProgramValue::Nil, num(1.0))],
        },
        ProgramTable {
            entries: vec![(num(f64::NAN), num(1.0))],
        },
        ProgramTable {
            entries: vec![(num(1.0), ProgramValue::Nil)],
        },
        ProgramTable {
            entries: vec![(num(-0.0), num(1.0)), (num(0.0), num(2.0))],
        },
        ProgramTable {
            entries: vec![(table(1), num(1.0)), (table(1), num(2.0))],
        },
    ];
    for table in failures {
        assert_kind(
            Heap::new(
                &catalog,
                &ProgramValueGraph {
                    values: vec![],
                    tables: vec![table],
                },
                &limits,
            ),
            Kind::InvalidInput,
        );
    }
}

#[test]
fn dense_length_rejects_holes_and_append_nil_does_not_advance_the_boundary() {
    let mut heap = heap();
    let root = heap.new_table().unwrap();
    heap.set(&root, text(b"name"), V::Boolean(true)).unwrap();
    heap.set(&root, V::Number(-1.0), V::Boolean(true)).unwrap();
    heap.set(&root, V::Number(1.5), V::Boolean(true)).unwrap();
    assert_eq!(heap.dense_len(&root).unwrap(), 0);
    heap.append(&root, V::Nil).unwrap();
    assert_eq!(heap.dense_len(&root).unwrap(), 0);
    heap.append(&root, V::Number(10.0)).unwrap();
    heap.set(&root, V::Number(3.0), V::Boolean(true)).unwrap();
    assert_kind(heap.dense_len(&root), Kind::UnsupportedCapability);
    assert_kind(
        heap.append(&root, V::Number(20.0)),
        Kind::UnsupportedCapability,
    );
    heap.set(&root, V::Number(2.0), V::Boolean(true)).unwrap();
    assert_eq!(heap.dense_len(&root).unwrap(), 3);
    heap.set(&root, V::Number(3.0), V::Nil).unwrap();
    assert_eq!(heap.dense_len(&root).unwrap(), 2);
    heap.set(&root, V::Number(2.0), V::Nil).unwrap();
    heap.append(&root, V::Number(30.0)).unwrap();
    assert_eq!(heap.dense_len(&root).unwrap(), 2);
    assert!(matches!(
        heap.get(&root, &V::Number(2.0)).unwrap(),
        V::Number(30.0)
    ));
}

#[test]
fn allocation_bounds_reject_before_adding_heap_entries_or_string_payload() {
    let limits = ProgramLimits {
        max_values: 2,
        max_bytes: 2,
        max_tables: 1,
        ..ProgramLimits::default()
    };
    let (mut heap, _) = Heap::new(&owner(), &ProgramValueGraph::default(), &limits).unwrap();
    let root = heap.new_table().unwrap();
    assert_kind(heap.new_table(), Kind::ResourceBound);
    assert_eq!(heap.stats().tables, 1);
    assert_kind(heap.bytes(b"abc"), Kind::ResourceBound);
    assert_eq!(heap.stats().bytes, 0);
    let bytes = heap.bytes(b"xy").unwrap();
    assert_eq!(bytes.as_bytes(), Some(b"xy".as_slice()));
    assert_eq!(heap.remaining_bytes(), 0);
    // A numeric entry needs key+value and the cached integer-boundary index.
    assert_kind(
        heap.set(&root, V::Number(1.0), V::Boolean(true)),
        Kind::ResourceBound,
    );
    assert!(matches!(heap.get(&root, &V::Number(1.0)).unwrap(), V::Nil));
    assert_eq!(heap.stats().values, 0);
    assert_eq!(heap.dense_len(&root).unwrap(), 0);
}

#[test]
fn graph_import_export_and_result_pack_use_cumulative_independent_bounds() {
    let owner = owner();
    let graph = ProgramValueGraph {
        values: vec![ProgramValue::Bytes(b"abc".to_vec())],
        tables: vec![],
    };
    let limits = ProgramLimits {
        max_bytes: 5,
        ..ProgramLimits::default()
    };
    let (mut heap, values) = Heap::new(&owner, &graph, &limits).unwrap();
    assert_eq!(heap.stats().bytes, 3);
    assert_kind(heap.freeze(&values), Kind::ResourceBound);
    assert_eq!(heap.stats().bytes, 3);
    let limits = ProgramLimits {
        max_values: 0,
        ..ProgramLimits::default()
    };
    assert_kind(Heap::new(&owner, &graph, &limits), Kind::ResourceBound);
    let limits = ProgramLimits {
        max_results: 0,
        ..ProgramLimits::default()
    };
    let (mut heap, _) = Heap::new(&owner, &ProgramValueGraph::default(), &limits).unwrap();
    assert_kind(heap.freeze(&[V::Nil]), Kind::ResourceBound);
    assert!(heap.freeze(&[]).unwrap().values.is_empty());
}

#[test]
fn deeply_nested_cyclic_graphs_import_and_export_iteratively() {
    let n = 2_048u32;
    let graph = ProgramValueGraph {
        values: vec![table(1)],
        tables: (1..=n)
            .map(|id| ProgramTable {
                entries: vec![(num(1.0), table(if id == n { 1 } else { id + 1 }))],
            })
            .collect(),
    };
    let (mut heap, values) = Heap::new(&owner(), &graph, &ProgramLimits::default()).unwrap();
    let output = heap.freeze(&values).unwrap();
    assert_eq!(output, graph);
}

#[test]
fn capture_tables_stay_borrowed_and_definition_strings_are_charged_when_reached() {
    let mut data = owner().data().clone();
    let callback_index = data
        .callbacks
        .iter()
        .position(|c| c.upvalues.len() < 124)
        .unwrap();
    let callback = ParserCallbackId(callback_index as u32 + 1);
    let offset = data.callbacks[callback_index].upvalues.len() as u16;
    let id = data.policy.mod_flags;
    data.callbacks[callback_index].upvalues.extend([
        poe_optimizer_data::modifier_parser::ParserUpvalue {
            name: "heap_test_table".into(),
            value: ParserValue::Table(id),
        },
        poe_optimizer_data::modifier_parser::ParserUpvalue {
            name: "heap_test_bytes".into(),
            value: ParserValue::Text("abc".into()),
        },
    ]);
    let owner = ModifierParserCatalog::new(data).unwrap();
    let limits = ProgramLimits {
        max_bytes: 3,
        ..ProgramLimits::default()
    };
    let (mut heap, _) = Heap::new(&owner, &ProgramValueGraph::default(), &limits).unwrap();
    let captured = heap.capture(callback, offset).unwrap();
    assert!(captured.lua_equal(&heap.definition(id).unwrap()));
    assert_kind(
        heap.set(&captured, text(b"test"), V::Nil),
        Kind::UnsupportedCapability,
    );
    assert_eq!(
        heap.capture(callback, offset + 1).unwrap().as_bytes(),
        Some(b"abc".as_slice())
    );
    assert_eq!(heap.stats().bytes, 3);
    assert_kind(heap.capture(callback, offset + 1), Kind::ResourceBound);
}
