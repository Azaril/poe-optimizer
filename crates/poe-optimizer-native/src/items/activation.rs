//! Native activation context over this inventory and its exact injected data owner.
//! Startup tree writes stay owned until the shared root/tree lifecycle adopts them.
use super::{ItemRecordId, numeric_key};
use crate::CompiledGameData;
use poe_optimizer_import::{
    item_loading::{
        BuiltinItemLoadProvider, DependencyResult, ItemNumber,
        assembly::{AssembledItem, AssemblyError},
    },
    item_sets::{ItemActivationContext, ItemActivationRune, ItemSetLimits, RuneChoiceCatalog},
    item_slot_validity::{
        SlotValidityContext, SlotValidityLimits, SlotValidityProgram, SlotValidityRequest, Value,
    },
};
use std::{collections::BTreeMap, sync::Arc};
type Result<T> = std::result::Result<T, AssemblyError>;

pub(super) struct NativeActivation {
    data: Arc<CompiledGameData>,
    program: SlotValidityProgram,
    ids: Vec<f64>,
    runes: Option<DependencyResult<RuneChoiceCatalog>>,
    /// This is actual private preparation state, not an imported diagnostic map.
    startup_jewels: BTreeMap<u32, f64>,
    node_write_reservation: BTreeMap<u32, f64>,
    limits: ItemSetLimits,
    steps: u64,
    bytes: usize,
    // Cumulative native preparation charges survive draining bytes to the set.
    preparation_bytes: usize,
}
impl NativeActivation {
    pub(super) fn new(
        data: &Arc<CompiledGameData>,
        winners: &BTreeMap<u64, ItemRecordId>,
        limits: ItemSetLimits,
    ) -> Result<Self> {
        let policy = &data.snapshot().item_assembly().policy().slot_validity;
        policy
            .validate()
            .map_err(|e| AssemblyError::unsupported(e.to_string()))?;
        // Count bounded borrowed inputs before cloning the policy or allocating
        // the ID vector. These are logical storage charges, not allocator/RSS.
        struct Count {
            bytes: usize,
            ceiling: usize,
        }
        impl std::io::Write for Count {
            fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                let next = self
                    .bytes
                    .checked_add(b.len())
                    .ok_or_else(|| std::io::Error::other("activation policy byte overflow"))?;
                if next > self.ceiling {
                    return Err(std::io::Error::other("activation retained byte limit"));
                }
                self.bytes = next;
                Ok(b.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let id_bytes = winners
            .len()
            .checked_mul(std::mem::size_of::<f64>())
            .ok_or_else(|| AssemblyError::resource("activation ID byte overflow"))?;
        let ceiling = limits
            .max_bytes
            .checked_sub(id_bytes)
            .ok_or_else(|| AssemblyError::resource("activation retained byte limit"))?;
        let mut count = Count { bytes: 0, ceiling };
        serde_json::to_writer(&mut count, policy)
            .map_err(|e| AssemblyError::resource(e.to_string()))?;
        let input_bytes = id_bytes + count.bytes; // Both parts fit the ceiling above.
        let program = SlotValidityProgram::new(
            policy,
            SlotValidityLimits {
                max_steps: limits.max_steps,
                max_text_bytes: limits.max_string_bytes,
                max_compiled_bytes: limits
                    .max_compiled_bytes
                    .min(limits.max_bytes - input_bytes),
                ..Default::default()
            },
        )?;
        let bytes = input_bytes
            .checked_add(program.compiled_bytes())
            .ok_or_else(|| AssemblyError::resource("activation retained byte overflow"))?;
        Ok(Self {
            data: Arc::clone(data),
            program,
            ids: winners.keys().map(|id| f64::from_bits(*id)).collect(),
            runes: None,
            startup_jewels: BTreeMap::new(),
            node_write_reservation: BTreeMap::new(),
            limits,
            steps: 0,
            bytes,
            preparation_bytes: bytes,
        })
    }
    pub(super) fn bind<'a>(
        &'a mut self,
        assembled: &'a BTreeMap<ItemRecordId, AssembledItem>,
        winners: &'a BTreeMap<u64, ItemRecordId>,
    ) -> ActivationContext<'a> {
        ActivationContext {
            owner: self,
            assembled,
            winners,
        }
    }
    pub(super) fn startup_jewels(&self) -> &BTreeMap<u32, f64> {
        &self.startup_jewels
    }
    fn reserve_preparation(&mut self, bytes: usize) -> Result<()> {
        let total = self
            .preparation_bytes
            .checked_add(bytes)
            .ok_or_else(|| AssemblyError::resource("activation preparation byte overflow"))?;
        if total > self.limits.max_bytes {
            return Err(AssemblyError::resource("activation preparation byte limit"));
        }
        let pending = self
            .bytes
            .checked_add(bytes)
            .ok_or_else(|| AssemblyError::resource("activation pending byte overflow"))?;
        self.preparation_bytes = total;
        self.bytes = pending;
        Ok(())
    }
    fn prepare_runes(&mut self) {
        if self.runes.is_some() {
            return;
        }
        // Source rune initialization precedes authored item parsing. Use a fresh
        // stable finite parser dependency; no post-item cache is injected here.
        let mut provider = BuiltinItemLoadProvider::new(self.data.snapshot());
        let prepared = RuneChoiceCatalog::prepare(
            self.data.snapshot().item_loading(),
            &self.data.snapshot().item_assembly().policy().inventory,
            &mut provider,
            ItemSetLimits {
                max_bytes: self.limits.max_bytes - self.preparation_bytes,
                ..self.limits
            },
        );
        self.steps = self
            .steps
            .saturating_add(prepared.usage.steps)
            .saturating_add(prepared.usage.pattern_steps);
        self.runes = Some(match self.reserve_preparation(prepared.usage.bytes) {
            Ok(()) => prepared.result,
            Err(error) => DependencyResult::ResourceError(error.message),
        });
    }
}
pub(super) struct ActivationContext<'a> {
    owner: &'a mut NativeActivation,
    assembled: &'a BTreeMap<ItemRecordId, AssembledItem>,
    winners: &'a BTreeMap<u64, ItemRecordId>,
}
fn lookup<'a>(
    assembled: &'a BTreeMap<ItemRecordId, AssembledItem>,
    winners: &BTreeMap<u64, ItemRecordId>,
    id: f64,
) -> Option<&'a AssembledItem> {
    winners
        .get(&numeric_key(id))
        .and_then(|key| assembled.get(key))
}
impl ActivationContext<'_> {
    fn item(&self, id: f64) -> Result<&AssembledItem> {
        lookup(self.assembled, self.winners, id)
            .ok_or_else(|| AssemblyError::source("activation indexed a missing selected item"))
    }
}
impl ItemActivationContext for ActivationContext<'_> {
    fn inventory_ids(&self) -> &[f64] {
        &self.owner.ids
    }
    fn valid_for_slot(&mut self, id: f64, slot: &str, active: Value<'_>) -> Result<bool> {
        let item = lookup(self.assembled, self.winners, id)
            .ok_or_else(|| AssemblyError::source("population indexed a missing item"))?;
        let mut context = StartupValidity {
            nodes: &self
                .owner
                .data
                .snapshot()
                .item_assembly()
                .policy()
                .inventory
                .layout
                .passive
                .nodes
                .validity_nodes,
            assembled: self.assembled,
            winners: self.winners,
        };
        let (result, steps) = self.owner.program.check_with_usage(
            SlotValidityRequest {
                item: Value::item(item),
                slot_name: slot,
                item_set: active,
                flag_state: Value::Nil,
            },
            &mut context,
        );
        self.owner.steps = self.owner.steps.saturating_add(steps);
        result.map(|v| v.truthy())
    }
    fn item_label(&mut self, id: f64) -> Result<String> {
        let item = Value::item(self.item(id)?);
        let Value::Text(rarity) = item.field("rarity")? else {
            return Err(AssemblyError::source(
                "population rarity color key is absent",
            ));
        };
        let color = self
            .owner
            .data
            .snapshot()
            .item_assembly()
            .policy()
            .inventory
            .activation
            .rarity_colors
            .get(rarity)
            .ok_or_else(|| AssemblyError::source("population rarity color is absent"))?;
        let Value::Text(name) = item.field("name")? else {
            return Err(AssemblyError::source(
                "population item name is not a string",
            ));
        };
        Ok(format!("{color}{name}"))
    }
    fn jewel_socket_count(&mut self, id: f64) -> Result<f64> {
        match Value::item(self.item(id)?).field("jewelSocketCount")? {
            Value::Nil | Value::Boolean(false) => Ok(0.0),
            Value::Number(n) => Ok(n),
            _ => Err(AssemblyError::source(
                "population compared a nonnumeric jewel socket count",
            )),
        }
    }
    fn selection_dependencies(&mut self, slot: &str) -> Result<Vec<String>> {
        let (result, steps) = self.owner.program.selection_dependencies_with_usage(slot);
        self.owner.steps = self.owner.steps.saturating_add(steps);
        result
    }
    fn take_validation_steps(&mut self) -> u64 {
        std::mem::take(&mut self.owner.steps)
    }
    fn take_preparation_bytes(&mut self) -> usize {
        std::mem::take(&mut self.owner.bytes)
    }
    fn initial_rune(&mut self, slot: &str) -> DependencyResult<ItemActivationRune> {
        self.owner.prepare_runes();
        match self.owner.runes.as_ref().unwrap() {
            DependencyResult::Available(c) => {
                let (result, steps) = c.initial_with_usage(slot);
                self.owner.steps = self.owner.steps.saturating_add(steps);
                result
            }
            DependencyResult::Unavailable(m) => DependencyResult::Unavailable(m.clone()),
            DependencyResult::SourceError(m) => DependencyResult::SourceError(m.clone()),
            DependencyResult::ResourceError(m) => DependencyResult::ResourceError(m.clone()),
        }
    }
    fn select_rune(
        &mut self,
        slot: &str,
        requested: Value<'_>,
        previous: &ItemActivationRune,
    ) -> DependencyResult<ItemActivationRune> {
        self.owner.prepare_runes();
        match self.owner.runes.as_ref().unwrap() {
            DependencyResult::Available(c) => {
                let (result, steps) = c.select_with_usage(slot, requested, previous);
                self.owner.steps = self.owner.steps.saturating_add(steps);
                result
            }
            DependencyResult::Unavailable(m) => DependencyResult::Unavailable(m.clone()),
            DependencyResult::SourceError(m) => DependencyResult::SourceError(m.clone()),
            DependencyResult::ResourceError(m) => DependencyResult::ResourceError(m.clone()),
        }
    }
    fn node_writes_available(&mut self, nodes: &[(f64, f64)]) -> Result<bool> {
        // Reserve a conservative cell allowance for both the temporary map and
        // eventual startup writes before allocating either. Charges remain
        // cumulative even if duplicate/conflicting inputs abandon this attempt.
        let bytes = nodes
            .len()
            .checked_mul(2 * std::mem::size_of::<(u32, f64)>())
            .ok_or_else(|| AssemblyError::resource("activation node reservation byte overflow"))?;
        let steps = (nodes.len() as u64)
            .checked_mul(nodes.len() as u64 + 1)
            .ok_or_else(|| AssemblyError::resource("activation node reservation work overflow"))?;
        if steps > self.owner.limits.max_steps {
            return Err(AssemblyError::resource(
                "activation node reservation work limit",
            ));
        }
        self.owner.steps = self.owner.steps.saturating_add(steps);
        self.owner.reserve_preparation(bytes)?;
        let mut reservation = BTreeMap::new();
        for (node, new) in nodes {
            if !node.is_finite()
                || node.fract() != 0.0
                || !(0.0..=u32::MAX as f64).contains(node)
                || !new.is_finite()
            {
                return Ok(false);
            }
            if reservation
                .insert(*node as u32, *new)
                .is_some_and(|old| old != *new)
            {
                return Ok(false);
            }
        }
        self.owner.node_write_reservation = reservation;
        Ok(true)
    }
    fn set_node_selection(&mut self, node: f64, new: f64, old: ItemNumber) -> DependencyResult<()> {
        if old.value() != Some(new)
            || !node.is_finite()
            || node.fract() != 0.0
            || !(0.0..=u32::MAX as f64).contains(&node)
            || self.owner.node_write_reservation.get(&(node as u32)) != Some(&new)
        {
            return DependencyResult::Unavailable(
                "node selection requires a represented cluster/tree transition".into(),
            );
        }
        self.owner.startup_jewels.insert(node as u32, new);
        DependencyResult::Available(())
    }
}
struct StartupValidity<'a> {
    nodes: &'a poe_optimizer_data::item_loading::ItemMetadataTable,
    assembled: &'a BTreeMap<ItemRecordId, AssembledItem>,
    winners: &'a BTreeMap<u64, ItemRecordId>,
}
impl<'a> SlotValidityContext<'a> for StartupValidity<'a> {
    fn tree_node(&mut self, key: Value<'_>) -> Result<Value<'a>> {
        Value::table(self.nodes).index(key)
    }
    fn effective_node(&mut self, _key: Value<'_>) -> Result<Value<'a>> {
        // Fresh PassiveSpec:Init has only inherited raw nodes, no dynamic graph.
        // A successful full raw-node miss therefore cannot find a startup override.
        Ok(Value::Nil)
    }
    fn inventory_item(&mut self, key: Value<'_>) -> Result<Value<'a>> {
        let Value::Number(id) = key else {
            return Ok(Value::Nil);
        };
        Ok(lookup(self.assembled, self.winners, id)
            .map(Value::item)
            .unwrap_or(Value::Nil))
    }
    fn has_calculation_environment(&mut self) -> Result<bool> {
        // BuildOutput occurs after root loading; CalcsTab construction and Load
        // do not create mainEnv. SlotValidity uses the injected startup defaults.
        Ok(false)
    }
}
