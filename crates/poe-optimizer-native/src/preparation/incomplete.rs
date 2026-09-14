//! Retained runtime owners for an incomplete native preparation.
//! Independent loader prefixes are not a source-ordered Build session.
use crate::{CompiledGameData, PreparationReport, configuration, items, skills};
use poe_optimizer_core::{data::DataIdentity, evaluation::*};
use poe_optimizer_import::{
    build_instance::ImportedBuildInstance,
    selected_view::{SelectedView, SelectedViewReport},
};
use std::sync::Arc;

/// The actual producer owners, moved into either outcome without reconstruction.
pub(crate) struct PreparedStages {
    pub skills: skills::PreparedSkills,
    pub configuration: configuration::PreparedConfiguration,
    pub items: items::PreparedItems,
}

/// Incomplete preparation retains its source, definitions, request and producer
/// state. It cannot calculate, admit candidates or resume a source-ordered Build
/// lifecycle until the missing producer transitions are implemented.
///
/// Only the diagnostic report can be serialized; runtime ownership is not a wire
/// format and cannot be reconstructed from a previously exported report.
///
/// ```compile_fail
/// use poe_optimizer_native::IncompletePreparation;
/// fn cannot_serialize_runtime(state: &IncompletePreparation) {
///     serde_json::to_value(state).unwrap();
/// }
/// ```
pub struct IncompletePreparation {
    source: ImportedBuildInstance,
    data: Arc<CompiledGameData>,
    identity: BackendIdentity,
    request: EvaluationRequest,
    report: PreparationReport,
    stages: PreparedStages,
}
impl IncompletePreparation {
    pub(super) fn new(
        source: ImportedBuildInstance,
        data: Arc<CompiledGameData>,
        identity: BackendIdentity,
        request: EvaluationRequest,
        report: PreparationReport,
        stages: PreparedStages,
    ) -> Self {
        Self {
            source,
            data,
            identity,
            request,
            report,
            stages,
        }
    }
    pub fn report(&self) -> &PreparationReport {
        &self.report
    }
    /// Discard runtime owners explicitly when only diagnostic output is wanted.
    pub fn into_report(self) -> PreparationReport {
        self.report
    }
    pub fn source(&self) -> &ImportedBuildInstance {
        &self.source
    }
    pub fn selected_view(&self) -> &SelectedViewReport {
        &self.report.view
    }
    pub fn data_identity(&self) -> &DataIdentity {
        self.data.identity()
    }
    pub fn backend_identity(&self) -> &BackendIdentity {
        &self.identity
    }
    pub fn request(&self) -> &EvaluationRequest {
        &self.request
    }
    pub fn authored_skills(&self) -> &skills::PreparedSkills {
        &self.stages.skills
    }
    pub fn authored_configuration(&self) -> &configuration::PreparedConfiguration {
        &self.stages.configuration
    }
    pub fn authored_items(&self) -> &items::PreparedItems {
        &self.stages.items
    }
    /// Validate real owners and resolved selection, never matching only public
    /// hashes, lineage labels, report fields or numeric set keys.
    pub fn validate_binding(
        &self,
        build: &ImportedBuildInstance,
        view: &SelectedView<'_>,
        data: &Arc<CompiledGameData>,
    ) -> Result<(), EvaluationError> {
        if !self.source.shares_storage_with(build) || !Arc::ptr_eq(&self.data, data) {
            return Err(EvaluationError::new(
                EvaluationErrorKind::BackendContract,
                "incomplete preparation belongs to another source or compiled data owner",
            ));
        }
        self.stages.skills.validate_binding(build, view, data)?;
        self.stages
            .configuration
            .validate_binding(build, view, data)?;
        self.stages.items.validate_binding(build, view, data)
    }
}
