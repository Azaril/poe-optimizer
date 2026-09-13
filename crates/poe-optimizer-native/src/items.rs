//! Ordered native inventory preparation, before item-set activation and actor use.
//! Diagnostic loader completion never authorizes registration without an owned
//! assembly result. This independent prefix is not the complete ItemsTab lifecycle.
mod slot_validity;
pub use slot_validity::{PreparedItemSlotEvaluator, PreparedItemSlotRequest};

use crate::CompiledGameData;
use poe_optimizer_core::{
    build_identity::ItemRecordId,
    data::DataIdentity,
    evaluation::{EvaluationError, EvaluationErrorKind},
};
use poe_optimizer_import::{
    build_instance::{AuthoredInstanceId, ImportedBuildInstance, SourceOccurrenceId},
    item_loading::{
        BuiltinItemLoadProvider, ItemLoadMachine, ItemLoadStatus, ItemNumber, ItemScalar,
        ItemState, JewelRadiusContext, JewelRadiusErrorKind, JewelRadiusEvidence,
        JewelRadiusProvenance, assembly::AssembledItem,
    },
    item_source::{ItemSourceKind, ItemSourceUse},
    selected_view::SelectedView,
    source_xml::PobContentEntry,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Debug, Clone, Copy)]
pub struct ItemPreparationLimits {
    pub max_items: usize,
    pub max_instructions: usize,
    /// Combined serialized diagnostics and logical owned assembly construction bytes.
    pub max_state_bytes: usize,
}
impl Default for ItemPreparationLimits {
    fn default() -> Self {
        Self {
            max_items: 4096,
            max_instructions: 131072,
            max_state_bytes: 64 * 1024 * 1024,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemRecordStatus {
    NotProcessed,
    NoBase,
    Registered,
    Pending,
    SourceFailure,
}
#[derive(Debug, Clone, Serialize)]
pub struct PreparedItemRecord {
    pub instance: ItemRecordId,
    pub source: SourceOccurrenceId,
    pub authored_id: Option<String>,
    pub status: ItemRecordStatus,
    pub registration_id: Option<ItemNumber>,
    pub instructions_executed: usize,
    pub loading_state: Option<ItemState>,
    pub frontier: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct ItemPreparationFailure {
    pub source: Option<SourceOccurrenceId>,
    pub instance: Option<ItemRecordId>,
    pub source_error: bool,
    pub stage: &'static str,
    pub message: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct ItemPreparationReport {
    pub schema_version: u32,
    pub source_sha256: String,
    pub view_sha256: String,
    pub data_identity: DataIdentity,
    /// Context selected before Items load; saved Tree/Spec loading happens later.
    pub jewel_radius_context: Option<JewelRadiusEvidence>,
    pub records: Vec<PreparedItemRecord>,
    /// Successful insertions in original order, including repeated numeric IDs.
    pub registration_order: Vec<ItemRecordId>,
    pub failure: Option<ItemPreparationFailure>,
    pub frontiers: Vec<&'static str>,
}
/// Private build/data ownership; report fields are never an admission token.
pub struct PreparedItems {
    owner: ImportedBuildInstance,
    data: Arc<CompiledGameData>,
    report: ItemPreparationReport,
    assembled: BTreeMap<ItemRecordId, AssembledItem>,
    /// Numeric equality lookup only. This does not claim original Lua hash order.
    winners: BTreeMap<u64, ItemRecordId>,
}
impl PreparedItems {
    pub fn report(&self) -> &ItemPreparationReport {
        &self.report
    }
    pub fn item(&self, id: ItemRecordId) -> Option<&AssembledItem> {
        self.assembled.get(&id)
    }
    pub fn registered_id(&self, source_id: f64) -> Option<ItemRecordId> {
        if source_id.is_nan() {
            None
        } else {
            self.winners.get(&numeric_key(source_id)).copied()
        }
    }
    pub fn validate_binding(
        &self,
        build: &ImportedBuildInstance,
        view: &SelectedView<'_>,
        data: &Arc<CompiledGameData>,
    ) -> Result<(), EvaluationError> {
        view.validate_binding(build, data.snapshot())
            .map_err(|e| contract(e.to_string()))?;
        if !self.owner.shares_storage_with(build)
            || !Arc::ptr_eq(&self.data, data)
            || self.report.view_sha256 != view_digest(view)?
        {
            return Err(contract(
                "item preparation belongs to another build, view or compiled data owner",
            ));
        }
        Ok(())
    }
}
fn numeric_key(v: f64) -> u64 {
    if v == 0.0 { 0 } else { v.to_bits() }
}
fn contract(message: impl Into<String>) -> EvaluationError {
    EvaluationError::new(EvaluationErrorKind::BackendContract, message)
}
fn resource(name: &'static str) -> EvaluationError {
    EvaluationError::new(
        EvaluationErrorKind::InvalidRequest,
        format!("native item preparation exceeds {name} limit"),
    )
}
fn view_digest(view: &SelectedView<'_>) -> Result<String, EvaluationError> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(view.report()).map_err(|e| contract(e.to_string()))?)
    ))
}
/// Prepare the exact authored inventory prefix. Unknown dependencies stop the
/// ordered source stream; later records remain present but explicitly unexecuted.
pub fn prepare_authored_items(
    build: &ImportedBuildInstance,
    view: &SelectedView<'_>,
    data: &Arc<CompiledGameData>,
    limits: ItemPreparationLimits,
) -> Result<PreparedItems, EvaluationError> {
    view.validate_binding(build, data.snapshot())
        .map_err(|e| contract(e.to_string()))?;
    let mut report = ItemPreparationReport {
        schema_version: 3,
        source_sha256: build.source_sha256().into(),
        view_sha256: view_digest(view)?,
        data_identity: data.identity().clone(),
        jewel_radius_context: None,
        records: Vec::new(),
        registration_order: Vec::new(),
        failure: None,
        frontiers: vec![
            "item_set_activation",
            "equipment_participation",
            "actor_item_effects",
        ],
    };
    let source_by_start = build
        .occurrences()
        .iter()
        .map(|o| (o.range().start, o.id()))
        .collect::<BTreeMap<_, _>>();
    let instance_by_source = build
        .instances()
        .iter()
        .filter_map(|b| match b.instance() {
            AuthoredInstanceId::ItemRecord(id) => Some((b.source(), id)),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    let mut assembled = BTreeMap::new();
    let mut winners = BTreeMap::new();
    let projection = match build.project_items() {
        Ok(p) => Some(p),
        Err(error) => {
            report.failure = Some(ItemPreparationFailure {
                source: None,
                instance: None,
                source_error: false,
                stage: "item_source_projection",
                message: error.to_string(),
            });
            None
        }
    };
    if let Some(projection) = &projection {
        for container in projection.containers() {
            for node in container
                .children()
                .iter()
                .filter(|n| n.kind() == ItemSourceKind::Item)
            {
                if report.records.len() >= limits.max_items {
                    return Err(resource("items"));
                }
                let source = *source_by_start
                    .get(&node.element().source_range().start)
                    .ok_or_else(|| contract("item source occurrence missing"))?;
                let instance = *instance_by_source
                    .get(&source)
                    .ok_or_else(|| contract("item instance missing"))?;
                report.records.push(PreparedItemRecord {
                    instance,
                    source,
                    authored_id: node.element().attribute("id").map(|v| v.decoded().into()),
                    status: ItemRecordStatus::NotProcessed,
                    registration_id: None,
                    instructions_executed: 0,
                    loading_state: None,
                    frontier: None,
                });
            }
        }
        let record_by_source = report
            .records
            .iter()
            .enumerate()
            .map(|(i, r)| (r.source, i))
            .collect::<BTreeMap<_, _>>();
        // Original Build initialization installs the injected startup version;
        // all saved Tree/Spec sections are deferred until after Items loading.
        // Final selected-view tree metadata therefore cannot supply this context.
        let catalog = data.snapshot().item_loading();
        let radius = match JewelRadiusContext::resolve(
            catalog,
            &catalog.policy().jewel_radius.latest_tree_version,
            JewelRadiusProvenance::BuildInitialization,
        ) {
            Ok(context) => {
                report.jewel_radius_context = Some(context.evidence().clone());
                Some(context)
            }
            Err(error) => {
                report.failure = Some(ItemPreparationFailure {
                    source: None,
                    instance: None,
                    source_error: error.kind == JewelRadiusErrorKind::Source,
                    stage: "item_radius_initialization",
                    message: error.to_string(),
                });
                None
            }
        };
        let mut provider = None;
        let mut instructions_left = limits.max_instructions;
        let mut state_bytes_left = limits.max_state_bytes;
        'containers: for (container_index, container) in projection.containers().iter().enumerate()
        {
            let Some(radius) = &radius else { break };
            if container.source_use() == ItemSourceUse::NamespaceUnknown {
                report.failure = Some(ItemPreparationFailure {
                    source: source_by_start
                        .get(&container.element().source_range().start)
                        .copied(),
                    instance: None,
                    source_error: false,
                    stage: "item_namespace_context",
                    message: "Namespaced inventory source has no admitted loading semantics".into(),
                });
                break;
            }
            if container_index != 0 {
                report.failure = Some(ItemPreparationFailure {
                    source: source_by_start
                        .get(&container.element().source_range().start)
                        .copied(),
                    instance: None,
                    source_error: false,
                    stage: "item_root_lifecycle",
                    message: "Repeated Items section requires its complete root lifecycle".into(),
                });
                break;
            }
            for node in container.children() {
                let source = *source_by_start
                    .get(&node.element().source_range().start)
                    .ok_or_else(|| contract("item-stream occurrence missing"))?;
                // No later item may pass an unexecuted source instruction that
                // could fail or change the surrounding ItemsTab state.
                if node.kind() != ItemSourceKind::Item {
                    report.failure = Some(ItemPreparationFailure {
                        source: Some(source),
                        instance: None,
                        source_error: false,
                        stage: "item_container_continuation",
                        message: format!(
                            "Inventory prefix stops before {:?}; complete item-set/slot/trade loading remains pending",
                            node.kind()
                        ),
                    });
                    break 'containers;
                }
                let row = &mut report.records[*record_by_source
                    .get(&source)
                    .ok_or_else(|| contract("item record missing"))?];
                let provider =
                    provider.get_or_insert_with(|| BuiltinItemLoadProvider::new(data.snapshot()));
                let mut machine = ItemLoadMachine::new(data.snapshot().item_loading());
                machine
                    .set_jewel_radius_context(radius.clone())
                    .map_err(|e| contract(e.to_string()))?;
                let attributes = node
                    .element()
                    .attributes()
                    .iter()
                    .map(|a| (a.name().to_owned(), a.value().decoded().to_owned()))
                    .collect();
                machine.set_xml_attributes(&attributes);
                let mut failure = None;
                for entry in node.ordered_content().consumed() {
                    instructions_left = instructions_left
                        .checked_sub(1)
                        .ok_or_else(|| resource("instructions"))?;
                    row.instructions_executed += 1;
                    let result = match entry {
                        PobContentEntry::Text { text, .. } => machine.apply_text(text, provider),
                        PobContentEntry::Element { child_index } => {
                            let child = node
                                .children()
                                .get(*child_index)
                                .ok_or_else(|| contract("item child missing"))?;
                            if child.source_use() == ItemSourceUse::NamespaceUnknown {
                                failure = Some(
                                    "Namespaced item instruction has no admitted loading semantics"
                                        .into(),
                                );
                                break;
                            }
                            if child.kind() == ItemSourceKind::ModRange {
                                machine.apply_mod_range(
                                    child.element().attribute("id").map(|v| v.decoded()),
                                    child.element().attribute("range").map(|v| v.decoded()),
                                )
                            } else {
                                Ok(())
                            }
                        }
                    };
                    if let Err(error) = result {
                        if machine.status() != ItemLoadStatus::SourceError {
                            return Err(EvaluationError::new(
                                EvaluationErrorKind::InvalidRequest,
                                error.to_string(),
                            ));
                        }
                        failure = Some(error.to_string());
                        break;
                    }
                    if machine.status() == ItemLoadStatus::Pending {
                        break;
                    }
                }
                if failure.is_none()
                    && !matches!(
                        machine.status(),
                        ItemLoadStatus::Pending | ItemLoadStatus::SourceError
                    )
                {
                    instructions_left = instructions_left
                        .checked_sub(1)
                        .ok_or_else(|| resource("instructions"))?;
                    row.instructions_executed += 1;
                    if let Err(error) = machine.finish_load(provider) {
                        if machine.status() != ItemLoadStatus::SourceError {
                            return Err(EvaluationError::new(
                                EvaluationErrorKind::InvalidRequest,
                                error.to_string(),
                            ));
                        }
                        failure = Some(error.to_string());
                    }
                }
                // Count before cloning, without materializing a second JSON buffer.
                let mut counter = ByteBudget {
                    left: state_bytes_left,
                };
                serde_json::to_writer(&mut counter, machine.state())
                    .map_err(|_| resource("retained state bytes"))?;
                state_bytes_left = counter.left;
                if let Some(item) = machine.assembly_progress() {
                    state_bytes_left = state_bytes_left
                        .checked_sub(item.usage().bytes)
                        .ok_or_else(|| resource("retained assembly bytes"))?;
                }
                row.loading_state = Some(machine.state().clone());
                if failure.is_some() || machine.status() == ItemLoadStatus::SourceError {
                    let source_error = machine.status() == ItemLoadStatus::SourceError;
                    row.status = if source_error {
                        ItemRecordStatus::SourceFailure
                    } else {
                        ItemRecordStatus::Pending
                    };
                    row.frontier =
                        Some(failure.unwrap_or_else(|| "source item loading failed".into()));
                    report.failure = Some(ItemPreparationFailure {
                        source: Some(source),
                        instance: Some(row.instance),
                        source_error,
                        stage: "item_loading",
                        message: row.frontier.clone().unwrap(),
                    });
                    break 'containers;
                }
                if machine.status() == ItemLoadStatus::Pending {
                    row.status = ItemRecordStatus::Pending;
                    row.frontier = Some(
                        machine
                            .pending()
                            .map(|p| p.message.clone())
                            .unwrap_or_else(|| "item dependency unavailable".into()),
                    );
                    report.failure = Some(ItemPreparationFailure {
                        source: Some(source),
                        instance: Some(row.instance),
                        source_error: false,
                        stage: "item_loading",
                        message: row.frontier.clone().unwrap(),
                    });
                    break 'containers;
                }
                if !machine.state().base_present {
                    row.status = ItemRecordStatus::NoBase;
                    continue;
                }
                let Some(item) = machine.assembled() else {
                    row.status = ItemRecordStatus::Pending;
                    row.frontier = Some("Complete owned native item assembly is unavailable; diagnostic completion cannot register an item".into());
                    report.failure = Some(ItemPreparationFailure {
                        source: Some(source),
                        instance: Some(row.instance),
                        source_error: false,
                        stage: "item_assembly",
                        message: row.frontier.clone().unwrap(),
                    });
                    break 'containers;
                };
                let id = match machine.state().retained_fields.get("id") {
                    Some(ItemScalar::Number(n)) => n.value().filter(|v| !v.is_nan()),
                    _ => None,
                }
                .ok_or_else(|| {
                    contract("successful item loader did not validate its registration ID")
                })?;
                // Original assignment precedes appending the order entry. Equal
                // IDs replace lookup winners without erasing either occurrence.
                winners.insert(numeric_key(id), row.instance);
                report.registration_order.push(row.instance);
                assembled.insert(row.instance, item.clone());
                row.registration_id = Some(ItemNumber::new(id));
                row.status = ItemRecordStatus::Registered;
            }
        }
    }
    Ok(PreparedItems {
        owner: build.clone(),
        data: Arc::clone(data),
        report,
        assembled,
        winners,
    })
}

struct ByteBudget {
    left: usize,
}
impl std::io::Write for ByteBudget {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.left = self
            .left
            .checked_sub(bytes.len())
            .ok_or_else(|| std::io::Error::other("item report byte limit"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
