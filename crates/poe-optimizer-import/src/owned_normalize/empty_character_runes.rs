//! Reviewed explicit-empty character rune selections, separate from item sockets.
use super::source_shape::{charge_row, container_text, plain_row, value};
use super::*;

/// A finite source control inventory. This proves only an explicit empty saved
/// selection; it does not supply occupied rune effects or ItemSet completeness.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmptyCharacterRuneSelections {
    pub mapping_source: OwnedContentDigest,
    pub slot_names: Vec<String>,
    pub explicit_empty_selection: String,
}

pub(super) struct CompiledEmptyCharacterRunes<'p> {
    slots: BTreeSet<&'p str>,
    empty: &'p str,
    pub work: usize,
}

fn token(value: &str) -> bool {
    !value.is_empty() && value.trim() == value && !value.chars().any(char::is_control)
}

pub(super) fn compile<'p>(
    policy: &'p NormalizationPolicy,
    mappings: &OwnedMappingIndex,
    limits: NormalizationLimits,
) -> Result<Option<CompiledEmptyCharacterRunes<'p>>> {
    let Some(EquipmentMembershipPolicy::PobOrdinaryImportedAndEmptyCharacterRunesV3 {
        empty_character_runes: policy,
        ..
    }) = &policy.equipment_membership
    else {
        return Ok(None);
    };
    // Even an empty reviewed domain must not bypass its source commitment.
    if &policy.mapping_source != mappings.source_identity() {
        return Err(NormalizationError::Binding);
    }
    if policy.slot_names.len() > 256 {
        return Err(NormalizationError::Policy(
            "empty character rune slot count",
        ));
    }
    let mut work = 0usize;
    let mut slots = BTreeSet::new();
    for name in policy
        .slot_names
        .iter()
        .chain(std::iter::once(&policy.explicit_empty_selection))
    {
        if name.len() > limits.mapping.max_string_bytes {
            return Err(NormalizationError::Limit(
                "empty character rune policy string",
            ));
        }
        work = work
            .checked_add(name.len().saturating_add(1))
            .filter(|work| *work <= limits.max_work)
            .ok_or(NormalizationError::Limit(
                "empty character rune policy work",
            ))?;
        if !token(name) {
            return Err(NormalizationError::Policy(
                "empty character rune policy token",
            ));
        }
    }
    for name in &policy.slot_names {
        if !slots.insert(name.as_str()) {
            return Err(NormalizationError::Policy(
                "duplicate empty character rune slot",
            ));
        }
    }
    Ok(Some(CompiledEmptyCharacterRunes {
        slots,
        empty: &policy.explicit_empty_selection,
        work,
    }))
}

fn numeric_key(value: &str, zero: bool) -> bool {
    (zero && value == "0")
        || (!value.starts_with('0')
            && !value.is_empty()
            && value.bytes().all(|v| v.is_ascii_digit())
            && value.parse::<u32>().is_ok_and(|v| v > 0))
}

/// A single census proves globally safe loader writes while retaining a stricter
/// per-set absence decision. Safe duplicates and occupied selections can leave
/// one set pending without poisoning another independent set.
fn empty_sources(
    b: &mut Builder<'_, '_>,
    source: SourceOccurrenceId,
    items: SourceOccurrenceId,
    policy: &CompiledEmptyCharacterRunes<'_>,
    ordinary_slots: &BTreeMap<&str, &EquipmentLoadoutRule>,
) -> Result<Option<Vec<SourceOccurrenceId>>> {
    let evidence = b.evidence;
    let row = &evidence.rows()[source.ordinal() as usize];
    charge_row(b, row)?;
    if row.occurrence().parent() != Some(items)
        || !plain_row(row, &["id", "title", "useSecondWeaponSet"], false)
        || !container_text(row)
        || value(row, "useSecondWeaponSet").is_some_and(|v| !equipment_membership::boolean_token(v))
    {
        return Ok(None);
    }
    let mut names = BTreeSet::new();
    let mut ordinary_names = BTreeSet::new();
    let mut urls = BTreeSet::new();
    let mut empty = Vec::new();
    let mut locally_proven = true;
    for child in row.children() {
        let child = &evidence.rows()[child.ordinal() as usize];
        charge_row(b, child)?;
        match child.occurrence().name() {
            "RuneSlot" => {
                if !plain_row(child, &["slotName", "runeName"], true) {
                    return Ok(None);
                }
                let Some(name) = value(child, "slotName") else {
                    return Ok(None);
                };
                // Both branches address the same ItemSet table. A name outside
                // the checked control union could resolve scalar metadata and
                // abort the loader, even in a different archived ItemSet.
                if !policy.slots.contains(name) && !ordinary_slots.contains_key(name) {
                    return Ok(None);
                }
                let selection = value(child, "runeName");
                if !policy.slots.contains(name)
                    || !names.insert(name)
                    || !selection.is_some_and(token)
                {
                    locally_proven = false;
                }
                if selection == Some(policy.empty) {
                    empty.push(child.occurrence().id());
                }
            }
            "Slot" => {
                if !plain_row(
                    child,
                    &["name", "itemId", "active", "itemPbURL", "note"],
                    true,
                ) || value(child, "active")
                    .is_some_and(|v| !equipment_membership::boolean_token(v))
                    || !value(child, "itemId").is_some_and(|v| numeric_key(v, true))
                {
                    return Ok(None);
                }
                let Some(name) = value(child, "name") else {
                    return Ok(None);
                };
                if !ordinary_slots.contains_key(name) && !policy.slots.contains(name) {
                    return Ok(None);
                }
                // An ordinary write onto a rune control is loader-safe but has
                // no reviewed local absence proof, in either source order.
                if policy.slots.contains(name) || !ordinary_names.insert(name) {
                    locally_proven = false;
                }
            }
            "SocketIdURL" => {
                if !plain_row(child, &["nodeId", "itemPbURL", "name"], true)
                    || !value(child, "nodeId").is_some_and(|v| numeric_key(v, false))
                    || value(child, "itemPbURL").is_none()
                {
                    return Ok(None);
                }
                if !urls.insert(value(child, "nodeId").expect("checked key")) {
                    locally_proven = false;
                }
            }
            _ => return Ok(None),
        }
    }
    if !locally_proven {
        empty.clear();
    }
    Ok(Some(empty))
}
fn pending<T>(field: &DraftField<T>, expected: &str) -> Option<DraftIssueId> {
    match field {
        DraftField::Pending(value)
            if value.code.as_str() == expected && value.candidates.is_empty() =>
        {
            Some(value.id)
        }
        _ => None,
    }
}

