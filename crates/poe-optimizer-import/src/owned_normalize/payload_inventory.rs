//! Empty authored payload membership, independent of effect discovery and usage.
use super::*;
use source_shape::retire_membership;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PayloadInventoryPolicy {
    SavedGroupsWithoutAuthoredContainersV1 {
        mapping_source: OwnedContentDigest,
        roles: OwnedContentDigest,
        non_container_gems: Vec<GemDefId>,
        nonphysical_non_container_skill_ids: Vec<String>,
    },
}

pub(super) struct CompiledPayloadInventory<'p> {
    gems: BTreeSet<&'p GemDefId>,
    nonphysical: BTreeSet<&'p str>,
    pub(super) work: usize,
}

pub(super) fn compile<'p>(
    policy: &'p NormalizationPolicy,
    mappings: &OwnedMappingIndex,
    limits: NormalizationLimits,
) -> Result<Option<CompiledPayloadInventory<'p>>> {
    let Some(PayloadInventoryPolicy::SavedGroupsWithoutAuthoredContainersV1 {
        mapping_source,
        non_container_gems,
        nonphysical_non_container_skill_ids,
        ..
    }) = &policy.payload_inventory
    else {
        return Ok(None);
    };
    if mapping_source != mappings.source_identity() {
        return Err(NormalizationError::Binding);
    }
    let count = non_container_gems
        .len()
        .saturating_add(nonphysical_non_container_skill_ids.len());
    if count > limits.mapping.max_entries
        || count > limits.draft.input.max_collection_entries
        || count > limits.max_work
    {
        return Err(NormalizationError::Limit("payload inventory policy rows"));
    }
    // Typed catalog identities are not selectors in a scalar value recipe.
    // Keep the smaller selector budget for the explicit source-string lane.
    if nonphysical_non_container_skill_ids.len() > limits.value.max_selectors {
        return Err(NormalizationError::Limit(
            "payload inventory skill selectors",
        ));
    }
    let mut gems = BTreeSet::new();
    for gem in non_container_gems {
        if gem.namespace() != &policy.namespace {
            return Err(NormalizationError::Binding);
        }
        if !gems.insert(gem) {
            return Err(NormalizationError::Policy(
                "payload inventory duplicate gem",
            ));
        }
    }
    let mut nonphysical = BTreeSet::new();
    let mut bytes = 0usize;
    for id in nonphysical_non_container_skill_ids {
        bytes = bytes.saturating_add(id.len());
        if id.len() > 128
            || id.len() > limits.value.max_selector_bytes
            || bytes > limits.value.max_total_selector_bytes
            || count.saturating_add(bytes) > limits.max_work
        {
            return Err(NormalizationError::Limit("payload inventory skill bytes"));
        }
        if id.is_empty()
            || id.trim() != id
            || id.chars().any(char::is_control)
            || !nonphysical.insert(id.as_str())
        {
            return Err(NormalizationError::Policy(
                "payload inventory skill identities",
            ));
        }
    }
    let work = count.saturating_add(bytes);
    Ok(Some(CompiledPayloadInventory {
        gems,
        nonphysical,
        work,
    }))
}

pub(super) fn validate_roles(
    policy: &NormalizationPolicy,
    roles: &OwnedSkillRoleIndex,
    limits: NormalizationLimits,
) -> Result<()> {
    let Some(PayloadInventoryPolicy::SavedGroupsWithoutAuthoredContainersV1 {
        roles: binding,
        non_container_gems,
        nonphysical_non_container_skill_ids,
        ..
    }) = &policy.payload_inventory
    else {
        return Ok(());
    };
    if binding != roles.identity() {
        return Err(NormalizationError::Binding);
    }
    let count = non_container_gems
        .len()
        .saturating_add(nonphysical_non_container_skill_ids.len());
    if count > limits.mapping.max_entries
        || count > limits.draft.input.max_collection_entries
        || count > limits.max_work
    {
        return Err(NormalizationError::Limit("payload inventory policy rows"));
    }
    for gem in non_container_gems {
        let Some(row) = roles.role(gem) else {
            return Err(NormalizationError::Policy("payload inventory unknown gem"));
        };
        if !matches!(row.primary, OwnedPrimarySkill::Known(_))
            || !matches!(row.role, OwnedGemRole::Known(_))
            || matches!(
                row.materialization,
                OwnedGemMaterialization::Unmapped { .. }
            )
        {
            return Err(NormalizationError::Policy(
                "payload inventory unresolved gem role",
            ));
        }
    }
    Ok(())
}

/// Only checked transitions may call this after validating the original package.
pub(crate) fn rebind_roles(policy: &mut NormalizationPolicy, roles: &OwnedSkillRoleIndex) {
    if let Some(PayloadInventoryPolicy::SavedGroupsWithoutAuthoredContainersV1 {
        roles: binding,
        ..
    }) = &mut policy.payload_inventory
    {
        *binding = *roles.identity();
    }
}

