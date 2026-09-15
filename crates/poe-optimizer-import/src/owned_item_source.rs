//! Source-format attribution only. Owned definitions and numerical recipes remain
//! in the injected item-line policy; no PoB checkout, VM or UI is loaded here.
use crate::{
    build_instance::{AuthoredInstanceId, SourceOccurrenceId},
    owned_item_lines::*,
    owned_mapping::{ExternalSourceSystem, SourcePin},
    owned_source::{
        SourceAttributeRef, SourceContentEvidence, SourceEvidenceError, SourceEvidenceRow,
        SourceProjectEvidence,
    },
    source_xml::PobContentEntry,
};
use poe_optimizer_core::{
    build_identity::{BuildLineage, BuildRevision, InstanceAllocatorState},
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::{GameVersionNamespace, ItemTemplateDefId, OwnedDefinitionKey},
    owned_schema::{DefinitionSchemaIndex, SchemaLookup},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
};

mod attribute;
pub const OWNED_ITEM_SOURCE_POLICY_VERSION: u32 = 1;
const DOMAIN: &str = "owned-item-source-policy-v1";
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemSourceLayoutPolicyInput {
    pub schema_version: u32,
    pub namespace: GameVersionNamespace,
    pub version: OwnedDefinitionKey,
    /// Offline provenance, not authentication of the exporter or runtime files.
    pub source: SourcePin,
    pub item_lines: OwnedContentDigest,
    pub dialect: ItemSourceDialect,
    pub rule_layouts: Vec<ItemRuleSourceLayout>,
    pub template_layouts: Vec<ItemTemplateSourceLayout>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemSourceDialect {
    PobExportedSingleTextV1,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemRuleSourceLayout {
    pub rule: OwnedDefinitionKey,
    pub role: ItemRuleSourceRole,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemRuleSourceRole {
    /// Reviewed zero-member preamble syntax; not valid after modifier insertion starts.
    Header,
    /// A reviewed one-member source grammar; malformed captures do not satisfy it.
    SingleModifier,
    Unresolved,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemTemplateSourceLayout {
    pub template: ItemTemplateDefId,
    pub load_index_prefix: ItemLoadIndexPrefix,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemLoadIndexPrefix {
    NoGeneratedBuffMembers,
    Unresolved,
}
#[derive(Clone, Copy, Debug)]
pub struct ItemSourceLimits {
    pub max_wire_bytes: usize,
    pub max_policy_text_bytes: usize,
    pub max_source_files: usize,
    pub max_rules: usize,
    pub max_templates: usize,
    pub max_source_bytes: usize,
    pub max_line_bytes: usize,
    pub max_lines: usize,
    pub max_tags: usize,
    pub max_overlays: usize,
    pub max_output_records: usize,
    pub max_work: usize,
}
impl Default for ItemSourceLimits {
    fn default() -> Self {
        Self {
            max_wire_bytes: 4 * 1024 * 1024,
            max_policy_text_bytes: 2 * 1024 * 1024,
            max_source_files: 64,
            max_rules: 2048,
            max_templates: 8192,
            max_source_bytes: 1024 * 1024,
            max_line_bytes: 65536,
            max_lines: 8192,
            max_tags: 32768,
            max_overlays: 16384,
            max_output_records: 65536,
            max_work: 64 * 1024 * 1024,
        }
    }
}
impl ItemSourceLimits {
    fn values(self) -> [usize; 12] {
        [
            self.max_wire_bytes,
            self.max_policy_text_bytes,
            self.max_source_files,
            self.max_rules,
            self.max_templates,
            self.max_source_bytes,
            self.max_line_bytes,
            self.max_lines,
            self.max_tags,
            self.max_overlays,
            self.max_output_records,
            self.max_work,
        ]
    }
    fn validate(self) -> Result<()> {
        if self
            .values()
            .into_iter()
            .zip(Self::default().values())
            .any(|(v, max)| v == 0 || v > max)
        {
            return Err(ItemSourceError::InvalidLimit);
        }
        Ok(())
    }
}
#[derive(Debug, thiserror::Error)]
pub enum ItemSourceError {
    #[error("unsupported item source policy version {0}")]
    UnsupportedVersion(u32),
    #[error("invalid item source resource limit")]
    InvalidLimit,
    #[error("item source exceeds {0}")]
    Limit(&'static str),
    #[error("invalid item source policy: {0}")]
    Policy(&'static str),
    #[error("item source policy binding differs")]
    Binding,
    #[error("source is not a supported PoB2 item occurrence")]
    SourceKind,
    #[error(transparent)]
    Evidence(#[from] SourceEvidenceError),
    #[error(transparent)]
    Lines(#[from] ItemLineError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
}
type Result<T> = std::result::Result<T, ItemSourceError>;
fn charge(left: &mut usize, n: usize, what: &'static str) -> Result<()> {
    *left = left.checked_sub(n).ok_or(ItemSourceError::Limit(what))?;
    Ok(())
}
#[derive(Clone, Debug)]
pub struct ItemSourceLayoutPolicy {
    input: ItemSourceLayoutPolicyInput,
    identity: OwnedContentDigest,
    limits: ItemSourceLimits,
    roles: BTreeMap<OwnedDefinitionKey, ItemRuleSourceRole>,
    prefixes: BTreeMap<ItemTemplateDefId, ItemLoadIndexPrefix>,
}
impl ItemSourceLayoutPolicy {
    pub fn new<I: DefinitionSchemaIndex>(
        input: ItemSourceLayoutPolicyInput,
        lines: &OwnedItemLinePolicy,
        schema: &I,
        limits: ItemSourceLimits,
    ) -> Result<Self> {
        limits.validate()?;
        if input.schema_version != OWNED_ITEM_SOURCE_POLICY_VERSION {
            return Err(ItemSourceError::UnsupportedVersion(input.schema_version));
        }
        lines.verify_bindings(schema)?;
        if input.namespace != *schema.namespace() || input.item_lines != *lines.identity() {
            return Err(ItemSourceError::Binding);
        }
        if input.source.system != ExternalSourceSystem::PathOfBuilding2
            || input.source.revision.trim().is_empty()
            || input.source.revision.chars().any(char::is_control)
            || input.source.files.is_empty()
        {
            return Err(ItemSourceError::Policy("source provenance"));
        }
        if input.source.files.len() > limits.max_source_files
            || input.rule_layouts.len() > limits.max_rules
            || input.template_layouts.len() > limits.max_templates
        {
            return Err(ItemSourceError::Limit("policy entries"));
        }
        let mut text_left = limits.max_policy_text_bytes;
        charge(&mut text_left, input.source.revision.len(), "policy text")?;
        let mut paths = BTreeSet::new();
        for f in &input.source.files {
            charge(
                &mut text_left,
                f.path.len().saturating_add(f.sha256.len()),
                "policy text",
            )?;
            if f.path.is_empty()
                || f.path.contains(['\\', ':'])
                || f.path.chars().any(char::is_control)
                || f.path
                    .split('/')
                    .any(|s| s.is_empty() || matches!(s, "." | ".."))
                || !paths.insert(&f.path)
                || f.sha256.len() != 64
                || !f
                    .sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err(ItemSourceError::Policy("source file provenance"));
            }
        }
        let known: BTreeSet<_> = lines.input().rules.iter().map(|r| &r.id).collect();
        let mut roles = BTreeMap::new();
        for r in &input.rule_layouts {
            charge(&mut text_left, r.rule.as_str().len(), "policy text")?;
            if !known.contains(&r.rule) || roles.insert(r.rule.clone(), r.role).is_some() {
                return Err(ItemSourceError::Policy("unknown or duplicate rule"));
            }
        }
        let mut prefixes = BTreeMap::new();
        for t in &input.template_layouts {
            charge(
                &mut text_left,
                t.template.key().as_str().len(),
                "policy text",
            )?;
            if !matches!(schema.definition(&t.template), SchemaLookup::Known(_))
                || prefixes
                    .insert(t.template.clone(), t.load_index_prefix)
                    .is_some()
            {
                return Err(ItemSourceError::Policy("unknown or duplicate template"));
            }
        }
        let identity = digest_owned(DOMAIN, &input, limits.max_wire_bytes)?;
        Ok(Self {
            input,
            identity,
            limits,
            roles,
            prefixes,
        })
    }
    pub fn input(&self) -> &ItemSourceLayoutPolicyInput {
        &self.input
    }
    pub fn identity(&self) -> &OwnedContentDigest {
        &self.identity
    }
    pub fn verify_bindings<I: DefinitionSchemaIndex>(
        &self,
        lines: &OwnedItemLinePolicy,
        schema: &I,
    ) -> Result<()> {
        lines.verify_bindings(schema)?;
        if self.input.item_lines != *lines.identity() || self.input.namespace != *schema.namespace()
        {
            return Err(ItemSourceError::Binding);
        }
        Ok(())
    }
}
pub fn decode_item_source_policy<I: DefinitionSchemaIndex>(
    bytes: &[u8],
    lines: &OwnedItemLinePolicy,
    schema: &I,
    limits: ItemSourceLimits,
) -> Result<ItemSourceLayoutPolicy> {
    limits.validate()?;
    if bytes.len() > limits.max_wire_bytes {
        return Err(ItemSourceError::Limit("wire bytes"));
    }
    ItemSourceLayoutPolicy::new(serde_json::from_slice(bytes)?, lines, schema, limits)
}
pub fn encode_item_source_policy(
    policy: &ItemSourceLayoutPolicy,
    limits: ItemSourceLimits,
) -> Result<Vec<u8>> {
    limits.validate()?;
    if policy.input.rule_layouts.len() > limits.max_rules
        || policy.input.template_layouts.len() > limits.max_templates
        || policy.input.source.files.len() > limits.max_source_files
    {
        return Err(ItemSourceError::Limit("policy entries"));
    }
    let mut text_left = limits.max_policy_text_bytes;
    charge(
        &mut text_left,
        policy.input.source.revision.len(),
        "policy text",
    )?;
    for file in &policy.input.source.files {
        charge(
            &mut text_left,
            file.path.len().saturating_add(file.sha256.len()),
            "policy text",
        )?;
    }
    for rule in &policy.input.rule_layouts {
        charge(&mut text_left, rule.rule.as_str().len(), "policy text")?;
    }
    for template in &policy.input.template_layouts {
        charge(
            &mut text_left,
            template.template.key().as_str().len(),
            "policy text",
        )?;
    }
    digest_owned(DOMAIN, &policy.input, limits.max_wire_bytes)?;
    Ok(serde_json::to_vec(&policy.input)?)
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ItemSourceBinding {
    pub source_sha256: String,
    pub source_bytes: usize,
    pub source_schema: u32,
    pub lineage: BuildLineage,
    pub revision: BuildRevision,
    pub allocator: InstanceAllocatorState,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceModifierCategory {
    Buff,
    Enchant,
    Implicit,
    Explicit,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct SourceModifierSlot {
    pub category: SourceModifierCategory,
    pub ordinal: usize,
    pub line: usize,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemSourceProblem {
    ContentUnavailable,
    UnsupportedContentOrder,
    UnsupportedSourceControl,
    UnsupportedChild,
    UnsupportedAttribute,
    MissingRarity,
    UnsupportedRarity,
    UnknownHeader,
    HeaderAfterModifiers,
    MissingImplicitCount,
    InvalidImplicitCount,
    UnknownTemplatePrefix,
    UnknownMember,
    MalformedCapture,
    PossibleCombinedLine,
    UnsupportedTag,
    MalformedTag,
    RuneLifecycle,
    UnprovedTaggedLine,
    InvalidRange,
    InvalidOverlay,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "status", content = "problems", rename_all = "snake_case")]
pub enum ItemLayoutStatus {
    Proven,
    Pending(Vec<ItemSourceProblem>),
    Unsupported(Vec<ItemSourceProblem>),
}
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ItemRangeOrigin {
    Inline {
        line: usize,
        span: Range<usize>,
    },
    Xml {
        occurrence: SourceOccurrenceId,
        content_entry: usize,
        id: Option<SourceAttributeRef>,
        range: Option<SourceAttributeRef>,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "status", content = "value", rename_all = "snake_case")]
pub enum ItemRangeTarget {
    Line(usize),
    Pending,
    IgnoredOutOfBounds,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ItemRangeWrite {
    pub origin: ItemRangeOrigin,
    pub source_id: Option<usize>,
    pub fraction: Option<f64>,
    pub target: ItemRangeTarget,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ItemRangeDecision {
    Absent,
    Resolved { fraction: f64, winning_write: usize },
    Pending,
}
#[derive(Clone, Debug, Serialize)]
pub struct ItemAttributedLine {
    pub index: usize,
    pub decoded_span: Range<usize>,
    pub raw: String,
    pub semantic_text: String,
    pub rule: Option<OwnedDefinitionKey>,
    pub presentation: bool,
    pub pending_candidates: Vec<OwnedDefinitionKey>,
    pub member: Option<SourceModifierSlot>,
    pub blockers: Vec<ItemSourceProblem>,
    pub range: ItemRangeDecision,
}
#[derive(Clone, Debug, Serialize)]
pub struct ItemAttributionReport {
    pub source: ItemSourceBinding,
    pub item: SourceOccurrenceId,
    pub content_entry: Option<usize>,
    pub policy: OwnedContentDigest,
    pub item_lines: OwnedContentDigest,
    pub layout: ItemLayoutStatus,
    pub lines: Vec<ItemAttributedLine>,
    pub writes: Vec<ItemRangeWrite>,
}
#[derive(Debug)]
pub struct ItemRangeAttribution {
    report: ItemAttributionReport,
    work_left: usize,
    output_left: usize,
    can_convert: bool,
}
impl ItemRangeAttribution {
    pub fn report(&self) -> &ItemAttributionReport {
        &self.report
    }
    pub fn into_report(self) -> ItemAttributionReport {
        self.report
    }
    pub fn can_convert_lines(&self) -> bool {
        self.can_convert
    }
    pub fn convert<'a>(&'a self, lines: &OwnedItemLinePolicy) -> Result<ItemTextConversion<'a>> {
        if *lines.identity() != self.report.item_lines {
            return Err(ItemSourceError::Binding);
        }
        let mut work = self.work_left;
        let mut output = self.output_left;
        Ok(lines.convert_source_lines(
            self.report.lines.iter().map(|l| {
                let pending = if l.presentation {
                    Some(SourceLinePending {
                        reason: ItemLinePending::SourcePresentation,
                        candidates: &[],
                    })
                } else if !self.can_convert
                    || l.blockers
                        .iter()
                        .any(|p| !matches!(p, ItemSourceProblem::InvalidRange))
                {
                    Some(SourceLinePending {
                        reason: ItemLinePending::SourceMeaningUnresolved,
                        candidates: &l.pending_candidates,
                    })
                } else {
                    None
                };
                (
                    ItemLineInput {
                        index: l.index,
                        text: &l.semantic_text,
                        range_fraction: match l.range {
                            ItemRangeDecision::Resolved { fraction, .. } => Some(fraction),
                            _ => None,
                        },
                    },
                    pending,
                )
            }),
            &mut work,
            &mut output,
        )?)
    }
}
