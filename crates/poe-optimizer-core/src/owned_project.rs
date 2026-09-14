//! Independent saved presets over one self-contained owned occurrence registry.
//!
//! Composition selects explicit IDs; it never pairs positions, changes supplying
//! occurrences, adopts external records, assigns stock, or fills semantic defaults.
//! Saved scenarios/queries, unresolved drafts and revisioned edits are separate work.
use crate::{
    build_identity::*,
    owned_build::{self, *},
    owned_definitions::*,
    owned_inventory::{InventoryError, InventorySnapshot},
};
use serde::{Deserialize, Deserializer, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

fn required_option<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    Option::<T>::deserialize(d)
}

/// No preset ID is a source-set ID or an index into another preset collection.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterPreset {
    pub id: CharacterPresetId,
    pub class: ClassDefId,
    #[serde(deserialize_with = "required_option")]
    pub ascendancy: Option<AscendancyDefId>,
    pub level: u16,
    pub rewards: Vec<RewardSelectionId>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EquipmentPreset {
    pub id: EquipmentPresetId,
    pub equipment: Vec<ItemSlotUseId>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllocationPreset {
    pub id: AllocationPresetId,
    pub allocations: Vec<AllocationId>,
    /// Receiving occurrences contributed alongside this allocation selection.
    pub equipment: Vec<ItemSlotUseId>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillPreset {
    pub id: SkillPresetId,
    pub skills: Vec<SkillUseId>,
    pub supports: Vec<SupportAssignmentId>,
    pub payload_links: Vec<PayloadLinkId>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChoicePreset {
    pub id: ChoicePresetId,
    pub choices: Vec<MechanicChoice>,
    /// Explicit selected reward occurrences, additive with character rewards.
    pub rewards: Vec<RewardSelectionId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VariantSelection {
    pub character: CharacterPresetId,
    pub equipment: EquipmentPresetId,
    pub allocations: AllocationPresetId,
    pub skills: SkillPresetId,
    pub choices: ChoicePresetId,
    pub active_weapon_loadout: WeaponLoadoutId,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedVariant {
    pub id: SavedVariantId,
    pub selection: VariantSelection,
}

/// Raw DTO. Every occurrence is stored once; presets only select that occurrence.
/// Inventory-only records must be explicitly adopted before project construction.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectInput {
    pub allocator: InstanceAllocatorState,
    pub revision: BuildRevision,
    pub game_version: GameVersionNamespace,
    pub weapon_loadouts: Vec<WeaponLoadoutId>,
    pub items: Vec<ItemRecord>,
    pub gems: Vec<GemInstance>,
    pub rewards: Vec<RewardSelection>,
    pub equipment: Vec<EquipmentUse>,
    pub allocations: Vec<Allocation>,
    pub skills: Vec<SkillUse>,
    pub supports: Vec<SupportAssignment>,
    pub payload_links: Vec<PayloadLink>,
    pub character_presets: Vec<CharacterPreset>,
    pub equipment_presets: Vec<EquipmentPreset>,
    pub allocation_presets: Vec<AllocationPreset>,
    pub skill_presets: Vec<SkillPreset>,
    pub choice_presets: Vec<ChoicePreset>,
    pub saved_variants: Vec<SavedVariant>,
}
impl ProjectInput {
    fn tables(&self) -> owned_build::RecordTables<'_> {
        owned_build::RecordTables {
            weapon_loadouts: &self.weapon_loadouts,
            rewards: &self.rewards,
            items: &self.items,
            gems: &self.gems,
            equipment: &self.equipment,
            allocations: &self.allocations,
            skills: &self.skills,
            supports: &self.supports,
            payload_links: &self.payload_links,
        }
    }
}

/// Canonical, structurally coherent registry. A combination can still fail to
/// compose if independently selected presets omit a required supplying occurrence.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct BuildProject(ProjectInput);
impl BuildProject {
    pub fn new(mut input: ProjectInput, limits: OwnedInputLimits) -> Result<Self, ProjectError> {
        validate_project(&input, limits)?;
        canonicalize_project(&mut input);
        Ok(Self(input))
    }
    pub fn input(&self) -> &ProjectInput {
        &self.0
    }
    pub fn into_input(self) -> ProjectInput {
        self.0
    }
    pub fn validate_limits(&self, limits: OwnedInputLimits) -> Result<(), ProjectError> {
        validate_project(&self.0, limits).map(|_| ())
    }
    pub fn saved_variant(&self, id: SavedVariantId) -> Option<&SavedVariant> {
        self.0
            .saved_variants
            .binary_search_by_key(&id, |entry| entry.id)
            .ok()
            .map(|index| &self.0.saved_variants[index])
    }
}

#[derive(Debug)]
pub enum ProjectError {
    Structure(StructuralError),
    Inventory(InventoryError),
}
impl fmt::Display for ProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Structure(error) => error.fmt(f),
            Self::Inventory(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for ProjectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Structure(error) => Some(error),
            Self::Inventory(error) => Some(error),
        }
    }
}
impl From<StructuralError> for ProjectError {
    fn from(value: StructuralError) -> Self {
        Self::Structure(value)
    }
}
impl From<InventoryError> for ProjectError {
    fn from(value: InventoryError) -> Self {
        Self::Inventory(value)
    }
}
fn error(path: &str, kind: StructuralErrorKind) -> ProjectError {
    StructuralError {
        path: path.into(),
        kind,
    }
    .into()
}
fn collection(
    path: &str,
    len: usize,
    used: &mut usize,
    limits: OwnedInputLimits,
) -> Result<(), ProjectError> {
    if len > limits.max_collection_entries || len > limits.max_entries - *used {
        return Err(error(path, StructuralErrorKind::LimitExceeded));
    }
    *used += len;
    Ok(())
}
fn reference(
    path: &str,
    id: InstanceId,
    expected: OccurrenceKind,
    allocator: InstanceAllocatorState,
    members: &BTreeMap<InstanceId, OccurrenceKind>,
) -> Result<(), ProjectError> {
    if id.lineage() != allocator.lineage() {
        return Err(error(path, StructuralErrorKind::ForeignLineage));
    }
    if id.local() > allocator.last_issued() {
        return Err(error(path, StructuralErrorKind::BeyondWatermark));
    }
    if members.get(&id) != Some(&expected) {
        return Err(error(
            path,
            StructuralErrorKind::MissingReference { expected, id },
        ));
    }
    Ok(())
}
fn references<T: BuildInstanceId + Ord>(
    path: &str,
    ids: &[T],
    expected: OccurrenceKind,
    allocator: InstanceAllocatorState,
    members: &BTreeMap<InstanceId, OccurrenceKind>,
) -> Result<(), ProjectError> {
    let mut seen = BTreeSet::new();
    for id in ids {
        reference(path, id.instance_id(), expected, allocator, members)?;
        if !seen.insert(*id) {
            return Err(error(path, StructuralErrorKind::DuplicateAssignment));
        }
    }
    Ok(())
}
fn validate_selection(
    selection: &VariantSelection,
    input: &ProjectInput,
    members: &BTreeMap<InstanceId, OccurrenceKind>,
) -> Result<(), ProjectError> {
    for (path, id, kind) in [
        (
            "selection.character",
            selection.character.instance_id(),
            OccurrenceKind::CharacterPreset,
        ),
        (
            "selection.equipment",
            selection.equipment.instance_id(),
            OccurrenceKind::EquipmentPreset,
        ),
        (
            "selection.allocations",
            selection.allocations.instance_id(),
            OccurrenceKind::AllocationPreset,
        ),
        (
            "selection.skills",
            selection.skills.instance_id(),
            OccurrenceKind::SkillPreset,
        ),
        (
            "selection.choices",
            selection.choices.instance_id(),
            OccurrenceKind::ChoicePreset,
        ),
        (
            "selection.active_weapon_loadout",
            selection.active_weapon_loadout.instance_id(),
            OccurrenceKind::Loadout,
        ),
    ] {
        reference(path, id, kind, input.allocator, members)?;
    }
    Ok(())
}
fn validate_project(
    input: &ProjectInput,
    limits: OwnedInputLimits,
) -> Result<Vec<(InstanceId, OccurrenceKind)>, ProjectError> {
    limits.validate()?;
    let mut additional = Vec::new();
    let mut preset_entries = 0;
    macro_rules! presets {
        ($field:ident, $kind:ident) => {{
            collection(
                concat!("project.", stringify!($field)),
                input.$field.len(),
                &mut preset_entries,
                limits,
            )?;
            additional.extend(
                input
                    .$field
                    .iter()
                    .map(|preset| (preset.id.instance_id(), OccurrenceKind::$kind)),
            );
        }};
    }
    presets!(character_presets, CharacterPreset);
    presets!(equipment_presets, EquipmentPreset);
    presets!(allocation_presets, AllocationPreset);
    presets!(skill_presets, SkillPreset);
    presets!(choice_presets, ChoicePreset);
    presets!(saved_variants, SavedVariant);
    // Check actual preset collections before constructing shared membership metadata.
    let mut reference_entries = 0;
    for preset in &input.character_presets {
        collection(
            "character_presets.rewards",
            preset.rewards.len(),
            &mut reference_entries,
            limits,
        )?;
    }
    for preset in &input.equipment_presets {
        collection(
            "equipment_presets.equipment",
            preset.equipment.len(),
            &mut reference_entries,
            limits,
        )?;
    }
    for preset in &input.allocation_presets {
        collection(
            "allocation_presets.allocations",
            preset.allocations.len(),
            &mut reference_entries,
            limits,
        )?;
        collection(
            "allocation_presets.equipment",
            preset.equipment.len(),
            &mut reference_entries,
            limits,
        )?;
    }
    for preset in &input.skill_presets {
        collection(
            "skill_presets.skills",
            preset.skills.len(),
            &mut reference_entries,
            limits,
        )?;
        collection(
            "skill_presets.supports",
            preset.supports.len(),
            &mut reference_entries,
            limits,
        )?;
        collection(
            "skill_presets.payload_links",
            preset.payload_links.len(),
            &mut reference_entries,
            limits,
        )?;
    }
    for preset in &input.choice_presets {
        collection(
            "choice_presets.rewards",
            preset.rewards.len(),
            &mut reference_entries,
            limits,
        )?;
    }
    let groups: Vec<_> = input
        .choice_presets
        .iter()
        .map(|preset| preset.choices.as_slice())
        .collect();
    let validated = owned_build::validate_record_tables(
        &input.game_version,
        input.allocator,
        input.tables(),
        &groups,
        &additional,
        limits,
    )?;
    if reference_entries > limits.max_entries - validated.entries {
        return Err(error("project", StructuralErrorKind::LimitExceeded));
    }
    let members: BTreeMap<_, _> = validated.occurrences.iter().copied().collect();
    for preset in &input.character_presets {
        if preset.class.namespace() != &input.game_version
            || preset
                .ascendancy
                .as_ref()
                .is_some_and(|id| id.namespace() != &input.game_version)
        {
            return Err(error(
                "character_presets",
                StructuralErrorKind::ForeignNamespace,
            ));
        }
        references(
            "character_presets.rewards",
            &preset.rewards,
            OccurrenceKind::Reward,
            input.allocator,
            &members,
        )?;
    }
    for preset in &input.equipment_presets {
        references(
            "equipment_presets.equipment",
            &preset.equipment,
            OccurrenceKind::EquipmentUse,
            input.allocator,
            &members,
        )?;
    }
    for preset in &input.allocation_presets {
        references(
            "allocation_presets.allocations",
            &preset.allocations,
            OccurrenceKind::Allocation,
            input.allocator,
            &members,
        )?;
        references(
            "allocation_presets.equipment",
            &preset.equipment,
            OccurrenceKind::EquipmentUse,
            input.allocator,
            &members,
        )?;
    }
    for preset in &input.skill_presets {
        references(
            "skill_presets.skills",
            &preset.skills,
            OccurrenceKind::SkillUse,
            input.allocator,
            &members,
        )?;
        references(
            "skill_presets.supports",
            &preset.supports,
            OccurrenceKind::SupportAssignment,
            input.allocator,
            &members,
        )?;
        references(
            "skill_presets.payload_links",
            &preset.payload_links,
            OccurrenceKind::PayloadLink,
            input.allocator,
            &members,
        )?;
    }
    for preset in &input.choice_presets {
        references(
            "choice_presets.rewards",
            &preset.rewards,
            OccurrenceKind::Reward,
            input.allocator,
            &members,
        )?;
    }
    for variant in &input.saved_variants {
        validate_selection(&variant.selection, input, &members)?;
    }
    Ok(validated.occurrences)
}
fn canonicalize_project(input: &mut ProjectInput) {
    owned_build::canonicalize_record_tables(owned_build::RecordTablesMut {
        weapon_loadouts: &mut input.weapon_loadouts,
        rewards: &mut input.rewards,
        items: &mut input.items,
        gems: &mut input.gems,
        equipment: &mut input.equipment,
        allocations: &mut input.allocations,
        skills: &mut input.skills,
        supports: &mut input.supports,
        payload_links: &mut input.payload_links,
    });
    input.character_presets.sort_by_key(|preset| preset.id);
    for preset in &mut input.character_presets {
        preset.rewards.sort();
    }
    input.equipment_presets.sort_by_key(|preset| preset.id);
    for preset in &mut input.equipment_presets {
        preset.equipment.sort();
    }
    input.allocation_presets.sort_by_key(|preset| preset.id);
    for preset in &mut input.allocation_presets {
        preset.allocations.sort();
        preset.equipment.sort();
    }
    input.skill_presets.sort_by_key(|preset| preset.id);
    for preset in &mut input.skill_presets {
        preset.skills.sort();
        preset.supports.sort();
        preset.payload_links.sort();
    }
    input.choice_presets.sort_by_key(|preset| preset.id);
    for preset in &mut input.choice_presets {
        owned_build::canonicalize_choices(&mut preset.choices);
        preset.rewards.sort();
    }
    input.saved_variants.sort_by_key(|variant| variant.id);
}

fn inventory_union(
    input: &ProjectInput,
    occurrences: Vec<(InstanceId, OccurrenceKind)>,
    inventory: &InventorySnapshot,
    limits: OwnedInputLimits,
) -> Result<InstanceAllocatorState, ProjectError> {
    let stock = inventory.input();
    if input.game_version != stock.game_version {
        return Err(InventoryError::ForeignNamespace.into());
    }
    if input.allocator.lineage() != stock.allocator.lineage() {
        return Err(InventoryError::ForeignLineage.into());
    }
    inventory.validate_limits(limits)?;
    let mut additional: Vec<_> = occurrences
        .into_iter()
        .filter(|(_, kind)| !matches!(kind, OccurrenceKind::Item | OccurrenceKind::Modifier))
        .collect();
    if stock.copies.len() > limits.max_entries - additional.len() {
        return Err(error(
            "project.inventory",
            StructuralErrorKind::LimitExceeded,
        ));
    }
    additional.extend(
        stock
            .copies
            .iter()
            .map(|copy| (copy.id.instance_id(), OccurrenceKind::InventoryCopy)),
    );
    let allocator = InstanceAllocatorState::from_parts(
        input.allocator.lineage(),
        input
            .allocator
            .last_issued()
            .max(stock.allocator.last_issued()),
    );
    let mut merged = BTreeMap::new();
    for item in input.items.iter().chain(&stock.items) {
        match merged.entry(item.id) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(item);
            }
            std::collections::btree_map::Entry::Occupied(entry) if *entry.get() != item => {
                return Err(InventoryError::ConflictingItemRecord(item.id).into());
            }
            _ => {}
        }
    }
    // Reuse shared item/modifier/global-domain checks for the entire union, even
    // when a conflict is unrelated to the selected preset. No table takes precedence.
    if merged.len() > limits.max_collection_entries {
        return Err(error(
            "project.inventory.items",
            StructuralErrorKind::LimitExceeded,
        ));
    }
    let items: Vec<_> = merged.into_values().cloned().collect();
    owned_build::validate_item_records(
        &input.game_version,
        allocator,
        &items,
        &additional,
        limits,
    )?;
    Ok(allocator)
}
fn selected<T, I: Ord + Copy>(records: &[T], id: I, key: impl Fn(&T) -> I) -> &T {
    &records[records
        .binary_search_by_key(&id, key)
        .expect("validated canonical project reference")]
}
fn selected_records<T: Clone, I: Ord + Copy>(
    records: &[T],
    ids: &[I],
    key: impl Fn(&T) -> I,
) -> Vec<T> {
    ids.iter()
        .map(|id| selected(records, *id, &key).clone())
        .collect()
}

