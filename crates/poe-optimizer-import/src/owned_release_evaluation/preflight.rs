use super::{
    OwnedReleaseEvaluationContent, OwnedReleaseEvaluationInput, OwnedReleaseEvaluationLimits,
};
use poe_optimizer_core::{
    owned_definitions::OwnedDefinitionKey,
    owned_readiness::ReadinessInput,
    owned_schema::{DeclaredSet, SchemaClosure, SchemaState},
    owned_source_properties::{
        SourcePropertyEffectEndpoint, SourcePropertyOccurrence, SourcePropertyPreparationInput,
    },
    owned_stages::{
        EvaluationStage, FrozenStageChannel, StagedEffectApplication, StagedRuleProgram,
    },
    owned_support_inputs::SupportTargetInputBindings,
    owned_support_receiving::{
        SupportAdmissionContext, SupportReceivingEntry, SupportReceivingRole,
        SupportTargetReceivingRoles,
    },
    owned_supports::{SupportPreparationEntry, SupportTypePredicate},
};

// A borrowed semantic view shares the exact budget traversal between runtime
// inputs and authored content without copying payloads or inventing seals.
pub(super) struct EvaluationBudgetView<'a> {
    metric_bindings: usize,
    support: Option<SupportBudgetView<'a>>,
}
struct SupportBudgetView<'a> {
    stages: StagesBudgetView<'a>,
    preparation: PreparationBudgetView<'a>,
    target: &'a SupportTargetInputBindings,
    receiving: ReceivingBudgetView<'a>,
    outputs: Option<usize>,
}
struct StagesBudgetView<'a> {
    stages: &'a [EvaluationStage],
    programs: &'a DeclaredSet<StagedRuleProgram>,
    effect_applications: &'a Option<DeclaredSet<StagedEffectApplication>>,
    frozen_channels: &'a [FrozenStageChannel],
    readiness: &'a Option<ReadinessInput>,
}
struct PreparationBudgetView<'a> {
    types: &'a [OwnedDefinitionKey],
    effects: &'a [OwnedDefinitionKey],
    families: &'a [OwnedDefinitionKey],
    supports: &'a [SupportPreparationEntry],
}
struct ReceivingBudgetView<'a> {
    roles: &'a [SupportReceivingRole],
    targets: &'a [SupportTargetReceivingRoles],
    supports: &'a [SupportReceivingEntry],
    source_properties: &'a Option<SourcePropertyPreparationInput>,
}

// Only projection is repeated: every accounting rule below has one definition.
macro_rules! budget_view {
    ($input:ty) => {
        impl<'a> From<&'a $input> for EvaluationBudgetView<'a> {
            fn from(input: &'a $input) -> Self {
                Self {
                    metric_bindings: input.metrics.bindings.len(),
                    support: input.support.as_ref().map(|s| SupportBudgetView {
                        stages: StagesBudgetView {
                            stages: &s.stages.stages,
                            programs: &s.stages.programs,
                            effect_applications: &s.stages.effect_applications,
                            frozen_channels: &s.stages.frozen_channels,
                            readiness: &s.stages.readiness,
                        },
                        preparation: PreparationBudgetView {
                            types: &s.preparation.types,
                            effects: &s.preparation.effects,
                            families: &s.preparation.families,
                            supports: &s.preparation.supports,
                        },
                        target: &s.inputs.target,
                        receiving: ReceivingBudgetView {
                            roles: &s.receiving.roles,
                            targets: &s.receiving.targets,
                            supports: &s.receiving.supports,
                            source_properties: &s.receiving.source_properties,
                        },
                        outputs: s.outputs.as_ref().map(|o| o.final_skill_types.len()),
                    }),
                }
            }
        }
    };
}
budget_view!(OwnedReleaseEvaluationInput);
budget_view!(OwnedReleaseEvaluationContent);