/// Retire only the exact fabricated use and its three unresolved fields. IDs
/// remain spent, preserving every later allocation and the allocator watermark.
pub(super) fn retire(
    b: &mut Builder<'_, '_>,
    draft: &mut DraftSessionInput,
    policy: &CompiledEmptyCharacterRunes<'_>,
    sets: &BTreeMap<SourceOccurrenceId, usize>,
    ordinary_slots: &BTreeMap<&str, &EquipmentLoadoutRule>,
) -> Result<()> {
    if policy.slots.is_empty() {
        return Ok(());
    }
    let evidence = b.evidence;
    b.charge(sets.len())?;
    let mut has_runes = false;
    for source in sets.keys() {
        let row = &evidence.rows()[source.ordinal() as usize];
        b.charge(row.children().len())?;
        has_runes |= row
            .children()
            .iter()
            .any(|id| evidence.rows()[id.ordinal() as usize].occurrence().name() == "RuneSlot");
    }
    if !has_runes {
        return Ok(());
    }
    let Some(items) = equipment_membership::ordinary_items(b)? else {
        return Ok(());
    };
    let mut candidate_sets = Vec::new();
    for (source, index) in sets {
        let Some(candidates) = empty_sources(b, *source, items, policy, ordinary_slots)? else {
            // ItemsTab activates its selection only after every saved set has
            // loaded. An unsafe later set is as relevant as an unsafe earlier one.
            return Ok(());
        };
        if !candidates.is_empty() {
            candidate_sets.push((*index, candidates));
        }
    }
    b.charge(draft.equipment.members.len())?;
    let uses: BTreeMap<_, _> = draft.equipment.members.iter().map(|v| (v.id, v)).collect();
    let mut retired = BTreeSet::new();
    for (index, candidates) in candidate_sets {
        let mut proven = Vec::new();
        for source in candidates {
            let origin = &b.origins[source.ordinal() as usize];
            b.charge(
                origin.links.len().saturating_add(
                    draft.equipment_presets.members[index]
                        .equipment
                        .members
                        .len(),
                ),
            )?;
            let origin = &b.origins[source.ordinal() as usize];
            let [
                OwnedOriginTarget::Equipment(id),
                OwnedOriginTarget::Issue(first),
                OwnedOriginTarget::Issue(second),
                OwnedOriginTarget::Issue(third),
            ] = origin.links.as_slice()
            else {
                continue;
            };
            let Some(record) = uses.get(id) else { continue };
            let DraftEquipmentDestination::Pending(destination) = &record.destination else {
                continue;
            };
            if !matches!(origin.disposition, SourceDisposition::Contributes)
                || pending(&record.item, "item-reference-unresolved") != Some(*first)
                || destination.id != *second
                || destination.code.as_str() != "socket-destination-not-converted"
                || !destination.candidates.is_empty()
                || pending(&record.scope, "equipment-scope-not-converted") != Some(*third)
                || draft.equipment_presets.members[index]
                    .equipment
                    .members
                    .iter()
                    .filter(|v| *v == id)
                    .count()
                    != 1
            {
                continue;
            }
            proven.push((source, *id));
        }
        b.charge(
            draft.equipment_presets.members[index]
                .equipment
                .members
                .len()
                .saturating_add(proven.len()),
        )?;
        let set_retired: BTreeSet<_> = proven.iter().map(|(_, id)| *id).collect();
        draft.equipment_presets.members[index]
            .equipment
            .members
            .retain(|id| !set_retired.contains(id));
        for (source, id) in proven {
            let origin = &mut b.origins[source.ordinal() as usize];
            origin.links.clear();
            origin.disposition =
                SourceDisposition::SourceOnly(key("explicit-empty-character-rune-selection"));
            retired.insert(id);
        }
    }
    b.charge(draft.equipment.members.len())?;
    draft
        .equipment
        .members
        .retain(|record| !retired.contains(&record.id));
    Ok(())
}
