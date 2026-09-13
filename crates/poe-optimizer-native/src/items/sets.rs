//! Fixed XML operations over owned item-set state. Activation is a continuation.
use super::{ItemPreparationFailure, contract, resource};
use poe_optimizer_core::evaluation::{EvaluationError, EvaluationErrorKind};
use poe_optimizer_import::{
    build_instance::SourceOccurrenceId,
    item_loading::assembly::{AssemblyError, AssemblyErrorKind},
    item_sets::{
        FinishLoadInput, ItemSetContinuation, ItemSetInput, ItemSetPhase, ItemSetState,
        ItemSetUsage, LegacySlotInput, SetRuneInput, SetSlotInput, SocketUrlInput,
        TradeWeightInput,
    },
    item_source::{ItemSourceKind, ItemSourceNode, ItemSourceUse},
    source_xml::PobContentEntry,
};
use serde::Serialize;
use std::collections::BTreeMap;

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
    sources: &BTreeMap<usize, SourceOccurrenceId>,
    instructions_left: &mut usize,
) -> Result<Option<ItemPreparationFailure>, EvaluationError> {
    let source = *sources
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
        ItemSourceKind::ItemSet => state.begin_item_set(ItemSetInput {
            id: attr(node, "id"),
            title: attr(node, "title"),
            use_second_weapon_set: attr(node, "useSecondWeaponSet"),
        }),
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
        charge(instructions_left)?;
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
        let child_source = *sources
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
        charge(instructions_left)?;
        return result(state.finish_item_set(), Some(source), "item_set_loading");
    }
    Ok(None)
}
pub(super) fn finish(
    state: &mut ItemSetState,
    container: &ItemSourceNode<'_>,
    source: Option<SourceOccurrenceId>,
    instructions_left: &mut usize,
) -> Result<Option<ItemPreparationFailure>, EvaluationError> {
    charge(instructions_left)?;
    result(
        state.finish_load(FinishLoadInput {
            active_item_set: attr(container, "activeItemSet"),
            use_second_weapon_set: attr(container, "useSecondWeaponSet"),
            show_stat_differences: attr(container, "showStatDifferences"),
        }),
        source,
        "item_set_loading",
    )
}
