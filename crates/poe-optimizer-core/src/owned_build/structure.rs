use super::records::*;
use crate::{build_identity::*, owned_definitions::*};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

/// Resource bounds, not game rules. Callers may tighten, but not raise, defaults.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OwnedInputLimits {
    pub max_entries: usize,
    pub max_collection_entries: usize,
    pub max_provider_steps: usize,
    pub max_wire_bytes: usize,
}
impl Default for OwnedInputLimits {
    fn default() -> Self {
        Self {
            max_entries: 100_000,
            max_collection_entries: 16_384,
            max_provider_steps: 64,
            max_wire_bytes: 8 * 1024 * 1024,
        }
    }
}
impl OwnedInputLimits {
    pub(crate) fn validate(self) -> Result {
        let hard = Self::default();
        for (name, value, max) in [
            ("max_entries", self.max_entries, hard.max_entries),
            (
                "max_collection_entries",
                self.max_collection_entries,
                hard.max_collection_entries,
            ),
            (
                "max_provider_steps",
                self.max_provider_steps,
                hard.max_provider_steps,
            ),
            ("max_wire_bytes", self.max_wire_bytes, hard.max_wire_bytes),
        ] {
            if value == 0 || value > max {
                return Err(error(name, StructuralErrorKind::InvalidLimit));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OccurrenceKind {
    Loadout,
    Reward,
    Item,
    InventoryCopy,
    Modifier,
    Gem,
    EquipmentUse,
    Allocation,
    SkillUse,
    SupportAssignment,
    PayloadLink,
    CharacterPreset,
    EquipmentPreset,
    AllocationPreset,
    SkillPreset,
    ChoicePreset,
    SavedVariant,
    DraftIssue,
    ScenarioPreset,
    QueryPreset,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuralError {
    pub path: String,
    pub kind: StructuralErrorKind,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StructuralErrorKind {
    InvalidLimit,
    LimitExceeded,
    ForeignNamespace,
    ForeignLineage,
    BeyondWatermark,
    DuplicateIdentity {
        id: InstanceId,
    },
    MissingReference {
        expected: OccurrenceKind,
        id: InstanceId,
    },
    DuplicateAssignment,
    InvalidModifierOrder,
    WrongDeclaration,
    WrongProviderOwner,
    EmptyLoadoutScope,
    ContainmentCycle,
}
impl fmt::Display for StructuralError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {:?}", self.path, self.kind)
    }
}
impl std::error::Error for StructuralError {}
fn error(path: &str, kind: StructuralErrorKind) -> StructuralError {
    StructuralError {
        path: path.into(),
        kind,
    }
}
type Result<T = ()> = std::result::Result<T, StructuralError>;

/// Borrowed occurrence registries, without a selected character or preset/default.
#[derive(Clone, Copy)]
pub(crate) struct RecordTables<'a> {
    pub weapon_loadouts: &'a [WeaponLoadoutId],
    pub rewards: &'a [RewardSelection],
    pub items: &'a [ItemRecord],
    pub gems: &'a [GemInstance],
    pub equipment: &'a [EquipmentUse],
    pub allocations: &'a [Allocation],
    pub skills: &'a [SkillUse],
    pub supports: &'a [SupportAssignment],
    pub payload_links: &'a [PayloadLink],
}
impl<'a> RecordTables<'a> {
    fn from_build(build: &'a BuildInput) -> Self {
        Self {
            weapon_loadouts: &build.weapon_loadouts,
            rewards: &build.character.rewards,
            items: &build.items,
            gems: &build.gems,
            equipment: &build.equipment,
            allocations: &build.allocations,
            skills: &build.skills,
            supports: &build.supports,
            payload_links: &build.payload_links,
        }
    }
}
pub(crate) struct RecordTablesMut<'a> {
    pub weapon_loadouts: &'a mut [WeaponLoadoutId],
    pub rewards: &'a mut [RewardSelection],
    pub items: &'a mut [ItemRecord],
    pub gems: &'a mut [GemInstance],
    pub equipment: &'a mut [EquipmentUse],
    pub allocations: &'a mut [Allocation],
    pub skills: &'a mut [SkillUse],
    pub supports: &'a mut [SupportAssignment],
    pub payload_links: &'a mut [PayloadLink],
}
pub(crate) struct RecordTableValidation {
    pub entries: usize,
    pub occurrences: Vec<(InstanceId, OccurrenceKind)>,
}

/// Validate every occurrence and choice group once against one shared membership map.
/// Choice groups are alternatives: duplicates are rejected within each group, while
/// allocation/group cross-selection conflicts are checked by the final BuildSpec.
pub(crate) fn validate_record_tables(
    namespace: &GameVersionNamespace,
    allocator: InstanceAllocatorState,
    tables: RecordTables<'_>,
    choice_groups: &[&[MechanicChoice]],
    additional_occurrences: &[(InstanceId, OccurrenceKind)],
    limits: OwnedInputLimits,
) -> Result<RecordTableValidation> {
    let mut check = StructuralCheck::new(namespace, limits, Some(allocator))?;
    check.register_tables(tables)?;
    // Extra occurrences are metadata across actual bounded project collections.
    if additional_occurrences.len() > check.remaining {
        return Err(error(
            "additional_occurrences",
            StructuralErrorKind::LimitExceeded,
        ));
    }
    check.remaining -= additional_occurrences.len();
    for (i, (id, kind)) in additional_occurrences.iter().enumerate() {
        check.register(&format!("additional_occurrences[{i}]"), *id, *kind)?;
    }
    check.record_values(tables)?;
    for choices in choice_groups {
        check.mechanic_choices(choices, BTreeSet::new())?;
    }
    Ok(RecordTableValidation {
        entries: limits.max_entries - check.remaining,
        occurrences: check
            .members
            .expect("registered record occurrences")
            .into_iter()
            .collect(),
    })
}

/// Crate-private structural leaves shared by complete inputs and typed drafts.
pub(crate) struct StructuralCheck<'a> {
    namespace: &'a GameVersionNamespace,
    allocator: Option<InstanceAllocatorState>,
    members: Option<BTreeMap<InstanceId, OccurrenceKind>>,
    remaining: usize,
    allow_missing_references: bool,
    modifier_items: BTreeMap<ModifierInstanceId, ItemRecordId>,
    equipment_items: BTreeMap<ItemSlotUseId, ItemRecordId>,
    limits: OwnedInputLimits,
}
impl<'a> StructuralCheck<'a> {
    pub(crate) fn new(
        namespace: &'a GameVersionNamespace,
        limits: OwnedInputLimits,
        allocator: Option<InstanceAllocatorState>,
    ) -> Result<Self> {
        limits.validate()?;
        Ok(Self {
            namespace,
            allocator,
            members: None,
            remaining: limits.max_entries,
            allow_missing_references: false,
            modifier_items: BTreeMap::new(),
            equipment_items: BTreeMap::new(),
            limits,
        })
    }
    /// Enable membership checks even for an empty registry. Charge each actual
    /// collection and register all rows before checking references.
    /// Repeated initialization does not erase existing membership.
    pub(crate) fn begin_membership(&mut self) {
        self.members.get_or_insert_with(BTreeMap::new);
    }
    /// Permit historical missing roots within saved scenario/query selector checks.
    /// Present wrong-domain IDs and known provider-owner conflicts still reject.
    /// The previous mode is restored when the closure returns either Ok or Err.
    pub(crate) fn with_query_references<T>(
        &mut self,
        check: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        let previous = self.allow_missing_references;
        self.allow_missing_references = true;
        let result = check(self);
        self.allow_missing_references = previous;
        result
    }
    fn known_reference(
        &self,
        path: &str,
        id: impl BuildInstanceId,
        expected: OccurrenceKind,
    ) -> Result {
        let id = id.instance_id();
        self.identity(path, id)?;
        if self.members.as_ref().and_then(|members| members.get(&id)) != Some(&expected) {
            return Err(error(
                path,
                StructuralErrorKind::MissingReference { expected, id },
            ));
        }
        Ok(())
    }
    /// Seed known ownership after registering all typed rows. Pending ownership
    /// must remain absent; it cannot be inferred from an unresolved candidate.
    pub(crate) fn seed_modifier_item(
        &mut self,
        path: &str,
        modifier: ModifierInstanceId,
        item: ItemRecordId,
    ) -> Result {
        self.known_reference(path, modifier, OccurrenceKind::Modifier)?;
        self.known_reference(path, item, OccurrenceKind::Item)?;
        if let Some(previous) = self.modifier_items.get(&modifier)
            && *previous != item
        {
            return Err(error(path, StructuralErrorKind::WrongProviderOwner));
        }
        self.modifier_items.insert(modifier, item);
        Ok(())
    }
    pub(crate) fn seed_equipment_item(
        &mut self,
        path: &str,
        equipment: ItemSlotUseId,
        item: ItemRecordId,
    ) -> Result {
        self.known_reference(path, equipment, OccurrenceKind::EquipmentUse)?;
        self.known_reference(path, item, OccurrenceKind::Item)?;
        if let Some(previous) = self.equipment_items.get(&equipment)
            && *previous != item
        {
            return Err(error(path, StructuralErrorKind::WrongProviderOwner));
        }
        self.equipment_items.insert(equipment, item);
        Ok(())
    }
    pub(crate) fn collection(&mut self, path: &str, len: usize) -> Result {
        if len > self.limits.max_collection_entries || len > self.remaining {
            return Err(error(path, StructuralErrorKind::LimitExceeded));
        }
        self.remaining -= len;
        Ok(())
    }
    pub(crate) fn namespace(&self, path: &str, namespace: &GameVersionNamespace) -> Result {
        if namespace != self.namespace {
            return Err(error(path, StructuralErrorKind::ForeignNamespace));
        }
        Ok(())
    }
    pub(crate) fn definition<K: DefinitionDomain>(&self, path: &str, id: &DefId<K>) -> Result {
        self.namespace(path, id.namespace())
    }
    pub(crate) fn slot<K: DefinitionDomain>(
        &self,
        path: &str,
        slot: &DeclaredSlot<DefId<K>>,
    ) -> Result {
        self.namespace(path, slot.declaration.namespace())?;
        self.definition(path, &slot.slot)
    }
    pub(crate) fn identity(&self, path: &str, id: InstanceId) -> Result {
        if let Some(allocator) = self.allocator {
            if id.lineage() != allocator.lineage() {
                return Err(error(path, StructuralErrorKind::ForeignLineage));
            }
            if id.local() > allocator.last_issued() {
                return Err(error(path, StructuralErrorKind::BeyondWatermark));
            }
        }
        Ok(())
    }
    pub(crate) fn register(
        &mut self,
        path: &str,
        id: impl BuildInstanceId,
        kind: OccurrenceKind,
    ) -> Result {
        let id = id.instance_id();
        self.identity(path, id)?;
        let members = self.members.get_or_insert_with(BTreeMap::new);
        if let std::collections::btree_map::Entry::Vacant(entry) = members.entry(id) {
            entry.insert(kind);
            Ok(())
        } else {
            Err(error(path, StructuralErrorKind::DuplicateIdentity { id }))
        }
    }
    pub(crate) fn reference(
        &self,
        path: &str,
        id: impl BuildInstanceId,
        expected: OccurrenceKind,
    ) -> Result {
        let id = id.instance_id();
        self.identity(path, id)?;
        if let Some(members) = &self.members {
            let invalid = match members.get(&id) {
                Some(actual) => *actual != expected,
                None => !self.allow_missing_references,
            };
            if invalid {
                return Err(error(
                    path,
                    StructuralErrorKind::MissingReference { expected, id },
                ));
            }
        }
        Ok(())
    }
    pub(crate) fn provider(&mut self, path: &str, key: &ProviderKey) -> Result {
        match key.root {
            ProviderRoot::Character => {}
            ProviderRoot::ItemModifier {
                equipment_use,
                modifier,
            } => {
                self.reference(path, equipment_use, OccurrenceKind::EquipmentUse)?;
                self.reference(path, modifier, OccurrenceKind::Modifier)?;
                if let (Some(receiving), Some(supplying)) = (
                    self.equipment_items.get(&equipment_use),
                    self.modifier_items.get(&modifier),
                ) && receiving != supplying
                {
                    return Err(error(path, StructuralErrorKind::WrongProviderOwner));
                }
            }
            ProviderRoot::SkillUse(id) => self.reference(path, id, OccurrenceKind::SkillUse)?,
            ProviderRoot::SupportAssignment(id) => {
                self.reference(path, id, OccurrenceKind::SupportAssignment)?
            }
            ProviderRoot::EquipmentUse(id) => {
                self.reference(path, id, OccurrenceKind::EquipmentUse)?
            }
            ProviderRoot::Allocation(id) => self.reference(path, id, OccurrenceKind::Allocation)?,
            ProviderRoot::Reward(id) => self.reference(path, id, OccurrenceKind::Reward)?,
        }
        if key.grant_path.len() > self.limits.max_provider_steps {
            return Err(error(path, StructuralErrorKind::LimitExceeded));
        }
        self.collection(path, key.grant_path.len())?;
        for slot in &key.grant_path {
            self.slot(path, slot)?;
        }
        // A repeated declaration in a finite symbolic path is not itself a cycle.
        // Generated ownership existence/cycles belong to definition resolution.
        Ok(())
    }
    pub(crate) fn actor(&mut self, path: &str, actor: &ActorKey) -> Result {
        if let ActorKey::Owned(actor) = actor {
            self.provider(path, &actor.provider)?;
            self.slot(path, &actor.slot)?;
        }
        Ok(())
    }
    pub(crate) fn skill(&mut self, path: &str, target: &SkillTarget) -> Result {
        match target {
            SkillTarget::Authored(id) => self.reference(path, *id, OccurrenceKind::SkillUse),
            SkillTarget::Generated(key) => {
                self.provider(path, &key.provider)?;
                self.slot(path, &key.slot)
            }
        }
    }
    pub(crate) fn action(&mut self, path: &str, action: &ActionSelection) -> Result {
        self.actor(path, &action.action.actor)?;
        self.provider(path, &action.action.provider)?;
        self.slot(path, &action.action.output)?;
        self.definition(path, &action.part)?;
        self.definition(path, &action.mode)?;
        self.definition(path, &action.stat_set)
    }
    pub(crate) fn value(&self, path: &str, value: &ParameterValue) -> Result {
        match value {
            ParameterValue::Boolean(_) | ParameterValue::Integer(_) => Ok(()),
            ParameterValue::Quantity(value) => self.definition(path, value.unit()),
            ParameterValue::Option(value) => self.definition(path, value),
        }
    }
    pub(crate) fn quality(&self, path: &str, quality: &Option<QualitySelection>) -> Result {
        if let Some(quality) = quality {
            self.definition(path, &quality.kind)?;
            self.definition(path, quality.amount.unit())?;
        }
        Ok(())
    }
    pub(crate) fn parameters(
        &mut self,
        path: &str,
        parameters: &[ParameterAssignment],
        declaration: &SlotOwnerDefId,
    ) -> Result {
        self.collection(path, parameters.len())?;
        let mut seen = BTreeSet::new();
        for (i, parameter) in parameters.iter().enumerate() {
            let path = format!("{path}[{i}]");
            if !seen.insert(&parameter.slot) {
                return Err(error(&path, StructuralErrorKind::DuplicateAssignment));
            }
            if &parameter.slot.declaration != declaration {
                return Err(error(&path, StructuralErrorKind::WrongDeclaration));
            }
            self.slot(&path, &parameter.slot)?;
            self.value(&path, &parameter.value)?;
        }
        Ok(())
    }
    pub(crate) fn choice(&self, path: &str, choice: &ChoiceSelection) -> Result {
        self.slot(path, &choice.slot)?;
        self.value(path, &choice.value)
    }
    pub(crate) fn choice_owner(&mut self, path: &str, owner: &ChoiceOwner) -> Result {
        match owner {
            ChoiceOwner::Character => Ok(()),
            ChoiceOwner::EquipmentUse(id) => {
                self.reference(path, *id, OccurrenceKind::EquipmentUse)
            }
            ChoiceOwner::Allocation(id) => self.reference(path, *id, OccurrenceKind::Allocation),
            ChoiceOwner::Skill(target) => self.skill(path, target),
            ChoiceOwner::Action(target) => self.action(path, target),
            ChoiceOwner::Provider(provider) => self.provider(path, provider),
        }
    }
    pub(crate) fn scope(&mut self, path: &str, scope: &LoadoutScope) -> Result {
        if let LoadoutScope::Selected { loadouts } = scope {
            if loadouts.is_empty() {
                return Err(error(path, StructuralErrorKind::EmptyLoadoutScope));
            }
            self.collection(path, loadouts.len())?;
            let mut seen = BTreeSet::new();
            for id in loadouts {
                self.reference(path, *id, OccurrenceKind::Loadout)?;
                if !seen.insert(id) {
                    return Err(error(path, StructuralErrorKind::DuplicateAssignment));
                }
            }
        }
        Ok(())
    }
    fn register_item_records(&mut self, path: &str, items: &[ItemRecord]) -> Result {
        self.collection(path, items.len())?;
        for (i, item) in items.iter().enumerate() {
            self.register(&format!("{path}[{i}].id"), item.id, OccurrenceKind::Item)?;
        }
        for (i, item) in items.iter().enumerate() {
            let path = format!("{path}[{i}].modifiers");
            self.collection(&path, item.modifiers.len())?;
            for (j, modifier) in item.modifiers.iter().enumerate() {
                self.register(
                    &format!("{path}[{j}].id"),
                    modifier.id,
                    OccurrenceKind::Modifier,
                )?;
                self.seed_modifier_item(&format!("{path}[{j}]"), modifier.id, item.id)?;
            }
        }
        Ok(())
    }
    /// Validate precedence independently from canonical record order. The owner
    /// registry is shared by complete records and known draft members.
    pub(crate) fn modifier_order(
        &mut self,
        path: &str,
        item: ItemRecordId,
        order: &[ModifierInstanceId],
        member_count: usize,
    ) -> Result {
        self.collection(path, order.len())?;
        let mut seen = BTreeSet::new();
        for (i, modifier) in order.iter().enumerate() {
            let path = format!("{path}[{i}]");
            self.known_reference(&path, *modifier, OccurrenceKind::Modifier)?;
            if self.modifier_items.get(modifier) != Some(&item) {
                return Err(error(&path, StructuralErrorKind::WrongProviderOwner));
            }
            if !seen.insert(*modifier) {
                return Err(error(&path, StructuralErrorKind::DuplicateAssignment));
            }
        }
        if order.len() != member_count {
            return Err(error(path, StructuralErrorKind::InvalidModifierOrder));
        }
        Ok(())
    }
    fn item_record_values(&mut self, path: &str, items: &[ItemRecord]) -> Result {
        for (i, item) in items.iter().enumerate() {
            let path = format!("{path}[{i}]");
            self.definition(&path, &item.template)?;
            self.parameters(
                &format!("{path}.parameters"),
                &item.parameters,
                &SlotOwnerDefId::ItemTemplate(item.template.clone()),
            )?;
            self.quality(&path, &item.quality)?;
            self.modifier_order(
                &format!("{path}.modifier_order"),
                item.id,
                &item.modifier_order,
                item.modifiers.len(),
            )?;
            for (j, modifier) in item.modifiers.iter().enumerate() {
                let path = format!("{path}.modifiers[{j}]");
                self.definition(&path, &modifier.definition)?;
                self.parameters(
                    &format!("{path}.rolls"),
                    &modifier.rolls,
                    &SlotOwnerDefId::Modifier(modifier.definition.clone()),
                )?;
            }
        }
        Ok(())
    }
    fn register_tables(&mut self, tables: RecordTables<'_>) -> Result {
        self.begin_membership();
        // Register all definitions of occurrences before checking references, so
        // forward references work but domain-confused or dangling values do not.
        macro_rules! register {
            ($collection:expr, $path:literal, $kind:ident, $id:expr) => {{
                self.collection($path, $collection.len())?;
                for (i, value) in $collection.iter().enumerate() {
                    self.register(
                        &format!("{}[{i}].id", $path),
                        $id(value),
                        OccurrenceKind::$kind,
                    )?;
                }
            }};
        }
        register!(
            tables.weapon_loadouts,
            "build.weapon_loadouts",
            Loadout,
            |v: &WeaponLoadoutId| *v
        );
        register!(
            tables.rewards,
            "build.character.rewards",
            Reward,
            |v: &RewardSelection| v.id
        );
        self.register_item_records("build.items", tables.items)?;
        register!(tables.gems, "build.gems", Gem, |v: &GemInstance| v.id);
        register!(
            tables.equipment,
            "build.equipment",
            EquipmentUse,
            |v: &EquipmentUse| v.id
        );
        register!(
            tables.allocations,
            "build.allocations",
            Allocation,
            |v: &Allocation| v.id
        );
        register!(tables.skills, "build.skills", SkillUse, |v: &SkillUse| v.id);
        register!(
            tables.supports,
            "build.supports",
            SupportAssignment,
            |v: &SupportAssignment| v.id
        );
        register!(
            tables.payload_links,
            "build.payload_links",
            PayloadLink,
            |v: &PayloadLink| v.id
        );
        Ok(())
    }
    fn record_values(&mut self, tables: RecordTables<'_>) -> Result {
        for (i, reward) in tables.rewards.iter().enumerate() {
            let path = format!("build.character.rewards[{i}]");
            self.definition(&path, &reward.definition)?;
            self.parameters(
                &format!("{path}.parameters"),
                &reward.parameters,
                &SlotOwnerDefId::Reward(reward.definition.clone()),
            )?;
        }
        self.item_record_values("build.items", tables.items)?;
        for (i, gem) in tables.gems.iter().enumerate() {
            let path = format!("build.gems[{i}]");
            self.definition(&path, &gem.definition)?;
            self.parameters(
                &format!("{path}.parameters"),
                &gem.parameters,
                &SlotOwnerDefId::Gem(gem.definition.clone()),
            )?;
            self.quality(&path, &gem.quality)?;
        }
        for (i, equipment) in tables.equipment.iter().enumerate() {
            let path = format!("build.equipment[{i}]");
            self.reference(&path, equipment.item, OccurrenceKind::Item)?;
            self.scope(&path, &equipment.scope)?;
            match &equipment.destination {
                EquipmentDestination::CharacterSlot(slot) => self.definition(&path, slot)?,
                EquipmentDestination::ItemSocket { container, slot } => {
                    self.reference(&path, *container, OccurrenceKind::EquipmentUse)?;
                    self.definition(&path, slot)?;
                }
                EquipmentDestination::PassiveSocket { allocation, slot } => {
                    self.reference(&path, *allocation, OccurrenceKind::Allocation)?;
                    self.definition(&path, slot)?;
                }
            }
        }
        check_containment(tables.equipment)?;
        // Item references were checked above; preserve existing error ordering.
        for (i, equipment) in tables.equipment.iter().enumerate() {
            self.seed_equipment_item(
                &format!("build.equipment[{i}]"),
                equipment.id,
                equipment.item,
            )?;
        }
        for (i, allocation) in tables.allocations.iter().enumerate() {
            let path = format!("build.allocations[{i}]");
            self.definition(&path, &allocation.node)?;
            self.definition(&path, &allocation.pool)?;
            self.scope(&path, &allocation.scope)?;
            if let AllocationAccess::Granted(provider) = &allocation.access {
                self.provider(&path, provider)?;
            }
            self.collection(&path, allocation.choices.len())?;
            let mut assigned_choices = BTreeSet::new();
            for choice in &allocation.choices {
                self.choice(&path, choice)?;
                if !assigned_choices.insert(&choice.slot) {
                    return Err(error(&path, StructuralErrorKind::DuplicateAssignment));
                }
            }
        }
        for (i, skill) in tables.skills.iter().enumerate() {
            let path = format!("build.skills[{i}]");
            self.scope(&path, &skill.scope)?;
            match &skill.source {
                AuthoredSkillSource::Gem(id) => self.reference(&path, *id, OccurrenceKind::Gem)?,
                AuthoredSkillSource::Direct(id) => self.definition(&path, id)?,
            }
        }
        for (i, support) in tables.supports.iter().enumerate() {
            let path = format!("build.supports[{i}]");
            self.reference(&path, support.support, OccurrenceKind::Gem)?;
            self.skill(&path, &support.target)?;
        }
        for (i, link) in tables.payload_links.iter().enumerate() {
            let path = format!("build.payload_links[{i}]");
            self.reference(&path, link.container, OccurrenceKind::SkillUse)?;
            self.reference(&path, link.payload, OccurrenceKind::SkillUse)?;
            self.definition(&path, &link.role)?;
            // Whether a role is containment or a trigger is definition semantics.
        }
        Ok(())
    }
    fn mechanic_choices(
        &mut self,
        choices: &[MechanicChoice],
        mut assigned_choices: BTreeSet<(ChoiceOwner, DeclaredSlot<ChoiceSlotDefId>)>,
    ) -> Result {
        self.collection("build.choices", choices.len())?;
        for (i, choice) in choices.iter().enumerate() {
            let path = format!("build.choices[{i}]");
            self.choice_owner(&path, &choice.owner)?;
            self.choice(&path, &choice.choice)?;
            if !assigned_choices.insert((
                canonical_choice_owner(&choice.owner),
                choice.choice.slot.clone(),
            )) {
                return Err(error(&path, StructuralErrorKind::DuplicateAssignment));
            }
        }
        Ok(())
    }
    fn build(&mut self, build: &BuildInput) -> Result {
        let tables = RecordTables::from_build(build);
        self.register_tables(tables)?;
        self.reference(
            "build.active_weapon_loadout",
            build.active_weapon_loadout,
            OccurrenceKind::Loadout,
        )?;
        self.definition("build.character.class", &build.character.class)?;
        if let Some(ascendancy) = &build.character.ascendancy {
            self.definition("build.character.ascendancy", ascendancy)?;
        }
        self.record_values(tables)?;
        let assigned_choices = build
            .allocations
            .iter()
            .flat_map(|allocation| {
                allocation.choices.iter().map(move |choice| {
                    (ChoiceOwner::Allocation(allocation.id), choice.slot.clone())
                })
            })
            .collect();
        self.mechanic_choices(&build.choices, assigned_choices)
    }
    fn scenario(&mut self, scenario: &ScenarioInput) -> Result {
        self.namespace("scenario.game_version", &scenario.game_version)?;
        self.definition("scenario.enemy.encounter", &scenario.enemy.encounter)?;
        self.collection("scenario.assumptions", scenario.assumptions.len())?;
        let mut seen = BTreeSet::new();
        for (i, assumption) in scenario.assumptions.iter().enumerate() {
            let path = format!("scenario.assumptions[{i}]");
            if !seen.insert((&assumption.target, &assumption.input)) {
                return Err(error(&path, StructuralErrorKind::DuplicateAssignment));
            }
            self.definition(&path, &assumption.input)?;
            self.value(&path, &assumption.value)?;
            match &assumption.target {
                AssumptionTarget::Environment | AssumptionTarget::Enemy => {}
                AssumptionTarget::Actor(actor) => self.actor(&path, actor)?,
                AssumptionTarget::Skill(skill) => self.skill(&path, skill)?,
            }
        }
        self.collection("scenario.usage", scenario.usage.len())?;
        let mut seen = BTreeSet::new();
        for (i, usage) in scenario.usage.iter().enumerate() {
            let path = format!("scenario.usage[{i}]");
            if !seen.insert((&usage.target, &usage.policy)) {
                return Err(error(&path, StructuralErrorKind::DuplicateAssignment));
            }
            self.definition(&path, &usage.policy)?;
            self.parameters(
                &format!("{path}.parameters"),
                &usage.parameters,
                &SlotOwnerDefId::UsagePolicy(usage.policy.clone()),
            )?;
            match &usage.target {
                UsageTarget::Actor(actor) => self.actor(&path, actor)?,
                UsageTarget::Skill(skill) => self.skill(&path, skill)?,
                UsageTarget::Action(action) => self.action(&path, action)?,
            }
        }
        Ok(())
    }
    fn queries(&mut self, queries: &QueryInput) -> Result {
        self.namespace("queries.game_version", &queries.game_version)?;
        self.collection("queries.requests", queries.requests.len())?;
        let mut seen = BTreeSet::new();
        for (i, request) in queries.requests.iter().enumerate() {
            let path = format!("queries.requests[{i}]");
            if !seen.insert(&request.id) {
                return Err(error(&path, StructuralErrorKind::DuplicateAssignment));
            }
            self.definition(&path, &request.metric)?;
            match &request.target {
                MetricTarget::Actor(actor) => self.actor(&path, actor)?,
                MetricTarget::Action(action) => self.action(&path, action)?,
            }
        }
        Ok(())
    }
}

fn check_containment(equipment: &[EquipmentUse]) -> Result {
    let parents: BTreeMap<_, _> = equipment
        .iter()
        .filter_map(|item| match item.destination {
            EquipmentDestination::ItemSocket { container, .. } => Some((item.id, container)),
            _ => None,
        })
        .collect();
    let mut complete = BTreeSet::new();
    for start in parents.keys() {
        let mut path = BTreeSet::new();
        let mut cursor = *start;
        while !complete.contains(&cursor) {
            if !path.insert(cursor) {
                return Err(error(
                    "build.equipment.destination",
                    StructuralErrorKind::ContainmentCycle,
                ));
            }
            let Some(parent) = parents.get(&cursor) else {
                break;
            };
            cursor = *parent;
        }
        complete.extend(path);
    }
    Ok(())
}

/// Shared structural checks for self-contained stock/project record tables.
/// Returns consumed collection entries so enclosing documents can preserve one budget.
pub(crate) fn validate_item_records(
    namespace: &GameVersionNamespace,
    allocator: InstanceAllocatorState,
    items: &[ItemRecord],
    additional_occurrences: &[(InstanceId, OccurrenceKind)],
    limits: OwnedInputLimits,
) -> Result<usize> {
    let mut check = StructuralCheck::new(namespace, limits, Some(allocator))?;
    check.begin_membership();
    check.register_item_records("items", items)?;
    // Metadata combines real collections; each caller bounds its own collection.
    if additional_occurrences.len() > check.remaining {
        return Err(error(
            "additional_occurrences",
            StructuralErrorKind::LimitExceeded,
        ));
    }
    check.remaining -= additional_occurrences.len();
    for (i, (id, kind)) in additional_occurrences.iter().enumerate() {
        check.register(&format!("additional_occurrences[{i}]"), *id, *kind)?;
    }
    check.item_record_values("items", items)?;
    Ok(limits.max_entries - check.remaining)
}

pub(crate) fn build_occurrences(
    build: &super::BuildSpec,
    limits: OwnedInputLimits,
) -> Result<Vec<(InstanceId, OccurrenceKind)>> {
    let input = build.input();
    let mut check = StructuralCheck::new(&input.game_version, limits, Some(input.allocator))?;
    check.build(input)?;
    Ok(check
        .members
        .expect("registered build occurrences")
        .into_iter()
        .collect())
}

pub(crate) fn validate_build(build: &BuildInput, limits: OwnedInputLimits) -> Result {
    StructuralCheck::new(&build.game_version, limits, Some(build.allocator))?.build(build)
}
pub(crate) fn validate_scenario(
    scenario: &ScenarioInput,
    limits: OwnedInputLimits,
    allocator: Option<InstanceAllocatorState>,
) -> Result {
    StructuralCheck::new(&scenario.game_version, limits, allocator)?.scenario(scenario)
}
pub(crate) fn validate_queries(
    queries: &QueryInput,
    limits: OwnedInputLimits,
    allocator: Option<InstanceAllocatorState>,
) -> Result {
    StructuralCheck::new(&queries.game_version, limits, allocator)?.queries(queries)
}
pub(crate) fn validate_request(
    build: &BuildInput,
    scenario: &ScenarioInput,
    queries: &QueryInput,
    limits: OwnedInputLimits,
) -> Result {
    let mut check = StructuralCheck::new(&build.game_version, limits, Some(build.allocator))?;
    check.build(build)?;
    // Saved selectors can refer to removed occurrences. Keep lineage/watermark and
    // shape checks; existence/effective-loadout availability binds at resolution.
    check.with_query_references(|check| {
        check.scenario(scenario)?;
        check.queries(queries)
    })
}

fn canonicalize_scope(scope: &mut LoadoutScope) {
    if let LoadoutScope::Selected { loadouts } = scope {
        loadouts.sort();
    }
}
fn canonicalize_parameters(parameters: &mut [ParameterAssignment]) {
    parameters.sort_by(|a, b| a.slot.cmp(&b.slot));
}
pub(crate) fn canonicalize_record_tables(tables: RecordTablesMut<'_>) {
    tables.weapon_loadouts.sort();
    tables.rewards.sort_by_key(|v| v.id);
    for reward in tables.rewards {
        canonicalize_parameters(&mut reward.parameters);
    }
    canonicalize_item_records(tables.items);
    tables.gems.sort_by_key(|v| v.id);
    for gem in tables.gems {
        canonicalize_parameters(&mut gem.parameters);
    }
    tables.equipment.sort_by_key(|v| v.id);
    for item in tables.equipment {
        canonicalize_scope(&mut item.scope);
    }
    tables.allocations.sort_by_key(|v| v.id);
    for allocation in tables.allocations {
        canonicalize_scope(&mut allocation.scope);
        allocation.choices.sort_by(|a, b| a.slot.cmp(&b.slot));
    }
    tables.skills.sort_by_key(|v| v.id);
    for skill in tables.skills {
        canonicalize_scope(&mut skill.scope);
    }
    tables.supports.sort_by_key(|v| v.id);
    tables.payload_links.sort_by_key(|v| v.id);
}
/// Logical choice identity only. Preserve the authored owner in serialized input.
/// Empty direct-provider aliases denote one port; paths/actions/generated selectors
/// and all other provider roots remain distinct semantic addresses.
pub(crate) fn canonical_choice_owner(owner: &ChoiceOwner) -> ChoiceOwner {
    if let ChoiceOwner::Provider(provider) = owner
        && provider.grant_path.is_empty()
    {
        match provider.root {
            ProviderRoot::Character => return ChoiceOwner::Character,
            ProviderRoot::EquipmentUse(id) => return ChoiceOwner::EquipmentUse(id),
            ProviderRoot::Allocation(id) => return ChoiceOwner::Allocation(id),
            ProviderRoot::SkillUse(id) => return ChoiceOwner::Skill(SkillTarget::Authored(id)),
            _ => {}
        }
    }
    owner.clone()
}
pub(crate) fn canonicalize_choices(choices: &mut [MechanicChoice]) {
    choices.sort_by(|a, b| (&a.owner, &a.choice.slot).cmp(&(&b.owner, &b.choice.slot)));
}
pub(crate) fn canonicalize_build(build: &mut BuildInput) {
    canonicalize_record_tables(RecordTablesMut {
        weapon_loadouts: &mut build.weapon_loadouts,
        rewards: &mut build.character.rewards,
        items: &mut build.items,
        gems: &mut build.gems,
        equipment: &mut build.equipment,
        allocations: &mut build.allocations,
        skills: &mut build.skills,
        supports: &mut build.supports,
        payload_links: &mut build.payload_links,
    });
    canonicalize_choices(&mut build.choices);
}
pub(crate) fn canonicalize_scenario(scenario: &mut ScenarioInput) {
    scenario
        .assumptions
        .sort_by(|a, b| (&a.target, &a.input).cmp(&(&b.target, &b.input)));
    scenario
        .usage
        .sort_by(|a, b| (&a.target, &a.policy).cmp(&(&b.target, &b.policy)));
    for usage in &mut scenario.usage {
        canonicalize_parameters(&mut usage.parameters);
    }
}

pub(crate) fn canonicalize_item_records(items: &mut [ItemRecord]) {
    items.sort_by_key(|v| v.id);
    for item in items {
        canonicalize_parameters(&mut item.parameters);
        item.modifiers.sort_by_key(|v| v.id);
        for modifier in &mut item.modifiers {
            canonicalize_parameters(&mut modifier.rolls);
        }
    }
}
