//! Project-owned, portable definition references and authored scalar values.
//!
//! Symbols identify semantic definitions within an owned game/version namespace.
//! They contain no display labels, external-source identities or dense plan indices.
//! Decoding establishes only a well-formed typed reference: a selected definition
//! package must still validate existence, compatibility, ownership and units.

use serde::{Deserialize, Deserializer, Serialize, Serializer, de, ser::SerializeStruct};
use std::{
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
    str::FromStr,
};

/// Maximum encoded size of each project-owned game, version or definition symbol.
pub const MAX_OWNED_DEFINITION_KEY_BYTES: usize = 128;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OwnedDefinitionError {
    EmptyKey,
    KeyTooLong { actual: usize, maximum: usize },
    InvalidKey,
    NonFiniteQuantity,
    IntegerOutOfRange { value: i64 },
}
impl fmt::Display for OwnedDefinitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyKey => f.write_str("owned definition symbol must not be empty"),
            Self::KeyTooLong { actual, maximum } => write!(
                f,
                "owned definition symbol has {actual} bytes; maximum is {maximum}"
            ),
            Self::InvalidKey => {
                f.write_str("owned definition symbol must match [a-z0-9][a-z0-9_.-]*")
            }
            Self::NonFiniteQuantity => f.write_str("authored quantity must be finite"),
            Self::IntegerOutOfRange { value } => write!(
                f,
                "authored integer {value} is outside the exact browser integer range"
            ),
        }
    }
}
impl std::error::Error for OwnedDefinitionError {}

/// A bounded, case-sensitive project-owned symbol. Construction never trims,
/// lowercases, hashes or translates an external identifier into semantic authority.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OwnedDefinitionKey(String);
impl OwnedDefinitionKey {
    pub fn new(value: impl Into<String>) -> Result<Self, OwnedDefinitionError> {
        let value = value.into();
        validate_key(&value)?;
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
fn validate_key(value: &str) -> Result<(), OwnedDefinitionError> {
    if value.is_empty() {
        return Err(OwnedDefinitionError::EmptyKey);
    }
    if value.len() > MAX_OWNED_DEFINITION_KEY_BYTES {
        return Err(OwnedDefinitionError::KeyTooLong {
            actual: value.len(),
            maximum: MAX_OWNED_DEFINITION_KEY_BYTES,
        });
    }
    let alphanumeric = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit();
    if !alphanumeric(value.as_bytes()[0])
        || !value
            .bytes()
            .all(|byte| alphanumeric(byte) || matches!(byte, b'_' | b'.' | b'-'))
    {
        return Err(OwnedDefinitionError::InvalidKey);
    }
    Ok(())
}
impl fmt::Display for OwnedDefinitionKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl FromStr for OwnedDefinitionKey {
    type Err = OwnedDefinitionError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}
impl Serialize for OwnedDefinitionKey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}
impl<'de> Deserialize<'de> for OwnedDefinitionKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct KeyVisitor;
        impl de::Visitor<'_> for KeyVisitor {
            type Value = OwnedDefinitionKey;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a bounded project-owned definition symbol")
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                validate_key(value).map_err(E::custom)?;
                Ok(OwnedDefinitionKey(value.to_owned()))
            }
            fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
                OwnedDefinitionKey::new(value).map_err(E::custom)
            }
        }
        deserializer.deserialize_string(KeyVisitor)
    }
}

/// Compatibility namespace; exact package content and semantics bind separately.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameVersionNamespace {
    game: OwnedDefinitionKey,
    version: OwnedDefinitionKey,
}
impl GameVersionNamespace {
    pub fn new(
        game: impl Into<String>,
        version: impl Into<String>,
    ) -> Result<Self, OwnedDefinitionError> {
        Ok(Self::from_keys(
            OwnedDefinitionKey::new(game)?,
            OwnedDefinitionKey::new(version)?,
        ))
    }
    pub fn from_keys(game: OwnedDefinitionKey, version: OwnedDefinitionKey) -> Self {
        Self { game, version }
    }
    pub fn game(&self) -> &OwnedDefinitionKey {
        &self.game
    }
    pub fn version(&self) -> &OwnedDefinitionKey {
        &self.version
    }
}

