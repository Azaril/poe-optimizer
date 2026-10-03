//! A reviewed singleton source actor and its explicitly mapped child actions.
use super::*;
use crate::owned_source::SourceEvidenceRow;
use poe_optimizer_core::owned_build::ParameterValue;

pub(super) struct CompiledMinion {
    actions: BTreeMap<u32, usize>,
    stat_sets: Vec<BTreeMap<u32, ActionStatSetDefId>>,
    main_action_index: ValueRecipe,
    calcs_action_index: ValueRecipe,
    map_skill_index: ValueRecipe,
    map_stat_set_index: ValueRecipe,
}

fn recipe(
    input: &ValueRecipeInput,
    name: &str,
    namespace: &GameVersionNamespace,
    limits: SourceActionLimits,
) -> Result<ValueRecipe> {
    require(
        input.codec.namespace == *namespace
            && input.codec.whitespace == WhitespacePolicy::Exact
            && matches!(input.codec.codec, ValueCodecKind::Integer { .. })
            && input.missing == MissingValuePolicy::Pending
            && input.numeric_aliases.is_empty()
            && input.tiers.len() == 1
            && input.tiers[0].duplicates == DuplicatePolicy::Reject
            && input.tiers[0].selectors
                == [ValueSelector {
                    lane: ValueLane::Attribute,
                    name: name.into(),
                }],
        "minion index recipe",
    )?;
    Ok(ValueRecipe::new(input.clone(), limits.value)?)
}

