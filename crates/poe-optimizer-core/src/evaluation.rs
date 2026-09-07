//! Stable application boundary: no Lua values, processes, filesystem paths or PoB outputs.
use crate::{BuildSummary, coverage::BuildCoverage, metrics::*, options::*};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildFormat {
    PathOfBuilding2Xml,
}

/// An interchange document is independent of the calculation implementation.
/// A native backend can parse the same XML in Rust. Canonical mutation state is M2 work.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildDocument {
    pub format: BuildFormat,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationRequest {
    pub build: BuildDocument,
    #[serde(default)]
    pub options: EvaluationOptions,
    /// Empty means the backend's complete available catalog; never an objective function.
    #[serde(default)]
    pub metrics: Vec<MetricQuery>,
}

#[derive(Debug, Clone, Copy)]
pub struct EvaluationBudget {
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendCapabilities {
    pub id: String,
    pub build_formats: Vec<BuildFormat>,
    pub metrics: Vec<MetricDefinition>,
    /// Accepts a complete build document, rather than only resolved numeric inputs.
    /// This does not certify build legality or complete mechanic coverage.
    pub full_build_evaluation: bool,
    pub skill_selection: bool,
    pub encounter_overrides: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendIdentity {
    pub id: String,
    pub implementation_version: String,
    pub rules_revision: String,
    pub source_fingerprint: String,
    pub adapter_fingerprint: String,
}

/// Attachments are optional evidence for tools/diagnostics, never consumed by objectives.
#[derive(Debug, Serialize, Deserialize)]
pub struct DiagnosticAttachment {
    pub media_type: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub backend: BackendIdentity,
    pub build: BuildSummary,
    pub context: EvaluationContext,
    pub coverage: BuildCoverage,
    pub measurements: Vec<MetricMeasurement>,
    pub exports: Vec<BuildDocument>,
    pub warnings: Vec<String>,
    pub elapsed_ms: f64,
    /// A calculated diagnostic result does not establish legality or recommendation parity.
    pub diagnostic_only: bool,
    pub attachments: Vec<DiagnosticAttachment>,
}

impl EvaluationResult {
    /// Validate the shared recorded-result structure without consulting a live backend.
    ///
    /// This checks every measurement and numeric evidence field, including measurements
    /// that an assessment does not use. It does not authenticate the source, interpret
    /// metric units against a current catalog, or establish game legality/data semantics.
    /// Both replay consumers and live engines should validate before using a result.
    pub fn validate_recorded(&self) -> Result<(), EvaluationError> {
        let invalid = |message| EvaluationError::new(EvaluationErrorKind::BackendContract, message);
        self.context.requested.validate().map_err(|message| {
            invalid(format!(
                "Recorded evaluation options are invalid: {message}"
            ))
        })?;
        if !self.elapsed_ms.is_finite() || self.elapsed_ms < 0.0 {
            return Err(invalid(
                "Recorded elapsed time must be finite and nonnegative".into(),
            ));
        }
        for (name, values) in [
            ("config_inputs", &self.context.config_inputs),
            ("config_placeholders", &self.context.config_placeholders),
        ] {
            if values
                .values()
                .any(|value| matches!(value, Scalar::Number(number) if !number.is_finite()))
            {
                return Err(invalid(format!(
                    "Recorded context {name} contains a nonfinite number"
                )));
            }
        }
        let mut queries = BTreeSet::new();
        for measurement in &self.measurements {
            if measurement.query.id.trim().is_empty() {
                return Err(invalid("Recorded metric IDs must be nonblank".into()));
            }
            if !queries.insert(&measurement.query) {
                return Err(invalid("Duplicate recorded measurement query".into()));
            }
            if measurement.schema_version == 0 {
                return Err(invalid(
                    "Recorded metric schema versions must be positive".into(),
                ));
            }
            if matches!(measurement.value, MeasurementValue::Finite { value } if !value.is_finite())
            {
                return Err(invalid(
                    "A recorded finite measurement contains a nonfinite value".into(),
                ));
            }
        }
        if self.coverage.schema_version != 1 {
            return Err(invalid(
                "Unsupported recorded coverage schema version".into(),
            ));
        }
        fn finite_optional(value: Option<f64>, field: &str) -> Result<(), EvaluationError> {
            if value.is_some_and(|number| !number.is_finite()) {
                return Err(EvaluationError::new(
                    EvaluationErrorKind::BackendContract,
                    format!("Recorded {field} must be finite when present"),
                ));
            }
            Ok(())
        }
        for group in &self.coverage.groups {
            finite_optional(group.group_count, "coverage.groups.group_count")?;
            for gem in &group.gems {
                finite_optional(gem.count, "coverage.groups.gems.count")?;
                finite_optional(gem.level, "coverage.groups.gems.level")?;
                finite_optional(gem.quality, "coverage.groups.gems.quality")?;
            }
        }
        for skill in &self.coverage.full_dps.active_skills {
            finite_optional(skill.count, "coverage.full_dps.active_skills.count")?;
        }
        for contribution in &self.coverage.full_dps.reported_contributions {
            finite_optional(
                contribution.dps,
                "coverage.full_dps.reported_contributions.dps",
            )?;
            finite_optional(
                contribution.count,
                "coverage.full_dps.reported_contributions.count",
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationErrorKind {
    InvalidRequest,
    UnsupportedCapability,
    Timeout,
    CalculationFailed,
    BackendContract,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EvaluationError {
    pub kind: EvaluationErrorKind,
    pub message: String,
}
impl EvaluationError {
    pub fn new(kind: EvaluationErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}
impl fmt::Display for EvaluationError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(&self.message)
    }
}
impl std::error::Error for EvaluationError {}

/// Calculation implementation boundary. A native calculator need not use IPC or Lua.
/// Each call returns one complete typed result or a typed failure, never a zero fallback.
///
/// The backend owns timeout enforcement and returns [`EvaluationErrorKind::Timeout`]
/// when its budget expires. The synchronous engine cannot preempt a borrowed call:
/// native backends need cooperative deadline checks or a host execution boundary,
/// while an isolated process backend can terminate its worker. `elapsed_ms` is
/// evidence, not a substitute for enforcing the budget during calculation.
pub trait CalculationBackend {
    fn capabilities(&self) -> BackendCapabilities;
    fn calculate(
        &self,
        request: &EvaluationRequest,
        budget: EvaluationBudget,
    ) -> Result<EvaluationResult, EvaluationError>;
}

/// Objective/search/front-end boundary. Scheduling implementations may wrap this later.
pub trait EvaluationEngine {
    fn capabilities(&self) -> BackendCapabilities;
    fn evaluate(
        &self,
        request: &EvaluationRequest,
        budget: EvaluationBudget,
    ) -> Result<EvaluationResult, EvaluationError>;
}

/// Current synchronous orchestration. Resource pools/cancellation are separate M2 work.
pub struct Engine<B> {
    backend: B,
}
impl<B: CalculationBackend> Engine<B> {
    pub fn new(backend: B) -> Self {
        Self { backend }
    }
}
impl<B: CalculationBackend + ?Sized> CalculationBackend for Box<B> {
    fn capabilities(&self) -> BackendCapabilities {
        (**self).capabilities()
    }
    fn calculate(
        &self,
        request: &EvaluationRequest,
        budget: EvaluationBudget,
    ) -> Result<EvaluationResult, EvaluationError> {
        (**self).calculate(request, budget)
    }
}
impl<B: CalculationBackend> EvaluationEngine for Engine<B> {
    fn capabilities(&self) -> BackendCapabilities {
        self.backend.capabilities()
    }
    fn evaluate(
        &self,
        request: &EvaluationRequest,
        budget: EvaluationBudget,
    ) -> Result<EvaluationResult, EvaluationError> {
        let invalid = |message| EvaluationError::new(EvaluationErrorKind::InvalidRequest, message);
        request.options.validate().map_err(invalid)?;
        if budget.timeout_ms == 0 {
            return Err(invalid("Evaluation timeout must be positive".into()));
        }
        let capabilities = self.capabilities();
        if !capabilities.full_build_evaluation
            || !capabilities.build_formats.contains(&request.build.format)
            || (request.options.selection.is_some() && !capabilities.skill_selection)
            || (request.options.encounter.is_some() && !capabilities.encounter_overrides)
        {
            return Err(EvaluationError::new(
                EvaluationErrorKind::UnsupportedCapability,
                "Backend does not support the requested build or evaluation options",
            ));
        }
        let mut seen = BTreeSet::new();
        for query in &request.metrics {
            if !seen.insert(query) {
                return Err(invalid("Duplicate metric query".into()));
            }
            if !capabilities
                .metrics
                .iter()
                .any(|metric| metric.id == query.id && metric.actors.contains(&query.actor))
            {
                return Err(EvaluationError::new(
                    EvaluationErrorKind::UnsupportedCapability,
                    format!("Unsupported metric {:?}.{}", query.actor, query.id),
                ));
            }
        }
        // An omitted filter requests every declared actor/metric combination,
        // including explicit Unavailable values for absent actors or mechanics.
        let expected: BTreeSet<MetricQuery> = if request.metrics.is_empty() {
            capabilities
                .metrics
                .iter()
                .flat_map(|definition| {
                    definition.actors.iter().map(|actor| MetricQuery {
                        actor: *actor,
                        id: definition.id.clone(),
                    })
                })
                .collect()
        } else {
            request.metrics.iter().cloned().collect()
        };
        let mut result = self.backend.calculate(request, budget)?;
        if result.backend.id != capabilities.id || result.context.requested != request.options {
            return Err(EvaluationError::new(
                EvaluationErrorKind::BackendContract,
                "Backend returned a different identity or requested evaluation options",
            ));
        }
        result.validate_recorded()?;
        let mut measured = BTreeSet::new();
        for measurement in &result.measurements {
            let definition = capabilities.metrics.iter().find(|metric| {
                metric.id == measurement.query.id
                    && metric.actors.contains(&measurement.query.actor)
            });
            measured.insert(measurement.query.clone());
            if definition.is_none_or(|metric| {
                metric.unit != measurement.unit
                    || metric.schema_version != measurement.schema_version
            }) {
                return Err(EvaluationError::new(
                    EvaluationErrorKind::BackendContract,
                    "Backend returned undeclared or mistyped measurements",
                ));
            }
        }
        if !expected.is_subset(&measured) {
            return Err(EvaluationError::new(
                EvaluationErrorKind::BackendContract,
                "Backend omitted a requested measurement; return explicit unavailability",
            ));
        }
        result
            .measurements
            .retain(|value| expected.contains(&value.query));
        Ok(result)
    }
}