/// Charge nested membership and semantic nodes before constituent cloning. The
/// shared counter includes the ordinary release; no per-bundle budget reset.
pub(super) fn check(
    input: EvaluationBudgetView<'_>,
    limits: OwnedReleaseEvaluationLimits,
    left: &mut usize,
) -> Result<(), crate::owned_release::OwnedReleaseError> {
    use crate::owned_release::OwnedReleaseError;
    fn charge(left: &mut usize, n: usize) -> Result<(), OwnedReleaseError> {
        *left = left
            .checked_sub(n)
            .ok_or(OwnedReleaseError::Limit("validation entries"))?;
        Ok(())
    }
    fn closure(left: &mut usize, value: &SchemaClosure) -> Result<(), OwnedReleaseError> {
        if let SchemaClosure::Partial { gaps } = value {
            charge(left, gaps.len())?;
        }
        Ok(())
    }
    fn predicate(
        left: &mut usize,
        value: &SupportTypePredicate,
        depth: usize,
        maximum: usize,
    ) -> Result<(), OwnedReleaseError> {
        // Bounded recursion avoids an unbounded traversal or stack before the
        // component constructor's own predicate validation and serialization.
        charge(left, 1)?;
        if depth > maximum {
            return Err(OwnedReleaseError::Limit("support predicate depth"));
        }
        match value {
            SupportTypePredicate::Type(_) => {}
            SupportTypePredicate::Not(value) => predicate(left, value, depth + 1, maximum)?,
            SupportTypePredicate::Any(values) | SupportTypePredicate::All(values) => {
                // Precharge the frontier before iterating, including empty nodes.
                charge(left, values.len())?;
                for value in values {
                    predicate(left, value, depth + 1, maximum)?;
                }
            }
        }
        Ok(())
    }
    charge(left, 1)?;
    charge(left, input.metric_bindings)?;
    let Some(s) = input.support else {
        return Ok(());
    };
    charge(left, 1)?;
    for count in [
        s.stages.stages.len(),
        s.stages.programs.members.len(),
        s.stages.frozen_channels.len(),
    ] {
        charge(left, count)?;
    }
    closure(left, &s.stages.programs.closure)?;
    if let Some(applications) = &s.stages.effect_applications {
        charge(left, applications.members.len())?;
        closure(left, &applications.closure)?;
    }
    for stage in s.stages.stages {
        charge(left, stage.predecessors.len())?;
    }
    if let Some(readiness) = &s.stages.readiness {
        charge(left, readiness.skills.len())?;
        charge(left, readiness.programs.members.len())?;
        closure(left, &readiness.programs.closure)?;
        for skill in &readiness.skills {
            charge(left, skill.parameters.members.len())?;
            charge(left, usize::from(skill.participation.is_some()))?;
            closure(left, &skill.parameters.closure)?;
        }
        for program in &readiness.programs.members {
            charge(left, program.outputs.len())?;
        }
    }
    for count in [
        s.preparation.types.len(),
        s.preparation.effects.len(),
        s.preparation.families.len(),
        s.preparation.supports.len(),
    ] {
        charge(left, count)?;
    }
    for support in s.preparation.supports {
        match &support.preparation {
            SchemaState::Unmapped { gaps } => charge(left, gaps.len())?,
            SchemaState::Known(p) => {
                charge(left, p.families.as_ref().map_or(0, Vec::len))?;
                charge(left, p.added_types.len())?;
                for p in [&p.requires, &p.excludes].into_iter().flatten() {
                    predicate(left, p, 1, limits.preparation.max_predicate_depth)?;
                }
            }
        }
    }
    charge(left, 9)?;
    for rows in [
        &s.target.skill_types,
        &s.target.minion_types.members,
        &s.target.summoner.skill_types,
        &s.target.summoner.minion_types.members,
    ] {
        charge(left, rows.len())?;
    }
    for count in [
        s.receiving.roles.len(),
        s.receiving.targets.len(),
        s.receiving.supports.len(),
    ] {
        charge(left, count)?;
    }
    for target in s.receiving.targets {
        charge(left, target.roles.members.len())?;
        closure(left, &target.roles.closure)?;
        for role in &target.roles.members {
            charge(left, role.endpoints.members.len())?;
            closure(left, &role.endpoints.closure)?;
            for endpoint in &role.endpoints.members {
                charge(left, endpoint.path().len())?;
                if endpoint.path().len() > limits.receiving.max_path_depth {
                    return Err(OwnedReleaseError::Limit("support receiver path depth"));
                }
                if let SupportAdmissionContext::ReceivingSkill {
                    summoner_path: Some(path),
                } = endpoint.admission()
                {
                    charge(left, path.len())?;
                    if path.len() > limits.receiving.max_path_depth {
                        return Err(OwnedReleaseError::Limit("support summoner path depth"));
                    }
                }
            }
        }
    }
    for support in s.receiving.supports {
        charge(left, support.receivers.members.len())?;
        closure(left, &support.receivers.closure)?;
        for role in &support.receivers.members {
            charge(left, role.delivery.len())?;
            if let Some(preparation) = &role.preparation {
                charge(left, 1)?;
                charge(left, preparation.properties.len())?;
            }
        }
    }
    if let Some(source) = &s.receiving.source_properties {
        charge(left, source.relations.members.len())?;
        closure(left, &source.relations.closure)?;
        for relation in &source.relations.members {
            charge(
                left,
                usize::from(matches!(
                    relation.occurrence,
                    SourcePropertyOccurrence::GeneratedSkill { .. }
                )),
            )?;
            for count in [
                relation.effects.members.len(),
                relation.inputs.len(),
                relation.channels.members.len(),
                relation.external.members.len(),
                relation.supports.members.len(),
                relation.assembly.members.len(),
            ] {
                charge(left, count)?;
            }
            // Invalid Partial inventories are still bounded before they are
            // cloned; only the semantic constructor may reject their authority.
            for value in [
                &relation.effects.closure,
                &relation.channels.closure,
                &relation.external.closure,
                &relation.supports.closure,
                &relation.assembly.closure,
            ] {
                closure(left, value)?;
            }
            for support in &relation.supports.members {
                charge(left, support.programs.members.len())?;
                closure(left, &support.programs.closure)?;
            }
            for effect in &relation.effects.members {
                if let SourcePropertyEffectEndpoint::Generated { path, .. } = &effect.endpoint {
                    charge(left, 1)?;
                    charge(left, path.len())?;
                    if path.len() > limits.receiving.max_path_depth {
                        return Err(OwnedReleaseError::Limit("source effect path depth"));
                    }
                }
                if let SupportAdmissionContext::ReceivingSkill {
                    summoner_path: Some(path),
                } = &effect.admission
                {
                    charge(left, path.len())?;
                    if path.len() > limits.receiving.max_path_depth {
                        return Err(OwnedReleaseError::Limit("source summoner path depth"));
                    }
                }
            }
        }
    }
    if let Some(outputs) = &s.outputs {
        charge(left, *outputs)?;
    }
    Ok(())
}
