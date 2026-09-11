//! Source-neutral identities for authored build occurrences.
//!
//! Hosts supply lineage bytes; there is no random generator, clock, global counter or
//! OS API. Persist allocation watermarks, including identifiers of deleted instances.
//! Restoring duplicate states does not coordinate branches or concurrent allocation.
//!
//! Public IDs are values, not membership certificates or prepared-plan authority.
//! Consumers check actual collection/domain membership and private plan bindings.
//! Integer components use fixed-width lowercase hexadecimal strings on the wire,
//! preserving all bits through JSON/browser round trips.

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::{fmt, num::NonZeroU64, str::FromStr};

/// Version for enclosing persisted contracts adopting these identity encodings.
pub const BUILD_IDENTITY_SCHEMA_VERSION: u32 = 1;

/// Host-selected identity for an independently created/imported build lineage.
///
/// All 128-bit values, including zero, are valid. Hosts must assign distinct lineages.
/// These bytes are neither an inferred content hash nor a secret.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BuildLineage([u8; 16]);

impl BuildLineage {
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
    pub const fn bytes(self) -> [u8; 16] {
        self.0
    }
}

impl fmt::Display for BuildLineage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl FromStr for BuildLineage {
    type Err = BuildIdentityError;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.len() != 32 {
            return Err(BuildIdentityError::InvalidEncoding);
        }
        let mut bytes = [0; 16];
        for (target, pair) in bytes.iter_mut().zip(text.as_bytes().as_chunks::<2>().0) {
            *target = (hex_digit(pair[0])? << 4) | hex_digit(pair[1])?;
        }
        Ok(Self(bytes))
    }
}

impl Serialize for BuildLineage {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for BuildLineage {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(LineageVisitor)
    }
}

struct LineageVisitor;
impl de::Visitor<'_> for LineageVisitor {
    type Value = BuildLineage;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("exactly 32 lowercase hexadecimal digits")
    }
    fn visit_str<E: de::Error>(self, text: &str) -> Result<Self::Value, E> {
        text.parse().map_err(E::custom)
    }
}

/// Host-managed revision counter within a lineage, not a unique owner token.
///
/// Advance on relevant edits and coordinate branches outside this module. Equal
/// revisions do not authorize prepared-data reuse or prove equal build contents.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BuildRevision(#[serde(with = "hex_u64")] u64);

impl BuildRevision {
    pub const INITIAL: Self = Self(0);
    pub const fn from_u64(value: u64) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u64 {
        self.0
    }
    pub fn checked_next(self) -> Result<Self, BuildIdentityError> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(BuildIdentityError::RevisionExhausted)
    }
}

impl fmt::Display for BuildRevision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

impl FromStr for BuildRevision {
    type Err = BuildIdentityError;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        parse_hex_u64(text).map(Self)
    }
}

/// Stable occurrence. Local zero is reserved for an empty allocation watermark.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstanceId {
    lineage: BuildLineage,
    #[serde(with = "hex_nonzero_u64")]
    local: NonZeroU64,
}

impl InstanceId {
    /// Constructs a public value without registering collection membership.
    pub fn from_parts(lineage: BuildLineage, local: u64) -> Result<Self, BuildIdentityError> {
        Ok(Self {
            lineage,
            local: NonZeroU64::new(local).ok_or(BuildIdentityError::ZeroLocal)?,
        })
    }
    pub const fn lineage(self) -> BuildLineage {
        self.lineage
    }
    pub const fn local(self) -> u64 {
        self.local.get()
    }
}

mod sealed {
    pub trait Sealed {}
}

/// Closed set of occurrence domains supported by the local allocator.
///
/// Wrappers prevent accidental Rust selector interchange. Their transparent wire
/// values do not certify a domain; the importing collection validates membership.
pub trait BuildInstanceId: sealed::Sealed + Copy + Eq {
    fn from_instance_id(id: InstanceId) -> Self;
    fn instance_id(self) -> InstanceId;
}
impl sealed::Sealed for InstanceId {}
impl BuildInstanceId for InstanceId {
    fn from_instance_id(id: InstanceId) -> Self {
        id
    }
    fn instance_id(self) -> InstanceId {
        self
    }
}

