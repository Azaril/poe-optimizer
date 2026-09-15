//! Offline owned-ID allocation and exact external identity mappings.
//!
//! External strings and source pins stay in import tooling. This module performs
//! no source parsing, file I/O, game conversion, source evaluation or input binding.
//! Mapping a definition identity does not establish schema/rule coverage. Hosts
//! persist registry edits with compare-and-swap against the previous digest.

mod codec;
mod registry;
mod snapshot;

pub use codec::{decode_mapping_package, decode_registry, encode_mapping_package, encode_registry};
pub use registry::OwnedIdRegistry;
pub use snapshot::OwnedMappingIndex;

use poe_optimizer_core::{
    data::DataIdentity,
    owned_content::{ContentDigestError, OwnedContentDigest},
    owned_definitions::*,
    owned_schema::{DefinitionAddress, SchemaSubject, SlotAddress, SocketKind},
};
use serde::{Deserialize, Serialize};

pub const OWNED_ID_REGISTRY_VERSION: u32 = 1;
pub const OWNED_MAPPING_PACKAGE_VERSION: u32 = 1;
const REGISTRY_DOMAIN: &str = "owned-id-registry-v1";
const MAPPING_DOMAIN: &str = "owned-external-mapping-v1";
const SOURCE_PIN_DOMAIN: &str = "owned-mapping-source-pin-v1";

