//! Bounded, source-independent storage for owned definition input schemas.
//!
//! This package establishes input declarations and potential links. It contains no
//! numerical rules, source programs, evaluator callbacks or proof of activation.

use poe_optimizer_core::{
    data::DataIdentity, owned_build::DeclaredSlot, owned_definitions::*, owned_schema::*,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    io,
};

pub const OWNED_SCHEMA_PACKAGE_VERSION: u32 = 2;
pub const DEFAULT_SCHEMA_MAX_ENTRIES: usize = 1_000_000;
pub const DEFAULT_SCHEMA_MAX_COLLECTION_ENTRIES: usize = 100_000;
pub const DEFAULT_SCHEMA_MAX_WIRE_BYTES: usize = 64 * 1024 * 1024;
pub const HARD_SCHEMA_MAX_ENTRIES: usize = 4_000_000;
pub const HARD_SCHEMA_MAX_COLLECTION_ENTRIES: usize = 1_000_000;
pub const HARD_SCHEMA_MAX_WIRE_BYTES: usize = 256 * 1024 * 1024;

/// The entire versioned artifact. These are input DTOs, not validated storage.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaPackageInput {
    pub schema_version: u32,
    pub namespace: GameVersionNamespace,
    pub release: OwnedDefinitionKey,
    pub semantics_version: OwnedDefinitionKey,
    pub definitions: Vec<DefinitionDescriptor>,
    pub slots: Vec<SlotDescriptor>,
}

