//! Authored Skills structure, before SkillsTab defaults or identity resolution.
//!
//! Known XML names and the original loader's consumer roles are separate: the
//! loader treats every child of Skill as a gem instance and every child of a
//! minion lookup as a map, irrespective of its tag. Unknown names stay unknown.
//! This projection preserves source, not effective settings, legality or native
//! capability. No data package, Build section, selected skill or Lua is required.
use crate::{
    build_source::{self, SourceElement},
    source_xml::{SourceXmlError, invalid},
};
use roxmltree::{Document, Node, ParsingOptions};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const MAX_SKILLS_CONTAINERS: usize = 128;
pub const MAX_SKILL_SETS: usize = 256;
pub const MAX_SKILL_GROUPS: usize = 4096;
pub const MAX_GEM_INSTANCES: usize = 16_384;
pub const MAX_SKILL_SOURCE_NODES: usize = 32_768;
pub const MAX_SKILL_SOURCE_DEPTH: usize = 32;
pub const MAX_SKILL_SOURCE_ATTRIBUTES: usize = 65_536;
pub const MAX_SKILL_SOURCE_DIAGNOSTICS: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillSourceKind {
    Skills,
    SkillSet,
    Skill,
    Gem,
    StatSetIndex,
    StatSetCalcsIndex,
    MinionSkillIndexLookup,
    MinionSkillIndexLookupCalcs,
    MinionSkillIndexMap,
    Unknown,
}

/// Source consumer role, not proof that loading or resolving this node succeeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillSourceUse {
    Container,
    SavedSet,
    Group,
    GemInstance,
    MainStatSetSelection,
    CalcsStatSetSelection,
    MainMinionLookup,
    CalcsMinionLookup,
    MinionIndexMap,
    Ignored,
    NamespaceUnknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillAttributeRole {
    SetIdentity,
    ActiveSetRequest,
    CatalogIdentity,
    NameIdentityHint,
    DisplayText,
    DisplayPreference,
    DefaultPreference,
    WeaponSlot,
    GrantSource,
    Enabled,
    GlobalEffectToggle,
    FullDpsInclusion,
    Count,
    Level,
    Quality,
    Corruption,
    MainSelection,
    CalcsSelection,
    LookupEffectIdentity,
    LookupIndex,
    LegacyResetSelection,
    LegacyGroupPart,
    Unknown,
}

/// Authored syntax only. In particular, non-canonical booleans are not changed
/// into false, and numeric text is not replaced with a source default or clamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillAttributeSyntax {
    Text,
    FiniteDecimal,
    NonFiniteDecimal,
    OtherNumericText,
    CanonicalBoolean,
    OtherBooleanText,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillSourceAttribute {
    source_index: usize,
    role: SkillAttributeRole,
    syntax: SkillAttributeSyntax,
}
impl SkillSourceAttribute {
    /// Index into this node's SourceElement::attributes(), retaining exact text.
    pub fn source_index(&self) -> usize {
        self.source_index
    }
    pub fn role(&self) -> SkillAttributeRole {
        self.role
    }
    pub fn syntax(&self) -> SkillAttributeSyntax {
        self.syntax
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillDiagnosticCode {
    UnknownElement,
    NamespaceContext,
    UnknownAttribute,
    NumericOutsideFiniteDecimal,
    NonCanonicalBoolean,
    MissingSetId,
    MissingGemIdentity,
    MissingLookupKey,
    DuplicateNumericSetId,
    DuplicateLookupKey,
    DuplicateSkillsContainer,
    MixedLegacyAndSavedSets,
    LegacyDirectGroup,
    PositionalGemChild,
    PositionalMinionMapChild,
    LegacyStatSetAttributeReset,
    UnhandledText,
}
#[derive(Debug, Clone, Serialize)]
pub struct SkillSourceDiagnostic {
    code: SkillDiagnosticCode,
    byte_offset: usize,
    message: &'static str,
}
impl SkillSourceDiagnostic {
    pub fn code(&self) -> SkillDiagnosticCode {
        self.code
    }
    pub fn byte_offset(&self) -> usize {
        self.byte_offset
    }
    pub fn message(&self) -> &str {
        self.message
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillSourceNode<'input> {
    kind: SkillSourceKind,
    source_use: SkillSourceUse,
    element: SourceElement<'input>,
    attributes: Vec<SkillSourceAttribute>,
    diagnostics: Vec<SkillSourceDiagnostic>,
    children: Vec<SkillSourceNode<'input>>,
}
impl<'input> SkillSourceNode<'input> {
    pub fn kind(&self) -> SkillSourceKind {
        self.kind
    }
    pub fn source_use(&self) -> SkillSourceUse {
        self.source_use
    }
    pub fn element(&self) -> &SourceElement<'input> {
        &self.element
    }
    pub fn attributes(&self) -> &[SkillSourceAttribute] {
        &self.attributes
    }
    pub fn diagnostics(&self) -> &[SkillSourceDiagnostic] {
        &self.diagnostics
    }
    pub fn children(&self) -> &[SkillSourceNode<'input>] {
        &self.children
    }
}

/// Immutable inspection evidence, bound to exact caller bytes. This is not a
/// deserializable admission token or a future optimizer candidate identity.
#[derive(Debug, Clone, Serialize)]
pub struct SkillProjection<'input> {
    source_sha256: String,
    #[serde(skip)]
    source_xml: &'input str,
    containers: Vec<SkillSourceNode<'input>>,
}
impl<'input> SkillProjection<'input> {
    pub fn source_sha256(&self) -> &str {
        &self.source_sha256
    }
    pub fn source_xml(&self) -> &'input str {
        self.source_xml
    }
    pub fn containers(&self) -> &[SkillSourceNode<'input>] {
        &self.containers
    }
}