/// Per-artifact bounds. Mapping entries include rows, manifest files, ambiguous
/// candidates and inspected complete output-topology members; registry entries are
/// allocations including tombstones. String bounds
/// cover external selector/pin text; owned symbols retain Core's independent bounds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OwnedMappingLimits {
    pub max_entries: usize,
    pub max_collection_entries: usize,
    pub max_candidates: usize,
    pub max_string_bytes: usize,
    pub max_total_string_bytes: usize,
    pub max_wire_bytes: usize,
}
impl Default for OwnedMappingLimits {
    fn default() -> Self {
        Self {
            max_entries: 1_000_000,
            max_collection_entries: 100_000,
            max_candidates: 64,
            max_string_bytes: 16 * 1024,
            max_total_string_bytes: 16 * 1024 * 1024,
            max_wire_bytes: 16 * 1024 * 1024,
        }
    }
}
impl OwnedMappingLimits {
    pub(crate) fn validate(self) -> Result<()> {
        for (name, value, ceiling) in [
            ("max_entries", self.max_entries, 4_000_000),
            (
                "max_collection_entries",
                self.max_collection_entries,
                1_000_000,
            ),
            ("max_candidates", self.max_candidates, 4096),
            ("max_string_bytes", self.max_string_bytes, 64 * 1024),
            (
                "max_total_string_bytes",
                self.max_total_string_bytes,
                64 * 1024 * 1024,
            ),
            ("max_wire_bytes", self.max_wire_bytes, 64 * 1024 * 1024),
        ] {
            if value == 0 || value > ceiling {
                return invalid(name, OwnedMappingErrorKind::InvalidLimit);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OwnedMappingErrorKind {
    InvalidLimit,
    LimitExceeded,
    InvalidCounter,
    CounterOverflow,
    RevisionOverflow,
    HistoryGap,
    WrongAllocatedKey,
    DuplicateTarget,
    UnknownRegistryTarget,
    RetiredTarget,
    AlreadyRetired,
    ActiveDependentSlots,
    MissingRegistryOwner,
    ForeignNamespace,
    SuccessorConflict,
    DuplicateSelector,
    DuplicateCandidate,
    TooFewCandidates,
    UnreviewedAlias,
    WrongTargetKind,
    WrongTargetOwner,
    WrongSocketKind,
    WrongOutputTopology,
    MissingSchemaTarget,
    InconsistentSchemaIndex,
    RegistryBindingMismatch,
    SchemaBindingMismatch,
    SourceBindingMismatch,
    PolicyBindingMismatch,
    InvalidSourcePin,
    InvalidDigest,
}

#[derive(Debug, thiserror::Error)]
pub enum OwnedMappingError {
    #[error("{path}: {kind:?}")]
    Invalid {
        path: String,
        kind: OwnedMappingErrorKind,
    },
    #[error("unsupported {artifact} version {version}")]
    UnsupportedVersion {
        artifact: &'static str,
        version: u32,
    },
    #[error("owned mapping artifact exceeds {maximum} bytes")]
    TooLarge { maximum: usize },
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
}
type Result<T> = std::result::Result<T, OwnedMappingError>;
fn invalid<T>(path: &str, kind: OwnedMappingErrorKind) -> Result<T> {
    Err(OwnedMappingError::Invalid {
        path: path.into(),
        kind,
    })
}

/// Snapshot DTO. History is established by validate_successor, not deserialization.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryInput {
    pub schema_version: u32,
    pub namespace: GameVersionNamespace,
    pub revision: BoundedInteger,
    pub last_issued: BoundedInteger,
    pub entries: Vec<RegistryEntry>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryEntry {
    pub sequence: BoundedInteger,
    pub target: SchemaSubject,
    pub state: RegistryState,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RegistryState {
    Active,
    Retired { reason: OwnedDefinitionKey },
}

/// Exact lexical source evidence. Missing differs from a present empty string.
/// The artifact constructor/codec bounds text; it never normalizes it.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum SourceComponent {
    Missing,
    Text(String),
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ExternalOwnerSelector {
    Class {
        key: SourceComponent,
    },
    Ascendancy {
        class: SourceComponent,
        key: SourceComponent,
    },
    Reward {
        key: SourceComponent,
    },
    ItemTemplate {
        base: SourceComponent,
        prototype: SourceComponent,
        variant: SourceComponent,
    },
    Modifier {
        catalog: SourceComponent,
        key: SourceComponent,
        variant: SourceComponent,
    },
    Gem {
        game_id: SourceComponent,
        variant_id: SourceComponent,
    },
    Skill {
        effect_id: SourceComponent,
    },
    PassiveNode {
        tree_version: SourceComponent,
        node_id: SourceComponent,
        view: SourceComponent,
    },
    UsagePolicy {
        key: SourceComponent,
        version: SourceComponent,
    },
}

macro_rules! mapping_enum {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($variant),+ }
    };
}
mapping_enum!(ExternalCatalogKind {
    PointPool,
    EquipmentSlot,
    Encounter,
    Metric,
    Option,
    SkillLinkRole,
    Unit,
    Quality,
    ExternalInput,
    Stat,
    Capability,
});
mapping_enum!(ExternalSlotKind {
    Parameter,
    Choice,
    Grant,
    Actor,
    SkillGrant,
    ActionOutput
});
mapping_enum!(ActionAlternativeKind {
    Part,
    Mode,
    StatSet
});
mapping_enum!(ConfigSourceRole {
    Input,
    Placeholder,
    Default
});
mapping_enum!(ConfigMappingRole {
    Parameter,
    Choice,
    Reward,
    Option,
    ExternalInput,
    UsagePolicy,
    ActionOutput,
    ActionPart,
    ActionMode,
    ActionStatSet,
});
// These are source-system identities, not claims of implemented game conversion.
mapping_enum!(ExternalSourceSystem {
    PathOfBuilding1,
    PathOfBuilding2
});

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ExternalSelector {
    Definition(ExternalOwnerSelector),
    Catalog {
        kind: ExternalCatalogKind,
        key: SourceComponent,
        version: SourceComponent,
        variant: SourceComponent,
    },
    Socket {
        owner: ExternalOwnerSelector,
        kind: SocketKind,
        key: SourceComponent,
    },
    Slot {
        owner: ExternalOwnerSelector,
        kind: ExternalSlotKind,
        key: SourceComponent,
    },
    ActionAlternative {
        owner: ExternalOwnerSelector,
        output: SourceComponent,
        key: SourceComponent,
        kind: ActionAlternativeKind,
    },
    Configuration {
        key: SourceComponent,
        source: ConfigSourceRole,
        role: ConfigMappingRole,
        value: SourceComponent,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourcePin {
    pub system: ExternalSourceSystem,
    pub revision: String,
    pub files: Vec<SourceFilePin>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFilePin {
    pub path: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum MappingBasis {
    Exact,
    /// A review claim whose explanation stays in separate tooling evidence.
    ReviewedAlias {
        reason: OwnedDefinitionKey,
    },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum MappingOutcome {
    Mapped {
        target: SchemaSubject,
        basis: MappingBasis,
    },
    Ambiguous {
        candidates: Vec<SchemaSubject>,
        issue: OwnedDefinitionKey,
    },
    Unmapped {
        issue: OwnedDefinitionKey,
    },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MappingEntry {
    pub source: ExternalSelector,
    pub outcome: MappingOutcome,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MappingPackageInput {
    pub schema_version: u32,
    pub namespace: GameVersionNamespace,
    pub registry: OwnedContentDigest,
    pub definitions: DataIdentity,
    pub source: SourcePin,
    pub policy_version: OwnedDefinitionKey,
    pub entries: Vec<MappingEntry>,
}

/// Ordering adapter only. Public targets reuse Core's SchemaSubject unchanged.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum TargetKey {
    Definition(DefinitionAddress),
    Slot(SlotAddress),
}
impl From<&SchemaSubject> for TargetKey {
    fn from(target: &SchemaSubject) -> Self {
        match target {
            SchemaSubject::Definition(id) => Self::Definition(id.clone()),
            SchemaSubject::Slot(id) => Self::Slot(id.clone()),
        }
    }
}
fn owner_address(owner: &SlotOwnerDefId) -> DefinitionAddress {
    match owner {
        SlotOwnerDefId::Class(id) => DefinitionAddress::Class(id.clone()),
        SlotOwnerDefId::Ascendancy(id) => DefinitionAddress::Ascendancy(id.clone()),
        SlotOwnerDefId::Reward(id) => DefinitionAddress::Reward(id.clone()),
        SlotOwnerDefId::ItemTemplate(id) => DefinitionAddress::ItemTemplate(id.clone()),
        SlotOwnerDefId::Modifier(id) => DefinitionAddress::Modifier(id.clone()),
        SlotOwnerDefId::Gem(id) => DefinitionAddress::Gem(id.clone()),
        SlotOwnerDefId::Skill(id) => DefinitionAddress::Skill(id.clone()),
        SlotOwnerDefId::PassiveNode(id) => DefinitionAddress::PassiveNode(id.clone()),
        SlotOwnerDefId::UsagePolicy(id) => DefinitionAddress::UsagePolicy(id.clone()),
    }
}
fn target_symbol(target: &SchemaSubject) -> &OwnedDefinitionKey {
    match target {
        SchemaSubject::Definition(id) => id.key(),
        SchemaSubject::Slot(id) => id.key(),
    }
}
fn target_kind(target: &SchemaSubject) -> DefinitionKind {
    match target {
        SchemaSubject::Definition(id) => id.kind(),
        SchemaSubject::Slot(id) => id.kind(),
    }
}
fn check_target_namespace(target: &SchemaSubject, namespace: &GameVersionNamespace) -> Result<()> {
    let valid = match target {
        SchemaSubject::Definition(id) => id.namespace() == namespace,
        SchemaSubject::Slot(id) => {
            id.namespace() == namespace && id.declaration().namespace() == namespace
        }
    };
    if !valid {
        return invalid("target.namespace", OwnedMappingErrorKind::ForeignNamespace);
    }
    Ok(())
}
