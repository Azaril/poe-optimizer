//! Cold output authority and same-attempt native membership publication.
use super::support_effects::{AdmissionStep, PreparedContext};
use super::*;
use poe_optimizer_data::owned_support_outputs::OwnedSupportOutputBindings;

/// An exact preparation occurrence within one ordered selection. The summoner
/// belongs to that same selection's explicit admission graph, never an inferred
/// ancestor. Plan identity binds the graph and all preparation definitions.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
pub struct SupportPreparationContextKey {
    pub selection: SkillTarget,
    pub target: SkillTarget,
    pub summoner: Option<SkillTarget>,
}
pub(super) fn target_copy_work(target: &SkillTarget) -> usize {
    match target {
        SkillTarget::Authored(_) => 1,
        SkillTarget::Generated(skill) => skill.provider.grant_path.len() + 2,
    }
}
impl SupportPreparationContextKey {
    pub(super) fn copy_work(&self) -> usize {
        target_copy_work(&self.selection)
            + target_copy_work(&self.target)
            + self.summoner.as_ref().map_or(0, target_copy_work)
    }
}

pub(super) struct BoundSupportOutputs {
    pub package: Arc<OwnedSupportOutputBindings>,
    pub keys: BTreeSet<PlanValueKey>,
    groups: BTreeMap<SkillTarget, BTreeMap<SkillTarget, SupportPreparationContextKey>>,
}

/// Private data from this execution, not a caller-supplied scalar or report.
pub(super) struct PreparedTypeOutput {
    pub context: SupportPreparationContextKey,
    pub support_type: OwnedDefinitionKey,
    pub stat: StatDefId,
    pub stage: OwnedDefinitionKey,
    pub member: Option<bool>,
}

impl BoundSupportOutputs {
    pub(super) fn bind<I>(
        package: Arc<OwnedSupportOutputBindings>,
        plan: &OwnedEffectPlan<I>,
        targets: BTreeSet<SkillTarget>,
        assignments: &mut BTreeMap<SkillTarget, Vec<SupportAssignmentId>>,
        admissions: &mut BTreeMap<SkillTarget, Vec<AdmissionStep>>,
        work: &mut usize,
    ) -> Result<Self> {
        let mut authority = BTreeMap::new();
        for (selection, rows) in admissions.iter() {
            charge(work, rows.len() + 1)?;
            for row in rows {
                if !targets.contains(&row.target) {
                    continue;
                }
                charge(
                    work,
                    target_copy_work(selection)
                        + 2 * target_copy_work(&row.target)
                        + row.summoner.as_ref().map_or(0, target_copy_work),
                )?;
                let context = SupportPreparationContextKey {
                    selection: selection.clone(),
                    target: row.target.clone(),
                    summoner: row.summoner.clone(),
                };
                if authority.insert(row.target.clone(), context).is_some() {
                    return Err(PlanError::Invalid(
                        "final support types have competing preparation contexts".into(),
                    ));
                }
            }
        }
        let total = targets
            .len()
            .checked_mul(package.input().final_skill_types.len())
            .and_then(|n| n.checked_add(plan.effects.len()))
            .filter(|n| *n <= plan.limits.max_effects)
            .ok_or(PlanError::Limit("prepared type effects"))?;
        charge(work, total - plan.effects.len())?;
        let mut groups: BTreeMap<_, BTreeMap<_, _>> = BTreeMap::new();
        let mut keys = BTreeSet::new();
        let mut context_count = admissions.values().map(Vec::len).sum::<usize>();
        for target in targets {
            charge(work, 1)?;
            if !plan.preparation_gates.contains_key(&target) {
                return Err(PlanError::Invalid(
                    "final support type target has no bound occurrence".into(),
                ));
            }
            let context = if let Some(context) = authority.remove(&target) {
                context
            } else {
                // No inherited context exists. The ordinary selection binder will
                // require the complete origin order, including known empty sets.
                context_count = context_count
                    .checked_add(1)
                    .filter(|n| *n <= plan.limits.max_owner_bindings)
                    .ok_or(PlanError::Limit("support admission contexts"))?;
                charge(work, 6 * target_copy_work(&target))?;
                assignments.entry(target.clone()).or_default();
                admissions.insert(
                    target.clone(),
                    vec![AdmissionStep {
                        target: target.clone(),
                        assigned: true,
                        summoner: None,
                    }],
                );
                SupportPreparationContextKey {
                    selection: target.clone(),
                    target: target.clone(),
                    summoner: None,
                }
            };
            charge(
                work,
                package
                    .input()
                    .final_skill_types
                    .len()
                    .checked_mul(target_copy_work(&target))
                    .ok_or(PlanError::Limit("work"))?,
            )?;
            for row in &package.input().final_skill_types {
                let key = PlanValueKey::Stat {
                    entity: ConcreteEntity::Skill(Box::new(target.clone())),
                    stat: row.stat.clone(),
                };
                if plan.values.contains_key(&key) {
                    return Err(PlanError::Invalid(
                        "prepared type conflicts with an ordinary final producer".into(),
                    ));
                }
                keys.insert(key);
            }
            charge(work, target_copy_work(&context.selection))?;
            groups
                .entry(context.selection.clone())
                .or_default()
                .insert(target, context);
        }
        Ok(Self {
            package,
            keys,
            groups,
        })
    }

    /// None is used only after the selection root was proven inactive. A missing
    /// row in a completed active admission group is an error, not false membership.
    pub(super) fn collect(
        &self,
        selection: &SkillTarget,
        contexts: Option<&BTreeMap<SkillTarget, PreparedContext>>,
        values: &mut Vec<PreparedTypeOutput>,
        work: &mut usize,
    ) -> Result<()> {
        let Some(group) = self.groups.get(selection) else {
            return Ok(());
        };
        charge(work, group.len())?;
        for (target, key) in group {
            let prepared = if let Some(contexts) = contexts {
                contexts
                    .get(target)
                    .ok_or_else(|| PlanError::Invalid("missing prepared output context".into()))?
                    .prepared
                    .as_ref()
            } else {
                None
            };
            if prepared.is_some_and(|p| !p.final_types_complete) {
                return Err(PlanError::Invalid(
                    "prepared output membership is incomplete".into(),
                ));
            }
            let rows = &self.package.input().final_skill_types;
            let lookup = prepared.map_or(1, |p| 1 + (p.final_types.len() + 1).ilog2() as usize);
            charge(
                work,
                rows.len()
                    .checked_mul(lookup + key.copy_work())
                    .ok_or(PlanError::Limit("work"))?,
            )?;
            for row in rows {
                values.push(PreparedTypeOutput {
                    context: key.clone(),
                    support_type: row.support_type.clone(),
                    stat: row.stat.clone(),
                    stage: self.package.input().output_stage.clone(),
                    member: prepared
                        .map(|p| p.final_types.binary_search(&row.support_type).is_ok()),
                });
            }
        }
        Ok(())
    }
}
