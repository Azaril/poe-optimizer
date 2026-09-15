//! Ordered routing of recorded reference projections into owned query drafts.
//!
//! This import-side ledger retains source-confirmed absent targets without inventing
//! Core actors. It certifies routing and exact input associations, never numerical
//! results, independent absence parity, source authenticity or objective suitability.
use crate::owned_normalize::NormalizedImport;
use poe_optimizer_core::{
    BuildSummary,
    build_identity::*,
    coverage::{BuildCoverage, SkillActor},
    data::DataIdentity,
    evaluation::{BackendIdentity, EvaluationError, EvaluationResult},
    metrics::{ActorScope, MeasurementValue, MetricQuery},
    options::EvaluationContext,
    owned_build::{QueryId, StructuralError},
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::{GameVersionNamespace, OwnedDefinitionKey},
    owned_draft::*,
};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const SNAPSHOT_MEDIA: &str = "application/vnd.poe-optimizer.pob-snapshot+json;version=2";
const SNAPSHOT_PREFIX: &str = "application/vnd.poe-optimizer.pob-snapshot+json";

#[derive(Clone, Copy, Debug)]
pub struct ProjectionLimits {
    pub draft: DraftLimits,
    pub max_report_bytes: usize,
    pub max_plan_bytes: usize,
    pub max_rows: usize,
    pub max_attachments: usize,
}
impl Default for ProjectionLimits {
    fn default() -> Self {
        Self {
            draft: DraftLimits::default(),
            max_report_bytes: 8 * 1024 * 1024,
            max_plan_bytes: 8 * 1024 * 1024,
            max_rows: 16_384,
            max_attachments: 64,
        }
    }
}
impl ProjectionLimits {
    fn validate(self) -> Result<()> {
        let hard = Self::default();
        for (name, value, maximum) in [
            ("report bytes", self.max_report_bytes, hard.max_report_bytes),
            ("plan bytes", self.max_plan_bytes, hard.max_plan_bytes),
            ("rows", self.max_rows, hard.max_rows),
            ("attachments", self.max_attachments, hard.max_attachments),
            (
                "draft entries",
                self.draft.input.max_entries,
                hard.draft.input.max_entries,
            ),
            (
                "draft collection",
                self.draft.input.max_collection_entries,
                hard.draft.input.max_collection_entries,
            ),
            (
                "draft provider path",
                self.draft.input.max_provider_steps,
                hard.draft.input.max_provider_steps,
            ),
            (
                "draft bytes",
                self.draft.input.max_wire_bytes,
                hard.draft.input.max_wire_bytes,
            ),
            ("draft issues", self.draft.max_issues, hard.draft.max_issues),
            (
                "draft candidates",
                self.draft.max_candidates_per_field,
                hard.draft.max_candidates_per_field,
            ),
        ] {
            if value == 0 || value > maximum {
                return Err(ProjectionError::InvalidLimit(name));
            }
        }
        Ok(())
    }
}
#[derive(Debug, thiserror::Error)]
pub enum ProjectionError {
    #[error("invalid reference projection {0} limit")]
    InvalidLimit(&'static str),
    #[error("reference projection exceeds {0} limit")]
    Limit(&'static str),
    #[error("unsupported recorded CLI report version {0}")]
    ReportVersion(u32),
    #[error("invalid recorded reference evidence: {0}")]
    Report(&'static str),
    #[error("recorded reference source does not match the supplied source")]
    SourceBinding,
    #[error("stale or inconsistent reference projection binding: {0}")]
    Binding(&'static str),
    #[error("reference query was not recorded: {0:?}")]
    UnknownReference(MetricQuery),
    #[error("duplicate reference projection row ID: {0:?}")]
    DuplicateRow(QueryId),
    #[error("invalid owned reference row mapping: {0}")]
    Mapping(&'static str),
    #[error("invalid joined result row IDs: {0}")]
    ResultIds(&'static str),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Recorded(#[from] EvaluationError),
    #[error(transparent)]
    Structure(#[from] StructuralError),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
}
type Result<T> = std::result::Result<T, ProjectionError>;
fn raw_digest(bytes: &[u8]) -> Result<OwnedContentDigest> {
    Ok(format!("{:x}", Sha256::digest(bytes)).parse()?)
}
fn required_option<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> std::result::Result<Option<T>, D::Error> {
    Option::<T>::deserialize(d)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CliReport {
    schema_version: u32,
    source: ReportSource,
    status: String,
    evaluation: EvaluationResult,
    #[serde(rename = "initialization")]
    _initialization: serde::de::IgnoredAny,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReportSource {
    format: String,
    xml_sha256: OwnedContentDigest,
}
// The old shared coverage DTO permits omitted Option fields. This narrow source
// witness requires explicit selected_minion, rather than turning omission into None.
#[derive(Deserialize)]
struct CoveragePresence {
    #[serde(deserialize_with = "required_option")]
    selected_minion: Option<serde::de::IgnoredAny>,
}
#[derive(Deserialize)]
struct EvaluationPresence {
    coverage: CoveragePresence,
}
#[derive(Deserialize)]
struct ReportPresence {
    evaluation: EvaluationPresence,
}
#[derive(Deserialize)]
struct SnapshotPresence {
    coverage: CoveragePresence,
}
#[derive(Deserialize)]
struct RuntimeWitness {
    upstream_revision: String,
    source_hash: String,
    adapter_hash: String,
}
#[derive(Deserialize)]
struct ActorWitness {
    #[serde(deserialize_with = "required_option")]
    skill_id: Option<String>,
    #[serde(deserialize_with = "required_option")]
    skill_name: Option<String>,
}
// Deliberately only selected metadata. Numeric actor output remains opaque outside
// this module; source snapshot types are not added to the owned Core model.
#[derive(Deserialize)]
struct SnapshotWitness {
    runtime: RuntimeWitness,
    build: BuildSummary,
    coverage: BuildCoverage,
    context: EvaluationContext,
    #[serde(deserialize_with = "required_option")]
    minion: Option<ActorWitness>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordedMinionState {
    ConfirmedAbsent,
    Present,
    Unknown,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RecordedReferenceBinding {
    pub report_sha256: OwnedContentDigest,
    pub source_sha256: OwnedContentDigest,
    pub backend: BackendIdentity,
    pub build: OwnedContentDigest,
    pub context: OwnedContentDigest,
    pub coverage: OwnedContentDigest,
    pub snapshot_sha256: Option<OwnedContentDigest>,
    pub selected_minion: RecordedMinionState,
}
/// Private construction validates complete recorded evidence. The report remains
/// a reference claim: this does not attest the source or run an independent engine.
#[derive(Debug)]
pub struct RecordedReference {
    binding: RecordedReferenceBinding,
    queries: BTreeSet<MetricQuery>,
}
impl RecordedReference {
    pub fn from_cli_report(
        bytes: &[u8],
        expected_source_sha256: OwnedContentDigest,
        limits: ProjectionLimits,
    ) -> Result<Self> {
        limits.validate()?;
        if bytes.len() > limits.max_report_bytes {
            return Err(ProjectionError::Limit("report bytes"));
        }
        let wire: CliReport = serde_json::from_slice(bytes)?;
        if wire.schema_version != 3 {
            return Err(ProjectionError::ReportVersion(wire.schema_version));
        }
        if wire.source.xml_sha256 != expected_source_sha256 {
            return Err(ProjectionError::SourceBinding);
        }
        if wire.source.format != "raw_xml" || wire.status != "experimental_evaluation" {
            return Err(ProjectionError::Report(
                "unsupported report source format or status",
            ));
        }
        let presence: ReportPresence = serde_json::from_slice(bytes)?;
        let report = wire.evaluation;
        report.validate_recorded()?;
        if report.measurements.len() > limits.max_rows {
            return Err(ProjectionError::Limit("rows"));
        }
        if report.attachments.len() > limits.max_attachments {
            return Err(ProjectionError::Limit("attachments"));
        }
        let build = digest_owned("reference-build-v1", &report.build, limits.max_report_bytes)?;
        let context = digest_owned(
            "reference-context-v1",
            &report.context,
            limits.max_report_bytes,
        )?;
        let coverage = digest_owned(
            "reference-coverage-v1",
            &report.coverage,
            limits.max_report_bytes,
        )?;
        let snapshots: Vec<_> = report
            .attachments
            .iter()
            .filter(|v| v.media_type.starts_with(SNAPSHOT_PREFIX))
            .collect();
        if snapshots.len() > 1 {
            return Err(ProjectionError::Report("multiple snapshot attachments"));
        }
        let (snapshot_sha256, selected_minion) = if let Some(attachment) = snapshots.first() {
            if attachment.media_type != SNAPSHOT_MEDIA {
                return Err(ProjectionError::Report("unsupported snapshot version"));
            }
            if attachment.content.len() > limits.max_report_bytes {
                return Err(ProjectionError::Limit("snapshot bytes"));
            }
            let witness: SnapshotWitness = serde_json::from_str(&attachment.content)?;
            let snapshot_presence: SnapshotPresence = serde_json::from_str(&attachment.content)?;
            if presence.evaluation.coverage.selected_minion.is_some()
                != snapshot_presence.coverage.selected_minion.is_some()
                || build
                    != digest_owned(
                        "reference-build-v1",
                        &witness.build,
                        limits.max_report_bytes,
                    )?
                || context
                    != digest_owned(
                        "reference-context-v1",
                        &witness.context,
                        limits.max_report_bytes,
                    )?
                || coverage
                    != digest_owned(
                        "reference-coverage-v1",
                        &witness.coverage,
                        limits.max_report_bytes,
                    )?
                || report.backend.id != "pob-poe2-mlua"
                || report.backend.rules_revision != witness.runtime.upstream_revision
                || report.backend.source_fingerprint != witness.runtime.source_hash
                || report.backend.adapter_fingerprint != witness.runtime.adapter_hash
            {
                return Err(ProjectionError::Report(
                    "snapshot metadata disagrees with recorded evaluation",
                ));
            }
            let state = match (&witness.minion, &report.coverage.selected_minion) {
                (None, None) => RecordedMinionState::ConfirmedAbsent,
                (Some(actor), Some(selected))
                    if selected.actor == SkillActor::Minion
                        && actor.skill_id == selected.skill_id
                        && actor.skill_name == selected.skill_name =>
                {
                    RecordedMinionState::Present
                }
                _ => {
                    return Err(ProjectionError::Report(
                        "snapshot minion and selected coverage disagree",
                    ));
                }
            };
            (Some(raw_digest(attachment.content.as_bytes())?), state)
        } else {
            (None, RecordedMinionState::Unknown)
        };
        if selected_minion == RecordedMinionState::ConfirmedAbsent
            && report.measurements.iter().any(|m| {
                m.query.actor == ActorScope::SelectedMinion
                    && !matches!(m.value, MeasurementValue::Unavailable { .. })
            })
        {
            return Err(ProjectionError::Report(
                "absent selected minion has a recorded numeric measurement",
            ));
        }
        Ok(Self {
            binding: RecordedReferenceBinding {
                report_sha256: raw_digest(bytes)?,
                source_sha256: expected_source_sha256,
                backend: report.backend,
                build,
                context,
                coverage,
                snapshot_sha256,
                selected_minion,
            },
            queries: report.measurements.into_iter().map(|m| m.query).collect(),
        })
    }
    pub fn binding(&self) -> &RecordedReferenceBinding {
        &self.binding
    }
    /// No metric reason or missing numerical output is used to establish absence.
    pub fn requires_evaluation(&self, query: &MetricQuery) -> Result<bool> {
        if !self.queries.contains(query) {
            return Err(ProjectionError::UnknownReference(query.clone()));
        }
        Ok(query.actor != ActorScope::SelectedMinion
            || self.binding.selected_minion != RecordedMinionState::ConfirmedAbsent)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProjectionPolicyBinding {
    pub version: OwnedDefinitionKey,
    pub game_version: GameVersionNamespace,
    pub normalization_policy: OwnedContentDigest,
    pub reward_policy: OwnedContentDigest,
    pub item_policy: OwnedContentDigest,
    pub mapping: OwnedContentDigest,
    pub mapping_source: OwnedContentDigest,
    pub registry: OwnedContentDigest,
    pub definitions: DataIdentity,
    pub skill_roles: OwnedContentDigest,
}
/// Raw caller correspondence. Absence needs None; every Evaluate row needs its
/// actual owned request, including pending fields if mapping is still incomplete.
#[derive(Clone, Debug)]
pub struct ProjectionRowInput {
    pub id: QueryId,
    pub reference: MetricQuery,
    pub request: Option<MetricRequestDraft>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceAbsenceReason {
    NoSelectedMinion,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProjectionRoute {
    Evaluate { request: Box<MetricRequestDraft> },
    KnownUnavailable { reason: ReferenceAbsenceReason },
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ProjectionRow {
    pub id: QueryId,
    pub reference: MetricQuery,
    pub route: ProjectionRoute,
}
/// Validated routing, not yet a validated standalone Core query. bind checks the
/// actual owning draft, avoiding a fabricated session solely to run its validator.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ProjectionPlan {
    reference: RecordedReferenceBinding,
    policy: ProjectionPolicyBinding,
    rows: Vec<ProjectionRow>,
}
impl ProjectionPlan {
    pub fn new(
        reference: &RecordedReference,
        policy: ProjectionPolicyBinding,
        rows: Vec<ProjectionRowInput>,
        limits: ProjectionLimits,
    ) -> Result<Self> {
        limits.validate()?;
        if rows.len() > limits.max_rows {
            return Err(ProjectionError::Limit("rows"));
        }
        policy
            .definitions
            .validate()
            .map_err(|_| ProjectionError::Binding("definition identity"))?;
        let mut seen = BTreeSet::new();
        let mut routed = Vec::with_capacity(rows.len());
        for row in rows {
            if !seen.insert(row.id.clone()) {
                return Err(ProjectionError::DuplicateRow(row.id));
            }
            let route = if reference.requires_evaluation(&row.reference)? {
                let request = row
                    .request
                    .ok_or(ProjectionError::Mapping("Evaluate row has no owned query"))?;
                if request.id != row.id {
                    return Err(ProjectionError::Mapping("owned and ledger row IDs differ"));
                }
                ProjectionRoute::Evaluate {
                    request: Box::new(request),
                }
            } else {
                if row.request.is_some() {
                    return Err(ProjectionError::Mapping(
                        "absent reference row must not fabricate an owned target",
                    ));
                }
                ProjectionRoute::KnownUnavailable {
                    reason: ReferenceAbsenceReason::NoSelectedMinion,
                }
            };
            routed.push(ProjectionRow {
                id: row.id,
                reference: row.reference,
                route,
            });
        }
        let plan = Self {
            reference: reference.binding.clone(),
            policy,
            rows: routed,
        };
        plan.identity(limits)?;
        Ok(plan)
    }
    pub fn rows(&self) -> &[ProjectionRow] {
        &self.rows
    }
    pub fn policy(&self) -> &ProjectionPolicyBinding {
        &self.policy
    }
    pub fn reference(&self) -> &RecordedReferenceBinding {
        &self.reference
    }
    pub fn identity(&self, limits: ProjectionLimits) -> Result<OwnedContentDigest> {
        limits.validate()?;
        if self.rows.len() > limits.max_rows {
            return Err(ProjectionError::Limit("rows"));
        }
        Ok(digest_owned(
            "owned-reference-plan-v1",
            self,
            limits.max_plan_bytes,
        )?)
    }
    pub fn query_draft(&self) -> QueryDraft {
        QueryDraft {
            game_version: self.policy.game_version.clone(),
            requests: DraftList {
                completion: DraftListCompletion::Complete,
                members: self
                    .rows
                    .iter()
                    .filter_map(|row| match &row.route {
                        ProjectionRoute::Evaluate { request } => Some((**request).clone()),
                        _ => None,
                    })
                    .collect(),
            },
        }
    }
    /// Exact query/draft binding that remains usable while full build selection
    /// (for example an unresolved weapon loadout) is still pending.
    pub fn validate_normalized(
        &self,
        normalized: &NormalizedImport,
        query_preset: QueryPresetId,
        limits: ProjectionLimits,
    ) -> Result<ProjectionDraftBinding> {
        let plan = self.identity(limits)?;
        normalized.draft().validate_limits(limits.draft)?;
        let sidecar = normalized.sidecar();
        let draft = normalized
            .draft()
            .digest(limits.draft.input.max_wire_bytes)?;
        let input = normalized.draft().input();
        if sidecar.source_sha256.parse::<OwnedContentDigest>()? != self.reference.source_sha256
            || sidecar.draft != draft
            || sidecar.allocator_after != input.allocator
            || sidecar.revision != input.revision
        {
            return Err(ProjectionError::Binding("normalization source or draft"));
        }
        if input.game_version != self.policy.game_version
            || sidecar.policy != self.policy.normalization_policy
            || sidecar.reward_policy != self.policy.reward_policy
            || sidecar.item_policy != self.policy.item_policy
            || sidecar.mapping != self.policy.mapping
            || sidecar.mapping_source != self.policy.mapping_source
            || sidecar.registry != self.policy.registry
            || sidecar.definitions != self.policy.definitions
            || sidecar.skill_roles != self.policy.skill_roles
        {
            return Err(ProjectionError::Binding("normalization artifacts"));
        }
        let query = input
            .query_presets
            .members
            .iter()
            .find(|p| p.id == query_preset)
            .ok_or(ProjectionError::Binding("selected query preset"))?;
        if query.queries != self.query_draft() {
            return Err(ProjectionError::Binding("selected owned query rows"));
        }
        Ok(ProjectionDraftBinding {
            plan,
            normalization: digest_owned(
                "reference-normalization-v1",
                sidecar,
                limits.max_plan_bytes,
            )?,
            draft,
            query_preset,
        })
    }
    fn binding_for(
        &self,
        normalized: &NormalizedImport,
        selection: EvaluationSelection,
        limits: ProjectionLimits,
    ) -> Result<ProjectionBinding> {
        let checked = self.validate_normalized(normalized, selection.queries, limits)?;
        let input = normalized.draft().input();
        macro_rules! selected {
            ($table:ident,$id:expr) => {{
                let id = $id;
                if !input.$table.members.iter().any(|row| row.id == id) {
                    return Err(ProjectionError::Binding(concat!(
                        "selected ",
                        stringify!($table)
                    )));
                }
            }};
        }
        selected!(character_presets, selection.build.character);
        selected!(equipment_presets, selection.build.equipment);
        selected!(allocation_presets, selection.build.allocations);
        selected!(skill_presets, selection.build.skills);
        selected!(choice_presets, selection.build.choices);
        selected!(scenario_presets, selection.scenario);
        selected!(query_presets, selection.queries);
        if !input
            .weapon_loadouts
            .members
            .contains(&selection.build.active_weapon_loadout)
        {
            return Err(ProjectionError::Binding("selected loadout"));
        }
        Ok(ProjectionBinding {
            plan: checked.plan,
            normalization: checked.normalization,
            draft: checked.draft,
            selection,
        })
    }
    /// Associate raw row positions only, including when Evaluate mappings remain
    /// pending. No value, computation status or finalized-request claim is made.
    pub fn join_ids(
        &self,
        result_ids: &[QueryId],
        limits: ProjectionLimits,
    ) -> Result<JoinedRouting> {
        let plan = self.identity(limits)?;
        if result_ids.len() > limits.max_rows {
            return Err(ProjectionError::Limit("rows"));
        }
        let mut positions = BTreeMap::new();
        for (index, id) in result_ids.iter().enumerate() {
            if positions.insert(id, index).is_some() {
                return Err(ProjectionError::ResultIds("duplicate ID"));
            }
        }
        if result_ids.len()
            != self
                .rows
                .iter()
                .filter(|r| matches!(r.route, ProjectionRoute::Evaluate { .. }))
                .count()
        {
            return Err(ProjectionError::ResultIds("missing or extra ID"));
        }
        let rows = self
            .rows
            .iter()
            .map(|row| {
                let association = match row.route {
                    ProjectionRoute::Evaluate { .. } => JoinedAssociation::Evaluate {
                        result_index: *positions
                            .get(&row.id)
                            .ok_or(ProjectionError::ResultIds("missing or extra ID"))?,
                    },
                    ProjectionRoute::KnownUnavailable { reason } => {
                        JoinedAssociation::ReferenceKnownUnavailable { reason }
                    }
                };
                Ok(JoinedProjectionRow {
                    id: row.id.clone(),
                    reference: row.reference.clone(),
                    association,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(JoinedRouting { plan, rows })
    }
    /// Bind after query extraction/normalization. No draft mutation, revision
    /// laundering or circular digest dependency is needed to exclude absent rows.
    pub fn bind(
        &self,
        normalized: &NormalizedImport,
        selection: EvaluationSelection,
        limits: ProjectionLimits,
    ) -> Result<BoundProjectionPlan> {
        Ok(BoundProjectionPlan {
            binding: self.binding_for(normalized, selection, limits)?,
            plan: self.clone(),
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProjectionDraftBinding {
    pub plan: OwnedContentDigest,
    pub normalization: OwnedContentDigest,
    pub draft: OwnedContentDigest,
    pub query_preset: QueryPresetId,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct JoinedRouting {
    pub plan: OwnedContentDigest,
    pub rows: Vec<JoinedProjectionRow>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProjectionBinding {
    pub plan: OwnedContentDigest,
    pub normalization: OwnedContentDigest,
    pub draft: OwnedContentDigest,
    pub selection: EvaluationSelection,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BoundProjectionPlan {
    binding: ProjectionBinding,
    plan: ProjectionPlan,
}
impl BoundProjectionPlan {
    pub fn binding(&self) -> &ProjectionBinding {
        &self.binding
    }
    pub fn plan(&self) -> &ProjectionPlan {
        &self.plan
    }
    pub fn validate_current(
        &self,
        normalized: &NormalizedImport,
        selection: EvaluationSelection,
        limits: ProjectionLimits,
    ) -> Result<()> {
        if self.plan.binding_for(normalized, selection, limits)? != self.binding {
            return Err(ProjectionError::Binding(
                "bound selection or exact snapshot changed",
            ));
        }
        Ok(())
    }
    /// Associate caller result positions with exactly the Evaluate row IDs. Values
    /// are deliberately absent: this is not a result validator or parity pass.
    pub fn join_ids(
        &self,
        finalized: &FinalizedDraft,
        result_ids: &[QueryId],
        limits: ProjectionLimits,
    ) -> Result<JoinedProjection> {
        self.plan.identity(limits)?;
        if finalized.draft_digest() != self.binding.draft
            || finalized.selection() != self.binding.selection
        {
            return Err(ProjectionError::Binding("finalized draft or selection"));
        }
        let expected = self
            .plan
            .query_draft()
            .to_resolved()
            .ok_or(ProjectionError::Mapping(
                "pending query cannot join finalized results",
            ))?;
        if finalized.request().queries().input() != &expected {
            return Err(ProjectionError::Binding("finalized query rows"));
        }
        let request = digest_owned(
            "owned-request-v1",
            finalized.request(),
            limits.draft.input.max_wire_bytes,
        )?;
        if request != finalized.request_digest() {
            return Err(ProjectionError::Binding("finalized request digest"));
        }
        let routed = self.plan.join_ids(result_ids, limits)?;
        let rows = routed.rows;
        Ok(JoinedProjection {
            binding: self.binding.clone(),
            request,
            rows,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum JoinedAssociation {
    Evaluate { result_index: usize },
    ReferenceKnownUnavailable { reason: ReferenceAbsenceReason },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct JoinedProjectionRow {
    pub id: QueryId,
    pub reference: MetricQuery,
    pub association: JoinedAssociation,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct JoinedProjection {
    pub binding: ProjectionBinding,
    pub request: OwnedContentDigest,
    pub rows: Vec<JoinedProjectionRow>,
}
