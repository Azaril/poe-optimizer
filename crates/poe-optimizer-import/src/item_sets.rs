//! Ordered item-set loading up to the original SetActiveItemSet call entry.
//! This state never certifies activation, PopulateSlots, loadout callbacks, or
//! actor participation. Diagnostic graphs cannot be imported as state.
mod graph;
mod layout;
#[cfg(test)]
mod tests;
use crate::item_loading::{ItemNumber, assembly::AssemblyError};
use graph::{Graph, table, truthy};
use poe_optimizer_data::item_assembly::{ItemInventoryPolicy, ItemInventoryPowerTransform};
use poe_optimizer_engine::{
    lua_number::parse_number,
    source_program::{ProgramTableId as Id, ProgramValue as V, ProgramValueGraph},
};
use serde::Serialize;
use std::sync::Arc;
pub type Result<T> = std::result::Result<T, AssemblyError>;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct ItemSetLimits {
    pub max_tables: usize,
    pub max_values: usize,
    pub max_bytes: usize,
    pub max_steps: u64,
    pub max_slots: usize,
    pub max_string_bytes: usize,
    pub max_compiled_bytes: usize,
}
impl Default for ItemSetLimits {
    fn default() -> Self {
        Self {
            max_tables: 32768,
            max_values: 1_000_000,
            max_bytes: 32 * 1024 * 1024,
            max_steps: 5_000_000,
            max_slots: 4096,
            max_string_bytes: 262144,
            max_compiled_bytes: 1024 * 1024,
        }
    }
}
/// Cumulative logical construction/copy/work charges, not allocator bytes/RSS.
/// Diagnostic snapshot copying is outside these producer charges.
#[derive(Debug, Default, Clone, Copy, Serialize)]
pub struct ItemSetUsage {
    pub tables: usize,
    pub values: usize,
    pub bytes: usize,
    pub steps: u64,
    pub operations: u64,
    pub pattern_steps: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemSetPhase {
    ConstructorReady,
    Loading,
    ItemSetOpen,
    AwaitingActivation,
    Failed,
}
#[derive(Debug, Default, Clone, Copy)]
pub struct LegacySlotInput<'a> {
    pub name: Option<&'a str>,
    pub item_id: Option<&'a str>,
    pub active: Option<&'a str>,
}
#[derive(Debug, Default, Clone, Copy)]
pub struct ItemSetInput<'a> {
    pub id: Option<&'a str>,
    pub title: Option<&'a str>,
    pub use_second_weapon_set: Option<&'a str>,
}
#[derive(Debug, Default, Clone, Copy)]
pub struct SetSlotInput<'a> {
    pub name: Option<&'a str>,
    pub item_id: Option<&'a str>,
    pub active: Option<&'a str>,
    pub item_pb_url: Option<&'a str>,
    pub note: Option<&'a str>,
}
#[derive(Debug, Default, Clone, Copy)]
pub struct SetRuneInput<'a> {
    pub slot_name: Option<&'a str>,
    pub rune_name: Option<&'a str>,
}
#[derive(Debug, Default, Clone, Copy)]
pub struct SocketUrlInput<'a> {
    pub node_id: Option<&'a str>,
    pub item_pb_url: Option<&'a str>,
}
#[derive(Debug, Default, Clone, Copy)]
pub struct TradeWeightInput<'a> {
    pub label: Option<&'a str>,
    pub stat: Option<&'a str>,
    pub weight_mult: Option<&'a str>,
}
#[derive(Debug, Default, Clone, Copy)]
pub struct FinishLoadInput<'a> {
    pub active_item_set: Option<&'a str>,
    pub use_second_weapon_set: Option<&'a str>,
    pub show_stat_differences: Option<&'a str>,
}
#[derive(Debug, Clone, Serialize)]
pub struct ItemSetContinuation {
    pub requested_set: ItemNumber,
    /// Deferred until activation/Populate/Sync actually succeed.
    pub show_stat_differences: Option<bool>,
    pub required_stage: &'static str,
}
/// Transform descriptors are retained with the exact private policy owner.
/// Equality of descriptor contents does not merge their source identities.
#[derive(Clone)]
pub struct ItemSetTransform {
    owner: Arc<ItemInventoryPolicy>,
    index: u16,
}
impl ItemSetTransform {
    pub fn descriptor(&self) -> &ItemInventoryPowerTransform {
        &self.owner.power_stats.transforms[usize::from(self.index)]
    }
    pub fn same_identity(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.index == other.index
    }
    pub fn catalog_index(&self) -> u16 {
        self.index
    }
}
pub struct ItemSetState {
    graph: Graph,
    policy: Arc<ItemInventoryPolicy>,
    root: Id,
    sets: Id,
    order: Id,
    slots: Id,
    runes: Id,
    trade: Id,
    ordered_slots: Vec<Id>,
    open: Option<Id>,
    phase: ItemSetPhase,
    pending: Option<ItemSetContinuation>,
    failure: Option<AssemblyError>,
    transforms: Vec<Option<u16>>,
}
impl ItemSetState {
    /// Ordinary fresh constructor state from injected, validated definitions.
    /// Passive IDs must be the catalog's authenticated complete constructor
    /// projection; the native owner is responsible for package/tree binding.
    pub fn new(policy: &ItemInventoryPolicy, limits: ItemSetLimits) -> Result<Self> {
        policy
            .validate()
            .map_err(|e| AssemblyError::unsupported(e.to_string()))?;
        // Bounded counting writer avoids allocating serialized policy bytes.
        struct Count(usize);
        impl std::io::Write for Count {
            fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                self.0 = self
                    .0
                    .checked_add(b.len())
                    .ok_or_else(|| std::io::Error::other("policy byte overflow"))?;
                Ok(b.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut count = Count(0);
        serde_json::to_writer(&mut count, policy)
            .map_err(|e| AssemblyError::resource(e.to_string()))?;
        let mut graph = Graph::new(limits);
        graph.charge(count.0, 1)?;
        let policy = Arc::new(policy.clone());
        let root = graph.table()?;
        let sets = graph.table()?;
        let order = graph.table()?;
        let slots = graph.table()?;
        let runes = graph.table()?;
        let trade = graph.table()?;
        for (name, id) in [
            ("itemSets", sets),
            ("itemSetOrderList", order),
            ("slots", slots),
            ("runeSlots", runes),
            ("trade", trade),
        ] {
            graph.set_field(root, name, V::Table(id))?;
        }
        graph.set_field(
            root,
            "showStatDifferences",
            V::Boolean(policy.defaults.show_stat_differences),
        )?;
        let mut state = Self {
            graph,
            policy,
            root,
            sets,
            order,
            slots,
            runes,
            trade,
            ordered_slots: Vec::new(),
            open: None,
            phase: ItemSetPhase::ConstructorReady,
            pending: None,
            failure: None,
            transforms: Vec::new(),
        };
        state.construct_layout()?;
        let policy = Arc::clone(&state.policy);
        let first = state.create(
            Some(policy.defaults.first_set_id),
            &policy.defaults.default_set_title,
        )?;
        state
            .graph
            .set_field(root, "activeItemSet", V::Table(first))?;
        state
            .graph
            .set_field(root, "previousActiveItemSet", V::Table(first))?;
        state.graph.set_field(
            root,
            "activeItemSetId",
            V::Number(policy.defaults.first_set_id),
        )?;
        let id = state.graph.field(first, "id")?;
        state.graph.append(order, id)?;
        if policy.defaults.empty_item_id > 0.0 {
            return Err(AssemblyError::source(
                "fresh PopulateSlots indexes a missing positive selected item",
            ));
        }
        Ok(state)
    }
    pub fn phase(&self) -> ItemSetPhase {
        self.phase
    }
    pub fn usage(&self) -> ItemSetUsage {
        self.graph.usage
    }
    pub fn limits(&self) -> ItemSetLimits {
        self.graph.limits
    }
    pub fn pending_activation(&self) -> Option<ItemNumber> {
        self.pending.as_ref().map(|p| p.requested_set)
    }
    pub fn continuation(&self) -> Option<&ItemSetContinuation> {
        self.pending.as_ref()
    }
    pub fn failure(&self) -> Option<&AssemblyError> {
        self.failure.as_ref()
    }
    pub fn snapshot(&self) -> Result<ProgramValueGraph> {
        self.graph.snapshot(self.root)
    }
    pub fn trade_transform(&self, index: usize) -> Option<ItemSetTransform> {
        self.transforms
            .get(index)
            .copied()
            .flatten()
            .map(|index| ItemSetTransform {
                owner: Arc::clone(&self.policy),
                index,
            })
    }
    /// Shrink the remaining producer ceiling when the enclosing ordered item
    /// stream consumes shared bytes. It cannot restore budget or erase charges.
    pub fn tighten_byte_limit(&mut self, ceiling: usize) -> Result<()> {
        if ceiling > self.graph.limits.max_bytes {
            return Err(AssemblyError::unsupported(
                "item-set byte ceiling cannot increase",
            ));
        }
        self.graph.limits.max_bytes = ceiling;
        if self.graph.usage.bytes > ceiling {
            return Err(AssemblyError::resource("shared item/set byte bound"));
        }
        Ok(())
    }
    fn run(
        &mut self,
        phase: ItemSetPhase,
        body: impl FnOnce(&mut Self) -> Result<()>,
    ) -> Result<()> {
        if self.phase != phase {
            return Err(AssemblyError::unsupported(
                "item-set operation is outside its required continuation",
            ));
        }
        let result = (|| {
            self.graph.charge(0, 1)?;
            self.graph.usage.operations = self
                .graph
                .usage
                .operations
                .checked_add(1)
                .ok_or_else(|| AssemblyError::resource("item-set operation overflow"))?;
            body(self)
        })();
        if let Err(error) = &result {
            self.phase = ItemSetPhase::Failed;
            self.failure = Some(error.clone());
        }
        result
    }
    /// An explicit restart after a reached error is supported. It cannot skip an
    /// unresolved activation continuation or an unfinished XML ItemSet body.
    pub fn begin_load(&mut self) -> Result<()> {
        if !matches!(
            self.phase,
            ItemSetPhase::ConstructorReady | ItemSetPhase::Failed
        ) {
            return Err(AssemblyError::unsupported(
                "item-set Load cannot bypass an unfinished continuation",
            ));
        }
        self.phase = ItemSetPhase::Loading;
        self.failure = None;
        self.pending = None;
        self.open = None;
        self.run(ItemSetPhase::Loading, |s| {
            let active = s.graph.field(s.root, "activeItemSet")?;
            s.graph.set_field(s.root, "previousActiveItemSet", active)?;
            s.graph.set_field(
                s.root,
                "activeItemSetId",
                V::Number(s.policy.defaults.reset_active_set_id),
            )?;
            s.sets = s.graph.table()?;
            s.graph.set_field(s.root, "itemSets", V::Table(s.sets))?;
            s.order = s.graph.table()?;
            s.graph
                .set_field(s.root, "itemSetOrderList", V::Table(s.order))?;
            s.trade = s.graph.table()?;
            s.graph.set_field(s.root, "trade", V::Table(s.trade))?;
            s.transforms.clear();
            Ok(())
        })
    }
    pub fn legacy_slot(&mut self, input: LegacySlotInput<'_>) -> Result<()> {
        self.run(ItemSetPhase::Loading, |s| {
            let policy = Arc::clone(&s.policy);
            let value = s.graph.field(
                s.slots,
                input.name.unwrap_or(&policy.defaults.empty_slot_name),
            )?;
            if truthy(&value) {
                let slot = table(value)?;
                s.graph
                    .set_field(slot, "selItemId", number(input.item_id))?;
                if let V::Table(activate) = s.graph.field(slot, "activate")? {
                    let active =
                        V::Boolean(input.active == Some(policy.defaults.true_token.as_str()));
                    s.graph.set_field(slot, "active", active.clone())?;
                    s.graph.set_field(activate, "state", active)?;
                }
            }
            Ok(())
        })
    }
    pub fn begin_item_set(&mut self, input: ItemSetInput<'_>) -> Result<()> {
        self.run(ItemSetPhase::Loading, |s| {
            let policy = Arc::clone(&s.policy);
            let set = s.create(
                parsed(input.id),
                input.title.unwrap_or(&policy.defaults.default_set_title),
            )?;
            s.open = Some(set);
            s.graph.set_field(
                set,
                "useSecondWeaponSet",
                V::Boolean(
                    input.use_second_weapon_set == Some(policy.defaults.true_token.as_str()),
                ),
            )?;
            s.phase = ItemSetPhase::ItemSetOpen;
            Ok(())
        })
    }
    pub fn set_slot(&mut self, input: SetSlotInput<'_>) -> Result<()> {
        self.run(ItemSetPhase::ItemSetOpen, |s| {
            let policy = Arc::clone(&s.policy);
            let row = s.graph.field(
                s.open.unwrap(),
                input.name.unwrap_or(&policy.defaults.empty_slot_name),
            )?;
            if truthy(&row) {
                let row = table(row)?;
                s.graph.set_field(row, "selItemId", number(input.item_id))?;
                s.graph.set_field(
                    row,
                    "active",
                    V::Boolean(input.active == Some(policy.defaults.true_token.as_str())),
                )?;
                let url = s
                    .graph
                    .text(input.item_pb_url.unwrap_or(&policy.defaults.empty_url))?;
                s.graph.set_field(row, "pbURL", url)?;
                let note = s.graph.optional_text(input.note)?;
                s.graph.set_field(row, "note", note)?;
            }
            Ok(())
        })
    }
    pub fn set_rune(&mut self, input: SetRuneInput<'_>) -> Result<()> {
        self.run(ItemSetPhase::ItemSetOpen, |s| {
            let policy = Arc::clone(&s.policy);
            let row = s.graph.field(
                s.open.unwrap(),
                input.slot_name.unwrap_or(&policy.defaults.empty_slot_name),
            )?;
            if truthy(&row) {
                let row = table(row)?;
                let name = s
                    .graph
                    .text(input.rune_name.unwrap_or(&policy.defaults.empty_rune_name))?;
                s.graph.set_field(row, "runeName", name)?;
            }
            Ok(())
        })
    }
    pub fn socket_url(&mut self, input: SocketUrlInput<'_>) -> Result<()> {
        self.run(ItemSetPhase::ItemSetOpen, |s| {
            let policy = Arc::clone(&s.policy);
            let id = number(input.node_id);
            let row = s.graph.table()?;
            let url = s
                .graph
                .text(input.item_pb_url.unwrap_or(&policy.defaults.empty_url))?;
            s.graph.set_field(row, "pbURL", url)?;
            s.graph.set(s.open.unwrap(), id, V::Table(row))
        })
    }
    pub fn finish_item_set(&mut self) -> Result<()> {
        self.run(ItemSetPhase::ItemSetOpen, |s| {
            let id = s.graph.field(s.open.unwrap(), "id")?;
            s.graph.append(s.order, id)?;
            s.open = None;
            s.phase = ItemSetPhase::Loading;
            Ok(())
        })
    }
    /// A consumed XML text/CDATA child has no attribute table. Preserve the
    /// earlier weight rows and stop at the original reached indexing error.
    pub fn trade_text(&mut self) -> Result<()> {
        self.run(ItemSetPhase::Loading, |_| {
            Err(AssemblyError::source(
                "trade weight text has no attribute table",
            ))
        })
    }
    pub fn trade_weight(&mut self, input: TradeWeightInput<'_>) -> Result<()> {
        self.run(ItemSetPhase::Loading, |s| {
            let policy = Arc::clone(&s.policy);
            let row = s.graph.table()?;
            let label = s.graph.optional_text(input.label)?;
            s.graph.set_field(row, "label", label)?;
            let stat = s.graph.optional_text(input.stat)?;
            s.graph.set_field(row, "stat", stat)?;
            s.graph
                .set_field(row, "weightMult", number(input.weight_mult))?;
            for entry in &policy.power_stats.rows {
                s.graph
                    .charge(0, entry.stat.as_ref().map_or(1, |v| v.len() as u64 + 1))?;
                if entry.stat.as_deref() == input.stat {
                    let label = s.graph.optional_text(entry.label.as_deref())?;
                    s.graph.set_field(row, "label", label)?;
                    // This is an explicit diagnostic marker, not the function
                    // value. The owner-bound descriptor is available separately.
                    s.graph.set_field(
                        row,
                        "transform_present",
                        V::Boolean(entry.transform.is_some()),
                    )?;
                    s.graph.charge(std::mem::size_of::<Option<u16>>(), 1)?;
                    s.graph.append(s.trade, V::Table(row))?;
                    s.transforms.push(entry.transform);
                    break;
                }
            }
            Ok(())
        })
    }
    pub fn finish_load(&mut self, input: FinishLoadInput<'_>) -> Result<()> {
        self.run(ItemSetPhase::Loading,|s|{
            let policy=Arc::clone(&s.policy);
            if matches!(s.graph.number(s.order,1.0)?,V::Nil) {
                let set=s.create(Some(policy.defaults.first_set_id),&policy.defaults.default_set_title)?;
                s.graph.set_field(s.root,"activeItemSet",V::Table(set))?;
                s.graph.set_field(set,"useSecondWeaponSet",V::Boolean(input.use_second_weapon_set==Some(policy.defaults.true_token.as_str())))?;
                s.graph.set(s.order,V::Number(1.0),V::Number(policy.defaults.first_set_id))?;
            }
            let active=s.graph.field(s.root,"activeItemSet")?;s.graph.set_field(s.root,"previousActiveItemSet",active)?;
            let requested=parsed(input.active_item_set).unwrap_or(policy.defaults.first_set_id);
            s.pending=Some(ItemSetContinuation{requested_set:ItemNumber::new(requested),
                show_stat_differences:input.show_stat_differences.map(|v|v==policy.defaults.true_token),
                required_stage:"SetActiveItemSet: publish selection, ordered slot/rune copy, PopulateSlots, SyncLoadouts, trailing flags/ResetUndo"});
            s.phase=ItemSetPhase::AwaitingActivation;Ok(())
        })
    }
    fn create(&mut self, id: Option<f64>, title: &str) -> Result<Id> {
        let policy = Arc::clone(&self.policy);
        let set = self.graph.table()?;
        if let Some(id) = id {
            self.graph.set_field(set, "id", V::Number(id))?;
        }
        let title = self.graph.text(title)?;
        self.graph.set_field(set, "title", title)?;
        if id.is_none() {
            let mut id = policy.defaults.first_set_id;
            self.graph.set_field(set, "id", V::Number(id))?;
            while truthy(&self.graph.number(self.sets, id)?) {
                self.graph.charge(0, 1)?;
                id += policy.defaults.set_id_increment;
                self.graph.set_field(set, "id", V::Number(id))?;
            }
        }
        // Constructor-created rows are independent. Their allocation order is
        // diagnostic only; this is not a certificate for later pairs traversal.
        let keys = self.graph.copy_entries(self.slots)?;
        for (key, slot) in keys {
            let slot = table(slot)?;
            if !truthy(&self.graph.field(slot, "nodeId")?) {
                let row = self.graph.table()?;
                self.graph
                    .set_field(row, "selItemId", V::Number(policy.defaults.empty_item_id))?;
                self.graph.set(set, key, V::Table(row))?;
            }
        }
        let keys = self.graph.copy_entries(self.runes)?;
        for (key, _) in keys {
            let row = self.graph.table()?;
            let name = self.graph.text(&policy.defaults.empty_rune_name)?;
            self.graph.set_field(row, "runeName", name)?;
            self.graph.set(set, key, V::Table(row))?;
        }
        let id = self.graph.field(set, "id")?;
        self.graph.set(self.sets, id, V::Table(set))?;
        Ok(set)
    }
}
fn parsed(text: Option<&str>) -> Option<f64> {
    text.and_then(|v| parse_number(v.as_bytes()))
}
fn number(text: Option<&str>) -> V {
    parsed(text).map(V::Number).unwrap_or(V::Nil)
}
pub fn implementation_sources() -> [&'static str; 3] {
    [
        include_str!("item_sets.rs"),
        include_str!("item_sets/graph.rs"),
        include_str!("item_sets/layout.rs"),
    ]
}
