//! Self-contained owned stock and exact snapshot availability bindings.
//!
//! A record describes an item; a copy is evidence of physical supply. Availability
//! binds those copies to particular receiving uses. It does not establish game
//! legality, definition coverage, numerical computability or private plan authority.

use crate::{
    build_identity::*,
    owned_build::{
        self, BuildSpec, EquipmentUse, ItemRecord, LoadoutScope, OccurrenceKind, OwnedInputLimits,
        StructuralError, StructuralErrorKind,
    },
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::GameVersionNamespace,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

const BUILD_DIGEST_DOMAIN: &str = "owned-build-v1";
const INVENTORY_DIGEST_DOMAIN: &str = "owned-inventory-v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InventoryCompleteness {
    /// The caller enumerates all physical copies available in this supplied stock domain.
    Complete,
    /// Omitted stock is unknown. A KnownCopy claim still requires an explicit record.
    Partial,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryItem {
    pub id: InventoryItemId,
    pub item: ItemRecordId,
}

/// Untrusted input DTO. Validate with InventorySnapshot::new before using its records.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryInput {
    pub allocator: InstanceAllocatorState,
    pub revision: BuildRevision,
    pub game_version: GameVersionNamespace,
    pub items: Vec<ItemRecord>,
    pub copies: Vec<InventoryItem>,
    pub completeness: InventoryCompleteness,
}

/// Immutable, canonical stock. It owns every record referenced by its physical copies.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct InventorySnapshot(InventoryInput);
impl InventorySnapshot {
    pub fn new(
        mut input: InventoryInput,
        limits: OwnedInputLimits,
    ) -> Result<Self, InventoryError> {
        validate_inventory(&input, limits)?;
        owned_build::canonicalize_item_records(&mut input.items);
        input.copies.sort_by_key(|copy| copy.id);
        Ok(Self(input))
    }
    pub fn input(&self) -> &InventoryInput {
        &self.0
    }
    pub fn validate_limits(&self, limits: OwnedInputLimits) -> Result<(), InventoryError> {
        validate_inventory(&self.0, limits)
    }
    pub fn into_input(self) -> InventoryInput {
        self.0
    }
    pub fn item(&self, id: ItemRecordId) -> Option<&ItemRecord> {
        self.0
            .items
            .binary_search_by_key(&id, |item| item.id)
            .ok()
            .map(|index| &self.0.items[index])
    }
    pub fn copy(&self, id: InventoryItemId) -> Option<&InventoryItem> {
        self.0
            .copies
            .binary_search_by_key(&id, |copy| copy.id)
            .ok()
            .map(|index| &self.0.copies[index])
    }
}

#[derive(Debug)]
pub enum InventoryError {
    Structure(StructuralError),
    Digest(ContentDigestError),
    ForeignNamespace,
    ForeignLineage,
    ConflictingItemRecord(ItemRecordId),
    BuildBindingMismatch,
    InventoryBindingMismatch,
    DuplicateEquipmentUse(ItemSlotUseId),
    UnknownEquipmentUse(ItemSlotUseId),
    MissingEquipmentUse(ItemSlotUseId),
    UnknownCopy(InventoryItemId),
    CopyItemMismatch {
        equipment_use: ItemSlotUseId,
        copy: InventoryItemId,
        selected: ItemRecordId,
        supplied: ItemRecordId,
    },
    OverlapAnalysisLimit,
}
impl fmt::Display for InventoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Structure(error) => error.fmt(f),
            Self::Digest(error) => error.fmt(f),
            Self::ForeignNamespace => {
                f.write_str("inventory and build have different game/version namespaces")
            }
            Self::ForeignLineage => {
                f.write_str("inventory and build have different owned editing lineages")
            }
            Self::ConflictingItemRecord(id) => {
                write!(f, "conflicting canonical content for item record {id:?}")
            }
            Self::BuildBindingMismatch => {
                f.write_str("availability build snapshot binding is stale or foreign")
            }
            Self::InventoryBindingMismatch => {
                f.write_str("availability inventory snapshot binding is stale or foreign")
            }
            Self::DuplicateEquipmentUse(id) => {
                write!(f, "availability repeats equipment use {id:?}")
            }
            Self::UnknownEquipmentUse(id) => {
                write!(f, "availability names absent equipment use {id:?}")
            }
            Self::MissingEquipmentUse(id) => write!(f, "availability omits equipment use {id:?}"),
            Self::UnknownCopy(id) => write!(f, "availability names absent physical copy {id:?}"),
            Self::CopyItemMismatch {
                equipment_use,
                copy,
                selected,
                supplied,
            } => write!(
                f,
                "copy {copy:?} supplies {supplied:?}, not selected record {selected:?} for {equipment_use:?}"
            ),
            Self::OverlapAnalysisLimit => {
                f.write_str("authored copy-overlap analysis exceeds the supplied work bound")
            }
        }
    }
}
impl std::error::Error for InventoryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Structure(error) => Some(error),
            Self::Digest(error) => Some(error),
            _ => None,
        }
    }
}
impl From<StructuralError> for InventoryError {
    fn from(value: StructuralError) -> Self {
        Self::Structure(value)
    }
}
impl From<ContentDigestError> for InventoryError {
    fn from(value: ContentDigestError) -> Self {
        Self::Digest(value)
    }
}