/// Select five independent presets, then validate their combined references.
/// This operation allocates no occurrence, advances no revision and assigns no
/// physical stock. Exact resulting content, not project revision alone, binds reuse.
/// Inventory conflicts are checked before selection; inventory cannot fill missing
/// project facts. Only selected item/gem records are retained in the emitted build.
pub fn compose(
    project: &BuildProject,
    selection: &VariantSelection,
    inventory: Option<&InventorySnapshot>,
    limits: OwnedInputLimits,
) -> Result<BuildSpec, ProjectError> {
    let input = project.input();
    let occurrences = validate_project(input, limits)?;
    let members = occurrences.iter().copied().collect();
    let allocator = match inventory {
        Some(stock) => inventory_union(input, occurrences, stock, limits)?,
        None => input.allocator,
    };
    validate_selection(selection, input, &members)?;
    let character = selected(&input.character_presets, selection.character, |v| v.id);
    let equipment_preset = selected(&input.equipment_presets, selection.equipment, |v| v.id);
    let allocation_preset = selected(&input.allocation_presets, selection.allocations, |v| v.id);
    let skill_preset = selected(&input.skill_presets, selection.skills, |v| v.id);
    let choice_preset = selected(&input.choice_presets, selection.choices, |v| v.id);
    // Both original lists were bounded and reference-checked before deduplication.
    // Equal receiving IDs contribute once; distinct uses never merge by item/slot.
    let equipment_ids: Vec<_> = equipment_preset
        .equipment
        .iter()
        .chain(&allocation_preset.equipment)
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let equipment = selected_records(&input.equipment, &equipment_ids, |v| v.id);
    let reward_ids: Vec<_> = character
        .rewards
        .iter()
        .chain(&choice_preset.rewards)
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let skills = selected_records(&input.skills, &skill_preset.skills, |v| v.id);
    let supports = selected_records(&input.supports, &skill_preset.supports, |v| v.id);
    let item_ids: BTreeSet<_> = equipment.iter().map(|v| v.item).collect();
    let gem_ids: BTreeSet<_> = skills
        .iter()
        .filter_map(|v| match &v.source {
            AuthoredSkillSource::Gem(id) => Some(*id),
            AuthoredSkillSource::Direct(_) => None,
        })
        .chain(supports.iter().map(|v| v.support))
        .collect();
    BuildSpec::new(
        BuildInput {
            allocator,
            revision: input.revision,
            game_version: input.game_version.clone(),
            character: CharacterSpec {
                class: character.class.clone(),
                ascendancy: character.ascendancy.clone(),
                level: character.level,
                rewards: selected_records(&input.rewards, &reward_ids, |v| v.id),
            },
            weapon_loadouts: input.weapon_loadouts.clone(),
            active_weapon_loadout: selection.active_weapon_loadout,
            items: item_ids
                .into_iter()
                .map(|id| selected(&input.items, id, |v| v.id).clone())
                .collect(),
            gems: gem_ids
                .into_iter()
                .map(|id| selected(&input.gems, id, |v| v.id).clone())
                .collect(),
            equipment,
            allocations: selected_records(
                &input.allocations,
                &allocation_preset.allocations,
                |v| v.id,
            ),
            skills,
            supports,
            payload_links: selected_records(
                &input.payload_links,
                &skill_preset.payload_links,
                |v| v.id,
            ),
            choices: choice_preset.choices.clone(),
        },
        limits,
    )
    .map_err(Into::into)
}
