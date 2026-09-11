//! Proven raw table coverage on the same identity heap, separate from values.
use super::{Error, Heap, Key, ProgramValueGraph, Result, TableRef, V, index, table_ref};
use poe_optimizer_data::source_program::{
    SourceTableCallFallback, SourceTableCoverage, SourceTableIndexFallback, SourceTableInventory,
    SourceTableKey,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// Coverage belongs to the one-based table IDs of one input graph. Omitted
/// records retain the existing complete plain-table transport contract. Metadata
/// obeys the shared source-key bounds; total imported graph storage remains
/// governed by ProgramLimits, including runtime key domains absent from source IR.
pub use poe_optimizer_data::source_program::SourceSessionCoverage as ProgramTableCoverage;

pub(super) struct Coverage {
    inventory: SourceTableInventory,
    known_absent: BTreeSet<Key>,
    unavailable: BTreeSet<Key>,
    index_fallback: SourceTableIndexFallback,
    call_fallback: SourceTableCallFallback,
}
impl Coverage {
    pub(super) fn conflicts(&self, key: &Key) -> bool {
        self.known_absent.contains(key) || self.unavailable.contains(key)
    }
    fn raw_absent(&self, key: &Key) -> bool {
        !self.unavailable.contains(key)
            && (self.inventory == SourceTableInventory::Complete || self.known_absent.contains(key))
    }
    fn complete(&self) -> bool {
        self.inventory == SourceTableInventory::Complete && self.unavailable.is_empty()
    }
}
fn source_key(key: &SourceTableKey) -> Key {
    match key {
        SourceTableKey::Text(value) => Key::Bytes(Arc::from(value.as_bytes())),
        SourceTableKey::Integer(value) => {
            Key::read(&V::Number(*value as f64)).expect("validated integer key")
        }
    }
}
impl Heap<'_> {
    fn convert_coverage(&mut self, source: &SourceTableCoverage) -> Result<Coverage> {
        // Charge before validation/cloning. Failed imports never replenish these
        // cumulative counters, and even unreachable coverage records are bounded.
        self.budget.values(1)?;
        for key in source.known_absent.iter().chain(&source.unavailable) {
            self.budget.values(1)?;
            if let SourceTableKey::Text(text) = key {
                self.budget.bytes(text.len())?;
            }
        }
        source
            .validate_shape()
            .map_err(|error| Error::input(error.to_string()))?;
        Ok(Coverage {
            inventory: source.inventory,
            known_absent: source.known_absent.iter().map(source_key).collect(),
            unavailable: source.unavailable.iter().map(source_key).collect(),
            index_fallback: source.index_fallback,
            call_fallback: source.call_fallback,
        })
    }
    pub(super) fn import_coverage(
        &mut self,
        input: &ProgramValueGraph,
        source: &ProgramTableCoverage,
        writable: bool,
        offset: u32,
    ) -> Result<BTreeMap<TableRef, Coverage>> {
        let mut out = BTreeMap::new();
        for (id, record) in source {
            let record = self.convert_coverage(record)?;
            input
                .tables
                .get(index(id.0)?)
                .ok_or_else(|| Error::input("coverage references a missing input table"))?;
            let reference = if writable {
                TableRef::Heap(offset + id.0)
            } else {
                TableRef::Argument(offset + id.0)
            };
            out.insert(reference, record);
        }
        Ok(out)
    }
    fn ensure_coverage(&mut self, reference: TableRef) -> Result<()> {
        if self.coverage.contains_key(&reference) {
            return Ok(());
        }
        if let TableRef::Definition(id) = reference
            && self.owner().table_coverage(id).is_some()
        {
            let owner = self.owner().clone();
            if let Some(source) = owner.table_coverage(id) {
                let coverage = self.convert_coverage(source)?;
                self.coverage.insert(reference, coverage);
            }
        }
        Ok(())
    }
    pub(super) fn ensure_raw_absence(&mut self, reference: TableRef, key: &Key) -> Result<()> {
        self.ensure_coverage(reference)?;
        if self
            .coverage
            .get(&reference)
            .is_some_and(|coverage| !coverage.raw_absent(key))
        {
            return Err(Error::unsupported("source table key/value is unavailable"));
        }
        Ok(())
    }
    pub(in crate::parser_program::runtime) fn ensure_index_fallback(
        &mut self,
        table: &V,
    ) -> Result<()> {
        let reference = table_ref(table)?;
        self.ensure_coverage(reference)?;
        if self.coverage.get(&reference).is_some_and(|coverage| {
            coverage.index_fallback == SourceTableIndexFallback::Unavailable
        }) {
            return Err(Error::unsupported(
                "source table index fallback is unavailable",
            ));
        }
        Ok(())
    }
    pub(in crate::parser_program::runtime) fn ensure_call_fallback(
        &mut self,
        table: &V,
    ) -> Result<()> {
        let reference = table_ref(table)?;
        self.ensure_coverage(reference)?;
        if self
            .coverage
            .get(&reference)
            .is_some_and(|coverage| coverage.call_fallback == SourceTableCallFallback::Unavailable)
        {
            return Err(Error::unsupported(
                "source table call fallback is unavailable",
            ));
        }
        Ok(())
    }
    pub(super) fn ensure_integer_inventory(&mut self, reference: TableRef) -> Result<()> {
        self.ensure_coverage(reference)?;
        if self.coverage.get(&reference).is_some_and(|coverage| {
            coverage.inventory != SourceTableInventory::Complete
                || coverage
                    .unavailable
                    .iter()
                    .any(|key| key.positive_integer().is_some())
        }) {
            return Err(Error::unsupported(
                "source table positive-integer inventory is unavailable",
            ));
        }
        Ok(())
    }
    pub(super) fn ensure_snapshot_coverage(&mut self, reference: TableRef) -> Result<()> {
        self.ensure_coverage(reference)?;
        if self.coverage.get(&reference).is_some_and(|coverage| {
            !coverage.complete()
                || coverage.index_fallback != SourceTableIndexFallback::Nil
                || coverage.call_fallback != SourceTableCallFallback::NonCallable
        }) {
            return Err(Error::unsupported(
                "snapshot would erase incomplete table coverage or metatable behavior",
            ));
        }
        Ok(())
    }
    pub(super) fn prepare_coverage_write(
        &mut self,
        reference: TableRef,
        key: &Key,
        value: &V,
    ) -> Result<()> {
        if matches!(value, V::Nil)
            && self.coverage.get(&reference).is_some_and(|coverage| {
                coverage.inventory == SourceTableInventory::Selective
                    && !coverage.known_absent.contains(key)
            })
        {
            // Keys hold shared string Arcs, so only the new set entry is charged.
            self.budget.values(1)?;
        }
        Ok(())
    }
    pub(super) fn coverage_written(&mut self, reference: TableRef, key: Key, deleted: bool) {
        if let Some(coverage) = self.coverage.get_mut(&reference) {
            coverage.unavailable.remove(&key);
            if deleted && coverage.inventory == SourceTableInventory::Selective {
                coverage.known_absent.insert(key);
            } else {
                coverage.known_absent.remove(&key);
            }
        }
    }
}