pub(super) fn compile<I: DefinitionSchemaIndex>(
    input: &SourceActionCorrespondenceInput,
    definitions: &I,
    mappings: &OwnedMappingIndex,
    limits: SourceActionLimits,
    budget: &mut Budget,
) -> Result<CompiledMinion> {
    let SourceFields {
        gem, primary, root, ..
    } = input.fields();
    let MinionFields {
        minion,
        actions,
        absent_action,
        main_action_index,
        calcs_action_index,
        map_skill_index,
        map_stat_set_index,
    } = input.minion_fields().expect("minion correspondence");
    require(
        !minion.source_id.is_empty() && minion.source_id.len() <= 16 * 1024,
        "singleton source minion",
    )?;
    let primary_schema = known(definitions.definition(primary), "primary Skill schema")?;
    let actor_schema = known(definitions.definition(&minion.actor), "Actor schema")?;
    if let RootAuthority::Physical {
        primary_supply,
        entering_grant,
    } = root
    {
        let gem_schema = known(definitions.definition(gem), "Gem schema")?;
        require(
            primary_supply.declaration == SlotOwnerDefId::Gem(gem.clone())
                && entering_grant.declaration == SlotOwnerDefId::Gem(gem.clone())
                && budget.members(&gem_schema.skills.members, primary)?
                && budget.members(&gem_schema.roles, &AuthoredGemRole::SkillUse)?
                && budget.members(
                    &gem_schema.declarations.skill_grants.members,
                    primary_supply,
                )?
                && budget.members(&gem_schema.declarations.grants.members, entering_grant)?,
            "declared minion topology",
        )?;
        let supply = known(definitions.slot(primary_supply), "primary supply schema")?;
        let grant = known(definitions.slot(entering_grant), "primary entering grant")?;
        require(
            supply.skill == *primary
                && grant.target == GrantTarget::Skill(primary_supply.clone())
                && budget.members(&grant.provider_roles, &ProviderRole::SkillUse)?,
            "minion actor correspondence",
        )?;
    }
    require(
        minion.population.declaration == SlotOwnerDefId::Skill(primary.clone())
            && minion.entering_grant.declaration == SlotOwnerDefId::Skill(primary.clone())
            && budget.members(
                &primary_schema.declarations.actors.members,
                &minion.population,
            )?
            && budget.members(
                &primary_schema.declarations.grants.members,
                &minion.entering_grant,
            )?,
        "declared minion topology",
    )?;
    let population = known(
        definitions.slot(&minion.population),
        "minion population schema",
    )?;
    let actor_grant = known(
        definitions.slot(&minion.entering_grant),
        "actor entering grant",
    )?;
    require(
        population.provider_definition.as_ref() == Some(&minion.actor)
            && actor_grant.target == GrantTarget::Actor(minion.population.clone())
            && budget.members(&actor_grant.provider_roles, &ProviderRole::SkillUse)?,
        "minion actor correspondence",
    )?;
    require(
        !actions.is_empty() && actions.len() <= limits.max_map_rows,
        "minion action rows",
    )?;
    let mut indexed = BTreeMap::new();
    let mut skills = BTreeSet::new();
    let mut effects = BTreeSet::new();
    let mut all_sets = Vec::new();
    for (index, row) in actions.iter().enumerate() {
        budget.charge(row.skill_id.len().saturating_add(1))?;
        require(
            row.source_index > 0
                && indexed.insert(row.source_index, index).is_none()
                && skills.insert(&row.skill)
                && !row.skill_id.is_empty()
                && row.skill_id.len() <= 16 * 1024
                && effects.insert(&row.skill_id),
            "unique minion action identity",
        )?;
        let selector = ExternalSelector::Definition(ExternalOwnerSelector::Skill {
            effect_id: SourceComponent::Text(row.skill_id.clone()),
        });
        require(
            matches!(mappings.lookup(&selector),Some(MappingOutcome::Mapped {target:SchemaSubject::Definition(DefinitionAddress::Skill(id)),basis:MappingBasis::Exact}) if id==&row.skill),
            "exact minion child effect mapping",
        )?;
        let child = known(definitions.definition(&row.skill), "minion child Skill")?;
        require(
            row.supply.declaration == SlotOwnerDefId::Actor(minion.actor.clone())
                && row.entering_grant.declaration == SlotOwnerDefId::Actor(minion.actor.clone())
                && row.output.declaration == SlotOwnerDefId::Skill(row.skill.clone())
                && budget.members(&actor_schema.declarations.skill_grants.members, &row.supply)?
                && budget.members(
                    &actor_schema.declarations.grants.members,
                    &row.entering_grant,
                )?
                && budget.members(&population.skills.members, &row.skill)?
                && budget.members(&population.outputs.members, &row.output)?
                && budget.members(&child.declarations.outputs.members, &row.output)?,
            "declared minion child topology",
        )?;
        let supply = known(definitions.slot(&row.supply), "minion child supply")?;
        let grant = known(definitions.slot(&row.entering_grant), "minion child grant")?;
        let output = known(definitions.slot(&row.output), "minion child output")?;
        require(
            supply.skill == row.skill
                && budget.members(&supply.outputs.members, &row.output)?
                && grant.target == GrantTarget::Skill(row.supply.clone())
                && budget.members(&grant.provider_roles, &ProviderRole::SkillUse)?
                && matches!(&output.actor_role, DeclaredActorRole::ProviderActor)
                && budget.members(&output.parts.members, &row.part)?
                && budget.members(&output.modes.members, &row.mode)?,
            "minion child action correspondence",
        )?;
        known(definitions.definition(&row.part), "minion action part")?;
        known(definitions.definition(&row.mode), "minion action mode")?;
        require(
            !row.stat_sets.is_empty() && row.stat_sets.len() <= limits.max_stat_sets,
            "minion stat set rows",
        )?;
        let mut sets = BTreeMap::new();
        let mut ids = BTreeSet::new();
        for set in &row.stat_sets {
            budget.charge(1)?;
            require(
                set.source_index > 0
                    && sets
                        .insert(set.source_index, set.stat_set.clone())
                        .is_none()
                    && ids.insert(&set.stat_set)
                    && budget.members(&output.stat_sets.members, &set.stat_set)?,
                "minion stat set correspondence",
            )?;
            known(
                definitions.definition(&set.stat_set),
                "minion stat set schema",
            )?;
        }
        require(
            row.absent_stat_set
                .as_ref()
                .is_none_or(|id| ids.contains(id)),
            "minion absence stat set",
        )?;
        all_sets.push(sets);
    }
    require(
        absent_action.is_none_or(|index| indexed.contains_key(&index)),
        "minion absence action",
    )?;
    Ok(CompiledMinion {
        actions: indexed,
        stat_sets: all_sets,
        main_action_index: recipe(
            main_action_index,
            "skillMinionSkill",
            definitions.namespace(),
            limits,
        )?,
        calcs_action_index: recipe(
            calcs_action_index,
            "skillMinionSkillCalcs",
            definitions.namespace(),
            limits,
        )?,
        map_skill_index: recipe(
            map_skill_index,
            "skillIndex",
            definitions.namespace(),
            limits,
        )?,
        map_stat_set_index: recipe(
            map_stat_set_index,
            "statSetIndex",
            definitions.namespace(),
            limits,
        )?,
    })
}

