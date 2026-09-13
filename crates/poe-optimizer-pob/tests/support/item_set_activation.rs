//! Activation population comparison; SyncLoadouts and whole Load are later stages.
#[allow(dead_code)]
#[path = "item_set_materialization.rs"]
pub mod materialization;
use materialization::graph;
#[path = "item_set_activation_source.rs"]
pub mod source;
use poe_optimizer_engine::source_program::{
    ProgramTableId as Id, ProgramValue as V, ProgramValueGraph as Graph,
};
use serde_json::{Value as Json, json};
use std::collections::BTreeMap;
fn id(v: &V) -> Id {
    if let V::Table(id) = v {
        *id
    } else {
        panic!("expected retained table")
    }
}
fn field(g: &Graph, t: Id, key: &str) -> V {
    g.tables[t.0 as usize - 1]
        .entries
        .iter()
        .find(|(k, _)| k == &V::Bytes(key.as_bytes().to_vec()))
        .map(|(_, v)| v.clone())
        .unwrap_or(V::Nil)
}
fn dense(g: &Graph, t: Id) -> Vec<V> {
    let entries = &g.tables[t.0 as usize - 1].entries;
    assert!(entries.len() <= 16384);
    let mut values = BTreeMap::new();
    for (k, v) in entries {
        let V::Number(n) = k else {
            panic!("nonnumeric choice index")
        };
        assert!(n.is_finite() && *n >= 1.0 && n.fract() == 0.0 && *n <= entries.len() as f64);
        assert!(values.insert(*n as usize, v.clone()).is_none());
    }
    assert_eq!(values.len(), entries.len());
    values.into_values().collect()
}
fn scalar(v: &V) -> Json {
    match v {
        V::Nil => json!(["nil"]),
        V::Boolean(v) => json!(["boolean", v]),
        V::Number(n) => json!(["number_bits", format!("{:016x}", n.to_bits())]),
        V::Bytes(s) => json!(["bytes", s]),
        _ => panic!("choice scalar"),
    }
}
/// Exact raw graph remains separate. This declared headless projection excludes
/// source pairs/UI positions and their container aliases, while preserving the
/// retained set/active/prior/child aliases and all declared selected fields.
pub fn headless(g: &Graph, source_runes: bool) -> Json {
    graph::canonical(g).unwrap();
    let root = id(&g.values[0]);
    let mut retained = materialization::common_graph(g.clone(), source_runes, false, false);
    let build_flag = field(g, root, "buildFlag");
    retained.tables[root.0 as usize - 1]
        .entries
        .push((V::Bytes(b"buildFlag".to_vec()), build_flag));
    let slots = id(&field(g, root, "slots"));
    let mut choices = BTreeMap::new();
    for (name, value) in &g.tables[slots.0 as usize - 1].entries {
        let V::Bytes(name) = name else {
            panic!("slot name")
        };
        let slot = id(value);
        let items = dense(g, id(&field(g, slot, "items")));
        let labels = dense(g, id(&field(g, slot, "list")));
        assert_eq!(items.len(), labels.len());
        let V::Number(selected) = field(g, slot, "selIndex") else {
            panic!("selected index")
        };
        assert!(
            selected.is_finite()
                && selected >= 1.0
                && selected.fract() == 0.0
                && selected <= items.len() as f64
        );
        let selected_item = field(g, slot, "selItemId");
        assert_eq!(
            items[selected as usize - 1],
            selected_item,
            "selected index must refer to live selected ID"
        );
        let mut counts = BTreeMap::<String, usize>::new();
        for (item, label) in items.iter().zip(&labels) {
            assert!(matches!(item,V::Number(n) if n.is_finite()));
            assert!(matches!(label, V::Bytes(_)));
            let key = json!([scalar(item), scalar(label)]).to_string();
            *counts.entry(key).or_default() += 1;
        }
        choices.insert(
            String::from_utf8(name.clone()).unwrap(),
            json!({"selected":scalar(&selected_item),"id_label_occurrences":counts}),
        );
    }
    json!({"retained_graph":graph::canonical(&retained).unwrap(),"slot_choice_occurrences":choices})
}
pub fn exact(g: &Graph) -> Json {
    graph::canonical(g).unwrap()
}

#[path = "item_set_activation_context.rs"]
mod context;
#[path = "item_set_activation_runner.rs"]
mod runner;
pub use runner::run;
/// Raw choice arrays and UI index; no sorting or index normalization.
pub fn raw_choices(g: &Graph) -> Json {
    let root = id(&g.values[0]);
    let slots = id(&field(g, root, "slots"));
    let mut out = BTreeMap::new();
    for (name, value) in &g.tables[slots.0 as usize - 1].entries {
        let V::Bytes(name) = name else {
            panic!("slot name")
        };
        let slot = id(value);
        out.insert(String::from_utf8(name.clone()).unwrap(),json!({"items":dense(g,id(&field(g,slot,"items"))).iter().map(scalar).collect::<Vec<_>>(),"labels":dense(g,id(&field(g,slot,"list"))).iter().map(scalar).collect::<Vec<_>>(),"selected_index":scalar(&field(g,slot,"selIndex"))}));
    }
    json!(out)
}
/// Control-only rune name counts; duplicates and selected name are retained,
/// while effect values and row/array alias identity are explicitly outside this key.
pub fn rune_names(g: &Graph) -> Json {
    let root = id(&g.values[0]);
    let runes = id(&field(g, root, "runeSlots"));
    let mut out = BTreeMap::new();
    for (key, value) in &g.tables[runes.0 as usize - 1].entries {
        let V::Bytes(key) = key else {
            panic!("rune slot name")
        };
        let row = id(value);
        let rows = dense(g, id(&field(g, row, "list")));
        let V::Number(index) = field(g, row, "selIndex") else {
            panic!("rune selected index")
        };
        assert!(
            index.is_finite() && index >= 1.0 && index.fract() == 0.0 && index <= rows.len() as f64
        );
        let mut counts = BTreeMap::<String, usize>::new();
        for row in &rows {
            let name = field(g, id(row), "name");
            assert!(matches!(name, V::Bytes(_)));
            *counts.entry(scalar(&name).to_string()).or_default() += 1;
        }
        out.insert(String::from_utf8(key.clone()).unwrap(),json!({"selected":scalar(&field(g,id(&rows[index as usize-1]),"name")),"name_occurrences":counts}));
    }
    json!(out)
}
