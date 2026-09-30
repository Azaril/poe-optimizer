//! Exact activation paths for the private computed-preparation driver.
use super::*;

impl<I: DefinitionSchemaIndex> Builder<'_, I> {
    pub(super) fn preparation_gates(&mut self) -> Result<BTreeMap<SkillTarget, Vec<PendingRead>>> {
        let input = self.request.build().input();
        charge(
            &mut self.work,
            input.skills.len() + input.supports.len() + self.skill_supplies.len(),
        )?;
        let mut targets = BTreeSet::new();
        for target in input
            .skills
            .iter()
            .map(|s| SkillTarget::Authored(s.id))
            .chain(input.supports.iter().map(|s| s.target.clone()))
            .chain(
                self.skill_supplies
                    .keys()
                    .cloned()
                    .map(|s| SkillTarget::Generated(Box::new(s))),
            )
        {
            if targets.len() >= self.limits.max_providers && !targets.contains(&target) {
                return Err(PlanError::Limit("preparation targets"));
            }
            targets.insert(target);
        }
        let mut result = BTreeMap::new();
        for target in targets {
            let resolution = self.resolver.skill(&target)?;
            charge(&mut self.work, resolution.work_used() + 1)?;
            let status = resolution.status();
            let gates = if let Some(resolved) = resolution.into_value() {
                let (provider, skill) = match &target {
                    SkillTarget::Authored(id) => (root(ProviderRoot::SkillUse(*id)), None),
                    SkillTarget::Generated(skill) => {
                        // GeneratedSkillKey.provider is the parent. Only the discovered
                        // supply includes the entering ability grant and all its gates.
                        let Some(provider) = self.skill_supplies.get(skill.as_ref()).cloned()
                        else {
                            result
                                .insert(target, vec![missing(PlanGapReason::UnresolvedActivation)]);
                            continue;
                        };
                        (provider, Some(skill.as_ref().clone()))
                    }
                };
                let context = Context {
                    origin: RuleOrigin::Provider {
                        provider: provider.clone(),
                    },
                    provider: Some(provider),
                    actor: resolved.provider().actor().clone(),
                    skill,
                    receiving_skill: None,
                    assigned_skill: None,
                    entity: ConcreteEntity::Skill(Box::new(target.clone())),
                };
                self.context_gates(&context)?
            } else if status == SelectorBindingStatus::Unavailable {
                vec![PendingRead::Ready(ReadBinding::Constant(Some(
                    ParameterValue::Boolean(false),
                )))]
            } else {
                vec![missing(PlanGapReason::UnresolvedTopology)]
            };
            result.insert(target, gates);
        }
        Ok(result)
    }
}
