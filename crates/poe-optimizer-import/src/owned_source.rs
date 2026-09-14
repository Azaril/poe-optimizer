//! Borrowed, source-only evidence for owned normalization. No saved set is selected.
//!
//! Every imported element has one row and one coverage entry. Lexical failures
//! retain the original source and do not erase unrelated fields or projections.
use crate::{
    build_instance::{
        AttributeOrigin, AuthoredInstanceId, INSTANCE_IMPORT_SCHEMA, ImportedBuildInstance,
        InstanceImportError, ProjectionState, SourceOccurrence, SourceOccurrenceId, SourceRole,
    },
    item_source::{ItemSourceKind, ItemSourceUse},
    skill_source::{SkillSourceKind, SkillSourceUse},
    source_xml::{
        self, PobContentEntry, SourceContent, SourceContentKind, SourceText, SourceXmlError,
    },
};
use poe_optimizer_core::build_identity::{BuildLineage, BuildRevision, InstanceAllocatorState};
use serde::Serialize;
use std::{collections::BTreeMap, ops::Range};
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceEvidenceLimits {
    pub max_occurrences: usize,
    pub max_attributes: usize,
    pub max_fragments: usize,
    pub max_depth: usize,
    pub max_value_bytes: usize,
    /// Conservative accounting: original XML bytes once, plus decoded strings.
    /// Borrowed nested element spans never charge or copy their subtrees again.
    pub max_total_text_bytes: usize,
    pub max_index_entries: usize,
}
impl Default for SourceEvidenceLimits {
    fn default() -> Self {
        Self {
            max_occurrences: 100_000,
            max_attributes: 200_000,
            max_fragments: 300_000,
            max_depth: 128,
            max_value_bytes: crate::MAX_XML_BYTES,
            max_total_text_bytes: 2 * crate::MAX_XML_BYTES,
            max_index_entries: 800_000,
        }
    }
}
impl SourceEvidenceLimits {
    fn validate(self) -> Result<(), SourceEvidenceError> {
        let hard = Self::default();
        for (name, value, maximum) in [
            ("occurrences", self.max_occurrences, hard.max_occurrences),
            ("attributes", self.max_attributes, hard.max_attributes),
            ("fragments", self.max_fragments, hard.max_fragments),
            ("depth", self.max_depth, hard.max_depth),
            ("value bytes", self.max_value_bytes, hard.max_value_bytes),
            (
                "total text bytes",
                self.max_total_text_bytes,
                hard.max_total_text_bytes,
            ),
            (
                "index entries",
                self.max_index_entries,
                hard.max_index_entries,
            ),
        ] {
            if value == 0 || value > maximum {
                return Err(SourceEvidenceError::InvalidLimit(name));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum SourceEvidenceError {
    #[error(transparent)]
    Source(#[from] InstanceImportError),
    #[error("invalid source evidence {0} limit")]
    InvalidLimit(&'static str),
    #[error("source evidence exceeds {0} limit")]
    ResourceLimit(&'static str),
    #[error("source evidence index disagrees with immutable source: {0}")]
    InconsistentSource(&'static str),
    #[error("attribute index does not belong to the source occurrence")]
    UnknownAttribute,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct SourceEvidenceIdentity<'s> {
    pub instance_import_schema: u32,
    pub source_sha256: &'s str,
    pub source_bytes: usize,
    pub lineage: BuildLineage,
    pub revision: BuildRevision,
    pub allocator: InstanceAllocatorState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceSectionKind {
    Root,
    Build,
    Tree,
    Items,
    Skills,
    Config,
    Other,
}

#[derive(Debug, Serialize)]
pub struct SourceAttributeEvidence<'s> {
    origin: &'s AttributeOrigin,
    value: Result<SourceText<'s>, SourceXmlError>,
    #[serde(skip)]
    raw: &'s str,
}
impl<'s> SourceAttributeEvidence<'s> {
    pub fn origin(&self) -> &'s AttributeOrigin {
        self.origin
    }
    pub fn raw(&self) -> &'s str {
        self.raw
    }
    pub fn range(&self) -> Range<usize> {
        self.origin.value_range.clone()
    }
    pub fn value(&self) -> Result<&SourceText<'s>, &SourceXmlError> {
        self.value.as_ref()
    }
    pub fn decoded(&self) -> Result<&str, &SourceXmlError> {
        self.value.as_ref().map(SourceText::decoded)
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", content = "value", rename_all = "snake_case")]
pub enum SourceContentEvidence<'s> {
    Available(SourceContent<'s>),
    Unavailable(SourceXmlError),
}

#[derive(Debug, Serialize)]
pub struct SourceEvidenceRow<'s> {
    occurrence: &'s SourceOccurrence,
    authored_instance: Option<AuthoredInstanceId>,
    section: SourceSectionKind,
    attributes: Vec<SourceAttributeEvidence<'s>>,
    content: SourceContentEvidence<'s>,
    children: Vec<SourceOccurrenceId>,
}
impl<'s> SourceEvidenceRow<'s> {
    pub fn occurrence(&self) -> &'s SourceOccurrence {
        self.occurrence
    }
    pub fn authored_instance(&self) -> Option<AuthoredInstanceId> {
        self.authored_instance
    }
    pub fn section(&self) -> SourceSectionKind {
        self.section
    }
    pub fn attributes(&self) -> &[SourceAttributeEvidence<'s>] {
        &self.attributes
    }
    /// Plain names address only unnamespaced attributes. Missing is not empty.
    pub fn attribute(&self, name: &str) -> Option<&SourceAttributeEvidence<'s>> {
        self.attributes
            .iter()
            .find(|attribute| attribute.origin.namespace.is_none() && attribute.origin.name == name)
    }
    pub fn content(&self) -> &SourceContentEvidence<'s> {
        &self.content
    }
    pub fn children(&self) -> &[SourceOccurrenceId] {
        &self.children
    }
}

/// Recognition describes shallow source grammar, not semantic or rule coverage.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceRecognition {
    KnownSourceShape,
    UnknownElementOrContext,
    NamespaceContext,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "index", rename_all = "snake_case")]
pub enum SourceLexicalField {
    Attribute(u32),
    Content,
}
/// The full error remains on the identified row field; this avoids copying it.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct SourceLexicalIssue {
    pub field: SourceLexicalField,
    pub byte_offset: usize,
}
#[derive(Debug, Serialize)]
pub struct SourceCoverageEntry {
    pub source: SourceOccurrenceId,
    pub section: SourceSectionKind,
    pub recognition: SourceRecognition,
    pub lexical_issues: Vec<SourceLexicalIssue>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
pub struct SourceAttributeRef {
    pub occurrence: SourceOccurrenceId,
    pub index: u32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceQName<'q> {
    pub namespace: Option<&'q str>,
    pub local: &'q str,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceKeyQuery<'q> {
    /// None addresses the document root; Some addresses its direct children.
    pub parent: Option<SourceOccurrenceId>,
    pub element: SourceQName<'q>,
    pub attribute: SourceQName<'q>,
    pub value: &'q str,
}
#[derive(Debug, PartialEq, Eq)]
pub enum SourceKeyLookup<'e> {
    Missing,
    Unique(SourceAttributeRef),
    Ambiguous(&'e [SourceAttributeRef]),
    /// An undecodable value in this exact scope can neither match nor be excluded.
    Unresolved {
        matches: &'e [SourceAttributeRef],
        unavailable: &'e [SourceAttributeRef],
    },
}

#[derive(Debug)]
pub struct SourceProjectEvidence<'s> {
    source: &'s ImportedBuildInstance,
    rows: Vec<SourceEvidenceRow<'s>>,
    ranges: BTreeMap<(usize, usize), usize>,
    sections: BTreeMap<SourceSectionKind, Vec<SourceOccurrenceId>>,
    keys: Vec<SourceAttributeRef>,
    unavailable_keys: Vec<SourceAttributeRef>,
    coverage: Vec<SourceCoverageEntry>,
}

fn charge(left: &mut usize, amount: usize, name: &'static str) -> Result<(), SourceEvidenceError> {
    *left = left
        .checked_sub(amount)
        .ok_or(SourceEvidenceError::ResourceLimit(name))?;
    Ok(())
}
fn section(occurrence: &SourceOccurrence) -> SourceSectionKind {
    if occurrence.has_namespace_context() {
        return SourceSectionKind::Other;
    }
    match occurrence.name() {
        "Build" => SourceSectionKind::Build,
        "Tree" | "Spec" => SourceSectionKind::Tree,
        "Items" => SourceSectionKind::Items,
        "Skills" => SourceSectionKind::Skills,
        "Config" => SourceSectionKind::Config,
        _ => SourceSectionKind::Other,
    }
}
fn recognition(occurrence: &SourceOccurrence, direct_root_child: bool) -> SourceRecognition {
    if occurrence.has_namespace_context() {
        return SourceRecognition::NamespaceContext;
    }
    let known = match occurrence.role() {
        SourceRole::Root | SourceRole::ConfigContainer | SourceRole::ConfigSet => true,
        SourceRole::ConfigRecord => matches!(
            occurrence.name(),
            "Input" | "Placeholder" | "CustomModifierBlock"
        ),
        SourceRole::Skill { kind, usage } => {
            kind != SkillSourceKind::Unknown
                && !matches!(
                    usage,
                    SkillSourceUse::Ignored | SkillSourceUse::NamespaceUnknown
                )
        }
        SourceRole::Item { kind, usage } => {
            kind != ItemSourceKind::Unknown
                && !matches!(
                    usage,
                    ItemSourceUse::Ignored | ItemSourceUse::NamespaceUnknown
                )
        }
        SourceRole::Opaque => direct_root_child && occurrence.name() == "Build",
    };
    if known {
        SourceRecognition::KnownSourceShape
    } else {
        SourceRecognition::UnknownElementOrContext
    }
}
/// Tight adapter for the existing helper's two resource errors. Lexical errors
/// are data; exhausting an allocation budget must abort collection instead.
fn content_limit(error: &SourceXmlError) -> Option<&'static str> {
    match error.reason.as_str() {
        "item source fragment limit exceeded" => Some("fragments"),
        "item source decoded text limit exceeded" => Some("total text bytes"),
        _ => None,
    }
}
type KeyPrefix<'a> = (
    Option<u32>,
    Option<&'a str>,
    &'a str,
    Option<&'a str>,
    &'a str,
);
fn key_prefix<'a>(
    rows: &'a [SourceEvidenceRow<'_>],
    reference: &SourceAttributeRef,
) -> KeyPrefix<'a> {
    let row = &rows[reference.occurrence.ordinal() as usize];
    let attribute = &row.attributes[reference.index as usize];
    (
        row.occurrence.parent().map(SourceOccurrenceId::ordinal),
        row.occurrence.namespace(),
        row.occurrence.name(),
        attribute.origin.namespace.as_deref(),
        &attribute.origin.name,
    )
}
fn key_value<'a>(rows: &'a [SourceEvidenceRow<'_>], reference: &SourceAttributeRef) -> &'a str {
    rows[reference.occurrence.ordinal() as usize].attributes[reference.index as usize]
        .decoded()
        .expect("only decoded attributes enter the exact value index")
}

impl<'s> SourceProjectEvidence<'s> {
    pub fn collect(
        source: &'s ImportedBuildInstance,
        limits: SourceEvidenceLimits,
    ) -> Result<Self, SourceEvidenceError> {
        limits.validate()?;
        let occurrences = source.occurrences();
        if occurrences.len() > limits.max_occurrences {
            return Err(SourceEvidenceError::ResourceLimit("occurrences"));
        }
        let mut text_left = limits.max_total_text_bytes;
        charge(
            &mut text_left,
            source.source_xml().len(),
            "total text bytes",
        )?;
        let mut attributes_left = limits.max_attributes;
        let mut fragments_left = limits.max_fragments;
        let mut index_left = limits.max_index_entries;
        charge(&mut index_left, occurrences.len(), "index entries")?;
        let mut authored = vec![None; occurrences.len()];
        for binding in source.instances() {
            source.occurrence(binding.source())?;
            let slot = &mut authored[binding.source().ordinal() as usize];
            if slot.replace(binding.instance()).is_some() {
                return Err(SourceEvidenceError::InconsistentSource(
                    "duplicate occurrence binding",
                ));
            }
        }
        let document =
            crate::parse_document(source.source_xml()).map_err(InstanceImportError::from)?;
        let mut rows: Vec<SourceEvidenceRow<'s>> = Vec::with_capacity(occurrences.len());
        let mut depths = Vec::with_capacity(occurrences.len());
        let mut ranges = BTreeMap::new();
        let mut sections: BTreeMap<SourceSectionKind, Vec<SourceOccurrenceId>> = BTreeMap::new();
        let mut keys = Vec::new();
        let mut unavailable_keys = Vec::new();
        let mut coverage = Vec::with_capacity(occurrences.len());
        for (ordinal, node) in document
            .descendants()
            .filter(roxmltree::Node::is_element)
            .enumerate()
        {
            let occurrence = occurrences
                .get(ordinal)
                .ok_or(SourceEvidenceError::InconsistentSource("element count"))?;
            if occurrence.range() != node.range() || occurrence.name() != node.tag_name().name() {
                return Err(SourceEvidenceError::InconsistentSource("element range"));
            }
            let parent = occurrence.parent().map(|id| id.ordinal() as usize);
            let depth = if let Some(parent) = parent {
                if parent >= rows.len() {
                    return Err(SourceEvidenceError::InconsistentSource("parent order"));
                }
                depths[parent] + 1
            } else {
                0
            };
            if depth > limits.max_depth {
                return Err(SourceEvidenceError::ResourceLimit("depth"));
            }
            depths.push(depth);
            let direct_root_child = parent == Some(0);
            let section = match parent {
                None => SourceSectionKind::Root,
                Some(0) => section(occurrence),
                Some(parent) => rows[parent].section,
            };
            if parent.is_none() || direct_root_child {
                charge(&mut index_left, 1, "index entries")?;
                sections.entry(section).or_default().push(occurrence.id());
            }
            if let Some(parent) = parent {
                charge(&mut index_left, 1, "index entries")?;
                rows[parent].children.push(occurrence.id());
            }
            charge(&mut index_left, 1, "index entries")?;
            let range = occurrence.range();
            if ranges.insert((range.start, range.end), ordinal).is_some() {
                return Err(SourceEvidenceError::InconsistentSource(
                    "duplicate element range",
                ));
            }
            charge(
                &mut attributes_left,
                occurrence.attributes().len(),
                "attributes",
            )?;
            let mut attributes = Vec::with_capacity(occurrence.attributes().len());
            let mut lexical_issues = Vec::new();
            for (index, origin) in occurrence.attributes().iter().enumerate() {
                let raw = &source.source_xml()[origin.value_range.clone()];
                if raw.len() > limits.max_value_bytes {
                    return Err(SourceEvidenceError::ResourceLimit("value bytes"));
                }
                // Named-entity decoding cannot allocate more bytes than its raw input.
                // SourceText reserves raw.len() even when entities decode shorter.
                // Keep that reservation charged; successful decoding is no refund.
                charge(&mut text_left, raw.len(), "total text bytes")?;
                let value =
                    SourceText::from_range(source.source_xml(), origin.value_range.clone(), false);
                let reference = SourceAttributeRef {
                    occurrence: occurrence.id(),
                    index: index as u32,
                };
                charge(&mut index_left, 1, "index entries")?;
                match &value {
                    Ok(_) => keys.push(reference),
                    Err(error) => {
                        lexical_issues.push(SourceLexicalIssue {
                            field: SourceLexicalField::Attribute(index as u32),
                            byte_offset: error.byte_offset,
                        });
                        unavailable_keys.push(reference);
                    }
                }
                attributes.push(SourceAttributeEvidence { origin, value, raw });
            }
            // Check direct non-element spans before the lexical helper allocates.
            // Element subtrees are borrowed references and never counted here.
            for child in node.children().filter(|child| !child.is_element()) {
                if child.range().len() > limits.max_value_bytes {
                    return Err(SourceEvidenceError::ResourceLimit("value bytes"));
                }
            }
            let content = match source_xml::ordered_content(
                node,
                &mut fragments_left,
                &mut text_left,
            ) {
                Ok(content) => {
                    if content.fragments().iter().any(|fragment| fragment.kind() != SourceContentKind::Element && fragment.raw().len() > limits.max_value_bytes)
                        || content.consumed().iter().any(|entry| matches!(entry, PobContentEntry::Text { text, .. } if text.len() > limits.max_value_bytes)) {
                        return Err(SourceEvidenceError::ResourceLimit("value bytes"));
                    }
                    SourceContentEvidence::Available(content)
                }
                Err(error) => {
                    if let Some(limit) = content_limit(&error) {
                        return Err(SourceEvidenceError::ResourceLimit(limit));
                    }
                    lexical_issues.push(SourceLexicalIssue {
                        field: SourceLexicalField::Content,
                        byte_offset: error.byte_offset,
                    });
                    SourceContentEvidence::Unavailable(error)
                }
            };
            coverage.push(SourceCoverageEntry {
                source: occurrence.id(),
                section,
                recognition: recognition(occurrence, direct_root_child),
                lexical_issues,
            });
            rows.push(SourceEvidenceRow {
                occurrence,
                authored_instance: authored[ordinal],
                section,
                attributes,
                content,
                children: Vec::new(),
            });
        }
        if rows.len() != occurrences.len() {
            return Err(SourceEvidenceError::InconsistentSource("element count"));
        }
        // Stable sorting retains source order among equal keys and unavailable values.
        keys.sort_by(|a, b| {
            key_prefix(&rows, a)
                .cmp(&key_prefix(&rows, b))
                .then_with(|| key_value(&rows, a).cmp(key_value(&rows, b)))
        });
        unavailable_keys.sort_by(|a, b| key_prefix(&rows, a).cmp(&key_prefix(&rows, b)));
        Ok(Self {
            source,
            rows,
            ranges,
            sections,
            keys,
            unavailable_keys,
            coverage,
        })
    }
    pub fn identity(&self) -> SourceEvidenceIdentity<'s> {
        SourceEvidenceIdentity {
            instance_import_schema: INSTANCE_IMPORT_SCHEMA,
            source_sha256: self.source.source_sha256(),
            source_bytes: self.source.source_xml().len(),
            lineage: self.source.lineage(),
            revision: self.source.revision(),
            allocator: *self.source.allocator_state(),
        }
    }
    pub fn rows(&self) -> &[SourceEvidenceRow<'s>] {
        &self.rows
    }
    pub fn row(
        &self,
        id: SourceOccurrenceId,
    ) -> Result<&SourceEvidenceRow<'s>, SourceEvidenceError> {
        self.source.occurrence(id)?;
        Ok(&self.rows[id.ordinal() as usize])
    }
    pub fn row_for_instance(
        &self,
        id: AuthoredInstanceId,
    ) -> Result<&SourceEvidenceRow<'s>, SourceEvidenceError> {
        self.row(self.source.binding(id)?.source())
    }
    pub fn row_at_range(&self, range: Range<usize>) -> Option<&SourceEvidenceRow<'s>> {
        self.ranges
            .get(&(range.start, range.end))
            .map(|&index| &self.rows[index])
    }
    pub fn children(
        &self,
        id: SourceOccurrenceId,
    ) -> Result<&[SourceOccurrenceId], SourceEvidenceError> {
        Ok(self.row(id)?.children())
    }
    /// Top-level source sections only, with independent order and no synthetic sets.
    pub fn sections(&self, kind: SourceSectionKind) -> &[SourceOccurrenceId] {
        self.sections.get(&kind).map_or(&[], Vec::as_slice)
    }
    pub fn projections(&self) -> &'s [ProjectionState] {
        self.source.projections()
    }
    pub fn coverage(&self) -> &[SourceCoverageEntry] {
        &self.coverage
    }
    pub fn source_xml(&self) -> &'s str {
        self.source.source_xml()
    }
    pub fn source_fragment(&self, id: SourceOccurrenceId) -> Result<&'s str, SourceEvidenceError> {
        Ok(self.source.source_fragment(id)?)
    }
    pub fn attribute(
        &self,
        reference: SourceAttributeRef,
    ) -> Result<&SourceAttributeEvidence<'s>, SourceEvidenceError> {
        self.row(reference.occurrence)?
            .attributes
            .get(reference.index as usize)
            .ok_or(SourceEvidenceError::UnknownAttribute)
    }
    pub fn lookup_key(
        &self,
        query: SourceKeyQuery<'_>,
    ) -> Result<SourceKeyLookup<'_>, SourceEvidenceError> {
        if let Some(parent) = query.parent {
            self.row(parent)?;
        }
        let prefix = (
            query.parent.map(SourceOccurrenceId::ordinal),
            query.element.namespace,
            query.element.local,
            query.attribute.namespace,
            query.attribute.local,
        );
        let start = self
            .keys
            .partition_point(|key| key_prefix(&self.rows, key) < prefix);
        let end = self
            .keys
            .partition_point(|key| key_prefix(&self.rows, key) <= prefix);
        let scope = &self.keys[start..end];
        let start = scope.partition_point(|key| key_value(&self.rows, key) < query.value);
        let end = scope.partition_point(|key| key_value(&self.rows, key) <= query.value);
        let matches = &scope[start..end];
        let start = self
            .unavailable_keys
            .partition_point(|key| key_prefix(&self.rows, key) < prefix);
        let end = self
            .unavailable_keys
            .partition_point(|key| key_prefix(&self.rows, key) <= prefix);
        let unavailable = &self.unavailable_keys[start..end];
        Ok(if !unavailable.is_empty() {
            SourceKeyLookup::Unresolved {
                matches,
                unavailable,
            }
        } else {
            match matches {
                [] => SourceKeyLookup::Missing,
                [one] => SourceKeyLookup::Unique(*one),
                _ => SourceKeyLookup::Ambiguous(matches),
            }
        })
    }
}
