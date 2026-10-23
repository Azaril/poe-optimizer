//! Offline derivation and authenticated reproduction of construction evidence.
use super::*;
use crate::owned_skill_roles::source_construction::{
    effect_selector, expected_missing, preflight, source_text,
};
use poe_optimizer_data::skill_identities::{GemIdentity, IndexedIdentityReference};
pub(super) type SourceSkills<'a> = BTreeMap<&'a str, (SkillDefId, Option<bool>, Option<bool>)>;

/// Charge every repeated copied source string and nested allocation before any
/// conversion. The catalog's own validation does not replace caller budgets.
fn preflight_gem(gem: &GemIdentity, budget: &mut Budget) -> Result<()> {
    for text in [
        &gem.key,
        &gem.game_id,
        &gem.variant_id,
        &gem.primary_effect_id,
    ] {
        source_text(text, budget)?;
    }
    for refs in [
        &gem.declared_additional_effects,
        &gem.constructed_additional_effects,
        &gem.declared_additional_stat_sets,
    ] {
        budget.collection("construction indexed references", refs.len())?;
        for reference in refs {
            source_text(&reference.id, budget)?;
        }
    }
    for refs in [&gem.additional_effects, &gem.effect_list] {
        budget.collection("construction resolved references", refs.len())?;
        for reference in refs {
            source_text(reference, budget)?;
        }
    }
    budget.collection(
        "construction display order",
        gem.display_order.as_ref().map_or(0, Vec::len),
    )?;
    Ok(())
}

pub(super) fn derive(
    gem: &GemIdentity,
    skills: &SourceSkills<'_>,
    budget: &mut Budget,
) -> Result<OwnedGemSourceConstruction> {
    preflight_gem(gem, budget)?;
    // The missing census repeats source strings, so account for that output too.
    let mut missing_count = 0;
    for id in std::iter::once(&gem.primary_effect_id).chain(
        gem.declared_additional_effects
            .iter()
            .chain(&gem.constructed_additional_effects)
            .map(|r| &r.id),
    ) {
        if !skills.contains_key(id.as_str()) {
            missing_count += 1;
            source_text(id, budget)?;
        }
    }
    budget.collection("construction missing references", missing_count)?;
    let effect = |id: &String| OwnedSourceEffectReference {
        effect_id: id.clone(),
        resolution: skills
            .get(id.as_str())
            .map_or(OwnedSourceEffectResolution::Missing, |(id, _, _)| {
                OwnedSourceEffectResolution::Mapped(id.clone())
            }),
    };
    let indexed = |r: &IndexedIdentityReference| OwnedIndexedSourceEffectReference {
        index: r.index,
        effect: effect(&r.id),
    };
    let mut facts = ObservedGemSourceConstruction {
        catalog_gem_key: gem.key.clone(),
        winning_declaration: gem.winning_declaration,
        game_id: gem.game_id.clone(),
        variant_id: gem.variant_id.clone(),
        primary: effect(&gem.primary_effect_id),
        declared_additional_effects: gem
            .declared_additional_effects
            .iter()
            .map(indexed)
            .collect(),
        constructed_additional_effects: gem
            .constructed_additional_effects
            .iter()
            .map(indexed)
            .collect(),
        resolved_additional_effects: gem.additional_effects.iter().map(effect).collect(),
        resolved_effects: gem.effect_list.iter().map(effect).collect(),
        display_order: gem.display_order.clone(),
        additional_stat_sets: gem
            .declared_additional_stat_sets
            .iter()
            .map(|r| OwnedSourceStatSetReference {
                index: r.index,
                id: r.id.clone(),
            })
            .collect(),
        missing_references: Vec::new(),
    };
    facts.missing_references = expected_missing(&facts);
    Ok(OwnedGemSourceConstruction::Observed(Box::new(facts)))
}

