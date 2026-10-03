//! Finite authored-root membership, separate from generated activation and usage.
use super::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SkillInventoryPolicy {
    PobFreshAuthoredRootsV1 {
        mapping_source: OwnedContentDigest,
        roles: OwnedContentDigest,
        direct_inputs: Option<OwnedContentDigest>,
        generated_groups: Vec<GeneratedSkillGroupInventory>,
    },
}

/// Source-owned generated representations, not an assertion of a live provider.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeneratedSkillGroupSource {
    TreeAllocation,
    ItemGrant,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratedSkillGroupInventory {
    pub source: GeneratedSkillGroupSource,
    pub gem: GemDefId,
    pub game_id: String,
    pub variant_id: String,
    pub skill_id: String,
    pub name_spec: String,
}

pub(super) fn present<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<SkillInventoryPolicy>, D::Error> {
    SkillInventoryPolicy::deserialize(deserializer).map(Some)
}

/// The complete existing Direct policy is the admission authority, not just its
/// list of identities. Checked transitions refresh this after rebinding Direct.
pub fn direct_skill_inputs_identity(
    input: &DirectSkillInputPolicy,
    limits: NormalizationLimits,
) -> Result<OwnedContentDigest> {
    Ok(digest_owned(
        "owned-direct-skill-inputs-v1",
        input,
        limits.max_policy_bytes.min(MAX_NORMALIZATION_POLICY_BYTES),
    )?)
}

pub(crate) fn rebind(
    policy: &mut NormalizationPolicy,
    roles: &OwnedSkillRoleIndex,
    limits: NormalizationLimits,
) -> Result<()> {
    if policy.skill_inventory.is_none() {
        return Ok(());
    }
    let direct = policy
        .direct_skill_inputs
        .as_ref()
        .map(|input| direct_skill_inputs_identity(input, limits))
        .transpose()?;
    let Some(SkillInventoryPolicy::PobFreshAuthoredRootsV1 {
        roles: binding,
        direct_inputs,
        ..
    }) = &mut policy.skill_inventory
    else {
        unreachable!("checked optional authored-root inventory")
    };
    *binding = *roles.identity();
    *direct_inputs = direct;
    Ok(())
}

pub(super) struct CompiledSkillInventory<'p> {
    generated:
        BTreeMap<(GeneratedSkillGroupSource, &'p GemDefId), &'p GeneratedSkillGroupInventory>,
    direct: bool,
    pub(super) work: usize,
}

