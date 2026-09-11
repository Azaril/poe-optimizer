//! Shared source/view entry point and explicit native preparation outcomes.
use crate::preparation_report::{self, PreparationReport};
use crate::{CompiledGameData, EvaluationClock, NativeBackend, PreparedEvaluation, profile};
use poe_optimizer_core::{
    build_identity::BuildLineage, build_view::ViewRequest, evaluation::*, metrics::MetricQuery,
    options::EvaluationOptions,
};
use poe_optimizer_import::{
    ImportFormat, ImportedBuild,
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    selected_view::{ResolveLimits, SelectedView, resolve_view},
};
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// Ready is still limited to its declared native metric/capability contract.
/// Incomplete has no calculated measurements and cannot enter the hot loop.
pub enum PreparationOutcome {
    Ready(Box<PreparedEvaluation>),
    Incomplete(Box<PreparationReport>),
}
impl PreparationOutcome {
    pub fn into_ready(self) -> Result<PreparedEvaluation, EvaluationError> {
        match self {
            Self::Ready(prepared) => Ok(*prepared),
            Self::Incomplete(report) => Err(incomplete_error(&report)),
        }
    }
}
pub(crate) fn incomplete_error(report: &PreparationReport) -> EvaluationError {
    // prepare_view records this field only for UnsupportedCapability failures.
    // The compatibility evaluator must preserve that original kind/message so
    // full-document and typed candidate paths report the same numeric failure.
    // Callers needing independent missing stages retain the Incomplete outcome.
    if let Some(message) = &report.legacy_adapter_error {
        return EvaluationError::new(EvaluationErrorKind::UnsupportedCapability, message.clone());
    }
    let stages: std::collections::BTreeSet<_> =
        report.issues.iter().map(|issue| issue.stage).collect();
    EvaluationError::new(
        EvaluationErrorKind::UnsupportedCapability,
        format!(
            "Native preparation incomplete ({})",
            stages.into_iter().collect::<Vec<_>>().join(", "),
        ),
    )
}
/// Native convenience host identity allocation. Portable hosts can supply their
/// own lineage through prepare_with_lineage or imported owner/view entry points.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn host_lineage() -> Result<BuildLineage, EvaluationError> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|e| {
        EvaluationError::new(
            EvaluationErrorKind::BackendContract,
            format!("Cannot allocate build lineage: {e}"),
        )
    })?;
    Ok(BuildLineage::from_bytes(bytes))
}
#[cfg(target_arch = "wasm32")]
pub(crate) fn host_lineage() -> Result<BuildLineage, EvaluationError> {
    Err(EvaluationError::new(
        EvaluationErrorKind::InvalidRequest,
        "This host must assign build lineage explicitly; use prepare_with_lineage, prepare_view or the candidate preparation variant with lineage",
    ))
}
pub(crate) fn import_request(
    request: &EvaluationRequest,
    lineage: BuildLineage,
) -> Result<ImportedBuildInstance, EvaluationError> {
    if request.build.content.len() > poe_optimizer_import::MAX_XML_BYTES {
        return Err(EvaluationError::new(
            EvaluationErrorKind::UnsupportedCapability,
            "Native XML exceeds byte limit",
        ));
    }
    // BuildDocument already declares XML. Do not reinterpret its bytes as a
    // share code and do not derive independent instance identity from a hash.
    let source = ImportedBuild {
        xml: request.build.content.clone(),
        format: ImportFormat::RawXml,
        sha256: format!("{:x}", Sha256::digest(request.build.content.as_bytes())),
    };
    ImportedBuildInstance::from_decoded(source, lineage, InstanceImportLimits::default()).map_err(
        |error| {
            use poe_optimizer_import::{ImportError, build_instance::InstanceImportError};
            let kind = match &error {
                InstanceImportError::Import(
                    ImportError::WrongRoot { .. }
                    | ImportError::SourceTooLarge { .. }
                    | ImportError::XmlTooLarge,
                ) => EvaluationErrorKind::UnsupportedCapability,
                _ => EvaluationErrorKind::InvalidRequest,
            };
            EvaluationError::new(kind, format!("Native build import: {error}"))
        },
    )
}
pub(crate) fn saved_view<'data>(
    build: &ImportedBuildInstance,
    data: &'data CompiledGameData,
) -> Result<SelectedView<'data>, EvaluationError> {
    resolve_view(
        build,
        data.snapshot(),
        &ViewRequest::default(),
        ResolveLimits::default(),
    )
    .map_err(|e| {
        EvaluationError::new(
            EvaluationErrorKind::UnsupportedCapability,
            format!("Native view resolution: {e}"),
        )
    })
}
fn validate_request(
    options: &EvaluationOptions,
    metrics: &[MetricQuery],
) -> Result<(), EvaluationError> {
    options
        .validate()
        .map_err(|e| EvaluationError::new(EvaluationErrorKind::InvalidRequest, e))?;
    let known = crate::metric_catalog();
    let mut queries = std::collections::BTreeSet::new();
    for query in metrics {
        if !queries.insert(query) {
            return Err(EvaluationError::new(
                EvaluationErrorKind::InvalidRequest,
                "Duplicate metric query",
            ));
        }
        if !known
            .iter()
            .any(|m| m.id == query.id && m.actors.contains(&query.actor))
        {
            return Err(EvaluationError::new(
                EvaluationErrorKind::UnsupportedCapability,
                format!(
                    "Native backend does not implement {:?}.{}",
                    query.actor, query.id
                ),
            ));
        }
    }
    Ok(())
}
impl<C: EvaluationClock> NativeBackend<C> {
    /// Ordinary document evaluation and explicit callers share this boundary.
    /// Host-supplied lineage keeps preparation portable, including browser hosts.
    pub fn prepare_with_lineage(
        &self,
        request: &EvaluationRequest,
        lineage: BuildLineage,
    ) -> Result<PreparedEvaluation, EvaluationError> {
        self.prepare_request_with_lineage(request, lineage)?
            .into_ready()
    }
    /// Detailed outcome for callers which need independently discoverable missing
    /// preparation stages instead of the compatibility evaluator's concise error.
    pub fn prepare_request_with_lineage(
        &self,
        request: &EvaluationRequest,
        lineage: BuildLineage,
    ) -> Result<PreparationOutcome, EvaluationError> {
        validate_request(&request.options, &request.metrics)?;
        let build = import_request(request, lineage)?;
        let view = saved_view(&build, &self.data)?;
        self.prepare_view(&build, &view, &request.options, &request.metrics)
    }
    /// Uses the caller's exact owned source, chosen instances and injected data.
    /// Never substitutes a different saved view if this one cannot be lowered.
    pub fn prepare_view(
        &self,
        build: &ImportedBuildInstance,
        view: &SelectedView<'_>,
        options: &EvaluationOptions,
        metrics: &[MetricQuery],
    ) -> Result<PreparationOutcome, EvaluationError> {
        view.validate_binding(build, self.data.snapshot())
            .map_err(|e| {
                EvaluationError::new(EvaluationErrorKind::BackendContract, e.to_string())
            })?;
        validate_request(options, metrics)?;
        let request = EvaluationRequest {
            build: BuildDocument {
                format: BuildFormat::PathOfBuilding2Xml,
                content: build.source_xml().into(),
            },
            options: options.clone(),
            metrics: metrics.to_vec(),
        };
        let authored_skills = crate::skills::prepare_authored_skills(
            build,
            view,
            &self.data,
            crate::skills::SkillPreparationLimits::default(),
        )?;
        let profile_result = profile::parse(&request, &self.data, view).and_then(|profile| {
            profile::validate_loaded_skill_projection(build, &self.data, &authored_skills)?;
            Ok(profile)
        });
        match profile_result {
            Ok(profile) => Ok(PreparationOutcome::Ready(Box::new(PreparedEvaluation {
                data: Arc::clone(&self.data),
                identity: self.identity.clone(),
                request,
                profile,
                source: build.clone(),
                selected_view: view.report().clone(),
                authored_skills,
            }))),
            Err(error) if error.kind == EvaluationErrorKind::UnsupportedCapability => {
                let mut report = preparation_report::collect_with_skills(
                    view,
                    options,
                    metrics,
                    &authored_skills,
                )?;
                report.legacy_adapter_error = Some(error.message);
                Ok(PreparationOutcome::Incomplete(Box::new(report)))
            }
            Err(error) => Err(error),
        }
    }
}
