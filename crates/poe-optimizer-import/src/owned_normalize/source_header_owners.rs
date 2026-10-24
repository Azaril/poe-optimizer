//! Account for proved source-header ownership without resolving saved selection.
use super::source_shape::{charge_frame, charge_row, container_text, plain_row, value};
use super::*;

fn saved_selection(
    b: &mut Builder<'_, '_>,
    draft: &DraftSessionInput,
) -> Result<Option<DraftIssueId>> {
    let DraftListCompletion::Pending { id, code } = &draft.saved_variants.completion else {
        return Ok(None);
    };
    let root = b.evidence.rows()[0].occurrence().id();
    let Some(origin) = b.origins.first() else {
        return Ok(None);
    };
    b.charge(origin.links.len().saturating_add(1))?;
    let origin = &b.origins[0];
    if !draft.saved_variants.members.is_empty()
        || code.as_str() != "saved-selection-not-converted"
        || origin.source != root
        || origin.disposition != SourceDisposition::Contributes
        || origin
            .links
            .iter()
            .filter(|link| **link == OwnedOriginTarget::Issue(*id))
            .count()
            != 1
    {
        return Ok(None);
    }
    Ok(Some(*id))
}

fn append<const N: usize>(
    b: &mut Builder<'_, '_>,
    source: SourceOccurrenceId,
    additions: [OwnedOriginTarget; N],
) -> Result<()> {
    // Preflight every addition before changing either links or their counter.
    // A late work/link limit failure must not leave a partly attributed header.
    b.charge(additions.len())?;
    let links = b
        .links
        .checked_add(additions.len())
        .filter(|links| *links <= b.limits.max_origin_links)
        .ok_or(NormalizationError::Limit("origin links"))?;
    b.origins[source.ordinal() as usize].links.extend(additions);
    b.links = links;
    Ok(())
}

/// Attribute the strict Tree header to its actual selected presets and the
/// existing unresolved selection. Child semantics and all draft obligations
/// remain independent. Run before unaccounted sources receive fallback links.
pub(super) fn account_tree(
    b: &mut Builder<'_, '_>,
    draft: &DraftSessionInput,
    specs: &BTreeMap<SourceOccurrenceId, usize>,
    characters: &BTreeMap<SourceOccurrenceId, usize>,
) -> Result<()> {
    let Some(tree) = passive_socket_membership::canonical_tree(b)? else {
        return Ok(());
    };
    let Some((selected_tree, selected_spec)) = source_selection::tree(b)? else {
        return Ok(());
    };
    let evidence = b.evidence;
    let row = &evidence.rows()[tree.ordinal() as usize];
    if tree != selected_tree || b.origins.len() != evidence.rows().len() {
        return Ok(());
    }
    let origin = &b.origins[tree.ordinal() as usize];
    if origin.source != tree
        || origin.disposition != SourceDisposition::Contributes
        || !origin.links.is_empty()
    {
        return Ok(());
    }
    let Some(selection_issue) = saved_selection(b, draft)? else {
        return Ok(());
    };

    // Require the complete ordinary Spec-to-preset correspondence, not just a
    // selected-key lookup. A missing/foreign map entry or an aliased sibling
    // cannot lend ownership to the selected source. Descendant projections
    // (Sockets, URL and Overrides) are not independent Spec owners.
    b.charge(
        row.children()
            .len()
            .saturating_add(specs.len())
            .saturating_add(characters.len())
            .saturating_add(draft.allocation_presets.members.len())
            .saturating_add(draft.character_presets.members.len()),
    )?;
    if row.children().len() != specs.len()
        || row.children().len() != characters.len()
        || row.children().len() != draft.allocation_presets.members.len()
        || row.children().len() != draft.character_presets.members.len()
    {
        return Ok(());
    }
    let mut allocation_owners = BTreeSet::new();
    let mut character_owners = BTreeSet::new();
    let mut allocation_indices = BTreeSet::new();
    let mut character_indices = BTreeSet::new();
    let mut selected = None;
    for source in row.children() {
        let Some(&allocation_index) = specs.get(source) else {
            return Ok(());
        };
        let Some(&character_index) = characters.get(source) else {
            return Ok(());
        };
        let Some(allocation) = draft.allocation_presets.members.get(allocation_index) else {
            return Ok(());
        };
        let Some(character) = draft.character_presets.members.get(character_index) else {
            return Ok(());
        };
        let origin = &b.origins[source.ordinal() as usize];
        b.charge(origin.links.len().saturating_mul(2).saturating_add(4))?;
        let origin = &b.origins[source.ordinal() as usize];
        if origin.source != *source
            || origin.disposition != SourceDisposition::Contributes
            || !allocation_indices.insert(allocation_index)
            || !character_indices.insert(character_index)
            || !allocation_owners.insert(allocation.id)
            || !character_owners.insert(character.id)
        {
            return Ok(());
        }
        let mut allocations = origin.links.iter().filter_map(|link| match link {
            OwnedOriginTarget::AllocationPreset(id) => Some(*id),
            _ => None,
        });
        let mut owners = origin.links.iter().filter_map(|link| match link {
            OwnedOriginTarget::CharacterPreset(id) => Some(*id),
            _ => None,
        });
        if allocations.next() != Some(allocation.id)
            || allocations.next().is_some()
            || owners.next() != Some(character.id)
            || owners.next().is_some()
        {
            return Ok(());
        }
        if *source == selected_spec {
            selected = Some((allocation.id, character.id));
        }
    }
    let Some((allocation, character)) = selected else {
        return Ok(());
    };
    append(
        b,
        tree,
        [
            OwnedOriginTarget::AllocationPreset(allocation),
            OwnedOriginTarget::CharacterPreset(character),
            OwnedOriginTarget::Issue(selection_issue),
        ],
    )
}