pub(super) fn compile<'p>(
    policy: &'p NormalizationPolicy,
    mappings: &OwnedMappingIndex,
    roles: &OwnedSkillRoleIndex,
    limits: NormalizationLimits,
) -> Result<Option<CompiledSkillInventory<'p>>> {
    let Some(SkillInventoryPolicy::PobFreshAuthoredRootsV1 {
        mapping_source,
        roles: binding,
        direct_inputs,
        generated_groups,
    }) = &policy.skill_inventory
    else {
        return Ok(None);
    };
    if mapping_source != mappings.source_identity()
        || binding != roles.identity()
        || roles.input().mapping != *mappings.identity()
    {
        return Err(NormalizationError::Binding);
    }
    let mut work = 1usize;
    let direct = policy
        .direct_skill_inputs
        .as_ref()
        .map(|input| direct_skill_inputs_identity(input, limits))
        .transpose()?;
    if direct != *direct_inputs {
        return Err(NormalizationError::Binding);
    }
    if let Some(input) = &policy.direct_skill_inputs {
        // Hashing above bounded the serialized value before this allocation.
        // Charge both serializations; no per-occurrence recompilation is needed.
        let bytes = serde_json::to_vec(input)
            .map_err(|_| NormalizationError::Policy("Direct inventory commitment"))?;
        work = work.saturating_add(bytes.len().saturating_mul(2));
    }
    if generated_groups.len() > 256
        || generated_groups.len() > limits.mapping.max_entries
        || generated_groups.len() > limits.draft.input.max_collection_entries
    {
        return Err(NormalizationError::Limit("generated root inventory rows"));
    }
    let mut generated = BTreeMap::new();
    let mut selectors = BTreeSet::new();
    let mut source_bytes = 0usize;
    for row in generated_groups {
        work = work.saturating_add(1);
        for value in [&row.game_id, &row.variant_id, &row.skill_id, &row.name_spec] {
            if value.is_empty()
                || value.trim() != value
                || value.chars().any(char::is_control)
                || value.len() > limits.mapping.max_string_bytes
            {
                return Err(NormalizationError::Policy("generated root source identity"));
            }
            work = work.saturating_add(value.len());
            source_bytes = source_bytes.saturating_add(value.len());
        }
        if work > limits.max_work || source_bytes > limits.mapping.max_total_string_bytes {
            return Err(NormalizationError::Limit("authored root policy work"));
        }
        if row.gem.namespace() != &policy.namespace
            || generated.insert((row.source, &row.gem), row).is_some()
            || !selectors.insert((row.source, &row.game_id, &row.variant_id))
        {
            return Err(NormalizationError::Policy(
                "generated root inventory identity",
            ));
        }
        let selector = ExternalSelector::Definition(ExternalOwnerSelector::Gem {
            game_id: SourceComponent::Text(row.game_id.clone()),
            variant_id: SourceComponent::Text(row.variant_id.clone()),
        });
        let Some(role) = roles.role(&row.gem) else {
            return Err(NormalizationError::Policy("generated root catalog role"));
        };
        let OwnedPrimarySkill::Known(primary) = &role.primary else {
            return Err(NormalizationError::Policy("generated root primary"));
        };
        let effect = ExternalSelector::Definition(ExternalOwnerSelector::Skill {
            effect_id: SourceComponent::Text(row.skill_id.clone()),
        });
        if !matches!(roles.lookup(&selector), Some(MappingOutcome::Mapped {
            target: SchemaSubject::Definition(DefinitionAddress::Gem(gem)),
            basis: MappingBasis::Exact,
        }) if gem == &row.gem)
            || !matches!(mappings.lookup(&effect), Some(MappingOutcome::Mapped {
                target: SchemaSubject::Definition(DefinitionAddress::Skill(skill)),
                basis: MappingBasis::Exact,
            }) if skill == primary)
            || role.role != OwnedGemRole::Known(AuthoredGemRole::SkillUse)
            || matches!(
                role.materialization,
                OwnedGemMaterialization::Unmapped { .. }
            )
        {
            return Err(NormalizationError::Policy(
                "generated root exact catalog mapping",
            ));
        }
    }
    if work > limits.max_work {
        return Err(NormalizationError::Limit("authored root policy work"));
    }
    Ok(Some(CompiledSkillInventory {
        generated,
        direct: direct.is_some(),
        work,
    }))
}

/// Reuses the existing selector/catalog pass and private successful disposition
/// results from this one fresh normalization. Neither can be supplied by callers.
pub(super) struct Census {
    rows: Option<BTreeMap<SourceOccurrenceId, OwnedGemRoleRow>>,
    nested: BTreeSet<SourceOccurrenceId>,
}
impl Census {
    pub(super) fn new(compiled: Option<&CompiledSkillInventory<'_>>) -> Self {
        Self {
            rows: compiled.map(|_| BTreeMap::new()),
            nested: BTreeSet::new(),
        }
    }
    pub(super) fn record(
        &mut self,
        b: &mut Builder<'_, '_>,
        source: SourceOccurrenceId,
        selector: Option<&ExternalSelector>,
        catalog: Option<&OwnedGemRoleRow>,
    ) -> Result<()> {
        let Some(rows) = &mut self.rows else {
            return Ok(());
        };
        b.charge(1)?;
        let (Some(selector), Some(catalog)) = (selector, catalog) else {
            return Ok(());
        };
        if !matches!(b.roles.lookup(selector), Some(MappingOutcome::Mapped {
            target: SchemaSubject::Definition(DefinitionAddress::Gem(gem)),
            basis: MappingBasis::Exact,
        }) if gem == &catalog.gem)
            || !matches!(catalog.primary, OwnedPrimarySkill::Known(_))
            || !matches!(catalog.role, OwnedGemRole::Known(_))
            || matches!(
                catalog.materialization,
                OwnedGemMaterialization::Unmapped { .. }
            )
        {
            return Ok(());
        }
        if rows.len() >= b.limits.draft.input.max_collection_entries {
            return Err(NormalizationError::Limit("authored root source rows"));
        }
        let OwnedPrimarySkill::Known(primary) = &catalog.primary else {
            unreachable!()
        };
        b.charge(identity_work(&catalog.gem).saturating_add(identity_work(primary)))?;
        rows.insert(source, catalog.clone());
        Ok(())
    }
    pub(super) fn proved_nested(
        &mut self,
        b: &mut Builder<'_, '_>,
        source: SourceOccurrenceId,
    ) -> Result<()> {
        if self.rows.is_some() {
            b.charge(1)?;
            self.nested.insert(source);
        }
        Ok(())
    }
}

