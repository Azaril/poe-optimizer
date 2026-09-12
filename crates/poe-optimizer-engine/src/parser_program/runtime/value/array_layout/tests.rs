use super::super::{ProgramTable, ProgramTableId, ProgramValue, ProgramValueGraph, tests::heap};
use super::*;
use crate::parser_program::runtime::{ProgramLimits, ProgramRuntimeErrorKind as Kind};
use std::collections::BTreeMap;

fn fixture() -> (Heap<'static>, MatchBudget) {
    (heap(), MatchBudget::new(ProgramLimits::default().pattern))
}
fn set(heap: &mut Heap, table: &V, key: f64, value: V, work: &mut MatchBudget) {
    heap.set(table, V::Number(key), value, work).unwrap();
}
fn layout(heap: &Heap, table: &V) -> Option<ArrayLayout> {
    let TableRef::Heap(id) = table_ref(table).unwrap() else {
        panic!()
    };
    heap.tables[index(id).unwrap()].array_layout
}
fn next(heap: &mut Heap, table: &V, control: V, work: &mut MatchBudget) -> Result<Option<(V, V)>> {
    heap.definition_next(table, &control, &BTreeMap::new(), work)
}
fn assert_kind<T>(result: Result<T>, kind: Kind) {
    assert_eq!(result.err().expect("expected error").kind, kind);
}

