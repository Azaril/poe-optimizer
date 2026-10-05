use super::*;
use crate::owned_normalize::source_shape::{charge_frame, container_text, plain_row, value};

pub(super) fn positive(text: &str) -> bool {
    !text.is_empty()
        && !text.starts_with('0')
        && text.bytes().all(|b| b.is_ascii_digit())
        && text.parse::<u64>().is_ok_and(|n| n < (1_u64 << 53))
}
pub(in crate::owned_normalize) struct Context<'a> {
    pub skills: &'a BTreeMap<SourceOccurrenceId, usize>,
    pub specs: &'a BTreeMap<SourceOccurrenceId, usize>,
    pub equipment: &'a BTreeMap<SourceOccurrenceId, usize>,
    pub items: &'a BTreeMap<SourceOccurrenceId, ItemRecordId>,
}
struct Selected {
    skills: Option<SourceOccurrenceId>,
    spec: Option<SourceOccurrenceId>,
    items: Option<(SourceOccurrenceId, SourceOccurrenceId)>,
}
struct Candidate {
    source: SourceOccurrenceId,
    group: SourceOccurrenceId,
    provider_sources: Vec<SourceOccurrenceId>,
    target: GeneratedSkillKey,
    slot: DeclaredSlot<ParameterSlotDefId>,
    value: Option<ParameterValue>,
}

fn selected(b: &mut Builder<'_, '_>, sets: Option<&[SourceOccurrenceId]>) -> Result<Selected> {
    let evidence = b.evidence;
    let mut result = Selected {
        skills: None,
        spec: None,
        items: None,
    };
    if let Some(sets) = sets {
        b.charge(sets.len())?;
        for set in sets {
            let row = &evidence.rows()[set.ordinal() as usize];
            let parent = &evidence.rows()
                [row.occurrence().parent().expect("checked set").ordinal() as usize];
            if value(row, "id") == value(parent, "activeSkillSet") {
                result.skills = Some(*set);
            }
        }
    }
    let root = &evidence.rows()[0];
    b.charge(root.children().len())?;
    let trees: Vec<_> = root
        .children()
        .iter()
        .map(|id| &evidence.rows()[id.ordinal() as usize])
        .filter(|row| matches!(row.occurrence().name(), "Tree" | "Spec"))
        .collect();
    if let [tree] = trees.as_slice() {
        charge_frame(b, tree, &["activeSpec"])?;
        if tree.occurrence().name() == "Tree"
            && plain_row(tree, &["activeSpec"], false)
            && container_text(tree)
            && let Some(index) = value(tree, "activeSpec")
                .filter(|v| positive(v))
                .and_then(|v| v.parse::<usize>().ok())
        {
            let mut valid = true;
            for id in tree.children() {
                let row = &evidence.rows()[id.ordinal() as usize];
                b.charge(1)?;
                valid &= row.occurrence().name() == "Spec"
                    && !row.occurrence().has_namespace_context()
                    && row.occurrence().parent() == Some(tree.occurrence().id())
                    && matches!(
                        row.authored_instance(),
                        Some(AuthoredInstanceId::PassiveSpec(_))
                    );
            }
            if valid {
                result.spec = tree.children().get(index - 1).copied();
            }
        }
    }
    if let Some(parent) = equipment_membership::ordinary_items(b)? {
        let row = &evidence.rows()[parent.ordinal() as usize];
        charge_frame(b, row, &["activeItemSet"])?;
        if let Some(key) = value(row, "activeItemSet").filter(|v| positive(v)) {
            for id in row.children() {
                let set = &evidence.rows()[id.ordinal() as usize];
                b.charge(1)?;
                if set.occurrence().name() == "ItemSet" && value(set, "id") == Some(key) {
                    result.items = Some((parent, *id));
                }
            }
        }
    }
    Ok(result)
}

fn unique<T>(mut values: impl Iterator<Item = T>) -> Option<T> {
    let first = values.next()?;
    values.next().is_none().then_some(first)
}
fn scalar(
    b: &mut Builder<'_, '_>,
    row: &SourceEvidenceRow<'_>,
    recipe: &ValueRecipe,
) -> Result<Option<ParameterValue>> {
    for selector in recipe.input().tiers.iter().flat_map(|tier| &tier.selectors) {
        b.charge(
            row.attribute(&selector.name)
                .map_or(0, |attribute| attribute.raw().len()),
        )?;
    }
    match b.scalar(row, recipe) {
        Err(NormalizationError::Value(ValuePolicyError::MultipleValues { .. })) => Ok(None),
        result => result,
    }
}

