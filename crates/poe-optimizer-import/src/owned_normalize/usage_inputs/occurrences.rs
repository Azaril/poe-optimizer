//! Occurrence target proof and typed usage transport are separate authorities.
//! No row here closes physical inputs, gameplay usage, or generated input lists.
use super::*;
use generated_skill_sources::{CompiledSources, Context, GeneratedSkillSourceRule};
use poe_optimizer_core::owned_preset_intent::PresetApplicability;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OccurrenceUsageRule {
    pub target: OccurrenceUsageTarget,
    pub attributes: Vec<String>,
    pub guards: Vec<GemInputGuard>,
    pub group_attributes: Vec<String>,
    pub group_guards: Vec<GemInputGuard>,
    pub policies: Vec<OccurrenceUsagePolicy>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OccurrenceUsageTarget {
    AuthoredDirect {
        gem: GemDefId,
        game_id: String,
        variant_id: String,
        skill_id: String,
        name_spec: String,
        skill: Box<SkillDefId>,
    },
    Generated {
        correspondence: Box<GeneratedSkillSourceRule>,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OccurrenceUsagePolicy {
    pub policy: UsagePolicyDefId,
    pub parameters: Vec<UsageParameterInput>,
}
struct BoundPolicy<'p> {
    policy: &'p UsagePolicyDefId,
    parameters: Vec<BoundParameter<'p>>,
}
struct BoundOccurrence<'p> {
    row: &'p OccurrenceUsageRule,
    attributes: Vec<&'p str>,
    group_attributes: Vec<&'p str>,
    policies: Vec<BoundPolicy<'p>>,
}
pub(super) struct CompiledOccurrences<'p> {
    rows: Vec<BoundOccurrence<'p>>,
    direct: BTreeMap<(&'p str, &'p str), usize>,
    generated: CompiledSources<'p>,
}
impl OccurrenceUsageTarget {
    fn identity(&self) -> (&GemDefId, &str, &str, &str, &str, &SkillDefId) {
        match self {
            Self::AuthoredDirect {
                gem,
                game_id,
                variant_id,
                skill_id,
                name_spec,
                skill,
            } => (
                gem,
                game_id,
                variant_id,
                skill_id,
                name_spec,
                skill.as_ref(),
            ),
            Self::Generated { correspondence: c } => (
                &c.gem,
                &c.game_id,
                &c.variant_id,
                &c.skill_id,
                &c.name_spec,
                &c.skill,
            ),
        }
    }
}
fn attributes(names: &[String]) -> Result<Vec<&str>> {
    if names.is_empty()
        || names.len() > 64
        || names.iter().any(|name| name.is_empty() || name.len() > 128)
        || names.iter().collect::<BTreeSet<_>>().len() != names.len()
    {
        return invalid("occurrence usage attribute frame");
    }
    Ok(names.iter().map(String::as_str).collect())
}
pub(super) fn compile<'p, I: DefinitionSchemaIndex>(
    rows: &'p [OccurrenceUsageRule],
    definitions: &I,
    roles: &OwnedSkillRoleIndex,
    mappings: &OwnedMappingIndex,
    work: &mut usize,
    limits: NormalizationLimits,
) -> Result<CompiledOccurrences<'p>> {
    if rows.is_empty() || rows.len() > 256 || rows.len() > limits.mapping.max_entries {
        return invalid("occurrence usage rows");
    }
    let mut result = CompiledOccurrences {
        rows: Vec::new(),
        direct: BTreeMap::new(),
        generated: CompiledSources::new(),
    };
    let source_context = generated_skill_sources::CompilationContext {
        definitions,
        roles,
        mappings,
        limits,
    };
    for (index, row) in rows.iter().enumerate() {
        charge(work, 1, limits)?;
        let (gem, game, variant, effect, name, skill) = row.target.identity();
        if [game, variant, effect, name].iter().any(|text| {
            text.is_empty()
                || text.trim() != *text
                || text.chars().any(char::is_control)
                || text.len() > limits.mapping.max_string_bytes
        }) {
            return invalid("occurrence usage source identity");
        }
        match &row.target {
            OccurrenceUsageTarget::Generated { correspondence } => result.generated.insert(
                correspondence.as_ref().into(),
                index,
                &source_context,
                work,
            )?,
            OccurrenceUsageTarget::AuthoredDirect { .. } => {
                let selector = ExternalSelector::Definition(ExternalOwnerSelector::Gem {
                    game_id: SourceComponent::Text(game.into()),
                    variant_id: SourceComponent::Text(variant.into()),
                });
                let effect = ExternalSelector::Definition(ExternalOwnerSelector::Skill {
                    effect_id: SourceComponent::Text(effect.into()),
                });
                if !matches!(roles.lookup(&selector), Some(MappingOutcome::Mapped { target: SchemaSubject::Definition(DefinitionAddress::Gem(id)), basis: MappingBasis::Exact }) if id == gem)
                    || !matches!(mappings.lookup(&effect), Some(MappingOutcome::Mapped { target: SchemaSubject::Definition(DefinitionAddress::Skill(id)), basis: MappingBasis::Exact }) if id == skill)
                    || !roles.role(gem).is_some_and(|role| {
                        role.materialization == OwnedGemMaterialization::ProviderOnly
                            && role.role == OwnedGemRole::Known(AuthoredGemRole::SkillUse)
                            && role.primary == OwnedPrimarySkill::Known(skill.clone())
                    })
                    || !matches!(definitions.definition(skill), SchemaLookup::Known(s) if s.directly_selectable)
                    || result.direct.insert((game, variant), index).is_some()
                {
                    return invalid("occurrence usage Direct source authority");
                }
            }
        }
        let attributes = attributes(&row.attributes)?;
        let group_attributes = self::attributes(&row.group_attributes)?;
        let attr: BTreeSet<_> = attributes.iter().copied().collect();
        let groups: BTreeSet<_> = group_attributes.iter().copied().collect();
        if ["gemId", "variantId", "skillId", "nameSpec"]
            .iter()
            .any(|name| !attr.contains(name))
            || !groups.contains("source")
        {
            return invalid("occurrence usage source frame");
        }
        validate_guards(&row.guards, &attr, limits)?;
        validate_guards(&row.group_guards, &groups, limits)?;
        if row.guards.len() > 64
            || row.group_guards.len() > 64
            || row.policies.is_empty()
            || row.policies.len() > 64
        {
            return invalid("occurrence usage policies");
        }
        let mut ids = BTreeSet::new();
        let mut policies = Vec::new();
        for policy in &row.policies {
            charge(work, 1, limits)?;
            let SchemaLookup::Known(schema) = definitions.definition(&policy.policy) else {
                return invalid("occurrence usage policy schema");
            };
            charge(
                work,
                schema.targets.len() + schema.declarations.parameters.members.len(),
                limits,
            )?;
            if !ids.insert(&policy.policy)
                || !schema.targets.contains(&UsageTargetKind::Skill)
                || !schema.declarations.parameters.is_complete()
                || policy.parameters.is_empty()
                || policy.parameters.len() > 64
                || schema.declarations.parameters.members.len() != policy.parameters.len()
            {
                return invalid("occurrence usage policy parameters");
            }
            let mut slots = BTreeSet::new();
            let mut parameters = Vec::new();
            for parameter in &policy.parameters {
                charge(work, schema.declarations.parameters.members.len(), limits)?;
                if parameter.slot.declaration != SlotOwnerDefId::UsagePolicy(policy.policy.clone())
                    || !slots.insert(&parameter.slot)
                    || !schema
                        .declarations
                        .parameters
                        .members
                        .contains(&parameter.slot)
                {
                    return invalid("occurrence usage parameter authority");
                }
                let SchemaLookup::Known(value) = definitions.slot(&parameter.slot) else {
                    return invalid("occurrence usage parameter schema");
                };
                if value.presence != SlotPresence::RequiredOnce
                    || value.sites != [ParameterSite::UsagePolicyParameter]
                {
                    return invalid("occurrence usage parameter site");
                }
                let (occurrence, group, fallback) = match &parameter.source {
                    UsageValueSource::Occurrence { value } => {
                        (value, None, BoundFallback::RequestedOccurrence)
                    }
                    UsageValueSource::ContainingGroupOverride {
                        group,
                        occurrence,
                        fallback_admission,
                    } => (
                        occurrence,
                        Some(group),
                        compile_fallback(fallback_admission, gem, skill, roles, work, limits)?,
                    ),
                };
                parameters.push(BoundParameter {
                    slot: parameter.slot.clone(),
                    occurrence: numeric_recipe(
                        occurrence,
                        &value.value,
                        &attr,
                        definitions,
                        limits,
                        true,
                    )?,
                    group: group
                        .map(|input| {
                            numeric_recipe(input, &value.value, &groups, definitions, limits, true)
                        })
                        .transpose()?,
                    schema: value.value.clone(),
                    fallback,
                });
            }
            policies.push(BoundPolicy {
                policy: &policy.policy,
                parameters,
            });
        }
        result.rows.push(BoundOccurrence {
            row,
            attributes,
            group_attributes,
            policies,
        });
    }
    Ok(result)
}
struct Projection {
    source: SourceOccurrenceId,
    group: SourceOccurrenceId,
    preset: usize,
    target: UsageTarget,
    applicability: PresetApplicability,
    rule: usize,
    origins: Vec<SourceOccurrenceId>,
}
fn frame(
    b: &mut Builder<'_, '_>,
    row: &SourceEvidenceRow<'_>,
    group: &SourceEvidenceRow<'_>,
    bound: &BoundOccurrence<'_>,
) -> Result<bool> {
    source_shape::charge_row(b, row)?;
    source_shape::charge_row(b, group)?;
    Ok(row.occurrence().name() == "Gem"
        && group.occurrence().name() == "Skill"
        && row.occurrence().parent() == Some(group.occurrence().id())
        && source_shape::plain_row(row, &bound.attributes, true)
        && source_shape::plain_row(group, &bound.group_attributes, false)
        && source_shape::container_text(group)
        && b.gem_guards_match(row, &bound.row.guards)?
        && b.gem_guards_match(group, &bound.row.group_guards)?)
}
fn direct_plans(
    b: &mut Builder<'_, '_>,
    draft: &DraftSessionInput,
    compiled: &CompiledOccurrences<'_>,
    context: Context<'_>,
) -> Result<Vec<Projection>> {
    let Some(sets) = skill_source_census::container_sets(b)? else {
        return Ok(vec![]);
    };
    let evidence = b.evidence;
    let mut plans = Vec::new();
    for set in sets {
        let Some(&preset) = context.skills.get(&set) else {
            continue;
        };
        b.charge(draft.skill_presets.members[preset].skills.members.len())?;
        for group in evidence.rows()[set.ordinal() as usize].children() {
            let group = &evidence.rows()[group.ordinal() as usize];
            source_shape::charge_frame(b, group, &["source"])?;
            // Source="" is truthy in PoB: a successful historical raw import
            // does not prove that the manual runtime occurrence was retained.
            if group.attribute("source").is_some() {
                continue;
            }
            for source in group.children() {
                let row = &evidence.rows()[source.ordinal() as usize];
                source_shape::charge_frame(b, row, &["gemId", "variantId", "skillId", "nameSpec"])?;
                let Some(identity) =
                    source_shape::value(row, "gemId").zip(source_shape::value(row, "variantId"))
                else {
                    continue;
                };
                let Some(&rule) = compiled.direct.get(&identity) else {
                    continue;
                };
                let bound = &compiled.rows[rule];
                let (_, _, _, effect, name, skill) = bound.row.target.identity();
                if source_shape::value(row, "skillId") != Some(effect)
                    || source_shape::value(row, "nameSpec") != Some(name)
                    || !frame(b, row, group, bound)?
                {
                    continue;
                }
                b.charge(
                    b.origins[source.ordinal() as usize].links.len() + draft.skills.members.len(),
                )?;
                let ids: Vec<_> = b.origins[source.ordinal() as usize]
                    .links
                    .iter()
                    .filter_map(|link| {
                        if let OwnedOriginTarget::Skill(id) = link {
                            Some(*id)
                        } else {
                            None
                        }
                    })
                    .collect();
                let [id] = ids.as_slice() else {
                    continue;
                };
                if !exact_preset(b, &draft.skill_presets.members[preset], *id, set)?
                    || !draft.skills.members.iter().any(|row| row.id == *id && matches!(&row.source, DraftAuthoredSkillSource::Direct(value) if value.to_resolved().as_ref() == Some(skill))) { continue; }
                plans.push(Projection {
                    source: *source,
                    group: group.occurrence().id(),
                    preset,
                    target: UsageTarget::Skill(SkillTarget::Authored(*id)),
                    applicability: PresetApplicability::Required,
                    rule,
                    origins: vec![set],
                });
            }
        }
    }
    Ok(plans)
}
pub(super) fn materialize(
    b: &mut Builder<'_, '_>,
    draft: &mut DraftSessionInput,
    compiled: &CompiledOccurrences<'_>,
    context: Context<'_>,
) -> Result<()> {
    let mut plans = direct_plans(b, draft, compiled, context)?;
    for generated_skill_sources::ResolvedPreset {
        source: set,
        preset_index: preset,
        sources: rows,
        ..
    } in generated_skill_sources::resolve(b, draft, &compiled.generated, context)?
    {
        for row in rows {
            let source = &b.evidence.rows()[row.source.ordinal() as usize];
            let group = &b.evidence.rows()[row.group.ordinal() as usize];
            if !frame(b, source, group, &compiled.rows[row.index])? {
                continue;
            }
            plans.push(Projection {
                source: row.source,
                group: row.group,
                preset,
                target: UsageTarget::Skill(SkillTarget::Generated(Box::new(row.target))),
                applicability: PresetApplicability::WhenExactSourceSelected,
                rule: row.index,
                origins: row.provider_sources.into_iter().chain([set]).collect(),
            });
        }
    }
    for plan in plans {
        let bound = &compiled.rows[plan.rule];
        let source = &b.evidence.rows()[plan.source.ordinal() as usize];
        let group = &b.evidence.rows()[plan.group.ordinal() as usize];
        let preset = &mut draft.skill_presets.members[plan.preset];
        // Only an actual proved projection adds a usage obligation. Unmatched
        // presets retain their exact old absence/completion and issue identity.
        if let Some(intent) = &mut preset.intent {
            if matches!(intent.usage.completion, DraftListCompletion::Complete) {
                intent.usage.completion = b
                    .closure::<PresetUsageBindingDraft>(
                        plan.source,
                        "usage-preferences-not-converted",
                        vec![],
                    )?
                    .completion;
            }
        } else if let Some(usage) = &mut preset.usage_preferences {
            if matches!(usage.completion, DraftListCompletion::Complete) {
                usage.completion = b
                    .closure::<UsagePolicyDraft>(
                        plan.source,
                        "usage-preferences-not-converted",
                        vec![],
                    )?
                    .completion;
            }
        } else {
            preset.usage_preferences =
                Some(b.closure(plan.source, "usage-preferences-not-converted", vec![])?);
        }
        if preset.intent.is_none()
            && plan.applicability == PresetApplicability::WhenExactSourceSelected
        {
            let legacy = preset.usage_preferences.take().expect("initialized usage");
            preset.intent = Some(SkillPresetIntentDraftV1 {
                schema_version: 1,
                usage: DraftList {
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
                generated_inputs: b.closure(
                    plan.source,
                    "generated-skill-inputs-not-converted",
                    vec![],
                )?,
            });
        }
        for policy in &bound.policies {
            let matches = |row: &UsagePolicyDraft| {
                row.policy.to_resolved().as_ref() == Some(policy.policy)
                    && row.target.to_resolved().as_ref() == Some(&plan.target)
            };
            // Charge before inspecting the existing records, without allocating
            // a temporary reference list for every projected policy.
            let duplicate = if let Some(intent) = &preset.intent {
                b.charge(intent.usage.members.len())?;
                intent
                    .usage
                    .members
                    .iter()
                    .any(|row| matches(&row.selection))
            } else {
                let existing = &preset.usage_preferences.as_ref().unwrap().members;
                b.charge(existing.len())?;
                existing.iter().any(matches)
            };
            if duplicate {
                return invalid("duplicate occurrence usage target policy");
            }
            let (parameters, _) = decode_parameters(b, source, group, &policy.parameters)?;
            let selection = UsagePolicyDraft {
                policy: policy.policy.clone().into(),
                target: plan.target.clone().into(),
                parameters,
            };
            if let Some(intent) = &mut preset.intent {
                intent.usage.members.push(PresetUsageBindingDraft {
                    selection,
                    applicability: plan.applicability,
                });
            } else {
                preset
                    .usage_preferences
                    .as_mut()
                    .unwrap()
                    .members
                    .push(selection);
            }
        }
        for origin in plan.origins.into_iter().chain([plan.source, plan.group]) {
            let target = OwnedOriginTarget::SkillPreset(preset.id);
            b.charge(b.origins[origin.ordinal() as usize].links.len())?;
            if !b.origins[origin.ordinal() as usize].links.contains(&target) {
                b.link(origin, target)?;
            }
        }
    }
    Ok(())
}
