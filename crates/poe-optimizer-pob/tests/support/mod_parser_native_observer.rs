//! Canonical native output and injected callback descriptors, separate from the
//! original-Lua capture implementation. Arc/table/closure identity is retained.
use super::public_source::{Atom, Callback, Graph};
use poe_optimizer_data::modifier_parser::{
    ModifierParserCatalog, ParserCallbackId, ParserCallbackKind, ParserNonFinite, ParserTableId,
    ParserValue,
};
use poe_optimizer_engine::modifier_parser::{ModifierTable, ModifierValue, ParseOutcome};
use std::collections::BTreeMap;
struct Capture<'a> {
    graph: Graph,
    seen: BTreeMap<usize, usize>,
    data_tables: BTreeMap<ParserTableId, usize>,
    callbacks: BTreeMap<ParserCallbackId, usize>,
    catalog: &'a ModifierParserCatalog,
    values: usize,
}
impl Capture<'_> {
    fn value(&mut self, value: &ModifierValue) -> Result<Atom, String> {
        self.values += 1;
        if self.values > 1_000_000 {
            return Err("native observer graph bound".into());
        }
        Ok(match value {
            ModifierValue::Nil => Atom::Nil,
            ModifierValue::Boolean(value) => Atom::Boolean(*value),
            ModifierValue::Number(value) => Atom::Number(value.to_bits()),
            ModifierValue::Bytes(value) => Atom::Bytes(value.clone()),
            ModifierValue::Table(value) => self.table(value)?,
            ModifierValue::Callback(id) => self.callback(*id)?,
        })
    }
    fn table(&mut self, table: &ModifierTable) -> Result<Atom, String> {
        let pointer = std::ptr::from_ref(table) as usize;
        if let Some(id) = self.seen.get(&pointer) {
            return Ok(Atom::Table(*id));
        }
        let id = self.graph.tables.len();
        self.seen.insert(pointer, id);
        self.graph.tables.push(vec![]);
        let mut entries = table
            .indexed
            .iter()
            .map(|(k, v)| (Atom::Number((*k as f64).to_bits()), v))
            .chain(
                table
                    .fields
                    .iter()
                    .map(|(k, v)| (Atom::Bytes(k.as_bytes().to_vec()), v)),
            )
            .collect::<Vec<_>>();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        let mut output = vec![];
        for (key, value) in entries {
            output.push((key, self.value(value)?));
        }
        self.graph.tables[id] = output;
        Ok(Atom::Table(id))
    }
    fn data_value(&mut self, value: &ParserValue) -> Result<Atom, String> {
        self.values += 1;
        if self.values > 1_000_000 {
            return Err("native observer graph bound".into());
        }
        Ok(match value {
            ParserValue::Nil => Atom::Nil,
            ParserValue::Boolean(value) => Atom::Boolean(*value),
            ParserValue::Number(value) => Atom::Number(value.to_bits()),
            ParserValue::Text(value) => Atom::Bytes(value.as_bytes().to_vec()),
            ParserValue::NonFinite(value) => Atom::Number(match value {
                ParserNonFinite::PositiveInfinity => f64::INFINITY.to_bits(),
                ParserNonFinite::NegativeInfinity => f64::NEG_INFINITY.to_bits(),
                ParserNonFinite::Nan => 0xfff8_0000_0000_0000,
            }),
            ParserValue::Callback(id) => self.callback(*id)?,
            ParserValue::Table(id) => {
                if let Some(id) = self.data_tables.get(id) {
                    return Ok(Atom::Table(*id));
                }
                let out = self.graph.tables.len();
                self.data_tables.insert(*id, out);
                self.graph.tables.push(vec![]);
                let table = self
                    .catalog
                    .table(*id)
                    .ok_or("unknown catalog table")?
                    .clone();
                let mut entries = table
                    .indexed
                    .into_iter()
                    .map(|(k, v)| (Atom::Number((k as f64).to_bits()), v))
                    .chain(
                        table
                            .fields
                            .into_iter()
                            .map(|(k, v)| (Atom::Bytes(k.into_bytes()), v)),
                    )
                    .collect::<Vec<_>>();
                entries.sort_by(|a, b| a.0.cmp(&b.0));
                let mut result = vec![];
                for (k, v) in entries {
                    result.push((k, self.data_value(&v)?));
                }
                self.graph.tables[out] = result;
                Atom::Table(out)
            }
        })
    }
    fn callback(&mut self, id: ParserCallbackId) -> Result<Atom, String> {
        if let Some(id) = self.callbacks.get(&id) {
            return Ok(Atom::Function(*id));
        }
        let out = self.graph.callbacks.len();
        self.callbacks.insert(id, out);
        let definition = self
            .catalog
            .callback(id)
            .ok_or("unknown catalog callback")?
            .clone();
        let mut callback = match definition.kind {
            ParserCallbackKind::Lua { source } => Callback {
                source: format!("@{}", source.path).into_bytes(),
                first_line: i64::from(source.line),
                last_line: i64::from(source.end_line),
                kind: b"Lua".to_vec(),
                upvalues: vec![],
                builtin: None,
            },
            ParserCallbackKind::Builtin { symbol } => Callback {
                source: b"=[C]".to_vec(),
                first_line: -1,
                last_line: -1,
                kind: b"C".to_vec(),
                upvalues: vec![],
                builtin: Some(symbol),
            },
        };
        self.graph.callbacks.push(callback.clone());
        for upvalue in definition.upvalues {
            callback
                .upvalues
                .push((upvalue.name.into_bytes(), self.data_value(&upvalue.value)?));
        }
        self.graph.callbacks[out] = callback;
        Ok(Atom::Function(out))
    }
}
pub fn capture(outcome: &ParseOutcome, catalog: &ModifierParserCatalog) -> Result<Graph, String> {
    let mut capture = Capture {
        graph: Graph::default(),
        seen: BTreeMap::new(),
        data_tables: BTreeMap::new(),
        callbacks: BTreeMap::new(),
        catalog,
        values: 0,
    };
    // Public unpack(copyTable(cache)) has one result for a complete modifier
    // table, two for a remainder (including a nil modifier table).
    let first = match &outcome.modifiers {
        Some(value) => capture.table(value)?,
        None => Atom::Nil,
    };
    if outcome.modifiers.is_some() || outcome.extra.is_some() {
        capture.graph.roots.push(first);
    }
    if let Some(extra) = &outcome.extra {
        capture.graph.roots.push(Atom::Bytes(extra.clone()));
    }
    Ok(capture.graph)
}
