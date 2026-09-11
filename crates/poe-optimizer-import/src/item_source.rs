//! Authored item and jewel-assignment source, before ItemsTab/Item loading.
//!
//! Inventory occurrences, independent saved sets and passive-spec jewel ownership
//! retain their original order. XML-consumed strings are separate from raw lexical
//! fragments: every string can trigger a separate Item.ParseRaw reset downstream.
//! This module does not run that method, choose equipment, resolve identifiers,
//! apply ModRange instructions, insert defaults or establish native capability.
use crate::{
    build_source::{self, SourceElement},
    source_xml::{self, SourceContent, SourceXmlError, invalid},
};
use roxmltree::{Document, Node, ParsingOptions};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const MAX_ITEM_SOURCE_CONTAINERS: usize = 128;
pub const MAX_ITEM_SOURCE_NODES: usize = 32_768;
pub const MAX_ITEM_SOURCE_DEPTH: usize = 32;
pub const MAX_ITEM_SOURCE_ATTRIBUTES: usize = 65_536;
pub const MAX_ITEM_SOURCE_FRAGMENTS: usize = 131_072;
pub const MAX_ITEM_SOURCE_TEXT_BYTES: usize = crate::MAX_XML_BYTES;
pub const MAX_ITEM_SOURCE_DIAGNOSTICS: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemSourceKind {
    Items,
    Item,
    ItemSet,
    Slot,
    RuneSlot,
    SocketIdUrl,
    ModRange,
    TradeSearchWeights,
    Stat,
    Tree,
    Spec,
    Sockets,
    Socket,
    Unknown,
}
/// A syntactic consumer role, not proof that loading the element succeeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemSourceUse {
    Container,
    InventoryItem,
    SavedSet,
    EquipmentSlot,
    LegacyEquipmentSlot,
    CharacterRuneSlot,
    SocketUrlMetadata,
    ModifierRangeInstruction,
    TradeWeights,
    TradeWeight,
    Tree,
    PassiveSpec,
    JewelSockets,
    JewelAssignment,
    Ignored,
    NamespaceUnknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemAttributeRole {
    ItemIdentity,
    SetIdentity,
    ActiveSetRequest,
    DisplayText,
    DisplayPreference,
    VariantSelection,
    WeaponSetRequest,
    SlotIdentity,
    SelectedItemReference,
    ActivationRequest,
    ExternalReference,
    RuneSelection,
    PassiveNodeReference,
    RangeIndex,
    RangeValue,
    TradeStatIdentity,
    TradeWeight,
    PassiveSpecState,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemAttributeSyntax {
    Text,
    FiniteDecimal,
    OtherNumericText,
    CanonicalBoolean,
    OtherBooleanText,
}
#[derive(Debug, Clone, Serialize)]
pub struct ItemSourceAttribute {
    source_index: usize,
    role: ItemAttributeRole,
    syntax: ItemAttributeSyntax,
}
impl ItemSourceAttribute {
    pub fn source_index(&self) -> usize {
        self.source_index
    }
    pub fn role(&self) -> ItemAttributeRole {
        self.role
    }
    pub fn syntax(&self) -> ItemAttributeSyntax {
        self.syntax
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemDiagnosticCode {
    UnknownElement,
    NamespaceContext,
    UnknownAttribute,
    OtherNumericText,
    NonCanonicalBoolean,
    MissingSourceIdentity,
    DuplicateSourceIdentity,
    DuplicateContainer,
    LegacyDirectSlot,
    LegacyRootSpec,
    UnhandledText,
}
#[derive(Debug, Clone, Serialize)]
pub struct ItemSourceDiagnostic {
    code: ItemDiagnosticCode,
    byte_offset: usize,
    message: &'static str,
}
impl ItemSourceDiagnostic {
    pub fn code(&self) -> ItemDiagnosticCode {
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
pub struct ItemSourceNode<'input> {
    kind: ItemSourceKind,
    source_use: ItemSourceUse,
    element: SourceElement<'input>,
    attributes: Vec<ItemSourceAttribute>,
    diagnostics: Vec<ItemSourceDiagnostic>,
    ordered_content: SourceContent<'input>,
    children: Vec<ItemSourceNode<'input>>,
}
impl<'input> ItemSourceNode<'input> {
    pub fn kind(&self) -> ItemSourceKind {
        self.kind
    }
    pub fn source_use(&self) -> ItemSourceUse {
        self.source_use
    }
    pub fn element(&self) -> &SourceElement<'input> {
        &self.element
    }
    pub fn attributes(&self) -> &[ItemSourceAttribute] {
        &self.attributes
    }
    pub fn diagnostics(&self) -> &[ItemSourceDiagnostic] {
        &self.diagnostics
    }
    pub fn ordered_content(&self) -> &SourceContent<'input> {
        &self.ordered_content
    }
    pub fn children(&self) -> &[ItemSourceNode<'input>] {
        &self.children
    }
}
/// Exact caller-source identity and occurrence trees. Not deserializable as an
/// admission token or future candidate identity. Empty collections imply no
/// matching root occurrences; they do not imply effective empty equipment.
#[derive(Debug, Clone, Serialize)]
pub struct ItemProjection<'input> {
    source_sha256: String,
    #[serde(skip)]
    source_xml: &'input str,
    containers: Vec<ItemSourceNode<'input>>,
    trees: Vec<ItemSourceNode<'input>>,
}
impl<'input> ItemProjection<'input> {
    pub fn source_sha256(&self) -> &str {
        &self.source_sha256
    }
    pub fn source_xml(&self) -> &'input str {
        self.source_xml
    }
    pub fn containers(&self) -> &[ItemSourceNode<'input>] {
        &self.containers
    }
    /// Original Tree and legacy root Spec occurrences, preserving their owners.
    pub fn trees(&self) -> &[ItemSourceNode<'input>] {
        &self.trees
    }
}
struct Budget {
    nodes: usize,
    attributes: usize,
    bytes: usize,
    fragments: usize,
    text: usize,
    diagnostics: usize,
}
fn take(
    value: &mut usize,
    amount: usize,
    at: usize,
    reason: &'static str,
) -> Result<(), SourceXmlError> {
    *value = value
        .checked_sub(amount)
        .ok_or_else(|| invalid(at, reason))?;
    Ok(())
}
fn note(
    node: &mut ItemSourceNode<'_>,
    budget: &mut Budget,
    code: ItemDiagnosticCode,
    at: usize,
    message: &'static str,
) -> Result<(), SourceXmlError> {
    take(
        &mut budget.diagnostics,
        1,
        at,
        "item source diagnostic limit exceeded",
    )?;
    node.diagnostics.push(ItemSourceDiagnostic {
        code,
        byte_offset: at,
        message,
    });
    Ok(())
}
pub(crate) fn classify(
    parent: Option<ItemSourceUse>,
    name: &str,
    namespace: bool,
) -> (ItemSourceKind, ItemSourceUse) {
    use ItemSourceKind as K;
    use ItemSourceUse as U;
    if namespace {
        return (K::Unknown, U::NamespaceUnknown);
    }
    let kind = match name {
        "Items" => K::Items,
        "Item" => K::Item,
        "ItemSet" => K::ItemSet,
        "Slot" => K::Slot,
        "RuneSlot" => K::RuneSlot,
        "SocketIdURL" => K::SocketIdUrl,
        "ModRange" => K::ModRange,
        "TradeSearchWeights" => K::TradeSearchWeights,
        "Stat" => K::Stat,
        "Tree" => K::Tree,
        "Spec" => K::Spec,
        "Sockets" => K::Sockets,
        "Socket" => K::Socket,
        _ => K::Unknown,
    };
    let usage = match (parent, name) {
        (None, "Items") => U::Container,
        (None, "Tree") => U::Tree,
        (None, "Spec") => U::PassiveSpec,
        (Some(U::Container), "Item") => U::InventoryItem,
        (Some(U::Container), "ItemSet") => U::SavedSet,
        (Some(U::Container), "Slot") => U::LegacyEquipmentSlot,
        (Some(U::Container), "TradeSearchWeights") => U::TradeWeights,
        (Some(U::SavedSet), "Slot") => U::EquipmentSlot,
        (Some(U::SavedSet), "RuneSlot") => U::CharacterRuneSlot,
        (Some(U::SavedSet), "SocketIdURL") => U::SocketUrlMetadata,
        (Some(U::InventoryItem), "ModRange") => U::ModifierRangeInstruction,
        (Some(U::TradeWeights), _) => U::TradeWeight,
        (Some(U::Tree), "Spec") => U::PassiveSpec,
        (Some(U::PassiveSpec), "Sockets") => U::JewelSockets,
        (Some(U::JewelSockets), "Socket") => U::JewelAssignment,
        _ => U::Ignored,
    };
    (kind, usage)
}
fn attribute_role(usage: ItemSourceUse, name: &str) -> (ItemAttributeRole, bool, bool) {
    use ItemAttributeRole as A;
    use ItemSourceUse as U;
    let (role, number, boolean) = match (usage, name) {
        (U::Container, "activeItemSet") => (A::ActiveSetRequest, true, false),
        (U::Container | U::SavedSet, "useSecondWeaponSet") => (A::WeaponSetRequest, false, true),
        (U::Container, "showStatDifferences") => (A::DisplayPreference, false, true),
        (U::InventoryItem, "id") => (A::ItemIdentity, true, false),
        (
            U::InventoryItem,
            "variant" | "variantAlt" | "variantAlt2" | "variantAlt3" | "variantAlt4"
            | "variantAlt5",
        ) => (A::VariantSelection, true, false),
        (U::SavedSet, "id") => (A::SetIdentity, true, false),
        (U::SavedSet, "title") => (A::DisplayText, false, false),
        (U::EquipmentSlot | U::LegacyEquipmentSlot, "name") => (A::SlotIdentity, false, false),
        (U::EquipmentSlot | U::LegacyEquipmentSlot, "itemId") => {
            (A::SelectedItemReference, true, false)
        }
        (U::EquipmentSlot | U::LegacyEquipmentSlot, "active") => {
            (A::ActivationRequest, false, true)
        }
        (U::EquipmentSlot | U::SocketUrlMetadata, "itemPbURL") => {
            (A::ExternalReference, false, false)
        }
        (U::EquipmentSlot, "note") => (A::DisplayText, false, false),
        (U::CharacterRuneSlot, "slotName") => (A::SlotIdentity, false, false),
        (U::CharacterRuneSlot, "runeName") => (A::RuneSelection, false, false),
        (U::SocketUrlMetadata | U::JewelAssignment, "nodeId") => {
            (A::PassiveNodeReference, true, false)
        }
        (U::JewelAssignment, "itemId") => (A::SelectedItemReference, true, false),
        (U::ModifierRangeInstruction, "id") => (A::RangeIndex, true, false),
        (U::ModifierRangeInstruction, "range") => (A::RangeValue, true, false),
        (U::TradeWeight, "label") => (A::DisplayText, false, false),
        (U::TradeWeight, "stat") => (A::TradeStatIdentity, false, false),
        (U::TradeWeight, "weightMult") => (A::TradeWeight, true, false),
        (U::Tree, "activeSpec") => (A::PassiveSpecState, true, false),
        (
            U::PassiveSpec,
            "title" | "treeVersion" | "nodes" | "masteryEffects" | "ascendancyInternalId",
        ) => (A::PassiveSpecState, false, false),
        (
            U::PassiveSpec,
            "classId" | "ascendClassId" | "classInternalId" | "secondaryAscendClassId",
        ) => (A::PassiveSpecState, true, false),
        _ => (A::Unknown, false, false),
    };
    (role, number, boolean)
}
fn identity_field(usage: ItemSourceUse) -> Option<&'static str> {
    use ItemSourceUse as U;
    match usage {
        U::InventoryItem | U::SavedSet => Some("id"),
        U::EquipmentSlot | U::LegacyEquipmentSlot => Some("name"),
        U::CharacterRuneSlot => Some("slotName"),
        U::JewelAssignment | U::SocketUrlMetadata => Some("nodeId"),
        _ => None,
    }
}
fn numeric_key(text: &str) -> Option<String> {
    let n = text.trim_ascii().parse::<f64>().ok()?;
    n.is_finite().then(|| {
        if n == 0.0 {
            "0".to_owned()
        } else {
            n.to_string()
        }
    })
}
fn node<'input>(
    raw: Node<'_, 'input>,
    parent: Option<ItemSourceUse>,
    inherited_namespace: bool,
    depth: usize,
    budget: &mut Budget,
) -> Result<ItemSourceNode<'input>, SourceXmlError> {
    let at = raw.range().start;
    if depth > MAX_ITEM_SOURCE_DEPTH {
        return Err(invalid(at, "item source depth limit exceeded"));
    }
    take(&mut budget.nodes, 1, at, "item source node limit exceeded")?;
    take(
        &mut budget.attributes,
        raw.attributes().len(),
        at,
        "item source attribute limit exceeded",
    )?;
    let element = build_source::element(raw, &mut budget.bytes)?;
    let namespace =
        inherited_namespace || element.has_namespaces() || element.namespace().is_some();
    let (kind, source_use) = classify(parent, element.name(), namespace);
    let ordered_content =
        source_xml::ordered_content(raw, &mut budget.fragments, &mut budget.text)?;
    let mut output = ItemSourceNode {
        kind,
        source_use,
        element,
        attributes: Vec::new(),
        diagnostics: Vec::new(),
        ordered_content,
        children: Vec::new(),
    };
    if namespace {
        note(
            &mut output,
            budget,
            ItemDiagnosticCode::NamespaceContext,
            at,
            "namespace context remains unclassified",
        )?;
    } else if source_use == ItemSourceUse::Ignored || kind == ItemSourceKind::Unknown {
        note(
            &mut output,
            budget,
            ItemDiagnosticCode::UnknownElement,
            at,
            "unhandled element remains source evidence",
        )?;
    }
    if source_use == ItemSourceUse::LegacyEquipmentSlot {
        note(
            &mut output,
            budget,
            ItemDiagnosticCode::LegacyDirectSlot,
            at,
            "legacy direct Slot is retained without applying set defaults",
        )?;
    }
    if parent.is_none() && source_use == ItemSourceUse::PassiveSpec {
        note(
            &mut output,
            budget,
            ItemDiagnosticCode::LegacyRootSpec,
            at,
            "legacy root Spec retains its own jewel-assignment ownership",
        )?;
    }
    for index in 0..output.element.attributes().len() {
        let a = &output.element.attributes()[index];
        let offset = a.value().range().start;
        let (role, numeric, boolean) = if namespace || a.namespace().is_some() {
            (ItemAttributeRole::Unknown, false, false)
        } else {
            attribute_role(source_use, a.name())
        };
        let syntax = if numeric {
            if numeric_key(a.value().decoded()).is_some() {
                ItemAttributeSyntax::FiniteDecimal
            } else {
                ItemAttributeSyntax::OtherNumericText
            }
        } else if boolean {
            if matches!(a.value().decoded(), "true" | "false") {
                ItemAttributeSyntax::CanonicalBoolean
            } else {
                ItemAttributeSyntax::OtherBooleanText
            }
        } else {
            ItemAttributeSyntax::Text
        };
        output.attributes.push(ItemSourceAttribute {
            source_index: index,
            role,
            syntax,
        });
        if role == ItemAttributeRole::Unknown {
            note(
                &mut output,
                budget,
                ItemDiagnosticCode::UnknownAttribute,
                offset,
                "unhandled attribute remains source evidence",
            )?;
        }
        if syntax == ItemAttributeSyntax::OtherNumericText {
            note(
                &mut output,
                budget,
                ItemDiagnosticCode::OtherNumericText,
                offset,
                "numeric source text is not replaced with a loader default",
            )?;
        }
        if syntax == ItemAttributeSyntax::OtherBooleanText {
            note(
                &mut output,
                budget,
                ItemDiagnosticCode::NonCanonicalBoolean,
                offset,
                "noncanonical boolean text remains authored",
            )?;
        }
    }
    if let Some(field) = identity_field(source_use)
        && output.element.attribute(field).is_none()
    {
        note(
            &mut output,
            budget,
            ItemDiagnosticCode::MissingSourceIdentity,
            at,
            "missing authored identity; no default or generated identity was inserted",
        )?;
    }
    if source_use != ItemSourceUse::InventoryItem
        && output
            .ordered_content
            .consumed()
            .iter()
            .any(|e| matches!(e, source_xml::PobContentEntry::Text { .. }))
    {
        note(
            &mut output,
            budget,
            ItemDiagnosticCode::UnhandledText,
            at,
            "XML-consumed text is retained without running the section loader",
        )?;
    }
    let mut seen = BTreeSet::new();
    for child in raw.children().filter(Node::is_element) {
        let mut child = node(child, Some(source_use), namespace, depth + 1, budget)?;
        if let Some(field) = identity_field(child.source_use)
            && let Some(value) = child.element.attribute(field)
        {
            let key = if matches!(field, "id" | "nodeId") {
                numeric_key(value.decoded()).unwrap_or_else(|| value.decoded().to_owned())
            } else {
                value.decoded().to_owned()
            };
            if !seen.insert((child.source_use as u8, key)) {
                let at = child.element.source_range().start;
                note(
                    &mut child,
                    budget,
                    ItemDiagnosticCode::DuplicateSourceIdentity,
                    at,
                    "duplicate authored identity remains a separate ordered occurrence",
                )?;
            }
        }
        output.children.push(child);
    }
    Ok(output)
}
pub fn project<'input>(
    document: &Document<'input>,
) -> Result<ItemProjection<'input>, SourceXmlError> {
    let xml = document.input_text();
    if xml.len() > crate::MAX_XML_BYTES
        || document.descendants().count() > crate::MAX_XML_NODES as usize
    {
        return Err(invalid(0, "build XML exceeds projection limits"));
    }
    source_xml::validate_ordered_xml_with_namespaces(xml)?;
    let root = document.root_element();
    if root.tag_name().name() != "PathOfBuilding2" || root.tag_name().namespace().is_some() {
        return Err(invalid(
            root.range().start,
            "expected unnamespaced PathOfBuilding2 root",
        ));
    }
    let mut budget = Budget {
        nodes: MAX_ITEM_SOURCE_NODES,
        attributes: MAX_ITEM_SOURCE_ATTRIBUTES,
        bytes: build_source::MAX_PROJECTED_ATTRIBUTE_BYTES,
        fragments: MAX_ITEM_SOURCE_FRAGMENTS,
        text: MAX_ITEM_SOURCE_TEXT_BYTES,
        diagnostics: MAX_ITEM_SOURCE_DIAGNOSTICS,
    };
    let mut output = ItemProjection {
        source_sha256: format!("{:x}", Sha256::digest(xml.as_bytes())),
        source_xml: xml,
        containers: Vec::new(),
        trees: Vec::new(),
    };
    let mut containers_left = MAX_ITEM_SOURCE_CONTAINERS;
    let mut seen = BTreeSet::new();
    for raw in root
        .children()
        .filter(Node::is_element)
        .filter(|n| matches!(n.tag_name().name(), "Items" | "Tree" | "Spec"))
    {
        take(
            &mut containers_left,
            1,
            raw.range().start,
            "item source container limit exceeded",
        )?;
        let mut value = node(raw, None, root.namespaces().len() != 0, 0, &mut budget)?;
        if !seen.insert(raw.tag_name().name()) {
            note(
                &mut value,
                &mut budget,
                ItemDiagnosticCode::DuplicateContainer,
                raw.range().start,
                "duplicate root occurrence remains ordered source evidence",
            )?;
        }
        if raw.tag_name().name() == "Items" {
            output.containers.push(value);
        } else {
            output.trees.push(value);
        }
    }
    Ok(output)
}
pub fn project_xml(xml: &str) -> Result<ItemProjection<'_>, SourceXmlError> {
    if xml.len() > crate::MAX_XML_BYTES {
        return Err(invalid(0, "build XML exceeds projection byte limit"));
    }
    let document = Document::parse_with_options(
        xml,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: crate::MAX_XML_NODES,
            ..ParsingOptions::default()
        },
    )
    .map_err(|e| invalid(0, format!("invalid source XML: {e}")))?;
    project(&document)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_xml::{PobContentEntry, PobTextKind, SourceContentKind};
    fn text_entries<'a>(node: &'a ItemSourceNode<'_>) -> Vec<(&'a str, PobTextKind)> {
        node.ordered_content()
            .consumed()
            .iter()
            .filter_map(|entry| match entry {
                PobContentEntry::Text {
                    text, text_kind, ..
                } => Some((text.as_str(), *text_kind)),
                _ => None,
            })
            .collect()
    }
    fn walk<'a, 'input>(
        nodes: &'a [ItemSourceNode<'input>],
        out: &mut Vec<&'a ItemSourceNode<'input>>,
    ) {
        for node in nodes {
            out.push(node);
            walk(node.children(), out);
        }
    }
    fn count(nodes: &[&ItemSourceNode<'_>], usage: ItemSourceUse) -> usize {
        nodes.iter().filter(|n| n.source_use() == usage).count()
    }
    #[test]
    fn mixed_item_payload_preserves_raw_fragments_and_each_ordered_load_instruction() {
        let xml = "<PathOfBuilding2><Items><Item id='1'> \r\nab<!-- ignored -->cd&amp; \t<![CDATA[ raw &amp;\r\n]]><ModRange id='1' range='0.5'/> tail </Item></Items></PathOfBuilding2>";
        let p = project_xml(xml).unwrap();
        let item = &p.containers()[0].children()[0];
        assert_eq!(
            text_entries(item),
            vec![
                ("abcd&", PobTextKind::Ordinary),
                (" raw &amp;\r\n", PobTextKind::Cdata),
                ("tail", PobTextKind::Ordinary)
            ]
        );
        assert_eq!(
            item.children()[0].source_use(),
            ItemSourceUse::ModifierRangeInstruction
        );
        let entries = item.ordered_content().consumed();
        assert!(matches!(
            entries[2],
            PobContentEntry::Element { child_index: 0 }
        ));
        assert!(
            matches!(&entries[0],PobContentEntry::Text{fragment_indices,..} if fragment_indices==&[0,1,2])
        );
        let fragments = item.ordered_content().fragments();
        assert_eq!(
            fragments.iter().map(|f| f.kind()).collect::<Vec<_>>(),
            vec![
                SourceContentKind::Text,
                SourceContentKind::Comment,
                SourceContentKind::Text,
                SourceContentKind::Cdata,
                SourceContentKind::Element,
                SourceContentKind::Text
            ]
        );
        for fragment in fragments {
            assert_eq!(&xml[fragment.range()], fragment.raw());
        }
        assert_eq!(fragments[4].child_index(), Some(0));
        assert_eq!(p.source_xml(), xml);
        assert_eq!(
            p.source_sha256(),
            format!("{:x}", Sha256::digest(xml.as_bytes()))
        );
    }
    #[test]
    fn comments_inside_cdata_are_removed_but_literal_entities_and_whitespace_are_kept() {
        let xml = "<PathOfBuilding2><Items><Item id='1'><![CDATA[  &amp; a<!--deleted-->b\r\n ]]><![CDATA[ \r\n\t ]]><?split one?> x <?split two?> y </Item></Items></PathOfBuilding2>";
        let p = project_xml(xml).unwrap();
        let item = &p.containers()[0].children()[0];
        assert_eq!(
            text_entries(item),
            vec![
                ("  &amp; ab\r\n ", PobTextKind::Cdata),
                ("x", PobTextKind::Ordinary),
                ("y", PobTextKind::Ordinary)
            ]
        );
        assert!(
            item.ordered_content().fragments()[0]
                .raw()
                .contains("<!--deleted-->")
        );
        assert_eq!(
            item.ordered_content()
                .fragments()
                .iter()
                .filter(|f| f.kind() == SourceContentKind::ProcessingInstruction)
                .count(),
            2
        );
    }
    #[test]
    fn duplicates_missing_ids_and_unselected_sets_are_retained_without_defaults() {
        let xml = "<PathOfBuilding2><Items activeItemSet='999'><Item id='1'>first</Item><Item id='1.0'>second</Item><ItemSet id='2' useSecondWeaponSet='nil'><Slot name='Weapon 1' itemId='1'/><Slot name='Weapon 1' itemId='2'/><RuneSlot slotName='Rune 1' runeName='None'/><SocketIdURL nodeId='9' itemPbURL='link'/></ItemSet><ItemSet id='2'/><ItemSet/><Slot name='Helmet' itemId='9'/></Items><Items/></PathOfBuilding2>";
        let p = project_xml(xml).unwrap();
        let c = &p.containers()[0];
        assert_eq!(p.containers().len(), 2);
        assert_eq!(c.children().len(), 6);
        assert_eq!(
            c.element().attribute("activeItemSet").unwrap().decoded(),
            "999"
        );
        assert!(
            c.children()[1]
                .diagnostics()
                .iter()
                .any(|d| d.code() == ItemDiagnosticCode::DuplicateSourceIdentity)
        );
        assert!(
            c.children()[3]
                .diagnostics()
                .iter()
                .any(|d| d.code() == ItemDiagnosticCode::DuplicateSourceIdentity)
        );
        assert!(c.children()[4].element().attribute("id").is_none());
        assert!(
            c.children()[4]
                .diagnostics()
                .iter()
                .any(|d| d.code() == ItemDiagnosticCode::MissingSourceIdentity)
        );
        let saved = &c.children()[2];
        assert_eq!(
            saved
                .element()
                .attribute("useSecondWeaponSet")
                .unwrap()
                .decoded(),
            "nil"
        );
        assert!(
            saved
                .diagnostics()
                .iter()
                .any(|d| d.code() == ItemDiagnosticCode::NonCanonicalBoolean)
        );
        assert_eq!(
            saved.children()[2].source_use(),
            ItemSourceUse::CharacterRuneSlot
        );
        assert_eq!(
            saved.children()[3].source_use(),
            ItemSourceUse::SocketUrlMetadata
        );
        assert_eq!(
            c.children()[5].source_use(),
            ItemSourceUse::LegacyEquipmentSlot
        );
    }
    #[test]
    fn actual_jewels_keep_spec_ownership_separate_from_set_socket_urls_and_nested_lookalikes() {
        let xml = "<PathOfBuilding2><Items><ItemSet id='1'><SocketIdURL nodeId='7' itemPbURL='x'/><Future><Socket nodeId='7' itemId='99'/></Future></ItemSet></Items><Tree activeSpec='9'><Spec title='first'><Sockets><Socket nodeId='7' itemId='3'/><Socket nodeId='7' itemId='4'/></Sockets></Spec><Spec title='second'><Sockets><Socket nodeId='7' itemId='5'/></Sockets></Spec></Tree><Spec title='legacy'><Sockets><Socket nodeId='7' itemId='6'/></Sockets></Spec></PathOfBuilding2>";
        let p = project_xml(xml).unwrap();
        assert_eq!(p.trees().len(), 2);
        assert_eq!(p.trees()[0].source_use(), ItemSourceUse::Tree);
        assert_eq!(p.trees()[1].source_use(), ItemSourceUse::PassiveSpec);
        let mut nodes = Vec::new();
        walk(p.containers(), &mut nodes);
        walk(p.trees(), &mut nodes);
        assert_eq!(count(&nodes, ItemSourceUse::JewelAssignment), 4);
        assert_eq!(count(&nodes, ItemSourceUse::SocketUrlMetadata), 1);
        let actual = p.trees()[0].children()[0].children()[0].children();
        assert!(
            actual[1]
                .diagnostics()
                .iter()
                .any(|d| d.code() == ItemDiagnosticCode::DuplicateSourceIdentity)
        );
        assert_eq!(
            p.containers()[0].children()[0].children()[1].children()[0].source_use(),
            ItemSourceUse::Ignored
        );
    }
    #[test]
    fn namespace_context_never_promotes_plain_or_reset_descendants_into_known_roles() {
        let xml = "<PathOfBuilding2><Items xmlns='urn:foreign'><Item xmlns='' id='1'>x</Item></Items><x:Items xmlns:x='urn:foreign'><x:Item id='2'/></x:Items><Items><Item id='3'/></Items></PathOfBuilding2>";
        let p = project_xml(xml).unwrap();
        assert_eq!(p.containers().len(), 3);
        for n in &p.containers()[..2] {
            assert_eq!(n.source_use(), ItemSourceUse::NamespaceUnknown);
            assert_eq!(
                n.children()[0].source_use(),
                ItemSourceUse::NamespaceUnknown
            );
        }
        assert_eq!(
            p.containers()[2].children()[0].source_use(),
            ItemSourceUse::InventoryItem
        );
    }
    #[test]
    fn builtin_xml_element_prefixes_never_become_item_consumers() {
        for xml in [
            "<PathOfBuilding2><xml:Items><Item id='1'/></xml:Items></PathOfBuilding2>",
            "<PathOfBuilding2><xml:Items xmlns:xml='http://www.w3.org/XML/1998/namespace'><Item id='1'/></xml:Items></PathOfBuilding2>",
            "<PathOfBuilding2><Items><xml:Item id='2'/></Items></PathOfBuilding2>",
        ] {
            assert!(project_xml(xml).is_err());
        }
    }

    #[test]
    fn authored_attribute_bytes_use_five_named_entities_without_xml_whitespace_normalization() {
        let xml = "<PathOfBuilding2><Items><ItemSet id='1' title='caf\u{e9}\r\n\t&lt;&gt;&amp;&quot;&apos;&amp;lt;'><RuneSlot slotName='Rune 1' runeName='None'/><Slot name='Ring 1' itemPbURL='a\r\nb' note='c'/></ItemSet><TradeSearchWeights><FutureStat label='d' stat='e' weightMult='nonnumeric'/></TradeSearchWeights></Items></PathOfBuilding2>";
        let p = project_xml(xml).unwrap();
        let set = &p.containers()[0].children()[0];
        let title = set.element().attribute("title").unwrap();
        assert_eq!(title.decoded(), "caf\u{e9}\r\n\t<>&\"'&lt;");
        assert_eq!(&xml[title.range()], title.raw());
        assert_eq!(
            set.children()[0].attributes()[0].role(),
            ItemAttributeRole::SlotIdentity
        );
        assert_eq!(
            set.children()[1].attributes()[1].role(),
            ItemAttributeRole::ExternalReference
        );
        let weight = &p.containers()[0].children()[1].children()[0];
        assert_eq!(weight.kind(), ItemSourceKind::Unknown);
        assert_eq!(weight.source_use(), ItemSourceUse::TradeWeight);
        assert!(
            weight
                .diagnostics()
                .iter()
                .any(|d| d.code() == ItemDiagnosticCode::OtherNumericText)
        );
    }
    #[test]
    fn all_five_immutable_documents_preserve_every_inventory_set_range_and_jewel_occurrence() {
        let documents = [
            include_str!("../../../tests/fixtures/builds/breadth-20260908/build-01.xml"),
            include_str!("../../../tests/fixtures/builds/breadth-20260908/build-02.xml"),
            include_str!("../../../tests/fixtures/builds/breadth-20260908/build-03.xml"),
            include_str!("../../../tests/fixtures/builds/breadth-20260908/build-04.xml"),
            include_str!("../../../tests/fixtures/builds/breadth-20260908/build-05.xml"),
        ];
        let hashes = [
            "e3c0d0b40fa682260a1713acb03d52d720f4b769ac91b0501cbe2a84dc468194",
            "91366bd82a9afdd12ae7d8f695508a1b8d99116567010e082a9d31c4c4d4f631",
            "d3f7c72092f77481d3d1c5e38ec71d8730d607f19c659fc05b8a5db3bbbf9490",
            "62d760d326e21291cd1024f20660bd5046043c4e242b761df5d9a764be61e711",
            "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089",
        ];
        let expected = [
            (16, 1, 74),
            (34, 6, 133),
            (17, 1, 89),
            (21, 1, 89),
            (28, 6, 101),
        ];
        let mut jewels = 0;
        for ((xml, hash), (items, sets, ranges)) in documents.into_iter().zip(hashes).zip(expected)
        {
            let p = project_xml(xml).unwrap();
            assert_eq!(p.source_sha256(), hash);
            let mut nodes = Vec::new();
            walk(p.containers(), &mut nodes);
            walk(p.trees(), &mut nodes);
            assert_eq!(count(&nodes, ItemSourceUse::InventoryItem), items);
            assert_eq!(count(&nodes, ItemSourceUse::SavedSet), sets);
            assert_eq!(
                count(&nodes, ItemSourceUse::ModifierRangeInstruction),
                ranges
            );
            jewels += count(&nodes, ItemSourceUse::JewelAssignment);
            for n in nodes {
                assert_eq!(&xml[n.element().source_range()], n.element().source_xml());
                for fragment in n.ordered_content().fragments() {
                    assert_eq!(&xml[fragment.range()], fragment.raw());
                }
            }
        }
        assert_eq!(jewels, 21);
    }
    #[test]
    fn lexical_and_xml_ambiguity_fails_without_returning_partial_evidence() {
        for xml in [
            "<!DOCTYPE PathOfBuilding2><PathOfBuilding2/>",
            "<PathOfBuilding2><Items><Item id='1' id='2'/></Items></PathOfBuilding2>",
            "<PathOfBuilding2><Items><Item id = '1'/></Items></PathOfBuilding2>",
            "<PathOfBuilding2><Items><Item id='1'>x&#10;y</Item></Items></PathOfBuilding2>",
            "<PathOfBuilding2><Items><Item id='1'><![CDATA[a<!-- ]]><Future> -->x</Future></Item></Items></PathOfBuilding2>",
            "<PathOfBuilding2><Items><Item id='1'><![CDATA[a]<!-- c -->]>b]]></Item></Items></PathOfBuilding2>",
            "<PathOfBuilding2><Items><Item></Items></PathOfBuilding2>",
            "<Other><Items/></Other>",
        ] {
            assert!(project_xml(xml).is_err(), "accepted {xml}");
        }
    }
    #[test]
    fn bounded_unknown_descendants_and_attributes_cannot_expand_without_limit() {
        let xml = format!(
            "<PathOfBuilding2><Items>{}{}</Items></PathOfBuilding2>",
            "<Future>".repeat(MAX_ITEM_SOURCE_DEPTH + 1),
            "</Future>".repeat(MAX_ITEM_SOURCE_DEPTH + 1)
        );
        assert!(project_xml(&xml).unwrap_err().reason.contains("depth"));
        let attrs = (0..129).map(|i| format!(" a{i}='x'")).collect::<String>();
        assert!(
            project_xml(&format!(
                "<PathOfBuilding2><Items><Future{attrs}/></Items></PathOfBuilding2>"
            ))
            .unwrap_err()
            .reason
            .contains("attribute")
        );
        let many = format!(
            "<PathOfBuilding2><Items>{}</Items></PathOfBuilding2>",
            "<Future/>".repeat(MAX_ITEM_SOURCE_NODES)
        );
        assert!(
            project_xml(&many)
                .unwrap_err()
                .reason
                .contains("node limit")
        );
        assert!(
            project_xml(&" ".repeat(crate::MAX_XML_BYTES + 1))
                .unwrap_err()
                .reason
                .contains("byte limit")
        );
    }
    #[test]
    fn serialized_evidence_does_not_repeat_descendant_xml_per_ancestor() {
        let payload = "x".repeat(200_000);
        let xml = format!(
            "<PathOfBuilding2><Items>{}<Item id='1'>{payload}</Item>{}</Items></PathOfBuilding2>",
            "<Future>".repeat(20),
            "</Future>".repeat(20)
        );
        let p = project_xml(&xml).unwrap();
        let report = serde_json::to_string(&p).unwrap();
        assert!(
            report.len() < xml.len() * 3,
            "{} bytes for {} source bytes",
            report.len(),
            xml.len()
        );
        assert_eq!(report.matches(&payload).count(), 2); // raw lexical text plus consumed text
        assert!(!report.contains("\"source_xml\""));
    }
    #[test]
    fn source_projections_allow_mixed_items_while_native_and_own_config_text_stay_strict() {
        let xml = "<PathOfBuilding2><Items><Item id='1'>a<![CDATA[b]]>c</Item></Items><Skills><SkillSet id='1'><Skill><Gem gemId='caller'/></Skill></SkillSet></Skills><Config><ConfigSet id='1'><Input name='caller' string='value'/></ConfigSet></Config></PathOfBuilding2>";
        crate::build_source::project_xml(xml).unwrap();
        crate::skill_source::project_xml(xml).unwrap();
        crate::configuration::project_xml(xml).unwrap();
        assert!(crate::xml_compat::validate(xml).is_err());
        assert!(
            crate::xml_compat::validate_native_with_configuration(&Document::parse(xml).unwrap())
                .is_err()
        );
        let bad = "<PathOfBuilding2><Config><ConfigSet id='1'><CustomModifierBlock title='x' enabled='true'>a<![CDATA[b]]></CustomModifierBlock></ConfigSet></Config></PathOfBuilding2>";
        assert!(crate::configuration::project_xml(bad).is_err());
    }
}
