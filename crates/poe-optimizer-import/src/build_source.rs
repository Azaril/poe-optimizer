//! Source-preserving root structure, before defaults, load migrations or mechanics.
//!
//! This projection accepts arbitrary caller builds within the PoB XML lexical
//! subset. It retains unknown and duplicate sections in authored order. It does
//! not establish native support, MAIN/CALCS context, legality or numeric parity.
use crate::{
    configuration::ScalarInput,
    source_xml::{SourceText, SourceXmlError, invalid},
};
use roxmltree::{Document, Node, ParsingOptions};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::ops::Range;

pub const MAX_ROOT_SECTIONS: usize = 128;
pub const MAX_AUXILIARY_RECORDS: usize = 4096;
pub const MAX_ELEMENT_ATTRIBUTES: usize = 128;
pub const MAX_SOURCE_NAME_BYTES: usize = 1024;
pub const MAX_SOURCE_VALUE_BYTES: usize = 64 * 1024;
/// Bounds decoded attribute allocations independently of the full XML byte limit.
pub const MAX_PROJECTED_ATTRIBUTE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RootSectionKind {
    Build,
    Config,
    Items,
    Skills,
    Tree,
    Notes,
    Import,
    Party,
    Calcs,
    TreeView,
    LegacySpec,
    Unknown,
}
impl RootSectionKind {
    fn from_node(node: Node<'_, '_>) -> Self {
        if node.tag_name().namespace().is_some() {
            return Self::Unknown;
        }
        match node.tag_name().name() {
            "Build" => Self::Build,
            "Config" => Self::Config,
            "Items" => Self::Items,
            "Skills" => Self::Skills,
            "Tree" => Self::Tree,
            "Notes" => Self::Notes,
            "Import" => Self::Import,
            "Party" => Self::Party,
            "Calcs" => Self::Calcs,
            "TreeView" => Self::TreeView,
            "Spec" => Self::LegacySpec,
            _ => Self::Unknown,
        }
    }
    fn has_auxiliary_records(self) -> bool {
        matches!(
            self,
            Self::Import | Self::Party | Self::Calcs | Self::TreeView
        )
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct SourceAttribute<'input> {
    name: String,
    namespace: Option<String>,
    value: SourceText<'input>,
}
impl<'input> SourceAttribute<'input> {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }
    pub fn value(&self) -> &SourceText<'input> {
        &self.value
    }
}
/// Exact source and shallow shape. Nested payloads are intentionally opaque.
/// Namespace declarations remain in source_xml; has_namespaces includes inherited declarations.
#[derive(Debug, Clone, Serialize)]
pub struct SourceElement<'input> {
    name: String,
    namespace: Option<String>,
    has_namespaces: bool,
    source_range: Range<usize>,
    #[serde(skip)]
    source_xml: &'input str,
    attributes: Vec<SourceAttribute<'input>>,
    child_element_count: usize,
    has_non_whitespace_text: bool,
}
impl<'input> SourceElement<'input> {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }
    pub fn has_namespaces(&self) -> bool {
        self.has_namespaces
    }
    pub fn source_range(&self) -> Range<usize> {
        self.source_range.clone()
    }
    pub fn source_xml(&self) -> &'input str {
        self.source_xml
    }
    pub fn attributes(&self) -> &[SourceAttribute<'input>] {
        &self.attributes
    }
    /// Only an unnamespaced attribute can be addressed by its plain name.
    pub fn attribute(&self, name: &str) -> Option<&SourceText<'input>> {
        self.attributes
            .iter()
            .find(|a| a.name == name && a.namespace.is_none())
            .map(|a| &a.value)
    }
    pub fn child_element_count(&self) -> usize {
        self.child_element_count
    }
    pub fn has_non_whitespace_text(&self) -> bool {
        self.has_non_whitespace_text
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct AuxiliaryRecord<'input> {
    element: SourceElement<'input>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scalar_input: Option<ScalarInput<'input>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scalar_error: Option<SourceXmlError>,
}
impl<'input> AuxiliaryRecord<'input> {
    pub fn element(&self) -> &SourceElement<'input> {
        &self.element
    }
    pub fn scalar_input(&self) -> Option<&ScalarInput<'input>> {
        self.scalar_input.as_ref()
    }
    pub fn scalar_error(&self) -> Option<&SourceXmlError> {
        self.scalar_error.as_ref()
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct RootSection<'input> {
    kind: RootSectionKind,
    element: SourceElement<'input>,
    records: Vec<AuxiliaryRecord<'input>>,
}
impl<'input> RootSection<'input> {
    pub fn kind(&self) -> RootSectionKind {
        self.kind
    }
    pub fn element(&self) -> &SourceElement<'input> {
        &self.element
    }
    pub fn records(&self) -> &[AuxiliaryRecord<'input>] {
        &self.records
    }
}
/// Immutable evidence tied to one exact input, never a deserializable admission token.
#[derive(Debug, Clone, Serialize)]
pub struct RootProjection<'input> {
    source_sha256: String,
    #[serde(skip)]
    source_xml: &'input str,
    root: SourceElement<'input>,
    sections: Vec<RootSection<'input>>,
}
impl<'input> RootProjection<'input> {
    pub fn source_sha256(&self) -> &str {
        &self.source_sha256
    }
    pub fn source_xml(&self) -> &'input str {
        self.source_xml
    }
    pub fn root(&self) -> &SourceElement<'input> {
        &self.root
    }
    pub fn sections(&self) -> &[RootSection<'input>] {
        &self.sections
    }
}
fn element<'input>(
    node: Node<'_, 'input>,
    budget: &mut usize,
) -> Result<SourceElement<'input>, SourceXmlError> {
    let at = node.range().start;
    if node.tag_name().name().len() > MAX_SOURCE_NAME_BYTES
        || node.attributes().len() > MAX_ELEMENT_ATTRIBUTES
    {
        return Err(invalid(
            at,
            "source element exceeds name/attribute count limit",
        ));
    }
    let mut attributes = Vec::with_capacity(node.attributes().len());
    for attr in node.attributes() {
        let size = attr.range_value().len();
        if attr.name().len() > MAX_SOURCE_NAME_BYTES || size > MAX_SOURCE_VALUE_BYTES {
            return Err(invalid(
                attr.range().start,
                "source attribute exceeds name/value byte limit",
            ));
        }
        // Account for duplicated namespace/name strings as well as decoded values.
        let bytes = size + attr.name().len() + attr.namespace().map_or(0, str::len);
        *budget = budget
            .checked_sub(bytes)
            .ok_or_else(|| invalid(at, "projected attribute byte limit exceeded"))?;
        attributes.push(SourceAttribute {
            name: attr.name().into(),
            namespace: attr.namespace().map(str::to_owned),
            value: SourceText::from_range(node.document().input_text(), attr.range_value(), false)?,
        });
    }
    let namespace = node.tag_name().namespace();
    *budget = budget
        .checked_sub(node.tag_name().name().len() + namespace.map_or(0, str::len))
        .ok_or_else(|| invalid(at, "projected attribute byte limit exceeded"))?;
    Ok(SourceElement {
        name: node.tag_name().name().into(),
        namespace: namespace.map(str::to_owned),
        has_namespaces: node.namespaces().len() != 0,
        source_range: node.range(),
        source_xml: &node.document().input_text()[node.range()],
        attributes,
        child_element_count: node.children().filter(Node::is_element).count(),
        has_non_whitespace_text: node.children().any(|child| {
            child.is_text() && child.text().is_some_and(|s| !s.trim_ascii().is_empty())
        }),
    })
}
/// Recheck bounds and lexical compatibility even for an externally parsed document.
pub fn project<'input>(
    document: &Document<'input>,
) -> Result<RootProjection<'input>, SourceXmlError> {
    let xml = document.input_text();
    if xml.len() > crate::MAX_XML_BYTES
        || document.descendants().count() > crate::MAX_XML_NODES as usize
    {
        return Err(invalid(0, "build XML exceeds projection limits"));
    }
    crate::xml_compat::validate(xml).map_err(|e| invalid(e.byte_offset, e.reason))?;
    let root = document.root_element();
    if root.tag_name().name() != "PathOfBuilding2" || root.tag_name().namespace().is_some() {
        return Err(invalid(
            root.range().start,
            "expected unnamespaced PathOfBuilding2 root",
        ));
    }
    if root.children().filter(Node::is_element).count() > MAX_ROOT_SECTIONS {
        return Err(invalid(
            root.range().start,
            "root section count exceeds projection limit",
        ));
    }
    let mut budget = MAX_PROJECTED_ATTRIBUTE_BYTES;
    let root_element = element(root, &mut budget)?;
    let mut sections = Vec::new();
    let mut record_count = 0usize;
    for node in root.children().filter(Node::is_element) {
        let kind = RootSectionKind::from_node(node);
        let source_element = element(node, &mut budget)?;
        let mut records = Vec::new();
        if kind.has_auxiliary_records() {
            for child in node.children().filter(Node::is_element) {
                record_count += 1;
                if record_count > MAX_AUXILIARY_RECORDS {
                    return Err(invalid(
                        child.range().start,
                        "auxiliary record count exceeds projection limit",
                    ));
                }
                let record_element = element(child, &mut budget)?;
                let (scalar_input, scalar_error) = if kind == RootSectionKind::Calcs
                    && child.has_tag_name("Input")
                    && child.namespaces().len() == 0
                {
                    match crate::configuration::scalar_record(child) {
                        Ok(input) => (Some(input), None),
                        Err(error) => (None, Some(error)),
                    }
                } else {
                    (None, None)
                };
                records.push(AuxiliaryRecord {
                    element: record_element,
                    scalar_input,
                    scalar_error,
                });
            }
        }
        sections.push(RootSection {
            kind,
            element: source_element,
            records,
        });
    }
    Ok(RootProjection {
        source_sha256: format!("{:x}", Sha256::digest(xml.as_bytes())),
        source_xml: xml,
        root: root_element,
        sections,
    })
}
/// Project caller XML without requiring Build, skills, equipment, or a data package.
/// Admission and effect evaluation are separate consumers.
pub fn project_xml(xml: &str) -> Result<RootProjection<'_>, SourceXmlError> {
    if xml.len() > crate::MAX_XML_BYTES {
        return Err(invalid(0, "build XML exceeds projection limits"));
    }
    let document = Document::parse_with_options(
        xml,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: crate::MAX_XML_NODES,
            entity_resolver: None,
        },
    )
    .map_err(|e| invalid(0, format!("invalid build XML: {e}")))?;
    project(&document)
}
