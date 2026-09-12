//! Private layout facts derived only from an admitted source array constructor.
//! Hash allocation ends this bounded simulation; raw values remain writable.
use super::{Error, Heap, Key, Result, TableRef, V, index, table_ref};
use crate::lua_pattern::MatchBudget;

// Pinned LuaJIT profile: lj_def.h and lj_tab.c countint/bestasize.
const ARRAY_BITS: usize = 28;
const MAX_ARRAY_SLOTS: u32 = (1 << 27) + 1;
const METADATA_VALUES: usize = ARRAY_BITS + 2;

#[derive(Clone, Copy)]
pub(super) struct ArrayLayout {
    slots: u32,
    live: u32,
    bins: [u32; ARRAY_BITS],
}
impl ArrayLayout {
    fn reserved(slots: u32) -> Self {
        Self {
            slots,
            live: 0,
            bins: [0; ARRAY_BITS],
        }
    }
    fn key(key: &Key) -> Option<u32> {
        let Key::Number(bits) = key else { return None };
        let value = f64::from_bits(*bits);
        (value >= 0.0 && value < f64::from(MAX_ARRAY_SLOTS) && value.fract() == 0.0)
            .then_some(value as u32)
    }
    fn bin(key: u32) -> usize {
        if key <= 2 {
            0
        } else {
            (31 - (key - 1).leading_zeros()) as usize
        }
    }
    /// Calculate against pre-write occupancy. Even a nil incoming value can
    /// cause rehash/allocation; only the final occupancy update uses that value.
    pub(super) fn transition(
        self,
        key: &Key,
        present: bool,
        non_nil: bool,
        work: &mut MatchBudget,
    ) -> Result<Option<Self>> {
        work.charge(1)?;
        let Some(key) = Self::key(key) else {
            return Ok(None);
        };
        let mut next = self;
        let bin = Self::bin(key);
        if key >= self.slots {
            let mut bins = self.bins;
            bins[bin] += 1;
            let total = self.live + 1;
            let (mut sum, mut selected, mut slots) = (0u32, 0u32, 0u32);
            for (b, count) in bins.into_iter().enumerate() {
                work.charge(1)?;
                if 2 * total <= (1u32 << b) || sum == total {
                    break;
                }
                if count > 0 {
                    sum += count;
                    if 2 * sum > (1u32 << b) {
                        slots = (2u32 << b) + 1;
                        selected = sum;
                    }
                }
            }
            if selected != total {
                return Ok(None);
            }
            if slots > MAX_ARRAY_SLOTS || slots <= key {
                return Err(Error::input("invalid derived array layout"));
            }
            next.slots = slots;
        }
        match (present, non_nil) {
            (false, true) => {
                next.live += 1;
                next.bins[bin] += 1;
            }
            (true, false) => {
                next.live -= 1;
                next.bins[bin] -= 1;
            }
            _ => (),
        }
        Ok(Some(next))
    }
    pub(super) fn growth(self, previous: Self) -> usize {
        self.slots.saturating_sub(previous.slots) as usize
    }
}
impl Heap<'_> {
    /// The caller must retain an exact validated source constructor seed.
    /// Ordinary new_table and every graph import deliberately omit these facts.
    pub(in crate::parser_program::runtime) fn native_array_table(
        &mut self,
        slots: u32,
        work: &mut MatchBudget,
    ) -> Result<V> {
        if slots > MAX_ARRAY_SLOTS {
            return Err(Error::resource("source array allocation bound"));
        }
        work.charge(1 + u64::from(slots))?;
        self.charge_values(METADATA_VALUES + slots as usize)?;
        let value = self.new_table()?;
        let TableRef::Heap(id) = table_ref(&value)? else {
            unreachable!()
        };
        self.tables[index(id)?].array_layout = Some(ArrayLayout::reserved(slots));
        Ok(value)
    }
    #[cfg(test)]
    fn native_empty_table(&mut self, work: &mut MatchBudget) -> Result<V> {
        self.native_array_table(0, work)
    }
    /// A source list store beyond its initial reservation may be recorded as
    /// different allocation history (including elided nil stores). Keep raw
    /// values, but do not carry a single physical layout across that frontier.
    pub(in crate::parser_program::runtime) fn native_array_list(
        &mut self,
        table: &V,
        key: usize,
        value: V,
        work: &mut MatchBudget,
    ) -> Result<()> {
        let TableRef::Heap(id) = table_ref(table)? else {
            return Err(Error::input(
                "array list requires private constructor storage",
            ));
        };
        let in_capacity = self.tables[index(id)?]
            .array_layout
            .is_some_and(|layout| key < layout.slots as usize);
        self.raw_set(table, V::Number(key as f64), value, work)?;
        if !in_capacity {
            self.tables[index(id)?].array_layout = None;
        }
        Ok(())
    }
    /// Source TSETM: evaluate the whole final call/vararg pack first, then grow
    /// once and copy slots (including nil). The pinned VM passes start+count to
    /// lj_tab_reasize, which reserves one additional slot when growth is needed.
    pub(in crate::parser_program::runtime) fn native_array_tail(
        &mut self,
        table: &V,
        start: usize,
        values: Vec<V>,
        work: &mut MatchBudget,
    ) -> Result<()> {
        work.charge(1)?;
        let reference = table_ref(table)?;
        let TableRef::Heap(id) = reference else {
            return Err(Error::input(
                "array tail requires a private constructor table",
            ));
        };
        if start == 0 || self.coverage.contains_key(&reference) || self.behavior(table).is_some() {
            return Err(Error::input(
                "array tail requires plain constructor storage",
            ));
        }
        let target = self
            .tables
            .get(index(id)?)
            .ok_or_else(|| Error::input("missing heap table"))?;
        let previous = target.array_layout;
        if values.is_empty() {
            return Ok(());
        }
        let needed = start
            .checked_add(values.len())
            .and_then(|n| u32::try_from(n).ok())
            .ok_or_else(|| Error::resource("array tail index bound"))?;
        let mut next = previous.unwrap_or_else(|| ArrayLayout::reserved(0));
        let preserves_layout = previous.is_some() && needed <= next.slots;
        if needed > next.slots {
            next.slots = needed
                .checked_add(1)
                .filter(|n| *n <= MAX_ARRAY_SLOTS)
                .ok_or_else(|| Error::resource("array tail allocation bound"))?;
        }
        let mut allocation = next.growth(previous.unwrap_or_else(|| ArrayLayout::reserved(0)));
        work.charge(allocation as u64)?;
        // Preflight every write and charge all storage before changing either
        // the private values or layout. Failed resource checks retain charges.
        for (offset, value) in values.iter().enumerate() {
            work.charge(1)?;
            let key = (start + offset) as u32;
            let present = target
                .entries
                .contains_key(&Key::Number(f64::from(key).to_bits()));
            let non_nil = !matches!(value, V::Nil);
            let bin = ArrayLayout::bin(key);
            match (present, non_nil) {
                (false, true) => {
                    if previous.is_some() {
                        next.live += 1;
                        next.bins[bin] += 1;
                    }
                    allocation = allocation
                        .checked_add(3)
                        .ok_or_else(|| Error::resource("array tail storage bound"))?;
                }
                (true, false) if previous.is_some() => {
                    next.live -= 1;
                    next.bins[bin] -= 1;
                }
                _ => (),
            }
        }
        self.charge_values(allocation)?;
        let target = &mut self.tables[index(id)?];
        for (offset, value) in values.into_iter().enumerate() {
            let key = Key::Number(((start + offset) as f64).to_bits());
            if matches!(value, V::Nil) {
                target.remove(&key);
            } else {
                target.insert(key, value);
            }
        }
        // rec_tsetm emits independent indexed stores without TABLE_BUMP.
        // A growing pack can therefore differ from the interpreter's bulk
        // resize. Preserve raw values without inventing a universal capacity.
        target.array_layout = preserves_layout.then_some(next);
        self.table_observations.remove(&reference);
        Ok(())
    }
    pub(super) fn native_array_len(
        &self,
        table: &V,
        work: &mut MatchBudget,
    ) -> Result<Option<usize>> {
        let TableRef::Heap(id) = table_ref(table)? else {
            return Ok(None);
        };
        let table = self
            .tables
            .get(index(id)?)
            .ok_or_else(|| Error::input("missing heap table"))?;
        let Some(layout) = table.array_layout else {
            return Ok(None);
        };
        work.charge(1)?;
        let mut hi = layout.slots.saturating_sub(1);
        let occupied = |key: u32| {
            table
                .entries
                .contains_key(&Key::Number(f64::from(key).to_bits()))
        };
        if hi > 0 && !occupied(hi) {
            let mut lo = 0;
            while hi - lo > 1 {
                work.charge(1)?;
                let mid = (lo + hi) >> 1;
                if occupied(mid) {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            return Ok(Some(lo as usize));
        }
        Ok(Some(hi as usize))
    }
    /// A traced ALEN may reuse any still-valid non-nil -> nil hint. Require
    /// every such boundary to equal the physical raw-length result, including
    /// slot zero. A lone positive run is necessary but its end must match raw.
    pub(in crate::parser_program::runtime) fn hint_safe_len(
        &mut self,
        table: &V,
        work: &mut MatchBudget,
    ) -> Result<usize> {
        let raw = self.raw_len(table, work)?;
        let TableRef::Heap(id) = table_ref(table)? else {
            return Ok(raw);
        };
        let table = &self.tables[index(id)?];
        if table.array_layout.is_none() {
            return Ok(raw);
        }
        work.charge(1)?;
        if let (Some(first), Some(last)) = (table.positive.first(), table.positive.last()) {
            let first = f64::from_bits(*first) as usize;
            let last = f64::from_bits(*last) as usize;
            let zero = table.entries.contains_key(&Key::Number(0.0f64.to_bits()));
            if last != raw || last - first + 1 != table.positive.len() || (zero && first > 1) {
                return Err(Error::unsupported(
                    "source JIT length hints have multiple possible array boundaries",
                ));
            }
        }
        Ok(raw)
    }
    pub(super) fn native_array_next(
        &self,
        table: &V,
        control: &V,
        work: &mut MatchBudget,
    ) -> Result<Option<Option<(V, V)>>> {
        let TableRef::Heap(id) = table_ref(table)? else {
            return Ok(None);
        };
        let table = self
            .tables
            .get(index(id)?)
            .ok_or_else(|| Error::input("missing heap table"))?;
        let Some(layout) = table.array_layout else {
            return Ok(None);
        };
        work.charge(1)?;
        let start = match control {
            V::Nil => 0,
            V::Number(value)
                if *value >= 0.0 && *value < f64::from(layout.slots) && value.fract() == 0.0 =>
            {
                *value as u32 + 1
            }
            _ => return Err(Error::source("invalid key to next")),
        };
        if start == 0
            && let Some(value) = table.entries.get(&Key::Number(0.0f64.to_bits()))
        {
            work.charge(1)?;
            return Ok(Some(Some((V::Number(0.0), value.clone()))));
        }
        let lower = f64::from(start.max(1)).to_bits();
        if let Some(bits) = table.positive.range(lower..).next() {
            let key = f64::from_bits(*bits);
            // The private invariant guarantees no live numeric key exceeds capacity.
            work.charge(key as u64 + 1 - u64::from(start))?;
            let value = table
                .entries
                .get(&Key::Number(*bits))
                .expect("positive index value");
            return Ok(Some(Some((V::Number(key), value.clone()))));
        }
        work.charge(u64::from(layout.slots.saturating_sub(start)))?;
        Ok(Some(None))
    }
}

#[cfg(test)]
mod tests;
