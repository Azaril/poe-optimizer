//! Saved physical assignment inventory, independent of effect discovery and targets.
//! A complete row census never asserts that granted effects or provider origins
//! are complete, nor that any assignment can already be evaluated.
use super::*;
use source_shape::{charge_frame, container_text, plain_row, retire_membership, value};

/// Results of the existing exact selector/catalog pass, scoped to this fresh
/// normalization and its already checked role identity. Original source framing
/// must still be proved before these rows can establish a complete inventory.
pub(super) struct Census {
    rows: Option<BTreeMap<SourceOccurrenceId, ReviewedRow>>,
}
struct ReviewedRow {
    role: AuthoredGemRole,
    physical: bool,
    definition: GemDefId,
}
impl Census {
    pub(super) fn new(policy: &NormalizationPolicy) -> Self {
        Self {
            rows: matches!(
                policy.support_origin_order,
                Some(SupportOriginOrderPolicy::SavedManualGroupOrderWithPhysicalInventoryV2 { .. })
            )
            .then(BTreeMap::new),
        }
    }
    pub(super) fn enabled(&self) -> bool {
        self.rows.is_some()
    }

    /// The caller charges one bounded cache entry before its existing role pass.
    /// Unknown/ambiguous catalog outcomes deliberately leave no positive proof.
    pub(super) fn record(
        &mut self,
        source: SourceOccurrenceId,
        selector: Option<&ExternalSelector>,
        catalog: Option<&OwnedGemRoleRow>,
    ) {
        let (Some(rows), Some(catalog)) = (&mut self.rows, catalog) else {
            return;
        };
        if !matches!(selector, Some(ExternalSelector::Definition(ExternalOwnerSelector::Gem {
            game_id: SourceComponent::Text(game), variant_id: SourceComponent::Text(variant),
        })) if !game.is_empty() && !variant.is_empty())
        {
            return;
        }
        let OwnedGemRole::Known(role) = catalog.role else {
            return;
        };
        if !matches!(catalog.primary, OwnedPrimarySkill::Known(_))
            || matches!(
                catalog.materialization,
                OwnedGemMaterialization::Unmapped { .. }
            )
        {
            return;
        }
        rows.insert(
            source,
            ReviewedRow {
                role,
                physical: matches!(catalog.materialization, OwnedGemMaterialization::Physical),
                definition: catalog.gem.clone(),
            },
        );
    }
}

pub(super) fn validate_source(
    policy: &NormalizationPolicy,
    mappings: &OwnedMappingIndex,
) -> Result<()> {
    if let Some(SupportOriginOrderPolicy::SavedManualGroupOrderWithPhysicalInventoryV2 {
        mapping_source,
        ..
    }) = &policy.support_origin_order
        && mapping_source != mappings.source_identity()
    {
        return Err(NormalizationError::Binding);
    }
    Ok(())
}

pub(super) fn validate_roles(
    policy: &NormalizationPolicy,
    roles: &OwnedSkillRoleIndex,
) -> Result<()> {
    if let Some(SupportOriginOrderPolicy::SavedManualGroupOrderWithPhysicalInventoryV2 {
        roles: binding,
        ..
    }) = &policy.support_origin_order
        && binding != roles.identity()
    {
        return Err(NormalizationError::Binding);
    }
    Ok(())
}

/// Only checked offline transitions may call this after validating the prior.
pub(crate) fn rebind_roles(policy: &mut NormalizationPolicy, roles: &OwnedSkillRoleIndex) {
    if let Some(SupportOriginOrderPolicy::SavedManualGroupOrderWithPhysicalInventoryV2 {
        roles: binding,
        ..
    }) = &mut policy.support_origin_order
    {
        *binding = *roles.identity();
    }
}

fn numeric_key(text: &str) -> Option<u64> {
    if text.is_empty() || text.starts_with('0') || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse::<u64>().ok().filter(|v| *v < (1_u64 << 53))
}

