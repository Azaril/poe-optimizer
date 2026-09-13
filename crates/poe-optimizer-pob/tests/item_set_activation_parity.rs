//! Complete original activation population at a declared pre-Sync boundary.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/item_set_activation.rs"]
mod original;
use poe_optimizer_engine::source_program::{
    ProgramTable, ProgramTableId as Id, ProgramValue as V, ProgramValueGraph as Graph,
};
fn fixture() -> Graph {
    let text = |s: &str| V::Bytes(s.as_bytes().to_vec());
    let table = |pairs: Vec<(&str, V)>| ProgramTable {
        entries: pairs.into_iter().map(|(k, v)| (text(k), v)).collect(),
    };
    Graph {
        values: vec![V::Table(Id(1))],
        tables: vec![
            table(vec![
                ("itemSets", V::Table(Id(2))),
                ("itemSetOrderList", V::Table(Id(4))),
                ("activeItemSet", V::Table(Id(3))),
                ("previousActiveItemSet", V::Table(Id(3))),
                ("activeItemSetId", V::Number(1.0)),
                ("slots", V::Table(Id(5))),
                ("runeSlots", V::Table(Id(8))),
                ("trade", V::Table(Id(13))),
                ("showStatDifferences", V::Boolean(true)),
                ("buildFlag", V::Boolean(true)),
            ]),
            ProgramTable {
                entries: vec![(V::Number(1.0), V::Table(Id(3)))],
            },
            table(vec![("id", V::Number(1.0)), ("Ring", V::Table(Id(6)))]),
            ProgramTable {
                entries: vec![(V::Number(1.0), V::Number(1.0))],
            },
            table(vec![("Ring", V::Table(Id(6)))]),
            table(vec![
                ("slotName", text("Ring")),
                ("selItemId", V::Number(7.0)),
                ("selIndex", V::Number(2.0)),
                ("items", V::Table(Id(10))),
                ("list", V::Table(Id(11))),
                ("jewelSocketList", V::Table(Id(7))),
            ]),
            ProgramTable::default(),
            table(vec![("Rune", V::Table(Id(9)))]),
            table(vec![("selected_name", text("None"))]),
            ProgramTable {
                entries: vec![
                    (V::Number(1.0), V::Number(0.0)),
                    (V::Number(2.0), V::Number(7.0)),
                ],
            },
            ProgramTable {
                entries: vec![
                    (V::Number(1.0), text("None")),
                    (V::Number(2.0), text("Ring")),
                ],
            },
            ProgramTable::default(),
            ProgramTable::default(),
        ],
    }
}
#[test]
fn declared_choice_projection_keeps_duplicates_selection_and_raw_order_separate() {
    let a = fixture();
    let baseline = original::headless(&a, false);
    let mut b = a.clone();
    b.tables[9].entries.swap(0, 1);
    b.tables[10].entries.swap(0, 1);
    // Swap numeric positions, not merely the unordered graph's physical row vector.
    b.tables[9].entries[0].0 = V::Number(1.0);
    b.tables[9].entries[1].0 = V::Number(2.0);
    b.tables[10].entries[0].0 = V::Number(1.0);
    b.tables[10].entries[1].0 = V::Number(2.0);
    b.tables[5]
        .entries
        .iter_mut()
        .find(|(k, _)| k == &V::Bytes(b"selIndex".to_vec()))
        .unwrap()
        .1 = V::Number(1.0);
    assert_eq!(baseline, original::headless(&b, false));
    assert_ne!(original::exact(&a), original::exact(&b));
    b.tables[9].entries.push((V::Number(3.0), V::Number(7.0)));
    b.tables[10]
        .entries
        .push((V::Number(3.0), V::Bytes(b"Ring".to_vec())));
    assert_ne!(baseline, original::headless(&b, false));
}
#[test]
fn activation_projection_keeps_prior_alias_notes_and_build_flag() {
    let a = fixture();
    let baseline = original::headless(&a, false);
    let mut b = a.clone();
    b.tables.push(b.tables[2].clone());
    b.tables[0]
        .entries
        .iter_mut()
        .find(|(k, _)| k == &V::Bytes(b"previousActiveItemSet".to_vec()))
        .unwrap()
        .1 = V::Table(Id(14));
    assert_ne!(baseline, original::headless(&b, false));
    let mut b = a.clone();
    b.tables[5]
        .entries
        .push((V::Bytes(b"note".to_vec()), V::Bytes(vec![])));
    assert_ne!(baseline, original::headless(&b, false));
    let mut b = a;
    b.tables[0]
        .entries
        .iter_mut()
        .find(|(k, _)| k == &V::Bytes(b"buildFlag".to_vec()))
        .unwrap()
        .1 = V::Boolean(false);
    assert_ne!(baseline, original::headless(&b, false));
}
#[test]
fn unavailable_or_inconsistent_choices_cannot_pass_projection() {
    let mut g = fixture();
    g.tables[9].entries[1].0 = V::Number(3.0);
    assert!(std::panic::catch_unwind(|| original::headless(&g, false)).is_err());
    let mut g = fixture();
    g.tables[5]
        .entries
        .iter_mut()
        .find(|(k, _)| k == &V::Bytes(b"selIndex".to_vec()))
        .unwrap()
        .1 = V::Number(1.0);
    assert!(std::panic::catch_unwind(|| original::headless(&g, false)).is_err());
}

#[test]
fn all_five_original_activation_population_and_component_histories() {
    original::run();
}