fn tree_provider(
    b: &mut Builder<'_, '_>,
    draft: &DraftSessionInput,
    context: &Context<'_>,
    selected: &Selected,
    group: &SourceEvidenceRow<'_>,
    bound: &BoundRow<'_>,
) -> Result<Option<(ProviderRoot, BoundedInteger, Vec<SourceOccurrenceId>)>> {
    let GeneratedSkillInputProvider::TreeAllocation {
        source_node_id,
        passive,
        ..
    } = &bound.row.provider
    else {
        unreachable!()
    };
    if value(group, "slot").is_some()
        || value(group, "source") != Some(format!("Tree:{source_node_id}").as_str())
    {
        return Ok(None);
    }
    let Some(spec) = selected.spec else {
        return Ok(None);
    };
    let Some(&index) = context.specs.get(&spec) else {
        return Ok(None);
    };
    let row = &b.evidence.rows()[spec.ordinal() as usize];
    charge_frame(b, row, &["treeVersion", "nodes"])?;
    // The existing materializer owns the node-token grammar. Here duplicate
    // source attributes and a nonexact external correspondence cannot pick a root.
    let names: BTreeSet<_> = row.attributes().iter().map(|a| &a.origin().name).collect();
    if names.len() != row.attributes().len()
        || row
            .attributes()
            .iter()
            .any(|a| a.origin().namespace.is_some() || a.decoded().is_err())
    {
        return Ok(None);
    }
    let Some(version) = value(row, "treeVersion") else {
        return Ok(None);
    };
    let selector = ExternalSelector::Definition(ExternalOwnerSelector::PassiveNode {
        tree_version: SourceComponent::Text(version.into()),
        node_id: SourceComponent::Text(source_node_id.clone()),
        view: SourceComponent::Missing,
    });
    if !matches!(b.mappings.lookup(&selector), Some(MappingOutcome::Mapped {
        target: SchemaSubject::Definition(DefinitionAddress::PassiveNode(node)), basis: MappingBasis::Exact,
    }) if node == passive)
    {
        return Ok(None);
    }
    let preset = &draft.allocation_presets.members[index];
    b.charge(preset.allocations.members.len() + draft.allocations.members.len())?;
    let ids: BTreeSet<_> = preset.allocations.members.iter().copied().collect();
    let allocation =
        unique(draft.allocations.members.iter().filter(|row| {
            ids.contains(&row.id) && row.node.to_resolved().as_ref() == Some(passive)
        }));
    let Some(allocation) = allocation else {
        return Ok(None);
    };
    b.charge(b.origins[spec.ordinal() as usize].links.len())?;
    if b.origins[spec.ordinal() as usize]
        .links
        .iter()
        .filter(|link| matches!(link, OwnedOriginTarget::Allocation(id) if *id == allocation.id))
        .count()
        != 1
    {
        return Ok(None);
    }
    let GeneratedSkillProviderLevel::Fixed { value } = bound.row.provider_level else {
        unreachable!()
    };
    Ok(Some((
        ProviderRoot::Allocation(allocation.id),
        value,
        vec![spec],
    )))
}