struct Budget {
    bytes: usize,
    nodes: usize,
    attributes: usize,
    diagnostics: usize,
    sets: usize,
    groups: usize,
    instances: usize,
}
fn consume(
    left: &mut usize,
    amount: usize,
    at: usize,
    reason: &'static str,
) -> Result<(), SourceXmlError> {
    *left = left
        .checked_sub(amount)
        .ok_or_else(|| invalid(at, reason))?;
    Ok(())
}
fn note(
    node: &mut SkillSourceNode<'_>,
    budget: &mut Budget,
    code: SkillDiagnosticCode,
    at: usize,
    message: &'static str,
) -> Result<(), SourceXmlError> {
    consume(
        &mut budget.diagnostics,
        1,
        at,
        "skills diagnostic count limit exceeded",
    )?;
    node.diagnostics.push(SkillSourceDiagnostic {
        code,
        byte_offset: at,
        message,
    });
    Ok(())
}
pub(crate) fn classification(
    parent: Option<SkillSourceUse>,
    name: &str,
    namespace: bool,
) -> (SkillSourceKind, SkillSourceUse) {
    use SkillSourceKind as K;
    use SkillSourceUse as U;
    if namespace {
        return (K::Unknown, U::NamespaceUnknown);
    }
    match (parent, name) {
        (None, "Skills") => (K::Skills, U::Container),
        (Some(U::Container), "SkillSet") => (K::SkillSet, U::SavedSet),
        (Some(U::Container | U::SavedSet), "Skill") => (K::Skill, U::Group),
        (Some(U::Group), "Gem") => (K::Gem, U::GemInstance),
        (Some(U::Group), _) => (K::Unknown, U::GemInstance),
        (Some(U::GemInstance), "StatSetIndex") => (K::StatSetIndex, U::MainStatSetSelection),
        (Some(U::GemInstance), "StatSetCalcsIndex") => {
            (K::StatSetCalcsIndex, U::CalcsStatSetSelection)
        }
        (Some(U::GemInstance), "MinionSkillIndexLookup") => {
            (K::MinionSkillIndexLookup, U::MainMinionLookup)
        }
        (Some(U::GemInstance), "MinionSkillIndexLookupCalcs") => {
            (K::MinionSkillIndexLookupCalcs, U::CalcsMinionLookup)
        }
        (Some(U::MainMinionLookup | U::CalcsMinionLookup), "MinionSkillIndexMap") => {
            (K::MinionSkillIndexMap, U::MinionIndexMap)
        }
        (Some(U::MainMinionLookup | U::CalcsMinionLookup), _) => (K::Unknown, U::MinionIndexMap),
        _ => (K::Unknown, U::Ignored),
    }
}
#[derive(Clone, Copy)]
enum Expected {
    Text,
    Number,
    Boolean,
}
fn attribute_role(usage: SkillSourceUse, name: &str) -> (SkillAttributeRole, Expected) {
    use Expected as E;
    use SkillAttributeRole as R;
    use SkillSourceUse as U;
    match (usage, name) {
        (U::Container, "activeSkillSet") => (R::ActiveSetRequest, E::Number),
        (U::Container, "defaultGemLevel") => (R::DefaultPreference, E::Text),
        (U::Container, "defaultGemQuality") => (R::DefaultPreference, E::Number),
        (U::Container, "matchGemLevelToCharacterLevel") => (R::DefaultPreference, E::Boolean),
        (U::Container, "sortGemsByDPS" | "showLegacyGems") => (R::DisplayPreference, E::Boolean),
        (U::Container, "sortGemsByDPSField" | "showSupportGemTypes") => {
            (R::DisplayPreference, E::Text)
        }
        (U::SavedSet, "id") => (R::SetIdentity, E::Number),
        (U::SavedSet, "title") | (U::Group, "label") | (U::GemInstance, "note") => {
            (R::DisplayText, E::Text)
        }
        (U::Group, "slot") => (R::WeaponSlot, E::Text),
        (U::Group, "source") => (R::GrantSource, E::Text),
        (U::Group, "enabled" | "active") | (U::GemInstance, "enabled") => (R::Enabled, E::Boolean),
        (U::Group, "includeInFullDPS") => (R::FullDpsInclusion, E::Boolean),
        (U::Group, "groupCount") | (U::GemInstance, "count") => (R::Count, E::Number),
        (U::Group, "mainActiveSkill") => (R::MainSelection, E::Number),
        (U::Group, "mainActiveSkillCalcs") => (R::CalcsSelection, E::Number),
        (U::Group, "skillPart") => (R::LegacyGroupPart, E::Number),
        (U::GemInstance, "gemId" | "variantId" | "skillId") => (R::CatalogIdentity, E::Text),
        (U::GemInstance, "nameSpec") => (R::NameIdentityHint, E::Text),
        (U::GemInstance, "level") => (R::Level, E::Number),
        (U::GemInstance, "quality") => (R::Quality, E::Number),
        (U::GemInstance, "enableGlobal1" | "enableGlobal2") => (R::GlobalEffectToggle, E::Boolean),
        (U::GemInstance, "corrupted") => (R::Corruption, E::Boolean),
        (U::GemInstance, "corruptLevel") => (R::Corruption, E::Number),
        (U::GemInstance, "statSetIndex" | "statSetIndexCalcs") => {
            (R::LegacyResetSelection, E::Number)
        }
        (U::GemInstance, "skillMinion") => (R::MainSelection, E::Text),
        (U::GemInstance, "skillMinionCalcs") => (R::CalcsSelection, E::Text),
        (
            U::GemInstance,
            "skillPart" | "skillStageCount" | "skillMineCount" | "skillMinionItemSet"
            | "skillMinionSkill",
        ) => (R::MainSelection, E::Number),
        (
            U::GemInstance,
            "skillPartCalcs"
            | "skillStageCountCalcs"
            | "skillMineCountCalcs"
            | "skillMinionItemSetCalcs"
            | "skillMinionSkillCalcs",
        ) => (R::CalcsSelection, E::Number),
        (
            U::MainStatSetSelection
            | U::CalcsStatSetSelection
            | U::MainMinionLookup
            | U::CalcsMinionLookup,
            "grantedEffect",
        ) => (R::LookupEffectIdentity, E::Text),
        (U::MainStatSetSelection | U::CalcsStatSetSelection, "index")
        | (U::MinionIndexMap, "skillIndex" | "statSetIndex") => (R::LookupIndex, E::Number),
        _ => (R::Unknown, E::Text),
    }
}
fn finite_decimal(text: &str) -> Result<f64, SkillAttributeSyntax> {
    let text = text.trim_ascii();
    let bytes = text.as_bytes();
    let mut at = usize::from(matches!(bytes.first(), Some(b'+' | b'-')));
    let mut digits = 0;
    while bytes.get(at).is_some_and(u8::is_ascii_digit) {
        at += 1;
        digits += 1;
    }
    if bytes.get(at) == Some(&b'.') {
        at += 1;
        while bytes.get(at).is_some_and(u8::is_ascii_digit) {
            at += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return Err(SkillAttributeSyntax::OtherNumericText);
    }
    if matches!(bytes.get(at), Some(b'e' | b'E')) {
        at += 1;
        if matches!(bytes.get(at), Some(b'+' | b'-')) {
            at += 1;
        }
        let start = at;
        while bytes.get(at).is_some_and(u8::is_ascii_digit) {
            at += 1;
        }
        if at == start {
            return Err(SkillAttributeSyntax::OtherNumericText);
        }
    }
    if at != bytes.len() {
        return Err(SkillAttributeSyntax::OtherNumericText);
    }
    match text.parse::<f64>() {
        Ok(value) if value.is_finite() => Ok(value),
        _ => Err(SkillAttributeSyntax::NonFiniteDecimal),
    }
}
fn syntax(text: &str, expected: Expected) -> SkillAttributeSyntax {
    use SkillAttributeSyntax as S;
    match expected {
        Expected::Text => S::Text,
        Expected::Number => finite_decimal(text).map_or_else(|s| s, |_| S::FiniteDecimal),
        Expected::Boolean if matches!(text, "true" | "false") => S::CanonicalBoolean,
        Expected::Boolean => S::OtherBooleanText,
    }
}
fn number_key(node: &SkillSourceNode<'_>, name: &str) -> Option<u64> {
    let value = finite_decimal(node.element.attribute(name)?.decoded()).ok()?;
    // Lua numeric keys equate positive/negative zero. This diagnostic only
    // compares finite decimal spellings; other tonumber forms stay unresolved.
    Some(if value == 0.0 { 0 } else { value.to_bits() })
}
fn duplicate_diagnostics(
    node: &mut SkillSourceNode<'_>,
    budget: &mut Budget,
) -> Result<(), SourceXmlError> {
    use SkillDiagnosticCode as D;
    use SkillSourceUse as U;
    let mut sets = BTreeMap::new();
    let mut lookups = BTreeMap::new();
    let mut maps = BTreeMap::new();
    let mut legacy = false;
    let mut saved = false;
    for child in &mut node.children {
        let at = child.element.source_range().start;
        match child.source_use {
            U::SavedSet if node.source_use == U::Container => {
                saved = true;
                if let Some(key) = number_key(child, "id")
                    && sets.insert(key, at).is_some()
                {
                    note(
                        child,
                        budget,
                        D::DuplicateNumericSetId,
                        at,
                        "finite decimal set IDs are numerically equivalent; both occurrences are retained",
                    )?;
                }
            }
            U::Group if node.source_use == U::Container => legacy = true,
            U::MainStatSetSelection
            | U::CalcsStatSetSelection
            | U::MainMinionLookup
            | U::CalcsMinionLookup
                if node.source_use == U::GemInstance =>
            {
                if let Some(effect) = child.element.attribute("grantedEffect") {
                    let family = match child.source_use {
                        U::MainStatSetSelection => 0,
                        U::CalcsStatSetSelection => 1,
                        U::MainMinionLookup => 2,
                        _ => 3,
                    };
                    let key = (family, effect.decoded().to_owned());
                    if lookups.insert(key, at).is_some() {
                        note(
                            child,
                            budget,
                            D::DuplicateLookupKey,
                            at,
                            "repeated lookup identity is retained; source ordered assignment can replace earlier state",
                        )?;
                    }
                }
            }
            U::MinionIndexMap => {
                if let Some(key) = number_key(child, "skillIndex")
                    && maps.insert(key, at).is_some()
                {
                    note(
                        child,
                        budget,
                        D::DuplicateLookupKey,
                        at,
                        "finite decimal skillIndex keys are numerically equivalent; both map occurrences are retained",
                    )?;
                }
            }
            _ => {}
        }
    }
    if legacy && saved {
        note(
            node,
            budget,
            D::MixedLegacyAndSavedSets,
            node.element.source_range().start,
            "legacy direct groups and explicit sets coexist; source load interpretation has not been applied",
        )?;
    }
    Ok(())
}
fn project_node<'input>(
    raw: Node<'_, 'input>,
    parent: Option<SkillSourceUse>,
    blocked_namespace: bool,
    depth: usize,
    budget: &mut Budget,
) -> Result<SkillSourceNode<'input>, SourceXmlError> {
    use SkillDiagnosticCode as D;
    use SkillSourceUse as U;
    let at = raw.range().start;
    if depth > MAX_SKILL_SOURCE_DEPTH {
        return Err(invalid(at, "skills source depth limit exceeded"));
    }
    consume(
        &mut budget.nodes,
        1,
        at,
        "skills source node count limit exceeded",
    )?;
    consume(
        &mut budget.attributes,
        raw.attributes().len(),
        at,
        "skills source attribute count limit exceeded",
    )?;
    let namespace =
        blocked_namespace || raw.namespaces().len() != 0 || raw.tag_name().namespace().is_some();
    let (kind, usage) = classification(parent, raw.tag_name().name(), namespace);
    match usage {
        U::SavedSet => consume(
            &mut budget.sets,
            1,
            at,
            "saved skill set count limit exceeded",
        )?,
        U::Group => consume(
            &mut budget.groups,
            1,
            at,
            "skill group count limit exceeded",
        )?,
        U::GemInstance => consume(
            &mut budget.instances,
            1,
            at,
            "gem instance count limit exceeded",
        )?,
        _ => {}
    }
    let element = build_source::element(raw, &mut budget.bytes)?;
    let mut node = SkillSourceNode {
        kind,
        source_use: usage,
        element,
        attributes: Vec::new(),
        diagnostics: Vec::new(),
        children: Vec::new(),
    };
    if namespace {
        note(
            &mut node,
            budget,
            D::NamespaceContext,
            at,
            "namespace-bearing context is retained without a recognized source consumer role",
        )?;
    } else if kind == SkillSourceKind::Unknown {
        note(
            &mut node,
            budget,
            D::UnknownElement,
            at,
            "unknown element name or position is retained",
        )?;
    }
    if parent == Some(U::Container) && usage == U::Group {
        note(
            &mut node,
            budget,
            D::LegacyDirectGroup,
            at,
            "direct Skill is a legacy source group; no implicit set has been created by this projection",
        )?;
    }
    if kind == SkillSourceKind::Unknown && usage == U::GemInstance {
        note(
            &mut node,
            budget,
            D::PositionalGemChild,
            at,
            "LoadSkill consumes every direct element child as a gem instance irrespective of its name",
        )?;
    }
    if kind == SkillSourceKind::Unknown && usage == U::MinionIndexMap {
        note(
            &mut node,
            budget,
            D::PositionalMinionMapChild,
            at,
            "LoadSkill consumes every lookup element child as an index map irrespective of its name",
        )?;
    }
    if node.element.has_non_whitespace_text() {
        note(
            &mut node,
            budget,
            D::UnhandledText,
            at,
            "non-whitespace child text is preserved in source; positional source loader loops may reject it",
        )?;
    }
    for index in 0..node.element.attributes().len() {
        let attr = &node.element.attributes()[index];
        let attr_at = attr.value().range().start;
        let (role, expected) = if attr.namespace().is_some() {
            (SkillAttributeRole::Unknown, Expected::Text)
        } else {
            attribute_role(usage, attr.name())
        };
        let value_syntax = syntax(attr.value().decoded(), expected);
        node.attributes.push(SkillSourceAttribute {
            source_index: index,
            role,
            syntax: value_syntax,
        });
        if role == SkillAttributeRole::Unknown {
            note(
                &mut node,
                budget,
                D::UnknownAttribute,
                attr_at,
                "attribute has no reviewed consumer role at this source position; value is retained",
            )?;
        }
        if matches!(
            value_syntax,
            SkillAttributeSyntax::OtherNumericText | SkillAttributeSyntax::NonFiniteDecimal
        ) {
            note(
                &mut node,
                budget,
                D::NumericOutsideFiniteDecimal,
                attr_at,
                "authored value is outside finite decimal syntax; source tonumber/default/error behavior is not resolved",
            )?;
        }
        if value_syntax == SkillAttributeSyntax::OtherBooleanText {
            note(
                &mut node,
                budget,
                D::NonCanonicalBoolean,
                attr_at,
                "authored value is not literal true/false; field-specific source boolean/default behavior is not applied",
            )?;
        }
        if role == SkillAttributeRole::LegacyResetSelection {
            note(
                &mut node,
                budget,
                D::LegacyStatSetAttributeReset,
                attr_at,
                "LoadSkill resets these legacy attribute maps before reading per-effect child selections",
            )?;
        }
    }
    if usage == U::SavedSet && node.element.attribute("id").is_none() {
        note(
            &mut node,
            budget,
            D::MissingSetId,
            at,
            "no authored set ID; source creation/default behavior is not applied",
        )?;
    }
    if usage == U::GemInstance
        && ["gemId", "skillId", "nameSpec"]
            .iter()
            .all(|name| node.element.attribute(name).is_none())
    {
        note(
            &mut node,
            budget,
            D::MissingGemIdentity,
            at,
            "no authored gem ID, effect ID or display name; no identity or default skill is invented",
        )?;
    }
    let missing_lookup_key = match usage {
        U::MainStatSetSelection | U::CalcsStatSetSelection => {
            node.element.attribute("grantedEffect").is_none()
                || node.element.attribute("index").is_none()
        }
        U::MainMinionLookup | U::CalcsMinionLookup => {
            node.element.attribute("grantedEffect").is_none()
        }
        U::MinionIndexMap => {
            node.element.attribute("skillIndex").is_none()
                || node.element.attribute("statSetIndex").is_none()
        }
        _ => false,
    };
    if missing_lookup_key {
        note(
            &mut node,
            budget,
            D::MissingLookupKey,
            at,
            "lookup key/value is absent; source skip, deletion or table-key error behavior is not applied",
        )?;
    }
    for child in raw.children().filter(Node::is_element) {
        node.children.push(project_node(
            child,
            Some(usage),
            namespace,
            depth + 1,
            budget,
        )?);
    }
    duplicate_diagnostics(&mut node, budget)?;
    Ok(node)
}

