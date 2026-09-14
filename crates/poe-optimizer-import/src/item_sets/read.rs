//! Bounded borrowed reads of privately produced item-set state.
//! No snapshots, authored-lineage inference, or general live loadout context.
use super::{AssemblyError, Id, ItemSetState, Result, V};
use crate::selected_view::NumericValue;

#[derive(Debug, Clone, Copy)]
pub struct ItemSetReadLimits {
    pub max_steps: u64,
    /// Cumulative bytes of titles validated and returned by this view.
    pub max_text_bytes: usize,
}
impl Default for ItemSetReadLimits {
    fn default() -> Self {
        Self {
            max_steps: 5_000_000,
            max_text_bytes: 262144,
        }
    }
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ItemSetReadUsage {
    pub steps: u64,
    pub text_bytes: usize,
}

/// An exact row in one borrowed state, including detached previous-active rows.
/// Numeric key equality does not imply row identity. This is not a SetOrigin.
///
/// A retained row prevents a producer restart while the row is still used:
///
/// ```compile_fail
/// use poe_optimizer_import::item_sets::{ItemSetReadLimits, ItemSetState};
/// fn cannot_restart(state: &mut ItemSetState) {
///     let mut view = state.read_view(ItemSetReadLimits::default());
///     let row = view.active().unwrap().unwrap();
///     state.begin_load().unwrap();
///     assert!(row.belongs_to(state));
/// }
/// ```
#[derive(Clone, Copy)]
pub struct ItemSetRow<'a> {
    owner: &'a ItemSetState,
    id: Id,
}
impl ItemSetRow<'_> {
    /// Retain this exact row's producer identity across subsequent state changes.
    /// Authored lineage is attached by the enclosing native build owner.
    pub fn identity(&self) -> super::ItemSetIdentity {
        super::ItemSetIdentity::for_row(self.owner, self.id)
    }

    pub fn belongs_to(&self, owner: &ItemSetState) -> bool {
        std::ptr::eq(self.owner, owner)
    }
    pub fn same_identity(&self, other: &Self) -> bool {
        self.belongs_to(other.owner) && self.id == other.id
    }
}

