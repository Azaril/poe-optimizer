//! Private source-observed table facts, invalidated by structural writes.
use super::closures::ImportClosures;
use super::{
    Error, Heap, Key, Result, Table, TableRef, V, index, input_value, table_ref, validate_input,
};
use crate::lua_pattern::MatchBudget;
use poe_optimizer_data::source_program::{SourceProgramErrorKind, SourceSessionInput};
use std::collections::BTreeMap;

pub(super) struct TableObservation {
    order: Vec<Key>,
    // Sorted positions into order; no copied key payload per lookup.
    positions: Vec<usize>,
    pub(super) raw_length: Option<usize>,
}

impl Heap<'_> {
    pub(super) fn import_table_traversal(
        &mut self,
        input: &SourceSessionInput,
        tables: &[Table],
        offset: u32,
        closures: Option<ImportClosures>,
    ) -> Result<BTreeMap<TableRef, TableObservation>> {
        let Some(source) = &input.traversal else {
            return Ok(BTreeMap::new());
        };
        let available_values = self
            .budget
            .limits
            .max_values
            .saturating_sub(self.stats().values);
        let allowed_bytes = self.budget.limits.max_bytes;
        self.budget.values(
            source
                .tables
                .len()
                .checked_mul(2)
                .ok_or_else(|| Error::resource("table observation headers"))?,
        )?;
        for (id, record) in &source.tables {
            // Retained keys/positions plus both temporary validator sets.
            // Malformed unequal raw/order inventories are still fully charged.
            let raw_count = input
                .state
                .tables
                .get(index(id.0)?)
                .map_or(0, |table| table.entries.len());
            self.budget.values(
                record
                    .order
                    .len()
                    .checked_mul(3)
                    .and_then(|count| count.checked_add(raw_count))
                    .ok_or_else(|| Error::resource("table observation keys"))?,
            )?;
            for key in &record.order {
                validate_input(
                    key,
                    input.state.tables.len(),
                    &self.catalog,
                    &mut self.budget,
                    closures,
                )?;
            }
        }
        input
            .validate_traversal(tables.len(), available_values, allowed_bytes)
            .map_err(|error| match error.kind {
                SourceProgramErrorKind::ResourceLimit => Error::resource(error.to_string()),
                _ => Error::input(error.to_string()),
            })?;
        let mut observations = BTreeMap::new();
        for (id, record) in &source.tables {
            let table = tables
                .get(index(id.0)?)
                .ok_or_else(|| Error::input("table observation references a missing table"))?;
            let mut order = Vec::with_capacity(record.order.len());
            for source in &record.order {
                let value = input_value(source, true, offset, closures);
                let key =
                    Key::write(&value).map_err(|_| Error::input("invalid observed table key"))?;
                if !table.entries.contains_key(&key) {
                    return Err(Error::input(
                        "observed key is absent from the imported table",
                    ));
                }
                order.push(key);
            }
            let mut positions: Vec<_> = (0..order.len()).collect();
            positions.sort_unstable_by(|a, b| order[*a].cmp(&order[*b]));
            if order.len() != table.entries.len()
                || positions
                    .windows(2)
                    .any(|pair| order[pair[0]] == order[pair[1]])
            {
                return Err(Error::input(
                    "observed order is not an exact raw key permutation",
                ));
            }
            observations.insert(
                TableRef::Heap(offset + id.0),
                TableObservation {
                    order,
                    positions,
                    raw_length: record.raw_length.map(|length| length as usize),
                },
            );
        }
        Ok(observations)
    }

    pub(super) fn observed_next(
        &mut self,
        table: &V,
        control: &V,
        work: &mut MatchBudget,
    ) -> Result<Option<(V, V)>> {
        let reference = table_ref(table)?;
        let observation = self.table_observations.get(&reference).ok_or_else(|| {
            Error::unsupported("current session table traversal order is unavailable")
        })?;
        let next = if matches!(control, V::Nil) {
            0
        } else {
            let control = Key::read(control).ok_or_else(|| Error::source("invalid key to next"))?;
            let mut low = 0;
            let mut high = observation.positions.len();
            let mut found = None;
            while low < high {
                work.charge(1)?;
                let middle = low + (high - low) / 2;
                let position = observation.positions[middle];
                let key = &observation.order[position];
                if let (Key::Bytes(left), Key::Bytes(right)) = (key, &control) {
                    work.charge(left.len().min(right.len()) as u64)?;
                }
                match key.cmp(&control) {
                    std::cmp::Ordering::Less => low = middle + 1,
                    std::cmp::Ordering::Greater => high = middle,
                    std::cmp::Ordering::Equal => {
                        found = Some(position + 1);
                        break;
                    }
                }
            }
            found.ok_or_else(|| {
                Error::unsupported("next control is outside the observed live-key inventory")
            })?
        };
        let Some(key) = observation.order.get(next) else {
            return Ok(None);
        };
        let key = key.value();
        let value = self.raw_get(table, &key)?;
        if matches!(value, V::Nil) {
            return Err(Error::input("observed traversal key has no raw value"));
        }
        Ok(Some((key, value)))
    }
}
