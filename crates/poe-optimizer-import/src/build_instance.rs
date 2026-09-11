//! Owned source snapshots and stable identities for authored build occurrences.
//!
//! This stage preserves all saved alternatives and unknown source. It does not
//! choose active sets, resolve item references, synthesize defaults or calculate.
use crate::{
    ImportError, ImportFormat, ImportedBuild, build_source, configuration, item_source,
    skill_source,
    source_xml::{SourceText, SourceXmlError},
};
use item_source::{ItemSourceKind, ItemSourceUse};
use poe_optimizer_core::build_identity::{
    BuildLineage, BuildRevision, ConfigSetId, InstanceAllocator, InstanceAllocatorState,
    InstanceId, ItemRecordId, ItemSetId, ItemSlotUseId, PassiveSpecId, SkillEntryId, SkillGroupId,
    SkillSetId,
};
use serde::{Serialize, Serializer, ser::SerializeStruct};
use sha2::{Digest, Sha256};
use skill_source::{SkillSourceKind, SkillSourceUse};
use std::{collections::BTreeMap, ops::Range, sync::Arc};
use thiserror::Error;

pub const INSTANCE_IMPORT_SCHEMA: u32 = 1;

/// Additional allocation bounds, independent of game rules and projection limits.
#[derive(Debug, Clone, Copy)]
pub struct InstanceImportLimits {
    pub max_occurrences: usize,
    pub max_instances: usize,
    /// Root has depth zero.
    pub max_depth: usize,
    pub max_attributes: usize,
    /// Total copied element/attribute names and namespace URI bytes.
    pub max_metadata_bytes: usize,
}
impl Default for InstanceImportLimits {
    fn default() -> Self {
        Self {
            max_occurrences: 100_000,
            max_instances: 32_768,
            max_depth: 128,
            max_attributes: 200_000,
            max_metadata_bytes: 4 * 1024 * 1024,
        }
    }
}
#[derive(Debug, Error)]
pub enum InstanceImportError {
    #[error(transparent)]
    Import(#[from] ImportError),
    #[error("decoded source hash does not match the exact XML bytes")]
    SourceIdentityMismatch,
    #[error("build instance import exceeds {0} limit")]
    ResourceLimit(&'static str),
    #[error("cannot allocate build instance identity: {0}")]
    Identity(String),
    #[error("source occurrence does not belong to this source snapshot")]
    ForeignOccurrence,
    #[error("instance does not belong to this imported build")]
    ForeignInstance,
    #[error(transparent)]
    Source(#[from] SourceXmlError),
}

/// Content-addressed location, not a candidate identity or admission token.
/// Identical XML shares source locations; hosts assign distinct import lineages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceOccurrenceId {
    snapshot: [u8; 32],
    ordinal: u32,
}
impl SourceOccurrenceId {
    pub fn ordinal(self) -> u32 {
        self.ordinal
    }
    pub fn source_sha256(self) -> String {
        hex_digest(&self.snapshot)
    }
}
impl Serialize for SourceOccurrenceId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("SourceOccurrenceId", 2)?;
        state.serialize_field("source_sha256", &self.source_sha256())?;
        state.serialize_field("ordinal", &self.ordinal)?;
        state.end()
    }
}

