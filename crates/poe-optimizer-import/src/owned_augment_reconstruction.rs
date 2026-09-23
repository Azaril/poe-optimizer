//! Native preparation of explicitly selected socketed-augment source lines.
//!
//! This bounded import stage preserves each socketed occurrence. It implements
//! source grouping, not modifier meaning, socket legality, activation or scaling.
//! Prepared text remains input to separately reviewed owned line recipes.
use crate::owned_augments::*;
use poe_optimizer_core::{
    build_identity::{ItemRecordId, ItemSlotUseId},
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::{ItemTemplateDefId, OwnedDefinitionKey, SocketSlotDefId},
};
use poe_optimizer_engine::item_tools::lua_number_text;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum AugmentFact<T> {
    Known(T),
    Unresolved { code: OwnedDefinitionKey },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AugmentReconstructionDialect {
    PobUpdateRunesV1,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AugmentReconstructionPolicy {
    pub schema_version: u32,
    pub version: OwnedDefinitionKey,
    pub catalog_sha256: String,
    pub dialect: AugmentReconstructionDialect,
    pub missing_stat_order: f64,
    pub bonded_display_prefix: String,
    pub bindings: Vec<AugmentTemplateBinding>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AugmentTemplateBinding {
    pub source_name: String,
    pub template: ItemTemplateDefId,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AugmentHost {
    pub item: ItemRecordId,
    pub equipment_use: ItemSlotUseId,
    pub template: ItemTemplateDefId,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SocketedAugmentOccurrence {
    pub item: ItemRecordId,
    pub equipment_use: ItemSlotUseId,
    pub template: ItemTemplateDefId,
    pub container: ItemSlotUseId,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum AugmentSocketSelection {
    Empty {
        slot: SocketSlotDefId,
    },
    Occupied {
        slot: SocketSlotDefId,
        source_name: String,
        occurrence: SocketedAugmentOccurrence,
    },
    Unresolved {
        slot: SocketSlotDefId,
        code: OwnedDefinitionKey,
    },
}
impl AugmentSocketSelection {
    fn slot(&self) -> &SocketSlotDefId {
        match self {
            Self::Empty { slot } | Self::Occupied { slot, .. } | Self::Unresolved { slot, .. } => {
                slot
            }
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AugmentCategories {
    /// Exact selected source categories, not guesses from item display names.
    pub broad: Option<String>,
    pub specific: String,
    /// Caller must establish the actual traversal order. BTree order is not a
    /// substitute for a source pairs-order witness when that order is relevant.
    pub extra_soul_core_selectors: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AugmentActivationContext {
    pub normal_enabled: AugmentFact<bool>,
    pub global_bonded_enabled: AugmentFact<bool>,
    pub item_idols_bonded_enabled: AugmentFact<bool>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AugmentMagnitudeContext {
    pub global_increase_percent: AugmentFact<f64>,
    pub rune_increase_percent: AugmentFact<f64>,
    pub soul_core_increase_percent: AugmentFact<f64>,
    pub source_magnitude_already_applied: AugmentFact<bool>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AugmentReconstructionRequest {
    pub host: AugmentHost,
    pub active_socket_count: AugmentFact<u32>,
    pub categories: AugmentFact<AugmentCategories>,
    /// Complete ordered selection, including explicitly empty sockets. Missing
    /// or surplus headers never acquire invented empty/ignored meaning here.
    pub selections: Vec<AugmentSocketSelection>,
    pub activation: AugmentActivationContext,
    pub magnitude: AugmentMagnitudeContext,
}
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AugmentLane {
    Normal,
    Bonded,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AugmentContextApplication {
    Unapplied,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedAugmentMember {
    pub selection_index: usize,
    pub slot: SocketSlotDefId,
    pub occurrence: SocketedAugmentOccurrence,
    pub source_name: String,
    pub source_selector: String,
    pub lane: AugmentLane,
    pub source_line_index: usize,
    pub source_text: String,
    pub source_stat_order: Option<f64>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AugmentMergeDiagnostic {
    DifferentIncomingSkeleton { member_index: usize },
    IgnoredIncomingNumbers { member_index: usize, count: usize },
}
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedAugmentLine {
    pub kind: AugmentKind,
    pub lane: AugmentLane,
    /// Group identity uses the source numeric rendering. Ordering uses the first
    /// encountered numeric order and keeps encounter order for equal values.
    pub order_key: String,
    pub first_stat_order: f64,
    /// Source parser input: Bonded decoration is kept separately below.
    pub text: String,
    pub display_text: String,
    /// First member's text determines meaning even when later skeletons differ.
    pub members: Vec<PreparedAugmentMember>,
    pub diagnostics: Vec<AugmentMergeDiagnostic>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AugmentReconstructionIssue {
    UnknownSocketCount,
    UnknownCategories,
    SocketMembership {
        supplied: usize,
        active: usize,
    },
    UnknownSelection {
        selection_index: usize,
    },
    UnknownAugment {
        selection_index: usize,
    },
    MissingTemplateBinding {
        selection_index: usize,
    },
    NoApplicableRows {
        selection_index: usize,
    },
    MissingNumericMatch {
        selection_index: usize,
        source_line_index: usize,
    },
    NonFiniteNumber {
        selection_index: usize,
        source_line_index: usize,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedAugmentReport {
    pub catalog_sha256: String,
    pub policy: OwnedContentDigest,
    pub request: OwnedContentDigest,
    pub host: AugmentHost,
    /// Retained even when reconstruction is unavailable.
    pub selections: Vec<AugmentSocketSelection>,
    pub lines: Vec<PreparedAugmentLine>,
    pub issues: Vec<AugmentReconstructionIssue>,
    pub activation: AugmentActivationContext,
    pub magnitude: AugmentMagnitudeContext,
    pub context_application: AugmentContextApplication,
    pub semantics: AugmentSemantics,
    pub work_used: usize,
}
#[derive(Clone, Copy, Debug)]
pub struct AugmentReconstructionLimits {
    pub max_wire_bytes: usize,
    pub max_sockets: usize,
    pub max_categories: usize,
    pub max_bindings: usize,
    pub max_members: usize,
    pub max_text_bytes: usize,
    pub max_output_bytes: usize,
    pub max_work: usize,
}
impl Default for AugmentReconstructionLimits {
    fn default() -> Self {
        Self {
            max_wire_bytes: 4 * 1024 * 1024,
            max_sockets: 256,
            max_categories: 128,
            max_bindings: 4096,
            max_members: 65_536,
            max_text_bytes: 16 * 1024,
            max_output_bytes: 4 * 1024 * 1024,
            max_work: 8 * 1024 * 1024,
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum AugmentReconstructionError {
    #[error("augment reconstruction limit: {0}")]
    Limit(&'static str),
    #[error("invalid augment reconstruction: {0}")]
    Invalid(&'static str),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
}
type Result<T> = std::result::Result<T, AugmentReconstructionError>;
struct Budget {
    limits: AugmentReconstructionLimits,
    left: usize,
    output: usize,
    members: usize,
}
impl Budget {
    fn work(&mut self, n: usize) -> Result<()> {
        self.left = self
            .left
            .checked_sub(n)
            .ok_or(AugmentReconstructionError::Limit("work"))?;
        Ok(())
    }
    fn text(&mut self, s: &str) -> Result<()> {
        if s.len() > self.limits.max_text_bytes {
            return Err(AugmentReconstructionError::Limit("text bytes"));
        }
        self.work(s.len().saturating_add(1).saturating_mul(16))
    }
    fn output(&mut self, n: usize) -> Result<()> {
        self.output = self
            .output
            .checked_add(n)
            .filter(|n| *n <= self.limits.max_output_bytes)
            .ok_or(AugmentReconstructionError::Limit("output bytes"))?;
        Ok(())
    }
    /// Charge retained representation before cloning or growing a collection.
    /// Wire bytes count owned string/identity payloads, while the inline size
    /// also bounds collection elements and their allocation bookkeeping. This
    /// deliberately conservative budget is checked again against final JSON.
    fn retain<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        let mut measure = OutputMeasure {
            bytes: 0,
            limit: self.limits.max_output_bytes.saturating_sub(self.output),
        };
        serde_json::to_writer(&mut measure, value)
            .map_err(|_| AugmentReconstructionError::Limit("output bytes"))?;
        self.output(measure.bytes.saturating_add(std::mem::size_of_val(value)))
    }
}
fn push_retained<T: Serialize>(budget: &mut Budget, values: &mut Vec<T>, value: T) -> Result<()> {
    budget.retain(&value)?;
    values.push(value);
    Ok(())
}
fn validate_limits(l: AugmentReconstructionLimits) -> Result<()> {
    let m = AugmentReconstructionLimits::default();
    for (n, a, b) in [
        ("wire bytes", l.max_wire_bytes, m.max_wire_bytes),
        ("sockets", l.max_sockets, m.max_sockets),
        ("categories", l.max_categories, m.max_categories),
        ("bindings", l.max_bindings, m.max_bindings),
        ("members", l.max_members, m.max_members),
        ("text bytes", l.max_text_bytes, m.max_text_bytes),
        ("output bytes", l.max_output_bytes, m.max_output_bytes),
        ("work", l.max_work, m.max_work),
    ] {
        if a == 0 || a > b {
            return Err(AugmentReconstructionError::Limit(n));
        }
    }
    Ok(())
}

pub fn reconstruct_owned_augments(
    catalog: &AcquiredAugmentCatalog,
    policy: &AugmentReconstructionPolicy,
    input: &AugmentReconstructionRequest,
    limits: AugmentReconstructionLimits,
) -> Result<PreparedAugmentReport> {
    validate_limits(limits)?;
    if policy.schema_version != 1
        || policy.catalog_sha256 != catalog.sha256()
        || !policy.missing_stat_order.is_finite()
    {
        return Err(AugmentReconstructionError::Invalid(
            "policy/catalog binding",
        ));
    }
    let policy_digest = digest_owned(
        "owned-augment-reconstruction-policy-v1",
        policy,
        limits.max_wire_bytes,
    )?;
    let request_digest = digest_owned(
        "owned-augment-reconstruction-request-v1",
        input,
        limits.max_wire_bytes,
    )?;
    let mut budget = Budget {
        limits,
        left: limits.max_work,
        output: 0,
        members: 0,
    };
    budget.text(&policy.bonded_display_prefix)?;
    if policy
        .bonded_display_prefix
        .bytes()
        .any(|b| b.is_ascii_digit())
    {
        return Err(AugmentReconstructionError::Invalid(
            "numeric Bonded decoration is unreviewed",
        ));
    }
    if policy.bindings.len() > limits.max_bindings || input.selections.len() > limits.max_sockets {
        return Err(AugmentReconstructionError::Limit("bindings or sockets"));
    }
    for fact in [
        &input.magnitude.global_increase_percent,
        &input.magnitude.rune_increase_percent,
        &input.magnitude.soul_core_increase_percent,
    ] {
        if let AugmentFact::Known(v) = fact
            && !v.is_finite()
        {
            return Err(AugmentReconstructionError::Invalid(
                "nonfinite magnitude fact",
            ));
        }
    }
    let mut bindings = BTreeMap::new();
    let mut templates = BTreeSet::new();
    for row in &policy.bindings {
        budget.text(&row.source_name)?;
        if row.source_name.is_empty()
            || bindings
                .insert(row.source_name.as_str(), &row.template)
                .is_some()
            || !templates.insert(&row.template)
        {
            return Err(AugmentReconstructionError::Invalid(
                "duplicate or empty template binding",
            ));
        }
    }
    let mut definitions = BTreeMap::new();
    for row in &catalog.catalog().augments {
        budget.text(&row.source_name)?;
        definitions.insert(row.source_name.as_str(), row);
    }
    let lineage = input.host.item.lineage();
    if input.host.equipment_use.lineage() != lineage
        || input.host.equipment_use.instance_id() == input.host.item.instance_id()
    {
        return Err(AugmentReconstructionError::Invalid(
            "host occurrence identity",
        ));
    }
    let mut instances = BTreeSet::from([
        input.host.item.instance_id(),
        input.host.equipment_use.instance_id(),
    ]);
    let mut slots = BTreeSet::new();
    // The request includes every cloned host/selection/context payload. Charge
    // it before any clone, plus the report's fixed fields and digests.
    budget.retain(input)?;
    budget.output(std::mem::size_of::<PreparedAugmentReport>() + 512)?;
    let mut report = PreparedAugmentReport {
        catalog_sha256: catalog.sha256().into(),
        policy: policy_digest,
        request: request_digest,
        host: input.host.clone(),
        selections: input.selections.clone(),
        lines: vec![],
        issues: vec![],
        activation: input.activation.clone(),
        magnitude: input.magnitude.clone(),
        context_application: AugmentContextApplication::Unapplied,
        semantics: AugmentSemantics::Unconverted,
        work_used: 0,
    };
    let count = match &input.active_socket_count {
        AugmentFact::Known(v) => Some(*v as usize),
        AugmentFact::Unresolved { .. } => {
            push_retained(
                &mut budget,
                &mut report.issues,
                AugmentReconstructionIssue::UnknownSocketCount,
            )?;
            None
        }
    };
    if let Some(count) = count {
        if count > limits.max_sockets {
            return Err(AugmentReconstructionError::Limit("sockets"));
        }
        if count != input.selections.len() {
            push_retained(
                &mut budget,
                &mut report.issues,
                AugmentReconstructionIssue::SocketMembership {
                    supplied: input.selections.len(),
                    active: count,
                },
            )?;
        }
    }
    let categories = match &input.categories {
        AugmentFact::Known(v) => Some(v),
        AugmentFact::Unresolved { .. } => {
            push_retained(
                &mut budget,
                &mut report.issues,
                AugmentReconstructionIssue::UnknownCategories,
            )?;
            None
        }
    };
    if let Some(c) = categories {
        if c.extra_soul_core_selectors.len() > limits.max_categories {
            return Err(AugmentReconstructionError::Limit("categories"));
        }
        let mut extra = BTreeSet::new();
        for s in c
            .broad
            .iter()
            .chain(std::iter::once(&c.specific))
            .chain(&c.extra_soul_core_selectors)
        {
            budget.text(s)?;
            if s.is_empty() {
                return Err(AugmentReconstructionError::Invalid("empty category"));
            }
        }
        for s in &c.extra_soul_core_selectors {
            if !extra.insert(s) {
                return Err(AugmentReconstructionError::Invalid(
                    "duplicate extra category",
                ));
            }
        }
    }
    // Validate all supplied entries, including surplus positions. No earlier
    // accepted entry can mask an unknown trailing source header.
    for (i, selection) in input.selections.iter().enumerate() {
        budget.work(1)?;
        if selection.slot().namespace() != input.host.template.namespace()
            || !slots.insert(selection.slot())
        {
            return Err(AugmentReconstructionError::Invalid("socket identity"));
        }
        match selection {
            AugmentSocketSelection::Unresolved { .. } => push_retained(
                &mut budget,
                &mut report.issues,
                AugmentReconstructionIssue::UnknownSelection { selection_index: i },
            )?,
            AugmentSocketSelection::Empty { .. } => {}
            AugmentSocketSelection::Occupied {
                source_name,
                occurrence,
                ..
            } => {
                budget.text(source_name)?;
                if occurrence.container != input.host.equipment_use
                    || occurrence.item.lineage() != lineage
                    || occurrence.equipment_use.lineage() != lineage
                    || occurrence.template.namespace() != input.host.template.namespace()
                    || !instances.insert(occurrence.item.instance_id())
                    || !instances.insert(occurrence.equipment_use.instance_id())
                {
                    return Err(AugmentReconstructionError::Invalid(
                        "socketed occurrence identity or ancestry",
                    ));
                }
                if !definitions.contains_key(source_name.as_str()) {
                    push_retained(
                        &mut budget,
                        &mut report.issues,
                        AugmentReconstructionIssue::UnknownAugment { selection_index: i },
                    )?;
                }
                if bindings.get(source_name.as_str()).copied() != Some(&occurrence.template) {
                    push_retained(
                        &mut budget,
                        &mut report.issues,
                        AugmentReconstructionIssue::MissingTemplateBinding { selection_index: i },
                    )?;
                }
            }
        }
    }
    if !report.issues.is_empty() {
        return finish(report, &mut budget);
    }
    let categories = categories.expect("unknown category returned above");
    let mut groups = BTreeMap::new();
    for (selection_index, selection) in input.selections.iter().enumerate() {
        let AugmentSocketSelection::Occupied {
            slot,
            source_name,
            occurrence,
        } = selection
        else {
            continue;
        };
        let definition = definitions[source_name.as_str()];
        budget.work(definition.selectors.len())?;
        let selected: BTreeMap<_, _> = definition
            .selectors
            .iter()
            .map(|r| (r.source_selector.as_str(), r))
            .collect();
        let mut rows = Vec::new();
        for s in categories
            .broad
            .iter()
            .chain(std::iter::once(&categories.specific))
        {
            if let Some(row) = selected.get(s.as_str()) {
                rows.push(*row);
            }
        }
        for s in &categories.extra_soul_core_selectors {
            if let Some(row) = selected.get(s.as_str())
                && row.kind == AugmentKind::SoulCore
            {
                rows.push(*row);
            }
        }
        if rows.is_empty() {
            push_retained(
                &mut budget,
                &mut report.issues,
                AugmentReconstructionIssue::NoApplicableRows { selection_index },
            )?;
            continue;
        }
        for row in rows {
            for (lane, lines) in [
                (AugmentLane::Normal, row.normal.as_slice()),
                (AugmentLane::Bonded, row.bonded.as_deref().unwrap_or(&[])),
            ] {
                for (source_line_index, line) in lines.iter().enumerate() {
                    budget.members += 1;
                    if budget.members > limits.max_members {
                        return Err(AugmentReconstructionError::Limit("members"));
                    }
                    budget.text(&line.text)?;
                    let order = line.stat_order.unwrap_or(policy.missing_stat_order);
                    // A finite Lua number uses at most 24 ASCII bytes. Charge
                    // both the group index key and output header before copies.
                    budget.output(2 * 24 + std::mem::size_of::<PreparedAugmentLine>())?;
                    let order_key = lua_number_text(order);
                    let group_key = (row.kind, lane, order_key.clone());
                    budget.retain(&(
                        slot,
                        occurrence,
                        source_name,
                        &row.source_selector,
                        &line.text,
                    ))?;
                    budget.output(std::mem::size_of::<PreparedAugmentMember>())?;
                    let member = PreparedAugmentMember {
                        selection_index,
                        slot: slot.clone(),
                        occurrence: occurrence.clone(),
                        source_name: source_name.clone(),
                        source_selector: row.source_selector.clone(),
                        lane,
                        source_line_index,
                        source_text: line.text.clone(),
                        source_stat_order: line.stat_order,
                    };
                    if let Some(index) = groups.get(&group_key).copied() {
                        let group: &mut PreparedAugmentLine = &mut report.lines[index];
                        match combine(&group.text, &line.text, &mut budget)? {
                            Ok((text, different, ignored)) => {
                                let member_index = group.members.len();
                                if different {
                                    push_retained(
                                        &mut budget,
                                        &mut group.diagnostics,
                                        AugmentMergeDiagnostic::DifferentIncomingSkeleton {
                                            member_index,
                                        },
                                    )?;
                                }
                                if ignored > 0 {
                                    push_retained(
                                        &mut budget,
                                        &mut group.diagnostics,
                                        AugmentMergeDiagnostic::IgnoredIncomingNumbers {
                                            member_index,
                                            count: ignored,
                                        },
                                    )?;
                                }
                                group.text = text;
                                group.members.push(member);
                            }
                            Err(NumericIssue::Missing) => push_retained(
                                &mut budget,
                                &mut report.issues,
                                AugmentReconstructionIssue::MissingNumericMatch {
                                    selection_index,
                                    source_line_index,
                                },
                            )?,
                            Err(NumericIssue::NonFinite) => push_retained(
                                &mut budget,
                                &mut report.issues,
                                AugmentReconstructionIssue::NonFiniteNumber {
                                    selection_index,
                                    source_line_index,
                                },
                            )?,
                        }
                    } else {
                        budget.retain(&line.text)?;
                        groups.insert(group_key, report.lines.len());
                        report.lines.push(PreparedAugmentLine {
                            kind: row.kind,
                            lane,
                            order_key,
                            first_stat_order: order,
                            text: line.text.clone(),
                            display_text: String::new(),
                            members: vec![member],
                            diagnostics: vec![],
                        });
                    }
                }
            }
        }
    }
    if !report.issues.is_empty() {
        report.lines.clear();
    } else {
        // Stable sort retains source encounter order on equal numeric order.
        budget.work(report.lines.len().saturating_mul(16))?;
        report.lines.sort_by(|a, b| {
            a.first_stat_order
                .partial_cmp(&b.first_stat_order)
                .expect("finite source order")
        });
        for line in &mut report.lines {
            let prefix = if line.lane == AugmentLane::Bonded {
                policy.bonded_display_prefix.as_str()
            } else {
                ""
            };
            budget.retain(&(prefix, &line.text))?;
            line.display_text = format!("{prefix}{}", line.text);
        }
    }
    finish(report, &mut budget)
}

fn finish(mut report: PreparedAugmentReport, budget: &mut Budget) -> Result<PreparedAugmentReport> {
    report.work_used = budget.limits.max_work - budget.left;
    let mut measure = OutputMeasure {
        bytes: 0,
        limit: budget.limits.max_output_bytes,
    };
    serde_json::to_writer(&mut measure, &report)
        .map_err(|_| AugmentReconstructionError::Limit("output bytes"))?;
    budget.work(measure.bytes)?;
    report.work_used = budget.limits.max_work - budget.left;
    let mut check = OutputMeasure {
        bytes: 0,
        limit: budget.limits.max_output_bytes,
    };
    serde_json::to_writer(&mut check, &report)
        .map_err(|_| AugmentReconstructionError::Limit("output bytes"))?;
    Ok(report)
}
struct OutputMeasure {
    bytes: usize,
    limit: usize,
}
impl std::io::Write for OutputMeasure {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.bytes = self
            .bytes
            .checked_add(bytes.len())
            .filter(|n| *n <= self.limit)
            .ok_or_else(|| std::io::Error::other("output byte limit"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
#[derive(Clone, Copy)]
enum NumericIssue {
    Missing,
    NonFinite,
}
/// Exact unsigned decimal token shape used by this reviewed source dialect.
/// Signs, words, subsequent dots and exponent markers remain first-line text.
fn spans(text: &str, budget: &mut Budget) -> Result<Vec<std::ops::Range<usize>>> {
    budget.text(text)?;
    let b = text.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    while i < b.len() {
        if !b[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        // Lua (%d%.?%d*) consumes ONE initial digit, then an optional dot,
        // then remaining digits. Thus 12.5 has two tokens, 12 and 5.
        i += 1;
        if i < b.len() && b[i] == b'.' {
            i += 1;
        }
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        out.push(start..i);
    }
    Ok(out)
}
fn skeleton(text: &str, spans: &[std::ops::Range<usize>]) -> String {
    let mut out = String::new();
    let mut at = 0;
    for span in spans {
        out.push_str(&text[at..span.start]);
        out.push('#');
        at = span.end;
    }
    out.push_str(&text[at..]);
    out
}
fn combine(
    stored: &str,
    incoming: &str,
    budget: &mut Budget,
) -> Result<std::result::Result<(String, bool, usize), NumericIssue>> {
    let a = spans(stored, budget)?;
    let b = spans(incoming, budget)?;
    if a.len() > b.len() {
        return Ok(Err(NumericIssue::Missing));
    }
    let different = skeleton(stored, &a) != skeleton(incoming, &b);
    let mut out = String::new();
    let mut at = 0;
    for (left, right) in a.iter().zip(&b) {
        let x = stored[left.clone()].parse::<f64>();
        let y = incoming[right.clone()].parse::<f64>();
        let (Ok(x), Ok(y)) = (x, y) else {
            return Ok(Err(NumericIssue::NonFinite));
        };
        let sum = x + y;
        if !sum.is_finite() {
            return Ok(Err(NumericIssue::NonFinite));
        }
        let number = lua_number_text(sum);
        budget.retain(&(&stored[at..left.start], &number))?;
        out.push_str(&stored[at..left.start]);
        out.push_str(&number);
        at = left.end;
    }
    budget.retain(&&stored[at..])?;
    out.push_str(&stored[at..]);
    budget.text(&out)?;
    Ok(Ok((out, different, b.len() - a.len())))
}
