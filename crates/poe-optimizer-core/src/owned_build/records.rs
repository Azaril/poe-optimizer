//! Authored semantic records. Public input DTOs are not validated build authority.
use crate::{build_identity::*, owned_definitions::*};
use serde::{Deserialize, Deserializer, Serialize};

// An explicit null is meaningful. Omission must not introduce an adapter/default policy.
fn required_option<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    Option::<T>::deserialize(d)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildInput {
    pub allocator: InstanceAllocatorState,
    pub revision: BuildRevision,
    pub game_version: GameVersionNamespace,
    pub character: CharacterSpec,
    pub weapon_loadouts: Vec<WeaponLoadoutId>,
    pub active_weapon_loadout: WeaponLoadoutId,
    pub items: Vec<ItemRecord>,
    pub gems: Vec<GemInstance>,
    pub equipment: Vec<EquipmentUse>,
    pub allocations: Vec<Allocation>,
    pub skills: Vec<SkillUse>,
    pub supports: Vec<SupportAssignment>,
    pub payload_links: Vec<PayloadLink>,
    pub choices: Vec<MechanicChoice>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterSpec {
    pub class: ClassDefId,
    #[serde(deserialize_with = "required_option")]
    pub ascendancy: Option<AscendancyDefId>,
    pub level: u16,
    pub rewards: Vec<RewardSelection>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RewardSelection {
    pub id: RewardSelectionId,
    pub definition: RewardDefId,
    pub parameters: Vec<ParameterAssignment>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemRecord {
    pub id: ItemRecordId,
    pub template: ItemTemplateDefId,
    pub parameters: Vec<ParameterAssignment>,
    pub item_level: u16,
    #[serde(deserialize_with = "required_option")]
    pub quality: Option<QualitySelection>,
    pub modifiers: Vec<RolledModifier>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RolledModifier {
    pub id: ModifierInstanceId,
    pub definition: ModifierDefId,
    pub rolls: Vec<ParameterAssignment>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualitySelection {
    pub kind: QualityDefId,
    pub amount: FiniteQuantity,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EquipmentUse {
    pub id: ItemSlotUseId,
    pub item: ItemRecordId,
    pub destination: EquipmentDestination,
    pub scope: LoadoutScope,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum EquipmentDestination {
    CharacterSlot(EquipmentSlotDefId),
    ItemSocket {
        container: ItemSlotUseId,
        slot: SocketSlotDefId,
    },
    PassiveSocket {
        allocation: AllocationId,
        slot: SocketSlotDefId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum LoadoutScope {
    Shared,
    Selected { loadouts: Vec<WeaponLoadoutId> },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GemInstance {
    pub id: GemInstanceId,
    pub definition: GemDefId,
    pub parameters: Vec<ParameterAssignment>,
    pub level: u16,
    #[serde(deserialize_with = "required_option")]
    pub quality: Option<QualitySelection>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillUse {
    pub id: SkillUseId,
    pub source: AuthoredSkillSource,
    pub enabled: bool,
    pub scope: LoadoutScope,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum AuthoredSkillSource {
    Gem(GemInstanceId),
    Direct(SkillDefId),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportAssignment {
    pub id: SupportAssignmentId,
    pub support: GemInstanceId,
    pub target: SkillTarget,
    pub enabled: bool,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum SkillTarget {
    Authored(SkillUseId),
    Generated(Box<GeneratedSkillKey>),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PayloadLink {
    pub id: PayloadLinkId,
    pub container: SkillUseId,
    pub payload: SkillUseId,
    pub role: SkillLinkRoleDefId,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Allocation {
    pub id: AllocationId,
    pub node: PassiveNodeDefId,
    pub pool: PointPoolDefId,
    pub scope: LoadoutScope,
    pub access: AllocationAccess,
    pub choices: Vec<ChoiceSelection>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum AllocationAccess {
    Ordinary,
    Granted(ProviderKey),
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeclaredSlot<S> {
    pub declaration: SlotOwnerDefId,
    pub slot: S,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ParameterValue {
    Boolean(bool),
    Integer(BoundedInteger),
    Quantity(FiniteQuantity),
    Option(OptionDefId),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParameterAssignment {
    pub slot: DeclaredSlot<ParameterSlotDefId>,
    pub value: ParameterValue,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChoiceSelection {
    pub slot: DeclaredSlot<ChoiceSlotDefId>,
    pub value: ParameterValue,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MechanicChoice {
    pub owner: ChoiceOwner,
    pub choice: ChoiceSelection,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ChoiceOwner {
    Character,
    EquipmentUse(ItemSlotUseId),
    Allocation(AllocationId),
    Skill(SkillTarget),
    Action(Box<ActionSelection>),
    Provider(ProviderKey),
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ProviderRoot {
    Character,
    ItemModifier {
        equipment_use: ItemSlotUseId,
        modifier: ModifierInstanceId,
    },
    SkillUse(SkillUseId),
    SupportAssignment(SupportAssignmentId),
    EquipmentUse(ItemSlotUseId),
    Allocation(AllocationId),
    Reward(RewardSelectionId),
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderKey {
    pub root: ProviderRoot,
    pub grant_path: Vec<DeclaredSlot<GrantSlotDefId>>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ActorKey {
    Player,
    Owned(Box<OwnedActorKey>),
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedActorKey {
    pub provider: ProviderKey,
    pub slot: DeclaredSlot<ActorSlotDefId>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratedSkillKey {
    pub provider: ProviderKey,
    pub slot: DeclaredSlot<SkillGrantSlotDefId>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionKey {
    pub actor: ActorKey,
    pub provider: ProviderKey,
    pub output: DeclaredSlot<ActionOutputDefId>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionSelection {
    pub action: ActionKey,
    pub part: ActionPartDefId,
    pub mode: ActionModeDefId,
    pub stat_set: ActionStatSetDefId,
}

/// Caller-assigned response correlation symbol, never build/admission authority.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct QueryId(OwnedDefinitionKey);
impl QueryId {
    pub fn new(value: impl Into<String>) -> Result<Self, OwnedDefinitionError> {
        OwnedDefinitionKey::new(value).map(Self)
    }
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryInput {
    pub game_version: GameVersionNamespace,
    pub requests: Vec<MetricRequest>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricRequest {
    pub id: QueryId,
    pub metric: MetricDefId,
    pub target: MetricTarget,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum MetricTarget {
    Actor(ActorKey),
    Action(Box<ActionSelection>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioInput {
    pub game_version: GameVersionNamespace,
    pub enemy: EnemySpec,
    pub assumptions: Vec<ExternalAssumption>,
    pub usage: Vec<UsagePolicySelection>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnemySpec {
    pub encounter: EncounterDefId,
    pub level: u16,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalAssumption {
    pub input: ExternalInputDefId,
    pub target: AssumptionTarget,
    pub value: ParameterValue,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum AssumptionTarget {
    Environment,
    Enemy,
    Actor(ActorKey),
    Skill(SkillTarget),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsagePolicySelection {
    pub policy: UsagePolicyDefId,
    pub target: UsageTarget,
    pub parameters: Vec<ParameterAssignment>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum UsageTarget {
    Actor(ActorKey),
    Action(Box<ActionSelection>),
    Skill(SkillTarget),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedRequestInput {
    pub build: BuildInput,
    pub scenario: ScenarioInput,
    pub queries: QueryInput,
}