/// Resource limits, independent of game legality. Callers may adjust defaults up
/// to the named hard caps; actual full-game package measurements remain separate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OwnedSchemaLimits {
    pub max_entries: usize,
    pub max_collection_entries: usize,
    pub max_wire_bytes: usize,
}
impl Default for OwnedSchemaLimits {
    fn default() -> Self {
        Self {
            max_entries: DEFAULT_SCHEMA_MAX_ENTRIES,
            max_collection_entries: DEFAULT_SCHEMA_MAX_COLLECTION_ENTRIES,
            max_wire_bytes: DEFAULT_SCHEMA_MAX_WIRE_BYTES,
        }
    }
}
impl OwnedSchemaLimits {
    fn validate(self) -> Result {
        for (name, value, maximum) in [
            ("max_entries", self.max_entries, HARD_SCHEMA_MAX_ENTRIES),
            (
                "max_collection_entries",
                self.max_collection_entries,
                HARD_SCHEMA_MAX_COLLECTION_ENTRIES,
            ),
            (
                "max_wire_bytes",
                self.max_wire_bytes,
                HARD_SCHEMA_MAX_WIRE_BYTES,
            ),
        ] {
            if value == 0 || value > maximum {
                return invalid(name, SchemaPackageErrorKind::InvalidLimit);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SchemaPackageErrorKind {
    InvalidLimit,
    LimitExceeded,
    DuplicateAddress,
    DuplicateMember,
    ForeignNamespace,
    MissingDefinition,
    MissingSlot,
    WrongDeclaration,
    UndeclaredSlot,
    WrongParameterSite,
    WrongSocketOwner,
    ReversedRange,
    UnitMismatch,
    EmptyGapEvidence,
    ImplicitPassiveHasPools,
}

#[derive(Debug, thiserror::Error)]
pub enum SchemaPackageError {
    #[error("{path}: {kind:?}")]
    Invalid {
        path: String,
        kind: SchemaPackageErrorKind,
    },
    #[error("unsupported owned schema package version {0}")]
    UnsupportedVersion(u32),
    #[error("owned schema package exceeds {maximum} bytes")]
    TooLarge { maximum: usize },
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
fn invalid<T>(path: &str, kind: SchemaPackageErrorKind) -> Result<T> {
    Err(SchemaPackageError::Invalid {
        path: path.into(),
        kind,
    })
}
type Result<T = ()> = std::result::Result<T, SchemaPackageError>;

#[derive(Clone, Copy, Debug)]
struct ResourceUse {
    entries: usize,
    largest_collection: usize,
}

/// Immutable descriptors indexed by typed addresses. Deserialization must go
/// through the bounded loader; ordinary input DTOs do not certify membership.
#[derive(Clone, Debug)]
pub struct OwnedDefinitionSchemaPackage {
    input: SchemaPackageInput,
    identity: DataIdentity,
    definitions: BTreeMap<DefinitionAddress, usize>,
    slots: BTreeMap<SlotAddress, usize>,
    canonical_bytes: Vec<u8>,
    resources: ResourceUse,
}
impl OwnedDefinitionSchemaPackage {
    pub fn new(mut input: SchemaPackageInput, limits: OwnedSchemaLimits) -> Result<Self> {
        limits.validate()?;
        if input.schema_version != OWNED_SCHEMA_PACKAGE_VERSION {
            return Err(SchemaPackageError::UnsupportedVersion(input.schema_version));
        }
        let resources = validate_and_canonicalize(&mut input, limits)?;
        let canonical_bytes = bounded_json(&input, limits.max_wire_bytes)?;
        let identity = DataIdentity {
            game: input.namespace.game().as_str().into(),
            release: input.release.as_str().into(),
            schema_version: input.schema_version,
            content_sha256: format!("{:x}", Sha256::digest(&canonical_bytes)),
            semantics_version: input.semantics_version.as_str().into(),
        };
        let definitions = input
            .definitions
            .iter()
            .enumerate()
            .map(|(i, d)| (d.address(), i))
            .collect();
        let slots = input
            .slots
            .iter()
            .enumerate()
            .map(|(i, s)| (s.address(), i))
            .collect();
        Ok(Self {
            input,
            identity,
            definitions,
            slots,
            canonical_bytes,
            resources,
        })
    }
    pub fn input(&self) -> &SchemaPackageInput {
        &self.input
    }
    pub fn identity(&self) -> &DataIdentity {
        &self.identity
    }
}
impl DefinitionSchemaIndex for OwnedDefinitionSchemaPackage {
    fn identity(&self) -> &DataIdentity {
        &self.identity
    }
    fn namespace(&self) -> &GameVersionNamespace {
        &self.input.namespace
    }
    fn lookup_definition(&self, address: &DefinitionAddress) -> Option<&DefinitionDescriptor> {
        self.definitions
            .get(address)
            .map(|i| &self.input.definitions[*i])
    }
    fn lookup_slot(&self, address: &SlotAddress) -> Option<&SlotDescriptor> {
        self.slots.get(address).map(|i| &self.input.slots[*i])
    }
}

pub fn decode_schema_package(
    bytes: &[u8],
    limits: OwnedSchemaLimits,
) -> Result<OwnedDefinitionSchemaPackage> {
    limits.validate()?;
    if bytes.len() > limits.max_wire_bytes {
        return Err(SchemaPackageError::TooLarge {
            maximum: limits.max_wire_bytes,
        });
    }
    OwnedDefinitionSchemaPackage::new(serde_json::from_slice(bytes)?, limits)
}
pub fn encode_schema_package(
    package: &OwnedDefinitionSchemaPackage,
    limits: OwnedSchemaLimits,
) -> Result<Vec<u8>> {
    limits.validate()?;
    if package.resources.entries > limits.max_entries
        || package.resources.largest_collection > limits.max_collection_entries
    {
        return invalid("package", SchemaPackageErrorKind::LimitExceeded);
    }
    if package.canonical_bytes.len() > limits.max_wire_bytes {
        return Err(SchemaPackageError::TooLarge {
            maximum: limits.max_wire_bytes,
        });
    }
    Ok(package.canonical_bytes.clone())
}

fn bounded_json(value: &impl Serialize, maximum: usize) -> Result<Vec<u8>> {
    struct Writer {
        bytes: Vec<u8>,
        maximum: usize,
        exceeded: bool,
    }
    impl io::Write for Writer {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if bytes.len() > self.maximum - self.bytes.len() {
                self.exceeded = true;
                return Err(io::Error::other("owned schema byte limit exceeded"));
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut writer = Writer {
        bytes: Vec::new(),
        maximum,
        exceeded: false,
    };
    let result = serde_json::to_writer(&mut writer, value);
    if writer.exceeded {
        return Err(SchemaPackageError::TooLarge { maximum });
    }
    result?;
    Ok(writer.bytes)
}

struct Check<'a> {
    namespace: &'a GameVersionNamespace,
    definitions: BTreeSet<DefinitionAddress>,
    slots: BTreeSet<SlotAddress>,
    limits: OwnedSchemaLimits,
    resources: ResourceUse,
}
impl Check<'_> {
    fn collection(&mut self, path: &str, len: usize) -> Result {
        if len > self.limits.max_collection_entries
            || len > self.limits.max_entries - self.resources.entries
        {
            return invalid(path, SchemaPackageErrorKind::LimitExceeded);
        }
        self.resources.entries += len;
        self.resources.largest_collection = self.resources.largest_collection.max(len);
        Ok(())
    }
    fn namespace(&self, path: &str, namespace: &GameVersionNamespace) -> Result {
        if namespace != self.namespace {
            return invalid(path, SchemaPackageErrorKind::ForeignNamespace);
        }
        Ok(())
    }
    fn definition_address(&self, path: &str, address: &DefinitionAddress) -> Result {
        self.namespace(path, address.namespace())?;
        if !self.definitions.contains(address) {
            return invalid(path, SchemaPackageErrorKind::MissingDefinition);
        }
        Ok(())
    }
    fn definition<I: SchemaDefinitionId>(&self, path: &str, id: &I) -> Result {
        self.definition_address(path, &id.address())
    }
    fn owner(&self, path: &str, owner: &SlotOwnerDefId) -> Result {
        self.definition_address(path, &owner_address(owner))
    }
    fn slot_address(&self, path: &str, address: &SlotAddress) -> Result {
        self.namespace(path, address.namespace())?;
        self.owner(path, address.declaration())?;
        if !self.slots.contains(address) {
            return invalid(path, SchemaPackageErrorKind::MissingSlot);
        }
        Ok(())
    }
    fn slot<S: SchemaSlotId>(&self, path: &str, key: &DeclaredSlot<S>) -> Result {
        self.slot_address(path, &S::address(key))
    }
    fn sorted<T: Ord>(&mut self, path: &str, values: &mut [T]) -> Result {
        self.collection(path, values.len())?;
        values.sort();
        if values.windows(2).any(|w| w[0] == w[1]) {
            return invalid(path, SchemaPackageErrorKind::DuplicateMember);
        }
        Ok(())
    }
    fn gaps(&mut self, path: &str, gaps: &mut [SchemaGap]) -> Result {
        if gaps.is_empty() {
            return invalid(path, SchemaPackageErrorKind::EmptyGapEvidence);
        }
        self.collection(path, gaps.len())?;
        gaps.sort_by(compare_gaps);
        if gaps.windows(2).any(|w| w[0] == w[1]) {
            return invalid(path, SchemaPackageErrorKind::DuplicateMember);
        }
        for (i, gap) in gaps.iter().enumerate() {
            let path = format!("{path}[{i}].subject");
            match &gap.subject {
                SchemaSubject::Definition(id) => self.definition_address(&path, id)?,
                SchemaSubject::Slot(id) => self.slot_address(&path, id)?,
            }
        }
        Ok(())
    }
    fn closure(&mut self, path: &str, closure: &mut SchemaClosure) -> Result {
        match closure {
            SchemaClosure::Complete => Ok(()),
            SchemaClosure::Partial { gaps } => self.gaps(path, gaps),
        }
    }
    fn state<T>(
        &mut self,
        path: &str,
        state: &mut SchemaState<T>,
        known: impl FnOnce(&mut Self, &mut T) -> Result,
    ) -> Result {
        match state {
            SchemaState::Known(value) => known(self, value),
            SchemaState::Unmapped { gaps } => self.gaps(path, gaps),
        }
    }
    fn definitions<I: SchemaDefinitionId + Ord>(
        &mut self,
        path: &str,
        set: &mut DeclaredSet<I>,
    ) -> Result {
        self.sorted(path, &mut set.members)?;
        for (i, id) in set.members.iter().enumerate() {
            self.definition(&format!("{path}[{i}]"), id)?;
        }
        self.closure(path, &mut set.closure)
    }
    fn slots<S: SchemaSlotId + Ord>(
        &mut self,
        path: &str,
        set: &mut DeclaredSet<DeclaredSlot<S>>,
    ) -> Result {
        self.sorted(path, &mut set.members)?;
        for (i, id) in set.members.iter().enumerate() {
            self.slot(&format!("{path}[{i}]"), id)?;
        }
        self.closure(path, &mut set.closure)
    }
    fn direct_slots<S: SchemaSlotId + Ord>(
        &mut self,
        path: &str,
        set: &mut DeclaredSet<DeclaredSlot<S>>,
        owner: &SlotOwnerDefId,
    ) -> Result {
        self.slots(path, set)?;
        if set.members.iter().any(|slot| &slot.declaration != owner) {
            return invalid(path, SchemaPackageErrorKind::WrongDeclaration);
        }
        Ok(())
    }
    fn declarations(
        &mut self,
        path: &str,
        slots: &mut DeclaredSlots,
        owner: &SlotOwnerDefId,
    ) -> Result {
        self.direct_slots(&format!("{path}.parameters"), &mut slots.parameters, owner)?;
        self.direct_slots(&format!("{path}.choices"), &mut slots.choices, owner)?;
        self.direct_slots(&format!("{path}.grants"), &mut slots.grants, owner)?;
        self.direct_slots(&format!("{path}.actors"), &mut slots.actors, owner)?;
        self.direct_slots(
            &format!("{path}.skill_grants"),
            &mut slots.skill_grants,
            owner,
        )?;
        self.direct_slots(&format!("{path}.outputs"), &mut slots.outputs, owner)?;
        self.definitions(&format!("{path}.sockets"), &mut slots.sockets)
    }
    fn integer_range(&self, path: &str, range: &IntegerRange) -> Result {
        if range.minimum > range.maximum {
            return invalid(path, SchemaPackageErrorKind::ReversedRange);
        }
        Ok(())
    }
    fn quantity_range(&self, path: &str, range: &QuantityRange) -> Result {
        self.definition(path, range.minimum.unit())?;
        self.definition(path, range.maximum.unit())?;
        if range.minimum.unit() != range.maximum.unit() {
            return invalid(path, SchemaPackageErrorKind::UnitMismatch);
        }
        if range.minimum.value() > range.maximum.value() {
            return invalid(path, SchemaPackageErrorKind::ReversedRange);
        }
        Ok(())
    }
    fn value(&mut self, path: &str, value: &mut ValueSchema) -> Result {
        match value {
            ValueSchema::Boolean => Ok(()),
            ValueSchema::Integer(range) => self.integer_range(path, range),
            ValueSchema::Quantity(range) => self.quantity_range(path, range),
            ValueSchema::Option { allowed } => self.definitions(path, allowed),
        }
    }
    fn quality(&mut self, path: &str, quality: &mut QualityUseSchema) -> Result {
        self.definitions(path, &mut quality.allowed_kinds)
    }
    fn descriptor(&mut self, path: &str, descriptor: &mut DefinitionDescriptor) -> Result {
        macro_rules! owned {
            ($entry:ident, $owner:ident, $body:expr) => {{
                let owner = SlotOwnerDefId::$owner($entry.id.clone());
                self.state(path, &mut $entry.schema, |check, schema| {
                    ($body)(check, schema)?;
                    check.declarations(
                        &format!("{path}.declarations"),
                        &mut schema.declarations,
                        &owner,
                    )
                })
            }};
        }
        match descriptor {
            DefinitionDescriptor::Class(e) => {
                owned!(e, Class, |c: &mut Self, s: &mut ClassSchema| {
                    c.integer_range(path, &s.level)?;
                    c.definitions(&format!("{path}.ascendancies"), &mut s.ascendancies)?;
                    c.definitions(
                        &format!("{path}.implicit_passives"),
                        &mut s.implicit_passives,
                    )
                })
            }
            DefinitionDescriptor::Ascendancy(e) => {
                owned!(e, Ascendancy, |c: &mut Self, s: &mut AscendancySchema| {
                    c.definitions(&format!("{path}.classes"), &mut s.classes)?;
                    c.definitions(
                        &format!("{path}.implicit_passives"),
                        &mut s.implicit_passives,
                    )
                })
            }
            DefinitionDescriptor::Reward(e) => owned!(e, Reward, |_c: &mut Self,
                                                                  _s: &mut RewardSchema|
             -> Result { Ok(()) }),
            DefinitionDescriptor::ItemTemplate(e) => owned!(
                e,
                ItemTemplate,
                |c: &mut Self, s: &mut ItemTemplateSchema| {
                    c.integer_range(path, &s.item_level)?;
                    c.definitions(&format!("{path}.equipment_slots"), &mut s.equipment_slots)?;
                    c.definitions(
                        &format!("{path}.socket_destinations"),
                        &mut s.socket_destinations,
                    )?;
                    c.definitions(&format!("{path}.modifiers"), &mut s.modifiers)?;
                    c.quality(&format!("{path}.quality"), &mut s.quality)
                }
            ),
            DefinitionDescriptor::Modifier(e) => {
                owned!(e, Modifier, |_c: &mut Self,
                                     _s: &mut ModifierSchema|
                 -> Result { Ok(()) })
            }
            DefinitionDescriptor::Gem(e) => owned!(e, Gem, |c: &mut Self, s: &mut GemSchema| {
                c.integer_range(path, &s.level)?;
                c.sorted(&format!("{path}.roles"), &mut s.roles)?;
                c.definitions(&format!("{path}.skills"), &mut s.skills)?;
                c.quality(&format!("{path}.quality"), &mut s.quality)
            }),
            DefinitionDescriptor::Skill(e) => owned!(e, Skill, |_c: &mut Self,
                                                                _s: &mut SkillSchema|
             -> Result { Ok(()) }),
            DefinitionDescriptor::PassiveNode(e) => {
                owned!(e, PassiveNode, |c: &mut Self, s: &mut PassiveNodeSchema| {
                    c.definitions(&format!("{path}.pools"), &mut s.pools)?;
                    c.definitions(&format!("{path}.adjacent"), &mut s.adjacent)
                })
            }
            DefinitionDescriptor::UsagePolicy(e) => {
                owned!(e, UsagePolicy, |c: &mut Self, s: &mut UsagePolicySchema| c
                    .sorted(&format!("{path}.targets"), &mut s.targets))
            }
            DefinitionDescriptor::PointPool(e) => self.state(path, &mut e.schema, |_, _| Ok(())),
            DefinitionDescriptor::EquipmentSlot(e) => {
                self.state(path, &mut e.schema, |_, _| Ok(()))
            }
            DefinitionDescriptor::SocketSlot(e) => self.state(path, &mut e.schema, |c, s| {
                c.owner(path, &s.owner)?;
                if !matches!(
                    (&s.owner, s.kind),
                    (SlotOwnerDefId::ItemTemplate(_), SocketKind::Item)
                        | (SlotOwnerDefId::PassiveNode(_), SocketKind::Passive)
                ) {
                    return invalid(path, SchemaPackageErrorKind::WrongSocketOwner);
                }
                Ok(())
            }),
            DefinitionDescriptor::Encounter(e) => self.state(path, &mut e.schema, |c, s| {
                c.integer_range(path, &s.enemy_level)?;
                c.definitions(&format!("{path}.external_inputs"), &mut s.external_inputs)
            }),
            DefinitionDescriptor::Metric(e) => self.state(path, &mut e.schema, |c, s| {
                c.definition(path, &s.unit)?;
                c.sorted(&format!("{path}.targets"), &mut s.targets)?;
                c.sorted(&format!("{path}.actor_roles"), &mut s.actor_roles)?;
                c.sorted(&format!("{path}.provider_roles"), &mut s.provider_roles)
            }),
            DefinitionDescriptor::SkillLinkRole(e) => self.state(path, &mut e.schema, |c, s| {
                c.definitions(&format!("{path}.containers"), &mut s.containers)?;
                c.definitions(&format!("{path}.payloads"), &mut s.payloads)
            }),
            DefinitionDescriptor::Unit(e) => self.state(path, &mut e.schema, |_, _| Ok(())),
            DefinitionDescriptor::Quality(e) => self.state(path, &mut e.schema, |c, s| {
                c.quantity_range(path, &s.amount)
            }),
            DefinitionDescriptor::ExternalInput(e) => self.state(path, &mut e.schema, |c, s| {
                c.value(path, &mut s.value)?;
                c.sorted(&format!("{path}.targets"), &mut s.targets)
            }),
            DefinitionDescriptor::Stat(e) => self.state(path, &mut e.schema, |c, s| {
                if let ComputedValueType::Quantity { unit } = &s.value {
                    c.definition(&format!("{path}.value.unit"), unit)?;
                }
                c.sorted(&format!("{path}.targets"), &mut s.targets)
            }),
            DefinitionDescriptor::Capability(e) => self.state(path, &mut e.schema, |c, s| {
                c.sorted(&format!("{path}.targets"), &mut s.targets)
            }),
            DefinitionDescriptor::Option(e) => self.state(path, &mut e.schema, |_, _| Ok(())),
            DefinitionDescriptor::ActionPart(e) => self.state(path, &mut e.schema, |_, _| Ok(())),
            DefinitionDescriptor::ActionMode(e) => self.state(path, &mut e.schema, |_, _| Ok(())),
            DefinitionDescriptor::ActionStatSet(e) => {
                self.state(path, &mut e.schema, |_, _| Ok(()))
            }
        }
    }
    fn slot_descriptor(&mut self, path: &str, descriptor: &mut SlotDescriptor) -> Result {
        match descriptor {
            SlotDescriptor::Parameter(e) => {
                let owner = e.id.declaration.clone();
                self.state(path, &mut e.schema, |c, s| {
                    c.value(path, &mut s.value)?;
                    c.sorted(&format!("{path}.sites"), &mut s.sites)?;
                    if s.sites
                        .iter()
                        .any(|site| Some(*site) != parameter_site(&owner))
                    {
                        return invalid(path, SchemaPackageErrorKind::WrongParameterSite);
                    }
                    Ok(())
                })
            }
            SlotDescriptor::Choice(e) => self.state(path, &mut e.schema, |c, s| {
                c.value(path, &mut s.value)?;
                c.collection(path, s.owners.len())?;
                s.owners.sort_by_key(choice_owner_key);
                if s.owners.windows(2).any(|w| w[0] == w[1]) {
                    return invalid(path, SchemaPackageErrorKind::DuplicateMember);
                }
                Ok(())
            }),
            SlotDescriptor::Grant(e) => self.state(path, &mut e.schema, |c, s| {
                c.sorted(&format!("{path}.provider_roles"), &mut s.provider_roles)?;
                match &mut s.target {
                    GrantTarget::Skill(key) => c.slot(path, key),
                    GrantTarget::Actor(key) => c.slot(path, key),
                    GrantTarget::AllocationAccess { pools } => c.definitions(path, pools),
                }
            }),
            SlotDescriptor::SkillGrant(e) => self.state(path, &mut e.schema, |c, s| {
                c.definition(path, &s.skill)?;
                c.slots(&format!("{path}.outputs"), &mut s.outputs)
            }),
            SlotDescriptor::Actor(e) => self.state(path, &mut e.schema, |c, s| {
                c.definitions(&format!("{path}.skills"), &mut s.skills)?;
                c.slots(&format!("{path}.outputs"), &mut s.outputs)
            }),
            SlotDescriptor::ActionOutput(e) => self.state(path, &mut e.schema, |c, s| {
                if let DeclaredActorRole::OwnedSlot(actor) = &s.actor_role {
                    c.slot(path, actor)?;
                }
                c.definitions(&format!("{path}.parts"), &mut s.parts)?;
                c.definitions(&format!("{path}.modes"), &mut s.modes)?;
                c.definitions(&format!("{path}.stat_sets"), &mut s.stat_sets)?;
                c.slots(&format!("{path}.choices"), &mut s.choices)
            }),
        }
    }
}

fn validate_and_canonicalize(
    input: &mut SchemaPackageInput,
    limits: OwnedSchemaLimits,
) -> Result<ResourceUse> {
    let mut check = Check {
        namespace: &input.namespace,
        definitions: BTreeSet::new(),
        slots: BTreeSet::new(),
        limits,
        resources: ResourceUse {
            entries: 0,
            largest_collection: 0,
        },
    };
    check.collection("definitions", input.definitions.len())?;
    check.collection("slots", input.slots.len())?;
    for (i, descriptor) in input.definitions.iter().enumerate() {
        let path = format!("definitions[{i}].id");
        let address = descriptor.address();
        check.namespace(&path, address.namespace())?;
        if !check.definitions.insert(address) {
            return invalid(&path, SchemaPackageErrorKind::DuplicateAddress);
        }
    }
    for (i, descriptor) in input.slots.iter().enumerate() {
        let path = format!("slots[{i}].id");
        let address = descriptor.address();
        check.namespace(&path, address.namespace())?;
        check.owner(&path, address.declaration())?;
        if !check.slots.insert(address) {
            return invalid(&path, SchemaPackageErrorKind::DuplicateAddress);
        }
    }
    for (i, descriptor) in input.definitions.iter_mut().enumerate() {
        check.descriptor(&format!("definitions[{i}]"), descriptor)?;
    }
    for (i, descriptor) in input.slots.iter_mut().enumerate() {
        check.slot_descriptor(&format!("slots[{i}]"), descriptor)?;
    }
    let resources = check.resources;
    input
        .definitions
        .sort_by_cached_key(DefinitionDescriptor::address);
    input.slots.sort_by_cached_key(SlotDescriptor::address);
    check_declaration_consistency(input)?;
    Ok(resources)
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
fn parameter_site(owner: &SlotOwnerDefId) -> Option<ParameterSite> {
    match owner {
        SlotOwnerDefId::ItemTemplate(_) => Some(ParameterSite::ItemParameter),
        SlotOwnerDefId::Gem(_) => Some(ParameterSite::GemParameter),
        SlotOwnerDefId::Modifier(_) => Some(ParameterSite::ModifierRoll),
        SlotOwnerDefId::Reward(_) => Some(ParameterSite::RewardParameter),
        SlotOwnerDefId::UsagePolicy(_) => Some(ParameterSite::UsagePolicyParameter),
        _ => None,
    }
}
fn compare_gaps(a: &SchemaGap, b: &SchemaGap) -> Ordering {
    let subject = match (&a.subject, &b.subject) {
        (SchemaSubject::Definition(a), SchemaSubject::Definition(b)) => a.cmp(b),
        (SchemaSubject::Slot(a), SchemaSubject::Slot(b)) => a.cmp(b),
        (SchemaSubject::Definition(_), SchemaSubject::Slot(_)) => Ordering::Less,
        (SchemaSubject::Slot(_), SchemaSubject::Definition(_)) => Ordering::Greater,
    };
    subject
        .then(a.facet.cmp(&b.facet))
        .then(a.code.cmp(&b.code))
}
fn choice_owner_key(owner: &ChoiceOwnerScope) -> (u8, Option<ProviderRole>) {
    match owner {
        ChoiceOwnerScope::Character => (0, None),
        ChoiceOwnerScope::EquipmentUse => (1, None),
        ChoiceOwnerScope::Allocation => (2, None),
        ChoiceOwnerScope::Skill => (3, None),
        ChoiceOwnerScope::Action => (4, None),
        ChoiceOwnerScope::Provider(role) => (5, Some(*role)),
    }
}

fn known_declarations(descriptor: &DefinitionDescriptor) -> Option<&DeclaredSlots> {
    macro_rules! declarations {
        ($e:ident) => {
            match &$e.schema {
                SchemaState::Known(schema) => Some(&schema.declarations),
                SchemaState::Unmapped { .. } => None,
            }
        };
    }
    match descriptor {
        DefinitionDescriptor::Class(e) => declarations!(e),
        DefinitionDescriptor::Ascendancy(e) => declarations!(e),
        DefinitionDescriptor::Reward(e) => declarations!(e),
        DefinitionDescriptor::ItemTemplate(e) => declarations!(e),
        DefinitionDescriptor::Modifier(e) => declarations!(e),
        DefinitionDescriptor::Gem(e) => declarations!(e),
        DefinitionDescriptor::Skill(e) => declarations!(e),
        DefinitionDescriptor::PassiveNode(e) => declarations!(e),
        DefinitionDescriptor::UsagePolicy(e) => declarations!(e),
        _ => None,
    }
}
fn closed_absence<T: Ord>(set: &DeclaredSet<T>, target: &T) -> bool {
    // Every declaration set was sorted and duplicate-checked before this pass.
    // Repeated owner membership checks therefore remain O(log N), not O(N).
    set.is_complete() && set.members.binary_search(target).is_err()
}
fn check_declaration_consistency(input: &SchemaPackageInput) -> Result {
    let definitions: BTreeMap<_, _> = input.definitions.iter().map(|d| (d.address(), d)).collect();
    for (i, descriptor) in input.definitions.iter().enumerate() {
        let implicit = match descriptor {
            DefinitionDescriptor::Class(DefinitionEntry {
                schema: SchemaState::Known(schema),
                ..
            }) => Some(&schema.implicit_passives),
            DefinitionDescriptor::Ascendancy(DefinitionEntry {
                schema: SchemaState::Known(schema),
                ..
            }) => Some(&schema.implicit_passives),
            _ => None,
        };
        if let Some(implicit) = implicit {
            for (j, node) in implicit.members.iter().enumerate() {
                if let DefinitionDescriptor::PassiveNode(DefinitionEntry {
                    schema: SchemaState::Known(schema),
                    ..
                }) = definitions[&node.address()]
                    && !schema.pools.members.is_empty()
                {
                    return invalid(
                        &format!("definitions[{i}].implicit_passives[{j}]"),
                        SchemaPackageErrorKind::ImplicitPassiveHasPools,
                    );
                }
            }
        }
    }
    for (i, descriptor) in input.slots.iter().enumerate() {
        let address = descriptor.address();
        let owner = definitions[&owner_address(address.declaration())];
        if let Some(declarations) = known_declarations(owner) {
            let missing = match &address {
                SlotAddress::Parameter(key) => closed_absence(&declarations.parameters, key),
                SlotAddress::Choice(key) => closed_absence(&declarations.choices, key),
                SlotAddress::Grant(key) => closed_absence(&declarations.grants, key),
                SlotAddress::Actor(key) => closed_absence(&declarations.actors, key),
                SlotAddress::SkillGrant(key) => closed_absence(&declarations.skill_grants, key),
                SlotAddress::ActionOutput(key) => closed_absence(&declarations.outputs, key),
            };
            if missing {
                return invalid(
                    &format!("slots[{i}]"),
                    SchemaPackageErrorKind::UndeclaredSlot,
                );
            }
        }
    }
    for (i, descriptor) in input.definitions.iter().enumerate() {
        if let DefinitionDescriptor::SocketSlot(entry) = descriptor
            && let SchemaState::Known(socket) = &entry.schema
        {
            let owner = definitions[&owner_address(&socket.owner)];
            if known_declarations(owner).is_some_and(|d| closed_absence(&d.sockets, &entry.id)) {
                return invalid(
                    &format!("definitions[{i}]"),
                    SchemaPackageErrorKind::UndeclaredSlot,
                );
            }
        }
        if let Some(declarations) = known_declarations(descriptor) {
            let owner_address = descriptor.address();
            for socket in &declarations.sockets.members {
                if let DefinitionDescriptor::SocketSlot(entry) = definitions[&socket.address()]
                    && let SchemaState::Known(schema) = &entry.schema
                    && owner_address != self::owner_address(&schema.owner)
                {
                    return invalid(
                        &format!("definitions[{i}].declarations.sockets"),
                        SchemaPackageErrorKind::WrongDeclaration,
                    );
                }
            }
        }
    }
    Ok(())
}
