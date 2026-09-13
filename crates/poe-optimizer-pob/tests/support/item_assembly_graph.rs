//! Bounded streaming observation and canonical-order validation, adapted from
//! the existing full parser corpus helper. No source Lua row handles are batched.
use mlua::Value;
use poe_optimizer_engine::source_program::*;
use serde_json::Value as Json;
use std::collections::{BTreeMap, BTreeSet};
#[allow(dead_code)]
#[path = "source_program_observation.rs"]
mod plain;
const MAX_DEPTH: usize = 96;
const MAX_ROWS: usize = 32768;
pub const MAX_TABLES: usize = 65536;
const MAX_VALUES: usize = 1000000;
const MAX_BYTES: usize = 16 * 1024 * 1024;
pub fn capture(values: &[Value]) -> Result<ProgramValueGraph, &'static str> {
    struct Capture {
        graph: ProgramValueGraph,
        seen: BTreeMap<usize, ProgramTableId>,
        nodes: usize,
        bytes: usize,
    }
    impl Capture {
        fn value(&mut self, v: &Value, depth: usize) -> Option<ProgramValue> {
            if self.nodes == 0 || depth > MAX_DEPTH {
                return None;
            }
            self.nodes -= 1;
            Some(match v {
                Value::Nil => ProgramValue::Nil,
                Value::Boolean(v) => ProgramValue::Boolean(*v),
                Value::Integer(v) => ProgramValue::Number(*v as f64),
                Value::Number(v) => ProgramValue::Number(*v),
                Value::String(v) => {
                    self.bytes = self.bytes.checked_sub(v.as_bytes().len())?;
                    ProgramValue::Bytes(v.as_bytes().to_vec())
                }
                Value::Table(t) => {
                    if t.metatable().is_some() {
                        return None;
                    }
                    let pointer = t.to_pointer() as usize;
                    if let Some(id) = self.seen.get(&pointer) {
                        return Some(ProgramValue::Table(*id));
                    }
                    if self.graph.tables.len() >= MAX_TABLES {
                        return None;
                    }
                    let id = ProgramTableId(self.graph.tables.len() as u32 + 1);
                    self.seen.insert(pointer, id);
                    self.graph.tables.push(ProgramTable::default());
                    // Retain only native values; never collect Lua key/value handles.
                    let mut entries = Vec::new();
                    for (index, row) in t.clone().pairs::<Value, Value>().enumerate() {
                        if index >= MAX_ROWS {
                            return None;
                        }
                        let (key, value) = row.ok()?;
                        if !matches!(key, Value::Integer(_) | Value::Number(_) | Value::String(_)) {
                            return None;
                        }
                        entries
                            .push((self.value(&key, depth + 1)?, self.value(&value, depth + 1)?));
                    }
                    self.graph.tables[id.0 as usize - 1].entries = entries;
                    ProgramValue::Table(id)
                }
                _ => return None,
            })
        }
    }
    let mut c = Capture {
        graph: ProgramValueGraph::default(),
        seen: BTreeMap::new(),
        nodes: MAX_VALUES,
        bytes: MAX_BYTES,
    };
    if values.len() > MAX_VALUES {
        return Err("comparison root bound");
    }
    c.graph.values = values
        .iter()
        .map(|v| c.value(v, 0))
        .collect::<Option<Vec<_>>>()
        .ok_or("comparison source graph representation/resource bound")?;
    Ok(c.graph)
}
fn bounded(graph: &ProgramValueGraph) -> bool {
    fn visit(
        v: &ProgramValue,
        g: &ProgramValueGraph,
        seen: &mut BTreeSet<ProgramTableId>,
        nodes: &mut usize,
        bytes: &mut usize,
        depth: usize,
    ) -> Option<()> {
        if *nodes == 0 || depth > MAX_DEPTH {
            return None;
        }
        *nodes -= 1;
        match v {
            ProgramValue::Nil | ProgramValue::Boolean(_) | ProgramValue::Number(_) => {}
            ProgramValue::Bytes(v) => {
                *bytes = bytes.checked_sub(v.len())?;
            }
            ProgramValue::Table(id) => {
                if !seen.insert(*id) {
                    return Some(());
                }
                let t = g.tables.get(id.0.checked_sub(1)? as usize)?;
                if t.entries.len() > MAX_ROWS {
                    return None;
                }
                // Check key sizes before making sorting strings. Walk exactly the
                // canonical visitor's key order: alias order affects DFS depth.
                let mut key_bytes = 0usize;
                for (key, _) in &t.entries {
                    match key {
                        ProgramValue::Number(_) => {}
                        ProgramValue::Bytes(v) => {
                            key_bytes = key_bytes.checked_add(v.len())?;
                        }
                        _ => return None,
                    }
                }
                if key_bytes > *bytes {
                    return None;
                }
                let mut rows = t.entries.iter().collect::<Vec<_>>();
                rows.sort_by_cached_key(|(key, _)| match key {
                    ProgramValue::Bytes(v) => format!("b:{v:?}"),
                    ProgramValue::Number(v) => format!("n:{:016x}", v.to_bits()),
                    _ => unreachable!(),
                });
                for (key, value) in rows {
                    visit(key, g, seen, nodes, bytes, depth + 1)?;
                    visit(value, g, seen, nodes, bytes, depth + 1)?;
                }
            }
            _ => return None,
        }
        Some(())
    }
    if graph.tables.len() > MAX_TABLES || graph.values.len() > MAX_VALUES {
        return false;
    }
    let mut seen = BTreeSet::new();
    let mut nodes = MAX_VALUES;
    let mut bytes = MAX_BYTES;
    graph
        .values
        .iter()
        .all(|v| visit(v, graph, &mut seen, &mut nodes, &mut bytes, 0).is_some())
}
pub fn canonical(graph: &ProgramValueGraph) -> Result<Json, &'static str> {
    if !bounded(graph) {
        return Err("comparison canonical-order graph resource/representation bound");
    }
    Ok(plain::canonical(graph))
}
#[test]
fn wide_source_graph_is_streamed_with_aliases() {
    let lua = mlua::Lua::new();
    let root = lua.create_table().unwrap();
    for i in 1..=9001 {
        let child = lua.create_table().unwrap();
        child.raw_set("value", i).unwrap();
        root.raw_set(i, child).unwrap();
    }
    let graph = capture(&[Value::Table(root.clone()), Value::Table(root)]).unwrap();
    assert_eq!(graph.tables.len(), 9002);
    assert_eq!(graph.values[0], graph.values[1]);
    canonical(&graph).unwrap();
}
#[test]
fn canonical_alias_order_depth_and_wide_rows_fail_explicitly() {
    let mut graph = ProgramValueGraph {
        values: vec![ProgramValue::Table(ProgramTableId(1))],
        tables: vec![ProgramTable::default(); 100],
    };
    graph.tables[0].entries = (2..=100)
        .rev()
        .map(|id| {
            (
                ProgramValue::Number(id as f64),
                ProgramValue::Table(ProgramTableId(id)),
            )
        })
        .collect();
    for id in 2..100 {
        graph.tables[id as usize - 1].entries.push((
            ProgramValue::Number(1.0),
            ProgramValue::Table(ProgramTableId(id + 1)),
        ));
    }
    assert!(canonical(&graph).is_err());
    graph.tables[0].entries = (0..=MAX_ROWS)
        .map(|id| (ProgramValue::Number(id as f64), ProgramValue::Boolean(true)))
        .collect();
    assert!(canonical(&graph).is_err());
    let lua = mlua::Lua::new();
    let root = lua.create_table().unwrap();
    for i in 0..=MAX_ROWS {
        root.raw_set(i, true).unwrap();
    }
    assert!(capture(&[Value::Table(root)]).is_err());
}