/// Independent caller-controlled read budget. Borrowing this view/its rows
/// prevents mutation or movement of their state; producer usage is unchanged.
pub struct ItemSetReadView<'a> {
    state: &'a ItemSetState,
    limits: ItemSetReadLimits,
    usage: ItemSetReadUsage,
}
impl ItemSetState {
    pub fn read_view(&self, limits: ItemSetReadLimits) -> ItemSetReadView<'_> {
        ItemSetReadView {
            state: self,
            limits,
            usage: ItemSetReadUsage::default(),
        }
    }
}
impl<'a> ItemSetReadView<'a> {
    pub fn usage(&self) -> ItemSetReadUsage {
        self.usage
    }
    fn charge(&mut self, steps: u64, text_bytes: usize) -> Result<()> {
        let next_steps = self
            .usage
            .steps
            .checked_add(steps)
            .filter(|n| *n <= self.limits.max_steps)
            .ok_or_else(|| AssemblyError::resource("item-set read work bound"))?;
        let next_text = self
            .usage
            .text_bytes
            .checked_add(text_bytes)
            .filter(|n| *n <= self.limits.max_text_bytes)
            .ok_or_else(|| AssemblyError::resource("item-set read text bound"))?;
        self.usage = ItemSetReadUsage {
            steps: next_steps,
            text_bytes: next_text,
        };
        Ok(())
    }
    fn entries(&mut self, id: Id) -> Result<&'a [(V, V)]> {
        self.charge(1, 0)?;
        self.state
            .graph
            .tables
            .get(id.0.checked_sub(1).unwrap_or(u32::MAX) as usize)
            .map(|table| table.entries.as_slice())
            .ok_or_else(|| AssemblyError::unsupported("foreign item-set read table"))
    }
    fn field(&mut self, id: Id, name: &str) -> Result<Option<&'a V>> {
        for (key, value) in self.entries(id)? {
            self.charge(1, 0)?;
            if let V::Bytes(bytes) = key {
                let cost = bytes
                    .len()
                    .checked_add(name.len())
                    .and_then(|n| u64::try_from(n).ok())
                    .ok_or_else(|| AssemblyError::resource("item-set read comparison overflow"))?;
                self.charge(cost, 0)?;
                if bytes == name.as_bytes() {
                    return Ok(Some(value));
                }
            }
        }
        Ok(None)
    }
    fn numeric(&mut self, id: Id, key: f64) -> Result<Option<&'a V>> {
        for (candidate, value) in self.entries(id)? {
            self.charge(1, 0)?;
            if matches!(candidate, V::Number(n) if *n == key) {
                return Ok(Some(value));
            }
        }
        Ok(None)
    }
    fn row(&mut self, value: Option<&V>) -> Result<Option<ItemSetRow<'a>>> {
        match value {
            None | Some(V::Nil) => Ok(None),
            Some(V::Table(id)) => {
                self.entries(*id)?;
                Ok(Some(ItemSetRow {
                    owner: self.state,
                    id: *id,
                }))
            }
            _ => Err(AssemblyError::unsupported(
                "item-set read row is not a table",
            )),
        }
    }
    fn root_table(&mut self, name: &str) -> Result<Id> {
        match self.field(self.state.root, name)? {
            Some(V::Table(id)) => Ok(*id),
            _ => Err(AssemblyError::unsupported(
                "item-set published field is not a table",
            )),
        }
    }
    fn number(value: Option<&V>) -> Result<Option<NumericValue>> {
        match value {
            None | Some(V::Nil) => Ok(None),
            Some(V::Number(number)) => Ok(Some(NumericValue::new(*number))),
            _ => Err(AssemblyError::unsupported(
                "item-set read selector is not numeric",
            )),
        }
    }
    /// Prove a dense sequence before exposing Lua length/singleton semantics.
    /// Graph::set guarantees unique keys; every stored key must be an integer
    /// in 1..=entry_count, with a present value. Numeric ID values are not coerced.
    pub fn dense_order_len(&mut self) -> Result<usize> {
        // The producer may allocate a replacement before a failing publication.
        // Resolve the actual root field, including its retained failure prefix.
        let order = self.root_table("itemSetOrderList")?;
        let entries = self.entries(order)?;
        if entries.len() as u128 > (1u128 << 53) {
            return Err(AssemblyError::resource(
                "item-set order length is not exactly representable",
            ));
        }
        for (key, value) in entries {
            self.charge(1, 0)?;
            if !matches!(key, V::Number(n) if n.is_finite()
                && *n >= 1.0 && *n <= entries.len() as f64 && n.fract() == 0.0)
                || matches!(value, V::Nil)
            {
                return Err(AssemblyError::unsupported(
                    "item-set order has no proven dense length",
                ));
            }
        }
        Ok(entries.len())
    }
    pub fn is_singleton(&mut self) -> Result<bool> {
        Ok(self.dense_order_len()? == 1)
    }
    /// Raw one-based order read, independent of the dense-length proof.
    pub fn ordered_key(&mut self, position: usize) -> Result<Option<NumericValue>> {
        self.charge(1, 0)?;
        if position == 0 || position as u128 > (1u128 << 53) {
            return Err(AssemblyError::unsupported(
                "item-set order position is not an exact positive index",
            ));
        }
        let order = self.root_table("itemSetOrderList")?;
        Self::number(self.numeric(order, position as f64)?)
    }
    /// Numeric map lookup uses Lua equality, including both zeros. Returned
    /// scalar order/active values preserve their original bits.
    pub fn winner(&mut self, key: NumericValue) -> Result<Option<ItemSetRow<'a>>> {
        let sets = self.root_table("itemSets")?;
        let value = self.numeric(sets, key.value())?;
        self.row(value)
    }
    pub fn active(&mut self) -> Result<Option<ItemSetRow<'a>>> {
        let value = self.field(self.state.root, "activeItemSet")?;
        self.row(value)
    }
    pub fn previous(&mut self) -> Result<Option<ItemSetRow<'a>>> {
        let value = self.field(self.state.root, "previousActiveItemSet")?;
        self.row(value)
    }
    pub fn active_key(&mut self) -> Result<Option<NumericValue>> {
        Self::number(self.field(self.state.root, "activeItemSetId")?)
    }
    /// Borrow the actual optional title; fallback formatting belongs to its
    /// consumer. A foreign row is rejected before touching that owner's graph.
    pub fn title(&mut self, row: &ItemSetRow<'_>) -> Result<Option<&'a str>> {
        self.charge(1, 0)?;
        if !row.belongs_to(self.state) {
            return Err(AssemblyError::unsupported(
                "item-set row belongs to another owner",
            ));
        }
        match self.field(row.id, "title")? {
            None | Some(V::Nil) => Ok(None),
            Some(V::Bytes(bytes)) => {
                let work = u64::try_from(bytes.len())
                    .map_err(|_| AssemblyError::resource("item-set read title overflow"))?;
                self.charge(work, bytes.len())?;
                std::str::from_utf8(bytes)
                    .map(Some)
                    .map_err(|_| AssemblyError::unsupported("item-set title is not UTF-8"))
            }
            _ => Err(AssemblyError::unsupported(
                "item-set title is outside borrowed text domain",
            )),
        }
    }
}

#[cfg(test)]
#[path = "read_tests.rs"]
mod tests;
