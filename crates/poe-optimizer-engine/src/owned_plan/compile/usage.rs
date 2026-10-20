//! Explicit usage selections bind one ordinary rule invocation to one occurrence.
//! Policy meanings and values remain in the injected schema and rule package.
use super::*;

/// One census candidate; an unadmitted entering path still participates in the
/// uniqueness proof for that exact Skill key. It never becomes a usage record.
pub(super) struct UsageCandidate {
    provider: ProviderKey,
    admitted: bool,
}

impl<I: DefinitionSchemaIndex> Builder<'_, I> {
    pub(super) fn usage_census_gap(
        &mut self,
        provider: Option<&ProviderKey>,
        subject: Option<SchemaSubject>,
        reason: PlanGapReason,
    ) -> Result<()> {
        // These gaps describe the shared structural traversal, including
        // inactive providers, whether or not any usage intent needs that census.
        self.requirement_gap(provider, subject.clone(), reason)?;
        if self.request.scenario().input().usage_intents.is_empty() {
            return Ok(());
        }
        charge(
            &mut self.work,
            self.usage_census_gaps.len() + provider.map_or(0, |p| p.grant_path.len()) + 1,
        )?;
        let gap = PlanGap {
            provider: provider.cloned(),
            subject,
            reason,
        };
        if !self.usage_census_gaps.contains(&gap) {
            if self.usage_census_gaps.len() >= self.limits.binding.max_issues {
                return Err(PlanError::Limit("usage census gaps"));
            }
            self.usage_census_gaps.push(gap);
        }
        Ok(())
    }

    pub(super) fn initialize_usage_census(&mut self) -> Result<()> {
        let intents = &self.request.scenario().input().usage_intents;
        if intents.len() > self.limits.max_owner_bindings {
            return Err(PlanError::Limit("usage intents"));
        }
        charge(&mut self.work, intents.len())?;
        for (index, intent) in intents.iter().enumerate() {
            let subject = SchemaSubject::Definition(intent.policy.address());
            let mut gaps = vec![];
            match self.index.definition(&intent.policy) {
                SchemaLookup::Known(schema) => {
                    let Some(allowed) = &schema.scenario_skill_applicability else {
                        return Err(PlanError::Invalid(
                            "Scenario usage policy has no expansion authority".into(),
                        ));
                    };
                    if !allowed.is_complete() {
                        gaps.push(PlanGap {
                            provider: None,
                            subject: Some(subject),
                            reason: PlanGapReason::PartialDeclarations,
                        });
                    }
                    charge(&mut self.work, allowed.members.len())?;
                    for allowance in &allowed.members {
                        let known = self.usage_allowance_known(allowance)?;
                        if !known {
                            let gap = PlanGap {
                                provider: None,
                                subject: Some(SchemaSubject::Definition(intent.policy.address())),
                                reason: PlanGapReason::UnresolvedTopology,
                            };
                            if !gaps.contains(&gap) {
                                gaps.push(gap);
                            }
                        }
                    }
                }
                _ => gaps.push(PlanGap {
                    provider: None,
                    subject: Some(subject),
                    reason: PlanGapReason::UnresolvedTopology,
                }),
            }
            self.usage_matches.push(BTreeMap::new());
            self.usage_resolution.intents.push(ResolvedUsageIntent {
                index,
                complete: false,
                matches: vec![],
                gaps,
            });
        }
        Ok(())
    }

    fn usage_allowance_known(&mut self, allowance: &UsageSkillApplicability) -> Result<bool> {
        match allowance {
            UsageSkillApplicability::AuthoredDirect { skill } => {
                match self.index.definition(skill) {
                    SchemaLookup::Known(schema) if schema.directly_selectable => Ok(true),
                    SchemaLookup::Known(_)
                    | SchemaLookup::NamespaceMismatch
                    | SchemaLookup::InconsistentIndex => Err(PlanError::Invalid(
                        "invalid direct Scenario usage applicability".into(),
                    )),
                    _ => Ok(false),
                }
            }
            UsageSkillApplicability::Supplied(supply) => {
                let mut known = match self.index.definition(&supply.skill) {
                    SchemaLookup::Known(_) => true,
                    SchemaLookup::NamespaceMismatch | SchemaLookup::InconsistentIndex => {
                        return Err(PlanError::Invalid("invalid Scenario usage Skill".into()));
                    }
                    _ => false,
                };
                match self.index.slot(&supply.grant) {
                    SchemaLookup::Known(schema)
                        if schema.target == GrantTarget::Skill(supply.supply.clone()) => {}
                    SchemaLookup::Known(_)
                    | SchemaLookup::NamespaceMismatch
                    | SchemaLookup::InconsistentIndex => {
                        return Err(PlanError::Invalid(
                            "Scenario usage grant has a different target".into(),
                        ));
                    }
                    _ => known = false,
                }
                match self.index.slot(&supply.supply) {
                    SchemaLookup::Known(schema) if schema.skill == supply.skill => {}
                    SchemaLookup::Known(_)
                    | SchemaLookup::NamespaceMismatch
                    | SchemaLookup::InconsistentIndex => {
                        return Err(PlanError::Invalid(
                            "Scenario usage supply has a different Skill".into(),
                        ));
                    }
                    _ => known = false,
                }
                for (owner, grant) in [
                    (&supply.grant.declaration, true),
                    (&supply.supply.declaration, false),
                ] {
                    macro_rules! declarations {
                        ($id:expr) => {
                            match self.index.definition($id) {
                                SchemaLookup::Known(schema) => Some(&schema.declarations),
                                SchemaLookup::NamespaceMismatch
                                | SchemaLookup::InconsistentIndex => {
                                    return Err(PlanError::Invalid(
                                        "invalid Scenario usage declaration".into(),
                                    ))
                                }
                                _ => None,
                            }
                        };
                    }
                    let d = match owner {
                        SlotOwnerDefId::Class(id) => declarations!(id),
                        SlotOwnerDefId::Ascendancy(id) => declarations!(id),
                        SlotOwnerDefId::Reward(id) => declarations!(id),
                        SlotOwnerDefId::ItemTemplate(id) => declarations!(id),
                        SlotOwnerDefId::Modifier(id) => declarations!(id),
                        SlotOwnerDefId::Gem(id) => declarations!(id),
                        SlotOwnerDefId::Skill(id) => declarations!(id),
                        SlotOwnerDefId::Actor(id) => declarations!(id),
                        SlotOwnerDefId::PassiveNode(id) => declarations!(id),
                        SlotOwnerDefId::UsagePolicy(id) => declarations!(id),
                    };
                    let Some(d) = d else {
                        known = false;
                        continue;
                    };
                    let (complete, member) = if grant {
                        charge(&mut self.work, d.grants.members.len())?;
                        (
                            d.grants.is_complete(),
                            d.grants.members.contains(&supply.grant),
                        )
                    } else {
                        charge(&mut self.work, d.skill_grants.members.len())?;
                        (
                            d.skill_grants.is_complete(),
                            d.skill_grants.members.contains(&supply.supply),
                        )
                    };
                    if complete && !member {
                        return Err(PlanError::Invalid(
                            "Scenario usage relation is not declared by its owner".into(),
                        ));
                    }
                    known &= complete && member;
                }
                Ok(known)
            }
        }
    }

    pub(super) fn census_usage_occurrence(
        &mut self,
        key: &ProviderKey,
        actor: &ActorKey,
        exposure: &ProviderExposure<'_>,
    ) -> Result<()> {
        // Requirement coverage also needs this structural census when no
        // Scenario intent is present. It observes inactive supplied providers.
        let potentials = match exposure {
            ProviderExposure::Root { skills, .. } => skills.as_ref().map(|s| &s.members),
            ProviderExposure::Actor { schema, .. } => Some(&schema.skills.members),
            _ => None,
        };
        if let Some(potentials) = potentials {
            for skill in potentials {
                charge(&mut self.work, key.grant_path.len() + 1)?;
                if self.potential_skill_census.len() >= self.limits.max_owner_bindings {
                    return Err(PlanError::Limit("usage potential skills"));
                }
                self.potential_skill_census
                    .insert((key.clone(), skill.clone()), actor.clone());
            }
        }
        let candidate = match exposure {
            ProviderExposure::Root { owners, .. } if key.grant_path.is_empty() => {
                if let ProviderRoot::SkillUse(id) = key.root {
                    owners.iter().find_map(|owner| match owner.definition() {
                        SlotOwnerDefId::Skill(skill) => Some((SkillTarget::Authored(id), skill)),
                        _ => None,
                    })
                } else {
                    None
                }
            }
            ProviderExposure::Skill {
                key: skill,
                owner: SlotOwnerDefId::Skill(definition),
                ..
            } => {
                charge(&mut self.work, self.potential_skill_census.len())?;
                self.potential_skill_census
                    .retain(|(potential, skill), owner_actor| {
                        !(skill == definition
                            && owner_actor == actor
                            && potential.root == key.root
                            && key.grant_path.starts_with(&potential.grant_path))
                    });
                Some((SkillTarget::Generated(Box::new(skill.clone())), definition))
            }
            _ => None,
        };
        let Some((target, definition)) = candidate else {
            return Ok(());
        };
        for (index, intent) in self
            .request
            .scenario()
            .input()
            .usage_intents
            .iter()
            .enumerate()
        {
            let SchemaLookup::Known(schema) = self.index.definition(&intent.policy) else {
                continue;
            };
            let Some(allowed) = &schema.scenario_skill_applicability else {
                continue;
            };
            charge(&mut self.work, allowed.members.len() + 1)?;
            let mut matching = 0usize;
            let mut considered = false;
            for allowance in &allowed.members {
                considered |= match allowance {
                    UsageSkillApplicability::AuthoredDirect { skill } => skill == definition,
                    UsageSkillApplicability::Supplied(supply) => &supply.skill == definition,
                };
                let matches = match (allowance, &target) {
                    (
                        UsageSkillApplicability::AuthoredDirect { skill },
                        SkillTarget::Authored(_),
                    ) => skill == definition,
                    (UsageSkillApplicability::Supplied(supply), SkillTarget::Generated(skill)) => {
                        &supply.skill == definition
                            && supply.supply == skill.slot
                            && key.grant_path.last() == Some(&supply.grant)
                    }
                    _ => false,
                };
                matching += usize::from(matches);
            }
            if matching > 1 {
                return Err(PlanError::Invalid(
                    "overlapping Scenario usage applicability".into(),
                ));
            }
            if !considered {
                continue;
            }
            charge(
                &mut self.work,
                self.usage_matches.len() + key.grant_path.len() + 1,
            )?;
            let total = self.usage_matches.iter().map(BTreeMap::len).sum::<usize>();
            if total >= self.limits.max_owner_bindings {
                return Err(PlanError::Limit("usage matches"));
            }
            if let Some(previous) = self.usage_matches[index].insert(
                target.clone(),
                UsageCandidate {
                    provider: key.clone(),
                    admitted: matching == 1,
                },
            ) && previous.provider != *key
            {
                return Err(PlanError::Invalid(
                    "ambiguous Scenario usage supplying relation".into(),
                ));
            }
        }
        Ok(())
    }

    pub(super) fn resolve_usage(&mut self) -> Result<()> {
        for ((provider, skill), _) in std::mem::take(&mut self.potential_skill_census) {
            self.usage_census_gap(
                Some(&provider),
                Some(SchemaSubject::Definition(skill.address())),
                PlanGapReason::UnresolvedTopology,
            )?;
        }
        let mut resolved =
            BTreeMap::<(UsagePolicyDefId, UsageTarget), ResolvedUsageSelection>::new();
        for (index, selection) in self
            .request
            .build()
            .input()
            .usage_preferences
            .iter()
            .enumerate()
        {
            Self::replace_usage(
                &mut resolved,
                selection,
                UsageSelectionSource::Preset { index },
                &mut self.work,
                self.limits.max_owner_bindings,
            )?;
        }
        let mut diagnostic_count = 0usize;
        for (index, matches) in std::mem::take(&mut self.usage_matches)
            .into_iter()
            .enumerate()
        {
            let intent = &self.request.scenario().input().usage_intents[index];
            let row = &mut self.usage_resolution.intents[index];
            charge(&mut self.work, self.usage_census_gaps.len())?;
            diagnostic_count = diagnostic_count
                .checked_add(row.gaps.len() + self.usage_census_gaps.len())
                .ok_or(PlanError::Limit("usage resolution diagnostics"))?;
            if diagnostic_count > self.limits.max_owner_bindings {
                return Err(PlanError::Limit("usage resolution diagnostics"));
            }
            if row.gaps.len() + self.usage_census_gaps.len() > self.limits.binding.max_issues {
                return Err(PlanError::Limit("usage intent gaps"));
            }
            for gap in &self.usage_census_gaps {
                charge(
                    &mut self.work,
                    gap.provider
                        .as_ref()
                        .map_or(0, |provider| provider.grant_path.len()),
                )?;
            }
            row.gaps.extend(self.usage_census_gaps.iter().cloned());
            row.complete = row.gaps.is_empty();
            for (target, candidate) in matches {
                if !candidate.admitted {
                    continue;
                }
                let provider = candidate.provider;
                charge(
                    &mut self.work,
                    intent.parameters.len() + provider.grant_path.len() + 1,
                )?;
                let selection = UsagePolicySelection {
                    policy: intent.policy.clone(),
                    target: UsageTarget::Skill(target.clone()),
                    parameters: intent.parameters.clone(),
                };
                Self::replace_usage(
                    &mut resolved,
                    &selection,
                    UsageSelectionSource::ScenarioIntent { index },
                    &mut self.work,
                    self.limits.max_owner_bindings,
                )?;
                row.matches.push(UsageSkillMatch { target, provider });
            }
            if !row.complete {
                self.gap(
                    None,
                    Some(SchemaSubject::Definition(intent.policy.address())),
                    PlanGapReason::UnresolvedTopology,
                )?;
            }
        }
        for (index, selection) in self.request.scenario().input().usage.iter().enumerate() {
            Self::replace_usage(
                &mut resolved,
                selection,
                UsageSelectionSource::ExactScenario { index },
                &mut self.work,
                self.limits.max_owner_bindings,
            )?;
        }
        self.usage_resolution.selections = resolved.into_values().collect();
        Ok(())
    }

    fn replace_usage(
        resolved: &mut BTreeMap<(UsagePolicyDefId, UsageTarget), ResolvedUsageSelection>,
        selection: &UsagePolicySelection,
        source: UsageSelectionSource,
        work: &mut usize,
        maximum: usize,
    ) -> Result<()> {
        charge(work, selection.parameters.len() + 1)?;
        let key = (selection.policy.clone(), selection.target.clone());
        if resolved.len() >= maximum && !resolved.contains_key(&key) {
            return Err(PlanError::Limit("resolved usage selections"));
        }
        let mut replaced = vec![];
        if let Some(previous) = resolved.remove(&key) {
            replaced = previous.replaced;
            replaced.push(previous.source);
        }
        resolved.insert(
            key,
            ResolvedUsageSelection {
                selection: selection.clone(),
                source,
                replaced,
            },
        );
        Ok(())
    }

    fn usage_context(
        &mut self,
        index: usize,
        target: &UsageTarget,
    ) -> Result<(SelectorBindingStatus, Option<Context>)> {
        let (status, mut context) = match target {
            UsageTarget::Actor(ActorKey::ScenarioEnemy) => {
                return Err(PlanError::Invalid(
                    "scenario Enemy usage has no admitted role".into(),
                ));
            }
            UsageTarget::Actor(actor) => self.actor_context(actor)?,
            UsageTarget::Action(action) => {
                let resolved = self.resolver.action(action)?;
                charge(&mut self.work, resolved.work_used())?;
                let status = resolved.status();
                let context = resolved.into_value().map(|resolved| Context {
                    property_owner: None,
                    origin: RuleOrigin::Usage { index },
                    provider: Some(action.action.provider.clone()),
                    actor: Some(resolved.expected_actor().clone()),
                    skill: match resolved.provider().exposure() {
                        ProviderExposure::Skill { key, .. } => Some(key.clone()),
                        _ => None,
                    },
                    receiving_skill: None,
                    assigned_skill: None,
                    entity: ConcreteEntity::Action(action.clone()),
                });
                (status, context)
            }
            UsageTarget::Skill(target) => {
                let resolved = self.resolver.skill(target)?;
                charge(&mut self.work, resolved.work_used())?;
                let status = resolved.status();
                let Some(resolved) = resolved.into_value() else {
                    return Ok((status, None));
                };
                let (provider, skill) = match target {
                    SkillTarget::Authored(_) => (resolved.provider().key().clone(), None),
                    SkillTarget::Generated(skill) => {
                        charge(&mut self.work, 1)?;
                        // The saved skill key names its supplying parent and slot.
                        // Gates need the discovered entering grant path as well;
                        // selecting a declared slot cannot activate an occurrence.
                        let Some(provider) = self.skill_supplies.get(skill.as_ref()) else {
                            self.gap(
                                Some(skill.provider.clone()),
                                Some(SchemaSubject::Slot(SkillGrantSlotDefId::address(
                                    &skill.slot,
                                ))),
                                PlanGapReason::UnresolvedActivation,
                            )?;
                            return Ok((SelectorBindingStatus::Unresolved, None));
                        };
                        (provider.clone(), Some(skill.as_ref().clone()))
                    }
                };
                (
                    status,
                    Some(Context {
                        property_owner: None,
                        origin: RuleOrigin::Usage { index },
                        provider: Some(provider),
                        actor: Some(resolved.provider().actor().clone()),
                        skill,
                        receiving_skill: None,
                        assigned_skill: None,
                        entity: ConcreteEntity::Skill(Box::new(target.clone())),
                    }),
                )
            }
        };
        if let Some(context) = &mut context {
            context.origin = RuleOrigin::Usage { index };
        }
        Ok((status, context))
    }

    pub(super) fn usage_programs(&mut self) -> Result<()> {
        let count = self.usage_resolution.selections.len();
        charge(&mut self.work, count)?;
        for index in 0..count {
            charge(
                &mut self.work,
                self.usage_resolution.selections[index]
                    .selection
                    .parameters
                    .len()
                    + 1,
            )?;
            let selection = self.usage_resolution.selections[index].selection.clone();
            let subject = SchemaSubject::Definition(selection.policy.address());
            self.requirement_owner(subject.clone())?;
            self.check_support_domain(&subject, None)?;
            charge(&mut self.work, self.rules.input().owners.len() + 1)?;
            let Some(owner) = self
                .rules
                .input()
                .owners
                .iter()
                .find(|r| r.owner == subject)
            else {
                self.gap(None, Some(subject), PlanGapReason::MissingPrograms)?;
                continue;
            };
            if !owner.programs.is_complete() {
                self.gap(None, Some(subject.clone()), PlanGapReason::PartialPrograms)?;
            }
            let kind = match &selection.target {
                UsageTarget::Actor(_) => RuleEntityKind::Actor,
                UsageTarget::Action(_) => RuleEntityKind::Action,
                UsageTarget::Skill(_) => RuleEntityKind::Skill,
            };
            let (status, context) = self.usage_context(index, &selection.target)?;
            if context.is_none() && status != SelectorBindingStatus::Unavailable {
                self.gap(
                    None,
                    Some(subject.clone()),
                    PlanGapReason::UnresolvedTopology,
                )?;
            }
            charge(&mut self.work, owner.programs.members.len())?;
            for program in &owner.programs.members {
                // No implicit fanout or coercion between an actor, a skill and
                // its possible outputs. Each program uses the saved target kind.
                if program.context != kind {
                    self.gap(
                        None,
                        Some(subject.clone()),
                        PlanGapReason::UnsupportedContext,
                    )?;
                    continue;
                }
                charge(&mut self.work, program.effects.len())?;
                if program.effects.iter().any(|effect| {
                    !matches!(
                        effect.effect,
                        RuleEffectKind::Derive { .. }
                            | RuleEffectKind::Contribute { .. }
                            | RuleEffectKind::Capability { .. }
                            | RuleEffectKind::Requirement { .. }
                    )
                }) {
                    // A policy is not a supplying provider. Its declared child
                    // slots cannot be attached to the selected occurrence, nor
                    // can it synthesize support or modifier-transform delivery.
                    self.gap(
                        None,
                        Some(subject.clone()),
                        PlanGapReason::UnsupportedRelation,
                    )?;
                    continue;
                }
                if let Some(context) = &context {
                    self.instantiate(subject.clone(), program, context)?;
                }
            }
        }
        Ok(())
    }
}
