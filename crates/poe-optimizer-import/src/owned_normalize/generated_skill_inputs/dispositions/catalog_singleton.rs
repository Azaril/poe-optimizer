//! Exact generated catalog dispatch, used only after live receipt validation.
use super::*;

/// The actual generated constructor uses a Skill-keyed reverse lookup. A unique
/// observed candidate must join this exact source and its complete construction;
/// neither a direct-shaped result nor a surviving role row proves that lookup.
/// Offline publication independently reproduces the census from the full catalog.
pub(super) fn prove(
    b: &mut Builder<'_, '_>,
    source: SourceOccurrenceId,
    correspondence: &generated_skill_sources::GeneratedSkillSourceRule,
) -> Result<bool> {
    b.charge(1)?;
    let roles = b.roles;
    let skill = &correspondence.skill;
    let identity_work = skill
        .namespace()
        .game()
        .as_str()
        .len()
        .saturating_add(skill.namespace().version().as_str().len())
        .saturating_add(skill.key().as_str().len());
    let routes = match &roles.input().reverse_primary {
        OwnedReversePrimaryCensus::Unavailable {} => return Ok(false),
        OwnedReversePrimaryCensus::Observed { routes } => routes,
    };
    // Binary lookup compares at most the queried text length per step. Charge
    // its worst-case depth and the exact owned-Skill comparison before lookup.
    let depth = usize::BITS - routes.len().leading_zeros() + 1;
    b.charge(
        (depth as usize)
            .saturating_mul(correspondence.skill_id.len().saturating_add(1))
            .saturating_add(identity_work.saturating_mul(4))
            .saturating_add(1),
    )?;
    let OwnedReversePrimaryLookup::Observed(route) =
        roles.reverse_primary(&correspondence.skill_id, skill)
    else {
        return Ok(false);
    };
    let [candidate] = route.candidates.as_slice() else {
        return Ok(false);
    };
    let row = &b.evidence.rows()[source.ordinal() as usize];
    let Some(role) =
        skill_input_disposition::charged_source_role(b, roles, row, &correspondence.gem)?
    else {
        return Ok(false);
    };
    let OwnedGemSourceConstruction::Observed(facts) = &role.construction else {
        return Ok(false);
    };
    b.charge(
        candidate
            .catalog_gem_key
            .len()
            .saturating_add(correspondence.game_id.len())
            .saturating_add(correspondence.variant_id.len())
            .saturating_add(correspondence.skill_id.len().saturating_mul(3))
            .saturating_add(identity_work.saturating_mul(4))
            .saturating_add(16),
    )?;
    Ok(candidate.gem == correspondence.gem
        && candidate.catalog_gem_key == facts.catalog_gem_key
        && role.role == OwnedGemRole::Known(AuthoredGemRole::SkillUse)
        && !matches!(
            role.materialization,
            OwnedGemMaterialization::Unmapped { .. }
        )
        && matches!(&role.primary, OwnedPrimarySkill::Known(primary) if primary == skill)
        && facts.game_id == correspondence.game_id
        && facts.variant_id == correspondence.variant_id
        && facts.primary.effect_id == correspondence.skill_id
        && value(row, "gemId") == Some(facts.game_id.as_str())
        && value(row, "variantId") == Some(facts.variant_id.as_str())
        && value(row, "skillId") == Some(facts.primary.effect_id.as_str())
        && matches!(&facts.primary.resolution,
            OwnedSourceEffectResolution::Mapped(resolved) if resolved == skill)
        && facts.declared_additional_effects.is_empty()
        && facts.constructed_additional_effects.is_empty()
        && facts.resolved_additional_effects.is_empty()
        && facts.missing_references.is_empty()
        && facts.resolved_effects.as_slice() == std::slice::from_ref(&facts.primary)
        && facts
            .display_order
            .as_deref()
            .is_none_or(|order| order == [1]))
}