macro_rules! instance_domains {
    ($($name:ident => $documentation:literal),+ $(,)?) => {
        $(
            #[doc = $documentation]
            #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
            #[serde(transparent)]
            pub struct $name(InstanceId);
            impl $name {
                /// Constructs a typed value without asserting collection membership.
                pub const fn from_instance_id(id: InstanceId) -> Self { Self(id) }
                pub const fn instance_id(self) -> InstanceId { self.0 }
                pub const fn lineage(self) -> BuildLineage { self.0.lineage() }
                pub const fn local(self) -> u64 { self.0.local() }
            }
            impl sealed::Sealed for $name {}
            impl BuildInstanceId for $name {
                fn from_instance_id(id: InstanceId) -> Self { Self(id) }
                fn instance_id(self) -> InstanceId { self.0 }
            }
            impl From<$name> for InstanceId {
                fn from(id: $name) -> Self { id.0 }
            }
        )+
    };
}

instance_domains! {
    SkillSetId => "One saved skill-set occurrence.",
    SkillGroupId => "One ordered group occurrence within a saved skill set.",
    SkillEntryId => "One ordered authored skill/gem entry, independent of its definition.",
    ItemSetId => "One saved item-set occurrence.",
    ItemRecordId => "One saved item record, independent of its potentially many uses.",
    ItemSlotUseId => "One equipment or jewel slot-use occurrence; role belongs to the build model.",
    PassiveSpecId => "One saved passive specification occurrence.",
    ConfigSetId => "One saved configuration-set occurrence.",
}

/// Serializable watermark, including IDs of deleted instances.
///
/// Restoring duplicate states can allocate duplicate IDs. Persist/coordinate this
/// together with the build; it does not prove exclusive allocation authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstanceAllocatorState {
    lineage: BuildLineage,
    #[serde(with = "hex_u64")]
    last_issued: u64,
}

impl InstanceAllocatorState {
    pub const fn from_parts(lineage: BuildLineage, last_issued: u64) -> Self {
        Self {
            lineage,
            last_issued,
        }
    }
    pub const fn lineage(self) -> BuildLineage {
        self.lineage
    }
    pub const fn last_issued(self) -> u64 {
        self.last_issued
    }
}

/// A freshly cloned occurrence and its immediate origin, both in the same domain.
///
/// A deserialized record is not a membership or ancestry proof.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstanceClone<T> {
    pub id: T,
    pub origin: T,
}

/// Single-owner, constant-space allocator for one build lineage.
///
/// Edits retain IDs through preserve; inserts allocate and clones retain an origin
/// link. Deliberately not Clone. Parallel coordination/reserved allocation ranges
/// belong to the host, not a hidden global counter.
#[derive(Debug)]
pub struct InstanceAllocator {
    state: InstanceAllocatorState,
}

