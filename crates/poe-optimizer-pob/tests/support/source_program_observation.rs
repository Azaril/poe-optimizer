//! Test-only raw graph observation; class/proxy behavior is deliberately excluded.
use mlua::Value;
use poe_optimizer_engine::source_program::*;
use serde_json::{Value as Json, json};
use std::collections::BTreeMap;

/// Test observation walks raw keys only. Plain receiver arguments isolate this
/// complete method; Common.new parent-proxy construction remains a separate gate.
/// The cases exercise finite scalars and string/integer keys, retaining table aliases.
pub fn capture(values: &[Value]) -> ProgramValueGraph {
    struct Capture {
        graph: ProgramValueGraph,
        ids: BTreeMap<usize, ProgramTableId>,
    }
    impl Capture {
        fn value(&mut self, value: &Value) -> ProgramValue {
            match value {
                Value::Nil => ProgramValue::Nil,
                Value::Boolean(v) => ProgramValue::Boolean(*v),
                Value::Integer(v) => ProgramValue::Number(*v as f64),
                Value::Number(v) => ProgramValue::Number(*v),
                Value::String(v) => ProgramValue::Bytes(v.as_bytes().to_vec()),
                Value::Table(table) => {
                    let pointer = table.to_pointer() as usize;
                    if let Some(id) = self.ids.get(&pointer) {
                        return ProgramValue::Table(*id);
                    }
                    let id = ProgramTableId(self.graph.tables.len() as u32 + 1);
                    self.ids.insert(pointer, id);
                    self.graph.tables.push(ProgramTable::default());
                    let mut raw = table
                        .clone()
                        .pairs::<Value, Value>()
                        .map(Result::unwrap)
                        .collect::<Vec<_>>();
                    raw.sort_by_key(|(key, _)| match key {
                        Value::Integer(n) => format!("0:{n:020}"),
                        Value::Number(n) => format!("0:{n:020}"),
                        Value::String(s) => format!("1:{}", s.to_str().unwrap()),
                        _ => panic!("unrepresented test observation key"),
                    });
                    let entries = raw
                        .into_iter()
                        .map(|(k, v)| (self.value(&k), self.value(&v)))
                        .collect();
                    self.graph.tables[id.0 as usize - 1].entries = entries;
                    ProgramValue::Table(id)
                }
                _ => panic!("unrepresented test observation value"),
            }
        }
    }
    let mut capture = Capture {
        graph: ProgramValueGraph::default(),
        ids: BTreeMap::new(),
    };
    capture.graph.values = values.iter().map(|v| capture.value(v)).collect();
    capture.graph
}

/// Normalize transport table numbering only; retain scalar types, order, aliases and cycles.
pub fn canonical(graph: &ProgramValueGraph) -> Json {
    fn scalar(value: &ProgramValue) -> String {
        match value {
            ProgramValue::Bytes(v) => format!("b:{v:?}"),
            ProgramValue::Number(v) => format!("n:{:016x}", v.to_bits()),
            _ => panic!("unrepresented test key"),
        }
    }
    fn visit(
        value: &ProgramValue,
        graph: &ProgramValueGraph,
        ids: &mut BTreeMap<ProgramTableId, usize>,
        tables: &mut Vec<Json>,
    ) -> Json {
        match value {
            ProgramValue::Nil => json!({"nil":true}),
            ProgramValue::Boolean(v) => json!({"boolean":v}),
            ProgramValue::Number(v) => json!({"number_bits":format!("{:016x}",v.to_bits())}),
            ProgramValue::Bytes(v) => json!({"bytes":v}),
            ProgramValue::Callback(_) => panic!("callback escaped observation"),
            ProgramValue::Table(id) => {
                if let Some(index) = ids.get(id) {
                    return json!({"table":index});
                }
                let index = tables.len();
                ids.insert(*id, index);
                tables.push(Json::Null);
                let mut rows = graph.tables[id.0 as usize - 1]
                    .entries
                    .iter()
                    .collect::<Vec<_>>();
                rows.sort_by_key(|(key, _)| scalar(key));
                tables[index] = Json::Array(
                    rows.into_iter()
                        .map(|(k, v)| {
                            json!([visit(k, graph, ids, tables), visit(v, graph, ids, tables)])
                        })
                        .collect(),
                );
                json!({"table":index})
            }
        }
    }
    let mut ids = BTreeMap::new();
    let mut tables = Vec::new();
    let values = graph
        .values
        .iter()
        .map(|v| visit(v, graph, &mut ids, &mut tables))
        .collect::<Vec<_>>();
    json!({"values":values,"tables":tables})
}