mod sealed {
    pub trait Sealed {}
}
/// Closed semantic definition domain. No content-specific Rust variants occur here.
pub trait DefinitionDomain: sealed::Sealed + Copy + Eq + Ord + Hash + fmt::Debug {
    const KIND: DefinitionKind;
}
macro_rules! definition_domains {
    ($($marker:ident, $alias:ident, $kind:ident => $tag:literal;)+) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
        pub enum DefinitionKind { $(#[serde(rename = $tag)] $kind,)+ }
        impl DefinitionKind {
            pub const fn as_str(self) -> &'static str { match self { $(Self::$kind => $tag,)+ } }
        }
        $(
            #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
            pub enum $marker {}
            impl sealed::Sealed for $marker {}
            impl DefinitionDomain for $marker { const KIND: DefinitionKind = DefinitionKind::$kind; }
            pub type $alias = DefId<$marker>;
        )+
    };
}
definition_domains! {
    ClassDefinition, ClassDefId, Class => "class";
    AscendancyDefinition, AscendancyDefId, Ascendancy => "ascendancy";
    RewardDefinition, RewardDefId, Reward => "reward";
    ItemTemplateDefinition, ItemTemplateDefId, ItemTemplate => "item_template";
    ModifierDefinition, ModifierDefId, Modifier => "modifier";
    GemDefinition, GemDefId, Gem => "gem";
    SkillDefinition, SkillDefId, Skill => "skill";
    PassiveNodeDefinition, PassiveNodeDefId, PassiveNode => "passive_node";
    PointPoolDefinition, PointPoolDefId, PointPool => "point_pool";
    EquipmentSlotDefinition, EquipmentSlotDefId, EquipmentSlot => "equipment_slot";
    EncounterDefinition, EncounterDefId, Encounter => "encounter";
    MetricDefinition, MetricDefId, Metric => "metric";
    ParameterSlotDefinition, ParameterSlotDefId, ParameterSlot => "parameter_slot";
    ChoiceSlotDefinition, ChoiceSlotDefId, ChoiceSlot => "choice_slot";
    OptionDefinition, OptionDefId, Option => "option";
    GrantSlotDefinition, GrantSlotDefId, GrantSlot => "grant_slot";
    ActorSlotDefinition, ActorSlotDefId, ActorSlot => "actor_slot";
    SkillGrantSlotDefinition, SkillGrantSlotDefId, SkillGrantSlot => "skill_grant_slot";
    ActionOutputDefinition, ActionOutputDefId, ActionOutput => "action_output";
    ActionPartDefinition, ActionPartDefId, ActionPart => "action_part";
    ActionModeDefinition, ActionModeDefId, ActionMode => "action_mode";
    ActionStatSetDefinition, ActionStatSetDefId, ActionStatSet => "action_stat_set";
    UsagePolicyDefinition, UsagePolicyDefId, UsagePolicy => "usage_policy";
    SkillLinkRoleDefinition, SkillLinkRoleDefId, SkillLinkRole => "skill_link_role";
    SocketSlotDefinition, SocketSlotDefId, SocketSlot => "socket_slot";
    UnitDefinition, UnitDefId, Unit => "unit";
    QualityDefinition, QualityDefId, Quality => "quality";
    ExternalInputDefinition, ExternalInputDefId, ExternalInput => "external_input";
    StatDefinition, StatDefId, Stat => "stat";
    CapabilityDefinition, CapabilityDefId, Capability => "capability";
}

/// A typed symbolic definition reference, independent of package availability.
/// The wire kind is mandatory, so decoding never silently casts between domains.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DefId<K: DefinitionDomain> {
    namespace: GameVersionNamespace,
    key: OwnedDefinitionKey,
    marker: PhantomData<fn() -> K>,
}
impl<K: DefinitionDomain> DefId<K> {
    pub fn new(namespace: GameVersionNamespace, key: OwnedDefinitionKey) -> Self {
        Self {
            namespace,
            key,
            marker: PhantomData,
        }
    }
    pub fn parse(
        namespace: GameVersionNamespace,
        key: impl Into<String>,
    ) -> Result<Self, OwnedDefinitionError> {
        Ok(Self::new(namespace, OwnedDefinitionKey::new(key)?))
    }
    pub fn namespace(&self) -> &GameVersionNamespace {
        &self.namespace
    }
    pub fn key(&self) -> &OwnedDefinitionKey {
        &self.key
    }
    pub const fn kind(&self) -> DefinitionKind {
        K::KIND
    }
}
impl<K: DefinitionDomain> Serialize for DefId<K> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut value = serializer.serialize_struct("DefId", 3)?;
        value.serialize_field("kind", &K::KIND)?;
        value.serialize_field("namespace", &self.namespace)?;
        value.serialize_field("key", &self.key)?;
        value.end()
    }
}
impl<'de, K: DefinitionDomain> Deserialize<'de> for DefId<K> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            kind: DefinitionKind,
            namespace: GameVersionNamespace,
            key: OwnedDefinitionKey,
        }
        let wire = Wire::deserialize(deserializer)?;
        if wire.kind != K::KIND {
            return Err(de::Error::custom(format!(
                "expected definition kind {}, received {}",
                K::KIND.as_str(),
                wire.kind.as_str()
            )));
        }
        Ok(Self::new(wire.namespace, wire.key))
    }
}