fn item_provider(
    b: &mut Builder<'_, '_>,
    draft: &DraftSessionInput,
    context: &Context<'_>,
    selected: &Selected,
    group: &SourceEvidenceRow<'_>,
    bound: &BoundRow<'_>,
) -> Result<Option<(ProviderRoot, BoundedInteger, Vec<SourceOccurrenceId>)>> {
    let GeneratedSkillInputProvider::ItemModifier {
        modifier,
        template,
        source_name,
        name_lines,
        ..
    } = &bound.row.provider
    else {
        unreachable!()
    };
    let Some((parent, set)) = selected.items else {
        return Ok(None);
    };
    let Some(&preset) = context.equipment.get(&set) else {
        return Ok(None);
    };
    let Some((item_key, name)) = value(group, "source")
        .and_then(|v| v.strip_prefix("Item:"))
        .and_then(|v| v.split_once(':'))
    else {
        return Ok(None);
    };
    if !positive(item_key) || name != source_name {
        return Ok(None);
    }
    let Some(slot_name) = value(group, "slot") else {
        return Ok(None);
    };
    let evidence = b.evidence;
    let set_row = &evidence.rows()[set.ordinal() as usize];
    charge_frame(b, set_row, &["id"])?;
    if !plain_row(set_row, &["id", "title", "useSecondWeaponSet"], false)
        || !container_text(set_row)
    {
        return Ok(None);
    }
    let mut names = BTreeSet::new();
    let mut matched = None;
    for id in set_row.children() {
        let slot = &evidence.rows()[id.ordinal() as usize];
        charge_frame(b, slot, &["name", "itemId"])?;
        if slot.occurrence().name() == "SocketIdURL" {
            if !plain_row(slot, &["nodeId", "itemPbURL", "name"], true) {
                return Ok(None);
            }
            continue;
        }
        if slot.occurrence().name() != "Slot"
            || !plain_row(
                slot,
                &["name", "itemId", "active", "itemPbURL", "note"],
                true,
            )
        {
            return Ok(None);
        }
        let Some(name) = value(slot, "name") else {
            return Ok(None);
        };
        if !names.insert(name) {
            return Ok(None);
        }
        if name == slot_name && value(slot, "itemId") == Some(item_key) {
            matched = Some(*id);
        }
    }
    let Some(slot) = matched else { return Ok(None) };
    let Ok(SourceKeyLookup::Unique(item_source)) = evidence.lookup_key(SourceKeyQuery {
        parent: Some(parent),
        element: SourceQName {
            namespace: None,
            local: "Item",
        },
        attribute: SourceQName {
            namespace: None,
            local: "id",
        },
        value: item_key,
    }) else {
        return Ok(None);
    };
    let source = item_source.occurrence;
    let Some(item_id) = context.items.get(&source) else {
        return Ok(None);
    };
    b.charge(
        b.origins[slot.ordinal() as usize].links.len()
            + draft.items.members.len()
            + draft.equipment.members.len()
            + draft.equipment_presets.members[preset]
                .equipment
                .members
                .len()
            + b.item_texts.len(),
    )?;
    let Some(equipment_id) = unique(b.origins[slot.ordinal() as usize].links.iter().filter_map(
        |link| {
            if let OwnedOriginTarget::Equipment(id) = link {
                Some(*id)
            } else {
                None
            }
        },
    )) else {
        return Ok(None);
    };
    let Some(equipment) = draft
        .equipment
        .members
        .iter()
        .find(|row| row.id == equipment_id)
    else {
        return Ok(None);
    };
    if equipment.item.to_resolved().as_ref() != Some(item_id)
        || !draft.equipment_presets.members[preset]
            .equipment
            .members
            .contains(&equipment_id)
    {
        return Ok(None);
    }
    let Some(item) = draft.items.members.iter().find(|row| &row.id == item_id) else {
        return Ok(None);
    };
    if item.template.to_resolved().as_ref() != Some(template)
        || !matches!(item.modifiers.completion, DraftListCompletion::Complete)
    {
        return Ok(None);
    }
    let Some(text_index) = unique(
        b.item_texts
            .iter()
            .enumerate()
            .filter_map(|(index, row)| (row.source == source).then_some(index)),
    ) else {
        return Ok(None);
    };
    // This postpass does not append or reorder Item text records.
    let text = &b.item_texts[text_index];
    if !matches!(text.attribution.layout, ItemLayoutStatus::Proven) {
        return Ok(None);
    }
    let inspected = text.attribution.lines.len()
        + text
            .attribution
            .lines
            .iter()
            .map(|line| line.semantic_text.len())
            .sum::<usize>()
        + name_lines.iter().map(|line| line.text.len()).sum::<usize>();
    b.charge(inspected)?;
    let text = &b.item_texts[text_index];
    // The source preamble uses nonblank normalized text positions; attribution
    // separately retains one-based raw line numbers for provenance. Blank XML
    // formatting lines must not shift the injected rarity/title/base selectors.
    let preamble: Vec<_> = text
        .attribution
        .lines
        .iter()
        .map(|line| line.semantic_text.trim_ascii())
        .filter(|line| !line.is_empty())
        .collect();
    if name_lines
        .iter()
        .any(|expected| preamble.get(expected.index).copied() != Some(expected.text.as_str()))
    {
        return Ok(None);
    }
    let line_work = text.lines.len()
        + text
            .lines
            .iter()
            .map(|line| line.modifiers.len())
            .sum::<usize>();
    b.charge(item.modifiers.members.len())?;
    let Some(rolled) = unique(
        item.modifiers
            .members
            .iter()
            .filter(|row| row.definition.to_resolved().as_ref() == Some(modifier)),
    ) else {
        return Ok(None);
    };
    let GeneratedSkillProviderLevel::ModifierRoll { slot: level_slot } = &bound.row.provider_level
    else {
        unreachable!()
    };
    b.charge(rolled.rolls.members.len())?;
    let Some(level) = unique(
        rolled
            .rolls
            .members
            .iter()
            .filter(|row| row.slot.to_resolved().as_ref() == Some(level_slot)),
    ) else {
        return Ok(None);
    };
    let Some(ParameterValue::Integer(level)) = level.value.to_resolved() else {
        return Ok(None);
    };
    // Attribute the selected grant to one actual converted source line, not just
    // a matching definition somewhere in an Item's pending text.
    b.charge(line_work)?;
    let text = &b.item_texts[text_index];
    if text
        .lines
        .iter()
        .flat_map(|line| &line.modifiers)
        .filter(|id| **id == rolled.id)
        .count()
        != 1
    {
        return Ok(None);
    }
    Ok(Some((
        ProviderRoot::ItemModifier {
            equipment_use: equipment_id,
            modifier: rolled.id,
        },
        level,
        vec![set, slot, source],
    )))
}

