//! Cold, composed-request coverage; authored order cannot certify source absence.
use super::*;

impl<I: DefinitionSchemaIndex> Builder<'_, I> {
    /// Record potential requirement owners on the existing structural traversal,
    /// before activation or requested Actions can suppress their invocations.
    /// Unknown declarations still leave the ordinary whole-plan coverage gaps.
    pub(super) fn requirement_owner(&mut self, subject: SchemaSubject) -> Result<()> {
        charge(
            &mut self.work,
            self.requirement_owners.len().checked_ilog2().unwrap_or(0) as usize + 1,
        )?;
        let key = requirements::owner_key(&subject);
        if !self.requirement_owners.contains_key(&key) {
            if self.requirement_owners.len() >= self.limits.max_owner_bindings {
                return Err(PlanError::Limit("requirement owners"));
            }
            self.requirement_owners.insert(key, subject);
        }
        Ok(())
    }

    pub(super) fn requirement_gap(
        &mut self,
        provider: Option<&ProviderKey>,
        subject: Option<SchemaSubject>,
        reason: PlanGapReason,
    ) -> Result<()> {
        charge(
            &mut self.work,
            self.requirement_gaps.len() + provider.map_or(0, |p| p.grant_path.len()) + 1,
        )?;
        let gap = PlanGap {
            provider: provider.cloned(),
            subject,
            reason,
        };
        if !self.requirement_gaps.contains(&gap) {
            if self.requirement_gaps.len() >= self.limits.binding.max_issues {
                return Err(PlanError::Limit("requirement gaps"));
            }
            self.requirement_gaps.push(gap);
        }
        Ok(())
    }

    /// Potential output owners can supply supports even without an Action query.
    /// Keep the source census on the structural traversal, before activation.
    fn structural_outputs(
        &mut self,
        provider: &ProviderKey,
        outputs: &DeclaredSet<DeclaredSlot<ActionOutputDefId>>,
        support_coverage: bool,
    ) -> Result<()> {
        if !outputs.is_complete() {
            self.requirement_gap(Some(provider), None, PlanGapReason::PartialDeclarations)?;
            if support_coverage {
                self.gap(
                    Some(provider.clone()),
                    None,
                    PlanGapReason::PartialDeclarations,
                )?;
            }
        }
        charge(&mut self.work, outputs.members.len())?;
        for output in &outputs.members {
            let subject = SchemaSubject::Slot(ActionOutputDefId::address(output));
            self.requirement_owner(subject.clone())?;
            if support_coverage {
                self.check_support_domain(&subject, Some(provider))?;
            }
        }
        self.requirement_output_coverage(provider, &outputs.members)
    }

    fn requirement_potential_skill(
        &mut self,
        provider: &ProviderKey,
        skill: &SkillDefId,
        support_coverage: bool,
    ) -> Result<()> {
        let subject = SchemaSubject::Definition(skill.address());
        self.requirement_owner(subject.clone())?;
        match self.index.definition(skill) {
            SchemaLookup::Known(schema) => {
                self.structural_outputs(provider, &schema.declarations.outputs, support_coverage)
            }
            _ => self.requirement_gap(
                Some(provider),
                Some(subject),
                PlanGapReason::UnresolvedTopology,
            ),
        }
    }

    /// One node of the shared topology traversal. This never registers an
    /// executable provider, actor, Action, value or activation.
    pub(super) fn structural_children(
        &mut self,
        key: &ProviderKey,
        exposure: &ProviderExposure<'_>,
        pending: &mut BTreeSet<ProviderKey>,
        inactive: bool,
        support_coverage: bool,
    ) -> Result<()> {
        let mut declarations = vec![];
        match exposure {
            ProviderExposure::Root {
                owners,
                skills,
                implicit_passives,
            } => {
                for roots in implicit_passives {
                    if !roots.declaration().is_complete() {
                        self.usage_census_gap(
                            Some(key),
                            Some(owner_subject(roots.owner())),
                            PlanGapReason::PartialDeclarations,
                        )?;
                    }
                    charge(&mut self.work, roots.nodes().len())?;
                    for (node, schema) in roots.nodes() {
                        if !matches!(schema, SchemaLookup::Known(_)) {
                            self.usage_census_gap(
                                Some(key),
                                Some(SchemaSubject::Definition(node.address())),
                                PlanGapReason::UnresolvedTopology,
                            )?;
                        }
                    }
                }
                if let Some(skills) = skills {
                    charge(&mut self.work, skills.members.len())?;
                    for skill in &skills.members {
                        self.requirement_potential_skill(key, skill, support_coverage)?;
                    }
                    if !skills.is_complete() {
                        self.usage_census_gap(Some(key), None, PlanGapReason::PartialDeclarations)?;
                        if inactive {
                            self.gap(Some(key.clone()), None, PlanGapReason::PartialDeclarations)?;
                        }
                    }
                    if inactive {
                        charge(&mut self.work, skills.members.len())?;
                        for skill in &skills.members {
                            self.check_support_domain(
                                &SchemaSubject::Definition(skill.address()),
                                Some(key),
                            )?;
                        }
                    }
                }
                for owner in owners {
                    self.requirement_owner(owner.subject())?;
                    if inactive {
                        self.check_support_domain(&owner.subject(), Some(key))?;
                    }
                    declarations.push(owner.declarations());
                }
            }
            ProviderExposure::Skill {
                owner,
                declarations: d,
                outputs,
                ..
            } => {
                self.requirement_owner(owner_subject(owner))?;
                self.structural_outputs(key, outputs, support_coverage)?;
                if inactive {
                    self.check_support_domain(&owner_subject(owner), Some(key))?;
                }
                declarations.push(*d);
            }
            ProviderExposure::Actor {
                key: actor,
                schema,
                owner,
                ..
            } => {
                self.requirement_owner(SchemaSubject::Slot(ActorSlotDefId::address(&actor.slot)))?;
                charge(&mut self.work, schema.skills.members.len())?;
                for skill in &schema.skills.members {
                    self.requirement_potential_skill(key, skill, support_coverage)?;
                }
                self.structural_outputs(key, &schema.outputs, support_coverage)?;
                if !schema.skills.is_complete() {
                    self.usage_census_gap(Some(key), None, PlanGapReason::PartialDeclarations)?;
                }
                if inactive {
                    self.check_support_domain(
                        &SchemaSubject::Slot(ActorSlotDefId::address(&actor.slot)),
                        Some(key),
                    )?;
                    if !schema.skills.is_complete() || !schema.outputs.is_complete() {
                        self.gap(Some(key.clone()), None, PlanGapReason::PartialDeclarations)?;
                    }
                    charge(&mut self.work, schema.skills.members.len())?;
                    for skill in &schema.skills.members {
                        self.check_support_domain(
                            &SchemaSubject::Definition(skill.address()),
                            Some(key),
                        )?;
                    }
                }
                if let Some(owner) = owner {
                    self.requirement_owner(owner.subject())?;
                    if inactive {
                        self.check_support_domain(&owner.subject(), Some(key))?;
                    }
                    declarations.push(owner.declarations());
                } else if !schema.skills.members.is_empty() {
                    self.usage_census_gap(Some(key), None, PlanGapReason::UnresolvedTopology)?;
                }
            }
            ProviderExposure::AllocationAccess { .. } => {
                self.usage_census_gap(Some(key), None, PlanGapReason::UnsupportedRelation)?;
            }
        }
        for d in declarations {
            self.structural_outputs(key, &d.outputs, support_coverage)?;
            if !d.grants.is_complete() || !d.actors.is_complete() || !d.skill_grants.is_complete() {
                self.usage_census_gap(Some(key), None, PlanGapReason::PartialDeclarations)?;
                if inactive {
                    self.gap(Some(key.clone()), None, PlanGapReason::PartialDeclarations)?;
                }
            }
            charge(&mut self.work, d.grants.members.len())?;
            for grant in &d.grants.members {
                if !matches!(self.index.slot(grant), SchemaLookup::Known(_)) {
                    self.usage_census_gap(
                        Some(key),
                        Some(SchemaSubject::Slot(GrantSlotDefId::address(grant))),
                        PlanGapReason::UnresolvedTopology,
                    )?;
                    if inactive {
                        self.gap(
                            Some(key.clone()),
                            Some(SchemaSubject::Slot(GrantSlotDefId::address(grant))),
                            PlanGapReason::UnresolvedTopology,
                        )?;
                    }
                    continue;
                }
                if key.grant_path.len() >= self.limits.binding.input.max_provider_steps {
                    return Err(PlanError::Limit("provider depth"));
                }
                if pending.len() + self.providers.len() + self.inactive_providers.len() + 1
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