fn identity_work<K: DefinitionDomain>(id: &DefId<K>) -> usize {
    id.namespace()
        .game()
        .as_str()
        .len()
        .saturating_add(id.namespace().version().as_str().len())
        .saturating_add(id.key().as_str().len())
        .saturating_add(1)
}

fn canonical_key(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('0')
        && value.bytes().all(|c| c.is_ascii_digit())
        && value.parse::<u64>().is_ok_and(|v| v < (1_u64 << 53))
}
fn generated_source(value: &str) -> Option<GeneratedSkillGroupSource> {
    if let Some(key) = value.strip_prefix("Tree:") {
        return canonical_key(key).then_some(GeneratedSkillGroupSource::TreeAllocation);
    }
    let (key, description) = value.strip_prefix("Item:")?.split_once(':')?;
    (canonical_key(key)
        && !description.is_empty()
        && description.trim() == description
        && !description.chars().any(char::is_control))
    .then_some(GeneratedSkillGroupSource::ItemGrant)
}

pub(super) fn complete(
    b: &mut Builder<'_, '_>,
    draft: &mut DraftSessionInput,
    presets: &BTreeMap<SourceOccurrenceId, usize>,
    census: &Census,
    compiled: Option<&CompiledSkillInventory<'_>>,
    policy: &NormalizationPolicy,
) -> Result<()> {
    let (Some(rows), Some(compiled)) = (&census.rows, compiled) else {
        return Ok(());
    };
    let Some(sets) = skill_source_census::container_sets(b)? else {
        return Ok(());
    };
    b.charge(
        draft
            .skills
            .members
            .len()
            .saturating_add(draft.gems.members.len())
            .saturating_add(draft.supports.members.len()),
    )?;
    let skills = draft
        .skills
        .members
        .iter()
        .map(|v| (v.id, v))
        .collect::<BTreeMap<_, _>>();
    let gems = draft
        .gems
        .members
        .iter()
        .map(|v| (v.id, v))
        .collect::<BTreeMap<_, _>>();
    let supports = draft
        .supports
        .members
        .iter()
        .map(|v| (v.id, v))
        .collect::<BTreeMap<_, _>>();
    let evidence = b.evidence;
    for set in sets {
        b.charge(1)?;
        let Some(index) = presets.get(&set).copied() else {
            continue;
        };
        let preset = &mut draft.skill_presets.members[index];
        b.charge(b.origins[set.ordinal() as usize].links.len())?;
        if b.origins[set.ordinal() as usize]
            .links
            .iter()
            .filter(|link| matches!(link, OwnedOriginTarget::SkillPreset(_)))
            .ne([&OwnedOriginTarget::SkillPreset(preset.id)])
        {
            continue;
        }
        let mut expected = Vec::new();
        let mut unique = BTreeSet::new();
        let mut proved = true;
        for group_id in evidence.rows()[set.ordinal() as usize].children() {
            let group = &evidence.rows()[group_id.ordinal() as usize];
            source_shape::charge_frame(b, group, &["source"])?;
            if group.occurrence().name() != "Skill"
                || group.occurrence().parent() != Some(set)
                || !source_shape::plain_row(group, skill_source_census::GROUP_ATTRIBUTES, false)
                || !source_shape::container_text(group)
                || !matches!(
                    group.authored_instance(),
                    Some(AuthoredInstanceId::SkillGroup(_))
                )
            {
                proved = false;
                break;
            }
            let origin = source_shape::value(group, "source");
            if origin.is_some_and(|value| value.len() > b.limits.mapping.max_string_bytes) {
                return Err(NormalizationError::Limit("generated root source bytes"));
            }
            // Lua treats an explicit empty string as truthy and source-owned.
            // Historical materialization stays unchanged; this proof cannot
            // certify that as an authored manual group.
            b.charge(policy.manual_skill_sources.len())?;
            let manual = origin.is_none()
                && policy
                    .manual_skill_sources
                    .contains(&SourceComponent::Missing);
            let generated = origin.and_then(generated_source);
            if !manual && (generated.is_none() || group.children().len() != 1) {
                proved = false;
                break;
            }
            for source in group.children() {
                let row = &evidence.rows()[source.ordinal() as usize];
                source_shape::charge_frame(b, row, &["gemId", "variantId", "skillId", "nameSpec"])?;
                let flat = row.children().is_empty();
                if row.occurrence().name() != "Gem"
                    || row.occurrence().parent() != Some(*group_id)
                    || !source_shape::plain_row(row, skill_source_census::GEM_ATTRIBUTES, flat)
                    || !source_shape::container_text(row)
                    || (!flat && (!manual || !census.nested.contains(source)))
                    || !matches!(
                        row.authored_instance(),
                        Some(AuthoredInstanceId::SkillEntry(_))
                    )
                {
                    proved = false;
                    break;
                }
                let Some(catalog) = rows.get(source) else {
                    proved = false;
                    break;
                };
                b.charge(b.origins[source.ordinal() as usize].links.len())?;
                let mut root = None;
                let mut gem = None;
                let mut support = None;
                for link in &b.origins[source.ordinal() as usize].links {
                    let duplicate = match link {
                        OwnedOriginTarget::Skill(id) => root.replace(*id).is_some(),
                        OwnedOriginTarget::Gem(id) => gem.replace(*id).is_some(),
                        OwnedOriginTarget::Support(id) => support.replace(*id).is_some(),
                        _ => false,
                    };
                    if duplicate {
                        proved = false;
                        break;
                    }
                }
                if !proved {
                    break;
                }
                if let Some(source_kind) = generated {
                    let Some(reviewed) = compiled.generated.get(&(source_kind, &catalog.gem))
                    else {
                        proved = false;
                        break;
                    };
                    if root.is_some()
                        || gem.is_some()
                        || support.is_some()
                        || [
                            ("gemId", reviewed.game_id.as_str()),
                            ("variantId", reviewed.variant_id.as_str()),
                            ("skillId", reviewed.skill_id.as_str()),
                            ("nameSpec", reviewed.name_spec.as_str()),
                        ]
                        .iter()
                        .any(|(name, expected)| source_shape::value(row, name) != Some(*expected))
                    {
                        proved = false;
                        break;
                    }
                    continue;
                }
                let physical = catalog.materialization == OwnedGemMaterialization::Physical;
                let known_gem = gem.and_then(|id| gems.get(&id)).is_some_and(|record| {
                    record.definition
                        == DraftField::Known {
                            value: catalog.gem.clone(),
                        }
                });
                if physical != known_gem || (!physical && gem.is_some()) {
                    proved = false;
                    break;
                }
                match catalog.role {
                    OwnedGemRole::Known(AuthoredGemRole::SupportAssignment) => {
                        if root.is_some()
                            || !physical
                            || support
                                .and_then(|id| supports.get(&id))
                                .is_none_or(|record| {
                                    Some(record.support.clone()) != gem.map(DraftField::from)
                                })
                        {
                            proved = false;
                            break;
                        }
                    }
                    OwnedGemRole::Known(AuthoredGemRole::SkillUse) => {
                        let Some(root) = root else {
                            proved = false;
                            break;
                        };
                        let Some(record) = skills.get(&root) else {
                            proved = false;
                            break;
                        };
                        let valid = match (&record.source, &catalog.primary) {
                            (DraftAuthoredSkillSource::Gem(DraftField::Known { value }), _) => {
                                physical && Some(*value) == gem
                            }
                            (
                                DraftAuthoredSkillSource::Direct(DraftField::Known { value }),
                                OwnedPrimarySkill::Known(primary),
                            ) => !physical && compiled.direct && value == primary,
                            _ => false,
                        };
                        if !valid || support.is_some() || !unique.insert(root) {
                            proved = false;
                            break;
                        }
                        if expected.len() >= b.limits.draft.input.max_collection_entries {
                            return Err(NormalizationError::Limit("authored root members"));
                        }
                        expected.push(root);
                    }
                    _ => {
                        proved = false;
                        break;
                    }
                }
            }
            if !proved {
                break;
            }
        }
        b.charge(expected.len().saturating_add(preset.skills.members.len()))?;
        if proved && expected == preset.skills.members {
            source_shape::retire_membership(
                b,
                set,
                &mut preset.skills.completion,
                "skill-membership-not-converted",
            )?;
        }
    }
    Ok(())
}
