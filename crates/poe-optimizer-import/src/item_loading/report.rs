use super::{
    ItemLoadError, ItemLoadMachine, ItemLoadProvider, ItemLoadStatus, ItemState, PendingDependency,
};
use crate::{
    item_source::{ItemProjection, ItemSourceKind},
    source_xml::PobContentEntry,
};
use poe_optimizer_core::data::DataIdentity;
use poe_optimizer_data::game_data::GameDataSnapshot;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, ops::Range};
pub const MAX_REPORTED_ITEMS: usize = 4096;
pub const MAX_REPORTED_INSTRUCTIONS: usize = 131072;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ItemOccurrence {
    pub container_index: usize,
    pub item_index: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstructionKind {
    Constructor,
    Text,
    ModRange,
    FinalAssembly,
    Ignored,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstructionStatus {
    Executed,
    Pending,
    NotExecuted,
}
#[derive(Debug, Clone, Serialize)]
pub struct InstructionReport {
    pub index: usize,
    pub consumed_index: Option<usize>,
    pub kind: InstructionKind,
    pub source_range: Option<Range<usize>>,
    pub text_sha256: Option<String>,
    pub status: InstructionStatus,
    pub error: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct ItemOccurrenceReport {
    pub source_occurrence: ItemOccurrence,
    pub source_range: Range<usize>,
    pub authored_id: Option<String>,
    pub instructions: Vec<InstructionReport>,
    pub state: ItemState,
    pub status: ItemLoadStatus,
    pub pending: Option<PendingDependency>,
}
#[derive(Debug, Clone, Serialize)]
pub struct ItemLoadingReport {
    pub schema_version: u32,
    pub source_sha256: String,
    pub data_identity: DataIdentity,
    pub implementation_sha256: String,
    pub items: Vec<ItemOccurrenceReport>,
}
/// Inspect each source occurrence independently. An unknown parser/assembly
/// dependency stops that occurrence, preserving all later instructions unexecuted.
pub fn inspect(
    projection: &ItemProjection<'_>,
    snapshot: &GameDataSnapshot,
    provider: &mut impl ItemLoadProvider,
) -> Result<ItemLoadingReport, ItemLoadError> {
    let mut reports = Vec::new();
    let mut instruction_count = 0usize;
    let mut evidence_bytes = 0usize;
    for (container_index, container) in projection.containers().iter().enumerate() {
        for (item_index, node) in container
            .children()
            .iter()
            .filter(|n| n.kind() == ItemSourceKind::Item)
            .enumerate()
        {
            if reports.len() >= MAX_REPORTED_ITEMS {
                return Err(ItemLoadError("item report count exceeds limit".into()));
            }
            let mut machine = ItemLoadMachine::new(snapshot.item_loading());
            let attributes: BTreeMap<String, String> = node
                .element()
                .attributes()
                .iter()
                .map(|a| (a.name().to_owned(), a.value().decoded().to_owned()))
                .collect();
            machine.set_xml_attributes(&attributes);
            let mut instructions = vec![InstructionReport {
                index: 0,
                consumed_index: None,
                kind: InstructionKind::Constructor,
                source_range: None,
                text_sha256: None,
                status: InstructionStatus::Executed,
                error: None,
            }];
            for (consumed_index, entry) in node.ordered_content().consumed().iter().enumerate() {
                let stopped = matches!(
                    machine.status(),
                    ItemLoadStatus::Pending | ItemLoadStatus::SourceError
                );
                let (kind, range, text_hash, result) =
                    match entry {
                        PobContentEntry::Text {
                            text,
                            fragment_indices,
                            ..
                        } => {
                            let mut range: Option<Range<usize>> = None;
                            for &index in fragment_indices {
                                let fragment =
                                    node.ordered_content().fragments().get(index).ok_or_else(
                                        || ItemLoadError("source fragment index is invalid".into()),
                                    )?;
                                let r = fragment.range();
                                range = Some(range.map_or(r.clone(), |prior| {
                                    prior.start.min(r.start)..prior.end.max(r.end)
                                }));
                            }
                            (
                                InstructionKind::Text,
                                range,
                                Some(format!("{:x}", Sha256::digest(text.as_bytes()))),
                                if stopped {
                                    Ok(())
                                } else {
                                    machine.apply_text(text, provider)
                                },
                            )
                        }
                        PobContentEntry::Element { child_index } => {
                            let child = node.children().get(*child_index).ok_or_else(|| {
                                ItemLoadError("source child index is invalid".into())
                            })?;
                            let range = Some(child.element().source_range());
                            if child.kind() == ItemSourceKind::ModRange {
                                (
                                    InstructionKind::ModRange,
                                    range,
                                    None,
                                    if stopped {
                                        Ok(())
                                    } else {
                                        machine.apply_mod_range(
                                            child.element().attribute("id").map(|v| v.decoded()),
                                            child.element().attribute("range").map(|v| v.decoded()),
                                        )
                                    },
                                )
                            } else {
                                (InstructionKind::Ignored, range, None, Ok(()))
                            }
                        }
                    };
                let error = match result {
                    Ok(()) => None,
                    Err(e) if machine.status() == ItemLoadStatus::SourceError => {
                        Some(e.to_string())
                    }
                    Err(e) => return Err(e),
                };
                let status = if stopped {
                    InstructionStatus::NotExecuted
                } else if machine.status() == ItemLoadStatus::Pending {
                    InstructionStatus::Pending
                } else {
                    InstructionStatus::Executed
                };
                instructions.push(InstructionReport {
                    index: instructions.len(),
                    consumed_index: Some(consumed_index),
                    kind,
                    source_range: range,
                    text_sha256: text_hash,
                    status,
                    error,
                });
            }
            let stopped = matches!(
                machine.status(),
                ItemLoadStatus::Pending | ItemLoadStatus::SourceError
            );
            let final_error = if !stopped {
                match machine.finish_load(provider) {
                    Ok(()) => None,
                    Err(e) if machine.status() == ItemLoadStatus::SourceError => {
                        Some(e.to_string())
                    }
                    Err(e) => return Err(e),
                }
            } else {
                None
            };
            instructions.push(InstructionReport {
                index: instructions.len(),
                consumed_index: None,
                kind: InstructionKind::FinalAssembly,
                source_range: None,
                text_sha256: None,
                status: if stopped {
                    InstructionStatus::NotExecuted
                } else if machine.status() == ItemLoadStatus::Pending {
                    InstructionStatus::Pending
                } else {
                    InstructionStatus::Executed
                },
                error: final_error,
            });
            instruction_count = instruction_count
                .checked_add(instructions.len())
                .ok_or_else(|| ItemLoadError("instruction count overflow".into()))?;
            if instruction_count > MAX_REPORTED_INSTRUCTIONS {
                return Err(ItemLoadError(
                    "instruction report count exceeds limit".into(),
                ));
            }
            evidence_bytes = evidence_bytes
                .checked_add(machine.evidence_bytes())
                .ok_or_else(|| ItemLoadError("report evidence size overflow".into()))?;
            if evidence_bytes > 64 * 1024 * 1024 {
                return Err(ItemLoadError("report evidence size bound".into()));
            }
            reports.push(ItemOccurrenceReport {
                source_occurrence: ItemOccurrence {
                    container_index,
                    item_index,
                },
                source_range: node.element().source_range(),
                authored_id: attributes.get("id").cloned(),
                instructions,
                status: machine.status(),
                pending: machine.pending().cloned(),
                state: machine.into_state(),
            });
        }
    }
    Ok(ItemLoadingReport {
        schema_version: 1,
        source_sha256: projection.source_sha256().into(),
        data_identity: snapshot.identity().clone(),
        implementation_sha256: super::implementation_fingerprint(),
        items: reports,
    })
}
