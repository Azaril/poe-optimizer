//! Fixed XML operations over owned item-set state. Activation is a continuation.
use super::{ItemPreparationFailure, contract, resource};
use poe_optimizer_core::evaluation::{EvaluationError, EvaluationErrorKind};
use poe_optimizer_import::{
    build_instance::{AuthoredInstanceId, ImportedBuildInstance, SourceOccurrenceId},
    item_loading::assembly::{AssemblyError, AssemblyErrorKind},
    item_sets::{
        FinishLoadInput, ItemSetContinuation, ItemSetIdentity, ItemSetInput, ItemSetPhase,
        ItemSetRow, ItemSetState, ItemSetUsage, LegacySlotInput, SetRuneInput, SetSlotInput,
        SocketUrlInput, TradeWeightInput,
    },
    item_source::{ItemSourceKind, ItemSourceNode, ItemSourceUse},
    selected_view::{SelectionDomain, SetOrigin},
    source_xml::PobContentEntry,
};
use serde::Serialize;
use std::{
    collections::{BTreeMap, HashMap},
    mem::size_of,
};

/// Exact publication identities, retained independently of current numeric winners.
/// This private map has no traversal semantics and cannot be imported from reports.
#[derive(Default)]
pub(super) struct ItemSetLineage {
    origins: HashMap<ItemSetIdentity, SetOrigin>,
    owner: Option<ItemSetIdentity>,
    published: u64,
    reserved: bool,
}
impl ItemSetLineage {
    // Logical metadata allowance, matching the enclosing producer budget rather
    // than allocator/RSS bytes. Capacity is reserved before source-state mutation.
    const ENTRY_BYTES: usize = size_of::<(ItemSetIdentity, SetOrigin)>() + 2 * size_of::<usize>();
    pub(super) fn reserve(
        &mut self,
        bytes_left: &mut usize,
        instructions_left: &mut usize,
    ) -> Result<(), EvaluationError> {
        if self.reserved {
            return Err(contract("item-set lineage reservation already active"));
        }
        let next = bytes_left
            .checked_sub(Self::ENTRY_BYTES)
            .ok_or_else(|| resource("item-set lineage metadata"))?;
        charge(instructions_left)?;
        self.origins
            .try_reserve(1)
            .map_err(|_| resource("item-set lineage capacity"))?;
        *bytes_left = next;
        self.reserved = true;
        Ok(())
    }
    pub(super) fn capture(
        &mut self,
        state: &ItemSetState,
        before: u64,
        origin: SetOrigin,
    ) -> Result<(), EvaluationError> {
        if self
            .owner
            .as_ref()
            .is_some_and(|owner| !owner.belongs_to(state))
        {
            return Err(contract("item-set lineage belongs to another producer"));
        }
        if !self.reserved || before != self.published {
            return Err(contract("item-set lineage creation boundary mismatch"));
        }
        self.reserved = false;
        let after = state.creation_count();
        if after == before {
            return Ok(());
        }
        if before.checked_add(1) != Some(after) {
            return Err(contract(
                "item-set operation published an unobserved number of rows",
            ));
        }
        let identity = state
            .last_created_set()
            .ok_or_else(|| contract("item-set publication identity missing"))?;
        if !identity.belongs_to(state) || self.origins.contains_key(identity) {
            return Err(contract(
                "item-set publication identity is foreign or repeated",
            ));
        }
        // reserve() provided capacity before the call; cloning this opaque token
        // only retains its existing Arc. Even an enclosing Source failure keeps it.
        if self.owner.is_none() {
            self.owner = Some(identity.clone());
        }
        self.origins.insert(identity.clone(), origin);
        self.published = after;
        Ok(())
    }
    pub(super) fn origin(
        &self,
        state: &ItemSetState,
        row: &ItemSetRow<'_>,
    ) -> Result<SetOrigin, EvaluationError> {
        if !row.belongs_to(state) {
            return Err(contract("item-set origin requested for a foreign row"));
        }
        self.origins
            .get(&row.identity())
            .copied()
            .ok_or_else(|| contract("item-set row has no observed creation origin"))
    }
}

