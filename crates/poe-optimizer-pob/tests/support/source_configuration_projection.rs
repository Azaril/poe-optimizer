//! Test-only producer of live session input. Full raw inventories distinguish
//! omitted values from absence; only __index and __call behavior may be unavailable.
use mlua::{Table, Value};
use poe_optimizer_data::source_program::{
    SourceTableCallFallback, SourceTableCoverage, SourceTableIndexFallback, SourceTableInventory,
    SourceTableKey,
};
use poe_optimizer_engine::source_program::*;
use std::collections::{BTreeMap, BTreeSet};

pub fn capture(config: &Table) -> (ProgramValueGraph, ProgramTableCoverage) {
    let fields = |names: &[&str]| names.iter().map(|name| (*name).to_owned()).collect();
    let build: Table = config.raw_get("build").unwrap();
    let sets: Table = config.raw_get("configSets").unwrap();
    let mut selected = BTreeMap::from([
        (
            config.to_pointer() as usize,
            fields(&[
                "configSets",
                "activeConfigSetId",
                "input",
                "placeholder",
                "build",
                "enemyLevel",
            ]),
        ),
        (
            build.to_pointer() as usize,
            fields(&["characterLevel", "configTab"]),
        ),
    ]);
    for entry in sets.pairs::<Value, Table>() {
        let (_, set) = entry.unwrap();
        selected.insert(set.to_pointer() as usize, fields(&["input", "placeholder"]));
    }
    struct Capture {
        selected: BTreeMap<usize, BTreeSet<String>>,
        ids: BTreeMap<usize, ProgramTableId>,
        graph: ProgramValueGraph,
        coverage: ProgramTableCoverage,
        values: usize,
        bytes: usize,
    }
    impl Capture {
        fn charge(&mut self, value: &Value) {
            self.values += 1;
            assert!(self.values <= 100_000, "live projection value bound");
            if let Value::String(value) = value {
                self.bytes += value.as_bytes().len();
                assert!(self.bytes <= 8 * 1024 * 1024, "live projection byte bound");
            }
        }
        fn key(value: &Value) -> SourceTableKey {
            match value {
                Value::String(value) => SourceTableKey::Text(value.to_str().unwrap().to_owned()),
                Value::Integer(value) => SourceTableKey::Integer(*value),
                Value::Number(value)
                    if value.fract() == 0.0 && value.abs() <= 9_007_199_254_740_991.0 =>
                {
                    SourceTableKey::Integer(*value as i64)
                }
                _ => panic!("unrepresented observed configuration key"),
            }
        }
        fn value(&mut self, value: Value, depth: usize) -> ProgramValue {
            self.charge(&value);
            assert!(depth <= 32, "live projection depth bound");
            match value {
                Value::Nil => ProgramValue::Nil,
                Value::Boolean(value) => ProgramValue::Boolean(value),
                Value::Integer(value) => ProgramValue::Number(value as f64),
                Value::Number(value) => ProgramValue::Number(value),
                Value::String(value) => ProgramValue::Bytes(value.as_bytes().to_vec()),
                Value::Table(table) => {
                    let pointer = table.to_pointer() as usize;
                    if let Some(id) = self.ids.get(&pointer) {
                        return ProgramValue::Table(*id);
                    }
                    let id = ProgramTableId(self.graph.tables.len() as u32 + 1);
                    self.ids.insert(pointer, id);
                    self.graph.tables.push(ProgramTable::default());
                    let selection = self.selected.get(&pointer).cloned();
                    let mut coverage = SourceTableCoverage {
                        inventory: SourceTableInventory::Complete,
                        known_absent: BTreeSet::new(),
                        unavailable: BTreeSet::new(),
                        index_fallback: SourceTableIndexFallback::Nil,
                        call_fallback: SourceTableCallFallback::NonCallable,
                    };
                    if let Some(meta) = table.metatable() {
                        assert!(selection.is_some(), "unselected behavior-bearing table");
                        for row in meta.clone().pairs::<Value, Value>() {
                            let (key, value) = row.unwrap();
                            self.charge(&key);
                            self.charge(&value);
                            if let SourceTableKey::Text(key) = Self::key(&key) {
                                assert!(
                                    !key.starts_with("__") || key == "__index" || key == "__call",
                                    "unrepresented observed metamethod {key}"
                                );
                            }
                        }
                        if !matches!(meta.raw_get::<Value>("__call").unwrap(), Value::Nil) {
                            coverage.call_fallback = SourceTableCallFallback::Unavailable;
                        }
                        if !matches!(meta.raw_get::<Value>("__index").unwrap(), Value::Nil) {
                            coverage.index_fallback = SourceTableIndexFallback::Unavailable;
                        }
                    }
                    let mut rows = Vec::new();
                    for row in table.pairs::<Value, Value>() {
                        let (key, value) = row.unwrap();
                        self.charge(&key);
                        self.charge(&value);
                        rows.push((key, value));
                    }
                    rows.sort_by_key(|(key, _)| Self::key(key));
                    let mut entries = Vec::new();
                    for (key, value) in rows {
                        let typed = Self::key(&key);
                        if selection.as_ref().is_some_and(|fields| !matches!(&typed, SourceTableKey::Text(key) if fields.contains(key))) {
                            coverage.unavailable.insert(typed);
                        } else {
                            entries.push((self.value(key, depth + 1), self.value(value, depth + 1)));
                        }
                    }
                    self.graph.tables[id.0 as usize - 1].entries = entries;
                    if selection.is_some() {
                        coverage.validate_shape().unwrap();
                        self.coverage.insert(id, coverage);
                    }
                    ProgramValue::Table(id)
                }
                _ => panic!("unrepresented live configuration value"),
            }
        }
    }
    let mut capture = Capture {
        selected,
        ids: BTreeMap::new(),
        graph: ProgramValueGraph::default(),
        coverage: BTreeMap::new(),
        values: 0,
        bytes: 0,
    };
    let value = capture.value(Value::Table(config.clone()), 0);
    capture.graph.values.push(value);
    (capture.graph, capture.coverage)
}
