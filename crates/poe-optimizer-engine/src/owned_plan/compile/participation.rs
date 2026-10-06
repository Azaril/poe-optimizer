//! Requested participation is an execution consumer on the existing supply graph.
//! It never manufactures a provider, an input value, or contributor completeness.
use super::*;

pub(super) fn participation_stat<'a, I: DefinitionSchemaIndex>(
    target: &SkillTarget,
    request: &OwnedEvaluationRequest,
    index: &I,
    stages: &'a OwnedEvaluationStages,
    work: &mut usize,
) -> Result<Option<&'a StatDefId>> {
    let definition = match target {
        SkillTarget::Authored(id) => {
            let skills = &request.build().input().skills;
            charge(work, skills.len())?;
            match skills
                .iter()
                .find(|row| row.id == *id)
                .map(|row| &row.source)
            {
                Some(AuthoredSkillSource::Direct(definition)) => definition,
                // A physical Gem container is not an alias for any of its effects.
                Some(AuthoredSkillSource::Gem(_)) | None => return Ok(None),
            }
        }
        SkillTarget::Generated(skill) => {
            charge(work, skill.provider.grant_path.len() + 1)?;
            let SchemaLookup::Known(schema) = index.slot(&skill.slot) else {
                // The ordinary structural binding retains this schema gap.
                return Ok(None);
            };
            &schema.skill
        }
    };
    charge(work, 1)?;
    Ok(stages.skill_participation(definition))
}

impl<I: DefinitionSchemaIndex> Builder<'_, I> {
    pub(super) fn participation_gates(&mut self, context: &Context) -> Result<Vec<PendingRead>> {
        let Some(stages) = self
            .stages
            .filter(|_| self.operations.supports_skill_participation())
        else {
            return Ok(vec![]);
        };
        let mut targets = BTreeSet::new();
        if let Some(skill) = &context.skill {
            charge(&mut self.work, skill.provider.grant_path.len() + 2)?;
            targets.insert(SkillTarget::Generated(Box::new(skill.clone())));
        }
        if let ConcreteEntity::Skill(target) = &context.entity {
            charge(&mut self.work, support_outputs::target_copy_work(target))?;
            targets.insert(target.as_ref().clone());
        }
        let mut providers = BTreeSet::new();
        if let Some(provider) = &context.provider {
            charge(&mut self.work, provider.grant_path.len() + 1)?;
            providers.insert(provider);
        }
        // A support origin can be an assignment while its receiving actor belongs
        // to another provider. Mechanical actor grants alone do not encode usage.
        if let ActorKey::Owned(actor) = &context.actor {
            charge(&mut self.work, actor.provider.grant_path.len() + 1)?;
            providers.insert(&actor.provider);
        }
        for provider in providers {
            if let ProviderRoot::SkillUse(id) = provider.root {
                targets.insert(SkillTarget::Authored(id));
            }
            for depth in 0..=provider.grant_path.len() {
                charge(&mut self.work, depth + 1)?;
                let ancestor = ProviderKey {
                    root: provider.root.clone(),
                    grant_path: provider.grant_path[..depth].to_vec(),
                };
                if let Some(skill) = self.readiness_skills.get(&ancestor) {
                    charge(&mut self.work, skill.provider.grant_path.len() + 2)?;
                    targets.insert(SkillTarget::Generated(Box::new(skill.clone())));
                }
            }
        }
        let mut gates = Vec::new();
        for target in targets {
            if let Some(stat) =
                participation_stat(&target, self.request, self.index, stages, &mut self.work)?
            {
                gates.push(PendingRead::Value(PlanValueKey::Stat {
                    entity: ConcreteEntity::Skill(Box::new(target)),
                    stat: stat.clone(),
                }));
            }
        }
        Ok(gates)
    }
}