/// Closed reference to the definition that declares a provider's semantic slot.
/// The outer owner kind and inner typed definition kind must agree on the wire.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "definition",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum SlotOwnerDefId {
    Class(ClassDefId),
    Ascendancy(AscendancyDefId),
    Reward(RewardDefId),
    ItemTemplate(ItemTemplateDefId),
    Modifier(ModifierDefId),
    Gem(GemDefId),
    Skill(SkillDefId),
    PassiveNode(PassiveNodeDefId),
    UsagePolicy(UsagePolicyDefId),
}
macro_rules! owner_access {
    ($self:ident, $method:ident) => {
        match $self {
            SlotOwnerDefId::Class(id) => id.$method(),
            SlotOwnerDefId::Ascendancy(id) => id.$method(),
            SlotOwnerDefId::Reward(id) => id.$method(),
            SlotOwnerDefId::ItemTemplate(id) => id.$method(),
            SlotOwnerDefId::Modifier(id) => id.$method(),
            SlotOwnerDefId::Gem(id) => id.$method(),
            SlotOwnerDefId::Skill(id) => id.$method(),
            SlotOwnerDefId::PassiveNode(id) => id.$method(),
            SlotOwnerDefId::UsagePolicy(id) => id.$method(),
        }
    };
}
impl SlotOwnerDefId {
    pub fn namespace(&self) -> &GameVersionNamespace {
        owner_access!(self, namespace)
    }
    pub fn key(&self) -> &OwnedDefinitionKey {
        owner_access!(self, key)
    }
    pub fn kind(&self) -> DefinitionKind {
        owner_access!(self, kind)
    }
}

/// Authored finite binary quantity. Its unit remains a typed definition reference;
/// unit compatibility and definition-specific bounds are checked during binding.
#[derive(Clone, Debug, Serialize)]
pub struct FiniteQuantity {
    value: f64,
    unit: UnitDefId,
}
impl FiniteQuantity {
    pub fn new(value: f64, unit: UnitDefId) -> Result<Self, OwnedDefinitionError> {
        if !value.is_finite() {
            return Err(OwnedDefinitionError::NonFiniteQuantity);
        }
        Ok(Self {
            value: if value == 0.0 { 0.0 } else { value },
            unit,
        })
    }
    pub fn value(&self) -> f64 {
        self.value
    }
    pub fn unit(&self) -> &UnitDefId {
        &self.unit
    }
}
// Finite values exclude NaN, and construction normalizes both zero encodings.
impl PartialEq for FiniteQuantity {
    fn eq(&self, other: &Self) -> bool {
        self.value.to_bits() == other.value.to_bits() && self.unit == other.unit
    }
}
impl Eq for FiniteQuantity {}
impl Hash for FiniteQuantity {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.to_bits().hash(state);
        self.unit.hash(state);
    }
}
impl<'de> Deserialize<'de> for FiniteQuantity {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            value: f64,
            unit: UnitDefId,
        }
        let wire = Wire::deserialize(deserializer)?;
        Self::new(wire.value, wire.unit).map_err(de::Error::custom)
    }
}

/// Bounded authored integer transported exactly by browser JSON numbers.
/// Game-specific limits are injected by the definition schema, not this type.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "i64", into = "i64")]
pub struct BoundedInteger(i64);
impl BoundedInteger {
    pub const MAX: i64 = (1_i64 << 53) - 1;
    pub const MIN: i64 = -Self::MAX;
    pub fn new(value: i64) -> Result<Self, OwnedDefinitionError> {
        if !(Self::MIN..=Self::MAX).contains(&value) {
            return Err(OwnedDefinitionError::IntegerOutOfRange { value });
        }
        Ok(Self(value))
    }
    pub const fn get(self) -> i64 {
        self.0
    }
}
impl TryFrom<i64> for BoundedInteger {
    type Error = OwnedDefinitionError;
    fn try_from(value: i64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<BoundedInteger> for i64 {
    fn from(value: BoundedInteger) -> Self {
        value.get()
    }
}