const SKILLS_ATTRIBUTES: &[&str] = &[
    "activeSkillSet",
    "defaultGemLevel",
    "defaultGemQuality",
    "sortGemsByDPS",
    "showLegacyGems",
    "showSupportGemTypes",
    "sortGemsByDPSField",
    "matchGemLevelToCharacterLevel",
];
const GROUP_ATTRIBUTES: &[&str] = &[
    "active",
    "enabled",
    "includeInFullDPS",
    "groupCount",
    "label",
    "slot",
    "source",
    "mainActiveSkill",
    "mainActiveSkillCalcs",
    "skillPart",
];
const GEM_ATTRIBUTES: &[&str] = &[
    "nameSpec",
    "gemId",
    "variantId",
    "skillId",
    "level",
    "quality",
    "note",
    "enabled",
    "enableGlobal1",
    "enableGlobal2",
    "count",
    "statSetIndex",
    "statSetIndexCalcs",
    "skillPart",
    "skillPartCalcs",
    "skillStageCount",
    "skillStageCountCalcs",
    "skillMineCount",
    "skillMineCountCalcs",
    "skillMinion",
    "skillMinionCalcs",
    "skillMinionItemSet",
    "skillMinionItemSetCalcs",
    "skillMinionSkill",
    "skillMinionSkillCalcs",
    "corrupted",
    "corruptLevel",
];

/// Check every container and row frame first. The source loader iterates all sets
/// and treats every group child as a Gem, so an unknown sibling is not ignorable.
fn sets(b: &mut Builder<'_, '_>) -> Result<Option<Vec<SourceOccurrenceId>>> {
    let evidence = b.evidence;
    let root = &evidence.rows()[0];
    charge_frame(b, root, &[])?;
    if root.occurrence().name() != "PathOfBuilding2"
        || !plain_row(root, &[], false)
        || !container_text(root)
    {
        return Ok(None);
    }
    let mut skills = None;
    for id in root.children() {
        let row = &evidence.rows()[id.ordinal() as usize];
        if row.occurrence().name() == "Skills" {
            if skills.replace(row).is_some() {
                return Ok(None);
            }
        } else if matches!(row.occurrence().name(), "SkillSet" | "Skill" | "Gem") {
            return Ok(None);
        }
    }
    let Some(skills) = skills else {
        return Ok(None);
    };
    charge_frame(b, skills, &["activeSkillSet"])?;
    if !plain_row(skills, SKILLS_ATTRIBUTES, false) || !container_text(skills) {
        return Ok(None);
    }
    let Some(selected) = value(skills, "activeSkillSet").and_then(numeric_key) else {
        return Ok(None);
    };
    if skills.children().len() > b.limits.draft.input.max_collection_entries {
        return Err(NormalizationError::Limit("support inventory sets"));
    }
    let mut keys = BTreeSet::new();
    let mut sets = Vec::new();
    for id in skills.children() {
        let set = &evidence.rows()[id.ordinal() as usize];
        charge_frame(b, set, &["id"])?;
        if set.occurrence().name() != "SkillSet"
            || set.occurrence().parent() != Some(skills.occurrence().id())
            || !plain_row(set, &["id", "title"], false)
            || !container_text(set)
            || !matches!(
                set.authored_instance(),
                Some(AuthoredInstanceId::SkillSet(_))
            )
        {
            return Ok(None);
        }
        let Some(key) = value(set, "id").and_then(numeric_key) else {
            return Ok(None);
        };
        if !keys.insert(key) {
            return Ok(None);
        }
        for id in set.children() {
            let group = &evidence.rows()[id.ordinal() as usize];
            charge_frame(b, group, &[])?;
            if group.occurrence().name() != "Skill"
                || group.occurrence().parent() != Some(set.occurrence().id())
                || !plain_row(group, GROUP_ATTRIBUTES, false)
                || !container_text(group)
                || !matches!(
                    group.authored_instance(),
                    Some(AuthoredInstanceId::SkillGroup(_))
                )
            {
                return Ok(None);
            }
            for id in group.children() {
                let gem = &evidence.rows()[id.ordinal() as usize];
                charge_frame(b, gem, &[])?;
                if gem.occurrence().name() != "Gem"
                    || gem.occurrence().parent() != Some(group.occurrence().id())
                    || !plain_row(gem, GEM_ATTRIBUTES, true)
                    || !matches!(
                        gem.authored_instance(),
                        Some(AuthoredInstanceId::SkillEntry(_))
                    )
                {
                    return Ok(None);
                }
            }
        }
        sets.push(set.occurrence().id());
    }
    Ok(keys.contains(&selected).then_some(sets))
}