fn structural(path: &str, kind: StructuralErrorKind) -> InventoryError {
    InventoryError::Structure(StructuralError {
        path: path.into(),
        kind,
    })
}
fn bounded_collection(
    path: &str,
    len: usize,
    limits: OwnedInputLimits,
) -> Result<(), InventoryError> {
    if len > limits.max_collection_entries || len > limits.max_entries {
        return Err(structural(path, StructuralErrorKind::LimitExceeded));
    }
    Ok(())
}
fn check_reference_identity(
    path: &str,
    id: InstanceId,
    allocator: InstanceAllocatorState,
) -> Result<(), InventoryError> {
    if id.lineage() != allocator.lineage() {
        return Err(structural(path, StructuralErrorKind::ForeignLineage));
    }
    if id.local() > allocator.last_issued() {
        return Err(structural(path, StructuralErrorKind::BeyondWatermark));
    }
    Ok(())
}
fn validate_inventory(
    input: &InventoryInput,
    limits: OwnedInputLimits,
) -> Result<(), InventoryError> {
    limits.validate()?;
    bounded_collection("inventory.copies", input.copies.len(), limits)?;
    let additional: Vec<_> = input
        .copies
        .iter()
        .map(|copy| (copy.id.instance_id(), OccurrenceKind::InventoryCopy))
        .collect();
    owned_build::validate_item_records(
        &input.game_version,
        input.allocator,
        &input.items,
        &additional,
        limits,
    )?;
    let items: BTreeSet<_> = input.items.iter().map(|item| item.id).collect();
    for (index, copy) in input.copies.iter().enumerate() {
        let path = format!("inventory.copies[{index}].item");
        check_reference_identity(&path, copy.item.instance_id(), input.allocator)?;
        if !items.contains(&copy.item) {
            return Err(structural(
                &path,
                StructuralErrorKind::MissingReference {
                    expected: OccurrenceKind::Item,
                    id: copy.item.instance_id(),
                },
            ));
        }
    }
    Ok(())
}

/// Canonical record union for later composition. This is neither stock nor a build:
/// selecting a record still needs a new supplying use and a self-contained BuildSpec.
#[derive(Clone, Debug, PartialEq)]
pub struct ItemRecordUnion {
    namespace: GameVersionNamespace,
    allocator: InstanceAllocatorState,
    items: Vec<ItemRecord>,
}
impl ItemRecordUnion {
    pub fn game_version(&self) -> &GameVersionNamespace {
        &self.namespace
    }
    pub fn allocator(&self) -> InstanceAllocatorState {
        self.allocator
    }
    pub fn items(&self) -> &[ItemRecord] {
        &self.items
    }
    pub fn item(&self, id: ItemRecordId) -> Option<&ItemRecord> {
        self.items
            .binary_search_by_key(&id, |item| item.id)
            .ok()
            .map(|index| &self.items[index])
    }
    pub fn into_items(self) -> Vec<ItemRecord> {
        self.items
    }
}

