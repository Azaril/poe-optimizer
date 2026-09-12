//! Membership-preserving traversal for an authenticated string-only TDUP seed.
//! Nil reservations remain in shared order; raw values stay in the ordinary heap.
use super::{CompiledReservedKeys, Error, Heap, Key, Result, TableRef, V, index, table_ref};
use crate::lua_pattern::MatchBudget;
use std::sync::Arc;

impl Heap<'_> {
    pub(in crate::parser_program::runtime) fn reserved_table(
        &mut self,
        keys: Arc<CompiledReservedKeys>,
        work: &mut MatchBudget,
    ) -> Result<V> {
        work.charge(1 + keys.len() as u64)?;
        // Only the per-table map entry/Arc is private. Key bytes and positions
        // belong to the bounded compiled seed and are never copied per instance.
        self.charge_values(2)?;
        let table = self.new_table()?;
        self.reserved_keys.insert(table_ref(&table)?, keys);
        Ok(table)
    }

    pub(super) fn reserved_next(
        &self,
        table: &V,
        control: &V,
        work: &mut MatchBudget,
    ) -> Result<Option<Option<(V, V)>>> {
        let reference = table_ref(table)?;
        let Some(keys) = self.reserved_keys.get(&reference) else {
            return Ok(None);
        };
        let TableRef::Heap(id) = reference else {
            return Err(Error::input(
                "reserved key certificate requires private storage",
            ));
        };
        let table = self
            .tables
            .get(index(id)?)
            .ok_or_else(|| Error::input("missing reserved-key heap table"))?;
        work.charge(1)?;
        let start = match control {
            V::Nil => 0,
            V::Bytes(bytes) => keys
                .find(bytes, work)?
                .map(|position| position + 1)
                .ok_or_else(|| {
                    Error::unsupported("next control is outside the reserved key inventory")
                })?,
            // No capacity/dead-node information is claimed for other controls.
            _ => {
                return Err(Error::unsupported(
                    "next control is outside the reserved key inventory",
                ));
            }
        };
        for position in start..keys.len() {
            work.charge(1)?;
            let bytes = keys.key(position);
            // BTreeMap may compare multiple keys within each node. Bound every
            // possible comparison by the complete live inventory, rather than
            // assuming a binary comparison count inside its implementation.
            let lookup_work = (1 + bytes.len() as u64)
                .checked_mul(table.entries.len() as u64)
                .ok_or_else(|| Error::resource("reserved-key lookup work"))?;
            work.charge(lookup_work)?;
            let key = Key::Bytes(bytes.clone());
            if let Some(value) = table.entries.get(&key) {
                return Ok(Some(Some((key.value(), value.clone()))));
            }
        }
        Ok(Some(None))
    }
}

#[cfg(test)]
mod tests;
