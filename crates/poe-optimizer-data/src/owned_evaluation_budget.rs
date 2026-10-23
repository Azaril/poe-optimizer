//! Shared bounded accounting for authored and runtime evaluation declarations.
//! Charging entries establishes neither dependency validity nor coverage authority.
use crate::{
    owned_stages::EvaluationStagesContent,
    owned_support_receiving::{
        SupportReceivingContent, SupportReceivingStorageError, SupportReceivingStorageLimits,
    },
    owned_supports::{SupportPreparationContent, SupportStorageError, SupportStorageLimits},
};
use poe_optimizer_core::{
    owned_definitions::OwnedDefinitionKey,
    owned_readiness::ReadinessInput,
    owned_schema::{DeclaredSet, SchemaClosure, SchemaState},
    owned_source_properties::{
        SourcePropertyEffectEndpoint, SourcePropertyOccurrence, SourcePropertyPreparationInput,
    },
    owned_stages::{
        EvaluationStage, EvaluationStagesInput, FrozenStageChannel, StagedEffectApplication,
        StagedRuleProgram,
    },
    owned_support_inputs::SupportTargetInputBindings,
    owned_support_receiving::{
        SupportAdmissionContext, SupportReceivingEntry, SupportReceivingInput,
        SupportReceivingRole, SupportTargetReceivingRoles,
    },
    owned_supports::{SupportPreparationEntry, SupportPreparationInput, SupportTypePredicate},
};

/// Borrowed semantic entries share one traversal without copying payloads or seals.
pub struct EvaluationBudgetView<'a> {
    pub metric_bindings: usize,
    pub support: Option<SupportBudgetView<'a>>,
}
pub struct SupportBudgetView<'a> {
    pub stages: StagesBudgetView<'a>,
    pub preparation: PreparationBudgetView<'a>,
    pub target: &'a SupportTargetInputBindings,
    pub receiving: ReceivingBudgetView<'a>,
    pub outputs: Option<usize>,
}
pub struct StagesBudgetView<'a> {
    stages: &'a [EvaluationStage],
    programs: &'a DeclaredSet<StagedRuleProgram>,
    effect_applications: &'a Option<DeclaredSet<StagedEffectApplication>>,
    frozen_channels: &'a [FrozenStageChannel],
    readiness: &'a Option<ReadinessInput>,
}
pub struct PreparationBudgetView<'a> {
    types: &'a [OwnedDefinitionKey],
    effects: &'a [OwnedDefinitionKey],
    families: &'a [OwnedDefinitionKey],
    supports: &'a [SupportPreparationEntry],
}
pub struct ReceivingBudgetView<'a> {
    roles: &'a [SupportReceivingRole],
    targets: &'a [SupportTargetReceivingRoles],
    supports: &'a [SupportReceivingEntry],
    source_properties: &'a Option<SourcePropertyPreparationInput>,
}

// Only projection is repeated: every accounting rule below has one definition.
macro_rules! budget_view {
    ($view:ident, [$($input:ty),+], $fields:tt) => {
        $(budget_view!(@one $view, $input, $fields);)+
    };
    (@one $view:ident, $input:ty, [$($field:ident),+]) => {
        impl<'a> From<&'a $input> for $view<'a> {
            fn from(input: &'a $input) -> Self {
                Self { $($field: &input.$field),+ }
            }
        }
    };
}
budget_view!(
    StagesBudgetView,
    [EvaluationStagesInput, EvaluationStagesContent],
    [
        stages,
        programs,
        effect_applications,
        frozen_channels,
        readiness
    ]
);
budget_view!(
    PreparationBudgetView,
    [SupportPreparationInput, SupportPreparationContent],
    [types, effects, families, supports]
);
budget_view!(
    ReceivingBudgetView,
    [SupportReceivingInput, SupportReceivingContent],
    [roles, targets, supports, source_properties]
);

