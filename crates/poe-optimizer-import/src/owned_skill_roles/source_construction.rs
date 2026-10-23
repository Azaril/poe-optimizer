//! Bounded source-construction evidence checked against owned mappings/schema.
//! These records describe source construction, never native effect membership.
use super::*;
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum OwnedGemSourceConstruction {
    /// Directly authored roles without authenticated construction evidence.
    Unavailable,
    Observed(Box<ObservedGemSourceConstruction>),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum OwnedSourceEffectResolution {
    Mapped(SkillDefId),
    Missing,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedSourceEffectReference {
    pub effect_id: String,
    pub resolution: OwnedSourceEffectResolution,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedIndexedSourceEffectReference {
    pub index: u32,
    pub effect: OwnedSourceEffectReference,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedSourceStatSetReference {
    pub index: u32,
    pub id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnedMissingSourceEffectKind {
    PrimaryEffect,
    DeclaredAdditionalEffect,
    ConstructedAdditionalEffect,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedMissingSourceEffect {
    pub kind: OwnedMissingSourceEffectKind,
    #[serde(deserialize_with = "required_option")]
    pub index: Option<u32>,
    pub effect_id: String,
}

fn required_option<'de, D, T>(deserializer: D) -> std::result::Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedGemSourceConstruction {
    pub catalog_gem_key: String,
    pub winning_declaration: u32,
    pub game_id: String,
    pub variant_id: String,
    pub primary: OwnedSourceEffectReference,
    pub declared_additional_effects: Vec<OwnedIndexedSourceEffectReference>,
    pub constructed_additional_effects: Vec<OwnedIndexedSourceEffectReference>,
    /// Resolved setup list, before display ordering; repetition is meaningful.
    pub resolved_additional_effects: Vec<OwnedSourceEffectReference>,
    /// Actual post-construction display list; never sort or deduplicate.
    pub resolved_effects: Vec<OwnedSourceEffectReference>,
    #[serde(deserialize_with = "required_option")]
    pub display_order: Option<Vec<i64>>,
    /// Stat sets are deliberately not represented as Skill references.
    pub additional_stat_sets: Vec<OwnedSourceStatSetReference>,
    pub missing_references: Vec<OwnedMissingSourceEffect>,
}

pub(crate) fn source_text(value: &str, budget: &mut Budget) -> Result<()> {
    budget.text(value)?;
    if value.is_empty() || value.contains('\0') {
        return Err(SkillCatalogError::ConstructionMismatch);
    }
    Ok(())
}

pub(crate) fn expected_missing(
    facts: &ObservedGemSourceConstruction,
) -> Vec<OwnedMissingSourceEffect> {
    let mut missing = Vec::new();
    let mut add = |kind, index, effect: &OwnedSourceEffectReference| {
        if effect.resolution == OwnedSourceEffectResolution::Missing {
            missing.push(OwnedMissingSourceEffect {
                kind,
                index,
                effect_id: effect.effect_id.clone(),
            });
        }
    };
    add(
        OwnedMissingSourceEffectKind::PrimaryEffect,
        None,
        &facts.primary,
    );
    for (kind, refs) in [
        (
            OwnedMissingSourceEffectKind::DeclaredAdditionalEffect,
            &facts.declared_additional_effects,
        ),
        (
            OwnedMissingSourceEffectKind::ConstructedAdditionalEffect,
            &facts.constructed_additional_effects,
        ),
    ] {
        for reference in refs {
            add(kind, Some(reference.index), &reference.effect);
        }
    }
    missing
}

pub(crate) fn preflight(value: &OwnedGemSourceConstruction, budget: &mut Budget) -> Result<()> {
    let OwnedGemSourceConstruction::Observed(facts) = value else {
        return Ok(());
    };
    for text in [&facts.catalog_gem_key, &facts.game_id, &facts.variant_id] {
        source_text(text, budget)?;
    }
    source_text(&facts.primary.effect_id, budget)?;
    for refs in [
        &facts.declared_additional_effects,
        &facts.constructed_additional_effects,
    ] {
        budget.collection("construction indexed references", refs.len())?;
        for reference in refs {
            source_text(&reference.effect.effect_id, budget)?;
        }
    }
    for refs in [&facts.resolved_additional_effects, &facts.resolved_effects] {
        budget.collection("construction resolved references", refs.len())?;
        for reference in refs {
            source_text(&reference.effect_id, budget)?;
        }
    }
    budget.collection("construction stat sets", facts.additional_stat_sets.len())?;
    for reference in &facts.additional_stat_sets {
        source_text(&reference.id, budget)?;
    }
    budget.collection(
        "construction missing references",
        facts.missing_references.len(),
    )?;
    for reference in &facts.missing_references {
        source_text(&reference.effect_id, budget)?;
    }
    // Validation constructs the expected census; bound those exact copies too.
    let mut missing_count = 0;
    for reference in std::iter::once(&facts.primary).chain(
        facts
            .declared_additional_effects
            .iter()
            .chain(&facts.constructed_additional_effects)
            .map(|r| &r.effect),
    ) {
        if reference.resolution == OwnedSourceEffectResolution::Missing {
            missing_count += 1;
            source_text(&reference.effect_id, budget)?;
        }
    }
    budget.collection("checked missing reference census", missing_count)?;
    budget.collection(
        "construction display order",
        facts.display_order.as_ref().map_or(0, Vec::len),
    )?;
    Ok(())
}

pub(crate) fn effect_selector(effect_id: &str) -> ExternalSelector {
    ExternalSelector::Definition(ExternalOwnerSelector::Skill {
        effect_id: SourceComponent::Text(effect_id.into()),
    })
}

pub(super) fn check_effect<I: DefinitionSchemaIndex>(
    reference: &OwnedSourceEffectReference,
    mappings: &OwnedMappingIndex,
    definitions: &I,
) -> Result<()> {
    match (
        &reference.resolution,
        mappings.lookup(&effect_selector(&reference.effect_id)),
    ) {
        (
            OwnedSourceEffectResolution::Mapped(id),
            Some(MappingOutcome::Mapped {
                target: SchemaSubject::Definition(DefinitionAddress::Skill(mapped)),
                ..
            }),
        ) if id == mapped => {
            if id.namespace() != definitions.namespace() {
                return Err(SkillCatalogError::ForeignNamespace);
            }
            match definitions.definition(id) {
                SchemaLookup::Known(_) | SchemaLookup::Unmapped(_) => Ok(()),
                _ => Err(SkillCatalogError::UnknownTarget),
            }
        }
        (OwnedSourceEffectResolution::Missing, None | Some(MappingOutcome::Unmapped { .. })) => {
            Ok(())
        }
        _ => Err(SkillCatalogError::ConstructionMismatch),
    }
}

fn increasing(indices: impl IntoIterator<Item = u32>) -> bool {
    let mut previous = 0;
    for index in indices {
        if index <= previous {
            return false;
        }
        previous = index;
    }
    true
}

pub(super) fn validate<'a, I: DefinitionSchemaIndex>(
    row: &'a OwnedGemRoleRow,
    mappings: &OwnedMappingIndex,
    definitions: &I,
    keys: &mut BTreeSet<&'a str>,
) -> Result<()> {
    let OwnedGemSourceConstruction::Observed(facts) = &row.construction else {
        return Ok(());
    };
    if facts.winning_declaration == 0 || !keys.insert(&facts.catalog_gem_key) {
        return Err(SkillCatalogError::ConstructionMismatch);
    }
    let selector = ExternalSelector::Definition(ExternalOwnerSelector::Gem {
        game_id: SourceComponent::Text(facts.game_id.clone()),
        variant_id: SourceComponent::Text(facts.variant_id.clone()),
    });
    let target = subject(&row.gem);
    let matches_gem = match mappings.lookup(&selector) {
        Some(MappingOutcome::Mapped { target: mapped, .. }) => mapped == &target,
        Some(MappingOutcome::Ambiguous { candidates, .. }) => candidates.contains(&target),
        _ => false,
    };
    if !matches_gem
        || !match (&row.primary, &facts.primary.resolution) {
            (OwnedPrimarySkill::Known(a), OwnedSourceEffectResolution::Mapped(b)) => a == b,
            (OwnedPrimarySkill::Unmapped { .. }, OwnedSourceEffectResolution::Missing) => true,
            _ => false,
        }
    {
        return Err(SkillCatalogError::ConstructionMismatch);
    }
    check_effect(&facts.primary, mappings, definitions)?;
    for refs in [
        &facts.declared_additional_effects,
        &facts.constructed_additional_effects,
    ] {
        if !increasing(refs.iter().map(|r| r.index)) {
            return Err(SkillCatalogError::ConstructionMismatch);
        }
        for reference in refs {
            check_effect(&reference.effect, mappings, definitions)?;
        }
    }
    if !increasing(facts.additional_stat_sets.iter().map(|r| r.index)) {
        return Err(SkillCatalogError::ConstructionMismatch);
    }
    // Binary search preserves sparse indices without quadratic declaration joins.
    for declared in &facts.declared_additional_effects {
        let found = facts
            .constructed_additional_effects
            .binary_search_by_key(&declared.index, |r| r.index)
            .ok()
            .map(|i| &facts.constructed_additional_effects[i]);
        if found != Some(declared) {
            return Err(SkillCatalogError::ConstructionMismatch);
        }
    }
    for refs in [&facts.resolved_additional_effects, &facts.resolved_effects] {
        for reference in refs {
            if matches!(reference.resolution, OwnedSourceEffectResolution::Missing) {
                return Err(SkillCatalogError::ConstructionMismatch);
            }
            check_effect(reference, mappings, definitions)?;
        }
    }
    if facts.missing_references != expected_missing(facts) {
        return Err(SkillCatalogError::ConstructionMismatch);
    }
    Ok(())
}
