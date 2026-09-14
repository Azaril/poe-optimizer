//! Source-independent raw drafts. These DTOs and conversions confer no validity,
//! membership, repair authority, definition coverage or numerical availability.
//! Pending candidates are complete owned values; conversion never chooses one.
use crate::{build_identity::*, owned_build::*, owned_definitions::*};
use serde::{Deserialize, Deserializer, Serialize};

fn required_value<'de, D: Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<T, D::Error> {
    T::deserialize(d)
}

/// A structural conversion only. Validation remains the owning draft/session's job.
pub trait ResolveDraft {
    type Resolved;
    fn to_resolved(&self) -> Option<Self::Resolved>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PendingValue<T> {
    pub id: DraftIssueId,
    pub code: OwnedDefinitionKey,
    pub candidates: Vec<T>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    deny_unknown_fields,
    bound(deserialize = "T: Deserialize<'de>")
)]
pub enum DraftField<T> {
    Known {
        #[serde(deserialize_with = "required_value")]
        value: T,
    },
    Pending(PendingValue<T>),
}
impl<T> From<T> for DraftField<T> {
    fn from(value: T) -> Self {
        Self::Known { value }
    }
}
impl<T: Clone> ResolveDraft for DraftField<T> {
    type Resolved = T;
    fn to_resolved(&self) -> Option<T> {
        match self {
            Self::Known { value } => Some(value.clone()),
            Self::Pending(_) => None,
        }
    }
}
impl<T: Clone> DraftField<T> {
    pub fn to_resolved(&self) -> Option<T> {
        ResolveDraft::to_resolved(self)
    }
}

/// Known members remain present even when further semantic members are unknown.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftList<T> {
    pub members: Vec<T>,
    pub completion: DraftListCompletion,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DraftListCompletion {
    Complete,
    Pending {
        id: DraftIssueId,
        code: OwnedDefinitionKey,
    },
}
impl<T, R> From<Vec<R>> for DraftList<T>
where
    T: From<R>,
{
    fn from(members: Vec<R>) -> Self {
        Self {
            members: members.into_iter().map(Into::into).collect(),
            completion: DraftListCompletion::Complete,
        }
    }
}
impl<T: ResolveDraft> ResolveDraft for DraftList<T> {
    type Resolved = Vec<T::Resolved>;
    fn to_resolved(&self) -> Option<Self::Resolved> {
        if !matches!(self.completion, DraftListCompletion::Complete) {
            return None;
        }
        self.members.iter().map(ResolveDraft::to_resolved).collect()
    }
}
impl<T: ResolveDraft> DraftList<T> {
    pub fn to_resolved(&self) -> Option<Vec<T::Resolved>> {
        ResolveDraft::to_resolved(self)
    }
}

macro_rules! from_field {
    (direct,$value:expr) => {
        $value
    };
    (copy,$value:expr) => {
        $value
    };
    (draft,$value:expr) => {
        $value.into()
    };
}
macro_rules! resolve_field {
    (direct,$value:expr) => {
        $value.clone()
    };
    (copy,$value:expr) => {
        $value
    };
    (draft,$value:expr) => {
        $value.to_resolved()?
    };
}
macro_rules! draft_record {
    ($name:ident=>$resolved:ident {$($field:ident:$kind:ty=>$mode:ident),+ $(,)?})=>{
        #[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
        #[serde(deny_unknown_fields)]
        pub struct $name { $(pub $field:$kind,)+ }
        impl From<$resolved> for $name {
            fn from(value:$resolved)->Self { Self{$($field:from_field!($mode,value.$field),)+} }
        }
        impl ResolveDraft for $name {
            type Resolved=$resolved;
            fn to_resolved(&self)->Option<$resolved> { Some($resolved{$($field:resolve_field!($mode,self.$field),)+}) }
        }
        impl $name { pub fn to_resolved(&self)->Option<$resolved> { ResolveDraft::to_resolved(self) } }
    };
}

