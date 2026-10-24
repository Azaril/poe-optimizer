//! Account for proved source-header ownership without resolving saved selection.
use super::*;

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
    let root = evidence.rows()[0].occurrence().id();
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
    let DraftListCompletion::Pending {
        id: selection_issue,
        code,
    } = &draft.saved_variants.completion
    else {
        return Ok(());
    };
    b.charge(b.origins[0].links.len().saturating_add(1))?;
    if !draft.saved_variants.members.is_empty()
        || code.as_str() != "saved-selection-not-converted"
        || b.origins[0].source != root
        || b.origins[0].disposition != SourceDisposition::Contributes
        || b.origins[0]
            .links
            .iter()
            .filter(|link| **link == OwnedOriginTarget::Issue(*selection_issue))
            .count()
            != 1
    {
        return Ok(());
    }

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
    let additions = [
        OwnedOriginTarget::AllocationPreset(allocation),
        OwnedOriginTarget::CharacterPreset(character),
        OwnedOriginTarget::Issue(*selection_issue),
    ];
    // Preflight every addition before changing either links or their counter.
    // A late work/link limit failure must not leave a partly attributed header.
    b.charge(additions.len())?;
    let links = b
        .links
        .checked_add(additions.len())
        .filter(|links| *links <= b.limits.max_origin_links)
        .ok_or(NormalizationError::Limit("origin links"))?;
    b.origins[tree.ordinal() as usize].links.extend(additions);
    b.links = links;
    Ok(())
}

#[cfg(test)]
#[path = "source_header_owners_tests.rs"]
mod tests;
