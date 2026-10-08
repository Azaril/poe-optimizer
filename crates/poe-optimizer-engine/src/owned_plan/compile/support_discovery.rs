//! Cold, composed-request coverage; authored order cannot certify source absence.
use super::*;

impl<I: DefinitionSchemaIndex> Builder<'_, I> {
    /// The ordinary resolver intentionally omits disabled roots. Inspect their
    /// same declared occurrence paths for coverage only; no actor, action,
    /// invocation, value or activation is installed in the execution graph.
    pub(super) fn check_inactive_support_sources(&mut self, root: &ProviderKey) -> Result<()> {
        let mut pending = BTreeSet::from([root.clone()]);
        while let Some(key) = pending.pop_first() {
            charge(&mut self.work, key.grant_path.len() + 1)?;
            if self.inactive_support_providers.contains(&key) {
                continue;
            }
            if self.providers.len() + self.inactive_support_providers.len()
                >= self.limits.max_providers
            {
                return Err(PlanError::Limit("providers"));
            }
            self.inactive_support_providers.insert(key.clone());
            let resolved = self.resolver.structural_provider(&key)?;
            charge(&mut self.work, resolved.work_used())?;
            let Some(provider) = resolved.into_value() else {
                self.gap(Some(key), None, PlanGapReason::UnresolvedTopology)?;
                continue;
            };
            let mut declarations = vec![];
            match provider.exposure() {
                ProviderExposure::Root { owners, skills, .. } => {
                    if let Some(skills) = skills {
                        charge(&mut self.work, skills.members.len() + 1)?;
                        if !skills.is_complete() {
                            self.gap(Some(key.clone()), None, PlanGapReason::PartialDeclarations)?;
                        }
                        for skill in &skills.members {
                            self.check_support_domain(
                                &SchemaSubject::Definition(skill.address()),
                                Some(&key),
                            )?;
                        }
                    }
                    for owner in owners {
                        self.check_support_domain(&owner.subject(), Some(&key))?;
                        declarations.push(owner.declarations());
                    }
                }
                ProviderExposure::Skill {
                    owner,
                    declarations: d,
                    ..
                } => {
                    self.check_support_domain(&owner_subject(owner), Some(&key))?;
                    declarations.push(*d);
                }
                ProviderExposure::Actor {
                    key: actor,
                    schema,
                    owner,
                    ..
                } => {
                    self.check_support_domain(
                        &SchemaSubject::Slot(ActorSlotDefId::address(&actor.slot)),
                        Some(&key),
                    )?;
                    if !schema.skills.is_complete() || !schema.outputs.is_complete() {
                        self.gap(Some(key.clone()), None, PlanGapReason::PartialDeclarations)?;
                    }
                    charge(&mut self.work, schema.skills.members.len())?;
                    for skill in &schema.skills.members {
                        self.check_support_domain(
                            &SchemaSubject::Definition(skill.address()),
                            Some(&key),
                        )?;
                    }
                    if let Some(owner) = owner {
                        self.check_support_domain(&owner.subject(), Some(&key))?;
                        declarations.push(owner.declarations());
                    }
                }
                ProviderExposure::AllocationAccess { .. } => {}
            }
            for d in declarations {
                if !d.grants.is_complete()
                    || !d.actors.is_complete()
                    || !d.skill_grants.is_complete()
                {
                    self.gap(Some(key.clone()), None, PlanGapReason::PartialDeclarations)?;
                }
                charge(&mut self.work, d.grants.members.len())?;
                for grant in &d.grants.members {
                    if key.grant_path.len() >= self.limits.binding.input.max_provider_steps {
                        return Err(PlanError::Limit("provider depth"));
                    }
                    if pending.len() + self.providers.len() + self.inactive_support_providers.len()
                        >= self.limits.max_providers
                    {
                        return Err(PlanError::Limit("providers"));
                    }
                    charge(&mut self.work, key.grant_path.len() + 1)?;
                    let mut child = key.clone();
                    child.grant_path.push(grant.clone());
                    pending.insert(child);
                }
            }
        }
        Ok(())
    }

    pub(super) fn check_support_domain(
        &mut self,
        subject: &SchemaSubject,
        provider: Option<&ProviderKey>,
    ) -> Result<()> {
        charge(
            &mut self.work,
            self.rules
                .input()
                .support_discovery
                .as_ref()
                .map_or(0, |d| d.providers.len())
                .checked_ilog2()
                .unwrap_or(0) as usize
                + 1,
        )?;
        let reason = match self.rules.support_source_domain(subject) {
            Some(true) => return Ok(()),
            Some(false) => PlanGapReason::UnmappedSupportSources,
            None => PlanGapReason::MissingSupportSources,
        };
        self.gap(provider.cloned(), Some(subject.clone()), reason)
    }
}