/// Absent quality is explicit Known(null); a missing field is never that value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DraftQuality {
    Known {
        #[serde(deserialize_with = "required_value")]
        value: Option<QualityDraft>,
    },
    Pending(PendingValue<Option<QualitySelection>>),
}
impl From<Option<QualitySelection>> for DraftQuality {
    fn from(value: Option<QualitySelection>) -> Self {
        Self::Known {
            value: value.map(Into::into),
        }
    }
}
impl ResolveDraft for DraftQuality {
    type Resolved = Option<QualitySelection>;
    fn to_resolved(&self) -> Option<Self::Resolved> {
        match self {
            Self::Pending(_) => None,
            Self::Known { value } => Some(match value {
                Some(value) => Some(value.to_resolved()?),
                None => None,
            }),
        }
    }
}
impl DraftQuality {
    pub fn to_resolved(&self) -> Option<Option<QualitySelection>> {
        ResolveDraft::to_resolved(self)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum DraftEquipmentDestination {
    CharacterSlot(DraftField<EquipmentSlotDefId>),
    ItemSocket {
        container: DraftField<ItemSlotUseId>,
        slot: DraftField<SocketSlotDefId>,
    },
    PassiveSocket {
        allocation: DraftField<AllocationId>,
        slot: DraftField<SocketSlotDefId>,
    },
    Pending(PendingValue<EquipmentDestination>),
}
impl From<EquipmentDestination> for DraftEquipmentDestination {
    fn from(value: EquipmentDestination) -> Self {
        match value {
            EquipmentDestination::CharacterSlot(value) => Self::CharacterSlot(value.into()),
            EquipmentDestination::ItemSocket { container, slot } => Self::ItemSocket {
                container: container.into(),
                slot: slot.into(),
            },
            EquipmentDestination::PassiveSocket { allocation, slot } => Self::PassiveSocket {
                allocation: allocation.into(),
                slot: slot.into(),
            },
        }
    }
}
impl ResolveDraft for DraftEquipmentDestination {
    type Resolved = EquipmentDestination;
    fn to_resolved(&self) -> Option<EquipmentDestination> {
        Some(match self {
            Self::CharacterSlot(value) => EquipmentDestination::CharacterSlot(value.to_resolved()?),
            Self::ItemSocket { container, slot } => EquipmentDestination::ItemSocket {
                container: container.to_resolved()?,
                slot: slot.to_resolved()?,
            },
            Self::PassiveSocket { allocation, slot } => EquipmentDestination::PassiveSocket {
                allocation: allocation.to_resolved()?,
                slot: slot.to_resolved()?,
            },
            Self::Pending(_) => return None,
        })
    }
}
impl DraftEquipmentDestination {
    pub fn to_resolved(&self) -> Option<EquipmentDestination> {
        ResolveDraft::to_resolved(self)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum DraftAuthoredSkillSource {
    Gem(DraftField<GemInstanceId>),
    Direct(DraftField<SkillDefId>),
    Pending(PendingValue<AuthoredSkillSource>),
}
impl From<AuthoredSkillSource> for DraftAuthoredSkillSource {
    fn from(value: AuthoredSkillSource) -> Self {
        match value {
            AuthoredSkillSource::Gem(value) => Self::Gem(value.into()),
            AuthoredSkillSource::Direct(value) => Self::Direct(value.into()),
        }
    }
}
impl ResolveDraft for DraftAuthoredSkillSource {
    type Resolved = AuthoredSkillSource;
    fn to_resolved(&self) -> Option<AuthoredSkillSource> {
        Some(match self {
            Self::Gem(value) => AuthoredSkillSource::Gem(value.to_resolved()?),
            Self::Direct(value) => AuthoredSkillSource::Direct(value.to_resolved()?),
            Self::Pending(_) => return None,
        })
    }
}
impl DraftAuthoredSkillSource {
    pub fn to_resolved(&self) -> Option<AuthoredSkillSource> {
        ResolveDraft::to_resolved(self)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum DraftAllocationAccess {
    Ordinary,
    Granted(ProviderKeyDraft),
    Pending(PendingValue<AllocationAccess>),
}
impl From<AllocationAccess> for DraftAllocationAccess {
    fn from(value: AllocationAccess) -> Self {
        match value {
            AllocationAccess::Ordinary => Self::Ordinary,
            AllocationAccess::Granted(value) => Self::Granted(value.into()),
        }
    }
}
impl ResolveDraft for DraftAllocationAccess {
    type Resolved = AllocationAccess;
    fn to_resolved(&self) -> Option<AllocationAccess> {
        Some(match self {
            Self::Ordinary => AllocationAccess::Ordinary,
            Self::Granted(value) => AllocationAccess::Granted(value.to_resolved()?),
            Self::Pending(_) => return None,
        })
    }
}
impl DraftAllocationAccess {
    pub fn to_resolved(&self) -> Option<AllocationAccess> {
        ResolveDraft::to_resolved(self)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum DraftProviderRoot {
    Character,
    ItemModifier {
        equipment_use: DraftField<ItemSlotUseId>,
        modifier: DraftField<ModifierInstanceId>,
    },
    SkillUse(DraftField<SkillUseId>),
    SupportAssignment(DraftField<SupportAssignmentId>),
    EquipmentUse(DraftField<ItemSlotUseId>),
    Allocation(DraftField<AllocationId>),
    Reward(DraftField<RewardSelectionId>),
    Pending(PendingValue<ProviderRoot>),
}
impl From<ProviderRoot> for DraftProviderRoot {
    fn from(value: ProviderRoot) -> Self {
        match value {
            ProviderRoot::Character => Self::Character,
            ProviderRoot::ItemModifier {
                equipment_use,
                modifier,
            } => Self::ItemModifier {
                equipment_use: equipment_use.into(),
                modifier: modifier.into(),
            },
            ProviderRoot::SkillUse(value) => Self::SkillUse(value.into()),
            ProviderRoot::SupportAssignment(value) => Self::SupportAssignment(value.into()),
            ProviderRoot::EquipmentUse(value) => Self::EquipmentUse(value.into()),
            ProviderRoot::Allocation(value) => Self::Allocation(value.into()),
            ProviderRoot::Reward(value) => Self::Reward(value.into()),
        }
    }
}
impl ResolveDraft for DraftProviderRoot {
    type Resolved = ProviderRoot;
    fn to_resolved(&self) -> Option<ProviderRoot> {
        Some(match self {
            Self::Character => ProviderRoot::Character,
            Self::ItemModifier {
                equipment_use,
                modifier,
            } => ProviderRoot::ItemModifier {
                equipment_use: equipment_use.to_resolved()?,
                modifier: modifier.to_resolved()?,
            },
            Self::SkillUse(value) => ProviderRoot::SkillUse(value.to_resolved()?),
            Self::SupportAssignment(value) => ProviderRoot::SupportAssignment(value.to_resolved()?),
            Self::EquipmentUse(value) => ProviderRoot::EquipmentUse(value.to_resolved()?),
            Self::Allocation(value) => ProviderRoot::Allocation(value.to_resolved()?),
            Self::Reward(value) => ProviderRoot::Reward(value.to_resolved()?),
            Self::Pending(_) => return None,
        })
    }
}
impl DraftProviderRoot {
    pub fn to_resolved(&self) -> Option<ProviderRoot> {
        ResolveDraft::to_resolved(self)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum DraftSkillTarget {
    Authored(DraftField<SkillUseId>),
    Generated(Box<GeneratedSkillKeyDraft>),
    Pending(PendingValue<SkillTarget>),
}
impl From<SkillTarget> for DraftSkillTarget {
    fn from(value: SkillTarget) -> Self {
        match value {
            SkillTarget::Authored(value) => Self::Authored(value.into()),
            SkillTarget::Generated(value) => Self::Generated(Box::new((*value).into())),
        }
    }
}
impl ResolveDraft for DraftSkillTarget {
    type Resolved = SkillTarget;
    fn to_resolved(&self) -> Option<SkillTarget> {
        Some(match self {
            Self::Authored(value) => SkillTarget::Authored(value.to_resolved()?),
            Self::Generated(value) => SkillTarget::Generated(Box::new(value.to_resolved()?)),
            Self::Pending(_) => return None,
        })
    }
}
impl DraftSkillTarget {
    pub fn to_resolved(&self) -> Option<SkillTarget> {
        ResolveDraft::to_resolved(self)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum DraftActorKey {
    Player,
    Owned(Box<OwnedActorKeyDraft>),
    Pending(PendingValue<ActorKey>),
}
impl From<ActorKey> for DraftActorKey {
    fn from(value: ActorKey) -> Self {
        match value {
            ActorKey::Player => Self::Player,
            ActorKey::Owned(value) => Self::Owned(Box::new((*value).into())),
        }
    }
}
impl ResolveDraft for DraftActorKey {
    type Resolved = ActorKey;
    fn to_resolved(&self) -> Option<ActorKey> {
        Some(match self {
            Self::Player => ActorKey::Player,
            Self::Owned(value) => ActorKey::Owned(Box::new(value.to_resolved()?)),
            Self::Pending(_) => return None,
        })
    }
}
impl DraftActorKey {
    pub fn to_resolved(&self) -> Option<ActorKey> {
        ResolveDraft::to_resolved(self)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum DraftChoiceOwner {
    Character,
    EquipmentUse(DraftField<ItemSlotUseId>),
    Allocation(DraftField<AllocationId>),
    Skill(DraftSkillTarget),
    Action(Box<ActionSelectionDraft>),
    Provider(ProviderKeyDraft),
    Pending(PendingValue<ChoiceOwner>),
}
impl From<ChoiceOwner> for DraftChoiceOwner {
    fn from(value: ChoiceOwner) -> Self {
        match value {
            ChoiceOwner::Character => Self::Character,
            ChoiceOwner::EquipmentUse(value) => Self::EquipmentUse(value.into()),
            ChoiceOwner::Allocation(value) => Self::Allocation(value.into()),
            ChoiceOwner::Skill(value) => Self::Skill(value.into()),
            ChoiceOwner::Action(value) => Self::Action(Box::new((*value).into())),
            ChoiceOwner::Provider(value) => Self::Provider(value.into()),
        }
    }
}
impl ResolveDraft for DraftChoiceOwner {
    type Resolved = ChoiceOwner;
    fn to_resolved(&self) -> Option<ChoiceOwner> {
        Some(match self {
            Self::Character => ChoiceOwner::Character,
            Self::EquipmentUse(value) => ChoiceOwner::EquipmentUse(value.to_resolved()?),
            Self::Allocation(value) => ChoiceOwner::Allocation(value.to_resolved()?),
            Self::Skill(value) => ChoiceOwner::Skill(value.to_resolved()?),
            Self::Action(value) => ChoiceOwner::Action(Box::new(value.to_resolved()?)),
            Self::Provider(value) => ChoiceOwner::Provider(value.to_resolved()?),
            Self::Pending(_) => return None,
        })
    }
}
impl DraftChoiceOwner {
    pub fn to_resolved(&self) -> Option<ChoiceOwner> {
        ResolveDraft::to_resolved(self)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum DraftAssumptionTarget {
    Environment,
    Enemy,
    Actor(DraftActorKey),
    Skill(DraftSkillTarget),
    Pending(PendingValue<AssumptionTarget>),
}
impl From<AssumptionTarget> for DraftAssumptionTarget {
    fn from(value: AssumptionTarget) -> Self {
        match value {
            AssumptionTarget::Environment => Self::Environment,
            AssumptionTarget::Enemy => Self::Enemy,
            AssumptionTarget::Actor(value) => Self::Actor(value.into()),
            AssumptionTarget::Skill(value) => Self::Skill(value.into()),
        }
    }
}
impl ResolveDraft for DraftAssumptionTarget {
    type Resolved = AssumptionTarget;
    fn to_resolved(&self) -> Option<AssumptionTarget> {
        Some(match self {
            Self::Environment => AssumptionTarget::Environment,
            Self::Enemy => AssumptionTarget::Enemy,
            Self::Actor(value) => AssumptionTarget::Actor(value.to_resolved()?),
            Self::Skill(value) => AssumptionTarget::Skill(value.to_resolved()?),
            Self::Pending(_) => return None,
        })
    }
}
impl DraftAssumptionTarget {
    pub fn to_resolved(&self) -> Option<AssumptionTarget> {
        ResolveDraft::to_resolved(self)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum DraftUsageTarget {
    Actor(DraftActorKey),
    Action(Box<ActionSelectionDraft>),
    Skill(DraftSkillTarget),
    Pending(PendingValue<UsageTarget>),
}
impl From<UsageTarget> for DraftUsageTarget {
    fn from(value: UsageTarget) -> Self {
        match value {
            UsageTarget::Actor(value) => Self::Actor(value.into()),
            UsageTarget::Action(value) => Self::Action(Box::new((*value).into())),
            UsageTarget::Skill(value) => Self::Skill(value.into()),
        }
    }
}
impl ResolveDraft for DraftUsageTarget {
    type Resolved = UsageTarget;
    fn to_resolved(&self) -> Option<UsageTarget> {
        Some(match self {
            Self::Actor(value) => UsageTarget::Actor(value.to_resolved()?),
            Self::Action(value) => UsageTarget::Action(Box::new(value.to_resolved()?)),
            Self::Skill(value) => UsageTarget::Skill(value.to_resolved()?),
            Self::Pending(_) => return None,
        })
    }
}
impl DraftUsageTarget {
    pub fn to_resolved(&self) -> Option<UsageTarget> {
        ResolveDraft::to_resolved(self)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum DraftMetricTarget {
    Actor(DraftActorKey),
    Action(Box<ActionSelectionDraft>),
    Pending(PendingValue<MetricTarget>),
}
impl From<MetricTarget> for DraftMetricTarget {
    fn from(value: MetricTarget) -> Self {
        match value {
            MetricTarget::Actor(value) => Self::Actor(value.into()),
            MetricTarget::Action(value) => Self::Action(Box::new((*value).into())),
        }
    }
}
impl ResolveDraft for DraftMetricTarget {
    type Resolved = MetricTarget;
    fn to_resolved(&self) -> Option<MetricTarget> {
        Some(match self {
            Self::Actor(value) => MetricTarget::Actor(value.to_resolved()?),
            Self::Action(value) => MetricTarget::Action(Box::new(value.to_resolved()?)),
            Self::Pending(_) => return None,
        })
    }
}
impl DraftMetricTarget {
    pub fn to_resolved(&self) -> Option<MetricTarget> {
        ResolveDraft::to_resolved(self)
    }
}

draft_record! { QualityDraft=>QualitySelection {
    kind:DraftField<QualityDefId> =>draft,
    amount:DraftField<FiniteQuantity> =>draft,
}}
draft_record! { ParameterDraft=>ParameterAssignment {
    slot:DraftField<DeclaredSlot<ParameterSlotDefId>> =>draft,
    value:DraftField<ParameterValue> =>draft,
}}
draft_record! { ChoiceSelectionDraft=>ChoiceSelection {
    slot:DraftField<DeclaredSlot<ChoiceSlotDefId>> =>draft,
    value:DraftField<ParameterValue> =>draft,
}}
draft_record! { ModifierDraft=>RolledModifier {
    id:ModifierInstanceId=>copy,
    definition:DraftField<ModifierDefId> =>draft,
    rolls:DraftList<ParameterDraft> =>draft,
}}
draft_record! { ItemDraft=>ItemRecord {
    id:ItemRecordId=>copy,
    template:DraftField<ItemTemplateDefId> =>draft,
    parameters:DraftList<ParameterDraft> =>draft,
    item_level:DraftField<u16> =>draft,
    quality:DraftQuality=>draft,
    modifiers:DraftList<ModifierDraft> =>draft,
}}
draft_record! { GemDraft=>GemInstance {
    id:GemInstanceId=>copy,
    definition:DraftField<GemDefId> =>draft,
    parameters:DraftList<ParameterDraft> =>draft,
    level:DraftField<u16> =>draft,
    quality:DraftQuality=>draft,
}}
draft_record! { RewardDraft=>RewardSelection {
    id:RewardSelectionId=>copy,
    definition:DraftField<RewardDefId> =>draft,
    parameters:DraftList<ParameterDraft> =>draft,
}}
draft_record! { CharacterDraft=>CharacterSpec {
    class:DraftField<ClassDefId> =>draft,
    ascendancy:DraftField<Option<AscendancyDefId>> =>draft,
    level:DraftField<u16> =>draft,
    rewards:DraftList<RewardDraft> =>draft,
}}
draft_record! { EquipmentDraft=>EquipmentUse {
    id:ItemSlotUseId=>copy,
    item:DraftField<ItemRecordId> =>draft,
    destination:DraftEquipmentDestination=>draft,
    scope:DraftField<LoadoutScope> =>draft,
}}
draft_record! { AllocationDraft=>Allocation {
    id:AllocationId=>copy,
    node:DraftField<PassiveNodeDefId> =>draft,
    pool:DraftField<PointPoolDefId> =>draft,
    scope:DraftField<LoadoutScope> =>draft,
    access:DraftAllocationAccess=>draft,
    choices:DraftList<ChoiceSelectionDraft> =>draft,
}}
draft_record! { SkillDraft=>SkillUse {
    id:SkillUseId=>copy,
    source:DraftAuthoredSkillSource=>draft,
    enabled:DraftField<bool> =>draft,
    scope:DraftField<LoadoutScope> =>draft,
}}
draft_record! { SupportDraft=>SupportAssignment {
    id:SupportAssignmentId=>copy,
    support:DraftField<GemInstanceId> =>draft,
    target:DraftSkillTarget=>draft,
    enabled:DraftField<bool> =>draft,
}}
draft_record! { PayloadDraft=>PayloadLink {
    id:PayloadLinkId=>copy,
    container:DraftField<SkillUseId> =>draft,
    payload:DraftField<SkillUseId> =>draft,
    role:DraftField<SkillLinkRoleDefId> =>draft,
}}
draft_record! { ChoiceDraft=>MechanicChoice {
    owner:DraftChoiceOwner=>draft,
    choice:ChoiceSelectionDraft=>draft,
}}
draft_record! { ProviderKeyDraft=>ProviderKey {
    root:DraftProviderRoot=>draft,
    grant_path:DraftList<DraftField<DeclaredSlot<GrantSlotDefId>>> =>draft,
}}
draft_record! { GeneratedSkillKeyDraft=>GeneratedSkillKey {
    provider:ProviderKeyDraft=>draft,
    slot:DraftField<DeclaredSlot<SkillGrantSlotDefId>> =>draft,
}}
draft_record! { OwnedActorKeyDraft=>OwnedActorKey {
    provider:ProviderKeyDraft=>draft,
    slot:DraftField<DeclaredSlot<ActorSlotDefId>> =>draft,
}}
draft_record! { ActionKeyDraft=>ActionKey {
    actor:DraftActorKey=>draft,
    provider:ProviderKeyDraft=>draft,
    output:DraftField<DeclaredSlot<ActionOutputDefId>> =>draft,
}}
draft_record! { ActionSelectionDraft=>ActionSelection {
    action:ActionKeyDraft=>draft,
    part:DraftField<ActionPartDefId> =>draft,
    mode:DraftField<ActionModeDefId> =>draft,
    stat_set:DraftField<ActionStatSetDefId> =>draft,
}}
draft_record! { EnemyDraft=>EnemySpec {
    encounter:DraftField<EncounterDefId> =>draft,
    level:DraftField<u16> =>draft,
}}
draft_record! { ExternalAssumptionDraft=>ExternalAssumption {
    input:DraftField<ExternalInputDefId> =>draft,
    target:DraftAssumptionTarget=>draft,
    value:DraftField<ParameterValue> =>draft,
}}
draft_record! { UsagePolicyDraft=>UsagePolicySelection {
    policy:DraftField<UsagePolicyDefId> =>draft,
    target:DraftUsageTarget=>draft,
    parameters:DraftList<ParameterDraft> =>draft,
}}
draft_record! { ScenarioDraft=>ScenarioInput {
    game_version:GameVersionNamespace=>direct,
    enemy:EnemyDraft=>draft,
    assumptions:DraftList<ExternalAssumptionDraft> =>draft,
    usage:DraftList<UsagePolicyDraft> =>draft,
}}
draft_record! { MetricRequestDraft=>MetricRequest {
    id:QueryId=>direct,
    metric:DraftField<MetricDefId> =>draft,
    target:DraftMetricTarget=>draft,
}}
draft_record! { QueryDraft=>QueryInput {
    game_version:GameVersionNamespace=>direct,
    requests:DraftList<MetricRequestDraft> =>draft,
}}