#[test]
fn empty_source_seed_derives_zero_and_sparse_two_slots_without_import_inference() {
    let (mut heap, mut work) = fixture();
    let unknown = heap.new_table().unwrap();
    set(&mut heap, &unknown, 2.0, V::Boolean(false), &mut work);
    assert_kind(
        heap.raw_len(&unknown, &mut work),
        Kind::UnsupportedCapability,
    );
    let table = heap.native_empty_table(&mut work).unwrap();
    assert_eq!(heap.raw_len(&table, &mut work).unwrap(), 0);
    assert!(
        next(&mut heap, &table, V::Nil, &mut work)
            .unwrap()
            .is_none()
    );
    set(&mut heap, &table, 2.0, V::Boolean(false), &mut work);
    assert_eq!(layout(&heap, &table).unwrap().slots, 3);
    assert_eq!(heap.raw_len(&table, &mut work).unwrap(), 2);
    assert_kind(
        heap.operator_len(&table, &mut work),
        Kind::UnsupportedCapability,
    );
    let (key, value) = next(&mut heap, &table, V::Nil, &mut work).unwrap().unwrap();
    assert!(key.lua_equal(&V::Number(2.0)) && value.lua_equal(&V::Boolean(false)));
    assert!(
        next(&mut heap, &table, V::Number(2.0), &mut work)
            .unwrap()
            .is_none()
    );
}
#[test]
fn in_capacity_holes_nil_and_deleted_controls_preserve_the_allocation_history() {
    let (mut heap, mut work) = fixture();
    let table = heap.native_empty_table(&mut work).unwrap();
    set(&mut heap, &table, 2.0, V::Nil, &mut work);
    assert_eq!(layout(&heap, &table).unwrap().slots, 3);
    assert_eq!(heap.raw_len(&table, &mut work).unwrap(), 0);
    assert!(
        next(&mut heap, &table, V::Number(1.0), &mut work)
            .unwrap()
            .is_none()
    );
    set(&mut heap, &table, 2.0, V::Number(-0.0), &mut work);
    let (_, value) = next(&mut heap, &table, V::Number(1.0), &mut work)
        .unwrap()
        .unwrap();
    assert!(matches!(value,V::Number(value) if value.to_bits()==(-0.0f64).to_bits()));
    set(&mut heap, &table, 2.0, V::Nil, &mut work);
    assert_eq!(layout(&heap, &table).unwrap().slots, 3);
    assert_eq!(layout(&heap, &table).unwrap().live, 0);
    assert!(
        next(&mut heap, &table, V::Number(2.0), &mut work)
            .unwrap()
            .is_none()
    );
    assert_kind(
        next(&mut heap, &table, V::Number(3.0), &mut work),
        Kind::Source,
    );
}
#[test]
fn growth_uses_current_occupancy_and_raw_length_uses_source_binary_search() {
    let (mut heap, mut work) = fixture();
    let table = heap.native_empty_table(&mut work).unwrap();
    for key in 1..=5 {
        set(
            &mut heap,
            &table,
            f64::from(key),
            V::Boolean(true),
            &mut work,
        );
    }
    assert_eq!(layout(&heap, &table).unwrap().slots, 9);
    for key in 1..=5 {
        set(&mut heap, &table, f64::from(key), V::Nil, &mut work);
    }
    set(&mut heap, &table, 6.0, V::Boolean(true), &mut work);
    assert_eq!(heap.raw_len(&table, &mut work).unwrap(), 0);
    assert!(
        next(&mut heap, &table, V::Number(5.0), &mut work)
            .unwrap()
            .unwrap()
            .0
            .lua_equal(&V::Number(6.0))
    );
    set(&mut heap, &table, 8.0, V::Boolean(true), &mut work);
    assert_eq!(heap.raw_len(&table, &mut work).unwrap(), 8);
    set(&mut heap, &table, 8.0, V::Nil, &mut work);
    set(&mut heap, &table, 9.0, V::Boolean(true), &mut work);
    assert!(
        layout(&heap, &table).is_none(),
        "sparse occupancy needs a hash allocation"
    );
    assert_kind(heap.raw_len(&table, &mut work), Kind::UnsupportedCapability);
}
#[test]
fn zero_signed_zero_and_table_values_have_exact_numeric_order_and_aliases() {
    let (mut heap, mut work) = fixture();
    let table = heap.native_empty_table(&mut work).unwrap();
    set(&mut heap, &table, -0.0, table.clone(), &mut work);
    set(&mut heap, &table, 2.0, V::Boolean(false), &mut work);
    let (key, value) = next(&mut heap, &table, V::Nil, &mut work).unwrap().unwrap();
    assert!(matches!(key,V::Number(value) if value.to_bits()==0.0f64.to_bits()));
    assert!(value.lua_equal(&table));
    assert!(
        next(&mut heap, &table, V::Number(-0.0), &mut work)
            .unwrap()
            .unwrap()
            .0
            .lua_equal(&V::Number(2.0))
    );
    set(&mut heap, &table, 0.0, V::Nil, &mut work);
    assert!(
        next(&mut heap, &table, V::Number(0.0), &mut work)
            .unwrap()
            .unwrap()
            .0
            .lua_equal(&V::Number(2.0))
    );
}
#[test]
fn hash_requiring_writes_drop_facts_even_for_nil_and_never_recover_from_raw_equality() {
    let (mut heap, mut work) = fixture();
    for key in [
        V::Number(3.0),
        V::Number(-1.0),
        V::Number(1.5),
        V::Number(f64::INFINITY),
        V::Boolean(false),
        V::Bytes(std::sync::Arc::from(b"field".as_slice())),
    ] {
        let table = heap.native_empty_table(&mut work).unwrap();
        heap.set(&table, key.clone(), V::Nil, &mut work).unwrap();
        assert!(layout(&heap, &table).is_none());
        set(&mut heap, &table, 2.0, V::Boolean(true), &mut work);
        assert_kind(heap.raw_len(&table, &mut work), Kind::UnsupportedCapability);
        assert_kind(
            next(&mut heap, &table, V::Nil, &mut work),
            Kind::UnsupportedCapability,
        );
        heap.set(&table, key, V::Nil, &mut work).unwrap();
        assert!(layout(&heap, &table).is_none());
    }
}
#[test]
fn failed_key_and_resource_writes_keep_values_capacity_and_prior_charges() {
    let (mut heap, mut work) = fixture();
    let table = heap.native_empty_table(&mut work).unwrap();
    set(&mut heap, &table, 2.0, V::Boolean(false), &mut work);
    for key in [V::Nil, V::Number(f64::NAN)] {
        assert_kind(
            heap.set(&table, key, V::Boolean(true), &mut work),
            Kind::Source,
        );
        assert_eq!(heap.raw_len(&table, &mut work).unwrap(), 2);
    }
    let before = heap.stats().values;
    // Existing capacity is3; adding key3 chooses5slots, then needs3 raw-map units.
    heap.budget.limits.max_values = before + 2;
    assert_kind(
        heap.set(&table, V::Number(3.0), V::Boolean(true), &mut work),
        Kind::ResourceBound,
    );
    assert_eq!(
        heap.stats().values,
        before + 2,
        "growth charge remains despite later map allocation failure"
    );
    assert_eq!(layout(&heap, &table).unwrap().slots, 3);
    assert_eq!(layout(&heap, &table).unwrap().live, 1);
    assert!(matches!(
        heap.raw_get(&table, &V::Number(3.0)).unwrap(),
        V::Nil
    ));
    assert_eq!(heap.raw_len(&table, &mut work).unwrap(), 2);
}
#[test]
fn source_append_preserves_layout_and_raw_snapshot_cannot_recover_it() {
    let (mut heap, mut work) = fixture();
    let table = heap.native_empty_table(&mut work).unwrap();
    set(&mut heap, &table, 2.0, V::Boolean(false), &mut work);
    heap.append(&table, V::Boolean(true), &mut work).unwrap();
    assert_eq!(heap.raw_len(&table, &mut work).unwrap(), 3);
    assert_eq!(layout(&heap, &table).unwrap().slots, 5);
    let snapshot = heap.freeze(std::slice::from_ref(&table)).unwrap();
    let imported = heap.import(&snapshot, true).unwrap();
    assert_kind(
        heap.raw_len(&imported[0], &mut work),
        Kind::UnsupportedCapability,
    );
    assert_kind(
        next(&mut heap, &imported[0], V::Nil, &mut work),
        Kind::UnsupportedCapability,
    );
    assert_eq!(heap.raw_len(&table, &mut work).unwrap(), 3);
}
#[test]
fn work_and_allocation_limits_cover_seed_transitions_length_and_traversal() {
    let (mut heap, mut work) = fixture();
    let before = heap.stats();
    heap.budget.limits.max_values = before.values + METADATA_VALUES - 1;
    assert_kind(heap.native_empty_table(&mut work), Kind::ResourceBound);
    assert_eq!(heap.stats().tables, before.tables);
    let (mut heap, mut work) = fixture();
    let table = heap.native_empty_table(&mut work).unwrap();
    set(&mut heap, &table, 2.0, V::Boolean(false), &mut work);
    let mut limits = ProgramLimits::default().pattern;
    limits.max_steps = 1;
    let mut bounded = MatchBudget::new(limits);
    assert_kind(
        heap.set(&table, V::Number(3.0), V::Boolean(true), &mut bounded),
        Kind::ResourceBound,
    );
    assert_eq!(layout(&heap, &table).unwrap().slots, 3);
    let mut bounded = MatchBudget::new(limits);
    assert_kind(
        next(&mut heap, &table, V::Nil, &mut bounded),
        Kind::ResourceBound,
    );
    assert_eq!(heap.raw_len(&table, &mut work).unwrap(), 2);
    assert!(work.steps_used() > 0);
}
#[test]
fn unknown_imports_remain_unknown_and_independent_heaps_never_share_layout() {
    let (mut heap, mut work) = fixture();
    let graph = ProgramValueGraph {
        values: vec![ProgramValue::Table(ProgramTableId(1))],
        tables: vec![ProgramTable { entries: vec![] }],
    };
    let imported = heap.import(&graph, true).unwrap();
    set(&mut heap, &imported[0], 2.0, V::Boolean(true), &mut work);
    assert_kind(
        heap.raw_len(&imported[0], &mut work),
        Kind::UnsupportedCapability,
    );
    let first = heap.native_empty_table(&mut work).unwrap();
    let (mut other, mut other_work) = fixture();
    let second = other.native_empty_table(&mut other_work).unwrap();
    set(&mut heap, &first, 2.0, V::Boolean(false), &mut work);
    set(&mut other, &second, 3.0, V::Boolean(false), &mut other_work);
    assert_eq!(heap.raw_len(&first, &mut work).unwrap(), 2);
    assert_kind(
        other.raw_len(&second, &mut other_work),
        Kind::UnsupportedCapability,
    );
}