/// Reject all conflicting shared records before any caller chooses equipment.
/// Same typed IDs with identical content join once; equal content under distinct IDs
/// remains distinct. All other build/copy occurrence domains remain collision-free.
/// Entry limits apply independently to each input document and the resulting
/// item/occurrence union, not cumulatively to every nested input collection.
pub fn union_build_inventory(
    build: &BuildSpec,
    inventory: &InventorySnapshot,
    limits: OwnedInputLimits,
) -> Result<ItemRecordUnion, InventoryError> {
    let input = build.input();
    if input.game_version != inventory.0.game_version {
        return Err(InventoryError::ForeignNamespace);
    }
    if input.allocator.lineage() != inventory.0.allocator.lineage() {
        return Err(InventoryError::ForeignLineage);
    }
    let mut additional: Vec<_> = owned_build::build_occurrences(build, limits)?
        .into_iter()
        .filter(|(_, kind)| !matches!(kind, OccurrenceKind::Item | OccurrenceKind::Modifier))
        .collect();
    validate_inventory(&inventory.0, limits)?;
    additional.extend(
        inventory
            .0
            .copies
            .iter()
            .map(|copy| (copy.id.instance_id(), OccurrenceKind::InventoryCopy)),
    );
    let allocator = InstanceAllocatorState::from_parts(
        input.allocator.lineage(),
        input
            .allocator
            .last_issued()
            .max(inventory.0.allocator.last_issued()),
    );
    let mut merged = BTreeMap::new();
    for item in input.items.iter().chain(&inventory.0.items) {
        match merged.entry(item.id) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(item.clone());
            }
            std::collections::btree_map::Entry::Occupied(entry) => {
                if entry.get() != item {
                    return Err(InventoryError::ConflictingItemRecord(item.id));
                }
            }
        }
    }
    let mut items: Vec<_> = merged.into_values().collect();
    owned_build::canonicalize_item_records(&mut items);
    owned_build::validate_item_records(
        &input.game_version,
        allocator,
        &items,
        &additional,
        limits,
    )?;
    Ok(ItemRecordUnion {
        namespace: input.game_version.clone(),
        allocator,
        items,
    })
}

