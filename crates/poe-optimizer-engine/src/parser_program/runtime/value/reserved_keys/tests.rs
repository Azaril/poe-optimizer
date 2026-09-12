use super::super::{TableBehavior, tests::heap};
use super::*;
use crate::parser_program::runtime::{ProgramLimits, ProgramRuntimeErrorKind as Kind};
use std::collections::BTreeMap;

fn text(value: &[u8]) -> V {
    V::Bytes(Arc::from(value))
}
fn work() -> MatchBudget {
    MatchBudget::new(ProgramLimits::default().pattern)
}
fn keys() -> Arc<CompiledReservedKeys> {
    Arc::new(CompiledReservedKeys::new(&[
        b"z".to_vec(),
        vec![0xff, 0],
        b"alpha".to_vec(),
    ]))
}
fn next(heap: &mut Heap, table: &V, control: &V, work: &mut MatchBudget) -> Result<Option<(V, V)>> {
    heap.definition_next(table, control, &BTreeMap::new(), work)
}
fn assert_kind<T>(result: Result<T>, kind: Kind) {
    assert_eq!(result.err().expect("expected failure").kind, kind);
}
fn certificate(heap: &Heap, table: &V) -> bool {
    heap.reserved_keys.contains_key(&table_ref(table).unwrap())
}

#[test]
fn reserved_order_is_independent_of_sorted_storage_and_reads_live_alias_values() {
    let mut heap = heap();
    let mut work = work();
    let seed = keys();
    let table = heap.reserved_table(seed.clone(), &mut work).unwrap();
    assert!(
        next(&mut heap, &table, &V::Nil, &mut work)
            .unwrap()
            .is_none()
    );
    assert_eq!(heap.raw_len(&table, &mut work).unwrap(), 0);
    assert!(matches!(heap.raw_get(&table, &text(b"z")).unwrap(), V::Nil));
    let child = heap.new_table().unwrap();
    heap.raw_set(&table, text(b"alpha"), child.clone(), &mut work)
        .unwrap();
    heap.raw_set(&table, text(&[0xff, 0]), V::Boolean(false), &mut work)
        .unwrap();
    heap.raw_set(&table, text(b"z"), child.clone(), &mut work)
        .unwrap();
    let (first, value) = next(&mut heap, &table, &V::Nil, &mut work)
        .unwrap()
        .unwrap();
    assert!(first.lua_equal(&text(b"z")) && value.lua_equal(&child));
    let (second, value) = next(&mut heap, &table, &first, &mut work).unwrap().unwrap();
    assert!(second.lua_equal(&text(&[0xff, 0])) && value.lua_equal(&V::Boolean(false)));
    let (third, value) = next(&mut heap, &table, &second, &mut work)
        .unwrap()
        .unwrap();
    assert!(third.lua_equal(&text(b"alpha")) && value.lua_equal(&child));
    assert!(
        next(&mut heap, &table, &third, &mut work)
            .unwrap()
            .is_none()
    );
    heap.raw_set(&table, text(b"z"), V::Number(-0.0), &mut work)
        .unwrap();
    let (_, value) = next(&mut heap, &table, &V::Nil, &mut work)
        .unwrap()
        .unwrap();
    assert!(matches!(value,V::Number(n) if n.to_bits()==(-0.0f64).to_bits()));
    assert!(Arc::ptr_eq(
        &seed,
        &heap.reserved_keys[&table_ref(&table).unwrap()]
    ));
}

