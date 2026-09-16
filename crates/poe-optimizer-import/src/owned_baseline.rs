//! Shared append-only authoring of exclusive baseline channels.
//! No owner is created or closed, and no prior writer is silently replaced.
use crate::owned_recipe::*;
use poe_optimizer_core::{owned_definitions::*, owned_rules::*, owned_schema::*};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Eq, Ord, PartialEq, PartialOrd)]
enum SubjectKey {
    Definition(DefinitionAddress),
    Slot(SlotAddress),
}
impl From<&SchemaSubject> for SubjectKey {
    fn from(subject: &SchemaSubject) -> Self {
        match subject {
            SchemaSubject::Definition(address) => Self::Definition(address.clone()),
            SchemaSubject::Slot(address) => Self::Slot(address.clone()),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum BaselineAppendError {
    #[error("exclusive baseline work limit")]
    Limit,
    #[error("exclusive baseline owner, writer or coverage differs")]
    Preservation,
    #[error(transparent)]
    Recipe(#[from] OwnedRecipeError),
}

pub(crate) fn append_exclusive_baselines(
    base: &StagedOwnedRecipe,
    programs: &[(SchemaSubject, RuleProgram)],
    stats: &BTreeSet<StatDefId>,
    capabilities: &BTreeSet<CapabilityDefId>,
    work_left: &mut usize,
    limits: OwnedRecipeLimits,
) -> Result<(OwnedRecipeInput, usize), BaselineAppendError> {
    let mut charge = |count: usize| {
        *work_left = work_left
            .checked_sub(count)
            .ok_or(BaselineAppendError::Limit)?;
        Ok::<(), BaselineAppendError>(())
    };
    charge(
        programs.len()
            + base.rules().input().owners.len()
            + base.rules().input().receivers.members.len(),
    )?;
    let mut desired = BTreeMap::new();
    for (owner, program) in programs {
        if desired.insert(SubjectKey::from(owner), program).is_some() {
            return Err(BaselineAppendError::Preservation);
        }
    }
    if base
        .rules()
        .input()
        .receivers
        .members
        .iter()
        .any(|r| stats.contains(&r.stat))
    {
        return Err(BaselineAppendError::Preservation);
    }
    let mut rules = base.rules().input().clone();
    let mut seen = BTreeSet::new();
    let mut changed = 0;
    for row in &mut rules.owners {
        let expected = desired.get(&SubjectKey::from(&row.owner)).copied();
        for prior in &row.programs.members {
            charge(prior.effects.len() + 1)?;
            if expected == Some(prior) {
                continue;
            }
            if expected.is_some_and(|p| p.id == prior.id)
                || prior.effects.iter().any(|e| match &e.effect {
                    RuleEffectKind::Derive { stat, .. }
                    | RuleEffectKind::Contribute { stat, .. }
                    | RuleEffectKind::ProjectActorStat { stat, .. } => stats.contains(stat),
                    RuleEffectKind::Capability { capability, .. } => {
                        capabilities.contains(capability)
                    }
                    _ => false,
                })
            {
                return Err(BaselineAppendError::Preservation);
            }
        }
        if let Some(expected) = expected {
            if !row.programs.members.contains(expected) {
                if row.programs.is_complete() {
                    return Err(BaselineAppendError::Preservation);
                }
                charge(expected.nodes.len() + expected.effects.len())?;
                row.programs.members.push(expected.clone());
                changed += 1;
            }
            seen.insert(SubjectKey::from(&row.owner));
        }
    }
    if seen.iter().ne(desired.keys()) {
        return Err(BaselineAppendError::Preservation);
    }
    let checked = assemble_owned_recipe(
        OwnedRecipeInput {
            schema_version: OWNED_RECIPE_VERSION,
            registry: base.registry().input().clone(),
            schema: base.schema().input().clone(),
            rules,
            routing: base.routing().input().clone(),
        },
        limits,
    )?;
    Ok((
        OwnedRecipeInput {
            schema_version: OWNED_RECIPE_VERSION,
            registry: checked.registry().input().clone(),
            schema: checked.schema().input().clone(),
            rules: checked.rules().input().clone(),
            routing: checked.routing().input().clone(),
        },
        changed,
    ))
}