impl InstanceAllocator {
    pub const fn new(lineage: BuildLineage) -> Self {
        Self::from_state(InstanceAllocatorState::from_parts(lineage, 0))
    }
    pub const fn from_state(state: InstanceAllocatorState) -> Self {
        Self { state }
    }
    /// Initial enumeration convenience. For an edited/saved build restore its saved
    /// watermark instead, so deleted high IDs cannot be reused. Duplicate IDs and
    /// actual membership are validated by the build owner, not this watermark scan.
    pub fn from_existing(
        lineage: BuildLineage,
        ids: impl IntoIterator<Item = InstanceId>,
    ) -> Result<Self, BuildIdentityError> {
        let mut allocator = Self::new(lineage);
        for id in ids {
            allocator.reserve_existing(id)?;
        }
        Ok(allocator)
    }
    pub const fn state(&self) -> InstanceAllocatorState {
        self.state
    }
    pub const fn lineage(&self) -> BuildLineage {
        self.state.lineage
    }
    pub fn allocate<T: BuildInstanceId>(&mut self) -> Result<T, BuildIdentityError> {
        let next = self
            .state
            .last_issued
            .checked_add(1)
            .ok_or(BuildIdentityError::InstanceExhausted)?;
        let id = InstanceId::from_parts(self.state.lineage, next)?;
        self.state.last_issued = next;
        Ok(T::from_instance_id(id))
    }
    /// Retains the exact value after lineage/watermark checks. This does not prove
    /// live membership, typed domain, or that a gap below the watermark was allocated.
    pub fn preserve<T: BuildInstanceId>(&self, id: T) -> Result<T, BuildIdentityError> {
        self.check_lineage(id.instance_id())?;
        if id.instance_id().local() > self.state.last_issued {
            return Err(BuildIdentityError::BeyondWatermark);
        }
        Ok(id)
    }
    /// Raises the watermark when importing a previously assigned occurrence.
    pub fn reserve_existing<T: BuildInstanceId>(&mut self, id: T) -> Result<T, BuildIdentityError> {
        let instance = id.instance_id();
        self.check_lineage(instance)?;
        self.state.last_issued = self.state.last_issued.max(instance.local());
        Ok(id)
    }
    pub fn clone_instance<T: BuildInstanceId>(
        &mut self,
        origin: T,
    ) -> Result<InstanceClone<T>, BuildIdentityError> {
        self.preserve(origin)?;
        Ok(InstanceClone {
            id: self.allocate()?,
            origin,
        })
    }
    fn check_lineage(&self, id: InstanceId) -> Result<(), BuildIdentityError> {
        if id.lineage() != self.state.lineage {
            return Err(BuildIdentityError::ForeignLineage {
                expected: self.state.lineage,
                actual: id.lineage(),
            });
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuildIdentityError {
    InvalidEncoding,
    ZeroLocal,
    ForeignLineage {
        expected: BuildLineage,
        actual: BuildLineage,
    },
    BeyondWatermark,
    InstanceExhausted,
    RevisionExhausted,
}
impl fmt::Display for BuildIdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEncoding => {
                f.write_str("identity requires exact-width lowercase hexadecimal text")
            }
            Self::ZeroLocal => f.write_str("instance local identifier must be nonzero"),
            Self::ForeignLineage { expected, actual } => {
                write!(f, "foreign build lineage {actual}; expected {expected}")
            }
            Self::BeyondWatermark => {
                f.write_str("instance identifier exceeds the allocation watermark")
            }
            Self::InstanceExhausted => f.write_str("build instance identifier space is exhausted"),
            Self::RevisionExhausted => f.write_str("build revision space is exhausted"),
        }
    }
}
impl std::error::Error for BuildIdentityError {}

fn hex_digit(byte: u8) -> Result<u8, BuildIdentityError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        _ => Err(BuildIdentityError::InvalidEncoding),
    }
}
fn parse_hex_u64(text: &str) -> Result<u64, BuildIdentityError> {
    if text.len() != 16 {
        return Err(BuildIdentityError::InvalidEncoding);
    }
    let mut value = 0;
    for byte in text.bytes() {
        value = (value << 4) | u64::from(hex_digit(byte)?);
    }
    Ok(value)
}
mod hex_u64 {
    use super::*;
    pub fn serialize<S: Serializer>(value: &u64, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&BuildRevision(*value))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
        deserializer.deserialize_str(HexVisitor)
    }
    struct HexVisitor;
    impl de::Visitor<'_> for HexVisitor {
        type Value = u64;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("exactly 16 lowercase hexadecimal digits")
        }
        fn visit_str<E: de::Error>(self, text: &str) -> Result<Self::Value, E> {
            parse_hex_u64(text).map_err(E::custom)
        }
    }
}
mod hex_nonzero_u64 {
    use super::*;
    pub fn serialize<S: Serializer>(value: &NonZeroU64, serializer: S) -> Result<S::Ok, S::Error> {
        hex_u64::serialize(&value.get(), serializer)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<NonZeroU64, D::Error> {
        NonZeroU64::new(hex_u64::deserialize(deserializer)?)
            .ok_or_else(|| de::Error::custom(BuildIdentityError::ZeroLocal))
    }
}