/// Checked traversal depths. Private fields prevent callers from bypassing the
/// existing storage hard ceilings and authorizing unbounded predicate recursion.
#[derive(Clone, Copy, Debug)]
pub struct EvaluationBudgetLimits {
    max_predicate_depth: usize,
    max_path_depth: usize,
}
impl EvaluationBudgetLimits {
    pub fn new(
        preparation: SupportStorageLimits,
        receiving: SupportReceivingStorageLimits,
    ) -> Result<Self, EvaluationBudgetError> {
        preparation.validate()?;
        receiving.validate()?;
        Ok(Self {
            max_predicate_depth: preparation.max_predicate_depth,
            max_path_depth: receiving.max_path_depth,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EvaluationBudgetError {
    #[error("evaluation declarations exceed {0}")]
    Limit(&'static str),
    #[error(transparent)]
    Preparation(#[from] SupportStorageError),
    #[error(transparent)]
    Receiving(#[from] SupportReceivingStorageError),
}

/// Charge nested membership and semantic nodes before constituent cloning. The
/// shared counter includes the ordinary release; no per-bundle budget reset.
pub fn charge_evaluation_entries(
    input: EvaluationBudgetView<'_>,
    limits: EvaluationBudgetLimits,
    left: &mut usize,
) -> Result<(), EvaluationBudgetError> {
    fn charge(left: &mut usize, n: usize) -> Result<(), EvaluationBudgetError> {
        *left = left
            .checked_sub(n)
            .ok_or(EvaluationBudgetError::Limit("validation entries"))?;
        Ok(())
    }
    fn closure(left: &mut usize, value: &SchemaClosure) -> Result<(), EvaluationBudgetError> {
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
    ) -> Result<(), EvaluationBudgetError> {
        // Bounded recursion avoids an unbounded traversal or stack before the
        // component constructor's own predicate validation and serialization.
        charge(left, 1)?;
        if depth > maximum {
            return Err(EvaluationBudgetError::Limit("support predicate depth"));
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
                    predicate(left, p, 1, limits.max_predicate_depth)?;
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
                if endpoint.path().len() > limits.max_path_depth {
                    return Err(EvaluationBudgetError::Limit("support receiver path depth"));
                }
                if let SupportAdmissionContext::ReceivingSkill {
                    summoner_path: Some(path),
                } = endpoint.admission()
                {
                    charge(left, path.len())?;
                    if path.len() > limits.max_path_depth {
                        return Err(EvaluationBudgetError::Limit("support summoner path depth"));
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
                    if path.len() > limits.max_path_depth {
                        return Err(EvaluationBudgetError::Limit("source effect path depth"));
                    }
                }
                if let SupportAdmissionContext::ReceivingSkill {
                    summoner_path: Some(path),
                } = &effect.admission
                {
                    charge(left, path.len())?;
                    if path.len() > limits.max_path_depth {
                        return Err(EvaluationBudgetError::Limit("source summoner path depth"));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_callers_cannot_authorize_unbounded_or_zero_depth() {
        let preparation = SupportStorageLimits::default();
        let receiving = SupportReceivingStorageLimits::default();
        EvaluationBudgetLimits::new(preparation, receiving).unwrap();
        for depth in [0, preparation.max_predicate_depth + 1, usize::MAX] {
            assert!(matches!(
                EvaluationBudgetLimits::new(
                    SupportStorageLimits {
                        max_predicate_depth: depth,
                        ..preparation
                    },
                    receiving,
                ),
                Err(EvaluationBudgetError::Preparation(
                    SupportStorageError::InvalidLimit("predicate depth")
                ))
            ));
        }
        for depth in [0, receiving.max_path_depth + 1, usize::MAX] {
            assert!(matches!(
                EvaluationBudgetLimits::new(
                    preparation,
                    SupportReceivingStorageLimits {
                        max_path_depth: depth,
                        ..receiving
                    },
                ),
                Err(EvaluationBudgetError::Receiving(
                    SupportReceivingStorageError::InvalidLimit("path depth")
                ))
            ));
        }
    }

    #[test]
    fn direct_accounting_preserves_prior_debits_and_cannot_overflow() {
        let limits = EvaluationBudgetLimits::new(Default::default(), Default::default()).unwrap();
        let mut exact = 3;
        charge_evaluation_entries(
            EvaluationBudgetView {
                metric_bindings: 2,
                support: None,
            },
            limits,
            &mut exact,
        )
        .unwrap();
        assert_eq!(exact, 0);
        let mut available = usize::MAX;
        assert!(matches!(
            charge_evaluation_entries(
                EvaluationBudgetView {
                    metric_bindings: usize::MAX,
                    support: None,
                },
                limits,
                &mut available,
            ),
            Err(EvaluationBudgetError::Limit("validation entries"))
        ));
        assert_eq!(available, usize::MAX - 1);
    }
}