fn candidate(
    b: &mut Builder<'_, '_>,
    draft: &DraftSessionInput,
    context: &Context<'_>,
    selected: &Selected,
    group: &SourceEvidenceRow<'_>,
    gem: &SourceEvidenceRow<'_>,
    bound: &BoundRow<'_>,
) -> Result<Option<Candidate>> {
    if value(gem, "skillId") != Some(bound.row.skill_id.as_str())
        || value(gem, "nameSpec") != Some(bound.row.name_spec.as_str())
    {
        return Ok(None);
    }
    let provider = match bound.row.provider {
        GeneratedSkillInputProvider::TreeAllocation { .. } => {
            tree_provider(b, draft, context, selected, group, bound)?
        }
        GeneratedSkillInputProvider::ItemModifier { .. } => {
            item_provider(b, draft, context, selected, group, bound)?
        }
    };
    let Some((root, level, provider_sources)) = provider else {
        return Ok(None);
    };
    if scalar(b, gem, &bound.level)? != Some(ParameterValue::Integer(level)) {
        return Ok(None);
    }
    let quality =
        scalar(b, gem, &bound.quality)?.filter(|v| gem_inputs::value_valid(v, &bound.schema.value));
    Ok(Some(Candidate {
        source: gem.occurrence().id(),
        group: group.occurrence().id(),
        provider_sources,
        target: GeneratedSkillKey {
            provider: ProviderKey {
                root,
                grant_path: vec![],
            },
            slot: bound.row.provider.supply().clone(),
        },
        slot: bound.row.parameters[0].slot.clone(),
        value: quality,
    }))
}