/// Exact semantic snapshot identity, not a numerical-plan digest. Its value is only
/// a caller claim until bind_availability recomputes it from immutable inputs.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildContentBinding {
    lineage: BuildLineage,
    revision: BuildRevision,
    digest: OwnedContentDigest,
}
impl BuildContentBinding {
    pub fn lineage(&self) -> BuildLineage {
        self.lineage
    }
    pub fn revision(&self) -> BuildRevision {
        self.revision
    }
    pub fn digest(&self) -> &OwnedContentDigest {
        &self.digest
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryContentBinding {
    lineage: BuildLineage,
    revision: BuildRevision,
    digest: OwnedContentDigest,
}
impl InventoryContentBinding {
    pub fn lineage(&self) -> BuildLineage {
        self.lineage
    }
    pub fn revision(&self) -> BuildRevision {
        self.revision
    }
    pub fn digest(&self) -> &OwnedContentDigest {
        &self.digest
    }
}
fn digest_build(
    build: &BuildSpec,
    limits: OwnedInputLimits,
) -> Result<BuildContentBinding, InventoryError> {
    Ok(BuildContentBinding {
        lineage: build.input().allocator.lineage(),
        revision: build.input().revision,
        digest: digest_owned(BUILD_DIGEST_DOMAIN, build, limits.max_wire_bytes)?,
    })
}
fn digest_inventory(
    inventory: &InventorySnapshot,
    limits: OwnedInputLimits,
) -> Result<InventoryContentBinding, InventoryError> {
    Ok(InventoryContentBinding {
        lineage: inventory.0.allocator.lineage(),
        revision: inventory.0.revision,
        digest: digest_owned(INVENTORY_DIGEST_DOMAIN, inventory, limits.max_wire_bytes)?,
    })
}
pub fn build_content_binding(
    build: &BuildSpec,
    limits: OwnedInputLimits,
) -> Result<BuildContentBinding, InventoryError> {
    owned_build::build_occurrences(build, limits)?;
    digest_build(build, limits)
}
pub fn inventory_content_binding(
    inventory: &InventorySnapshot,
    limits: OwnedInputLimits,
) -> Result<InventoryContentBinding, InventoryError> {
    validate_inventory(&inventory.0, limits)?;
    digest_inventory(inventory, limits)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum AvailabilityClaim {
    KnownCopy(InventoryItemId),
    Unspecified,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UseAvailability {
    pub equipment_use: ItemSlotUseId,
    pub claim: AvailabilityClaim,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AvailabilityAssignments {
    pub build: BuildContentBinding,
    pub inventory: InventoryContentBinding,
    pub uses: Vec<UseAvailability>,
}

/// Private bound result. Deserializing assignments cannot construct this value.
#[derive(Clone, Debug)]
pub struct BoundAvailability {
    assignments: AvailabilityAssignments,
}
impl BoundAvailability {
    pub fn assignments(&self) -> &AvailabilityAssignments {
        &self.assignments
    }
    /// Separate, bounded advisory over authored loadout scopes. Socket/allocation
    /// activity and other game rules may further constrain actual simultaneous use.
    /// Failure to finish this advisory does not invalidate the structural binding.
    pub fn authored_copy_overlaps(
        &self,
        build: &BuildSpec,
        limits: OwnedInputLimits,
    ) -> Result<Vec<KnownCopyOverlap>, InventoryError> {
        if build_content_binding(build, limits)? != self.assignments.build {
            return Err(InventoryError::BuildBindingMismatch);
        }
        let equipment: BTreeMap<_, _> = build
            .input()
            .equipment
            .iter()
            .map(|usage| (usage.id, usage))
            .collect();
        let mut groups: BTreeMap<InventoryItemId, Vec<&EquipmentUse>> = BTreeMap::new();
        for assignment in &self.assignments.uses {
            if let AvailabilityClaim::KnownCopy(copy) = assignment.claim {
                groups
                    .entry(copy)
                    .or_default()
                    .push(equipment[&assignment.equipment_use]);
            }
        }
        let mut remaining = limits.max_entries;
        let mut overlaps = Vec::new();
        for (copy, usages) in groups {
            for (index, first) in usages.iter().enumerate() {
                for second in &usages[index + 1..] {
                    consume_overlap_step(&mut remaining)?;
                    let mut loadouts = Vec::new();
                    for loadout in &build.input().weapon_loadouts {
                        consume_overlap_step(&mut remaining)?;
                        if scope_contains(&first.scope, *loadout)
                            && scope_contains(&second.scope, *loadout)
                        {
                            loadouts.push(*loadout);
                        }
                    }
                    if !loadouts.is_empty() {
                        overlaps.push(KnownCopyOverlap {
                            copy,
                            equipment_uses: [first.id, second.id],
                            loadouts,
                        });
                    }
                }
            }
        }
        Ok(overlaps)
    }
}
fn consume_overlap_step(remaining: &mut usize) -> Result<(), InventoryError> {
    *remaining = remaining
        .checked_sub(1)
        .ok_or(InventoryError::OverlapAnalysisLimit)?;
    Ok(())
}
fn scope_contains(scope: &LoadoutScope, loadout: WeaponLoadoutId) -> bool {
    match scope {
        LoadoutScope::Shared => true,
        LoadoutScope::Selected { loadouts } => loadouts.binary_search(&loadout).is_ok(),
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KnownCopyOverlap {
    pub copy: InventoryItemId,
    pub equipment_uses: [ItemSlotUseId; 2],
    pub loadouts: Vec<WeaponLoadoutId>,
}

/// Binds stock evidence to every receiving use exactly once. Physical-copy reuse is
/// intentionally accepted here; authored_copy_overlaps reports it separately.
/// Limits bound each supplied document, the union table and the assignments
/// independently; authored overlap analysis has its own separate work bound.
pub fn bind_availability(
    build: &BuildSpec,
    inventory: &InventorySnapshot,
    mut assignments: AvailabilityAssignments,
    limits: OwnedInputLimits,
) -> Result<BoundAvailability, InventoryError> {
    limits.validate()?;
    bounded_collection("availability.uses", assignments.uses.len(), limits)?;
    union_build_inventory(build, inventory, limits)?;
    if assignments.build != digest_build(build, limits)? {
        return Err(InventoryError::BuildBindingMismatch);
    }
    if assignments.inventory != digest_inventory(inventory, limits)? {
        return Err(InventoryError::InventoryBindingMismatch);
    }
    let equipment: BTreeMap<_, _> = build
        .input()
        .equipment
        .iter()
        .map(|usage| (usage.id, usage.item))
        .collect();
    let mut seen = BTreeSet::new();
    for assignment in &assignments.uses {
        check_reference_identity(
            "availability.equipment_use",
            assignment.equipment_use.instance_id(),
            build.input().allocator,
        )?;
        if !seen.insert(assignment.equipment_use) {
            return Err(InventoryError::DuplicateEquipmentUse(
                assignment.equipment_use,
            ));
        }
        let selected =
            equipment
                .get(&assignment.equipment_use)
                .ok_or(InventoryError::UnknownEquipmentUse(
                    assignment.equipment_use,
                ))?;
        if let AvailabilityClaim::KnownCopy(copy_id) = assignment.claim {
            check_reference_identity(
                "availability.copy",
                copy_id.instance_id(),
                inventory.0.allocator,
            )?;
            let copy = inventory
                .copy(copy_id)
                .ok_or(InventoryError::UnknownCopy(copy_id))?;
            if copy.item != *selected {
                return Err(InventoryError::CopyItemMismatch {
                    equipment_use: assignment.equipment_use,
                    copy: copy_id,
                    selected: *selected,
                    supplied: copy.item,
                });
            }
        }
    }
    for id in equipment.keys() {
        if !seen.contains(id) {
            return Err(InventoryError::MissingEquipmentUse(*id));
        }
    }
    assignments.uses.sort_by_key(|usage| usage.equipment_use);
    Ok(BoundAvailability { assignments })
}
