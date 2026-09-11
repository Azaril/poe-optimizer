//! Invocation-local identity graph. Imported and definition tables stay read-only;
//! only tables created by this invocation admit writes. No copy-on-write occurs.
use super::{ProgramLimits, ProgramRuntimeError as Error, RuntimeResult as Result};
use poe_optimizer_data::modifier_parser::{
    ModifierParserCatalog, ParserCallbackId, ParserFactoryLiteral, ParserNonFinite, ParserTableId,
    ParserValue,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

/// One-based table reference in a single input/output graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProgramTableId(pub u32);

/// Raw Lua values. Numbers retain their IEEE bits; strings need not be UTF-8.
/// Callback IDs refer to the catalog retained by the compiled program/output.
#[derive(Debug, Clone, PartialEq)]
pub enum ProgramValue {
    Nil,
    Boolean(bool),
    Number(f64),
    Bytes(Vec<u8>),
    Table(ProgramTableId),
    Callback(ParserCallbackId),
}

/// Unordered Lua entries, including arbitrary scalar/table/function keys. Input
/// rejects nil values, nil/NaN keys and duplicate keys after Lua normalization.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProgramTable {
    pub entries: Vec<(ProgramValue, ProgramValue)>,
}

/// A return/argument pack and identity graph, not the public parser copy boundary.
/// Cycles and shared tables (including table keys) are represented without recursion.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProgramValueGraph {
    pub values: Vec<ProgramValue>,
    pub tables: Vec<ProgramTable>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum TableRef {
    Heap(u32),
    Argument(u32),
    Definition(ParserTableId),
}

#[derive(Debug, Clone)]
pub(super) enum V {
    Nil,
    Boolean(bool),
    Number(f64),
    Bytes(Arc<[u8]>),
    Table(TableRef),
    Callback(ParserCallbackId),
}
impl V {
    pub(super) fn truthy(&self) -> bool {
        !matches!(self, Self::Nil | Self::Boolean(false))
    }
    pub(super) fn as_bytes(&self) -> Option<&[u8]> {
        if let Self::Bytes(bytes) = self {
            Some(bytes)
        } else {
            None
        }
    }
    /// Lua scalar/identity equality, deliberately distinct from bit equality:
    /// NaN is unequal to itself; signed zero compares equal.
    pub(super) fn lua_equal(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Nil, Self::Nil) => true,
            (Self::Boolean(a), Self::Boolean(b)) => a == b,
            (Self::Number(a), Self::Number(b)) => a == b,
            (Self::Bytes(a), Self::Bytes(b)) => a == b,
            (Self::Table(a), Self::Table(b)) => a == b,
            (Self::Callback(a), Self::Callback(b)) => a == b,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    Boolean(bool),
    Number(u64),
    Bytes(Arc<[u8]>),
    Table(TableRef),
    Callback(ParserCallbackId),
}
impl Key {
    fn read(value: &V) -> Option<Self> {
        match value {
            V::Nil => None,
            V::Boolean(v) => Some(Self::Boolean(*v)),
            V::Number(v) if v.is_nan() => None,
            V::Number(v) => Some(Self::Number(if *v == 0.0 { 0 } else { v.to_bits() })),
            V::Bytes(v) => Some(Self::Bytes(v.clone())),
            V::Table(v) => Some(Self::Table(*v)),
            V::Callback(v) => Some(Self::Callback(*v)),
        }
    }
    fn write(value: &V) -> Result<Self> {
        Self::read(value).ok_or_else(|| Error::source("table index is nil or NaN"))
    }
    fn value(&self) -> V {
        match self {
            Self::Boolean(v) => V::Boolean(*v),
            Self::Number(v) => V::Number(f64::from_bits(*v)),
            Self::Bytes(v) => V::Bytes(v.clone()),
            Self::Table(v) => V::Table(*v),
            Self::Callback(v) => V::Callback(*v),
        }
    }
    fn positive_integer(&self) -> Option<f64> {
        if let Self::Number(bits) = self {
            let number = f64::from_bits(*bits);
            (number.is_finite() && number >= 1.0 && number.fract() == 0.0).then_some(number)
        } else {
            None
        }
    }
}
#[derive(Default)]
struct Table {
    entries: BTreeMap<Key, V>,
    // Positive finite integer keys have monotonically ordered IEEE bits. This
    // index keeps append/length logarithmic rather than rescanning growing lists.
    positive: BTreeSet<u64>,
}
impl Table {
    fn insert(&mut self, key: Key, value: V) -> Option<V> {
        if let Some(n) = key.positive_integer() {
            self.positive.insert(n.to_bits());
        }
        self.entries.insert(key, value)
    }
    fn remove(&mut self, key: &Key) {
        if let Some(n) = key.positive_integer() {
            self.positive.remove(&n.to_bits());
        }
        self.entries.remove(key);
    }
    fn boundary(&self) -> (usize, f64) {
        (
            self.positive.len(),
            self.positive.last().map_or(0.0, |n| f64::from_bits(*n)),
        )
    }
}

