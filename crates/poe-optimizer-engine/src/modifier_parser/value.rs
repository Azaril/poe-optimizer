//! Owned, lossless parser results. They are not admitted numerical modifiers.
use super::{ParserError, ParserResult};
use crate::item_tools::lua_number_text;
use poe_optimizer_data::modifier_parser::{
    ModifierParserCatalog, ParserCallbackId, ParserNonFinite, ParserTableId, ParserValue,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub const MAX_OUTPUT_VALUES: usize = 65_536;
pub const MAX_OUTPUT_DEPTH: usize = 64;
pub const MAX_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
#[derive(Debug, Clone, PartialEq)]
pub enum ModifierValue {
    Nil,
    Boolean(bool),
    /// Includes IEEE non-finite values; adapters must preserve or explicitly defer them.
    Number(f64),
    /// Lua strings are bytes, including capture fragments that are not valid UTF-8.
    Bytes(Vec<u8>),
    Table(Arc<ModifierTable>),
    /// Opaque function identity belongs to the injected catalog; it is not executable here.
    Callback(ParserCallbackId),
}
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ModifierTable {
    pub fields: BTreeMap<String, ModifierValue>,
    pub indexed: BTreeMap<i64, ModifierValue>,
}
impl ModifierTable {
    pub fn field(&self, key: &str) -> &ModifierValue {
        self.fields.get(key).unwrap_or(&ModifierValue::Nil)
    }
    pub fn indexed_value(&self, key: i64) -> &ModifierValue {
        self.indexed.get(&key).unwrap_or(&ModifierValue::Nil)
    }
    pub(super) fn set(&mut self, key: &str, value: ModifierValue) {
        if matches!(value, ModifierValue::Nil) {
            self.fields.remove(key);
        } else {
            self.fields.insert(key.into(), value);
        }
    }
    pub(super) fn dense(&self) -> Vec<ModifierValue> {
        (1..).map_while(|i| self.indexed.get(&i).cloned()).collect()
    }
    pub(super) fn from_values(values: impl IntoIterator<Item = ModifierValue>) -> Self {
        Self {
            fields: BTreeMap::new(),
            indexed: values
                .into_iter()
                .enumerate()
                .filter(|(_, v)| !matches!(v, ModifierValue::Nil))
                .map(|(i, v)| (i as i64 + 1, v))
                .collect(),
        }
    }
}
impl ModifierValue {
    pub fn truthy(&self) -> bool {
        !matches!(self, Self::Nil | Self::Boolean(false))
    }
    pub fn as_table(&self) -> Option<&ModifierTable> {
        if let Self::Table(table) = self {
            Some(table)
        } else {
            None
        }
    }
    pub fn as_bytes(&self) -> Option<&[u8]> {
        if let Self::Bytes(bytes) = self {
            Some(bytes)
        } else {
            None
        }
    }
    pub(super) fn table(&self) -> ParserResult<&ModifierTable> {
        self.as_table()
            .ok_or_else(|| ParserError::SourceError("expected table".into()))
    }
    pub(super) fn field(&self, key: &str) -> ParserResult<&Self> {
        match self {
            Self::Table(table) => Ok(table.field(key)),
            // Only source structural tag fields are read. None is a string method.
            Self::Bytes(_) => Ok(&Self::Nil),
            _ => Err(ParserError::SourceError(
                "attempt to index a non-table value".into(),
            )),
        }
    }
    pub(super) fn number(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            Self::Bytes(bytes) => crate::lua_number::parse_number(bytes),
            _ => None,
        }
    }
    pub(super) fn negated(&self) -> ParserResult<Self> {
        self.number()
            .map(|n| Self::Number(-n))
            .ok_or_else(|| ParserError::SourceError("arithmetic on a non-number".into()))
    }
    pub(super) fn string(&self) -> ParserResult<Vec<u8>> {
        match self {
            Self::Bytes(bytes) => Ok(bytes.clone()),
            Self::Number(n) => Ok(lua_number_text(*n).into_bytes()),
            _ => Err(ParserError::SourceError(
                "concatenation of a non-string value".into(),
            )),
        }
    }
    pub(super) fn text(text: impl AsRef<[u8]>) -> Self {
        Self::Bytes(text.as_ref().to_vec())
    }
}
#[derive(Debug, Default)]
pub(super) struct OutputBudget {
    values: usize,
    bytes: usize,
}
impl OutputBudget {
    pub(super) fn charge(&mut self, bytes: usize) -> ParserResult<()> {
        self.values = self
            .values
            .checked_add(1)
            .ok_or(ParserError::ResourceBound("output values"))?;
        self.bytes = self
            .bytes
            .checked_add(bytes)
            .ok_or(ParserError::ResourceBound("output bytes"))?;
        if self.values > MAX_OUTPUT_VALUES || self.bytes > MAX_OUTPUT_BYTES {
            return Err(ParserError::ResourceBound("output values/bytes"));
        }
        Ok(())
    }
}
pub(super) fn copy_value(
    catalog: &ModifierParserCatalog,
    value: &ParserValue,
    budget: &mut OutputBudget,
    memo: &mut BTreeMap<ParserTableId, Arc<ModifierTable>>,
) -> ParserResult<ModifierValue> {
    fn visit(
        catalog: &ModifierParserCatalog,
        value: &ParserValue,
        depth: usize,
        active: &mut BTreeSet<ParserTableId>,
        budget: &mut OutputBudget,
        memo: &mut BTreeMap<ParserTableId, Arc<ModifierTable>>,
    ) -> ParserResult<ModifierValue> {
        if depth > MAX_OUTPUT_DEPTH {
            return Err(ParserError::ResourceBound("output depth"));
        }
        budget.charge(if let ParserValue::Text(s) = value {
            s.len()
        } else {
            0
        })?;
        Ok(match value {
            ParserValue::Nil => ModifierValue::Nil,
            ParserValue::Boolean(v) => ModifierValue::Boolean(*v),
            ParserValue::Number(v) => ModifierValue::Number(*v),
            ParserValue::Text(v) => ModifierValue::Bytes(v.as_bytes().to_vec()),
            ParserValue::NonFinite(v) => ModifierValue::Number(match v {
                ParserNonFinite::PositiveInfinity => f64::INFINITY,
                ParserNonFinite::NegativeInfinity => f64::NEG_INFINITY,
                ParserNonFinite::Nan => f64::from_bits(0xfff8_0000_0000_0000),
            }),
            ParserValue::Callback(v) => ModifierValue::Callback(*v),
            ParserValue::Table(id) => {
                if let Some(table) = memo.get(id) {
                    return Ok(ModifierValue::Table(table.clone()));
                }
                if !active.insert(*id) {
                    return Err(ParserError::ResourceBound("cyclic source copy"));
                }
                let source = catalog
                    .table(*id)
                    .ok_or_else(|| ParserError::InvalidData("missing table reference".into()))?;
                let mut table = ModifierTable::default();
                for (key, value) in &source.fields {
                    budget.charge(key.len())?;
                    table.fields.insert(
                        key.clone(),
                        visit(catalog, value, depth + 1, active, budget, memo)?,
                    );
                }
                for (key, value) in &source.indexed {
                    table.indexed.insert(
                        *key,
                        visit(catalog, value, depth + 1, active, budget, memo)?,
                    );
                }
                active.remove(id);
                let table = Arc::new(table);
                memo.insert(*id, table.clone());
                ModifierValue::Table(table)
            }
        })
    }
    visit(catalog, value, 0, &mut BTreeSet::new(), budget, memo)
}

