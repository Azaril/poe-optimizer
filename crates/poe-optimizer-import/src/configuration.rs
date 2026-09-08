//! Bounded authored configuration projection, independent of native capability.
//! Values model XML source decoding, before ConfigTab defaults, key migrations,
//! Placeholder destination rules, legacy customMods migration, or game effects.
use crate::source_xml::{SourceText, SourceXmlError, attribute, element_text, invalid};
use poe_optimizer_core::options::Scalar;
use roxmltree::{Document, Node, ParsingOptions};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, ops::Range};
pub type ConfigurationError = SourceXmlError;
pub const MAX_CONFIG_BYTES: usize = 1024 * 1024;
pub const MAX_CONFIG_SETS: usize = 64;
pub const MAX_CONFIG_RECORDS: usize = 4096;
pub const MAX_CONFIG_VALUE_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScalarKind {
    Number,
    String,
    Boolean,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScalarInput<'input> {
    name: String,
    name_source: SourceText<'input>,
    kind: ScalarKind,
    value: Scalar,
    value_source: SourceText<'input>,
    source_range: Range<usize>,
    source_xml: &'input str,
}
impl<'input> ScalarInput<'input> {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn name_source(&self) -> &SourceText<'input> {
        &self.name_source
    }
    pub fn kind(&self) -> ScalarKind {
        self.kind
    }
    pub fn value(&self) -> &Scalar {
        &self.value
    }
    pub fn value_source(&self) -> &SourceText<'input> {
        &self.value_source
    }
    pub fn source_range(&self) -> Range<usize> {
        self.source_range.clone()
    }
    pub fn source_xml(&self) -> &'input str {
        self.source_xml
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnhandledRecord<'input> {
    element_name: String,
    source_range: Range<usize>,
    source_xml: &'input str,
}
impl<'input> UnhandledRecord<'input> {
    pub fn element_name(&self) -> &str {
        &self.element_name
    }
    pub fn source_range(&self) -> Range<usize> {
        self.source_range.clone()
    }
    pub fn source_xml(&self) -> &'input str {
        self.source_xml
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConfigurationBlock<'input> {
    title: String,
    title_source: Option<SourceText<'input>>,
    enabled: bool,
    enabled_source: Option<SourceText<'input>>,
    text: SourceText<'input>,
    pob_text: String,
    source_range: Range<usize>,
    source_xml: &'input str,
}
impl<'input> ConfigurationBlock<'input> {
    pub fn title(&self) -> &str {
        &self.title
    }
    pub fn title_source(&self) -> Option<&SourceText<'input>> {
        self.title_source.as_ref()
    }
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    pub fn enabled_source(&self) -> Option<&SourceText<'input>> {
        self.enabled_source.as_ref()
    }
    pub fn text(&self) -> &SourceText<'input> {
        &self.text
    }
    /// Original XML table's whole text string (ordinary boundary whitespace stripped).
    pub fn pob_text(&self) -> &str {
        &self.pob_text
    }
    pub fn source_range(&self) -> Range<usize> {
        self.source_range.clone()
    }
    pub fn source_xml(&self) -> &'input str {
        self.source_xml
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigurationLayout {
    Missing,
    Empty,
    Legacy,
    ExplicitSets,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActiveSetResolution {
    Requested,
    DefaultOne,
    MissingRequestedUsesFirst,
    MissingDefaultUsesFirst,
}
/// Source-order index into the corresponding immutable per-kind record slice.
/// Consumers must use this order when modeling ConfigTab's sequential writes;
/// notably a string Placeholder writes into its input table in the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "index", rename_all = "snake_case")]
pub enum ConfigurationRecordIndex {
    Input(usize),
    Placeholder(usize),
    Block(usize),
    Unknown(usize),
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ConfigSetProjection<'input> {
    id: u32,
    id_source: Option<SourceText<'input>>,
    title: String,
    title_source: Option<SourceText<'input>>,
    implicit: bool,
    source_range: Option<Range<usize>>,
    source_xml: Option<&'input str>,
    records_in_source_order: Vec<ConfigurationRecordIndex>,
    inputs: Vec<ScalarInput<'input>>,
    placeholders: Vec<ScalarInput<'input>>,
    blocks: Vec<ConfigurationBlock<'input>>,
    unknown_records: Vec<UnhandledRecord<'input>>,
}
impl<'input> ConfigSetProjection<'input> {
    pub fn id(&self) -> u32 {
        self.id
    }
    pub fn id_source(&self) -> Option<&SourceText<'input>> {
        self.id_source.as_ref()
    }
    pub fn title(&self) -> &str {
        &self.title
    }
    pub fn title_source(&self) -> Option<&SourceText<'input>> {
        self.title_source.as_ref()
    }
    pub fn is_implicit(&self) -> bool {
        self.implicit
    }
    pub fn source_range(&self) -> Option<Range<usize>> {
        self.source_range.clone()
    }
    pub fn source_xml(&self) -> Option<&'input str> {
        self.source_xml
    }
    pub fn records_in_source_order(&self) -> &[ConfigurationRecordIndex] {
        &self.records_in_source_order
    }
    pub fn inputs(&self) -> &[ScalarInput<'input>] {
        &self.inputs
    }
    pub fn placeholders(&self) -> &[ScalarInput<'input>] {
        &self.placeholders
    }
    pub fn blocks(&self) -> &[ConfigurationBlock<'input>] {
        &self.blocks
    }
    pub fn unknown_records(&self) -> &[UnhandledRecord<'input>] {
        &self.unknown_records
    }
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ConfigurationProjection<'input> {
    #[serde(skip)]
    source_xml: &'input str,
    source_sha256: String,
    layout: ConfigurationLayout,
    config_range: Option<Range<usize>>,
    config_source: Option<&'input str>,
    requested_active_set_id: Option<u32>,
    active_source: Option<SourceText<'input>>,
    active_set_id: u32,
    active_set_resolution: ActiveSetResolution,
    sets: Vec<ConfigSetProjection<'input>>,
}
impl<'input> ConfigurationProjection<'input> {
    pub fn source_xml(&self) -> &'input str {
        self.source_xml
    }
    pub fn source_sha256(&self) -> &str {
        &self.source_sha256
    }
    pub fn layout(&self) -> ConfigurationLayout {
        self.layout
    }
    pub fn config_range(&self) -> Option<Range<usize>> {
        self.config_range.clone()
    }
    pub fn config_source(&self) -> Option<&'input str> {
        self.config_source
    }
    pub fn sets(&self) -> &[ConfigSetProjection<'input>] {
        &self.sets
    }
    pub fn requested_active_set_id(&self) -> Option<u32> {
        self.requested_active_set_id
    }
    pub fn active_set_id(&self) -> u32 {
        self.active_set_id
    }
    pub fn active_set_resolution(&self) -> ActiveSetResolution {
        self.active_set_resolution
    }
    pub fn active_set(&self) -> &ConfigSetProjection<'input> {
        self.sets
            .iter()
            .find(|s| s.id == self.active_set_id)
            .expect("validated active source set")
    }
    pub fn diagnostic(&self) -> serde_json::Value {
        serde_json::json!({"schema_version":1,"interpretation":"authored_configuration_source",
            "effective_configuration":"not_evaluated","mechanics":"not_evaluated","game_legality":"not_evaluated",
            "projection":self})
    }
    pub(crate) fn preserved_string_input_ranges(&self) -> Vec<Range<usize>> {
        self.sets
            .iter()
            .flat_map(|s| &s.inputs)
            .filter(|i| i.kind == ScalarKind::String)
            .map(|i| i.value_source.range())
            .collect()
    }
}
fn attributes_only(node: Node<'_, '_>, allowed: &[&str]) -> Result<(), ConfigurationError> {
    if node.tag_name().namespace().is_some()
        || node.namespaces().len() != 0
        || node
            .attributes()
            .any(|a| a.namespace().is_some() || !allowed.contains(&a.name()))
    {
        return Err(invalid(
            node.range().start,
            "unsupported configuration namespace or attributes",
        ));
    }
    Ok(())
}
fn non_element_children_are_ignorable(node: Node<'_, '_>) -> bool {
    node.children().filter(|n| !n.is_element()).all(|n| {
        n.is_comment()
            || n.is_pi()
            || (n.is_text() && n.text().is_some_and(|t| t.trim_ascii().is_empty()))
    })
}
fn bounded_attribute<'input>(
    node: Node<'_, 'input>,
    name: &str,
    max: usize,
) -> Result<Option<SourceText<'input>>, ConfigurationError> {
    if node
        .attributes()
        .find(|a| a.name() == name)
        .is_some_and(|a| a.range_value().len() > max)
    {
        return Err(invalid(
            node.range().start,
            "configuration attribute exceeds byte limit",
        ));
    }
    attribute(node, name)
}
pub(crate) fn scalar_record<'input>(
    node: Node<'_, 'input>,
) -> Result<ScalarInput<'input>, ConfigurationError> {
    let placeholder = node.has_tag_name("Placeholder");
    if !placeholder && !node.has_tag_name("Input") {
        return Err(invalid(
            node.range().start,
            "expected Input or Placeholder scalar",
        ));
    }
    attributes_only(
        node,
        if placeholder {
            &["name", "number", "string"]
        } else {
            &["name", "number", "string", "boolean"]
        },
    )?;
    if node.children().any(|n| n.is_element()) || !non_element_children_are_ignorable(node) {
        return Err(invalid(
            node.range().start,
            "configuration scalar must have no child payload",
        ));
    }
    let name_source = bounded_attribute(node, "name", 1024)?
        .ok_or_else(|| invalid(node.range().start, "configuration scalar requires name"))?;
    let mut values = Vec::new();
    for (name, kind) in [
        ("number", ScalarKind::Number),
        ("string", ScalarKind::String),
        ("boolean", ScalarKind::Boolean),
    ] {
        if let Some(source) = bounded_attribute(node, name, MAX_CONFIG_VALUE_BYTES)? {
            values.push((kind, source));
        }
    }
    if values.len() != 1 {
        return Err(invalid(
            node.range().start,
            "configuration scalar requires exactly one type",
        ));
    }
    let (kind, value_source) = values.pop().expect("one scalar");
    let value = match kind {
        ScalarKind::Number => {
            let number = value_source
                .decoded()
                .trim_ascii()
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .ok_or_else(|| {
                    invalid(
                        value_source.range().start,
                        "configuration number requires a finite decimal",
                    )
                })?;
            Scalar::Number(number)
        }
        ScalarKind::String => Scalar::Text(value_source.decoded().into()),
        ScalarKind::Boolean => match value_source.decoded() {
            "true" => Scalar::Boolean(true),
            "false" => Scalar::Boolean(false),
            _ => {
                return Err(invalid(
                    value_source.range().start,
                    "configuration boolean must be true or false",
                ));
            }
        },
    };
    Ok(ScalarInput {
        name: name_source.decoded().into(),
        name_source,
        kind,
        value,
        value_source,
        source_range: node.range(),
        source_xml: &node.document().input_text()[node.range()],
    })
}
/// Shared source-preserving reader. It accepts bounded Input/Placeholder source
/// scalars only; the caller retains its separate structure/key/capability policy.
pub fn read_input_scalar(node: Node<'_, '_>) -> Result<Scalar, ConfigurationError> {
    crate::xml_compat::validate(&node.document().input_text()[node.range()])
        .map_err(|error| invalid(node.range().start + error.byte_offset, error.reason))?;
    Ok(scalar_record(node)?.value)
}
fn positive_id(value: &SourceText<'_>) -> Result<u32, ConfigurationError> {
    let raw = value.decoded();
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid(
            value.range().start,
            "configuration set ID must be a positive decimal integer",
        ));
    }
    raw.parse::<u32>().ok().filter(|v| *v > 0).ok_or_else(|| {
        invalid(
            value.range().start,
            "configuration set ID is outside positive u32 range",
        )
    })
}
fn block<'input>(node: Node<'_, 'input>) -> Result<ConfigurationBlock<'input>, ConfigurationError> {
    attributes_only(node, &["title", "enabled"])?;
    let title_source = bounded_attribute(node, "title", 1024)?;
    let enabled_source = bounded_attribute(node, "enabled", 5)?;
    let enabled = match enabled_source.as_ref().map(SourceText::decoded) {
        None | Some("true") => true,
        Some("false") => false,
        _ => {
            return Err(invalid(
                node.range().start,
                "block enabled must be true or false",
            ));
        }
    };
    let text = element_text(node)?;
    if text.raw().len() > MAX_CONFIG_VALUE_BYTES {
        return Err(invalid(
            text.range().start,
            "configuration block exceeds byte limit",
        ));
    }
    let cdata = node
        .document()
        .input_text()
        .get(text.range().start.saturating_sub(9)..text.range().start)
        == Some("<![CDATA[");
    let pob_text = if cdata && !text.decoded().trim_ascii().is_empty() {
        text.decoded().to_owned()
    } else {
        text.decoded().trim_ascii().to_owned()
    };
    Ok(ConfigurationBlock {
        title: title_source
            .as_ref()
            .map_or("Default", SourceText::decoded)
            .into(),
        title_source,
        enabled,
        enabled_source,
        text,
        pob_text,
        source_range: node.range(),
        source_xml: &node.document().input_text()[node.range()],
    })
}
fn make_set<'input>(
    node: Option<Node<'_, 'input>>,
    legacy: bool,
) -> Result<ConfigSetProjection<'input>, ConfigurationError> {
    let mut set = ConfigSetProjection {
        id: 1,
        id_source: None,
        title: "Default".into(),
        title_source: None,
        implicit: legacy || node.is_none(),
        source_range: node.map(|n| n.range()),
        source_xml: node.map(|n| &n.document().input_text()[n.range()]),
        records_in_source_order: vec![],
        inputs: vec![],
        placeholders: vec![],
        blocks: vec![],
        unknown_records: vec![],
    };
    let Some(node) = node else { return Ok(set) };
    if !legacy {
        attributes_only(node, &["id", "title"])?;
        let id = bounded_attribute(node, "id", 32)?
            .ok_or_else(|| invalid(node.range().start, "ConfigSet requires an explicit ID"))?;
        set.id = positive_id(&id)?;
        set.id_source = Some(id);
        set.title_source = bounded_attribute(node, "title", 1024)?;
        set.title = set
            .title_source
            .as_ref()
            .map_or("Default", SourceText::decoded)
            .into();
    }
    if !non_element_children_are_ignorable(node) {
        return Err(invalid(
            node.range().start,
            "configuration containers must not contain text payloads",
        ));
    }
    let mut input_names = BTreeSet::new();
    let mut placeholder_names = BTreeSet::new();
    for child in node.children().filter(Node::is_element) {
        if child.tag_name().namespace().is_some() || child.namespaces().len() != 0 {
            return Err(invalid(
                child.range().start,
                "namespaced configuration records are unsupported",
            ));
        }
        match child.tag_name().name() {
            "Input" | "Placeholder" => {
                let input = scalar_record(child)?;
                let names = if child.has_tag_name("Input") {
                    &mut input_names
                } else {
                    &mut placeholder_names
                };
                if !names.insert(input.name.clone()) {
                    return Err(invalid(
                        child.range().start,
                        "duplicate configuration scalar name",
                    ));
                }
                if child.has_tag_name("Input") {
                    set.records_in_source_order
                        .push(ConfigurationRecordIndex::Input(set.inputs.len()));
                    set.inputs.push(input)
                } else {
                    set.records_in_source_order
                        .push(ConfigurationRecordIndex::Placeholder(
                            set.placeholders.len(),
                        ));
                    set.placeholders.push(input)
                }
            }
            "CustomModifierBlock" => {
                set.records_in_source_order
                    .push(ConfigurationRecordIndex::Block(set.blocks.len()));
                set.blocks.push(block(child)?);
            }
            "ConfigSet" => {
                return Err(invalid(
                    child.range().start,
                    "nested or mixed ConfigSet structure is unsupported",
                ));
            }
            _ => {
                set.records_in_source_order
                    .push(ConfigurationRecordIndex::Unknown(set.unknown_records.len()));
                set.unknown_records.push(UnhandledRecord {
                    element_name: child.tag_name().name().into(),
                    source_range: child.range(),
                    source_xml: &child.document().input_text()[child.range()],
                });
            }
        }
    }
    Ok(set)
}
/// Project a previously parsed bounded build document. Rechecks bounds/root and
/// lexical compatibility; unsupported game keys remain ordinary source records.
pub fn project<'input>(
    document: &Document<'input>,
) -> Result<ConfigurationProjection<'input>, ConfigurationError> {
    let xml = document.input_text();
    if xml.len() > crate::MAX_XML_BYTES
        || document.descendants().count() > crate::MAX_XML_NODES as usize
    {
        return Err(invalid(0, "build XML exceeds projection limits"));
    }
    crate::source_xml::validate_ordered_xml(xml)?;
    let root = document.root_element();
    if !root.has_tag_name("PathOfBuilding2")
        || root.tag_name().namespace().is_some()
        || root.namespaces().len() != 0
    {
        return Err(invalid(
            root.range().start,
            "configuration requires an unnamespaced PathOfBuilding2 root",
        ));
    }
    let nodes: Vec<_> = root
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "Config")
        .collect();
    if nodes.len() > 1 {
        return Err(invalid(
            nodes[1].range().start,
            "duplicate Config containers",
        ));
    }
    let config = nodes.first().copied();
    let mut sets = Vec::new();
    let mut active_source = None;
    let layout = if let Some(config) = config {
        if config.range().len() > MAX_CONFIG_BYTES {
            return Err(invalid(
                config.range().start,
                "configuration exceeds 1 MiB source limit",
            ));
        }
        if config.descendants().filter(Node::is_element).count()
            > MAX_CONFIG_RECORDS + MAX_CONFIG_SETS + 1
        {
            return Err(invalid(
                config.range().start,
                "too many configuration source records",
            ));
        }
        attributes_only(config, &["activeConfigSet"])?;
        active_source = bounded_attribute(config, "activeConfigSet", 32)?;
        if !non_element_children_are_ignorable(config) {
            return Err(invalid(
                config.range().start,
                "Config must not contain text payloads",
            ));
        }
        let children: Vec<_> = config.children().filter(Node::is_element).collect();
        let explicit = children
            .iter()
            .filter(|n| n.has_tag_name("ConfigSet"))
            .count();
        if explicit > 0 {
            if explicit != children.len() {
                return Err(invalid(
                    config.range().start,
                    "mixed legacy and explicit configuration sets are unsupported",
                ));
            }
            if explicit > MAX_CONFIG_SETS {
                return Err(invalid(config.range().start, "too many configuration sets"));
            }
            let mut ids = BTreeSet::new();
            for node in children {
                let set = make_set(Some(node), false)?;
                if !ids.insert(set.id) {
                    return Err(invalid(
                        node.range().start,
                        "duplicate configuration set ID",
                    ));
                }
                sets.push(set);
            }
            ConfigurationLayout::ExplicitSets
        } else {
            sets.push(make_set(Some(config), true)?);
            if children.is_empty() {
                ConfigurationLayout::Empty
            } else {
                ConfigurationLayout::Legacy
            }
        }
    } else {
        sets.push(make_set(None, true)?);
        ConfigurationLayout::Missing
    };
    let requested_active_set_id = active_source.as_ref().map(positive_id).transpose()?;
    let requested = requested_active_set_id.unwrap_or(1);
    let present = sets.iter().any(|s| s.id == requested);
    let active_set_id = if present { requested } else { sets[0].id };
    let active_set_resolution = match (requested_active_set_id.is_some(), present) {
        (true, true) => ActiveSetResolution::Requested,
        (false, true) => ActiveSetResolution::DefaultOne,
        (true, false) => ActiveSetResolution::MissingRequestedUsesFirst,
        (false, false) => ActiveSetResolution::MissingDefaultUsesFirst,
    };
    let records: usize = sets
        .iter()
        .map(|s| s.inputs.len() + s.placeholders.len() + s.blocks.len() + s.unknown_records.len())
        .sum();
    if records > MAX_CONFIG_RECORDS {
        return Err(invalid(
            config.map_or(0, |n| n.range().start),
            "too many configuration records",
        ));
    }
    Ok(ConfigurationProjection {
        source_xml: xml,
        source_sha256: format!("{:x}", Sha256::digest(xml.as_bytes())),
        layout,
        config_range: config.map(|n| n.range()),
        config_source: config.map(|n| &xml[n.range()]),
        requested_active_set_id,
        active_source,
        active_set_id,
        active_set_resolution,
        sets,
    })
}
/// Parse and project with the same bounded, DTD-disabled generic import rules.
pub fn project_xml(xml: &str) -> Result<ConfigurationProjection<'_>, ConfigurationError> {
    if xml.len() > crate::MAX_XML_BYTES {
        return Err(invalid(0, "build XML exceeds projection limits"));
    }
    let document = Document::parse_with_options(
        xml,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: crate::MAX_XML_NODES,
            ..ParsingOptions::default()
        },
    )
    .map_err(|e| invalid(0, e.to_string()))?;
    project(&document)
}
