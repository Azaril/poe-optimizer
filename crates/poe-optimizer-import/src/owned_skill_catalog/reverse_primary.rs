//! Offline derivation of complete primary lookup observations from source rows.
use super::*;
use crate::owned_skill_roles::reverse_primary::{candidate_budget, text};
/// Both fresh/extension compilation and independent publication reproduction use
/// every actual injected Skill and its full catalog primary-candidate iterator.
/// Roles supply owned row identities, never the set of source candidates.
pub(super) fn derive(
    catalog: &SkillIdentityCatalog,
    skills: &source_construction::SourceSkills<'_>,
    roles: &[OwnedGemRoleRow],
    budget: &mut Budget,
) -> Result<OwnedReversePrimaryCensus> {
    budget.collection("reverse primary role joins", roles.len())?;
    budget.collection("reverse primary routes", catalog.data().skills.len())?;
    for skill in &catalog.data().skills {
        text(&skill.id, budget)?;
        candidate_budget(catalog.gems_for_effect(&skill.id).count(), budget)?;
        for gem in catalog.gems_for_effect(&skill.id) {
            text(&gem.key, budget)?;
        }
    }
    let mut by_key = BTreeMap::new();
    for row in roles {
        let OwnedGemSourceConstruction::Observed(facts) = &row.construction else {
            return Err(SkillCatalogError::ReversePrimaryMismatch);
        };
        if by_key.insert(facts.catalog_gem_key.as_str(), row).is_some() {
            return Err(SkillCatalogError::ReversePrimaryMismatch);
        }
    }
    let mut routes = Vec::with_capacity(catalog.data().skills.len());
    for source in &catalog.data().skills {
        let Some((skill, _, _)) = skills.get(source.id.as_str()) else {
            return Err(SkillCatalogError::ReversePrimaryMismatch);
        };
        let mut candidates = Vec::new();
        for gem in catalog.gems_for_effect(&source.id) {
            let Some(row) = by_key.get(gem.key.as_str()) else {
                return Err(SkillCatalogError::ReversePrimaryMismatch);
            };
            candidates.push(OwnedReversePrimaryCandidate {
                gem: row.gem.clone(),
                catalog_gem_key: gem.key.clone(),
            });
        }
        candidates.sort_by(|a, b| a.catalog_gem_key.cmp(&b.catalog_gem_key));
        routes.push(OwnedReversePrimaryRoute {
            skill: OwnedSourceEffectReference {
                effect_id: source.id.clone(),
                resolution: OwnedSourceEffectResolution::Mapped(skill.clone()),
            },
            candidates,
        });
    }
    routes.sort_by(|a, b| a.skill.effect_id.cmp(&b.skill.effect_id));
    Ok(OwnedReversePrimaryCensus::Observed { routes })
}
