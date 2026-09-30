//! Bind the explicit authored order to preparation facts, without source/UI discovery.
use super::*;
use poe_optimizer_core::owned_build::{BuildSpec, LoadoutScope, SupportOrigin};
use std::collections::BTreeMap;

/// Resolved numerical dependencies for one existing assignment. These values
/// cannot replace its Gem definition, target, enabled state, or selection order.
#[derive(Clone, Debug, PartialEq)]
pub struct EffectiveSupportValues {
    pub assignment: SupportAssignmentId,
    pub effective_level: Option<BoundedInteger>,
    pub effective_quality: Option<FiniteQuantity>,
}

/// Prepare the support origins declared for this exact target in a validated
/// build. Generated activation and receiving facts remain explicit caller input.
/// Missing effective values are unresolved only when the policy demands them.
pub fn prepare_build_supports(
    package: &OwnedSupportPreparation,
    build: &BuildSpec,
    target: &SupportPreparationTarget,
    values: &[EffectiveSupportValues],
    limits: SupportPreparationLimits,
) -> Result<SupportPreparationOutcome> {
    let mut budget = Budget::new(limits)?;
    let input = build.input();
    if input.game_version != package.input().namespace {
        return Err(SupportPreparationError::Invalid(
            "build and support preparation namespace differ",
        ));
    }
    let target_depth = match &target.target {
        SkillTarget::Authored(_) => 0,
        SkillTarget::Generated(skill) => skill.provider.grant_path.len(),
    };
    if target_depth > limits.max_target_depth {
        return Err(SupportPreparationError::Limit("target depth"));
    }
    let Some(sequences) = &input.support_origins else {
        return Ok(unresolved(SupportPreparationGap::OriginOrder, None));
    };
    budget.charge(
        sequences
            .len()
            .checked_mul(target_depth + 1)
            .ok_or(SupportPreparationError::Limit("work"))?,
    )?;
    let sequence = sequences
        .iter()
        .find(|sequence| sequence.target == target.target);
    let ordered = sequence.map_or(&[][..], |sequence| sequence.origins.as_slice());
    if ordered.len() > limits.max_origins || values.len() > limits.max_origins {
        return Err(SupportPreparationError::Limit("origins"));
    }
    budget.charge(values.len())?;
    let mut effective = BTreeMap::new();
    for value in values {
        budget.charge(input.supports.len() + effective.len() + 1)?;
        let Some(assignment) = input
            .supports
            .iter()
            .find(|assignment| assignment.id == value.assignment)
        else {
            return Err(SupportPreparationError::Invalid(
                "effective support input has no build assignment",
            ));
        };
        budget.charge(target_depth + 1)?;
        if assignment.target != target.target {
            return Err(SupportPreparationError::Invalid(
                "effective support input belongs to another target",
            ));
        }
        if effective.insert(value.assignment, value).is_some() {
            return Err(SupportPreparationError::Invalid(
                "duplicate effective support input",
            ));
        }
    }
    budget.charge(ordered.len())?;
    let mut origins = Vec::with_capacity(ordered.len());
    for origin in ordered {
        let SupportOrigin::Assignment(id) = origin;
        budget
            .charge(input.supports.len() + input.gems.len() + effective.len() + target_depth + 1)?;
        let assignment = input
            .supports
            .iter()
            .find(|assignment| assignment.id == *id)
            .ok_or(SupportPreparationError::Invalid(
                "support order has no build assignment",
            ))?;
        if assignment.target != target.target {
            return Err(SupportPreparationError::Invalid(
                "support order crosses its target",
            ));
        }
        let gem = input
            .gems
            .iter()
            .find(|gem| gem.id == assignment.support)
            .ok_or(SupportPreparationError::Invalid(
                "support assignment has no build gem",
            ))?;
        let scalar = effective.get(id);
        origins.push(ResolvedSupportOrigin {
            assignment: *id,
            gem: gem.definition.clone(),
            enabled: Some(assignment.enabled),
            effective_level: scalar.and_then(|value| value.effective_level),
            effective_quality: scalar.and_then(|value| value.effective_quality.clone()),
        });
    }
    // Authored disabled state is a fact of the build and cannot be overridden by
    // caller-supplied target activity. The active loadout also comes from the
    // build. Generated-grant activation still requires resolved target facts.
    if let SkillTarget::Authored(id) = target.target {
        budget.charge(input.skills.len())?;
        let skill = input.skills.iter().find(|skill| skill.id == id).ok_or(
            SupportPreparationError::Invalid("preparation target has no authored build skill"),
        )?;
        let in_loadout = match &skill.scope {
            LoadoutScope::Shared => true,
            LoadoutScope::Selected { loadouts } => {
                budget.charge(loadouts.len())?;
                loadouts.contains(&input.active_weapon_loadout)
            }
        };
        if !skill.enabled || !in_loadout {
            return Ok(SupportPreparationOutcome::Inactive {
                target: target.target.clone(),
            });
        }
    }
    if budget.remaining == 0 {
        return Err(SupportPreparationError::Limit("work"));
    }
    prepare_supports(
        package,
        &origins,
        target,
        SupportPreparationLimits {
            max_work: budget.remaining,
            ..limits
        },
    )
}
