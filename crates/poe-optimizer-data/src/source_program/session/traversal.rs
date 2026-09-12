//! Source-observed raw traversal/length facts, never inferred from entry order.
use super::*;
use crate::source_program::{SourceTableInventory, SourceTableKey};
use std::collections::BTreeSet;

/// Optional per-input observations. The enclosing input retains the owner;
/// every table/closure reference in an order is local to that same input.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SourceSessionTraversal {
    pub tables: BTreeMap<SourceSessionTableId, SourceSessionTableTraversal>,
}
/// A complete raw key permutation in observed `next` order, and optionally the
/// source's actual raw length. Equal unordered entries do not prove either fact.
/// These observations describe initial state only. Importing runtimes invalidate
/// both on structural writes unless an exact layout transition is modeled.
/// Existing non-nil value replacement can retain them and reads stay live.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SourceSessionTableTraversal {
    pub order: Vec<SourceSessionValue>,
    pub raw_length: Option<u32>,
}

// Borrow strings while validating: metadata does not require another byte copy.
// Numeric order is irrelevant here; bits canonicalize only Lua key equality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Key<'a> {
    Boolean(bool),
    Number(u64),
    Bytes(&'a [u8]),
    Table(SourceSessionTableId),
    Callback(SourceCallbackId),
    Closure(SourceSessionClosureId),
    DefinitionTable(SourceTableId),
}
fn invalid(message: &str) -> super::super::SourceProgramError {
    failure(SourceProgramErrorKind::Binding, message)
}
fn resource() -> super::super::SourceProgramError {
    failure(
        SourceProgramErrorKind::ResourceLimit,
        "session traversal metadata bound",
    )
}
impl SourceSessionInput {
    /// Check all sizes before allocating temporary normalized-key sets; callers
    /// charge this bounded work/storage against their cumulative import budget.
    /// Bytes are borrowed, never cloned. `max_keys` bounds each of the raw-entry
    /// and observation totals, not an individual table's size.
    ///
    /// Validity is structural, not source authentication. In particular multiple
    /// lengths/orders can fit one map. The observer authenticates actual layout;
    /// the engine separately validates/imports the whole graph and owner identity
    /// atomically, with all value and allocation bounds.
    pub fn validate_traversal(
        &self,
        max_tables: usize,
        max_keys: usize,
        max_bytes: usize,
    ) -> SourceProgramResult<()> {
        let Some(traversal) = &self.traversal else {
            return Ok(());
        };
        if self.state.tables.len() > max_tables || traversal.tables.len() > max_tables {
            return Err(resource());
        }
        let mut raw_count = 0usize;
        let mut order_count = 0usize;
        let mut bytes = 0usize;
        // Preflight the entire facet before any normalized sets are allocated.
        for (id, observation) in &traversal.tables {
            let table = self.session_table(*id)?;
            raw_count = raw_count
                .checked_add(table.entries.len())
                .ok_or_else(resource)?;
            order_count = order_count
                .checked_add(observation.order.len())
                .ok_or_else(resource)?;
            if raw_count > max_keys || order_count > max_keys {
                return Err(resource());
            }
            for key in table
                .entries
                .iter()
                .map(|(key, _)| key)
                .chain(&observation.order)
            {
                if let SourceSessionValue::Bytes(value) = key {
                    bytes = bytes.checked_add(value.len()).ok_or_else(resource)?;
                    if bytes > max_bytes {
                        return Err(resource());
                    }
                }
            }
        }
        for (id, observation) in &traversal.tables {
            let table = self.session_table(*id)?;
            if let Some(coverage) = self.coverage.get(id) {
                coverage.validate_shape()?;
                if coverage.inventory != SourceTableInventory::Complete
                    || !coverage.unavailable.is_empty()
                {
                    return Err(invalid(
                        "session traversal requires complete available raw inventory",
                    ));
                }
            }
            let mut raw = BTreeSet::new();
            for (key, value) in &table.entries {
                if matches!(value, SourceSessionValue::Nil) {
                    return Err(invalid("session traversal raw entry has nil value"));
                }
                let key = self.traversal_key(key)?;
                if !raw.insert(key) {
                    return Err(invalid(
                        "session traversal raw keys duplicate after Lua normalization",
                    ));
                }
            }
            if let Some(coverage) = self.coverage.get(id) {
                for key in &coverage.known_absent {
                    let key = match key {
                        SourceTableKey::Text(value) => Key::Bytes(value.as_bytes()),
                        SourceTableKey::Integer(value) => Key::Number(if *value == 0 {
                            0
                        } else {
                            (*value as f64).to_bits()
                        }),
                    };
                    if raw.contains(&key) {
                        return Err(invalid(
                            "session traversal raw key contradicts known absence",
                        ));
                    }
                }
            }
            if raw.len() != observation.order.len() {
                return Err(invalid(
                    "session traversal order is not a complete raw permutation",
                ));
            }
            let mut seen = BTreeSet::new();
            for key in &observation.order {
                let key = self.traversal_key(key)?;
                if !raw.contains(&key) || !seen.insert(key) {
                    return Err(invalid(
                        "session traversal order has a missing or duplicate raw key",
                    ));
                }
            }
            if let Some(length) = observation.raw_length {
                let number = |value: u64| Key::Number((value as f64).to_bits());
                if (length != 0 && !raw.contains(&number(u64::from(length))))
                    || raw.contains(&number(u64::from(length) + 1))
                {
                    return Err(invalid(
                        "observed session raw length is not a valid raw boundary",
                    ));
                }
            }
        }
        Ok(())
    }
    fn session_table(&self, id: SourceSessionTableId) -> SourceProgramResult<&SourceSessionTable> {
        id.0.checked_sub(1)
            .and_then(|index| self.state.tables.get(index as usize))
            .ok_or_else(|| invalid("session traversal has no local state table"))
    }
    fn traversal_key<'a>(&self, value: &'a SourceSessionValue) -> SourceProgramResult<Key<'a>> {
        Ok(match value {
            SourceSessionValue::Nil => return Err(invalid("nil session traversal key")),
            SourceSessionValue::Number(value) if value.is_nan() => {
                return Err(invalid("NaN session traversal key"));
            }
            SourceSessionValue::Number(value) => {
                Key::Number(if *value == 0.0 { 0 } else { value.to_bits() })
            }
            SourceSessionValue::Boolean(value) => Key::Boolean(*value),
            SourceSessionValue::Bytes(value) => Key::Bytes(value),
            SourceSessionValue::Table(id) => {
                self.session_table(*id)?;
                Key::Table(*id)
            }
            SourceSessionValue::Callback(id) => {
                if self.owner.callback(*id).is_none()
                    || self.owner.closure_prototype_id(*id).is_some()
                {
                    return Err(invalid(
                        "session traversal lacks a definition-owned callback",
                    ));
                }
                Key::Callback(*id)
            }
            SourceSessionValue::DefinitionTable(id) => {
                if self.owner.table(*id).is_none() {
                    return Err(invalid("session traversal lacks a definition table"));
                }
                Key::DefinitionTable(*id)
            }
            SourceSessionValue::Closure(id) => {
                let closure =
                    id.0.checked_sub(1)
                        .and_then(|index| self.closures.get(index as usize))
                        .ok_or_else(|| invalid("session traversal lacks a local closure"))?;
                self.owner.resolve_closure_prototype(&closure.prototype)?;
                Key::Closure(*id)
            }
        })
    }
}