/// Project every direct root Skills occurrence, including namespace lookalikes.
/// Other root payloads remain in source_xml and are not interpreted here.
pub fn project<'input>(
    document: &Document<'input>,
) -> Result<SkillProjection<'input>, SourceXmlError> {
    let xml = document.input_text();
    if xml.len() > crate::MAX_XML_BYTES
        || document.descendants().count() > crate::MAX_XML_NODES as usize
    {
        return Err(invalid(0, "build XML exceeds skills projection limits"));
    }
    crate::source_xml::validate_ordered_xml(xml)?;
    let root = document.root_element();
    if root.tag_name().name() != "PathOfBuilding2" || root.tag_name().namespace().is_some() {
        return Err(invalid(
            root.range().start,
            "expected unnamespaced PathOfBuilding2 root",
        ));
    }
    let count = root
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "Skills")
        .count();
    if count > MAX_SKILLS_CONTAINERS {
        return Err(invalid(
            root.range().start,
            "Skills container count limit exceeded",
        ));
    }
    let mut budget = Budget {
        bytes: build_source::MAX_PROJECTED_ATTRIBUTE_BYTES,
        nodes: MAX_SKILL_SOURCE_NODES,
        attributes: MAX_SKILL_SOURCE_ATTRIBUTES,
        diagnostics: MAX_SKILL_SOURCE_DIAGNOSTICS,
        sets: MAX_SKILL_SETS,
        groups: MAX_SKILL_GROUPS,
        instances: MAX_GEM_INSTANCES,
    };
    let mut containers = Vec::with_capacity(count);
    for raw in root
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "Skills")
    {
        let mut node = project_node(raw, None, root.namespaces().len() != 0, 1, &mut budget)?;
        if !containers.is_empty() {
            note(
                &mut node,
                &mut budget,
                SkillDiagnosticCode::DuplicateSkillsContainer,
                raw.range().start,
                "multiple authored Skills containers are retained; sequential source loading is not applied",
            )?;
        }
        containers.push(node);
    }
    Ok(SkillProjection {
        source_sha256: format!("{:x}", Sha256::digest(xml.as_bytes())),
        source_xml: xml,
        containers,
    })
}
/// Bounded caller XML entry point; no source-load defaults or data resolution.
pub fn project_xml(xml: &str) -> Result<SkillProjection<'_>, SourceXmlError> {
    if xml.len() > crate::MAX_XML_BYTES {
        return Err(invalid(0, "build XML exceeds skills projection limits"));
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