/// Account for an ordinary Items header while keeping set contents, loadout and
/// selection unresolved. With actual ItemSets present, the source loader never
/// reads the root swap flag; showStatDifferences only requests tooltip compares.
/// Neither field supplies a native value or overwrites a selected set's state.
pub(super) fn account_items(
    b: &mut Builder<'_, '_>,
    draft: &DraftSessionInput,
    sets: &BTreeMap<SourceOccurrenceId, usize>,
) -> Result<()> {
    let Some((items, selected_set)) = source_selection::items(b)? else {
        return Ok(());
    };
    let evidence = b.evidence;
    let root = &evidence.rows()[0];
    charge_row(b, root)?;
    if !plain_row(root, &[], false)
        || !container_text(root)
        || b.origins.len() != evidence.rows().len()
    {
        return Ok(());
    }
    let row = &evidence.rows()[items.ordinal() as usize];
    let origin = &b.origins[items.ordinal() as usize];
    if origin.source != items
        || origin.disposition != SourceDisposition::Contributes
        || !origin.links.is_empty()
    {
        return Ok(());
    }
    let Some(selection_issue) = saved_selection(b, draft)? else {
        return Ok(());
    };
    b.charge(
        row.children()
            .len()
            .saturating_add(sets.len())
            .saturating_add(draft.equipment_presets.members.len()),
    )?;
    let mut owners = BTreeSet::new();
    let mut indices = BTreeSet::new();
    let mut set_count = 0;
    let mut selected = None;
    for source in row.children() {
        let set = &evidence.rows()[source.ordinal() as usize];
        if set.occurrence().name() != "ItemSet" {
            continue;
        }
        set_count += 1;
        if set.occurrence().has_namespace_context()
            || set.occurrence().parent() != Some(items)
            || !matches!(
                set.authored_instance(),
                Some(AuthoredInstanceId::ItemSet(_))
            )
        {
            return Ok(());
        }
        let Some(&index) = sets.get(source) else {
            return Ok(());
        };
        let Some(preset) = draft.equipment_presets.members.get(index) else {
            return Ok(());
        };
        let origin = &b.origins[source.ordinal() as usize];
        b.charge(origin.links.len().saturating_add(2))?;
        let origin = &b.origins[source.ordinal() as usize];
        if origin.source != *source
            || origin.disposition != SourceDisposition::Contributes
            || !indices.insert(index)
            || !owners.insert(preset.id)
        {
            return Ok(());
        }
        let mut linked = origin.links.iter().filter_map(|link| match link {
            OwnedOriginTarget::EquipmentPreset(id) => Some(*id),
            _ => None,
        });
        if linked.next() != Some(preset.id) || linked.next().is_some() {
            return Ok(());
        }
        if *source == selected_set {
            selected = Some(preset.id);
        }
    }
    if set_count == 0
        || set_count != sets.len()
        || set_count != draft.equipment_presets.members.len()
    {
        return Ok(());
    }
    let Some(selected) = selected else {
        return Ok(());
    };
    append(
        b,
        items,
        [
            OwnedOriginTarget::EquipmentPreset(selected),
            OwnedOriginTarget::Issue(selection_issue),
        ],
    )
}

