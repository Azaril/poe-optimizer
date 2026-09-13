//! Finite owned item values. Graph IDs preserve identity inside one artifact;
//! metadata ingress cannot recover aliases erased by its source tree adapter.
use super::super::machine::AssemblyBinding;
use poe_optimizer_data::item_loading::{ItemMetadataTable, ItemMetadataValue};
use poe_optimizer_engine::source_program::{
    ProgramTable, ProgramTableId, ProgramValue, ProgramValueGraph,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    mem::size_of,
    sync::Arc,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AssemblyTableId(pub u32);
#[derive(Debug, Clone, PartialEq)]
pub enum AssemblyValue {
    Nil,
    Boolean(bool),
    Number(f64),
    Text(String),
    Table(AssemblyTableId),
}
impl AssemblyValue {
    pub fn truthy(&self) -> bool {
        !matches!(self, Self::Nil | Self::Boolean(false))
    }
    pub fn as_table(&self) -> Option<AssemblyTableId> {
        if let Self::Table(id) = self {
            Some(*id)
        } else {
            None
        }
    }
    pub fn as_number(&self) -> Option<f64> {
        if let Self::Number(n) = self {
            Some(*n)
        } else {
            None
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        if let Self::Text(s) = self {
            Some(s)
        } else {
            None
        }
    }
    fn text_bytes(&self) -> usize {
        if let Self::Text(s) = self { s.len() } else { 0 }
    }
}
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AssemblyTable {
    pub fields: BTreeMap<String, AssemblyValue>,
    pub indexed: BTreeMap<i64, AssemblyValue>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssemblyErrorKind {
    Unsupported,
    Source,
    Resource,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssemblyError {
    pub kind: AssemblyErrorKind,
    pub message: String,
}
impl AssemblyError {
    pub fn unsupported(message: impl Into<String>) -> Self {
        Self {
            kind: AssemblyErrorKind::Unsupported,
            message: message.into(),
        }
    }
    pub fn source(message: impl Into<String>) -> Self {
        Self {
            kind: AssemblyErrorKind::Source,
            message: message.into(),
        }
    }
    pub fn resource(message: impl Into<String>) -> Self {
        Self {
            kind: AssemblyErrorKind::Resource,
            message: message.into(),
        }
    }
}
impl std::fmt::Display for AssemblyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for AssemblyError {}
pub(crate) type Result<T> = std::result::Result<T, AssemblyError>;
pub(crate) type Value = AssemblyValue;
pub(crate) type TableId = AssemblyTableId;

#[derive(Debug, Clone, Copy)]
pub struct AssemblyLimits {
    pub max_tables: usize,
    pub max_values: usize,
    pub max_bytes: usize,
    pub max_steps: u64,
    pub max_depth: usize,
}
impl Default for AssemblyLimits {
    fn default() -> Self {
        Self {
            max_tables: 65_536,
            max_values: 1_000_000,
            max_bytes: 32 * 1024 * 1024,
            max_steps: 10_000_000,
            max_depth: 64,
        }
    }
}
/// Cumulative logical construction/copy/work charges, not allocator bytes or RSS.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AssemblyUsage {
    pub tables: usize,
    pub values: usize,
    pub bytes: usize,
    pub steps: u64,
}
#[derive(Debug)]
struct OwnedGraph {
    tables: Vec<AssemblyTable>,
    root: TableId,
    usage: AssemblyUsage,
    complete: bool,
    binding: Option<AssemblyBinding>,
}
/// Immutable owned payload, which can also describe an explicitly labelled
/// failure prefix. Possession alone does not grant completion or numerical admission.
#[derive(Debug, Clone)]
pub struct AssembledItem(Arc<OwnedGraph>);
impl AssembledItem {
    pub fn root(&self) -> TableId {
        self.0.root
    }
    pub fn is_complete(&self) -> bool {
        self.0.complete
    }
    pub(crate) fn binding(&self) -> Option<&AssemblyBinding> {
        self.0.binding.as_ref()
    }
    pub fn tables(&self) -> &[AssemblyTable] {
        &self.0.tables
    }
    pub fn table(&self, id: TableId) -> Option<&AssemblyTable> {
        id.0.checked_sub(1)
            .and_then(|i| self.0.tables.get(i as usize))
    }
    pub fn field(&self, id: TableId, key: &str) -> Option<&Value> {
        self.table(id)?.fields.get(key)
    }
    pub fn index(&self, id: TableId, key: i64) -> Option<&Value> {
        self.table(id)?.indexed.get(&key)
    }
    pub fn usage(&self) -> AssemblyUsage {
        self.0.usage
    }
    pub fn shares_storage_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
    /// A bounded ordinary graph projection. It claims neither Lua behavior nor
    /// source traversal order. No callback descriptors enter this value domain.
    pub fn snapshot(&self) -> Result<ProgramValueGraph> {
        self.snapshot_with_limits(AssemblyLimits::default())
    }
    pub fn snapshot_with_limits(&self, limits: AssemblyLimits) -> Result<ProgramValueGraph> {
        let mut check = Arena::new(limits);
        check.charge_graph(&self.0.tables)?;
        fn value(v: &Value) -> ProgramValue {
            match v {
                Value::Nil => ProgramValue::Nil,
                Value::Boolean(v) => ProgramValue::Boolean(*v),
                Value::Number(v) => ProgramValue::Number(*v),
                Value::Text(v) => ProgramValue::Bytes(v.as_bytes().to_vec()),
                Value::Table(id) => ProgramValue::Table(ProgramTableId(id.0)),
            }
        }
        Ok(ProgramValueGraph {
            values: vec![ProgramValue::Table(ProgramTableId(self.root().0))],
            tables: self
                .tables()
                .iter()
                .map(|t| ProgramTable {
                    entries: t
                        .fields
                        .iter()
                        .map(|(k, v)| (ProgramValue::Bytes(k.as_bytes().to_vec()), value(v)))
                        .chain(
                            t.indexed
                                .iter()
                                .map(|(k, v)| (ProgramValue::Number(*k as f64), value(v))),
                        )
                        .collect(),
                })
                .collect(),
        })
    }
}

pub(crate) struct Arena {
    tables: Vec<AssemblyTable>,
    limits: AssemblyLimits,
    usage: AssemblyUsage,
    binding: Option<AssemblyBinding>,
}
impl Arena {
    pub(crate) fn new(limits: AssemblyLimits) -> Self {
        Self {
            tables: Vec::new(),
            limits,
            usage: AssemblyUsage::default(),
            binding: None,
        }
    }
    pub(crate) fn from_item(item: &AssembledItem, limits: AssemblyLimits) -> Result<Self> {
        let mut arena = Self::new(limits);
        arena.charge_graph(item.tables())?;
        arena.tables = item.tables().to_vec();
        arena.binding = item.binding().cloned();
        Ok(arena)
    }
    pub(crate) fn usage(&self) -> AssemblyUsage {
        self.usage
    }
    pub(crate) fn work(&mut self, steps: u64) -> Result<()> {
        self.charge_cost(0, 0, 0, steps)
    }
    pub(crate) fn reserve_bytes(&mut self, bytes: usize) -> Result<()> {
        self.charge_cost(0, 0, bytes, 0)
    }
    pub(crate) fn text(&mut self, text: &str) -> Result<Value> {
        self.charge_cost(0, 1, text.len(), 1)?;
        Ok(Value::Text(text.to_owned()))
    }
    fn charge_cost(
        &mut self,
        tables: usize,
        values: usize,
        bytes: usize,
        steps: u64,
    ) -> Result<()> {
        let fail = || AssemblyError::resource("item assembly cumulative resource bound");
        let next = AssemblyUsage {
            tables: self.usage.tables.checked_add(tables).ok_or_else(fail)?,
            values: self.usage.values.checked_add(values).ok_or_else(fail)?,
            bytes: self.usage.bytes.checked_add(bytes).ok_or_else(fail)?,
            steps: self.usage.steps.checked_add(steps).ok_or_else(fail)?,
        };
        if next.tables > self.limits.max_tables
            || next.values > self.limits.max_values
            || next.bytes > self.limits.max_bytes
            || next.steps > self.limits.max_steps
        {
            return Err(fail());
        }
        self.usage = next;
        Ok(())
    }
    fn charge_graph(&mut self, tables: &[AssemblyTable]) -> Result<()> {
        // No graph clone is performed before this complete storage preflight.
        self.charge_cost(
            tables.len(),
            tables.len(),
            tables
                .len()
                .checked_mul(size_of::<AssemblyTable>())
                .and_then(|n| n.checked_add(size_of::<OwnedGraph>() + 2 * size_of::<usize>()))
                .ok_or_else(|| AssemblyError::resource("item graph bytes"))?,
            tables.len() as u64,
        )?;
        for table in tables {
            for (key, value) in &table.fields {
                self.charge_cost(0, 2, entry_bytes(key.len(), value)?, 1)?;
            }
            for value in table.indexed.values() {
                self.charge_cost(0, 2, entry_bytes(0, value)?, 1)?;
            }
        }
        Ok(())
    }
    pub(crate) fn table(&self, id: TableId) -> Result<&AssemblyTable> {
        id.0.checked_sub(1)
            .and_then(|i| self.tables.get(i as usize))
            .ok_or_else(|| AssemblyError::source("invalid item assembly table reference"))
    }
    fn table_mut(&mut self, id: TableId) -> Result<&mut AssemblyTable> {
        id.0.checked_sub(1)
            .and_then(|i| self.tables.get_mut(i as usize))
            .ok_or_else(|| AssemblyError::source("invalid item assembly table reference"))
    }
    pub(crate) fn new_table(&mut self) -> Result<TableId> {
        let id = u32::try_from(self.tables.len())
            .ok()
            .and_then(|n| n.checked_add(1))
            .ok_or_else(|| AssemblyError::resource("item assembly table identity"))?;
        let wrapper = if self.tables.is_empty() {
            size_of::<OwnedGraph>() + 2 * size_of::<usize>()
        } else {
            0
        };
        self.charge_cost(1, 1, size_of::<AssemblyTable>() + wrapper, 1)?;
        self.tables.push(AssemblyTable::default());
        Ok(AssemblyTableId(id))
    }
    fn validate_value(&self, value: &Value) -> Result<()> {
        match value {
            Value::Number(n) if !n.is_finite() => {
                Err(AssemblyError::unsupported("nonfinite item assembly value"))
            }
            Value::Table(id) => self.table(*id).map(|_| ()),
            _ => Ok(()),
        }
    }
    pub(crate) fn get_field(&mut self, id: TableId, key: &str) -> Result<Value> {
        let steps = search_work(self.table(id)?.fields.len(), key.len())?;
        self.work(steps)?;
        let bytes = self.table(id)?.fields.get(key).map_or(0, Value::text_bytes);
        self.charge_cost(0, 1, bytes, 0)?;
        Ok(self
            .table(id)?
            .fields
            .get(key)
            .cloned()
            .unwrap_or(Value::Nil))
    }
    pub(crate) fn get_index(&mut self, id: TableId, key: i64) -> Result<Value> {
        let steps = search_work(self.table(id)?.indexed.len(), 0)?;
        self.work(steps)?;
        let bytes = self
            .table(id)?
            .indexed
            .get(&key)
            .map_or(0, Value::text_bytes);
        self.charge_cost(0, 1, bytes, 0)?;
        Ok(self
            .table(id)?
            .indexed
            .get(&key)
            .cloned()
            .unwrap_or(Value::Nil))
    }
    pub(crate) fn set_field(&mut self, id: TableId, key: &str, value: Value) -> Result<()> {
        self.validate_value(&value)?;
        let steps = search_work(self.table(id)?.fields.len(), key.len())?;
        self.charge_cost(0, 2, entry_bytes(key.len(), &value)?, steps)?;
        if matches!(value, Value::Nil) {
            self.table_mut(id)?.fields.remove(key);
        } else {
            self.table_mut(id)?.fields.insert(key.to_owned(), value);
        }
        Ok(())
    }
    pub(crate) fn set_index(&mut self, id: TableId, key: i64, value: Value) -> Result<()> {
        // Numeric source keys must survive the later f64 raw-graph projection exactly.
        if (key as f64) as i128 != key as i128 {
            return Err(AssemblyError::unsupported(
                "integer item key is not exactly representable",
            ));
        }
        self.validate_value(&value)?;
        let steps = search_work(self.table(id)?.indexed.len(), 0)?;
        self.charge_cost(0, 2, entry_bytes(0, &value)?, steps)?;
        if matches!(value, Value::Nil) {
            self.table_mut(id)?.indexed.remove(&key);
        } else {
            self.table_mut(id)?.indexed.insert(key, value);
        }
        Ok(())
    }
    pub(crate) fn dense_len(&mut self, id: TableId) -> Result<usize> {
        let n = self.table(id)?.indexed.len();
        self.work(n as u64 + 1)?;
        if !self.table(id)?.indexed.keys().copied().eq(1..=n as i64) {
            return Err(AssemblyError::unsupported(
                "item assembly list has sparse or nonpositive indices",
            ));
        }
        Ok(n)
    }
    pub(crate) fn append(&mut self, id: TableId, value: Value) -> Result<()> {
        let n = self.dense_len(id)?;
        self.set_index(id, n as i64 + 1, value)
    }
    pub(crate) fn remove(&mut self, id: TableId, index: usize) -> Result<Value> {
        let n = self.dense_len(id)?;
        if index == 0 || index > n {
            return Ok(Value::Nil);
        }
        self.charge_cost(0, n - index + 1, 0, (n - index + 1) as u64)?;
        let table = self.table_mut(id)?;
        let value = table
            .indexed
            .remove(&(index as i64))
            .expect("validated dense list");
        for i in index..n {
            let next = table
                .indexed
                .remove(&(i as i64 + 1))
                .expect("validated dense list");
            table.indexed.insert(i as i64, next);
        }
        Ok(value)
    }
    pub(crate) fn import_metadata(&mut self, table: &ItemMetadataTable) -> Result<TableId> {
        self.import_table_at(table, 0)
    }
    #[cfg(test)]
    pub(crate) fn import_value(&mut self, value: &ItemMetadataValue) -> Result<Value> {
        self.import_value_at(value, 0)
    }
    fn import_table_at(&mut self, table: &ItemMetadataTable, depth: usize) -> Result<TableId> {
        self.depth(depth)?;
        let id = self.new_table()?;
        for (key, value) in &table.fields {
            let value = self.import_value_at(value, depth + 1)?;
            self.set_field(id, key, value)?;
        }
        for (key, value) in &table.indexed {
            let value = self.import_value_at(value, depth + 1)?;
            self.set_index(id, *key, value)?;
        }
        Ok(id)
    }
    fn import_value_at(&mut self, value: &ItemMetadataValue, depth: usize) -> Result<Value> {
        self.depth(depth)?;
        self.work(1)?;
        Ok(match value {
            ItemMetadataValue::Boolean(v) => Value::Boolean(*v),
            ItemMetadataValue::Number(n) if n.is_finite() => Value::Number(*n),
            ItemMetadataValue::Number(_) => {
                return Err(AssemblyError::unsupported(
                    "nonfinite metadata requires a richer assembly value model",
                ));
            }
            ItemMetadataValue::Text(s) => {
                self.charge_cost(0, 1, s.len(), 0)?;
                Value::Text(s.clone())
            }
            ItemMetadataValue::Table(t) => Value::Table(self.import_table_at(t, depth)?),
            ItemMetadataValue::Array(values) => {
                let id = self.new_table()?;
                for (i, v) in values.iter().enumerate() {
                    let value = self.import_value_at(v, depth + 1)?;
                    self.set_index(id, i as i64 + 1, value)?;
                }
                Value::Table(id)
            }
            ItemMetadataValue::Callback(_) => {
                return Err(AssemblyError::unsupported(
                    "opaque item callback descriptor has no executable captured-state owner",
                ));
            }
        })
    }
    fn depth(&self, depth: usize) -> Result<()> {
        if depth > self.limits.max_depth || depth > 256 {
            Err(AssemblyError::resource("item assembly graph copy depth"))
        } else {
            Ok(())
        }
    }
    pub(crate) fn deep_copy(&mut self, value: &Value) -> Result<Value> {
        self.copy_at(value, 0, &mut BTreeSet::new())
    }
    fn copy_at(
        &mut self,
        value: &Value,
        depth: usize,
        path: &mut BTreeSet<TableId>,
    ) -> Result<Value> {
        self.depth(depth)?;
        self.validate_value(value)?;
        self.charge_cost(0, 1, value.text_bytes(), 1)?;
        let Value::Table(id) = value else {
            return Ok(value.clone());
        };
        self.charge_cost(0, 1, size_of::<TableId>() + 32, 0)?;
        if !path.insert(*id) {
            return Err(AssemblyError::unsupported(
                "recursive item table copy reaches a cycle",
            ));
        }
        // Retain only owned row keys between recursive writes; clones are precharged.
        let (field_count, index_count, key_bytes) = {
            let t = self.table(*id)?;
            (
                t.fields.len(),
                t.indexed.len(),
                t.fields
                    .keys()
                    .try_fold(0usize, |n, k| n.checked_add(k.len()))
                    .ok_or_else(|| AssemblyError::resource("item copy key bytes"))?,
            )
        };
        self.charge_cost(
            0,
            field_count + index_count,
            key_bytes
                .checked_add(
                    (field_count + index_count)
                        .checked_mul(size_of::<i64>() + size_of::<String>())
                        .ok_or_else(|| AssemblyError::resource("item copy row bytes"))?,
                )
                .ok_or_else(|| AssemblyError::resource("item copy row bytes"))?,
            (field_count + index_count) as u64,
        )?;
        let fields = self.table(*id)?.fields.keys().cloned().collect::<Vec<_>>();
        let indices = self.table(*id)?.indexed.keys().copied().collect::<Vec<_>>();
        let out = self.new_table()?;
        for key in fields {
            let v = self.get_field(*id, &key)?;
            let copy = self.copy_at(&v, depth + 1, path)?;
            self.set_field(out, &key, copy)?;
        }
        for key in indices {
            let v = self.get_index(*id, key)?;
            let copy = self.copy_at(&v, depth + 1, path)?;
            self.set_index(out, key, copy)?;
        }
        path.remove(id);
        Ok(Value::Table(out))
    }
    /// Only the assembly algorithm can seal a fully completed result.
    pub(super) fn finish_complete(
        self,
        root: TableId,
        binding: AssemblyBinding,
    ) -> Result<AssembledItem> {
        self.table(root)?;
        Ok(AssembledItem(Arc::new(OwnedGraph {
            tables: self.tables,
            root,
            usage: self.usage,
            complete: true,
            binding: Some(binding),
        })))
    }
    pub(super) fn finish_partial(
        self,
        root: TableId,
        binding: AssemblyBinding,
    ) -> Result<AssembledItem> {
        self.table(root)?;
        Ok(AssembledItem(Arc::new(OwnedGraph {
            tables: self.tables,
            root,
            usage: self.usage,
            complete: false,
            binding: Some(binding),
        })))
    }
    /// Wrapper storage was charged when the first table was admitted, so even an
    /// exhausted work/storage budget can retain its already-reached prefix.
    #[cfg(test)]
    pub(crate) fn finish(self, root: TableId) -> Result<AssembledItem> {
        self.table(root)?;
        Ok(AssembledItem(Arc::new(OwnedGraph {
            tables: self.tables,
            root,
            usage: self.usage,
            complete: false,
            binding: self.binding,
        })))
    }
}
fn entry_bytes(key_bytes: usize, value: &Value) -> Result<usize> {
    key_bytes
        .checked_add(value.text_bytes())
        .and_then(|n| n.checked_add(2 * size_of::<Value>() + 64))
        .ok_or_else(|| AssemblyError::resource("item assembly entry bytes"))
}
fn search_work(entries: usize, key_bytes: usize) -> Result<u64> {
    (entries as u64)
        .checked_add(1)
        .and_then(|n| n.checked_mul(key_bytes as u64 + 1))
        .ok_or_else(|| AssemblyError::resource("item assembly lookup work"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mutations_preserve_aliases_and_nil_removes_without_retargeting() {
        let mut a = Arena::new(AssemblyLimits::default());
        let root = a.new_table().unwrap();
        let child = a.new_table().unwrap();
        a.set_field(root, "a", Value::Table(child)).unwrap();
        a.set_field(root, "b", Value::Table(child)).unwrap();
        a.set_field(child, "n", Value::Number(4.)).unwrap();
        assert_eq!(
            a.get_field(root, "a").unwrap(),
            a.get_field(root, "b").unwrap()
        );
        a.set_field(child, "n", Value::Nil).unwrap();
        assert_eq!(a.get_field(child, "n").unwrap(), Value::Nil);
        let item = a.finish(root).unwrap();
        let cloned = item.clone();
        assert!(item.shares_storage_with(&cloned));
        assert_eq!(item.snapshot().unwrap().tables.len(), 2);
    }
    #[test]
    fn deep_copy_duplicates_each_occurrence_and_rejects_cycle_with_prefix_retained() {
        let mut a = Arena::new(AssemblyLimits::default());
        let root = a.new_table().unwrap();
        let child = a.new_table().unwrap();
        a.set_index(root, 1, Value::Table(child)).unwrap();
        a.set_index(root, 2, Value::Table(child)).unwrap();
        let copied = a
            .deep_copy(&Value::Table(root))
            .unwrap()
            .as_table()
            .unwrap();
        assert_ne!(
            a.get_index(copied, 1).unwrap(),
            a.get_index(copied, 2).unwrap()
        );
        a.set_field(root, "self", Value::Table(root)).unwrap();
        assert_eq!(
            a.deep_copy(&Value::Table(root)).unwrap_err().kind,
            AssemblyErrorKind::Unsupported
        );
        assert_eq!(a.get_field(root, "self").unwrap(), Value::Table(root));
        assert!(a.finish(root).is_ok());
    }
    #[test]
    fn bound_failure_keeps_reached_prefix_and_foreign_id_is_rejected() {
        let mut a = Arena::new(AssemblyLimits {
            max_tables: 1,
            ..AssemblyLimits::default()
        });
        let root = a.new_table().unwrap();
        a.set_field(root, "done", Value::Boolean(true)).unwrap();
        let before = a.usage();
        assert_eq!(a.new_table().unwrap_err().kind, AssemblyErrorKind::Resource);
        assert_eq!(a.usage(), before);
        assert!(
            a.set_field(root, "bad", Value::Table(AssemblyTableId(2)))
                .is_err()
        );
        assert_eq!(a.get_field(root, "done").unwrap(), Value::Boolean(true));
    }
    #[test]
    fn dense_remove_preserves_order_and_sparse_lists_reject() {
        let mut a = Arena::new(AssemblyLimits::default());
        let root = a.new_table().unwrap();
        for n in 1..=3 {
            a.append(root, Value::Number(n as f64)).unwrap();
        }
        assert_eq!(a.remove(root, 2).unwrap(), Value::Number(2.));
        assert_eq!(a.get_index(root, 2).unwrap(), Value::Number(3.));
        a.set_index(root, 4, Value::Number(4.)).unwrap();
        assert!(a.append(root, Value::Number(5.)).is_err());
    }
    #[test]
    fn private_restart_does_not_publish_mutable_aliases() {
        let mut a = Arena::new(AssemblyLimits::default());
        let root = a.new_table().unwrap();
        let child = a.new_table().unwrap();
        a.set_field(root, "left", Value::Table(child)).unwrap();
        a.set_field(root, "right", Value::Table(child)).unwrap();
        a.set_field(child, "n", Value::Number(-0.0)).unwrap();
        let partial = a.finish(root).unwrap();
        assert!(!partial.is_complete());
        let mut next = Arena::from_item(&partial, AssemblyLimits::default()).unwrap();
        next.set_field(child, "n", Value::Number(9.)).unwrap();
        let complete = next.finish(root).unwrap();
        assert!(!complete.is_complete());
        assert!(!partial.shares_storage_with(&complete));
        assert_eq!(
            partial
                .field(child, "n")
                .unwrap()
                .as_number()
                .unwrap()
                .to_bits(),
            (-0.0f64).to_bits()
        );
        assert_eq!(complete.field(child, "n"), Some(&Value::Number(9.)));
        assert_eq!(complete.field(root, "left"), complete.field(root, "right"));
    }
    #[test]
    fn opaque_callbacks_nonfinite_values_and_rounded_keys_are_not_admitted() {
        use poe_optimizer_data::item_loading::{ItemOpaqueFunction, ItemSourceSpan};
        let mut a = Arena::new(AssemblyLimits::default());
        let root = a.new_table().unwrap();
        a.set_field(root, "prefix", Value::Boolean(true)).unwrap();
        let opaque = ItemMetadataValue::Callback(ItemOpaqueFunction {
            callback: ItemSourceSpan {
                path: "src/item.lua".into(),
                line: 1,
                end_line: 2,
                sha256: "0".repeat(64),
            },
        });
        assert_eq!(
            a.import_value(&opaque).unwrap_err().kind,
            AssemblyErrorKind::Unsupported
        );
        assert_eq!(
            a.set_field(root, "bad", Value::Number(f64::NAN))
                .unwrap_err()
                .kind,
            AssemblyErrorKind::Unsupported
        );
        assert_eq!(
            a.set_index(root, 9_007_199_254_740_993, Value::Boolean(true))
                .unwrap_err()
                .kind,
            AssemblyErrorKind::Unsupported
        );
        assert_eq!(a.get_field(root, "prefix").unwrap(), Value::Boolean(true));
        assert_eq!(a.table(root).unwrap().fields.len(), 1);
    }
    #[test]
    fn snapshot_and_restart_preflight_refuse_small_limits_without_changing_source() {
        let mut a = Arena::new(AssemblyLimits::default());
        let root = a.new_table().unwrap();
        a.set_field(root, "text", Value::Text("x".repeat(1024)))
            .unwrap();
        let item = a.finish(root).unwrap();
        let tiny = AssemblyLimits {
            max_bytes: 64,
            ..AssemblyLimits::default()
        };
        assert!(item.snapshot_with_limits(tiny).is_err());
        assert!(Arena::from_item(&item, tiny).is_err());
        assert!(!item.is_complete());
        assert_eq!(
            item.field(root, "text").unwrap().as_str().unwrap().len(),
            1024
        );
        assert!(item.snapshot().is_ok());
    }
}