#[test]
fn reserved_deletions_reinsertions_and_deleted_controls_keep_original_positions() {
    let mut heap = heap();
    let mut work = work();
    let seed = keys();
    let table = heap.reserved_table(seed.clone(), &mut work).unwrap();
    for position in 0..seed.len() {
        heap.raw_set(
            &table,
            text(seed.key(position)),
            V::Number(position as f64),
            &mut work,
        )
        .unwrap();
    }
    heap.raw_set(&table, text(&[0xff, 0]), V::Nil, &mut work)
        .unwrap();
    let (key, _) = next(&mut heap, &table, &text(&[0xff, 0]), &mut work)
        .unwrap()
        .unwrap();
    assert!(key.lua_equal(&text(b"alpha")));
    heap.raw_set(&table, text(b"z"), V::Nil, &mut work).unwrap();
    let (key, _) = next(&mut heap, &table, &text(b"z"), &mut work)
        .unwrap()
        .unwrap();
    assert!(key.lua_equal(&text(b"alpha")));
    heap.raw_set(&table, text(b"alpha"), V::Nil, &mut work)
        .unwrap();
    assert!(
        next(&mut heap, &table, &V::Nil, &mut work)
            .unwrap()
            .is_none()
    );
    for position in 0..seed.len() {
        assert!(
            next(&mut heap, &table, &text(seed.key(position)), &mut work)
                .unwrap()
                .is_none()
        );
    }
    heap.raw_set(&table, text(&[0xff, 0]), V::Boolean(false), &mut work)
        .unwrap();
    let (key, value) = next(&mut heap, &table, &V::Nil, &mut work)
        .unwrap()
        .unwrap();
    assert!(key.lua_equal(&text(&[0xff, 0])) && value.lua_equal(&V::Boolean(false)));
    assert!(certificate(&heap, &table));
    for control in [
        text(b"missing"),
        V::Number(0.0),
        V::Number(f64::NAN),
        V::Boolean(false),
        table.clone(),
    ] {
        assert_kind(
            next(&mut heap, &table, &control, &mut work),
            Kind::UnsupportedCapability,
        );
    }
}

#[test]
fn unreserved_writes_invalidate_even_nil_and_equal_final_contents_do_not_restore_proof() {
    for key in [
        text(b"new"),
        V::Number(0.0),
        V::Number(1.0),
        V::Boolean(false),
    ] {
        for value in [V::Nil, V::Boolean(false)] {
            let mut heap = heap();
            let mut work = work();
            let table = heap.reserved_table(keys(), &mut work).unwrap();
            heap.raw_set(&table, text(b"z"), V::Number(1.0), &mut work)
                .unwrap();
            let before = heap.freeze(std::slice::from_ref(&table)).unwrap();
            heap.raw_set(&table, key.clone(), value, &mut work).unwrap();
            assert!(!certificate(&heap, &table));
            heap.raw_set(&table, key.clone(), V::Nil, &mut work)
                .unwrap();
            assert_eq!(heap.freeze(std::slice::from_ref(&table)).unwrap(), before);
            assert_kind(
                next(&mut heap, &table, &V::Nil, &mut work),
                Kind::UnsupportedCapability,
            );
        }
    }
}

#[test]
fn metadata_and_write_failures_retain_admitted_state_and_cumulative_work() {
    for (max_values, max_tables) in [(1, 10), (2, 0)] {
        let mut heap = heap();
        let mut work = work();
        heap.budget.limits.max_values = max_values;
        heap.budget.limits.max_tables = max_tables;
        assert_kind(heap.reserved_table(keys(), &mut work), Kind::ResourceBound);
        assert!(heap.tables.is_empty() && heap.reserved_keys.is_empty());
        assert_eq!(work.steps_used(), 4);
    }
    let mut heap = heap();
    let mut normal = work();
    let table = heap.reserved_table(keys(), &mut normal).unwrap();
    heap.raw_set(&table, text(b"z"), V::Boolean(false), &mut normal)
        .unwrap();
    let before = heap.freeze(std::slice::from_ref(&table)).unwrap();
    for key in [V::Nil, V::Number(f64::NAN)] {
        assert_kind(
            heap.raw_set(&table, key, V::Boolean(true), &mut normal),
            Kind::Source,
        );
        assert!(certificate(&heap, &table));
    }
    heap.budget.limits.max_values = heap.stats().values;
    let steps = normal.steps_used();
    assert_kind(
        heap.raw_set(&table, text(b"new"), V::Boolean(true), &mut normal),
        Kind::ResourceBound,
    );
    assert!(normal.steps_used() > steps && certificate(&heap, &table));
    assert!(matches!(
        heap.raw_get(&table, &text(b"new")).unwrap(),
        V::Nil
    ));
    let mut limits = ProgramLimits::default().pattern;
    limits.max_steps = 1;
    let mut limited = MatchBudget::new(limits);
    assert_kind(
        heap.raw_set(&table, text(b"z"), V::Nil, &mut limited),
        Kind::ResourceBound,
    );
    assert!(certificate(&heap, &table));
    assert!(matches!(
        heap.raw_get(&table, &text(b"z")).unwrap(),
        V::Boolean(false)
    ));
    heap.budget.limits.max_values = ProgramLimits::default().max_values;
    assert_eq!(heap.freeze(std::slice::from_ref(&table)).unwrap(), before);
}