/// Conservative cumulative units across graph import, new heap storage and graph
/// export. Shared Arc clones do not copy/charge their string payload again.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct HeapStats {
    pub(super) values: usize,
    pub(super) bytes: usize,
    pub(super) tables: usize,
}
struct Budget {
    limits: ProgramLimits,
    used: HeapStats,
}
impl Budget {
    fn add(used: &mut usize, amount: usize, max: usize, name: &'static str) -> Result<()> {
        let next = used
            .checked_add(amount)
            .filter(|v| *v <= max)
            .ok_or_else(|| Error::resource(name))?;
        *used = next;
        Ok(())
    }
    fn values(&mut self, amount: usize) -> Result<()> {
        Self::add(
            &mut self.used.values,
            amount,
            self.limits.max_values,
            "program heap values",
        )
    }
    fn bytes(&mut self, amount: usize) -> Result<()> {
        Self::add(
            &mut self.used.bytes,
            amount,
            self.limits.max_bytes,
            "program heap bytes",
        )
    }
    fn tables(&mut self, amount: usize) -> Result<()> {
        Self::add(
            &mut self.used.tables,
            amount,
            self.limits.max_tables.min(u32::MAX as usize),
            "program heap tables",
        )
    }
}

pub(super) struct Heap {
    catalog: ModifierParserCatalog,
    arguments: Vec<Table>,
    tables: Vec<Table>,
    budget: Budget,
}
impl Heap {
    pub(super) fn new(
        catalog: &ModifierParserCatalog,
        input: &ProgramValueGraph,
        limits: &ProgramLimits,
    ) -> Result<(Self, Vec<V>)> {
        let mut budget = Budget {
            limits: *limits,
            used: HeapStats::default(),
        };
        // Validate every input node, including unreachable tables, before copying.
        budget.tables(input.tables.len())?;
        budget.values(input.values.len())?;
        for value in &input.values {
            validate_input(value, input.tables.len(), catalog, &mut budget)?;
        }
        for table in &input.tables {
            budget.values(
                table
                    .entries
                    .len()
                    .checked_mul(2)
                    .ok_or_else(|| Error::resource("input graph entries"))?,
            )?;
            for (key, value) in &table.entries {
                validate_input(key, input.tables.len(), catalog, &mut budget)?;
                validate_input(value, input.tables.len(), catalog, &mut budget)?;
                if matches!(key, ProgramValue::Nil)
                    || matches!(key, ProgramValue::Number(n) if n.is_nan())
                {
                    return Err(Error::input("nil/NaN input table key"));
                }
                if matches!(value, ProgramValue::Nil) {
                    return Err(Error::input("nil input table entry"));
                }
            }
        }
        // Each auxiliary integer-key index entry is charged before construction.
        for table in &input.tables {
            let count = table
                .entries
                .iter()
                .filter(|(key, _)| {
                    matches!(key,
                ProgramValue::Number(n) if n.is_finite() && *n >= 1.0 && n.fract() == 0.0)
                })
                .count();
            budget.values(count)?;
        }
        let values = input.values.iter().map(input_value).collect();
        let mut arguments = Vec::with_capacity(input.tables.len());
        for source in &input.tables {
            let mut table = Table::default();
            for (key, value) in &source.entries {
                let key = Key::read(&input_value(key)).expect("validated input key");
                if table.insert(key, input_value(value)).is_some() {
                    return Err(Error::input("duplicate input table key under Lua equality"));
                }
            }
            arguments.push(table);
        }
        Ok((
            Self {
                catalog: catalog.clone(),
                arguments,
                tables: Vec::new(),
                budget,
            },
            values,
        ))
    }
    pub(super) fn stats(&self) -> HeapStats {
        self.budget.used
    }
    pub(super) fn remaining_bytes(&self) -> usize {
        self.budget.limits.max_bytes - self.stats().bytes
    }
    pub(super) fn charge_values(&mut self, amount: usize) -> Result<()> {
        self.budget.values(amount)
    }
    pub(super) fn charge_bytes(&mut self, amount: usize) -> Result<()> {
        self.budget.bytes(amount)
    }
    pub(super) fn bytes(&mut self, bytes: &[u8]) -> Result<V> {
        self.charge_bytes(bytes.len())?;
        Ok(V::Bytes(Arc::from(bytes)))
    }
    pub(super) fn literal(&mut self, value: &ParserFactoryLiteral) -> Result<V> {
        Ok(match value {
            ParserFactoryLiteral::Nil => V::Nil,
            ParserFactoryLiteral::Boolean(v) => V::Boolean(*v),
            ParserFactoryLiteral::Number(v) => V::Number(*v),
            ParserFactoryLiteral::Text(v) => self.bytes(v.as_bytes())?,
            ParserFactoryLiteral::NonFinite(v) => V::Number(nonfinite(*v)),
        })
    }
    pub(super) fn definition(&self, id: ParserTableId) -> Result<V> {
        self.catalog
            .table(id)
            .ok_or_else(|| Error::input("missing definition table"))?;
        Ok(V::Table(TableRef::Definition(id)))
    }
    pub(super) fn capture(&mut self, callback: ParserCallbackId, upvalue: u16) -> Result<V> {
        let source = self
            .catalog
            .callback(callback)
            .and_then(|c| c.upvalues.get(upvalue as usize))
            .ok_or_else(|| Error::input("missing callback capture"))?;
        definition_value(&source.value, &mut self.budget)
    }
    pub(super) fn get(&mut self, table: &V, key: &V) -> Result<V> {
        if table.as_bytes().is_some() {
            return Err(Error::unsupported("generic string metatable lookup"));
        }
        let reference = table_ref(table)?;
        let Some(key) = Key::read(key) else {
            return Ok(V::Nil);
        };
        match reference {
            TableRef::Heap(id) => Ok(self
                .tables
                .get(index(id)?)
                .ok_or_else(|| Error::input("missing heap table"))?
                .entries
                .get(&key)
                .cloned()
                .unwrap_or(V::Nil)),
            TableRef::Argument(id) => Ok(self
                .arguments
                .get(index(id)?)
                .ok_or_else(|| Error::input("missing argument table"))?
                .entries
                .get(&key)
                .cloned()
                .unwrap_or(V::Nil)),
            TableRef::Definition(id) => {
                let source = self
                    .catalog
                    .table(id)
                    .ok_or_else(|| Error::input("missing definition table"))?;
                let value = match &key {
                    Key::Bytes(bytes) => std::str::from_utf8(bytes)
                        .ok()
                        .and_then(|k| source.fields.get(k)),
                    Key::Number(bits) => {
                        let n = f64::from_bits(*bits);
                        (n.is_finite() && n.fract() == 0.0 && n.abs() <= 9_007_199_254_740_991.0)
                            .then(|| source.indexed.get(&(n as i64)))
                            .flatten()
                    }
                    _ => None,
                };
                match value {
                    Some(v) => definition_value(v, &mut self.budget),
                    None => Ok(V::Nil),
                }
            }
        }
    }
    pub(super) fn new_table(&mut self) -> Result<V> {
        self.budget.tables(1)?;
        let id =
            u32::try_from(self.tables.len() + 1).map_err(|_| Error::resource("table identity"))?;
        self.tables.push(Table::default());
        Ok(V::Table(TableRef::Heap(id)))
    }
    pub(super) fn set(&mut self, table: &V, key: V, value: V) -> Result<()> {
        let reference = table_ref(table)?;
        let key = Key::write(&key)?;
        let TableRef::Heap(id) = reference else {
            return Err(Error::unsupported("mutation of a borrowed table"));
        };
        let target = self
            .tables
            .get_mut(index(id)?)
            .ok_or_else(|| Error::input("missing heap table"))?;
        if matches!(value, V::Nil) {
            target.remove(&key);
        } else {
            if !target.entries.contains_key(&key) {
                self.budget
                    .values(2 + usize::from(key.positive_integer().is_some()))?;
            }
            target.insert(key, value);
        }
        Ok(())
    }
    /// Only a unique dense positive-integer boundary is supported. Hash fields,
    /// negative and fractional numeric keys do not change that boundary. Any
    /// positive-integer hole defers rather than assuming a Lua table layout.
    pub(super) fn dense_len(&mut self, table: &V) -> Result<usize> {
        let reference = table_ref(table)?;
        let (count, max) = match reference {
            TableRef::Heap(id) | TableRef::Argument(id) => {
                let table = if matches!(reference, TableRef::Heap(_)) {
                    &self.tables
                } else {
                    &self.arguments
                };
                let table = table
                    .get(index(id)?)
                    .ok_or_else(|| Error::input("missing runtime table"))?;
                table.boundary()
            }
            TableRef::Definition(id) => {
                let source = self
                    .catalog
                    .table(id)
                    .ok_or_else(|| Error::input("missing definition table"))?;
                let mut count = 0usize;
                let mut max = 0.0f64;
                for key in source.indexed.keys().filter(|k| **k > 0) {
                    count += 1;
                    max = max.max(*key as f64);
                }
                (count, max)
            }
        };
        if count as f64 != max {
            return Err(Error::unsupported(
                "Lua length for a table with integer holes",
            ));
        }
        Ok(count)
    }
    pub(super) fn append(&mut self, table: &V, value: V) -> Result<()> {
        let length = self.dense_len(table)?;
        let next = length
            .checked_add(1)
            .filter(|n| *n as u64 <= 9_007_199_254_740_991)
            .ok_or_else(|| Error::resource("table append index"))?;
        self.set(table, V::Number(next as f64), value)
    }
    pub(super) fn freeze(&mut self, values: &[V]) -> Result<ProgramValueGraph> {
        if values.len() > self.budget.limits.max_results {
            return Err(Error::resource("program result pack"));
        }
        self.budget.values(values.len())?;
        let mut export = Export::default();
        let mut roots = Vec::with_capacity(values.len());
        for value in values {
            roots.push(export.value(value, &mut self.budget)?);
        }
        let mut cursor = 0;
        while cursor < export.references.len() {
            let reference = export.references[cursor];
            let entries = match reference {
                TableRef::Heap(id) | TableRef::Argument(id) => {
                    let tables = if matches!(reference, TableRef::Heap(_)) {
                        &self.tables
                    } else {
                        &self.arguments
                    };
                    let source = tables
                        .get(index(id)?)
                        .ok_or_else(|| Error::input("missing runtime export table"))?;
                    self.budget.values(
                        source
                            .entries
                            .len()
                            .checked_mul(2)
                            .ok_or_else(|| Error::resource("export entries"))?,
                    )?;
                    let mut entries = Vec::with_capacity(source.entries.len());
                    for (key, value) in &source.entries {
                        entries.push((
                            export.value(&key.value(), &mut self.budget)?,
                            export.value(value, &mut self.budget)?,
                        ));
                    }
                    entries
                }
                TableRef::Definition(id) => {
                    let source = self
                        .catalog
                        .table(id)
                        .ok_or_else(|| Error::input("missing definition export table"))?;
                    let count = source
                        .fields
                        .len()
                        .checked_add(source.indexed.len())
                        .ok_or_else(|| Error::resource("export entries"))?;
                    self.budget.values(
                        count
                            .checked_mul(2)
                            .ok_or_else(|| Error::resource("export entries"))?,
                    )?;
                    let mut entries = Vec::with_capacity(count);
                    for (key, value) in &source.fields {
                        self.budget.bytes(key.len())?;
                        entries.push((
                            ProgramValue::Bytes(key.as_bytes().to_vec()),
                            export.definition_value(value, &mut self.budget)?,
                        ));
                    }
                    for (key, value) in &source.indexed {
                        entries.push((
                            ProgramValue::Number(*key as f64),
                            export.definition_value(value, &mut self.budget)?,
                        ));
                    }
                    entries
                }
            };
            export.tables[cursor] = ProgramTable { entries };
            cursor += 1;
        }
        Ok(ProgramValueGraph {
            values: roots,
            tables: export.tables,
        })
    }
}

