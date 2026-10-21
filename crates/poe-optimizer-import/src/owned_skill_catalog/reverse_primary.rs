//! Import-only primary lookup observations, independently reproduced offline.
//! Candidate order is canonical serialization, never an upstream dispatch winner.
use super::*;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OwnedReversePrimaryCensus {
    Unavailable {},
    Observed {
        routes: Vec<OwnedReversePrimaryRoute>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedReversePrimaryRoute {
    pub skill: OwnedSourceEffectReference,
    /// Empty is observed absence; multiple candidates remain ambiguous.
    pub candidates: Vec<OwnedReversePrimaryCandidate>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedReversePrimaryCandidate {
    pub gem: GemDefId,
    /// Joins the existing construction row, including its declaration identity.
    pub catalog_gem_key: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OwnedReversePrimaryLookup<'a> {
    /// No observation, no exact source route, or a different owned Skill.
    Unavailable,
    Observed(&'a OwnedReversePrimaryRoute),
}

fn text(value: &str, budget: &mut Budget) -> Result<()> {
    budget.text(value)?;
    if value.is_empty() || value.contains('\0') {
        return Err(SkillCatalogError::ReversePrimaryMismatch);
    }
    Ok(())
}

fn candidate_budget(count: usize, budget: &mut Budget) -> Result<()> {
    if count > budget.limits.max_candidates {
        return Err(SkillCatalogError::LimitExceeded(
            "reverse primary candidates",
        ));
    }
    budget.collection("reverse primary candidates", count)
}

/// Charge nested collections and copied text before building indexes or outputs.
pub(super) fn preflight(value: &OwnedReversePrimaryCensus, budget: &mut Budget) -> Result<()> {
    let OwnedReversePrimaryCensus::Observed { routes } = value else {
        return Ok(());
    };
    budget.collection("reverse primary routes", routes.len())?;
    for route in routes {
        text(&route.skill.effect_id, budget)?;
        candidate_budget(route.candidates.len(), budget)?;
        for candidate in &route.candidates {
            text(&candidate.catalog_gem_key, budget)?;
        }
    }
    Ok(())
}

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

pub(super) fn validate<I: DefinitionSchemaIndex>(
    value: &mut OwnedReversePrimaryCensus,
    roles: &[OwnedGemRoleRow],
    positions: &BTreeMap<GemDefId, usize>,
    skill_count: usize,
    mappings: &OwnedMappingIndex,
    definitions: &I,
) -> Result<()> {
    let OwnedReversePrimaryCensus::Observed { routes } = value else {
        return Ok(());
    };
    if routes.len() != skill_count {
        return Err(SkillCatalogError::ReversePrimaryMismatch);
    }
    routes.sort_by(|a, b| a.skill.effect_id.cmp(&b.skill.effect_id));
    for (index, route) in routes.iter().enumerate() {
        if index > 0 && routes[index - 1].skill.effect_id == route.skill.effect_id {
            return Err(SkillCatalogError::ReversePrimaryMismatch);
        }
        if !matches!(
            route.skill.resolution,
            OwnedSourceEffectResolution::Mapped(_)
        ) {
            return Err(SkillCatalogError::ReversePrimaryMismatch);
        }
        source_construction::check_effect(&route.skill, mappings, definitions)?;
    }
    for route in routes.iter_mut() {
        route
            .candidates
            .sort_by(|a, b| a.catalog_gem_key.cmp(&b.catalog_gem_key));
        for (index, candidate) in route.candidates.iter().enumerate() {
            if candidate.gem.namespace() != definitions.namespace() {
                return Err(SkillCatalogError::ForeignNamespace);
            }
            if index > 0 && route.candidates[index - 1].catalog_gem_key == candidate.catalog_gem_key
            {
                return Err(SkillCatalogError::ReversePrimaryMismatch);
            }
            let Some(position) = positions.get(&candidate.gem) else {
                return Err(SkillCatalogError::ReversePrimaryMismatch);
            };
            let row = &roles[*position];
            let OwnedGemSourceConstruction::Observed(facts) = &row.construction else {
                return Err(SkillCatalogError::ReversePrimaryMismatch);
            };
            if facts.catalog_gem_key != candidate.catalog_gem_key || facts.primary != route.skill {
                return Err(SkillCatalogError::ReversePrimaryMismatch);
            }
        }
    }
    // Local completeness only: every observed role's mapped primary must occur
    // exactly once. A coherent role/census subset still requires full-catalog
    // reproduction; unrelated historical mappings do not enlarge this census.
    for row in roles {
        let OwnedGemSourceConstruction::Observed(facts) = &row.construction else {
            return Err(SkillCatalogError::ReversePrimaryMismatch);
        };
        if matches!(
            facts.primary.resolution,
            OwnedSourceEffectResolution::Missing
        ) {
            continue;
        }
        let Some(route) = find_route(routes, &facts.primary.effect_id) else {
            return Err(SkillCatalogError::ReversePrimaryMismatch);
        };
        let candidate = route
            .candidates
            .binary_search_by(|candidate| candidate.catalog_gem_key.cmp(&facts.catalog_gem_key))
            .ok()
            .map(|index| &route.candidates[index]);
        if route.skill != facts.primary
            || candidate.is_none_or(|candidate| candidate.gem != row.gem)
        {
            return Err(SkillCatalogError::ReversePrimaryMismatch);
        }
    }
    Ok(())
}

fn find_route<'a>(
    routes: &'a [OwnedReversePrimaryRoute],
    effect_id: &str,
) -> Option<&'a OwnedReversePrimaryRoute> {
    routes
        .binary_search_by(|route| route.skill.effect_id.as_str().cmp(effect_id))
        .ok()
        .map(|index| &routes[index])
}

pub(super) fn lookup<'a>(
    value: &'a OwnedReversePrimaryCensus,
    effect_id: &str,
    skill: &SkillDefId,
) -> OwnedReversePrimaryLookup<'a> {
    let OwnedReversePrimaryCensus::Observed { routes } = value else {
        return OwnedReversePrimaryLookup::Unavailable;
    };
    let Some(route) = find_route(routes, effect_id) else {
        return OwnedReversePrimaryLookup::Unavailable;
    };
    if !matches!(&route.skill.resolution, OwnedSourceEffectResolution::Mapped(id) if id == skill) {
        return OwnedReversePrimaryLookup::Unavailable;
    }
    OwnedReversePrimaryLookup::Observed(route)
}
