//! Original parser dependency for isolating the native assembly producer.
//! No source assembly output enters this provider; lost input aliasing is refused.
use mlua::{Function, Table, Value};
use poe_optimizer_data::item_loading::{ItemMetadataTable, ItemMetadataValue};
use poe_optimizer_import::item_loading::*;
use std::collections::BTreeSet;
pub struct OriginalParser {
    pub function: Function,
    pub calls: usize,
}
struct Convert {
    seen: BTreeSet<usize>,
    values: usize,
    bytes: usize,
}
impl Convert {
    fn value(&mut self, v: Value, depth: usize) -> Result<ItemMetadataValue, String> {
        if self.values == 0 || depth > 64 {
            return Err("source parser metadata bound".into());
        }
        self.values -= 1;
        Ok(match v {
            Value::Boolean(v) => ItemMetadataValue::Boolean(v),
            Value::Integer(v) => ItemMetadataValue::Number(v as f64),
            Value::Number(v) if v.is_finite() => ItemMetadataValue::Number(v),
            Value::String(v) => {
                self.bytes = self
                    .bytes
                    .checked_sub(v.as_bytes().len())
                    .ok_or("source parser text bound")?;
                ItemMetadataValue::Text(
                    v.to_str()
                        .map_err(|_| "source parser non-UTF8 text")?
                        .to_owned(),
                )
            }
            Value::Table(t) => {
                let table = self.table(t, depth + 1)?;
                if table.fields.is_empty()
                    && !table.indexed.is_empty()
                    && table
                        .indexed
                        .keys()
                        .copied()
                        .eq(1..=table.indexed.len() as i64)
                {
                    ItemMetadataValue::Array(table.indexed.into_values().collect())
                } else {
                    ItemMetadataValue::Table(table)
                }
            }
            _ => return Err("source parser value outside finite metadata contract".into()),
        })
    }
    fn table(&mut self, t: Table, depth: usize) -> Result<ItemMetadataTable, String> {
        if depth > 64 || !self.seen.insert(t.to_pointer() as usize) {
            return Err(
                "source parser input alias/cycle cannot be recovered from metadata tree".into(),
            );
        }
        if t.metatable().is_some() {
            return Err("source parser input metatable".into());
        }
        let mut out = ItemMetadataTable::default();
        for row in t.pairs::<Value, Value>() {
            let (key, value) = row.map_err(|e| e.to_string())?;
            let value = self.value(value, depth + 1)?;
            match key {
                Value::String(key) => {
                    self.bytes = self
                        .bytes
                        .checked_sub(key.as_bytes().len())
                        .ok_or("source parser key bytes")?;
                    out.fields.insert(
                        key.to_str()
                            .map_err(|_| "source parser non-UTF8 key")?
                            .to_owned(),
                        value,
                    );
                }
                Value::Integer(key) => {
                    out.indexed.insert(key, value);
                }
                Value::Number(key)
                    if key.is_finite()
                        && key.fract() == 0.0
                        && key.abs() <= 9_007_199_254_740_991.0 =>
                {
                    out.indexed.insert(key as i64, value);
                }
                _ => return Err("source parser key representation".into()),
            }
        }
        Ok(out)
    }
}
impl ItemLoadProvider for OriginalParser {
    fn parse_modifier(&mut self, request: &ParseRequest) -> DependencyResult<ParseOutcome> {
        self.calls += 1;
        let packed = match self
            .function
            .call::<mlua::MultiValue>((request.text.as_str(), request.combined))
        {
            Ok(v) => v,
            Err(e) => return DependencyResult::SourceError(e.to_string()),
        };
        if packed.len() > 2 {
            return DependencyResult::Unavailable(
                "source parser pack exceeds provider contract".into(),
            );
        }
        let mut values = packed.into_iter();
        let first = values.next().unwrap_or(Value::Nil);
        let extra = match values.next().unwrap_or(Value::Nil) {
            Value::Nil => None,
            Value::String(t) => match t.to_str() {
                Ok(s) => Some(s.to_owned()),
                Err(_) => {
                    return DependencyResult::Unavailable("source parser extra non-UTF8".into());
                }
            },
            _ => return DependencyResult::Unavailable("source parser extra type".into()),
        };
        let modifiers = match first {
            Value::Nil => None,
            Value::Table(table) => {
                let mut c = Convert {
                    seen: BTreeSet::new(),
                    values: 100000,
                    bytes: 4 * 1024 * 1024,
                };
                let parsed = match c.table(table, 0) {
                    Ok(v) => v,
                    Err(e) => return DependencyResult::Unavailable(e),
                };
                if !parsed.fields.is_empty()
                    || !parsed
                        .indexed
                        .keys()
                        .copied()
                        .eq(1..=parsed.indexed.len() as i64)
                {
                    return DependencyResult::Unavailable("source parser non-list result".into());
                }
                let mut rows = Vec::new();
                for value in parsed.indexed.into_values() {
                    match value {
                        ItemMetadataValue::Table(t) => rows.push(t),
                        _ => {
                            return DependencyResult::Unavailable(
                                "source parser row is not a mixed modifier table".into(),
                            );
                        }
                    }
                }
                Some(rows)
            }
            _ => return DependencyResult::Unavailable("source parser result type".into()),
        };
        DependencyResult::Available(ParseOutcome { modifiers, extra })
    }
}

/// Record authentic machine-issued requests without substituting assembly outcomes.
pub struct Recorded<P> {
    pub inner: P,
    pub requests: Vec<AssemblyRequest>,
}
impl<P: ItemLoadProvider> ItemLoadProvider for Recorded<P> {
    fn parse_modifier(&mut self, r: &ParseRequest) -> DependencyResult<ParseOutcome> {
        self.inner.parse_modifier(r)
    }
    fn format_line(&mut self, r: &FormatRequest) -> DependencyResult<String> {
        self.inner.format_line(r)
    }
    fn format_with_trace(&mut self, r: &FormatRequest) -> FormatOutcome {
        self.inner.format_with_trace(r)
    }
    fn catalyst_scaling(
        &self,
    ) -> Option<&poe_optimizer_data::item_scalability::CatalystScalingData> {
        self.inner.catalyst_scaling()
    }
    fn lookup_unique(&mut self, r: &UniqueRequest) -> DependencyResult<Option<UniqueOutcome>> {
        self.inner.lookup_unique(r)
    }
    fn assemble(&mut self, r: &AssemblyRequest) -> DependencyResult<AssemblyOutcome> {
        self.assemble_with_trace(r).outcome
    }
    fn assemble_with_trace(&mut self, r: &AssemblyRequest) -> AssemblyExecution {
        assert!(
            self.requests.len() < 256,
            "directed assembly request record bound"
        );
        self.requests.push(r.clone());
        self.inner.assemble_with_trace(r)
    }
}
