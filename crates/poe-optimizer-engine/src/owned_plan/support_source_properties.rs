//! Same-attempt source census over sealed native support selections.
use super::compile::{
    BoundSourceProgram, BoundSourceProperties, BoundSourceRelation, BoundSupportAdmission,
};
use super::support_effects::PreparedContext;
use super::*;
use crate::owned_supports::SelectedSupports;

pub(super) struct RetainedSourceProgram<'a> {
    pub relation: &'a BoundSourceRelation,
    pub template: &'a BoundSourceProgram,
    pub position: Option<u32>,
}

/// None means the source selection was proven inactive. An active empty census
/// is Some(0); missing or partial evidence never creates either value.
pub(super) struct PreparedSourceCount<'a> {
    pub relation: &'a BoundSourceRelation,
    pub count: Option<BoundedInteger>,
}

#[derive(Default)]
pub(super) struct SourcePropertyAttempt<'a> {
    pub programs: Vec<RetainedSourceProgram<'a>>,
    pub counts: Vec<PreparedSourceCount<'a>>,
}

fn invalid(message: &str) -> PlanError {
    PlanError::Invalid(message.into())
}

impl<'a> SourcePropertyAttempt<'a> {
    fn append(
        &mut self,
        relation: &'a BoundSourceRelation,
        programs: &'a [BoundSourceProgram],
        position: Option<u32>,
        limits: PlanLimits,
        work: &mut usize,
    ) -> Result<()> {
        charge(work, programs.len())?;
        if self
            .programs
            .len()
            .checked_add(programs.len())
            .is_none_or(|n| n > limits.max_invocations)
        {
            return Err(PlanError::Limit("source property invocations"));
        }
        self.programs
            .extend(programs.iter().map(|template| RetainedSourceProgram {
                relation,
                template,
                position,
            }));
        Ok(())
    }

    pub(super) fn collect(
        &mut self,
        properties: &'a BoundSourceProperties,
        owner: &SkillTarget,
        selected: Option<&SelectedSupports>,
        contexts: Option<&BTreeMap<SkillTarget, PreparedContext>>,
        limits: PlanLimits,
        work: &mut usize,
    ) -> Result<()> {
        if selected.is_some() != contexts.is_some() {
            return Err(invalid("source census has inconsistent selection state"));
        }
        if properties.relations.is_empty() {
            return Ok(());
        }
        charge(work, 1 + (properties.by_owner.len() + 1).ilog2() as usize)?;
        let Some(indices) = properties.by_owner.get(owner) else {
            return Ok(());
        };
        charge(work, indices.len())?;
        for index in indices {
            let relation = &properties.relations[*index];
            if relation.owner != *owner {
                return Err(invalid("source relation owner index differs"));
            }
            if !relation.complete {
                return Err(invalid(
                    "source census requires complete bound contributors",
                ));
            }
            if self.counts.len() >= limits.max_owner_bindings {
                return Err(PlanError::Limit("source property relations"));
            }
            self.append(relation, &relation.external, None, limits, work)?;
            let count = if let (Some(selected), Some(contexts)) = (selected, contexts) {
                let mut count = 0i64;
                for (position, origin_index) in
                    selected.selected_origin_indices().iter().enumerate()
                {
                    charge(work, relation.effects.len() + 1)?;
                    let origin = selected
                        .origins()
                        .get(*origin_index)
                        .ok_or_else(|| invalid("source census has an invalid selected origin"))?;
                    let support = relation
                        .supports
                        .get(&origin.assignment)
                        .ok_or_else(|| invalid("source census has no complete support binding"))?;
                    let mut admitted = false;
                    for effect in &relation.effects {
                        let target = match effect {
                            BoundSupportAdmission::AssignedSkill { target }
                            | BoundSupportAdmission::ReceivingSkill { target, .. } => target,
                        };
                        let context = contexts
                            .get(target)
                            .ok_or_else(|| invalid("source effect has no admission context"))?;
                        let Some(prepared) = &context.prepared else {
                            continue;
                        };
                        let retained = prepared.selected.get(position).ok_or_else(|| {
                            invalid("source admission changed selected positions")
                        })?;
                        if retained.assignment != origin.assignment
                            || retained.position != position
                            || retained.origin_index != *origin_index
                        {
                            return Err(invalid(
                                "source admission changed selected origin identity",
                            ));
                        }
                        // One position can be admitted by several member effects.
                        // Distinct positions of one origin still visit this loop twice.
                        admitted |= retained.applicable;
                    }
                    if admitted {
                        if support.counted {
                            count = count
                                .checked_add(1)
                                .ok_or(PlanError::Limit("source support count"))?;
                        }
                        self.append(
                            relation,
                            &support.programs,
                            Some(
                                u32::try_from(position)
                                    .map_err(|_| PlanError::Limit("support positions"))?,
                            ),
                            limits,
                            work,
                        )?;
                    }
                }
                Some(
                    BoundedInteger::new(count)
                        .map_err(|_| PlanError::Limit("source support count"))?,
                )
            } else {
                None
            };
            self.counts.push(PreparedSourceCount { relation, count });
            self.append(relation, &relation.assembly, None, limits, work)?;
        }
        Ok(())
    }
}
