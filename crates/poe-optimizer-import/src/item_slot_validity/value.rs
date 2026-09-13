//! Borrowed finite values; table references always retain their original owner.
use super::{AssemblyError, Result};
use crate::item_loading::assembly::{AssembledItem, AssemblyTableId, AssemblyValue};
use poe_optimizer_data::item_loading::{ItemMetadataTable, ItemMetadataValue};
use std::collections::btree_map;

#[derive(Clone, Copy, Debug)]
pub enum Value<'a> {
    Nil,
    Boolean(bool),
    Number(f64),
    Text(&'a str),
    Table(Table<'a>),
}
#[derive(Clone, Copy, Debug)]
pub struct Table<'a>(Storage<'a>);
#[derive(Clone, Copy, Debug)]
enum Storage<'a> {
    Item(&'a AssembledItem, AssemblyTableId),
    Metadata(&'a ItemMetadataTable),
    // Retain the Vec object, not its possibly shared empty-slice sentinel.
    Array(&'a Vec<ItemMetadataValue>),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key<'a> {
    Text(&'a str),
    Index(i64),
}
impl<'a> Value<'a> {
    pub fn item(owner: &'a AssembledItem) -> Self {
        Self::Table(Table(Storage::Item(owner, owner.root())))
    }
    pub fn table(table: &'a ItemMetadataTable) -> Self {
        Self::Table(Table(Storage::Metadata(table)))
    }
    pub fn metadata(value: &'a ItemMetadataValue) -> Result<Self> {
        Ok(match value {
            ItemMetadataValue::Boolean(v) => Self::Boolean(*v),
            ItemMetadataValue::Number(v) => finite(*v)?,
            ItemMetadataValue::Text(v) => Self::Text(v),
            ItemMetadataValue::Table(v) => Self::table(v),
            ItemMetadataValue::Array(v) => Self::Table(Table(Storage::Array(v))),
            ItemMetadataValue::Callback(_) => {
                return Err(AssemblyError::unsupported(
                    "slot validity reached a callable metadata value",
                ));
            }
        })
    }
    fn assembled(owner: &'a AssembledItem, value: &'a AssemblyValue) -> Result<Self> {
        Ok(match value {
            AssemblyValue::Nil => Self::Nil,
            AssemblyValue::Boolean(v) => Self::Boolean(*v),
            AssemblyValue::Number(v) => finite(*v)?,
            AssemblyValue::Text(v) => Self::Text(v),
            AssemblyValue::Table(v) => Self::Table(Table(Storage::Item(owner, *v))),
        })
    }
    pub fn truthy(self) -> bool {
        !matches!(self, Self::Nil | Self::Boolean(false))
    }
    pub fn same_identity(self, other: Self) -> bool {
        match (self, other) {
            (Self::Table(a), Self::Table(b)) => a.same_identity(b),
            (Self::Nil, Self::Nil) => true,
            (Self::Boolean(a), Self::Boolean(b)) => a == b,
            (Self::Number(a), Self::Number(b)) => a == b,
            (Self::Text(a), Self::Text(b)) => a == b,
            _ => false,
        }
    }
    pub fn field(self, key: &str) -> Result<Self> {
        self.index(Value::Text(key))
    }
    /// Raw finite table lookup. Text receivers use the pinned standard string
    /// library's absent fields; reached method values remain an explicit frontier.
    pub fn index(self, key: Value<'_>) -> Result<Self> {
        match self {
            Self::Table(table) => table.index(key),
            Self::Text(_) => match key {
                // These structural lookups are absent from the pinned standard
                // and Common-extended string tables. No arbitrary text lookup
                // is treated as absent: injected names can denote functions.
                Value::Text(
                    "type" | "base" | "baseName" | "subType" | "tags" | "rarity" | "selItemId",
                ) => Ok(Self::Nil),
                Value::Text(_) => Err(AssemblyError::unsupported(
                    "slot validity requires an unrepresented string-library field",
                )),
                _ => Ok(Self::Nil),
            },
            _ => Err(AssemblyError::source(
                "slot validity attempted to index a non-table value",
            )),
        }
    }
}
fn finite(value: f64) -> Result<Value<'static>> {
    if value.is_finite() {
        Ok(Value::Number(value))
    } else {
        Err(AssemblyError::unsupported("nonfinite slot-validity value"))
    }
}
impl<'a> Table<'a> {
    pub fn len(self) -> usize {
        match self.0 {
            Storage::Item(o, id) => o.table(id).map_or(0, |t| t.fields.len() + t.indexed.len()),
            Storage::Metadata(t) => t.fields.len() + t.indexed.len(),
            Storage::Array(v) => v.len(),
        }
    }
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }
    pub fn same_identity(self, other: Self) -> bool {
        match (self.0, other.0) {
            (Storage::Item(a, x), Storage::Item(b, y)) => a.shares_storage_with(b) && x == y,
            (Storage::Metadata(a), Storage::Metadata(b)) => std::ptr::eq(a, b),
            (Storage::Array(a), Storage::Array(b)) => std::ptr::eq(a, b),
            _ => false,
        }
    }
    fn index(self, key: Value<'_>) -> Result<Value<'a>> {
        let integer = match key {
            Value::Number(v)
                if v.is_finite() && v.fract() == 0.0 && v.abs() <= 9_007_199_254_740_991.0 =>
            {
                Some(v as i64)
            }
            _ => None,
        };
        match self.0 {
            Storage::Item(o, id) => {
                let t = o
                    .table(id)
                    .ok_or_else(|| AssemblyError::unsupported("invalid borrowed item table"))?;
                let v = match key {
                    Value::Text(k) => t.fields.get(k),
                    _ => integer.and_then(|k| t.indexed.get(&k)),
                };
                v.map_or(Ok(Value::Nil), |v| Value::assembled(o, v))
            }
            Storage::Metadata(t) => {
                let v = match key {
                    Value::Text(k) => t.fields.get(k),
                    _ => integer.and_then(|k| t.indexed.get(&k)),
                };
                v.map_or(Ok(Value::Nil), Value::metadata)
            }
            Storage::Array(v) => integer
                .and_then(|k| usize::try_from(k).ok())
                .and_then(|k| k.checked_sub(1))
                .and_then(|k| v.get(k))
                .map_or(Ok(Value::Nil), Value::metadata),
        }
    }
    /// Raw finite entries, for bounded diagnostics. Order is not a Lua `next` claim.
    pub fn entries(self) -> Result<Entries<'a>> {
        Ok(match self.0 {
            Storage::Item(o, id) => {
                let t = o
                    .table(id)
                    .ok_or_else(|| AssemblyError::unsupported("invalid borrowed item table"))?;
                Entries::Item(o, t.fields.iter(), t.indexed.iter())
            }
            Storage::Metadata(t) => Entries::Metadata(t.fields.iter(), t.indexed.iter()),
            Storage::Array(v) => Entries::Array(v.iter().enumerate()),
        })
    }
}
pub enum Entries<'a> {
    Item(
        &'a AssembledItem,
        btree_map::Iter<'a, String, AssemblyValue>,
        btree_map::Iter<'a, i64, AssemblyValue>,
    ),
    Metadata(
        btree_map::Iter<'a, String, ItemMetadataValue>,
        btree_map::Iter<'a, i64, ItemMetadataValue>,
    ),
    Array(std::iter::Enumerate<std::slice::Iter<'a, ItemMetadataValue>>),
}
impl<'a> Iterator for Entries<'a> {
    type Item = Result<(Key<'a>, Value<'a>)>;
    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        match self {
            Self::Item(o, f, i) => f
                .next()
                .map(|(k, v)| (Key::Text(k), v))
                .or_else(|| i.next().map(|(k, v)| (Key::Index(*k), v)))
                .map(|(k, v)| Value::assembled(o, v).map(|v| (k, v))),
            Self::Metadata(f, i) => f
                .next()
                .map(|(k, v)| (Key::Text(k), v))
                .or_else(|| i.next().map(|(k, v)| (Key::Index(*k), v)))
                .map(|(k, v)| Value::metadata(v).map(|v| (k, v))),
            Self::Array(i) => i
                .next()
                .map(|(k, v)| Value::metadata(v).map(|v| (Key::Index(k as i64 + 1), v))),
        }
    }
}