fn index(id: u32) -> Result<usize> {
    id.checked_sub(1)
        .map(|n| n as usize)
        .ok_or_else(|| Error::input("zero table identity"))
}
fn table_ref(value: &V) -> Result<TableRef> {
    match value {
        V::Table(reference) => Ok(*reference),
        _ => Err(Error::source("attempt to index a non-table value")),
    }
}
fn nonfinite(value: ParserNonFinite) -> f64 {
    match value {
        ParserNonFinite::PositiveInfinity => f64::INFINITY,
        ParserNonFinite::NegativeInfinity => f64::NEG_INFINITY,
        ParserNonFinite::Nan => f64::from_bits(0xfff8_0000_0000_0000),
    }
}
fn definition_value(value: &ParserValue, budget: &mut Budget) -> Result<V> {
    Ok(match value {
        ParserValue::Nil => V::Nil,
        ParserValue::Boolean(v) => V::Boolean(*v),
        ParserValue::Number(v) => V::Number(*v),
        ParserValue::Text(v) => {
            budget.bytes(v.len())?;
            V::Bytes(Arc::from(v.as_bytes()))
        }
        ParserValue::NonFinite(v) => V::Number(nonfinite(*v)),
        ParserValue::Table(id) => V::Table(TableRef::Definition(*id)),
        ParserValue::Callback(id) => V::Callback(*id),
    })
}
fn validate_input(
    value: &ProgramValue,
    tables: usize,
    catalog: &ModifierParserCatalog,
    budget: &mut Budget,
) -> Result<()> {
    match value {
        ProgramValue::Bytes(bytes) => budget.bytes(bytes.len())?,
        ProgramValue::Table(id) if index(id.0)? >= tables => {
            return Err(Error::input("missing input table reference"));
        }
        ProgramValue::Callback(id) if catalog.callback(*id).is_none() => {
            return Err(Error::input("missing input callback reference"));
        }
        _ => (),
    }
    Ok(())
}
fn input_value(value: &ProgramValue) -> V {
    match value {
        ProgramValue::Nil => V::Nil,
        ProgramValue::Boolean(v) => V::Boolean(*v),
        ProgramValue::Number(v) => V::Number(*v),
        ProgramValue::Bytes(v) => V::Bytes(Arc::from(v.as_slice())),
        ProgramValue::Table(id) => V::Table(TableRef::Argument(id.0)),
        ProgramValue::Callback(id) => V::Callback(*id),
    }
}