fn index(
    row: &SourceEvidenceRow<'_>,
    recipe: &ValueRecipe,
    budget: &mut Budget,
) -> Result<Option<(u32, SourceAttributeRef)>> {
    let selector = &recipe.input().tiers[0].selectors[0];
    let mut candidates = Vec::new();
    for (index, attribute) in row.attributes().iter().enumerate() {
        budget.charge(1)?;
        if attribute.origin().name == selector.name {
            candidates.push(ValueCandidate {
                selector,
                origin: SourceAttributeRef {
                    occurrence: row.occurrence().id(),
                    index: index as u32,
                },
                value: match attribute.decoded() {
                    Ok(value) => CandidateValue::Decoded(value),
                    Err(error) => CandidateValue::Unavailable(error),
                },
            });
        }
    }
    Ok(match recipe.decide(&candidates)?.outcome {
        ValueOutcome::Selected {
            origin,
            value: ParameterValue::Integer(value),
        } => u32::try_from(value.get()).ok().map(|value| (value, origin)),
        _ => None,
    })
}

pub(super) fn inspect(
    adapter: &SourceActionCorrespondence,
    evidence: &SourceProjectEvidence<'_>,
    context: ImportReferenceContext,
    row: &SourceEvidenceRow<'_>,
    budget: &mut Budget,
    ignored: Vec<SourceAttributeRef>,
) -> Result<resolve::Resolution> {
    let skill_id = adapter.input.fields().skill_id;
    let MinionFields {
        minion,
        actions,
        absent_action,
        ..
    } = adapter
        .input
        .minion_fields()
        .expect("minion correspondence");
    let CompiledSourceActions::Minion(compiled) = &adapter.compiled else {
        unreachable!("checked minion adapter")
    };
    let names: &[&str] = match context {
        ImportReferenceContext::Main => &["skillMinion"],
        ImportReferenceContext::Calcs => &["skillMinion", "skillMinionCalcs"],
    };
    let mut actor_attributes = Vec::new();
    for name in names {
        budget.charge(row.attributes().len())?;
        match row
            .attributes()
            .iter()
            .enumerate()
            .find(|(_, a)| a.origin().name == *name)
        {
            Some((index, attribute))
                if attribute.decoded().ok() == Some(minion.source_id.as_str()) =>
            {
                actor_attributes.push(SourceAttributeRef {
                    occurrence: row.occurrence().id(),
                    index: index as u32,
                })
            }
            None if minion.allow_absent => {}
            _ => return Ok(resolve::pending("query-source-minion-identity", ignored)),
        }
    }
    let action_recipe = match context {
        ImportReferenceContext::Main => &compiled.main_action_index,
        ImportReferenceContext::Calcs => &compiled.calcs_action_index,
    };
    let action_name = &action_recipe.input().tiers[0].selectors[0].name;
    budget.charge(row.attributes().len())?;
    let (selected, action_selection) = if row.attribute(action_name).is_some() {
        let Some((value, attribute)) = index(row, action_recipe, budget)? else {
            return Ok(resolve::pending(
                "query-source-minion-action-invalid",
                ignored,
            ));
        };
        (
            value,
            SourceActionSelection::Explicit {
                source_index: value,
                attribute,
            },
        )
    } else if let Some(value) = absent_action {
        (*value, SourceActionSelection::Absent)
    } else {
        return Ok(resolve::pending(
            "query-source-minion-action-absent",
            ignored,
        ));
    };
    let Some(selected_row) = compiled.actions.get(&selected).copied() else {
        return Ok(resolve::pending(
            "query-source-minion-action-unmapped",
            ignored,
        ));
    };
    let wanted = match context {
        ImportReferenceContext::Main => "MinionSkillIndexLookup",
        ImportReferenceContext::Calcs => "MinionSkillIndexLookupCalcs",
    };
    let mut seen = BTreeSet::new();
    let mut accounted = Vec::new();
    let mut selected_stat = None;
    let mut total = 0usize;
    for child_id in row.children() {
        total = total
            .checked_add(1)
            .filter(|n| *n <= adapter.limits.max_map_rows)
            .ok_or(SourceActionError::Limit("map rows"))?;
        let child = &evidence.rows()[child_id.ordinal() as usize];
        let tag = child.occurrence().name();
        if child.occurrence().parent() != Some(row.occurrence().id())
            || !matches!(
                tag,
                "MinionSkillIndexLookup" | "MinionSkillIndexLookupCalcs"
            )
            || !resolve::frame(child, Some(&["grantedEffect"]), false, budget)?
            || resolve::value(child, "grantedEffect") != Some(skill_id)
            || !seen.insert(tag)
        {
            return Ok(resolve::pending("query-source-minion-map-frame", ignored));
        }
        accounted.push(*child_id);
        let mut keys = BTreeSet::new();
        for map_id in child.children() {
            total = total
                .checked_add(1)
                .filter(|n| *n <= adapter.limits.max_map_rows)
                .ok_or(SourceActionError::Limit("map rows"))?;
            let map = &evidence.rows()[map_id.ordinal() as usize];
            if map.occurrence().parent() != Some(*child_id)
                || map.occurrence().name() != "MinionSkillIndexMap"
                || !resolve::frame(map, Some(&["skillIndex", "statSetIndex"]), true, budget)?
            {
                return Ok(resolve::pending("query-source-minion-map-frame", ignored));
            }
            let Some((action, _)) = index(map, &compiled.map_skill_index, budget)? else {
                return Ok(resolve::pending(
                    "query-source-minion-map-action-invalid",
                    ignored,
                ));
            };
            let Some(action_row) = compiled.actions.get(&action).copied() else {
                return Ok(resolve::pending(
                    "query-source-minion-map-action-unmapped",
                    ignored,
                ));
            };
            if !keys.insert(action) {
                return Ok(resolve::pending(
                    "query-source-minion-map-duplicate",
                    ignored,
                ));
            }
            let Some((stat_set, attribute)) = index(map, &compiled.map_stat_set_index, budget)?
            else {
                return Ok(resolve::pending("query-source-selection-invalid", ignored));
            };
            let Some(mapped) = compiled.stat_sets[action_row].get(&stat_set) else {
                return Ok(resolve::pending("query-source-selection-unmapped", ignored));
            };
            accounted.push(*map_id);
            if tag == wanted && action == selected {
                selected_stat = Some((
                    mapped.clone(),
                    SourceActionSelection::Explicit {
                        source_index: stat_set,
                        attribute,
                    },
                ));
            }
        }
    }
    let Some((stat_set, selection)) = selected_stat.or_else(|| {
        actions[selected_row]
            .absent_stat_set
            .as_ref()
            .map(|id| (id.clone(), SourceActionSelection::Absent))
    }) else {
        return Ok(resolve::pending("query-source-selection-absent", ignored));
    };
    Ok(resolve::Resolution {
        selection,
        stat_set: Some(stat_set),
        ignored,
        minion: Some(SourceMinionActionReport {
            actor_attributes,
            action_selection,
            accounted_occurrences: accounted,
        }),
        action: Some(selected_row),
    })
}

pub(super) fn target<L: Clone>(
    adapter: &SourceActionCorrespondence,
    request: &SourceActionRequest<L>,
    action: usize,
    stat_set: ActionStatSetDefId,
) -> ImportActionTarget<L> {
    let MinionFields {
        minion, actions, ..
    } = adapter
        .input
        .minion_fields()
        .expect("minion correspondence");
    let root_path = match adapter.input.fields().root {
        RootAuthority::Physical { entering_grant, .. } => vec![entering_grant.clone()],
        RootAuthority::Direct { .. } => vec![],
    };
    let child = &actions[action];
    let mut action_path = root_path.clone();
    action_path.extend([minion.entering_grant.clone(), child.entering_grant.clone()]);
    ImportActionTarget {
        provider: ImportProviderTarget {
            skill_use: request.skill_use.clone(),
            grant_path: action_path,
        },
        actor: ImportActorTarget::Owned {
            provider: Box::new(ImportProviderTarget {
                skill_use: request.skill_use.clone(),
                grant_path: root_path,
            }),
            slot: minion.population.clone(),
        },
        output: child.output.clone(),
        part: child.part.clone(),
        mode: child.mode.clone(),
        stat_set,
    }
}