pub(super) struct ItemSetLoadContext<'a> {
    pub build: &'a ImportedBuildInstance,
    pub sources: &'a BTreeMap<usize, SourceOccurrenceId>,
    pub lineage: &'a mut ItemSetLineage,
    pub instructions_left: &'a mut usize,
    pub bytes_left: &'a mut usize,
}
impl ItemSetLoadContext<'_> {
    fn authored_origin(
        &mut self,
        source: SourceOccurrenceId,
    ) -> Result<SetOrigin, EvaluationError> {
        // Join the reached XML occurrence to this exact imported owner. Numeric
        // authored IDs and the final SelectedView never enter this association.
        for binding in self.build.instances() {
            charge(self.instructions_left)?;
            if binding.source() == source {
                if !matches!(binding.instance(), AuthoredInstanceId::ItemSet(_)) {
                    return Err(contract(
                        "item-set source is bound to another instance kind",
                    ));
                }
                return Ok(SetOrigin::Authored {
                    instance: binding.instance(),
                    source,
                });
            }
        }
        Err(contract("authored item-set instance binding missing"))
    }
    fn creating(
        &mut self,
        state: &mut ItemSetState,
        origin: SetOrigin,
        operation: impl FnOnce(&mut ItemSetState) -> Result<(), AssemblyError>,
    ) -> Result<Result<(), AssemblyError>, EvaluationError> {
        if self
            .lineage
            .owner
            .as_ref()
            .is_some_and(|owner| !owner.belongs_to(state))
        {
            return Err(contract("item-set lineage belongs to another producer"));
        }
        let before = state.creation_count();
        self.lineage
            .reserve(self.bytes_left, self.instructions_left)?;
        tighten(state, *self.bytes_left)?;
        let outcome = operation(state);
        self.lineage.capture(state, before, origin)?;
        // Source/Unsupported is preserved when no row was published, and after
        // recording a row published before a later failure in the same operation.
        Ok(outcome)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ItemSetPreparationReport {
    pub phase: ItemSetPhase,
    pub usage: ItemSetUsage,
    /// An unfinished call, never proof of effective equipment or full Load success.
    pub continuation: Option<ItemSetContinuation>,
}
impl ItemSetPreparationReport {
    pub(super) fn from_state(state: &ItemSetState) -> Self {
        Self {
            phase: state.phase(),
            usage: state.usage(),
            continuation: state.continuation().cloned(),
        }
    }
}
pub(super) fn charge(left: &mut usize) -> Result<(), EvaluationError> {
    *left = left
        .checked_sub(1)
        .ok_or_else(|| resource("instructions"))?;
    Ok(())
}
/// Producer construction shares the enclosing item/diagnostic budget; it cannot
/// use a separate full allowance after item state has already consumed it.
pub(super) fn tighten(
    state: &mut ItemSetState,
    other_bytes_left: usize,
) -> Result<(), EvaluationError> {
    state
        .tighten_byte_limit(other_bytes_left.min(state.limits().max_bytes))
        .map_err(|e| EvaluationError::new(EvaluationErrorKind::InvalidRequest, e.to_string()))
}
pub(super) fn result(
    result: Result<(), AssemblyError>,
    source: Option<SourceOccurrenceId>,
    stage: &'static str,
) -> Result<Option<ItemPreparationFailure>, EvaluationError> {
    match result {
        Ok(()) => Ok(None),
        Err(error) if error.kind == AssemblyErrorKind::Resource => Err(EvaluationError::new(
            EvaluationErrorKind::InvalidRequest,
            error.to_string(),
        )),
        Err(error) => Ok(Some(ItemPreparationFailure {
            source,
            instance: None,
            source_error: error.kind == AssemblyErrorKind::Source,
            stage,
            message: error.to_string(),
        })),
    }
}
fn attr<'a>(node: &'a ItemSourceNode<'_>, name: &str) -> Option<&'a str> {
    node.element().attribute(name).map(|v| v.decoded())
}
fn namespace(
    node: &ItemSourceNode<'_>,
    source: SourceOccurrenceId,
) -> Option<ItemPreparationFailure> {
    (node.source_use() == ItemSourceUse::NamespaceUnknown).then(|| ItemPreparationFailure {
        source: Some(source),
        instance: None,
        source_error: false,
        stage: "item_namespace_context",
        message: "Namespaced item-set instruction has no admitted loading semantics".into(),
    })
}
pub(super) fn apply(
    state: &mut ItemSetState,
    node: &ItemSourceNode<'_>,
    context: &mut ItemSetLoadContext<'_>,
) -> Result<Option<ItemPreparationFailure>, EvaluationError> {
    let source = *context
        .sources
        .get(&node.element().source_range().start)
        .ok_or_else(|| contract("item-set source occurrence missing"))?;
    if let Some(failure) = namespace(node, source) {
        return Ok(Some(failure));
    }
    let operation = match node.kind() {
        ItemSourceKind::Slot => state.legacy_slot(LegacySlotInput {
            name: attr(node, "name"),
            item_id: attr(node, "itemId"),
            active: attr(node, "active"),
        }),
        ItemSourceKind::ItemSet => {
            let origin = context.authored_origin(source)?;
            context.creating(state, origin, |state| {
                state.begin_item_set(ItemSetInput {
                    id: attr(node, "id"),
                    title: attr(node, "title"),
                    use_second_weapon_set: attr(node, "useSecondWeaponSet"),
                })
            })?
        }
        _ => Ok(()),
    };
    if let Some(failure) = result(operation, Some(source), "item_set_loading")? {
        return Ok(Some(failure));
    }
    if !matches!(
        node.kind(),
        ItemSourceKind::ItemSet | ItemSourceKind::TradeSearchWeights
    ) {
        return Ok(None);
    }
    for entry in node.ordered_content().consumed() {
        charge(context.instructions_left)?;
        let PobContentEntry::Element { child_index } = entry else {
            // String.elem is nil in the reference; String.attrib.label throws.
            if node.kind() == ItemSourceKind::TradeSearchWeights {
                return result(state.trade_text(), Some(source), "item_set_loading");
            }
            continue;
        };
        let child = node
            .children()
            .get(*child_index)
            .ok_or_else(|| contract("item-set child missing"))?;
        let child_source = *context
            .sources
            .get(&child.element().source_range().start)
            .ok_or_else(|| contract("item-set child source occurrence missing"))?;
        if let Some(failure) = namespace(child, child_source) {
            return Ok(Some(failure));
        }
        let operation = if node.kind() == ItemSourceKind::TradeSearchWeights {
            // Original Load consumes every child name here, not only Stat.
            state.trade_weight(TradeWeightInput {
                label: attr(child, "label"),
                stat: attr(child, "stat"),
                weight_mult: attr(child, "weightMult"),
            })
        } else {
            match child.kind() {
                ItemSourceKind::Slot => state.set_slot(SetSlotInput {
                    name: attr(child, "name"),
                    item_id: attr(child, "itemId"),
                    active: attr(child, "active"),
                    item_pb_url: attr(child, "itemPbURL"),
                    note: attr(child, "note"),
                }),
                ItemSourceKind::RuneSlot => state.set_rune(SetRuneInput {
                    slot_name: attr(child, "slotName"),
                    rune_name: attr(child, "runeName"),
                }),
                ItemSourceKind::SocketIdUrl => state.socket_url(SocketUrlInput {
                    node_id: attr(child, "nodeId"),
                    item_pb_url: attr(child, "itemPbURL"),
                }),
                _ => Ok(()),
            }
        };
        if let Some(failure) = result(operation, Some(child_source), "item_set_loading")? {
            return Ok(Some(failure));
        }
    }
    if node.kind() == ItemSourceKind::ItemSet {
        charge(context.instructions_left)?;
        return result(state.finish_item_set(), Some(source), "item_set_loading");
    }
    Ok(None)
}
pub(super) fn finish(
    state: &mut ItemSetState,
    container: &ItemSourceNode<'_>,
    context: &mut ItemSetLoadContext<'_>,
) -> Result<Option<ItemPreparationFailure>, EvaluationError> {
    charge(context.instructions_left)?;
    let source = *context
        .sources
        .get(&container.element().source_range().start)
        .ok_or_else(|| contract("item-set fallback container occurrence missing"))?;
    let operation = context.creating(
        state,
        SetOrigin::Default {
            domain: SelectionDomain::Items,
            container: Some(source),
        },
        |state| {
            state.finish_load(FinishLoadInput {
                active_item_set: attr(container, "activeItemSet"),
                use_second_weapon_set: attr(container, "useSecondWeaponSet"),
                show_stat_differences: attr(container, "showStatDifferences"),
            })
        },
    )?;
    result(operation, Some(source), "item_set_loading")
}

#[cfg(test)]
#[path = "sets/lineage_tests.rs"]
mod lineage_tests;