#[derive(Default)]
struct Export {
    tables: Vec<ProgramTable>,
    references: Vec<TableRef>,
    seen: BTreeMap<TableRef, ProgramTableId>,
}
impl Export {
    fn table(&mut self, reference: TableRef, budget: &mut Budget) -> Result<ProgramValue> {
        if let Some(id) = self.seen.get(&reference) {
            return Ok(ProgramValue::Table(*id));
        }
        budget.tables(1)?;
        let id = ProgramTableId(
            u32::try_from(self.tables.len() + 1)
                .map_err(|_| Error::resource("export table identity"))?,
        );
        self.seen.insert(reference, id);
        self.references.push(reference);
        self.tables.push(ProgramTable::default());
        Ok(ProgramValue::Table(id))
    }
    fn value(&mut self, value: &V, budget: &mut Budget) -> Result<ProgramValue> {
        Ok(match value {
            V::Nil => ProgramValue::Nil,
            V::Boolean(v) => ProgramValue::Boolean(*v),
            V::Number(v) => ProgramValue::Number(*v),
            V::Bytes(v) => {
                budget.bytes(v.len())?;
                ProgramValue::Bytes(v.to_vec())
            }
            V::Table(reference) => self.table(*reference, budget)?,
            V::Callback(id) => ProgramValue::Callback(*id),
        })
    }
    fn definition_value(
        &mut self,
        value: &ParserValue,
        budget: &mut Budget,
    ) -> Result<ProgramValue> {
        // Export definitions directly: no intermediate Arc/string copy.
        Ok(match value {
            ParserValue::Nil => ProgramValue::Nil,
            ParserValue::Boolean(v) => ProgramValue::Boolean(*v),
            ParserValue::Number(v) => ProgramValue::Number(*v),
            ParserValue::Text(v) => {
                budget.bytes(v.len())?;
                ProgramValue::Bytes(v.as_bytes().to_vec())
            }
            ParserValue::NonFinite(v) => ProgramValue::Number(nonfinite(*v)),
            ParserValue::Table(id) => self.table(TableRef::Definition(*id), budget)?,
            ParserValue::Callback(id) => ProgramValue::Callback(*id),
        })
    }
}

#[cfg(test)]
mod tests;