#[derive(Default)]
pub(super) struct Census {
    rows: BTreeMap<SourceOccurrenceId, ReviewedRow>,
}
struct ReviewedRow {
    support: bool,
    nonphysical: bool,
}
impl Census {
    fn insert(&mut self, source: SourceOccurrenceId, row: ReviewedRow, limit: usize) -> Result<()> {
        if self.rows.len() >= limit {
            return Err(NormalizationError::Limit("payload inventory source rows"));
        }
        self.rows.insert(source, row);
        Ok(())
    }
    /// Reuse the already charged exact selector/catalog pass; never resolve names.
    pub(super) fn record(
        &mut self,
        b: &mut Builder<'_, '_>,
        row: &SourceEvidenceRow<'_>,
        selector: Option<&ExternalSelector>,
        catalog: Option<&OwnedGemRoleRow>,
        compiled: Option<&CompiledPayloadInventory<'_>>,
    ) -> Result<()> {
        let Some(compiled) = compiled else {
            return Ok(());
        };
        b.charge(1)?;
        if let Some(catalog) = catalog
            && matches!(selector, Some(ExternalSelector::Definition(ExternalOwnerSelector::Gem {
                game_id: SourceComponent::Text(game), variant_id: SourceComponent::Text(variant),
            })) if !game.is_empty() && !variant.is_empty())
            && compiled.gems.contains(&catalog.gem)
        {
            let OwnedGemRole::Known(role) = catalog.role else {
                return Ok(());
            };
            self.insert(
                row.occurrence().id(),
                ReviewedRow {
                    support: role == AuthoredGemRole::SupportAssignment,
                    nonphysical: false,
                },
                b.limits.draft.input.max_collection_entries,
            )?;
            return Ok(());
        }
        // A present invalid/empty/undecodable gemId cannot take the skillId branch.
        if !matches!(
            selector,
            Some(ExternalSelector::Definition(ExternalOwnerSelector::Gem {
                game_id: SourceComponent::Missing,
                ..
            }))
        ) {
            return Ok(());
        }
        let attributes = &b.attributes[row.occurrence().id().ordinal() as usize];
        let Some((_, skill)) = attributes.get("skillId").copied() else {
            return Ok(());
        };
        b.charge(skill.raw().len())?;
        if skill.raw().len() > b.limits.mapping.max_string_bytes {
            return Err(NormalizationError::Limit("payload source skill bytes"));
        }
        if let Ok(skill) = skill.decoded()
            && compiled.nonphysical.contains(skill)
        {
            self.insert(
                row.occurrence().id(),
                ReviewedRow {
                    support: false,
                    nonphysical: true,
                },
                b.limits.draft.input.max_collection_entries,
            )?;
        }
        Ok(())
    }
}

pub(super) fn complete(
    b: &mut Builder<'_, '_>,
    draft: &mut DraftSessionInput,
    presets: &BTreeMap<SourceOccurrenceId, usize>,
    sets: Option<&[SourceOccurrenceId]>,
    census: &Census,
    enabled: bool,
) -> Result<()> {
    if !enabled || !draft.payload_links.members.is_empty() {
        return Ok(());
    }
    let Some(sets) = sets else {
        return Ok(());
    };
    let evidence = b.evidence;
    for &source in sets {
        b.charge(1)?;
        let Some(index) = presets.get(&source).copied() else {
            continue;
        };
        if !draft.skill_presets.members[index]
            .payload_links
            .members
            .is_empty()
        {
            continue;
        }
        let mut proved = true;
        for group in evidence.rows()[source.ordinal() as usize].children() {
            b.charge(1)?;
            let mut non_supports = 0usize;
            for id in evidence.rows()[group.ordinal() as usize].children() {
                b.charge(1)?;
                let Some(row) = census.rows.get(id) else {
                    proved = false;
                    break;
                };
                if !row.support {
                    non_supports += 1;
                }
                if non_supports > 1 {
                    proved = false;
                    break;
                }
                if row.nonphysical {
                    b.charge(b.origins[id.ordinal() as usize].links.len())?;
                    if b.origins[id.ordinal() as usize].links.iter().any(|link| {
                        matches!(
                            link,
                            OwnedOriginTarget::Gem(_)
                                | OwnedOriginTarget::Skill(_)
                                | OwnedOriginTarget::Support(_)
                        )
                    }) {
                        proved = false;
                        break;
                    }
                }
            }
            if !proved {
                break;
            }
        }
        if proved {
            retire_membership(
                b,
                source,
                &mut draft.skill_presets.members[index].payload_links.completion,
                "payload-membership-not-converted",
            )?;
        }
    }
    Ok(())
}
