//! Private computed input access shared by support drivers. No caller scalars.
use super::*;
use crate::owned_supports::{SupportPreparationTarget, SupportTypeContext};
use poe_optimizer_core::owned_support_inputs::*;

pub(in crate::owned_plan) struct UnavailableInput {
    pub(in crate::owned_plan) key: Box<PlanValueKey>,
    pub(in crate::owned_plan) cause: EffectValue,
}
pub(in crate::owned_plan) type InputResult<T> =
    Result<std::result::Result<T, Box<UnavailableInput>>>;

/// Unknown flags retain their exact causes until applicability determines
/// whether they matter. This storage is bounded independently of support count.
#[derive(Default)]
pub(in crate::owned_plan) struct TargetInputFailures {
    cannot_be_supported: Option<Box<UnavailableInput>>,
    has_gem: Option<Box<UnavailableInput>>,
    from_item: Option<Box<UnavailableInput>>,
    is_player_actor: Option<Box<UnavailableInput>>,
}
impl TargetInputFailures {
    /// Call only for an applicability gap on the reported exact origin. Type
    /// inputs remain complete; an unused flag cannot explain that gap.
    pub(in crate::owned_plan) fn applicability_failure(
        self,
        preparation: &OwnedSupportPreparation,
        gem: &GemDefId,
        work: &mut usize,
    ) -> Result<Option<Box<UnavailableInput>>> {
        charge(work, preparation.input().supports.len().saturating_add(1))?;
        let Some(SchemaState::Known(definition)) = preparation.preparation_for(gem) else {
            return Err(invalid("support applicability gap has no known definition"));
        };
        charge(work, 4)?;
        for (used, failure) in [
            (true, self.cannot_be_supported),
            (definition.gems_only, self.has_gem),
            (
                definition.from_item && definition.is_support,
                self.from_item,
            ),
            (definition.is_trigger, self.is_player_actor),
        ] {
            if used && failure.is_some() {
                return Ok(failure);
            }
        }
        Ok(None)
    }
}

pub(in crate::owned_plan) struct ComputedSupportInputs<'a, I> {
    plan: &'a OwnedEffectPlan<I>,
    inputs: &'a OwnedSupportInputBindings,
    limits: SupportPreparationLimits,
}
impl<'a, I: DefinitionSchemaIndex> ComputedSupportInputs<'a, I> {
    pub(in crate::owned_plan) fn new(
        plan: &'a OwnedEffectPlan<I>,
        inputs: &'a OwnedSupportInputBindings,
        limits: SupportPreparationLimits,
    ) -> Self {
        Self {
            plan,
            inputs,
            limits,
        }
    }
    pub(in crate::owned_plan) fn stat(
        &self,
        scratch: &OwnedPlanScratch,
        work: &mut usize,
        entity: ConcreteEntity,
        stat: &StatDefId,
    ) -> Result<EffectValue> {
        graph::read(
            &ReadBinding::Final {
                effect: self
                    .plan
                    .values
                    .get(&PlanValueKey::Stat {
                        entity,
                        stat: stat.clone(),
                    })
                    .copied(),
                complete: self.plan.complete,
            },
            &scratch.values,
            stat.key(),
            work,
        )
    }
    fn boolean(
        &self,
        target: &SkillTarget,
        scratch: &OwnedPlanScratch,
        work: &mut usize,
        stat: &StatDefId,
    ) -> InputResult<bool> {
        Ok(
            match self.stat(
                scratch,
                work,
                ConcreteEntity::Skill(Box::new(target.clone())),
                stat,
            )? {
                EffectValue::Known {
                    value: ParameterValue::Boolean(v),
                } => Ok(v),
                EffectValue::Known { .. } => {
                    return Err(invalid("computed support input is not boolean"));
                }
                cause => Err(Box::new(UnavailableInput {
                    key: Box::new(PlanValueKey::Stat {
                        entity: ConcreteEntity::Skill(Box::new(target.clone())),
                        stat: stat.clone(),
                    }),
                    cause,
                })),
            },
        )
    }
    fn type_set(
        &self,
        target: &SkillTarget,
        rows: &[SupportTypeStat],
        scratch: &OwnedPlanScratch,
        work: &mut usize,
    ) -> InputResult<DeclaredSet<OwnedDefinitionKey>> {
        charge(work, rows.len())?;
        let mut members = Vec::new();
        for row in rows {
            match self.boolean(target, scratch, work, &row.stat)? {
                Ok(true) => {
                    if members.len() >= self.limits.max_types {
                        return Err(PlanError::Limit("types"));
                    }
                    members.push(row.support_type.clone());
                }
                Ok(false) => {}
                Err(cause) => return Ok(Err(cause)),
            }
        }
        Ok(Ok(DeclaredSet::complete(members)))
    }
    fn optional_boolean(
        &self,
        target: &SkillTarget,
        scratch: &OwnedPlanScratch,
        work: &mut usize,
        stat: &StatDefId,
        failure: &mut Option<Box<UnavailableInput>>,
    ) -> Result<Option<bool>> {
        // Only unavailable values become unknown facts. Hard errors, including
        // type mismatches and exhausted work, retain their ordinary propagation.
        match self.boolean(target, scratch, work, stat)? {
            Ok(value) => Ok(Some(value)),
            Err(cause) => {
                *failure = Some(cause);
                Ok(None)
            }
        }
    }
    fn optional_types(
        &self,
        target: &SkillTarget,
        input: &OptionalTypeInputs,
        scratch: &OwnedPlanScratch,
        work: &mut usize,
    ) -> InputResult<Option<DeclaredSet<OwnedDefinitionKey>>> {
        match self.boolean(target, scratch, work, &input.present)? {
            Ok(false) => Ok(Ok(None)),
            Ok(true) => Ok(self
                .type_set(target, &input.members, scratch, work)?
                .map(Some)),
            Err(cause) => Ok(Err(cause)),
        }
    }
    pub(in crate::owned_plan) fn target_inputs(
        &self,
        target: &SkillTarget,
        scratch: &OwnedPlanScratch,
        work: &mut usize,
    ) -> InputResult<(SupportPreparationTarget, TargetInputFailures)> {
        self.read_target(target, true, scratch, work)
    }

