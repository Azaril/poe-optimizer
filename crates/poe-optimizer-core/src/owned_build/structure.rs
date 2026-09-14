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

struct Check<'a> {
    namespace: &'a GameVersionNamespace,
    allocator: Option<InstanceAllocatorState>,
    members: Option<BTreeMap<InstanceId, OccurrenceKind>>,
    remaining: usize,
    allow_missing_references: bool,
    modifier_items: BTreeMap<ModifierInstanceId, ItemRecordId>,
    equipment_items: BTreeMap<ItemSlotUseId, ItemRecordId>,
    limits: OwnedInputLimits,
}
impl<'a> Check<'a> {
    fn new(
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
    fn collection(&mut self, path: &str, len: usize) -> Result {
        if len > self.limits.max_collection_entries || len > self.remaining {
            return Err(error(path, StructuralErrorKind::LimitExceeded));
        }
        self.remaining -= len;
        Ok(())
    }
    fn namespace(&self, path: &str, namespace: &GameVersionNamespace) -> Result {
        if namespace != self.namespace {
            return Err(error(path, StructuralErrorKind::ForeignNamespace));
        }
        Ok(())
    }
    fn definition<K: DefinitionDomain>(&self, path: &str, id: &DefId<K>) -> Result {
        self.namespace(path, id.namespace())
    }
    fn slot<K: DefinitionDomain>(&self, path: &str, slot: &DeclaredSlot<DefId<K>>) -> Result {
        self.namespace(path, slot.declaration.namespace())?;
        self.definition(path, &slot.slot)
    }
    fn identity(&self, path: &str, id: InstanceId) -> Result {
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
    fn register(&mut self, path: &str, id: impl BuildInstanceId, kind: OccurrenceKind) -> Result {
        let id = id.instance_id();
        self.identity(path, id)?;
        if self
            .members
            .as_mut()
            .expect("build membership collection")
            .insert(id, kind)
            .is_some()
        {
            return Err(error(path, StructuralErrorKind::DuplicateIdentity { id }));
        }
        Ok(())
    }
    fn reference(&self, path: &str, id: impl BuildInstanceId, expected: OccurrenceKind) -> Result {
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
    fn provider(&mut self, path: &str, key: &ProviderKey) -> Result {
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
    fn actor(&mut self, path: &str, actor: &ActorKey) -> Result {
        if let ActorKey::Owned(actor) = actor {
            self.provider(path, &actor.provider)?;
            self.slot(path, &actor.slot)?;
        }
        Ok(())
    }
    fn skill(&mut self, path: &str, target: &SkillTarget) -> Result {
        match target {
            SkillTarget::Authored(id) => self.reference(path, *id, OccurrenceKind::SkillUse),
            SkillTarget::Generated(key) => {
                self.provider(path, &key.provider)?;
                self.slot(path, &key.slot)
            }
        }
    }
    fn action(&mut self, path: &str, action: &ActionSelection) -> Result {
        self.actor(path, &action.action.actor)?;
        self.provider(path, &action.action.provider)?;
        self.slot(path, &action.action.output)?;
        self.definition(path, &action.part)?;
        self.definition(path, &action.mode)?;
        self.definition(path, &action.stat_set)
    }
    fn value(&self, path: &str, value: &ParameterValue) -> Result {
        match value {
            ParameterValue::Boolean(_) | ParameterValue::Integer(_) => Ok(()),
            ParameterValue::Quantity(value) => self.definition(path, value.unit()),
            ParameterValue::Option(value) => self.definition(path, value),
        }
    }
    fn quality(&self, path: &str, quality: &Option<QualitySelection>) -> Result {
        if let Some(quality) = quality {
            self.definition(path, &quality.kind)?;
            self.definition(path, quality.amount.unit())?;
        }
        Ok(())
    }
    fn parameters(
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
    fn choice(&self, path: &str, choice: &ChoiceSelection) -> Result {
        self.slot(path, &choice.slot)?;
        self.value(path, &choice.value)
    }
    fn choice_owner(&mut self, path: &str, owner: &ChoiceOwner) -> Result {
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
    fn scope(&mut self, path: &str, scope: &LoadoutScope) -> Result {
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
            }
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
    fn build(&mut self, build: &BuildInput) -> Result {
        self.members = Some(BTreeMap::new());
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
            &build.weapon_loadouts,
            "build.weapon_loadouts",
            Loadout,
            |v: &WeaponLoadoutId| *v
        );
        register!(
            &build.character.rewards,
            "build.character.rewards",
            Reward,
            |v: &RewardSelection| v.id
        );
        self.register_item_records("build.items", &build.items)?;
        register!(&build.gems, "build.gems", Gem, |v: &GemInstance| v.id);
        register!(
            &build.equipment,
            "build.equipment",
            EquipmentUse,
            |v: &EquipmentUse| v.id
        );
        register!(
            &build.allocations,
            "build.allocations",
            Allocation,
            |v: &Allocation| v.id
        );
        register!(&build.skills, "build.skills", SkillUse, |v: &SkillUse| v.id);
        register!(
            &build.supports,
            "build.supports",
            SupportAssignment,
            |v: &SupportAssignment| v.id
        );
        register!(
            &build.payload_links,
            "build.payload_links",
            PayloadLink,
            |v: &PayloadLink| v.id
        );
        self.modifier_items = build
            .items
            .iter()
            .flat_map(|item| {
                item.modifiers
                    .iter()
                    .map(move |modifier| (modifier.id, item.id))
            })
            .collect();
        self.equipment_items = build
            .equipment
            .iter()
            .map(|equipment| (equipment.id, equipment.item))
            .collect();
        self.reference(
            "build.active_weapon_loadout",
            build.active_weapon_loadout,
            OccurrenceKind::Loadout,
        )?;
        self.definition("build.character.class", &build.character.class)?;
        if let Some(ascendancy) = &build.character.ascendancy {
            self.definition("build.character.ascendancy", ascendancy)?;
        }
        for (i, reward) in build.character.rewards.iter().enumerate() {
            let path = format!("build.character.rewards[{i}]");
            self.definition(&path, &reward.definition)?;
            self.parameters(
                &format!("{path}.parameters"),
                &reward.parameters,
                &SlotOwnerDefId::Reward(reward.definition.clone()),
            )?;
        }
        self.item_record_values("build.items", &build.items)?;
        for (i, gem) in build.gems.iter().enumerate() {
            let path = format!("build.gems[{i}]");
            self.definition(&path, &gem.definition)?;
            self.parameters(
                &format!("{path}.parameters"),
                &gem.parameters,
                &SlotOwnerDefId::Gem(gem.definition.clone()),
            )?;
            self.quality(&path, &gem.quality)?;
        }
        for (i, equipment) in build.equipment.iter().enumerate() {
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
        check_containment(&build.equipment)?;
        let mut assigned_choices = BTreeSet::new();
        for (i, allocation) in build.allocations.iter().enumerate() {
            let path = format!("build.allocations[{i}]");
            self.definition(&path, &allocation.node)?;
            self.definition(&path, &allocation.pool)?;
            self.scope(&path, &allocation.scope)?;
            if let AllocationAccess::Granted(provider) = &allocation.access {
                self.provider(&path, provider)?;
            }
            self.collection(&path, allocation.choices.len())?;
            for choice in &allocation.choices {
                self.choice(&path, choice)?;
                if !assigned_choices.insert((ChoiceOwner::Allocation(allocation.id), &choice.slot))
                {
                    return Err(error(&path, StructuralErrorKind::DuplicateAssignment));
                }
            }
        }
        for (i, skill) in build.skills.iter().enumerate() {
            let path = format!("build.skills[{i}]");
            self.scope(&path, &skill.scope)?;
            match &skill.source {
                AuthoredSkillSource::Gem(id) => self.reference(&path, *id, OccurrenceKind::Gem)?,
                AuthoredSkillSource::Direct(id) => self.definition(&path, id)?,
            }
        }
        for (i, support) in build.supports.iter().enumerate() {
            let path = format!("build.supports[{i}]");
            self.reference(&path, support.support, OccurrenceKind::Gem)?;
            self.skill(&path, &support.target)?;
        }
        for (i, link) in build.payload_links.iter().enumerate() {
            let path = format!("build.payload_links[{i}]");
            self.reference(&path, link.container, OccurrenceKind::SkillUse)?;
            self.reference(&path, link.payload, OccurrenceKind::SkillUse)?;
            self.definition(&path, &link.role)?;
            // Whether a role is containment or a trigger is definition semantics.
        }
        self.collection("build.choices", build.choices.len())?;
        for (i, choice) in build.choices.iter().enumerate() {
            let path = format!("build.choices[{i}]");
            self.choice_owner(&path, &choice.owner)?;
            self.choice(&path, &choice.choice)?;
            if !assigned_choices.insert((choice.owner.clone(), &choice.choice.slot)) {
                return Err(error(&path, StructuralErrorKind::DuplicateAssignment));
            }
        }
        Ok(())
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
    let mut check = Check::new(namespace, limits, Some(allocator))?;
    check.members = Some(BTreeMap::new());
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
    let mut check = Check::new(&input.game_version, limits, Some(input.allocator))?;
    check.build(input)?;
    Ok(check
        .members
        .expect("registered build occurrences")
        .into_iter()
        .collect())
}

pub(crate) fn validate_build(build: &BuildInput, limits: OwnedInputLimits) -> Result {
    Check::new(&build.game_version, limits, Some(build.allocator))?.build(build)
}
pub(crate) fn validate_scenario(
    scenario: &ScenarioInput,
    limits: OwnedInputLimits,
    allocator: Option<InstanceAllocatorState>,
) -> Result {
    Check::new(&scenario.game_version, limits, allocator)?.scenario(scenario)
}
pub(crate) fn validate_queries(
    queries: &QueryInput,
    limits: OwnedInputLimits,
    allocator: Option<InstanceAllocatorState>,
) -> Result {
    Check::new(&queries.game_version, limits, allocator)?.queries(queries)
}
pub(crate) fn validate_request(
    build: &BuildInput,
    scenario: &ScenarioInput,
    queries: &QueryInput,
    limits: OwnedInputLimits,
) -> Result {
    let mut check = Check::new(&build.game_version, limits, Some(build.allocator))?;
    check.build(build)?;
    // Saved selectors can refer to removed occurrences. Keep lineage/watermark and
    // shape checks; existence/effective-loadout availability binds at resolution.
    check.allow_missing_references = true;
    check.scenario(scenario)?;
    check.queries(queries)
}

fn canonicalize_scope(scope: &mut LoadoutScope) {
    if let LoadoutScope::Selected { loadouts } = scope {
        loadouts.sort();
    }
}
fn canonicalize_parameters(parameters: &mut [ParameterAssignment]) {
    parameters.sort_by(|a, b| a.slot.cmp(&b.slot));
}
pub(crate) fn canonicalize_build(build: &mut BuildInput) {
    build.weapon_loadouts.sort();
    build.character.rewards.sort_by_key(|v| v.id);
    for reward in &mut build.character.rewards {
        canonicalize_parameters(&mut reward.parameters);
    }
    canonicalize_item_records(&mut build.items);
    build.gems.sort_by_key(|v| v.id);
    for gem in &mut build.gems {
        canonicalize_parameters(&mut gem.parameters);
    }
    build.equipment.sort_by_key(|v| v.id);
    for item in &mut build.equipment {
        canonicalize_scope(&mut item.scope);
    }
    build.allocations.sort_by_key(|v| v.id);
    for allocation in &mut build.allocations {
        canonicalize_scope(&mut allocation.scope);
        allocation.choices.sort_by(|a, b| a.slot.cmp(&b.slot));
    }
    build.skills.sort_by_key(|v| v.id);
    for skill in &mut build.skills {
        canonicalize_scope(&mut skill.scope);
    }
    build.supports.sort_by_key(|v| v.id);
    build.payload_links.sort_by_key(|v| v.id);
    build
        .choices
        .sort_by(|a, b| (&a.owner, &a.choice.slot).cmp(&(&b.owner, &b.choice.slot)));
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