/// Source-consumer roles are not proof that the original loader succeeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "domain", rename_all = "snake_case")]
pub enum SourceRole {
    Root,
    Opaque,
    Skill {
        kind: SkillSourceKind,
        usage: SkillSourceUse,
    },
    Item {
        kind: ItemSourceKind,
        usage: ItemSourceUse,
    },
    ConfigContainer,
    ConfigSet,
    ConfigRecord,
}
#[derive(Debug, Clone, Serialize)]
pub struct AttributeOrigin {
    pub name: String,
    pub namespace: Option<String>,
    pub value_range: Range<usize>,
}
#[derive(Debug, Clone, Serialize)]
pub struct SourceOccurrence {
    id: SourceOccurrenceId,
    parent_ordinal: Option<u32>,
    source_range: Range<usize>,
    name: String,
    namespace: Option<String>,
    namespace_context: bool,
    role: SourceRole,
    attributes: Vec<AttributeOrigin>,
}
impl SourceOccurrence {
    pub fn id(&self) -> SourceOccurrenceId {
        self.id
    }
    pub fn parent(&self) -> Option<SourceOccurrenceId> {
        self.parent_ordinal
            .map(|ordinal| SourceOccurrenceId { ordinal, ..self.id })
    }
    pub fn range(&self) -> Range<usize> {
        self.source_range.clone()
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }
    pub fn has_namespace_context(&self) -> bool {
        self.namespace_context
    }
    pub fn role(&self) -> SourceRole {
        self.role
    }
    pub fn attributes(&self) -> &[AttributeOrigin] {
        &self.attributes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum AuthoredInstanceId {
    SkillSet(SkillSetId),
    SkillGroup(SkillGroupId),
    SkillEntry(SkillEntryId),
    ItemSet(ItemSetId),
    ItemRecord(ItemRecordId),
    ItemSlotUse(ItemSlotUseId),
    PassiveSpec(PassiveSpecId),
    ConfigSet(ConfigSetId),
}
impl AuthoredInstanceId {
    pub fn instance_id(self) -> InstanceId {
        match self {
            Self::SkillSet(id) => id.instance_id(),
            Self::SkillGroup(id) => id.instance_id(),
            Self::SkillEntry(id) => id.instance_id(),
            Self::ItemSet(id) => id.instance_id(),
            Self::ItemRecord(id) => id.instance_id(),
            Self::ItemSlotUse(id) => id.instance_id(),
            Self::PassiveSpec(id) => id.instance_id(),
            Self::ConfigSet(id) => id.instance_id(),
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct InstanceBinding {
    instance: AuthoredInstanceId,
    source: SourceOccurrenceId,
}
impl InstanceBinding {
    pub fn instance(&self) -> AuthoredInstanceId {
        self.instance
    }
    pub fn source(&self) -> SourceOccurrenceId {
        self.source
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionKind {
    Root,
    Skills,
    Items,
    Configuration,
}
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ProjectionState {
    Available {
        projection: ProjectionKind,
    },
    Unavailable {
        projection: ProjectionKind,
        error: SourceXmlError,
    },
}
fn projection_state<T>(
    projection: ProjectionKind,
    result: Result<T, SourceXmlError>,
) -> ProjectionState {
    match result {
        Ok(_) => ProjectionState::Available { projection },
        Err(error) => ProjectionState::Unavailable { projection, error },
    }
}
#[derive(Debug)]
struct ImportedState {
    source: Arc<str>,
    format: ImportFormat,
    source_sha256: String,
    digest: [u8; 32],
    lineage: BuildLineage,
    revision: BuildRevision,
    allocator: InstanceAllocatorState,
    occurrences: Vec<SourceOccurrence>,
    instances: Vec<InstanceBinding>,
    membership: BTreeMap<AuthoredInstanceId, usize>,
    projections: Vec<ProjectionState>,
}
/// Cloning shares one immutable snapshot and preserves all identities.
/// Copying an editable instance later needs a fresh ID and explicit origin.
#[derive(Debug, Clone)]
pub struct ImportedBuildInstance {
    state: Arc<ImportedState>,
}
#[derive(Debug, Serialize)]
pub struct ImportedInstanceReport<'a> {
    pub schema_version: u32,
    pub lineage: BuildLineage,
    pub revision: BuildRevision,
    pub source_sha256: &'a str,
    pub source_bytes: usize,
    pub format: ImportFormat,
    pub occurrences: &'a [SourceOccurrence],
    pub instances: &'a [InstanceBinding],
    pub projections: &'a [ProjectionState],
}
impl ImportedBuildInstance {
    pub fn from_decoded(
        decoded: ImportedBuild,
        lineage: BuildLineage,
        limits: InstanceImportLimits,
    ) -> Result<Self, InstanceImportError> {
        // ImportedBuild is a public mutable DTO. Recheck bounds, hash and XML.
        if decoded.xml.len() > crate::MAX_XML_BYTES {
            return Err(ImportError::XmlTooLarge.into());
        }
        let digest: [u8; 32] = Sha256::digest(decoded.xml.as_bytes()).into();
        let source_sha256 = hex_digest(&digest);
        if decoded.sha256 != source_sha256 {
            return Err(InstanceImportError::SourceIdentityMismatch);
        }
        let source: Arc<str> = decoded.xml.into();
        let document = crate::parse_document(&source)?;
        let mut occurrences = Vec::new();
        let mut instances = Vec::new();
        let mut membership = BTreeMap::new();
        let mut allocator = InstanceAllocator::new(lineage);
        let mut metadata_left = limits.max_metadata_bytes;
        let mut attributes_left = limits.max_attributes;
        // Preorder keeps source order, including duplicate external IDs. The stack
        // and output combined are bounded before another pending node is pushed.
        let mut stack = vec![(
            document.root_element(),
            None,
            SourceRole::Opaque,
            false,
            0usize,
        )];
        while let Some((node, parent, parent_role, inherited_namespace, depth)) = stack.pop() {
            if occurrences.len() >= limits.max_occurrences {
                return Err(InstanceImportError::ResourceLimit("occurrences"));
            }
            if depth > limits.max_depth {
                return Err(InstanceImportError::ResourceLimit("depth"));
            }
            let ordinal = u32::try_from(occurrences.len())
                .map_err(|_| InstanceImportError::ResourceLimit("occurrence identity"))?;
            let id = SourceOccurrenceId {
                snapshot: digest,
                ordinal,
            };
            let namespace_context = inherited_namespace
                || node.namespaces().len() != 0
                || node.tag_name().namespace().is_some();
            let role = if parent.is_none() {
                SourceRole::Root
            } else {
                classify_role(parent_role, node.tag_name().name(), namespace_context)
            };
            if has_instance(role) {
                if instances.len() >= limits.max_instances {
                    return Err(InstanceImportError::ResourceLimit("instances"));
                }
                let instance = allocate_role(role, &mut allocator)?;
                membership.insert(instance, instances.len());
                instances.push(InstanceBinding {
                    instance,
                    source: id,
                });
            }
            let name = copy_metadata(node.tag_name().name(), &mut metadata_left)?;
            let namespace = node
                .tag_name()
                .namespace()
                .map(|v| copy_metadata(v, &mut metadata_left))
                .transpose()?;
            let mut attributes = Vec::new();
            for attribute in node.attributes() {
                attributes_left = attributes_left
                    .checked_sub(1)
                    .ok_or(InstanceImportError::ResourceLimit("attributes"))?;
                attributes.push(AttributeOrigin {
                    name: copy_metadata(attribute.name(), &mut metadata_left)?,
                    namespace: attribute
                        .namespace()
                        .map(|v| copy_metadata(v, &mut metadata_left))
                        .transpose()?,
                    value_range: attribute.range_value(),
                });
            }
            occurrences.push(SourceOccurrence {
                id,
                parent_ordinal: parent,
                source_range: node.range(),
                name,
                namespace,
                namespace_context,
                role,
                attributes,
            });
            for child in node.children().rev().filter(roxmltree::Node::is_element) {
                if stack.len() >= limits.max_occurrences - occurrences.len() {
                    return Err(InstanceImportError::ResourceLimit("occurrences"));
                }
                stack.push((child, Some(ordinal), role, namespace_context, depth + 1));
            }
        }
        // Existing typed projections have different lexical/structural limits.
        // Their failures remain evidence, never a reason to erase authored rows.
        let projections = vec![
            projection_state(ProjectionKind::Root, build_source::project(&document)),
            projection_state(ProjectionKind::Skills, skill_source::project(&document)),
            projection_state(ProjectionKind::Items, item_source::project(&document)),
            projection_state(
                ProjectionKind::Configuration,
                configuration::project(&document),
            ),
        ];
        drop(document);
        Ok(Self {
            state: Arc::new(ImportedState {
                source,
                format: decoded.format,
                source_sha256,
                digest,
                lineage,
                revision: BuildRevision::INITIAL,
                allocator: allocator.state(),
                occurrences,
                instances,
                membership,
                projections,
            }),
        })
    }
    pub fn source_xml(&self) -> &str {
        &self.state.source
    }
    pub fn source_sha256(&self) -> &str {
        &self.state.source_sha256
    }
    pub fn lineage(&self) -> BuildLineage {
        self.state.lineage
    }
    pub fn revision(&self) -> BuildRevision {
        self.state.revision
    }
    /// Resume watermark only; allocation still requires coordinated ownership.
    pub fn allocator_state(&self) -> &InstanceAllocatorState {
        &self.state.allocator
    }
    pub fn occurrences(&self) -> &[SourceOccurrence] {
        &self.state.occurrences
    }
    pub fn instances(&self) -> &[InstanceBinding] {
        &self.state.instances
    }
    pub fn projections(&self) -> &[ProjectionState] {
        &self.state.projections
    }
    pub fn shares_storage_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.state, &other.state)
    }
    pub fn occurrence(
        &self,
        id: SourceOccurrenceId,
    ) -> Result<&SourceOccurrence, InstanceImportError> {
        if id.snapshot != self.state.digest {
            return Err(InstanceImportError::ForeignOccurrence);
        }
        self.state
            .occurrences
            .get(id.ordinal as usize)
            .ok_or(InstanceImportError::ForeignOccurrence)
    }
    pub fn binding(&self, id: AuthoredInstanceId) -> Result<&InstanceBinding, InstanceImportError> {
        self.state
            .membership
            .get(&id)
            .map(|&index| &self.state.instances[index])
            .ok_or(InstanceImportError::ForeignInstance)
    }
    pub fn source_fragment(&self, id: SourceOccurrenceId) -> Result<&str, InstanceImportError> {
        Ok(&self.state.source[self.occurrence(id)?.range()])
    }
    pub fn attribute(
        &self,
        id: SourceOccurrenceId,
        name: &str,
    ) -> Result<Option<SourceText<'_>>, InstanceImportError> {
        self.occurrence(id)?
            .attributes
            .iter()
            .find(|a| a.name == name && a.namespace.is_none())
            .map(|a| {
                SourceText::from_range(self.source_xml(), a.value_range.clone(), false)
                    .map_err(InstanceImportError::Source)
            })
            .transpose()
    }
    pub fn project_skills(&self) -> Result<skill_source::SkillProjection<'_>, SourceXmlError> {
        skill_source::project_xml(self.source_xml())
    }
    pub fn project_items(&self) -> Result<item_source::ItemProjection<'_>, SourceXmlError> {
        item_source::project_xml(self.source_xml())
    }
    pub fn project_configuration(
        &self,
    ) -> Result<configuration::ConfigurationProjection<'_>, SourceXmlError> {
        configuration::project_xml(self.source_xml())
    }
    pub fn report(&self) -> ImportedInstanceReport<'_> {
        ImportedInstanceReport {
            schema_version: INSTANCE_IMPORT_SCHEMA,
            lineage: self.lineage(),
            revision: self.revision(),
            source_sha256: self.source_sha256(),
            source_bytes: self.source_xml().len(),
            format: self.state.format,
            occurrences: self.occurrences(),
            instances: self.instances(),
            projections: self.projections(),
        }
    }
}
fn classify_role(parent: SourceRole, name: &str, namespace: bool) -> SourceRole {
    match parent {
        SourceRole::Root if name == "Skills" => {
            let (kind, usage) = skill_source::classification(None, name, namespace);
            SourceRole::Skill { kind, usage }
        }
        SourceRole::Skill { usage, .. } => {
            let (kind, usage) = skill_source::classification(Some(usage), name, namespace);
            SourceRole::Skill { kind, usage }
        }
        SourceRole::Root if matches!(name, "Items" | "Tree" | "Spec") => {
            let (kind, usage) = item_source::classify(None, name, namespace);
            SourceRole::Item { kind, usage }
        }
        SourceRole::Item { usage, .. } => {
            let (kind, usage) = item_source::classify(Some(usage), name, namespace);
            SourceRole::Item { kind, usage }
        }
        SourceRole::Root if name == "Config" && !namespace => SourceRole::ConfigContainer,
        SourceRole::ConfigContainer if name == "ConfigSet" && !namespace => SourceRole::ConfigSet,
        SourceRole::ConfigContainer | SourceRole::ConfigSet if !namespace => {
            SourceRole::ConfigRecord
        }
        _ => SourceRole::Opaque,
    }
}
fn has_instance(role: SourceRole) -> bool {
    matches!(
        role,
        SourceRole::Skill {
            usage: SkillSourceUse::SavedSet | SkillSourceUse::Group | SkillSourceUse::GemInstance,
            ..
        } | SourceRole::Item {
            usage: ItemSourceUse::SavedSet
                | ItemSourceUse::InventoryItem
                | ItemSourceUse::EquipmentSlot
                | ItemSourceUse::LegacyEquipmentSlot
                | ItemSourceUse::CharacterRuneSlot
                | ItemSourceUse::JewelAssignment
                | ItemSourceUse::PassiveSpec,
            ..
        } | SourceRole::ConfigSet
    )
}
fn allocate_role(
    role: SourceRole,
    allocator: &mut InstanceAllocator,
) -> Result<AuthoredInstanceId, InstanceImportError> {
    use AuthoredInstanceId as I;
    let result = match role {
        SourceRole::Skill {
            usage: SkillSourceUse::SavedSet,
            ..
        } => allocator.allocate().map(I::SkillSet),
        SourceRole::Skill {
            usage: SkillSourceUse::Group,
            ..
        } => allocator.allocate().map(I::SkillGroup),
        SourceRole::Skill {
            usage: SkillSourceUse::GemInstance,
            ..
        } => allocator.allocate().map(I::SkillEntry),
        SourceRole::Item {
            usage: ItemSourceUse::SavedSet,
            ..
        } => allocator.allocate().map(I::ItemSet),
        SourceRole::Item {
            usage: ItemSourceUse::InventoryItem,
            ..
        } => allocator.allocate().map(I::ItemRecord),
        SourceRole::Item {
            usage:
                ItemSourceUse::EquipmentSlot
                | ItemSourceUse::LegacyEquipmentSlot
                | ItemSourceUse::CharacterRuneSlot
                | ItemSourceUse::JewelAssignment,
            ..
        } => allocator.allocate().map(I::ItemSlotUse),
        SourceRole::Item {
            usage: ItemSourceUse::PassiveSpec,
            ..
        } => allocator.allocate().map(I::PassiveSpec),
        SourceRole::ConfigSet => allocator.allocate().map(I::ConfigSet),
        _ => unreachable!("only authored instance roles reach allocation"),
    };
    result.map_err(|error| InstanceImportError::Identity(error.to_string()))
}
fn copy_metadata(value: &str, remaining: &mut usize) -> Result<String, InstanceImportError> {
    *remaining = remaining
        .checked_sub(value.len())
        .ok_or(InstanceImportError::ResourceLimit("metadata bytes"))?;
    Ok(value.to_owned())
}
fn hex_digest(digest: &[u8; 32]) -> String {
    use std::fmt::Write;
    let mut value = String::with_capacity(64);
    for byte in digest {
        write!(value, "{byte:02x}").expect("writing to a String");
    }
    value
}