pub(in crate::owned_normalize) fn materialize(
    b: &mut Builder<'_, '_>,
    draft: &mut DraftSessionInput,
    compiled: Option<&CompiledGeneratedInputs<'_>>,
    context: Context<'_>,
) -> Result<()> {
    let Some(compiled) = compiled else {
        return Ok(());
    };
    let evidence = b.evidence;
    let frames = skill_source_census::container_sets(b)?;
    let selected = selected(b, frames.as_deref())?;
    b.charge(frames.as_ref().map_or(0, Vec::len))?;
    let frame_index = frames.map(|rows| rows.into_iter().collect::<BTreeSet<_>>());
    let mut plans = Vec::new();
    for (set, index) in context.skills {
        b.charge(1)?;
        let mut proven = frame_index.as_ref().is_some_and(|rows| rows.contains(set));
        let mut candidates: BTreeMap<GeneratedSkillKey, Vec<Candidate>> = BTreeMap::new();
        if proven {
            // Reconstruction chooses a saved group before our narrower scalar
            // admission. An unsupported competing row must not make a later row
            // appear unique. Conservatively census source+slot, even when its
            // level spelling, first effect or additional children are unreviewed.
            let mut joins = BTreeMap::new();
            let mut unambiguous_identity = true;
            let mut group_frames = Vec::new();
            for group_id in evidence.rows()[set.ordinal() as usize].children() {
                let group = &evidence.rows()[group_id.ordinal() as usize];
                charge_frame(b, group, &["source", "slot"])?;
                for name in ["source", "slot"] {
                    let attributes: Vec<_> = group
                        .attributes()
                        .iter()
                        .filter(|a| a.origin().name == name)
                        .collect();
                    if attributes.len() > 1
                        || attributes
                            .iter()
                            .any(|a| a.origin().namespace.is_some() || a.decoded().is_err())
                    {
                        unambiguous_identity = false;
                    }
                }
                let source = value(group, "source");
                let slot = value(group, "slot");
                if let Some(source) = source {
                    *joins.entry((source, slot)).or_insert(0usize) += 1;
                }
                let valid = group.occurrence().name() == "Skill"
                    && plain_row(group, skill_source_census::GROUP_ATTRIBUTES, false)
                    && container_text(group);
                b.charge(1)?;
                group_frames.push((
                    group,
                    valid,
                    source,
                    slot,
                    group.attribute("source").is_some(),
                ));
            }
            if !unambiguous_identity {
                proven = false;
            }
            // Admission reuses the exact inspected frame. The pre-admission
            // ambiguity census still includes unsupported scalar/child rows.
            for (group, valid, source, slot, has_source) in group_frames {
                b.charge(1)?;
                if !valid {
                    proven = false;
                    continue;
                }
                // Only actual absent source is manual. Empty and literal nil
                // are truthy source identities in the reference runtime.
                if !has_source {
                    continue;
                }
                if !unambiguous_identity
                    || selected.skills != Some(*set)
                    || group.children().len() != 1
                    || source.and_then(|source| joins.get(&(source, slot))) != Some(&1)
                {
                    proven = false;
                    continue;
                }
                let gem = &evidence.rows()[group.children()[0].ordinal() as usize];
                charge_frame(b, gem, &["gemId", "variantId", "skillId", "nameSpec"])?;
                if gem.occurrence().name() != "Gem"
                    || !plain_row(gem, skill_source_census::GEM_ATTRIBUTES, true)
                    || !matches!(
                        gem.authored_instance(),
                        Some(AuthoredInstanceId::SkillEntry(_))
                    )
                {
                    proven = false;
                    continue;
                }
                let identity = value(gem, "gemId").zip(value(gem, "variantId"));
                let Some(rows) = identity.and_then(|id| compiled.rows.get(&id)) else {
                    proven = false;
                    continue;
                };
                b.charge(
                    rows.len()
                        + b.origins[gem.occurrence().id().ordinal() as usize]
                            .links
                            .len(),
                )?;
                if b.origins[gem.occurrence().id().ordinal() as usize]
                    .links
                    .iter()
                    .any(|link| {
                        matches!(
                            link,
                            OwnedOriginTarget::Gem(_)
                                | OwnedOriginTarget::Skill(_)
                                | OwnedOriginTarget::Support(_)
                        )
                    })
                {
                    proven = false;
                    continue;
                }
                let mut matched = Vec::new();
                for row in rows {
                    if let Some(value) = candidate(b, draft, &context, &selected, group, gem, row)?
                    {
                        matched.push(value);
                    }
                }
                if matched.len() != 1 {
                    proven = false;
                    continue;
                }
                let row = matched.pop().expect("one exact provider");
                candidates.entry(row.target.clone()).or_default().push(row);
            }
        }
        let mut rows = Vec::new();
        for (_, mut matches) in candidates {
            b.charge(matches.len())?;
            if matches.len() == 1 {
                rows.push(matches.pop().expect("one saved occurrence"));
            } else {
                proven = false;
            }
        }
        plans.push((*set, *index, proven, rows));
    }
    for (set, index, proven, rows) in plans {
        let preset = &mut draft.skill_presets.members[index];
        if preset.intent.is_some() {
            return invalid("normalization generated intent already exists");
        }
        b.charge(
            preset
                .usage_preferences
                .as_ref()
                .map_or(0, |usage| usage.members.len()),
        )?;
        let usage = match preset.usage_preferences.take() {
            Some(legacy) => DraftList {
                completion: legacy.completion,
                members: legacy
                    .members
                    .into_iter()
                    .map(|selection| PresetUsageBindingDraft {
                        selection,
                        applicability: PresetApplicability::Required,
                    })
                    .collect(),
            },
            // This explicit opt-in conversion preserves legacy absence. It does
            // not retire any existing source/usage/configuration obligation.
            None => complete(vec![]),
        };
        let mut members = Vec::new();
        for row in rows {
            let value = match row.value {
                Some(v) => v.into(),
                None => b.pending(row.source, "generated-skill-input-value-unresolved")?,
            };
            members.push(GeneratedSkillInputBindingDraft {
                target: row.target.clone().into(),
                parameters: complete(vec![ParameterDraft {
                    slot: row.slot.into(),
                    value,
                }]),
                applicability: PresetApplicability::WhenExactSourceSelected,
            });
            let link = OwnedOriginTarget::GeneratedSkillInput {
                skill_preset: preset.id,
                target: row.target,
            };
            let sources: BTreeSet<_> = row
                .provider_sources
                .into_iter()
                .chain([set, row.group, row.source])
                .collect();
            b.charge(sources.len())?;
            for source in sources {
                b.link(source, link.clone())?;
            }
        }
        let generated_inputs = if proven {
            complete(members)
        } else {
            b.closure(set, "generated-skill-inputs-not-converted", members)?
        };
        preset.intent = Some(SkillPresetIntentDraftV1 {
            schema_version: 1,
            usage,
            generated_inputs,
        });
    }
    Ok(())
}