#[test]
fn unpack_requires_raw_length_to_match_every_admissible_jit_hint_boundary() {
    let (mut heap, mut work) = fixture();
    let table = heap.native_empty_table(&mut work).unwrap();
    set(&mut heap, &table, 2.0, V::Boolean(false), &mut work);
    assert_eq!(heap.hint_safe_len(&table, &mut work).unwrap(), 2);
    set(&mut heap, &table, 0.0, V::Boolean(false), &mut work);
    assert_eq!(heap.raw_len(&table, &mut work).unwrap(), 2);
    assert_kind(
        heap.hint_safe_len(&table, &mut work),
        Kind::UnsupportedCapability,
    );
    set(&mut heap, &table, 1.0, V::Boolean(true), &mut work);
    assert_eq!(heap.hint_safe_len(&table, &mut work).unwrap(), 2);
    for key in 3..=5 {
        set(
            &mut heap,
            &table,
            f64::from(key),
            V::Boolean(true),
            &mut work,
        );
    }
    set(&mut heap, &table, 2.0, V::Nil, &mut work);
    assert_kind(
        heap.hint_safe_len(&table, &mut work),
        Kind::UnsupportedCapability,
    );
    for key in [0.0, 1.0, 3.0, 4.0, 5.0] {
        set(&mut heap, &table, key, V::Nil, &mut work);
    }
    set(&mut heap, &table, 6.0, V::Boolean(false), &mut work);
    assert_eq!(heap.raw_len(&table, &mut work).unwrap(), 0);
    assert_kind(
        heap.hint_safe_len(&table, &mut work),
        Kind::UnsupportedCapability,
    );
}

#[test]
fn append_rejects_ambiguous_jit_hint_before_writing_or_changing_layout() {
    let (mut heap, mut work) = fixture();
    let table = heap.native_empty_table(&mut work).unwrap();
    set(&mut heap, &table, 0.0, V::Boolean(false), &mut work);
    set(&mut heap, &table, 2.0, V::Boolean(false), &mut work);
    assert_kind(
        heap.append(&table, V::Boolean(true), &mut work),
        Kind::UnsupportedCapability,
    );
    assert_eq!(layout(&heap, &table).unwrap().slots, 3);
    assert_eq!(layout(&heap, &table).unwrap().live, 2);
    for key in [1.0, 3.0] {
        assert!(matches!(
            heap.raw_get(&table, &V::Number(key)).unwrap(),
            V::Nil
        ));
    }
    set(&mut heap, &table, 1.0, V::Boolean(true), &mut work);
    heap.append(&table, V::Boolean(true), &mut work).unwrap();
    assert!(matches!(
        heap.raw_get(&table, &V::Number(3.0)).unwrap(),
        V::Boolean(true)
    ));
}