/// Retire only the physical list obligation after comparing the entire ordered
/// materialization against source origins. Spent IDs and all unresolved targets,
/// origin discovery, skills, payloads and definition obligations are preserved.
pub(super) fn complete(
    b: &mut Builder<'_, '_>,
    draft: &mut DraftSessionInput,
    presets: &BTreeMap<SourceOccurrenceId, usize>,
    census: &Census,
    group_sources: &BTreeMap<SourceOccurrenceId, Option<SourceComponent>>,
    policy: &NormalizationPolicy,
) -> Result<()> {
    let Some(rows) = &census.rows else {
        return Ok(());
    };
    let Some(sets) = sets(b)? else {
        return Ok(());
    };
    b.charge(
        draft
            .gems
            .members
            .len()
            .saturating_add(draft.supports.members.len()),
    )?;
    let gems = draft
        .gems
        .members
        .iter()
        .map(|g| (g.id, g))
        .collect::<BTreeMap<_, _>>();
    let supports = draft
        .supports
        .members
        .iter()
        .map(|s| (s.id, s))
        .collect::<BTreeMap<_, _>>();
    let evidence = b.evidence;
    for source in sets {
        b.charge(1)?;
        let Some(index) = presets.get(&source).copied() else {
            continue;
        };
        let set = &evidence.rows()[source.ordinal() as usize];
        let mut expected = Vec::new();
        let mut proved = true;
        for id in set.children() {
            let group = &evidence.rows()[id.ordinal() as usize];
            b.charge(
                policy
                    .manual_skill_sources
                    .len()
                    .saturating_add(policy.generated_support_prefixes.len())
                    .saturating_add(1),
            )?;
            let origin = group_sources.get(id).and_then(Option::as_ref);
            let manual = origin
                .as_ref()
                .is_some_and(|v| policy.manual_skill_sources.contains(v));
            let generated = origin.as_ref().is_some_and(|v| match v {
                SourceComponent::Text(v) => policy
                    .generated_support_prefixes
                    .iter()
                    .any(|p| v.starts_with(p)),
                _ => false,
            });
            for id in group.children() {
                b.charge(1)?;
                let Some(catalog) = rows.get(id) else {
                    proved = false;
                    break;
                };
                let physical = catalog.physical;
                let role = catalog.role;
                b.charge(
                    b.origins[id.ordinal() as usize]
                        .links
                        .len()
                        .saturating_mul(2),
                )?;
                let links = &b.origins[id.ordinal() as usize].links;
                let mut gem_links = links.iter().filter_map(|v| match v {
                    OwnedOriginTarget::Gem(id) => Some(*id),
                    _ => None,
                });
                let gem = gem_links.next();
                let extra_gem = gem_links.next().is_some();
                let mut support_links = links.iter().filter_map(|v| match v {
                    OwnedOriginTarget::Support(id) => Some(*id),
                    _ => None,
                });
                let support = support_links.next();
                let extra_support = support_links.next().is_some();
                if role != AuthoredGemRole::SupportAssignment || !physical {
                    if support.is_some() || extra_support || (!physical && gem.is_some()) {
                        proved = false;
                        break;
                    }
                    continue;
                }
                // Origin admission is needed to materialize an assignment.
                // Reviewed active/provider rows add no physical assignments;
                // their unresolved provider/effect semantics remain separate.
                if !manual && !generated {
                    proved = false;
                    break;
                }
                let (Some(gem), Some(support)) = (gem, support) else {
                    proved = false;
                    break;
                };
                if extra_gem
                    || extra_support
                    || gems
                        .get(&gem)
                        .is_none_or(|g| !matches!(&g.definition, DraftField::Known { value } if value == &catalog.definition))
                    || supports
                        .get(&support)
                        .is_none_or(|s| s.support != DraftField::Known { value: gem })
                {
                    proved = false;
                    break;
                }
                if expected.len() >= b.limits.draft.input.max_collection_entries {
                    return Err(NormalizationError::Limit("support inventory assignments"));
                }
                expected.push(support);
            }
            if !proved {
                break;
            }
        }
        let preset = &mut draft.skill_presets.members[index];
        b.charge(expected.len().saturating_add(preset.supports.members.len()))?;
        if proved && expected == preset.supports.members {
            retire_membership(
                b,
                source,
                &mut preset.supports.completion,
                "support-membership-not-converted",
            )?;
        }
    }
    Ok(())
}