fn skill_preferences(b: &mut Builder<'_, '_>, row: &SourceEvidenceRow<'_>) -> Result<bool> {
    charge_frame(
        b,
        row,
        &[
            "defaultGemLevel",
            "defaultGemQuality",
            "sortGemsByDPS",
            "showLegacyGems",
            "showSupportGemTypes",
            "sortGemsByDPSField",
        ],
    )?;
    // The checked container has already rejected unknown/duplicate attributes
    // and unavailable decoding. Admit ordinary finite editing preferences only;
    // do not import legacy precedence, numeric fallback, or clamping behavior.
    if row.attribute("matchGemLevelToCharacterLevel").is_some()
        || value(row, "defaultGemLevel")
            .is_some_and(|v| !matches!(v, "normalMaximum" | "corruptedMaximum" | "characterLevel"))
        || value(row, "defaultGemQuality").is_some_and(|v| {
            v.is_empty()
                || v.len() > 2
                || (v.len() > 1 && v.starts_with('0'))
                || !v.bytes().all(|byte| byte.is_ascii_digit())
                || !v.parse::<u8>().is_ok_and(|quality| quality <= 23)
        })
        || ["sortGemsByDPS", "showLegacyGems"]
            .iter()
            .any(|name| value(row, name).is_some_and(|v| !matches!(v, "true" | "false")))
        || value(row, "showSupportGemTypes")
            .is_some_and(|v| !matches!(v, "ALL" | "LINEAGE" | "NORMAL"))
        || value(row, "sortGemsByDPSField").is_some_and(|v| {
            !matches!(
                v,
                "FullDPS"
                    | "CombinedDPS"
                    | "TotalDPS"
                    | "AverageDamage"
                    | "TotalDot"
                    | "BleedDPS"
                    | "IgniteDPS"
                    | "TotalPoisonDPS"
                    | "TotalEHP"
            )
        })
    {
        return Ok(false);
    }
    Ok(true)
}

/// Retain ordinary source editing/ranking/filter preferences and the saved set
/// key under their existing owner. Saved Gem inputs have separate authority:
/// absent preferences emit no defaults, and rankings create no native objective.
/// This does not prove the final active source tuple or resolve MAIN selection.
pub(super) fn account_skills(
    b: &mut Builder<'_, '_>,
    draft: &DraftSessionInput,
    sets: &BTreeMap<SourceOccurrenceId, usize>,
) -> Result<()> {
    let Some(sources) = skill_source_census::container_sets(b)? else {
        return Ok(());
    };
    let Some((skills, selected_set)) = source_selection::skills(b, Some(&sources))? else {
        return Ok(());
    };
    let evidence = b.evidence;
    let row = &evidence.rows()[skills.ordinal() as usize];
    if b.origins.len() != evidence.rows().len() || !skill_preferences(b, row)? {
        return Ok(());
    }
    let origin = &b.origins[skills.ordinal() as usize];
    if origin.source != skills
        || origin.disposition != SourceDisposition::Contributes
        || !origin.links.is_empty()
    {
        return Ok(());
    }
    let Some(selection_issue) = saved_selection(b, draft)? else {
        return Ok(());
    };
    b.charge(
        sources
            .len()
            .saturating_add(sets.len())
            .saturating_add(draft.skill_presets.members.len()),
    )?;
    if sources.len() != sets.len() || sources.len() != draft.skill_presets.members.len() {
        return Ok(());
    }
    let mut owners = BTreeSet::new();
    let mut indices = BTreeSet::new();
    let mut selected = None;
    for source in sources {
        let Some(&index) = sets.get(&source) else {
            return Ok(());
        };
        let Some(preset) = draft.skill_presets.members.get(index) else {
            return Ok(());
        };
        b.charge(
            b.origins[source.ordinal() as usize]
                .links
                .len()
                .saturating_add(2),
        )?;
        let origin = &b.origins[source.ordinal() as usize];
        if origin.source != source
            || origin.disposition != SourceDisposition::Contributes
            || !indices.insert(index)
            || !owners.insert(preset.id)
        {
            return Ok(());
        }
        let mut linked = origin.links.iter().filter_map(|link| match link {
            OwnedOriginTarget::SkillPreset(id) => Some(*id),
            _ => None,
        });
        if linked.next() != Some(preset.id) || linked.next().is_some() {
            return Ok(());
        }
        if source == selected_set {
            selected = Some(preset.id);
        }
    }
    let Some(selected) = selected else {
        return Ok(());
    };
    append(
        b,
        skills,
        [
            OwnedOriginTarget::SkillPreset(selected),
            OwnedOriginTarget::Issue(selection_issue),
        ],
    )
}

#[cfg(test)]
#[path = "source_header_owners_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "items_header_owners_tests.rs"]
mod items_tests;

#[cfg(test)]
#[path = "skills_header_owners_tests.rs"]
mod skills_tests;
