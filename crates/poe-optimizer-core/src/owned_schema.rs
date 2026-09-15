//! Portable definition-input schemas and immutable indexed access.
//!
//! These are raw schema DTOs, not validated packages or evaluation authority. The
//! data loader validates ranges, reference closure, direct declaration ownership,
//! duplicate membership, resource bounds and package identity. Binding checks the
//! concrete authored context. Potential links do not establish activation, game
//! legality, numerical rule coverage or the existence of a generated actor.

use crate::{data::DataIdentity, owned_build::DeclaredSlot, owned_definitions::*};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum SchemaState<T> {
    Known(T),
    Unmapped { gaps: Vec<SchemaGap> },
}
impl<T> SchemaState<T> {
    pub fn as_lookup(&self) -> SchemaLookup<'_, T> {
        match self {
            Self::Known(value) => SchemaLookup::Known(value),
            Self::Unmapped { gaps } => SchemaLookup::Unmapped(gaps),
        }
    }
}

/// An empty complete set is known empty; a partial set never implies absence.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeclaredSet<T> {
    pub members: Vec<T>,
    pub closure: SchemaClosure,
}
impl<T> DeclaredSet<T> {
    pub fn complete(members: Vec<T>) -> Self {
        Self {
            members,
            closure: SchemaClosure::Complete,
        }
    }
    pub fn partial(members: Vec<T>, gaps: Vec<SchemaGap>) -> Self {
        Self {
            members,
            closure: SchemaClosure::Partial { gaps },
        }
    }
    pub fn is_complete(&self) -> bool {
        matches!(self.closure, SchemaClosure::Complete)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum SchemaClosure {
    Complete,
    Partial { gaps: Vec<SchemaGap> },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefinitionEntry<I, D> {
    pub id: I,
    pub schema: SchemaState<D>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaGap {
    pub subject: SchemaSubject,
    pub facet: SchemaFacet,
    pub code: OwnedDefinitionKey,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum SchemaSubject {
    Definition(DefinitionAddress),
    Slot(SlotAddress),
}

macro_rules! schema_enum {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($variant),+ }
    };
}
schema_enum!(SchemaFacet {
    Identity,
    InputSchema,
    StaticLinks,
    GameRules
});
schema_enum!(SlotPresence {
    RequiredOnce,
    OptionalOnce
});
schema_enum!(QualityPresence {
    Forbidden,
    Optional,
    Required
});
schema_enum!(ParameterSite {
    ItemParameter,
    GemParameter,
    ModifierRoll,
    RewardParameter,
    UsagePolicyParameter,
});
schema_enum!(ProviderRole {
    Character,
    EquipmentUse,
    ItemModifier,
    SkillUse,
    SupportAssignment,
    Allocation,
    Reward,
});
schema_enum!(AuthoredGemRole {
    SkillUse,
    SupportAssignment
});
// Allocation-scope eligibility only; this does not define costs or pool capacities.
schema_enum!(PointPoolScope {
    Shared,
    PerLoadout,
    Either
});
schema_enum!(ScopePolicy {
    Shared,
    Selected,
    Either
});
schema_enum!(SocketKind { Item, Passive });
schema_enum!(MetricTargetKind { Actor, Action });
schema_enum!(MetricActorRole { Player, Owned });
schema_enum!(AssumptionTargetKind {
    Environment,
    Enemy,
    Actor,
    Skill
});
schema_enum!(UsageTargetKind {
    Actor,
    Action,
    Skill
});
// Semantic targets for computed facts. These are independent of provider roots
// and source actor/category names; concrete target compatibility binds later.
schema_enum!(RuleEntityKind {
    Actor,
    Action,
    EquipmentUse,
    Modifier,
    Enemy,
    Environment,
});
schema_enum!(UnitDimension {
    DimensionlessFactor,
    PercentagePoints,
    Count,
    Time,
    Rate,
    Distance,
    Damage,
    DamagePerTime,
    ResourcePoints,
    Rating,
});

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ChoiceOwnerScope {
    Character,
    EquipmentUse,
    Allocation,
    Skill,
    Action,
    Provider(ProviderRole),
}

/// Endpoints are inclusive. The loader rejects a reversed range.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntegerRange {
    pub minimum: BoundedInteger,
    pub maximum: BoundedInteger,
}

/// Endpoints must share one exact unit; equal dimensions do not imply conversion.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuantityRange {
    pub minimum: FiniteQuantity,
    pub maximum: FiniteQuantity,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ValueSchema {
    Boolean,
    Integer(IntegerRange),
    Quantity(QuantityRange),
    Option { allowed: DeclaredSet<OptionDefId> },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualityUseSchema {
    pub presence: QualityPresence,
    pub allowed_kinds: DeclaredSet<QualityDefId>,
}

/// Direct declaration lists; each member must belong to the enclosing owner.
/// Cross-links inside slot payloads are explicit references, not inherited slots.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeclaredSlots {
    pub parameters: DeclaredSet<DeclaredSlot<ParameterSlotDefId>>,
    pub choices: DeclaredSet<DeclaredSlot<ChoiceSlotDefId>>,
    pub grants: DeclaredSet<DeclaredSlot<GrantSlotDefId>>,
    pub actors: DeclaredSet<DeclaredSlot<ActorSlotDefId>>,
    pub skill_grants: DeclaredSet<DeclaredSlot<SkillGrantSlotDefId>>,
    pub outputs: DeclaredSet<DeclaredSlot<ActionOutputDefId>>,
    pub sockets: DeclaredSet<SocketSlotDefId>,
}

macro_rules! schema_record {
    ($name:ident { $($field:ident: $ty:ty),* $(,)? }) => {
        #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
        #[serde(deny_unknown_fields)]
        pub struct $name { $(pub $field: $ty),* }
    };
}

schema_record!(ClassSchema {
    level: IntegerRange,
    ascendancies: DeclaredSet<AscendancyDefId>,
    implicit_passives: DeclaredSet<PassiveNodeDefId>,
    declarations: DeclaredSlots,
});
schema_record!(AscendancySchema {
    classes: DeclaredSet<ClassDefId>,
    implicit_passives: DeclaredSet<PassiveNodeDefId>,
    declarations: DeclaredSlots,
});
schema_record!(RewardSchema {
    declarations: DeclaredSlots
});
schema_record!(ItemTemplateSchema {
    item_level: IntegerRange,
    equipment_slots: DeclaredSet<EquipmentSlotDefId>,
    socket_destinations: DeclaredSet<SocketSlotDefId>,
    modifiers: DeclaredSet<ModifierDefId>,
    quality: QualityUseSchema,
    declarations: DeclaredSlots,
});
schema_record!(ModifierSchema {
    declarations: DeclaredSlots
});
schema_record!(GemSchema {
    level: IntegerRange,
    roles: Vec<AuthoredGemRole>,
    skills: DeclaredSet<SkillDefId>,
    quality: QualityUseSchema,
    declarations: DeclaredSlots,
});
schema_record!(SkillSchema {
    directly_selectable: bool,
    declarations: DeclaredSlots,
});
schema_record!(PassiveNodeSchema {
    pools: DeclaredSet<PointPoolDefId>,
    adjacent: DeclaredSet<PassiveNodeDefId>,
    declarations: DeclaredSlots,
});
schema_record!(PointPoolSchema {
    scope: PointPoolScope
});
schema_record!(EquipmentSlotSchema { scope: ScopePolicy });
schema_record!(SocketSlotSchema {
    owner: SlotOwnerDefId,
    kind: SocketKind,
    scope: ScopePolicy,
});
schema_record!(EncounterSchema {
    enemy_level: IntegerRange,
    external_inputs: DeclaredSet<ExternalInputDefId>,
});
schema_record!(MetricSchema {
    targets: Vec<MetricTargetKind>,
    unit: UnitDefId,
    actor_roles: Vec<MetricActorRole>,
    provider_roles: Vec<ProviderRole>,
});
schema_record!(UsagePolicySchema {
    targets: Vec<UsageTargetKind>,
    declarations: DeclaredSlots,
});
schema_record!(SkillLinkRoleSchema {
    containers: DeclaredSet<SkillDefId>,
    payloads: DeclaredSet<SkillDefId>,
});
/// Computed kind only: ranges, reductions, stages and contextual option membership
/// belong to the rule/input contracts. Quantity retains an exact owned unit.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ComputedValueType {
    Boolean,
    Integer,
    Quantity { unit: UnitDefId },
    Option,
}
schema_record!(StatSchema {
    value: ComputedValueType,
    targets: Vec<RuleEntityKind>,
});
schema_record!(CapabilitySchema {
    targets: Vec<RuleEntityKind>,
});
schema_record!(UnitSchema {
    dimension: UnitDimension
});
schema_record!(QualitySchema {
    amount: QuantityRange
});
schema_record!(ExternalInputSchema {
    value: ValueSchema,
    targets: Vec<AssumptionTargetKind>,
});
// These identities acquire contextual membership in the declaring slot/output.
schema_record!(OptionSchema {});
schema_record!(ActionPartSchema {});
schema_record!(ActionModeSchema {});
schema_record!(ActionStatSetSchema {});

schema_record!(ParameterSlotSchema {
    value: ValueSchema,
    presence: SlotPresence,
    sites: Vec<ParameterSite>,
});
schema_record!(ChoiceSlotSchema {
    value: ValueSchema,
    presence: SlotPresence,
    owners: Vec<ChoiceOwnerScope>,
});
schema_record!(GrantSlotSchema {
    provider_roles: Vec<ProviderRole>,
    target: GrantTarget,
});
schema_record!(SkillGrantSlotSchema {
    skill: SkillDefId,
    outputs: DeclaredSet<DeclaredSlot<ActionOutputDefId>>,
});
schema_record!(ActorSlotSchema {
    skills: DeclaredSet<SkillDefId>,
    outputs: DeclaredSet<DeclaredSlot<ActionOutputDefId>>,
});
schema_record!(ActionOutputSchema {
    actor_role: DeclaredActorRole,
    parts: DeclaredSet<ActionPartDefId>,
    modes: DeclaredSet<ActionModeDefId>,
    stat_sets: DeclaredSet<ActionStatSetDefId>,
    choices: DeclaredSet<DeclaredSlot<ChoiceSlotDefId>>,
});

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum GrantTarget {
    Skill(DeclaredSlot<SkillGrantSlotDefId>),
    Actor(DeclaredSlot<ActorSlotDefId>),
    AllocationAccess { pools: DeclaredSet<PointPoolDefId> },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum DeclaredActorRole {
    Player,
    ProviderActor,
    OwnedSlot(DeclaredSlot<ActorSlotDefId>),
}

mod sealed {
    pub trait Definition {}
    pub trait Slot {}
}

/// Sealed association between an existing standalone typed ID and its schema.
pub trait SchemaDefinitionId: sealed::Definition {
    type Descriptor;
    fn address(&self) -> DefinitionAddress;
    fn project(descriptor: &DefinitionDescriptor) -> Option<&SchemaState<Self::Descriptor>>;
}

/// Sealed association for a slot ID; access always includes its exact declaration.
pub trait SchemaSlotId: sealed::Slot + Sized {
    type Descriptor;
    fn address(key: &DeclaredSlot<Self>) -> SlotAddress;
    fn project(descriptor: &SlotDescriptor) -> Option<&SchemaState<Self::Descriptor>>;
}

macro_rules! definition_catalog {
    ($($variant:ident: $id:ty => $schema:ty),+ $(,)?) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
        #[serde(tag = "kind", content = "value", rename_all = "snake_case", deny_unknown_fields)]
        pub enum DefinitionAddress { $($variant($id)),+ }
        impl DefinitionAddress {
            pub fn namespace(&self) -> &GameVersionNamespace {
                match self { $(Self::$variant(id) => id.namespace()),+ }
            }
            pub fn key(&self) -> &OwnedDefinitionKey {
                match self { $(Self::$variant(id) => id.key()),+ }
            }
            pub fn kind(&self) -> DefinitionKind {
                match self { $(Self::$variant(id) => id.kind()),+ }
            }
        }
        #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
        #[serde(tag = "kind", content = "value", rename_all = "snake_case", deny_unknown_fields)]
        pub enum DefinitionDescriptor { $($variant(DefinitionEntry<$id, $schema>)),+ }
        impl DefinitionDescriptor {
            pub fn address(&self) -> DefinitionAddress {
                match self { $(Self::$variant(entry) => DefinitionAddress::$variant(entry.id.clone())),+ }
            }
        }
        $(
            impl sealed::Definition for $id {}
            impl SchemaDefinitionId for $id {
                type Descriptor = $schema;
                fn address(&self) -> DefinitionAddress { DefinitionAddress::$variant(self.clone()) }
                fn project(descriptor: &DefinitionDescriptor) -> Option<&SchemaState<Self::Descriptor>> {
                    match descriptor {
                        DefinitionDescriptor::$variant(entry) => Some(&entry.schema),
                        _ => None,
                    }
                }
            }
        )+
    };
}
definition_catalog! {
    Class: ClassDefId => ClassSchema,
    Ascendancy: AscendancyDefId => AscendancySchema,
    Reward: RewardDefId => RewardSchema,
    ItemTemplate: ItemTemplateDefId => ItemTemplateSchema,
    Modifier: ModifierDefId => ModifierSchema,
    Gem: GemDefId => GemSchema,
    Skill: SkillDefId => SkillSchema,
    PassiveNode: PassiveNodeDefId => PassiveNodeSchema,
    PointPool: PointPoolDefId => PointPoolSchema,
    EquipmentSlot: EquipmentSlotDefId => EquipmentSlotSchema,
    Encounter: EncounterDefId => EncounterSchema,
    Metric: MetricDefId => MetricSchema,
    Option: OptionDefId => OptionSchema,
    ActionPart: ActionPartDefId => ActionPartSchema,
    ActionMode: ActionModeDefId => ActionModeSchema,
    ActionStatSet: ActionStatSetDefId => ActionStatSetSchema,
    UsagePolicy: UsagePolicyDefId => UsagePolicySchema,
    SkillLinkRole: SkillLinkRoleDefId => SkillLinkRoleSchema,
    SocketSlot: SocketSlotDefId => SocketSlotSchema,
    Unit: UnitDefId => UnitSchema,
    Quality: QualityDefId => QualitySchema,
    ExternalInput: ExternalInputDefId => ExternalInputSchema,
    Stat: StatDefId => StatSchema,
    Capability: CapabilityDefId => CapabilitySchema,
}

macro_rules! slot_catalog {
    ($($variant:ident: $id:ty => $schema:ty),+ $(,)?) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
        #[serde(tag = "kind", content = "value", rename_all = "snake_case", deny_unknown_fields)]
        pub enum SlotAddress { $($variant(DeclaredSlot<$id>)),+ }
        impl SlotAddress {
            /// Namespace of the typed slot ID; its declaration is checked separately.
            pub fn namespace(&self) -> &GameVersionNamespace {
                match self { $(Self::$variant(key) => key.slot.namespace()),+ }
            }
            pub fn declaration(&self) -> &SlotOwnerDefId {
                match self { $(Self::$variant(key) => &key.declaration),+ }
            }
            pub fn key(&self) -> &OwnedDefinitionKey {
                match self { $(Self::$variant(key) => key.slot.key()),+ }
            }
            pub fn kind(&self) -> DefinitionKind {
                match self { $(Self::$variant(key) => key.slot.kind()),+ }
            }
        }
        #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
        #[serde(tag = "kind", content = "value", rename_all = "snake_case", deny_unknown_fields)]
        pub enum SlotDescriptor { $($variant(DefinitionEntry<DeclaredSlot<$id>, $schema>)),+ }
        impl SlotDescriptor {
            pub fn address(&self) -> SlotAddress {
                match self { $(Self::$variant(entry) => SlotAddress::$variant(entry.id.clone())),+ }
            }
        }
        $(
            impl sealed::Slot for $id {}
            impl SchemaSlotId for $id {
                type Descriptor = $schema;
                fn address(key: &DeclaredSlot<Self>) -> SlotAddress { SlotAddress::$variant(key.clone()) }
                fn project(descriptor: &SlotDescriptor) -> Option<&SchemaState<Self::Descriptor>> {
                    match descriptor {
                        SlotDescriptor::$variant(entry) => Some(&entry.schema),
                        _ => None,
                    }
                }
            }
        )+
    };
}
slot_catalog! {
    Parameter: ParameterSlotDefId => ParameterSlotSchema,
    Choice: ChoiceSlotDefId => ChoiceSlotSchema,
    Grant: GrantSlotDefId => GrantSlotSchema,
    Actor: ActorSlotDefId => ActorSlotSchema,
    SkillGrant: SkillGrantSlotDefId => SkillGrantSlotSchema,
    ActionOutput: ActionOutputDefId => ActionOutputSchema,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SchemaLookup<'a, T> {
    Known(&'a T),
    Missing,
    Unmapped(&'a [SchemaGap]),
    NamespaceMismatch,
    /// The hook returned a different typed address than the requested entry.
    InconsistentIndex,
}

/// Read-only data access. Production implementations use validated indexed storage.
/// Hooks return stored descriptors; they must not compute effects or traverse links.
/// Empty closed role/site vectors mean known none, never unrestricted applicability.
pub trait DefinitionSchemaIndex {
    fn identity(&self) -> &DataIdentity;
    fn namespace(&self) -> &GameVersionNamespace;
    fn lookup_definition(&self, address: &DefinitionAddress) -> Option<&DefinitionDescriptor>;
    fn lookup_slot(&self, address: &SlotAddress) -> Option<&SlotDescriptor>;

    fn definition<I: SchemaDefinitionId>(&self, id: &I) -> SchemaLookup<'_, I::Descriptor> {
        let address = id.address();
        if address.namespace() != self.namespace() {
            return SchemaLookup::NamespaceMismatch;
        }
        let Some(descriptor) = self.lookup_definition(&address) else {
            return SchemaLookup::Missing;
        };
        if descriptor.address() != address {
            return SchemaLookup::InconsistentIndex;
        }
        match I::project(descriptor) {
            Some(schema) => schema.as_lookup(),
            None => SchemaLookup::InconsistentIndex,
        }
    }

    fn slot<S: SchemaSlotId>(&self, key: &DeclaredSlot<S>) -> SchemaLookup<'_, S::Descriptor> {
        let address = S::address(key);
        if address.namespace() != self.namespace()
            || address.declaration().namespace() != self.namespace()
        {
            return SchemaLookup::NamespaceMismatch;
        }
        let Some(descriptor) = self.lookup_slot(&address) else {
            return SchemaLookup::Missing;
        };
        if descriptor.address() != address {
            return SchemaLookup::InconsistentIndex;
        }
        match S::project(descriptor) {
            Some(schema) => schema.as_lookup(),
            None => SchemaLookup::InconsistentIndex,
        }
    }
}