/// Offline publication proof. Runtime package binding alone cannot certify an
/// observation supplied by a caller, even when that caller recomputes its digest.
/// This rederives every complete role row from the authenticated catalog under
/// its recorded policy and checks exact equality, including order and omissions.
/// Ambiguous source selectors remain valid runtime diagnostics, but cannot prove
/// which owned Gem received a catalog row without an independent pairing witness.
pub fn verify_owned_skill_catalog_reproduction(
    catalog: &SkillIdentityCatalog,
    roles: &OwnedSkillRoleIndex,
    mappings: &OwnedMappingIndex,
    limits: SkillCatalogLimits,
) -> Result<()> {
    let mut budget = Budget::new(limits)?;
    mappings.validate_limits(limits.mapping)?;
    let data = catalog.data();
    let input = roles.input();
    digest_owned(
        "owned-skill-role-reproduction-input-v1",
        input,
        limits.mapping.max_wire_bytes,
    )?;
    budget.pin(&input.compilation.source)?;
    // The validated catalog may have been built under looser bounds. Retained
    // declarations and unresolved references still consume verification work
    // even when they collapse to a small number of final construction rows.
    for (name, count) in [
        ("gem declarations", data.gem_declarations.len()),
        ("skill declarations", data.skill_declarations.len()),
        ("missing references", data.missing_references.len()),
        ("source module order", data.source.skill_module_order.len()),
    ] {
        budget.collection(name, count)?;
    }
    budget.collection("reproduced source skills", data.skills.len())?;
    budget.collection("reproduced source gems", data.gems.len())?;
    budget.collection("reproduced role rows", input.roles.len())?;
    if input.mapping != *mappings.identity()
        || input.namespace != mappings.input().namespace
        || input.definitions != mappings.input().definitions
        || !provenance_is_subset(&input.compilation.source, &mappings.input().source)
        || input.compilation.policy.version != mappings.input().policy_version
        || input.compilation.source.system != ExternalSourceSystem::PathOfBuilding2
        || input.compilation.source.revision != data.source.upstream_revision
        || input.compilation.gem_count != data.gems.len()
        || input.compilation.skill_count != data.skills.len()
        || input.roles.len() != data.gems.len()
        || input.compilation.catalog_digest
            != digest_owned(
                "owned-skill-source-catalog-v1",
                data,
                limits.mapping.max_wire_bytes,
            )?
    {
        return Err(SkillCatalogError::ReproductionMismatch);
    }
    for (path, hash) in &data.source.files {
        if input
            .compilation
            .source
            .files
            .binary_search_by(|p| p.path.cmp(path))
            .ok()
            .is_none_or(|i| &input.compilation.source.files[i].sha256 != hash)
        {
            return Err(SkillCatalogError::SourceMismatch);
        }
    }
    let mut skills = BTreeMap::new();
    for skill in &data.skills {
        source_text(&skill.id, &mut budget)?;
        let Some(MappingOutcome::Mapped {
            target: SchemaSubject::Definition(DefinitionAddress::Skill(id)),
            ..
        }) = mappings.lookup(&effect_selector(&skill.id))
        else {
            return Err(SkillCatalogError::ReproductionMismatch);
        };
        skills.insert(
            skill.id.as_str(),
            (id.clone(), skill.support, skill.from_tree),
        );
    }
    // Role keys are compiler row identities, not names or primary selectors.
    let mut by_key = BTreeMap::new();
    for row in &input.roles {
        preflight(&row.construction, &mut budget)?;
        let OwnedGemSourceConstruction::Observed(facts) = &row.construction else {
            return Err(SkillCatalogError::ReproductionMismatch);
        };
        let selector = ExternalSelector::Definition(ExternalOwnerSelector::Gem {
            game_id: SourceComponent::Text(facts.game_id.clone()),
            variant_id: SourceComponent::Text(facts.variant_id.clone()),
        });
        if !matches!(
            mappings.lookup(&selector),
            Some(MappingOutcome::Mapped {
                target: SchemaSubject::Definition(DefinitionAddress::Gem(gem)),
                ..
            }) if gem == &row.gem
        ) {
            // Candidate membership does not authenticate the association between
            // this exact catalog key and one particular allocated owned Gem.
            return Err(SkillCatalogError::ReproductionMismatch);
        }
        if by_key.insert(facts.catalog_gem_key.as_str(), row).is_some() {
            return Err(SkillCatalogError::ReproductionMismatch);
        }
    }
    for gem in &data.gems {
        let Some(row) = by_key.remove(gem.key.as_str()) else {
            return Err(SkillCatalogError::ReproductionMismatch);
        };
        let (primary, role, materialization) = classify_primary(
            skills.get(gem.primary_effect_id.as_str()),
            &input.compilation.policy,
        );
        if row.primary != primary
            || row.role != role
            || row.materialization != materialization
            || row.construction != derive(gem, &skills, &mut budget)?
        {
            return Err(SkillCatalogError::ReproductionMismatch);
        }
    }
    if !by_key.is_empty() {
        return Err(SkillCatalogError::ReproductionMismatch);
    }
    if input.reverse_primary
        != reverse_primary::derive(catalog, &skills, &input.roles, &mut budget)?
    {
        return Err(SkillCatalogError::ReproductionMismatch);
    }
    Ok(())
}