    /// Receiver admission obtains summoner facts from explicit receiving
    /// metadata and the parent's native prepared result. Do not demand unrelated
    /// frozen summoner input fields while reading the receiver's own facts.
    pub(in crate::owned_plan) fn receiver_inputs(
        &self,
        target: &SkillTarget,
        scratch: &OwnedPlanScratch,
        work: &mut usize,
    ) -> InputResult<(SupportPreparationTarget, TargetInputFailures)> {
        self.read_target(target, false, scratch, work)
    }

    fn read_target(
        &self,
        target: &SkillTarget,
        read_summoner: bool,
        scratch: &OwnedPlanScratch,
        work: &mut usize,
    ) -> InputResult<(SupportPreparationTarget, TargetInputFailures)> {
        macro_rules! known {
            ($v:expr) => {
                match $v? {
                    Ok(value) => value,
                    Err(cause) => return Ok(Err(cause)),
                }
            };
        }
        let input = &self.inputs.input().target;
        let types = SupportTypeContext {
            skill_types: known!(self.type_set(target, &input.skill_types, scratch, work)),
            minion_types: known!(self.optional_types(target, &input.minion_types, scratch, work)),
        };
        let summoner = if read_summoner
            && known!(self.boolean(target, scratch, work, &input.summoner.present))
        {
            Some(SupportTypeContext {
                skill_types: known!(self.type_set(
                    target,
                    &input.summoner.skill_types,
                    scratch,
                    work
                )),
                minion_types: known!(self.optional_types(
                    target,
                    &input.summoner.minion_types,
                    scratch,
                    work
                )),
            })
        } else {
            None
        };
        let mut failures = TargetInputFailures::default();
        Ok(Ok((
            SupportPreparationTarget {
                target: target.clone(),
                enabled: Some(true),
                types,
                summoner,
                cannot_be_supported: self.optional_boolean(
                    target,
                    scratch,
                    work,
                    &input.cannot_be_supported,
                    &mut failures.cannot_be_supported,
                )?,
                has_gem: self.optional_boolean(
                    target,
                    scratch,
                    work,
                    &input.has_gem,
                    &mut failures.has_gem,
                )?,
                from_item: self.optional_boolean(
                    target,
                    scratch,
                    work,
                    &input.from_item,
                    &mut failures.from_item,
                )?,
                is_player_actor: self.optional_boolean(
                    target,
                    scratch,
                    work,
                    &input.is_player_actor,
                    &mut failures.is_player_actor,
                )?,
            },
            failures,
        )))
    }
}
