//! Fixed activation and population over privately produced set state.
//! Traversal-independent consumer results are admitted only after a bounded proof.
use super::*;
use crate::item_loading::DependencyResult;
use crate::item_slot_validity::Value;
use std::collections::BTreeSet;

/// Native context probes have no game-state effects and remain stable throughout
/// one activation. They may be evaluated during the order-independence proof.
/// Inventory IDs are the complete numeric lookup winners, not a Lua pairs order.
pub trait ItemActivationContext {
    fn inventory_ids(&self) -> &[f64];
    /// Drain actual cumulative validity/dependency query work, including failures.
    fn take_validation_steps(&mut self) -> u64;
    /// Drain newly retained native preparation storage (not source-host memory).
    fn take_preparation_bytes(&mut self) -> usize {
        0
    }
    /// Prove these unchanged-selection writes use owned, prevalidated storage.
    /// No source-fallible behavior or graph rebuild may remain after true.
    fn node_writes_available(&mut self, nodes: &[(f64, f64)]) -> Result<bool>;
    fn valid_for_slot(
        &mut self,
        item_id: f64,
        slot_name: &str,
        active_set: Value<'_>,
    ) -> Result<bool>;
    fn item_label(&mut self, item_id: f64) -> Result<String>;
    fn jewel_socket_count(&mut self, item_id: f64) -> Result<f64>;
    /// All active-set slots this slot's validity can read, from injected rules.
    fn selection_dependencies(&mut self, slot_name: &str) -> Result<Vec<String>>;
    fn initial_rune(&mut self, slot_name: &str) -> DependencyResult<ItemActivationRune>;
    fn select_rune(
        &mut self,
        slot_name: &str,
        requested: Value<'_>,
        previous: &ItemActivationRune,
    ) -> DependencyResult<ItemActivationRune>;
    /// Perform the actual reached spec.jewels write. Unavailable means no write;
    /// a SourceError must retain any external prefix. Cluster-changing selections
    /// are stopped separately before this method until their ordering is proved.
    fn set_node_selection(
        &mut self,
        node_id: f64,
        new_id: f64,
        old_id: ItemNumber,
    ) -> DependencyResult<()>;
}
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ItemActivationProgress {
    AwaitingDependency {
        stage: &'static str,
        message: String,
    },
    AwaitingSyncLoadouts,
}
#[derive(Clone, Copy)]
enum Stage {
    Resolve,
    Copy,
    Runes,
    Proof,
    Populate,
    Sync,
}
pub(super) struct ActivationFrame {
    stage: Stage,
    previous: Option<Id>,
    current: Option<Id>,
    slots: Vec<(String, Id)>,
    runes: Vec<(String, Id)>,
    plans: Vec<SlotPlan>,
    slot_index: usize,
    node_pending: bool,
}
struct SlotPlan {
    name: String,
    slot: Id,
    choices: Vec<(f64, String)>,
    old_selection: V,
    selection: f64,
    cleared: bool,
    socket_count: f64,
    node: Option<f64>,
    children: Vec<Id>,
}
impl ActivationFrame {
    fn new() -> Self {
        Self {
            stage: Stage::Resolve,
            previous: None,
            current: None,
            slots: Vec::new(),
            runes: Vec::new(),
            plans: Vec::new(),
            slot_index: 0,
            node_pending: false,
        }
    }
}
fn waiting(stage: &'static str, message: impl Into<String>) -> ItemActivationProgress {
    ItemActivationProgress::AwaitingDependency {
        stage,
        message: message.into(),
    }
}
fn dependency<T>(
    value: DependencyResult<T>,
    stage: &'static str,
) -> Result<std::result::Result<T, ItemActivationProgress>> {
    match value {
        DependencyResult::Available(value) => Ok(Ok(value)),
        DependencyResult::Unavailable(message) => Ok(Err(waiting(stage, message))),
        DependencyResult::SourceError(message) => Err(AssemblyError::source(message)),
        DependencyResult::ResourceError(message) => Err(AssemblyError::resource(message)),
    }
}
impl ItemSetState {
    /// Advance the pending Load's actual activation. SyncLoadouts is always an
    /// explicit next dependency; this API cannot certify whole activation/Load.
    /// Context probes must remain unchanged when resuming a suspended node write.
    pub fn continue_activation(
        &mut self,
        context: &mut impl ItemActivationContext,
    ) -> Result<ItemActivationProgress> {
        if self.phase == ItemSetPhase::AwaitingSyncLoadouts {
            return Ok(ItemActivationProgress::AwaitingSyncLoadouts);
        }
        if self.phase != ItemSetPhase::AwaitingActivation || self.pending.is_none() {
            return Err(AssemblyError::unsupported(
                "activation is outside its required continuation",
            ));
        }
        let mut frame = self.activation.take().unwrap_or_else(ActivationFrame::new);
        let result = (|| {
            self.charge_context(context)?;
            self.graph.usage.operations = self
                .graph
                .usage
                .operations
                .checked_add(1)
                .ok_or_else(|| AssemblyError::resource("activation operation overflow"))?;
            self.advance_activation(&mut frame, context)
        })();
        if let Err(error) = &result {
            self.phase = ItemSetPhase::Failed;
            self.failure = Some(error.clone());
        }
        self.activation = Some(frame);
        result
    }
    pub fn selected_rune(&self, slot_name: &str) -> Option<&ItemActivationRune> {
        self.rune_selections.get(slot_name)
    }
    fn map_slots(&mut self, map: Id) -> Result<Vec<(String, Id)>> {
        let entries = self.graph.copy_entries(map)?;
        let mut result = Vec::new();
        let mut identities = BTreeSet::new();
        for (name, slot) in entries {
            self.graph.charge(
                std::mem::size_of::<(String, Id)>() + std::mem::size_of::<u32>() * 4,
                1,
            )?;
            let V::Bytes(name) = name else {
                return Err(AssemblyError::unsupported(
                    "activation requires string slot keys",
                ));
            };
            let name = String::from_utf8(name)
                .map_err(|_| AssemblyError::unsupported("activation slot key is not UTF-8"))?;
            let slot = table(slot)?;
            if !identities.insert(slot.0) {
                return Err(AssemblyError::unsupported(
                    "activation slot controls alias across map keys",
                ));
            }
            result.push((name, slot));
        }
        Ok(result)
    }
    fn advance_activation(
        &mut self,
        f: &mut ActivationFrame,
        context: &mut impl ItemActivationContext,
    ) -> Result<ItemActivationProgress> {
        loop {
            self.graph.charge(0, 1)?;
            match f.stage {
                Stage::Resolve => {
                    // Capture the actual active object before replacing its binding.
                    f.previous = match self.graph.field(self.root, "activeItemSet")? {
                        V::Nil | V::Boolean(false) => None,
                        value => Some(table(value)?),
                    };
                    let mut requested = self
                        .pending
                        .as_ref()
                        .unwrap()
                        .requested_set
                        .value()
                        .ok_or_else(|| {
                            AssemblyError::unsupported("non-numeric activation request")
                        })?;
                    if !truthy(&self.graph.number(self.order, 1.0)?) {
                        return Ok(waiting(
                            "activation_empty_order",
                            "SetActiveItemSet NewItemSet fallback requires its complete callback transition",
                        ));
                    }
                    if !truthy(&self.graph.number(self.sets, requested)?) {
                        let V::Number(first) = self.graph.number(self.order, 1.0)? else {
                            return Err(AssemblyError::source(
                                "first item-set order value is not numeric",
                            ));
                        };
                        requested = first;
                    }
                    self.graph
                        .set_field(self.root, "activeItemSetId", V::Number(requested))?;
                    let current = self.graph.number(self.sets, requested)?;
                    self.graph
                        .set_field(self.root, "activeItemSet", current.clone())?;
                    f.current = Some(table(current)?);
                    self.graph.set_field(
                        self.root,
                        "previousActiveItemSet",
                        f.previous.map(V::Table).unwrap_or(V::Nil),
                    )?;
                    f.stage = Stage::Copy;
                }
                Stage::Copy => {
                    let proof = (|| {
                        let slots = self.map_slots(self.slots)?;
                        let runes = self.map_slots(self.runes)?;
                        let mut rows = BTreeSet::new();
                        for (name, slot) in &slots {
                            if truthy(&self.graph.field(*slot, "nodeId")?) {
                                continue;
                            }
                            let current = table(self.graph.field(f.current.unwrap(), name)?)?;
                            let previous = f
                                .previous
                                .map(|id| self.graph.field(id, name).and_then(table))
                                .transpose()?;
                            for row in [Some(current), previous.filter(|row| *row != current)]
                                .into_iter()
                                .flatten()
                            {
                                self.graph.charge(std::mem::size_of::<u32>() * 4, 1)?;
                                if !rows.insert(row.0) {
                                    return Err(AssemblyError::unsupported(
                                        "activation rows alias across slot keys",
                                    ));
                                }
                                // Force valid table lookup before admitting commuting writes.
                                let _ = self.graph.field(row, "selItemId")?;
                            }
                        }
                        Ok((slots, runes))
                    })();
                    let (slots, runes) = match proof {
                        Ok(value) => value,
                        Err(e)
                            if e.kind
                                == crate::item_loading::assembly::AssemblyErrorKind::Resource =>
                        {
                            return Err(e);
                        }
                        Err(e) => {
                            return Ok(waiting(
                                "slot_copy_order",
                                format!("unrepresented pairs/error-order: {e}"),
                            ));
                        }
                    };
                    f.slots = slots;
                    f.runes = runes;
                    for (name, slot) in &f.slots {
                        if truthy(&self.graph.field(*slot, "nodeId")?) {
                            continue;
                        }
                        if let Some(previous) = f.previous {
                            let row = table(self.graph.field(previous, name)?)?;
                            for field in ["selItemId", "active", "note"] {
                                let value = self.graph.field(*slot, field)?;
                                self.graph.set_field(row, field, value)?;
                            }
                        }
                        let row = table(self.graph.field(f.current.unwrap(), name)?)?;
                        for field in ["selItemId", "active", "note"] {
                            let value = self.graph.field(row, field)?;
                            self.graph.set_field(*slot, field, value)?;
                        }
                        if let V::Table(control) = self.graph.field(*slot, "activate")? {
                            let active = self.graph.field(*slot, "active")?;
                            self.graph.set_field(control, "state", active)?;
                        }
                    }
                    f.stage = Stage::Runes;
                }
                Stage::Runes => {
                    // Resolve the complete successful family before choosing an
                    // arbitrary native traversal. No source error prefix is
                    // invented for a fallible source pairs permutation.
                    let mut resolved = Vec::new();
                    for (name, slot) in &f.runes {
                        if !self.rune_selections.contains_key(name) {
                            let initial = context.initial_rune(name);
                            self.charge_context(context)?;
                            let handle = match initial {
                                DependencyResult::Available(value) => value,
                                DependencyResult::ResourceError(message) => {
                                    return Err(AssemblyError::resource(message));
                                }
                                DependencyResult::Unavailable(message)
                                | DependencyResult::SourceError(message) => {
                                    return Ok(waiting("rune_initial_order", message));
                                }
                            };
                            let retained = self.graph.field(*slot, "selected_name")?;
                            if !matches!(retained, V::Bytes(ref bytes) if bytes == handle.name().as_bytes())
                            {
                                return Ok(waiting(
                                    "rune_initial_state",
                                    "prepared first rune does not match the represented constructor selection",
                                ));
                            }
                            self.graph.charge(
                                name.len() + std::mem::size_of::<ItemActivationRune>(),
                                1,
                            )?;
                            self.rune_selections.insert(name.clone(), handle);
                        }
                        let previous = self.rune_selections[name].clone();
                        let requested = if f.previous == f.current {
                            // The source first replaces this same saved row.
                            self.graph.text(previous.name())?
                        } else {
                            let row = self.graph.field(f.current.unwrap(), name)?;
                            if truthy(&row) {
                                match table(row).and_then(|id| self.graph.field(id, "runeName")) {
                                    Ok(value) => value,
                                    Err(e) => {
                                        return Ok(waiting("rune_selection_order", e.to_string()));
                                    }
                                }
                            } else {
                                V::Nil
                            }
                        };
                        let requested = if truthy(&requested) {
                            requested
                        } else {
                            self.graph
                                .text(&self.policy.defaults.empty_rune_name.clone())?
                        };
                        let value = match scalar_value(&requested) {
                            Ok(value) => value,
                            Err(e) => return Ok(waiting("rune_selection_order", e.to_string())),
                        };
                        let selection = context.select_rune(name, value, &previous);
                        self.charge_context(context)?;
                        let selected = match selection {
                            DependencyResult::Available(value) => value,
                            DependencyResult::ResourceError(message) => {
                                return Err(AssemblyError::resource(message));
                            }
                            DependencyResult::Unavailable(message)
                            | DependencyResult::SourceError(message) => {
                                return Ok(waiting("rune_selection_order", message));
                            }
                        };
                        self.graph.charge(
                            std::mem::size_of::<(Id, ItemActivationRune, ItemActivationRune)>(),
                            1,
                        )?;
                        resolved.push((*slot, previous, selected));
                    }
                    for ((name, _), (slot, previous, selected)) in f.runes.iter().zip(resolved) {
                        if let Some(previous_set) = f.previous {
                            let row = self.graph.table()?;
                            let value = self.graph.text(previous.name())?;
                            self.graph.set_field(row, "runeName", value)?;
                            self.graph.set_field(previous_set, name, V::Table(row))?;
                        }
                        let selected_name = self.graph.text(selected.name())?;
                        self.graph.set_field(slot, "selected_name", selected_name)?;
                        self.rune_selections.insert(name.clone(), selected);
                    }
                    self.graph
                        .set_field(self.root, "buildFlag", V::Boolean(true))?;
                    f.stage = Stage::Proof;
                }
                Stage::Proof => match self.population_proof(f, context) {
                    Ok(plans) => {
                        f.plans = plans;
                        f.stage = Stage::Populate;
                    }
                    Err(e)
                        if e.kind == crate::item_loading::assembly::AssemblyErrorKind::Resource =>
                    {
                        return Err(e);
                    }
                    Err(e) => {
                        return Ok(waiting(
                            "population_order",
                            format!("unrepresented pairs/error-order: {e}"),
                        ));
                    }
                },
                Stage::Populate => {
                    while f.slot_index < f.plans.len() {
                        let p = &f.plans[f.slot_index];
                        if !f.node_pending {
                            self.populate_local_prefix(p)?;
                            if p.node.is_some() && p.cleared {
                                f.node_pending = true;
                            }
                        }
                        if f.node_pending {
                            let old = match p.old_selection {
                                V::Number(n) => ItemNumber::new(n),
                                _ => ItemNumber::Nil,
                            };
                            let selected =
                                context.set_node_selection(p.node.unwrap(), p.selection, old);
                            self.charge_context(context)?;
                            match dependency(selected, "node_selection")? {
                                Ok(()) => {
                                    self.graph.set_field(
                                        p.slot,
                                        "selItemId",
                                        V::Number(p.selection),
                                    )?;
                                    f.node_pending = false;
                                }
                                Err(progress) => return Ok(progress),
                            }
                        }
                        for (index, child) in p.children.iter().enumerate() {
                            self.graph.set_field(
                                *child,
                                "inactive",
                                V::Boolean(index as f64 + 1.0 > p.socket_count),
                            )?;
                        }
                        if p.node.is_none() {
                            let row = table(self.graph.field(f.current.unwrap(), &p.name)?)?;
                            let note = self.graph.field(p.slot, "note")?;
                            self.graph.set_field(row, "note", note)?;
                        }
                        f.slot_index += 1;
                    }
                    f.stage = Stage::Sync;
                }
                Stage::Sync => {
                    self.phase = ItemSetPhase::AwaitingSyncLoadouts;
                    if let Some(pending) = &mut self.pending {
                        pending.required_stage = "SyncLoadouts, trailing flags/ResetUndo";
                    }
                    return Ok(ItemActivationProgress::AwaitingSyncLoadouts);
                }
            }
        }
    }
    fn population_proof(
        &mut self,
        f: &ActivationFrame,
        context: &mut impl ItemActivationContext,
    ) -> Result<Vec<SlotPlan>> {
        let count = context.inventory_ids().len();
        self.graph.charge(
            count
                .checked_mul(std::mem::size_of::<f64>())
                .ok_or_else(|| AssemblyError::resource("inventory proof size"))?,
            (count as u64)
                .saturating_mul(count as u64)
                .saturating_add(1),
        )?;
        let mut ids = context.inventory_ids().to_vec();
        if ids.iter().any(|n| n.is_nan()) {
            return Err(AssemblyError::unsupported("inventory has NaN key"));
        }
        ids.sort_by(f64::total_cmp);
        if ids.windows(2).any(|p| p[0] == p[1]) {
            return Err(AssemblyError::unsupported(
                "inventory contains duplicate numeric lookup keys",
            ));
        }
        let mut plans = Vec::new();
        let mut child_writers = BTreeSet::new();
        for (name, slot) in &f.slots {
            self.graph
                .charge(std::mem::size_of::<SlotPlan>() + name.len(), 1)?;
            let mut choices = Vec::new();
            for id in &ids {
                self.graph.charge(0, 1)?;
                let valid = self.probe_valid(context, *id, name, f.current.unwrap())?;
                if valid {
                    let label = context.item_label(*id);
                    self.charge_context(context)?;
                    let label = label?;
                    if label.len() > self.graph.limits.max_string_bytes {
                        return Err(AssemblyError::resource("population label byte bound"));
                    }
                    self.graph
                        .charge(std::mem::size_of::<(f64, String)>() + label.len(), 1)?;
                    choices.push((*id, label));
                }
            }
            let old_selection = self.graph.field(*slot, "selItemId")?;
            let selected = if let V::Number(id) = old_selection {
                Some(id)
            } else {
                None
            };
            self.graph.charge(0, ids.len() as u64 + 1)?;
            let exists = selected.filter(|id| ids.contains(id));
            let valid = if let Some(id) = exists {
                self.probe_valid(context, id, name, f.current.unwrap())?
            } else {
                false
            };
            let selection = if valid {
                selected.unwrap()
            } else {
                self.policy.defaults.empty_item_id
            };
            let node = match self.graph.field(*slot, "nodeId")? {
                V::Nil | V::Boolean(false) => None,
                V::Number(n) => Some(n),
                _ => return Err(AssemblyError::unsupported("non-numeric node identity")),
            };
            if node.is_some() && !valid && selected != Some(selection) {
                return Err(AssemblyError::unsupported(
                    "changed node selection requires ordered cluster graph rebuilding",
                ));
            }
            let socket_count = if selection > 0.0 {
                let count = context.jewel_socket_count(selection);
                self.charge_context(context)?;
                count?
            } else {
                0.0
            };
            if !socket_count.is_finite() {
                return Err(AssemblyError::unsupported("nonfinite jewel socket count"));
            }
            // Constructor-owned child lists are dense. Prove the actual retained
            // lists still have that shape and no cross-parent competing writer;
            // never use a map traversal index as an unproved ipairs index.
            let child_list = table(self.graph.field(*slot, "jewelSocketList")?)?;
            let entries = self.graph.copy_entries(child_list)?;
            let mut children = Vec::new();
            for (index, (key, value)) in entries.into_iter().enumerate() {
                if !matches!(key, V::Number(n) if n == index as f64 + 1.0) {
                    return Err(AssemblyError::unsupported(
                        "child list lacks a dense constructor order",
                    ));
                }
                let child = table(value)?;
                self.graph.charge(std::mem::size_of::<Id>() * 5, 1)?;
                if !child_writers.insert(child.0) {
                    return Err(AssemblyError::unsupported(
                        "child inactive state has competing parent writers",
                    ));
                }
                children.push(child);
            }
            plans.push(SlotPlan {
                name: name.clone(),
                slot: *slot,
                choices,
                old_selection,
                selection,
                cleared: !valid,
                socket_count,
                node,
                children,
            });
        }
        for p in &plans {
            let dependencies = context.selection_dependencies(&p.name);
            self.charge_context(context)?;
            for dependency in dependencies? {
                self.graph
                    .charge(dependency.len(), plans.len() as u64 + 1)?;
                let target = plans.iter().find(|p| p.name == dependency).ok_or_else(|| {
                    AssemblyError::unsupported("validity dependency names a missing live slot")
                })?;
                if target.cleared
                    && !matches!(target.old_selection,V::Number(n) if n==target.selection)
                {
                    return Err(AssemblyError::unsupported(
                        "selected main-hand/parent changes make population order observable",
                    ));
                }
            }
        }
        self.graph.charge(
            plans
                .len()
                .checked_mul(std::mem::size_of::<(f64, f64)>())
                .ok_or_else(|| AssemblyError::resource("node proof bytes"))?,
            plans.len() as u64,
        )?;
        let writes = plans
            .iter()
            .filter(|p| p.cleared)
            .filter_map(|p| p.node.map(|node| (node, p.selection)))
            .collect::<Vec<_>>();
        if !writes.is_empty() {
            let available = context.node_writes_available(&writes);
            self.charge_context(context)?;
            if !available? {
                return Err(AssemblyError::unsupported(
                    "node selection storage is an unresolved dependency",
                ));
            }
        }
        Ok(plans)
    }
    fn charge_context(&mut self, context: &mut impl ItemActivationContext) -> Result<()> {
        self.graph.charge(
            context.take_preparation_bytes(),
            context.take_validation_steps(),
        )
    }
    fn probe_valid(
        &mut self,
        context: &mut impl ItemActivationContext,
        id: f64,
        name: &str,
        current: Id,
    ) -> Result<bool> {
        let value = Value::program_table(&self.graph.tables, current)?;
        let result = context.valid_for_slot(id, name, value);
        self.charge_context(context)?;
        result
    }
    fn populate_local_prefix(&mut self, p: &SlotPlan) -> Result<()> {
        // These retained lists are diagnostic headless choices. Their canonical
        // numeric order/selIndex is not represented as source pairs/UI order.
        let mut lists = Vec::new();
        for field in ["items", "list"] {
            let list = match self.graph.field(p.slot, field)? {
                V::Nil => {
                    let id = self.graph.table()?;
                    self.graph.set_field(p.slot, field, V::Table(id))?;
                    id
                }
                value => table(value)?,
            };
            for (key, _) in self.graph.copy_entries(list)? {
                self.graph.set(list, key, V::Nil)?;
            }
            lists.push(list);
        }
        self.graph
            .append(lists[0], V::Number(self.policy.defaults.empty_item_id))?;
        let label = self
            .graph
            .text(&self.policy.defaults.empty_item_label.clone())?;
        self.graph.append(lists[1], label)?;
        self.graph.set_field(p.slot, "selIndex", V::Number(1.0))?;
        for (index, (id, label)) in p.choices.iter().enumerate() {
            self.graph.append(lists[0], V::Number(*id))?;
            let label = self.graph.text(label)?;
            self.graph.append(lists[1], label)?;
            if matches!(p.old_selection,V::Number(selected) if selected==*id) {
                self.graph
                    .set_field(p.slot, "selIndex", V::Number(index as f64 + 2.0))?;
            }
        }
        if p.cleared && p.node.is_none() {
            let active = table(self.graph.field(self.root, "activeItemSet")?)?;
            let row = table(self.graph.field(active, &p.name)?)?;
            self.graph
                .set_field(row, "selItemId", V::Number(p.selection))?;
            self.graph
                .set_field(p.slot, "selItemId", V::Number(p.selection))?;
        }
        Ok(())
    }
}
fn scalar_value(value: &V) -> Result<Value<'_>> {
    Ok(match value {
        V::Nil => Value::Nil,
        V::Boolean(b) => Value::Boolean(*b),
        V::Number(n) => Value::Number(*n),
        V::Bytes(bytes) => Value::Text(
            std::str::from_utf8(bytes)
                .map_err(|_| AssemblyError::unsupported("non-UTF8 rune name"))?,
        ),
        _ => {
            return Err(AssemblyError::unsupported(
                "unrepresented table-valued rune name",
            ));
        }
    })
}

#[cfg(test)]
#[path = "activation_tests.rs"]
mod tests;
