//! Retain Build-level and saved MAIN ownership without resolving a native query.
use super::*;
use character_reward_inventory::CompiledCharacterRewardInventory;

fn selected_level(
    b: &mut Builder<'_, '_>,
    row: &SourceEvidenceRow<'_>,
    recipe: &ValueRecipe,
) -> Result<Option<u16>> {
    // A matching number from a different attribute/default is not the source
    // of every Character level. Keep the sole ordinary Attribute producer.
    let [tier] = recipe.input().tiers.as_slice() else {
        return Ok(None);
    };
    let [selector] = tier.selectors.as_slice() else {
        return Ok(None);
    };
    b.charge(1)?;
    if selector.lane != ValueLane::Attribute
        || selector.name != "level"
        || tier.duplicates != DuplicatePolicy::Reject
    {
        return Ok(None);
    }
    let (ScalarValue::Selected(ParameterValue::Integer(level)), Some(origin)) =
        b.scalar_provenance(row, recipe)?
    else {
        return Ok(None);
    };
    if origin.occurrence != row.occurrence().id()
        || !b.evidence.attribute(origin).is_ok_and(|attribute| {
            attribute.origin().name == "level" && attribute.origin().namespace.is_none()
        })
    {
        return Ok(None);
    }
    Ok(u16::try_from(level.get())
        .ok()
        .filter(|n| (1..=100).contains(n)))
}

/// Links describe source ownership only. Class labels do not select a class;
/// source MAIN position and UI view do not create native queries or actions.
pub(in crate::owned_normalize) fn account_build(
    b: &mut Builder<'_, '_>,
    draft: &DraftSessionInput,
    characters: &BTreeMap<SourceOccurrenceId, usize>,
    skills: &BTreeMap<SourceOccurrenceId, usize>,
    frame: Option<&CompiledCharacterRewardInventory<'_>>,
    level_recipe: &ValueRecipe,
) -> Result<()> {
    let Some(frame) = frame else {
        return Ok(());
    };
    let evidence = b.evidence;
    let [build] = evidence.sections(SourceSectionKind::Build) else {
        return Ok(());
    };
    let root = &evidence.rows()[0];
    b.charge(root.children().len())?;
    let raw_builds = root
        .children()
        .iter()
        .filter(|id| evidence.rows()[id.ordinal() as usize].occurrence().name() == "Build")
        .count();
    let row = &evidence.rows()[build.ordinal() as usize];
    if raw_builds != 1
        || row.occurrence().parent() != Some(root.occurrence().id())
        || b.origins.len() != evidence.rows().len()
        || !frame.accepts_build_frame(b, row)?
        || !matches!(
            value(row, "viewMode"),
            Some(
                "IMPORT"
                    | "NOTES"
                    | "CONFIG"
                    | "TREE"
                    | "SKILLS"
                    | "ITEMS"
                    | "CALCS"
                    | "PARTY"
                    | "COMPARE"
            )
        )
    {
        return Ok(());
    }
    let origin = &b.origins[build.ordinal() as usize];
    if origin.source != *build
        || origin.disposition != SourceDisposition::Contributes
        || !origin.links.is_empty()
    {
        return Ok(());
    }
    let Some(level) = selected_level(b, row, level_recipe)? else {
        return Ok(());
    };
    let Some(tree) = passive_socket_membership::canonical_tree(b)? else {
        return Ok(());
    };
    let Some((selected_tree, _)) = source_selection::tree(b)? else {
        return Ok(());
    };
    if tree != selected_tree {
        return Ok(());
    }
    let specs = evidence.rows()[tree.ordinal() as usize].children();
    b.charge(
        specs
            .len()
            .saturating_add(characters.len())
            .saturating_add(draft.character_presets.members.len()),
    )?;
    if specs.is_empty()
        || specs.len() != characters.len()
        || specs.len() != draft.character_presets.members.len()
    {
        return Ok(());
    }
    let Some(sets) = skill_source_census::container_sets(b)? else {
        return Ok(());
    };
    let Some((_, selected_set)) = source_selection::skills(b, Some(&sets))? else {
        return Ok(());
    };
    let Some(skill) = selected_skill_owner(b, draft, skills, &sets, selected_set)? else {
        return Ok(());
    };
    let selected = &evidence.rows()[selected_set.ordinal() as usize];
    b.charge(selected.children().len())?;
    // This is solely the saved ordered source group. Generated-group repair
    // and the eventual MAIN receiver remain with unresolved selection.
    if !value(row, "mainSocketGroup")
        .and_then(|text| text.parse::<usize>().ok())
        .is_some_and(|index| (1..=selected.children().len()).contains(&index))
        || selected.children().iter().any(|id| {
            let child = &evidence.rows()[id.ordinal() as usize];
            child.occurrence().name() != "Skill"
                || child.occurrence().parent() != Some(selected_set)
                || child.occurrence().has_namespace_context()
                || !matches!(
                    child.authored_instance(),
                    Some(AuthoredInstanceId::SkillGroup(_))
                )
        })
    {
        return Ok(());
    }
    let Some(selection) = saved_selection(b, draft)? else {
        return Ok(());
    };
    // Charge the prospective collection before allocation. No source links,
    // issues, allocator state or completion flags change until final append.
    let count = specs
        .len()
        .checked_add(2)
        .ok_or(NormalizationError::Limit("origin links"))?;
    b.charge(count)?;
    if count > b.limits.max_origin_links {
        return Err(NormalizationError::Limit("origin links"));
    }
    let mut links = Vec::with_capacity(count);
    let mut owners = BTreeSet::new();
    let mut indices = BTreeSet::new();
    for source in specs {
        let Some(&index) = characters.get(source) else {
            return Ok(());
        };
        let Some(character) = draft.character_presets.members.get(index) else {
            return Ok(());
        };
        let origin = &b.origins[source.ordinal() as usize];
        b.charge(origin.links.len().saturating_add(2))?;
        let origin = &b.origins[source.ordinal() as usize];
        if origin.source != *source
            || origin.disposition != SourceDisposition::Contributes
            || !indices.insert(index)
            || !owners.insert(character.id)
            || character.level.to_resolved() != Some(level)
        {
            return Ok(());
        }
        let mut linked = origin.links.iter().filter_map(|link| match link {
            OwnedOriginTarget::CharacterPreset(id) => Some(*id),
            _ => None,
        });
        if linked.next() != Some(character.id) || linked.next().is_some() {
            return Ok(());
        }
        links.push(OwnedOriginTarget::CharacterPreset(character.id));
    }
    links.extend([
        OwnedOriginTarget::SkillPreset(skill),
        OwnedOriginTarget::Issue(selection),
    ]);
    append_links(b, *build, links.into_iter())
}
