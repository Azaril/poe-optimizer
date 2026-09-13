//! Private bounded storage for fixed item-set records; no executable source IR.
use super::{ItemSetLimits, ItemSetUsage, Result};
use crate::item_loading::assembly::AssemblyError;
use poe_optimizer_engine::source_program::{
    ProgramTable, ProgramTableId as Id, ProgramValue as V, ProgramValueGraph,
};
use std::mem::size_of;

pub(super) struct Graph {
    pub tables: Vec<ProgramTable>,
    pub usage: ItemSetUsage,
    pub limits: ItemSetLimits,
}
impl Graph {
    pub fn new(limits: ItemSetLimits) -> Self {
        Self {
            tables: Vec::new(),
            usage: ItemSetUsage::default(),
            limits,
        }
    }
    pub fn charge(&mut self, bytes: usize, steps: u64) -> Result<()> {
        let next_bytes = self
            .usage
            .bytes
            .checked_add(bytes)
            .ok_or_else(|| AssemblyError::resource("item-set byte overflow"))?;
        let next_steps = self
            .usage
            .steps
            .checked_add(steps)
            .ok_or_else(|| AssemblyError::resource("item-set work overflow"))?;
        if next_bytes > self.limits.max_bytes || next_steps > self.limits.max_steps {
            return Err(AssemblyError::resource(
                "item-set construction byte/work bound",
            ));
        }
        self.usage.bytes = next_bytes;
        self.usage.steps = next_steps;
        Ok(())
    }
    pub fn table(&mut self) -> Result<Id> {
        if self.tables.len() >= self.limits.max_tables || self.tables.len() >= u32::MAX as usize {
            return Err(AssemblyError::resource("item-set table bound"));
        }
        self.charge(size_of::<ProgramTable>(), 1)?;
        let id = Id(self.tables.len() as u32 + 1);
        self.tables.push(ProgramTable::default());
        self.usage.tables += 1;
        Ok(id)
    }
    pub fn text(&mut self, text: &str) -> Result<V> {
        if text.len() > self.limits.max_string_bytes {
            return Err(AssemblyError::resource("item-set string bound"));
        }
        self.charge(text.len(), 1)?;
        Ok(V::Bytes(text.as_bytes().to_vec()))
    }
    pub fn optional_text(&mut self, text: Option<&str>) -> Result<V> {
        text.map(|v| self.text(v)).unwrap_or(Ok(V::Nil))
    }
    fn entries(&self, id: Id) -> Result<&[(V, V)]> {
        self.tables
            .get(id.0.checked_sub(1).unwrap_or(u32::MAX) as usize)
            .map(|t| t.entries.as_slice())
            .ok_or_else(|| AssemblyError::unsupported("foreign item-set table"))
    }
    pub fn raw_field(&self, id: Id, field: &str) -> Option<&V> {
        self.tables
            .get(id.0.checked_sub(1)? as usize)?
            .entries
            .iter()
            .find_map(|(key, value)| {
                matches!(key,V::Bytes(bytes) if bytes==field.as_bytes()).then_some(value)
            })
    }
    pub fn field(&mut self, id: Id, field: &str) -> Result<V> {
        let bytes = match self.raw_field(id, field) {
            Some(V::Bytes(bytes)) => bytes.len(),
            _ => 0,
        };
        self.charge(
            bytes,
            self.entries(id)?
                .len()
                .saturating_mul(field.len().saturating_add(1))
                .saturating_mul(2) as u64
                + 1,
        )?;
        Ok(self.raw_field(id, field).cloned().unwrap_or(V::Nil))
    }
    pub fn number(&mut self, id: Id, key: f64) -> Result<V> {
        let bytes = self
            .entries(id)?
            .iter()
            .find_map(|(k, v)| matches!(k,V::Number(n) if *n==key).then_some(v))
            .map_or(0, |v| {
                if let V::Bytes(bytes) = v {
                    bytes.len()
                } else {
                    0
                }
            });
        self.charge(bytes, self.entries(id)?.len() as u64 * 2 + 1)?;
        Ok(self
            .entries(id)?
            .iter()
            .find_map(|(k, v)| matches!(k,V::Number(n) if *n==key).then_some(v))
            .cloned()
            .unwrap_or(V::Nil))
    }
    pub fn copy_entries(&mut self, id: Id) -> Result<Vec<(V, V)>> {
        let entries = self.entries(id)?;
        let bytes = entries
            .iter()
            .fold(
                entries.len().checked_mul(size_of::<(V, V)>()),
                |sum, (k, v)| {
                    sum.and_then(|n| n.checked_add(if let V::Bytes(b) = k { b.len() } else { 0 }))
                        .and_then(|n| n.checked_add(if let V::Bytes(b) = v { b.len() } else { 0 }))
                },
            )
            .ok_or_else(|| AssemblyError::resource("item-set copied entries overflow"))?;
        let steps = entries.len() as u64;
        self.charge(bytes, steps)?;
        Ok(self.entries(id)?.to_vec())
    }
    pub fn set_field(&mut self, id: Id, field: &str, value: V) -> Result<()> {
        let key = self.text(field)?;
        self.set(id, key, value)
    }
    pub fn set(&mut self, id: Id, key: V, value: V) -> Result<()> {
        // Lua numeric key equality merges both zeros; stored scalar values keep their bits.
        let key = match key {
            V::Number(n) if n == 0.0 && n.is_sign_negative() => V::Number(0.0),
            other => other,
        };
        match &key {
            V::Nil => return Err(AssemblyError::source("table index is nil")),
            V::Number(n) if n.is_nan() => return Err(AssemblyError::source("table index is NaN")),
            V::Number(_) | V::Bytes(_) => {}
            _ => {
                return Err(AssemblyError::unsupported(
                    "item-set table-valued key requires identity-key storage",
                ));
            }
        }
        let len = self.entries(id)?.len();
        let key_bytes = if let V::Bytes(bytes) = &key {
            bytes.len()
        } else {
            0
        };
        self.charge(0, (len as u64).saturating_mul(key_bytes as u64 + 1) + 1)?;
        let at = self.entries(id)?.iter().position(|(k, _)| equal(k, &key));
        if matches!(value, V::Nil) {
            if let Some(at) = at {
                self.tables[id.0 as usize - 1].entries.remove(at);
            }
            return Ok(());
        }
        if self.usage.values >= self.limits.max_values {
            return Err(AssemblyError::resource("item-set value bound"));
        }
        self.charge(size_of::<(V, V)>(), 1)?;
        self.usage.values += 1;
        let entries = &mut self.tables[id.0 as usize - 1].entries;
        if let Some(at) = at {
            entries[at].1 = value
        } else {
            entries.push((key, value))
        }
        Ok(())
    }
    pub fn append(&mut self, id: Id, value: V) -> Result<()> {
        let next = self.entries(id)?.len() + 1;
        self.set(id, V::Number(next as f64), value)
    }
    /// Diagnostic only: copy just reachable tables, preserving all aliases and
    /// IEEE values. This never imports state or certifies producer completion.
    pub fn snapshot(&self, root: Id) -> Result<ProgramValueGraph> {
        let mut output = ProgramValueGraph::default();
        let mut ids = vec![None; self.tables.len()];
        let mut pending = Vec::new();
        fn remap(
            v: &V,
            ids: &mut [Option<Id>],
            pending: &mut Vec<Id>,
            out: &mut ProgramValueGraph,
        ) -> Result<V> {
            if let V::Table(id) = v {
                let slot = ids
                    .get_mut(id.0.checked_sub(1).unwrap_or(u32::MAX) as usize)
                    .ok_or_else(|| {
                        AssemblyError::unsupported("invalid item-set diagnostic reference")
                    })?;
                let mapped = if let Some(mapped) = slot {
                    *mapped
                } else {
                    let mapped = Id(out.tables.len() as u32 + 1);
                    *slot = Some(mapped);
                    out.tables.push(ProgramTable::default());
                    pending.push(*id);
                    mapped
                };
                Ok(V::Table(mapped))
            } else {
                Ok(v.clone())
            }
        }
        let value = remap(&V::Table(root), &mut ids, &mut pending, &mut output)?;
        output.values.push(value);
        let mut index = 0;
        while index < pending.len() {
            let original = pending[index];
            index += 1;
            let target = ids[original.0 as usize - 1].unwrap();
            let mut entries = Vec::new();
            for (key, value) in self.entries(original)? {
                entries.push((
                    remap(key, &mut ids, &mut pending, &mut output)?,
                    remap(value, &mut ids, &mut pending, &mut output)?,
                ));
            }
            output.tables[target.0 as usize - 1].entries = entries;
        }
        Ok(output)
    }
}
pub(super) fn equal(a: &V, b: &V) -> bool {
    match (a, b) {
        (V::Nil, V::Nil) => true,
        (V::Boolean(a), V::Boolean(b)) => a == b,
        (V::Number(a), V::Number(b)) => a == b,
        (V::Bytes(a), V::Bytes(b)) => a == b,
        (V::Table(a), V::Table(b)) => a == b,
        _ => false,
    }
}
pub(super) fn table(v: V) -> Result<Id> {
    match v {
        V::Table(id) => Ok(id),
        V::Nil => Err(AssemblyError::source("attempt to index a nil value")),
        V::Boolean(_) => Err(AssemblyError::source("attempt to index a boolean value")),
        V::Number(_) => Err(AssemblyError::source("attempt to index a number value")),
        V::Bytes(_) => Err(AssemblyError::source("attempt to index a string value")),
        _ => Err(AssemblyError::unsupported(
            "unsupported item-set record value",
        )),
    }
}
pub(super) fn truthy(v: &V) -> bool {
    !matches!(v, V::Nil | V::Boolean(false))
}