#[test]
fn skipped_nil_reservations_consume_shared_work_without_mutating_the_certificate() {
    let mut heap = heap();
    let mut normal = work();
    let seed = Arc::new(CompiledReservedKeys::new(
        &(0..20)
            .map(|n| format!("key-{n}").into_bytes())
            .collect::<Vec<_>>(),
    ));
    let table = heap.reserved_table(seed, &mut normal).unwrap();
    heap.raw_set(&table, text(b"key-19"), V::Boolean(false), &mut normal)
        .unwrap();
    let mut limits = ProgramLimits::default().pattern;
    limits.max_steps = 8;
    let mut limited = MatchBudget::new(limits);
    assert_kind(
        next(&mut heap, &table, &V::Nil, &mut limited),
        Kind::ResourceBound,
    );
    assert!(limited.steps_used() > 8 && certificate(&heap, &table));
    let (key, value) = next(&mut heap, &table, &V::Nil, &mut normal)
        .unwrap()
        .unwrap();
    assert!(key.lua_equal(&text(b"key-19")) && value.lua_equal(&V::Boolean(false)));
}

#[test]
fn shared_seed_lifetime_private_values_and_raw_snapshot_boundary_are_distinct() {
    let seed = keys();
    let weak = Arc::downgrade(&seed);
    let mut first = heap();
    let mut second = heap();
    let mut work = work();
    let left = first.reserved_table(seed.clone(), &mut work).unwrap();
    let right = second.reserved_table(seed.clone(), &mut work).unwrap();
    first
        .raw_set(&left, text(b"z"), left.clone(), &mut work)
        .unwrap();
    assert!(matches!(
        second.raw_get(&right, &text(b"z")).unwrap(),
        V::Nil
    ));
    let raw = first.freeze(std::slice::from_ref(&left)).unwrap();
    let imported = second.import(&raw, true).unwrap();
    assert!(!certificate(&second, &imported[0]));
    assert_eq!(second.freeze(&imported).unwrap(), raw);
    assert_kind(
        next(&mut second, &imported[0], &V::Nil, &mut work),
        Kind::UnsupportedCapability,
    );
    drop(seed);
    drop(first);
    assert!(weak.upgrade().is_some());
    drop(second);
    assert!(weak.upgrade().is_none());
}

#[test]
fn behavior_install_and_append_invalidate_only_after_their_successful_commit() {
    let mut heap = heap();
    let mut work = work();
    let table = heap.reserved_table(keys(), &mut work).unwrap();
    let values = heap.stats().values;
    heap.budget.limits.max_values = values;
    assert_kind(
        heap.set_behavior(&table, TableBehavior::ParentProxy),
        Kind::ResourceBound,
    );
    assert!(certificate(&heap, &table) && heap.behavior(&table).is_none());
    heap.budget.limits.max_values = values + 2;
    heap.set_behavior(&table, TableBehavior::ParentProxy)
        .unwrap();
    assert!(!certificate(&heap, &table));
    heap.budget.limits.max_values = ProgramLimits::default().max_values;
    let appended = heap.reserved_table(keys(), &mut work).unwrap();
    heap.append(&appended, V::Boolean(false), &mut work)
        .unwrap();
    assert!(!certificate(&heap, &appended));
    assert!(matches!(
        heap.raw_get(&appended, &V::Number(1.0)).unwrap(),
        V::Boolean(false)
    ));
}
