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
    ) -> InputResult<SupportPreparationTarget> {
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
    ) -> InputResult<SupportPreparationTarget> {
        self.read_target(target, false, scratch, work)
    }

    fn read_target(
        &self,
        target: &SkillTarget,
        read_summoner: bool,
        scratch: &OwnedPlanScratch,
        work: &mut usize,
    ) -> InputResult<SupportPreparationTarget> {
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
        Ok(Ok(SupportPreparationTarget {
            target: target.clone(),
            enabled: Some(true),
            types,
            summoner,
            cannot_be_supported: Some(known!(self.boolean(
                target,
                scratch,
                work,
                &input.cannot_be_supported
            ))),
            has_gem: Some(known!(self.boolean(target, scratch, work, &input.has_gem))),
            from_item: Some(known!(self.boolean(
                target,
                scratch,
                work,
                &input.from_item
            ))),
            is_player_actor: Some(known!(self.boolean(
                target,
                scratch,
                work,
                &input.is_player_actor
            ))),
        }))
    }
}