/// Original copyTable creates fresh nested tables even when source references
/// are shared. Functions remain opaque references and scalars preserve bits.
pub(super) fn deep_copy(
    value: &ModifierValue,
    budget: &mut OutputBudget,
) -> ParserResult<ModifierValue> {
    fn visit(
        value: &ModifierValue,
        depth: usize,
        budget: &mut OutputBudget,
    ) -> ParserResult<ModifierValue> {
        if depth > MAX_OUTPUT_DEPTH {
            return Err(ParserError::ResourceBound("copy depth"));
        }
        budget.charge(if let ModifierValue::Bytes(s) = value {
            s.len()
        } else {
            0
        })?;
        if let ModifierValue::Table(source) = value {
            let mut table = ModifierTable::default();
            for (key, value) in &source.fields {
                budget.charge(key.len())?;
                table
                    .fields
                    .insert(key.clone(), visit(value, depth + 1, budget)?);
            }
            for (key, value) in &source.indexed {
                table.indexed.insert(*key, visit(value, depth + 1, budget)?);
            }
            Ok(ModifierValue::Table(Arc::new(table)))
        } else {
            Ok(value.clone())
        }
    }
    visit(value, 0, budget)
}

/// Compare contents with exact numerical bits and bounded graph-pair work.
/// A DAG is visited once per pair instead of recursively expanding shared tails.
pub(super) fn equivalent(
    left: &ModifierValue,
    right: &ModifierValue,
    budget: &mut crate::lua_pattern::MatchBudget,
) -> ParserResult<bool> {
    let mut pending = vec![(left, right)];
    let mut visited = BTreeSet::new();
    while let Some((left, right)) = pending.pop() {
        budget.charge(1)?;
        match (left, right) {
            (ModifierValue::Nil, ModifierValue::Nil) => {}
            (ModifierValue::Boolean(a), ModifierValue::Boolean(b)) if a == b => {}
            (ModifierValue::Number(a), ModifierValue::Number(b)) if a.to_bits() == b.to_bits() => {}
            (ModifierValue::Callback(a), ModifierValue::Callback(b)) if a == b => {}
            (ModifierValue::Bytes(a), ModifierValue::Bytes(b)) => {
                budget.charge(a.len().min(b.len()) as u64)?;
                if a != b {
                    return Ok(false);
                }
            }
            (ModifierValue::Table(a), ModifierValue::Table(b)) => {
                if Arc::ptr_eq(a, b) {
                    continue;
                }
                let pair = (Arc::as_ptr(a) as usize, Arc::as_ptr(b) as usize);
                if !visited.insert(pair) {
                    continue;
                }
                if a.fields.len() != b.fields.len() || a.indexed.len() != b.indexed.len() {
                    return Ok(false);
                }
                if pending
                    .len()
                    .checked_add(a.fields.len() + a.indexed.len())
                    .is_none_or(|n| n > MAX_OUTPUT_VALUES)
                {
                    return Err(ParserError::ResourceBound("equality graph worklist"));
                }
                for ((ak, av), (bk, bv)) in a.fields.iter().zip(&b.fields) {
                    budget.charge(ak.len().min(bk.len()) as u64)?;
                    if ak != bk {
                        return Ok(false);
                    }
                    pending.push((av, bv));
                }
                for ((ak, av), (bk, bv)) in a.indexed.iter().zip(&b.indexed) {
                    if ak != bk {
                        return Ok(false);
                    }
                    pending.push((av, bv));
                }
            }
            _ => return Ok(false),
        }
    }
    Ok(true)
}
