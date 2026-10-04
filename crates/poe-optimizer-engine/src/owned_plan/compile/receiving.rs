//! Cold finite receiving topology. No position selection, applicability or delivery executes here.
use super::*;
use poe_optimizer_core::owned_support_receiving::*;
use poe_optimizer_data::owned_support_receiving::OwnedSupportReceiving;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::owned_plan) enum BoundReceivingAvailability {
    Bound,
    Unavailable,
    Unresolved,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::owned_plan) enum BoundSupportAdmission {
    AssignedSkill {
        target: SkillTarget,
    },
    ReceivingSkill {
        target: SkillTarget,
        summoner: Option<SkillTarget>,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::owned_plan) struct BoundSupportReceiverContext {
    pub(in crate::owned_plan) receiver: SupportReceiverKey,
    pub(in crate::owned_plan) provider: ProviderKey,
    pub(in crate::owned_plan) skill: Option<SkillTarget>,
    pub(in crate::owned_plan) admission: BoundSupportAdmission,
}
#[derive(Clone, Debug)]
pub(in crate::owned_plan) struct BoundSupportReceiver {
    pub(in crate::owned_plan) context: Arc<BoundSupportReceiverContext>,
    pub(in crate::owned_plan) applicability: OwnedDefinitionKey,
    pub(in crate::owned_plan) delivery: Vec<OwnedDefinitionKey>,
    pub(in crate::owned_plan) preparation: Option<SupportPreparationPrograms>,
}
#[derive(Clone, Debug)]
pub(in crate::owned_plan) struct BoundSupportAssignment {
    pub(in crate::owned_plan) origin: SupportOrigin,
    pub(in crate::owned_plan) target: SkillTarget,
    pub(in crate::owned_plan) gem: GemDefId,
    pub(in crate::owned_plan) complete: bool,
    pub(in crate::owned_plan) availability: BoundReceivingAvailability,
    pub(in crate::owned_plan) receivers: Vec<BoundSupportReceiver>,
    pub(in crate::owned_plan) classified_programs: BTreeSet<OwnedDefinitionKey>,
}
#[derive(Clone, Debug)]
pub(in crate::owned_plan) struct BoundSupportReceiving {
    pub(in crate::owned_plan) package_identity: OwnedContentDigest,
    pub(in crate::owned_plan) assignments: BTreeMap<SupportAssignmentId, BoundSupportAssignment>,
}
impl BoundSupportReceiving {
    pub(in crate::owned_plan) fn classifies(
        &self,
        assignment: SupportAssignmentId,
        owner: &SchemaSubject,
        program: &OwnedDefinitionKey,
    ) -> bool {
        self.assignments.get(&assignment).is_some_and(|binding| {
            binding.complete
                && owner == &SchemaSubject::Definition(binding.gem.address())
                && binding.classified_programs.contains(program)
        })
    }
}

#[derive(Clone)]
struct TargetAnchor {
    owner: SupportTargetDefinition,
    provider: Option<ProviderKey>,
    availability: BoundReceivingAvailability,
    complete: bool,
}
#[derive(Clone)]
struct RoleBinding {
    receivers: Vec<Arc<BoundSupportReceiverContext>>,
    complete: bool,
}
#[derive(Default)]
struct ReceivingCounts {
    contexts: usize,
    applications: usize,
    program_references: usize,
}
#[derive(Clone, Copy)]
struct ActionEndpoint<'a> {
    provider: &'a ProviderKey,
    actor: &'a ActorKey,
    output: &'a DeclaredSlot<ActionOutputDefId>,
    skill: Option<&'a SkillTarget>,
    admission: &'a BoundSupportAdmission,
}
fn bounded_add(total: &mut usize, n: usize, maximum: usize, label: &'static str) -> Result<()> {
    *total = total
        .checked_add(n)
        .filter(|v| *v <= maximum)
        .ok_or(PlanError::Limit(label))?;
    Ok(())
}
fn target_depth(target: &SkillTarget) -> usize {
    match target {
        SkillTarget::Authored(_) => 0,
        SkillTarget::Generated(skill) => skill.provider.grant_path.len(),
    }
}
fn actor_depth(actor: &ActorKey) -> usize {
    match actor {
        ActorKey::Player => 0,
        ActorKey::Owned(actor) => actor.provider.grant_path.len(),
    }
}
fn receiver_subject(owner: &SupportTargetDefinition) -> SchemaSubject {
    SchemaSubject::Definition(match owner {
        SupportTargetDefinition::Gem(id) => id.address(),
        SupportTargetDefinition::Skill(id) => id.address(),
    })
}

impl<I: DefinitionSchemaIndex> Builder<'_, I> {
    pub(super) fn receiving(
        &mut self,
        package: &OwnedSupportReceiving,
    ) -> Result<BoundSupportReceiving> {
        if package.input().definitions != *self.index.identity()
            || package.input().namespace != *self.index.namespace()
            || Some(package.input().rules) != self.rules.source_identity()
        {
            return Err(PlanError::Invalid(
                "support receiving and compiled rules/schema bindings differ".into(),
            ));
        }
        if !self.operations.supports_actor_support_applicability() {
            return Err(PlanError::Invalid(
                "support receiving requires operation v13".into(),
            ));
        }
        let input = self.request.build().input();
        charge(
            &mut self.work,
            input.gems.len() + input.skills.len() + input.supports.len(),
        )?;
        if input.supports.len() > self.limits.max_owner_bindings {
            return Err(PlanError::Limit("support receiving assignments"));
        }
        let gems: BTreeMap<_, _> = input.gems.iter().map(|gem| (gem.id, gem)).collect();
        let skills: BTreeMap<_, _> = input.skills.iter().map(|skill| (skill.id, skill)).collect();
        let mut anchors = BTreeMap::<SkillTarget, Option<TargetAnchor>>::new();
        let mut roles = BTreeMap::<(SkillTarget, OwnedDefinitionKey), RoleBinding>::new();
        let mut result = BoundSupportReceiving {
            package_identity: *package.identity(),
            assignments: BTreeMap::new(),
        };
        let mut counts = ReceivingCounts::default();
        for assignment in &input.supports {
            charge(&mut self.work, target_depth(&assignment.target) + 2)?;
            let gem = gems.get(&assignment.support).ok_or_else(|| {
                PlanError::Invalid("support assignment has no physical Gem".into())
            })?;
            let origin_provider = root(ProviderRoot::SupportAssignment(assignment.id));
            let mut bound = BoundSupportAssignment {
                origin: SupportOrigin::Assignment(assignment.id),
                target: assignment.target.clone(),
                gem: gem.definition.clone(),
                complete: true,
                availability: BoundReceivingAvailability::Unresolved,
                receivers: vec![],
                classified_programs: BTreeSet::new(),
            };
            let Some(support) = package.support_for(&gem.definition) else {
                self.gap(
                    Some(origin_provider),
                    Some(SchemaSubject::Definition(gem.definition.address())),
                    PlanGapReason::PartialReceivers,
                )?;
                bound.complete = false;
                result.assignments.insert(assignment.id, bound);
                continue;
            };
            if !support.receivers.is_complete() {
                self.gap(
                    Some(origin_provider.clone()),
                    Some(SchemaSubject::Definition(gem.definition.address())),
                    PlanGapReason::PartialReceivers,
                )?;
                bound.complete = false;
            }
            if !anchors.contains_key(&assignment.target) {
                if anchors.len() >= self.limits.max_providers {
                    return Err(PlanError::Limit("support receiving targets"));
                }
                let anchor = self.receiving_anchor(&assignment.target, &gems, &skills)?;
                charge(&mut self.work, target_depth(&assignment.target) + 1)?;
                anchors.insert(assignment.target.clone(), anchor);
            }
            let Some(anchor) = anchors
                .get(&assignment.target)
                .expect("inserted receiving target")
                .as_ref()
            else {
                bound.complete = false;
                self.gap(
                    Some(origin_provider),
                    Some(SchemaSubject::Definition(gem.definition.address())),
                    PlanGapReason::PartialReceivers,
                )?;
                result.assignments.insert(assignment.id, bound);
                continue;
            };
            bound.availability = anchor.availability;
            bound.complete &= anchor.complete;
            let mut receivers = BTreeSet::new();
            charge(&mut self.work, support.receivers.members.len())?;
            for row in &support.receivers.members {
                let preparation_count = row
                    .preparation
                    .as_ref()
                    .map_or(0, |p| p.properties.len() + 1);
                bounded_add(
                    &mut counts.program_references,
                    row.delivery.len() + preparation_count + 1,
                    self.limits.max_edges,
                    "support receiving program references",
                )?;
                charge(&mut self.work, row.delivery.len() + preparation_count + 2)?;
                bound.classified_programs.insert(row.applicability.clone());
                bound
                    .classified_programs
                    .extend(row.delivery.iter().cloned());
                if let Some(preparation) = &row.preparation {
                    bound
                        .classified_programs
                        .insert(preparation.applicability.clone());
                    bound
                        .classified_programs
                        .extend(preparation.properties.iter().cloned());
                }
                charge(&mut self.work, target_depth(&assignment.target) + 1)?;
                let key = (assignment.target.clone(), row.role.clone());
                if !roles.contains_key(&key) {
                    if roles.len() >= self.limits.max_providers {
                        return Err(PlanError::Limit("support receiving roles"));
                    }
                    let resolved = self.receiving_role(
                        package,
                        &assignment.target,
                        anchor,
                        &row.role,
                        &mut counts,
                    )?;
                    roles.insert(key.clone(), resolved);
                }
                let role = roles.get(&key).expect("inserted receiving role");
                bound.complete &= role.complete;
                bounded_add(
                    &mut counts.applications,
                    role.receivers.len(),
                    self.limits.max_owner_bindings,
                    "support receiving application templates",
                )?;
                for context in &role.receivers {
                    bounded_add(
                        &mut counts.program_references,
                        row.delivery.len() + preparation_count + 1,
                        self.limits.max_edges,
                        "support receiving program references",
                    )?;
                    charge(
                        &mut self.work,
                        context.provider.grant_path.len()
                            + row.delivery.len()
                            + preparation_count
                            + 3,
                    )?;
                    if !receivers.insert(context.receiver.clone()) {
                        return Err(PlanError::Invalid("support receiving roles resolve to competing applicability producers for one exact receiver".into()));
                    }
                    bound.receivers.push(BoundSupportReceiver {
                        context: Arc::clone(context),
                        applicability: row.applicability.clone(),
                        delivery: row.delivery.clone(),
                        preparation: row.preparation.clone(),
                    });
                }
            }
            if !bound.complete {
                self.gap(
                    Some(origin_provider),
                    Some(SchemaSubject::Definition(gem.definition.address())),
                    PlanGapReason::PartialReceivers,
                )?;
            }
            result.assignments.insert(assignment.id, bound);
        }
        Ok(result)
    }

    fn receiving_anchor(
        &mut self,
        target: &SkillTarget,
        gems: &BTreeMap<GemInstanceId, &GemInstance>,
        skills: &BTreeMap<SkillUseId, &SkillUse>,
    ) -> Result<Option<TargetAnchor>> {
        charge(&mut self.work, target_depth(target) + 2)?;
        let resolved = self.resolver.skill(target)?;
        charge(&mut self.work, resolved.work_used() + 1)?;
        if resolved.schema() == SchemaBindingStatus::Invalid {
            return Err(PlanError::Invalid(
                "support receiving target has invalid schema bindings".into(),
            ));
        }
        let (owner, provider) = match target {
            SkillTarget::Authored(id) => {
                let skill = skills.get(id).ok_or_else(|| {
                    PlanError::Invalid("support target has no authored skill".into())
                })?;
                let owner = match &skill.source {
                    AuthoredSkillSource::Direct(skill) => {
                        SupportTargetDefinition::Skill(skill.clone())
                    }
                    AuthoredSkillSource::Gem(gem) => SupportTargetDefinition::Gem(
                        gems.get(gem)
                            .ok_or_else(|| {
                                PlanError::Invalid(
                                    "authored support target has no physical Gem".into(),
                                )
                            })?
                            .definition
                            .clone(),
                    ),
                };
                (owner, Some(root(ProviderRoot::SkillUse(*id))))
            }
            SkillTarget::Generated(skill) => {
                let SchemaLookup::Known(schema) = self.index.slot(&skill.slot) else {
                    self.gap(
                        Some(skill.provider.clone()),
                        Some(SchemaSubject::Slot(SkillGrantSlotDefId::address(
                            &skill.slot,
                        ))),
                        PlanGapReason::SchemaUnresolved,
                    )?;
                    return Ok(None);
                };
                (
                    SupportTargetDefinition::Skill(schema.skill.clone()),
                    self.skill_supplies.get(skill.as_ref()).cloned(),
                )
            }
        };
        let mut complete = resolved.schema() == SchemaBindingStatus::Valid;
        let availability = if resolved.status() == SelectorBindingStatus::Unavailable {
            BoundReceivingAvailability::Unavailable
        } else if resolved.value().is_some()
            && provider
                .as_ref()
                .is_some_and(|provider| self.providers.contains(provider))
        {
            BoundReceivingAvailability::Bound
        } else {
            complete = false;
            BoundReceivingAvailability::Unresolved
        };
        if !complete {
            self.gap(
                provider.clone(),
                Some(receiver_subject(&owner)),
                PlanGapReason::UnresolvedTopology,
            )?;
        }
        Ok(Some(TargetAnchor {
            owner,
            provider,
            availability,
            complete,
        }))
    }

    fn receiving_role(
        &mut self,
        package: &OwnedSupportReceiving,
        target: &SkillTarget,
        anchor: &TargetAnchor,
        role: &OwnedDefinitionKey,
        counts: &mut ReceivingCounts,
    ) -> Result<RoleBinding> {
        let mut result = RoleBinding {
            receivers: vec![],
            complete: true,
        };
        let Some(target_rows) = package.target_for(&anchor.owner) else {
            self.gap(
                anchor.provider.clone(),
                Some(receiver_subject(&anchor.owner)),
                PlanGapReason::PartialReceivers,
            )?;
            result.complete = false;
            return Ok(result);
        };
        if !target_rows.roles.is_complete() {
            self.gap(
                anchor.provider.clone(),
                Some(receiver_subject(&anchor.owner)),
                PlanGapReason::PartialReceivers,
            )?;
            result.complete = false;
        }
        charge(&mut self.work, target_rows.roles.members.len() + 1)?;
        let Some(row) = target_rows.roles.members.iter().find(|r| &r.role == role) else {
            // Only a complete local inventory establishes that this role has no receiver.
            return Ok(result);
        };
        if !row.endpoints.is_complete() {
            self.gap(
                anchor.provider.clone(),
                Some(receiver_subject(&anchor.owner)),
                PlanGapReason::PartialReceivers,
            )?;
            result.complete = false;
        }
        if package.role(role).is_none() {
            return Err(PlanError::Invalid("unknown support receiving role".into()));
        }
        if anchor.availability != BoundReceivingAvailability::Bound {
            return Ok(result);
        }
        let anchor_provider = anchor.provider.as_ref().ok_or_else(|| {
            PlanError::Invalid("bound support target has no discovered entering provider".into())
        })?;
        let mut seen = BTreeSet::new();
        charge(&mut self.work, row.endpoints.members.len())?;
        for endpoint in &row.endpoints.members {
            let provider = self.receiving_path(anchor_provider, endpoint.path())?;
            if !self.providers.contains(&provider) {
                self.gap(
                    Some(provider),
                    Some(receiver_subject(&anchor.owner)),
                    PlanGapReason::UnresolvedTopology,
                )?;
                result.complete = false;
                continue;
            }
            let resolved = self.resolver.provider(&provider)?;
            charge(&mut self.work, resolved.work_used() + 1)?;
            if resolved.schema() == SchemaBindingStatus::Invalid {
                return Err(PlanError::Invalid(
                    "receiving provider has invalid schema bindings".into(),
                ));
            }
            if resolved.schema() != SchemaBindingStatus::Valid {
                result.complete = false;
                self.gap(
                    Some(provider.clone()),
                    None,
                    PlanGapReason::UnresolvedTopology,
                )?;
            }
            let Some(resolved) = resolved.into_value() else {
                result.complete = false;
                self.gap(Some(provider), None, PlanGapReason::UnresolvedTopology)?;
                continue;
            };
            let skill = self.receiving_skill(target, &provider, resolved.exposure())?;
            let Some(admission) = self.receiving_admission(
                target,
                anchor_provider,
                &provider,
                skill.as_ref(),
                endpoint.admission(),
                &mut result.complete,
            )?
            else {
                continue;
            };
            match endpoint {
                SupportReceiverEndpoint::Actor { .. } => {
                    let actor = resolved.actor().clone();
                    if !self.receiving_actor_discovered(&actor)? {
                        self.gap(Some(provider), None, PlanGapReason::UnresolvedTopology)?;
                        result.complete = false;
                        continue;
                    }
                    self.push_receiving_context(
                        BoundSupportReceiverContext {
                            receiver: SupportReceiverKey::Actor(actor),
                            provider,
                            skill: None,
                            admission,
                        },
                        &mut result.receivers,
                        &mut seen,
                        counts,
                    )?;
                }
                SupportReceiverEndpoint::Action {
                    output, selection, ..
                } => {
                    let SchemaLookup::Known(schema) = self.index.slot(output) else {
                        self.gap(
                            Some(provider),
                            Some(SchemaSubject::Slot(ActionOutputDefId::address(output))),
                            PlanGapReason::SchemaUnresolved,
                        )?;
                        result.complete = false;
                        continue;
                    };
                    let actor = match &schema.actor_role {
                        DeclaredActorRole::Player => ActorKey::Player,
                        DeclaredActorRole::ProviderActor => resolved.actor().clone(),
                        DeclaredActorRole::OwnedSlot(slot) => {
                            ActorKey::Owned(Box::new(OwnedActorKey {
                                provider: provider.clone(),
                                slot: slot.clone(),
                            }))
                        }
                    };
                    if !self.receiving_actor_discovered(&actor)? {
                        self.gap(Some(provider), None, PlanGapReason::UnresolvedTopology)?;
                        result.complete = false;
                        continue;
                    }
                    if !schema.parts.is_complete()
                        || !schema.modes.is_complete()
                        || !schema.stat_sets.is_complete()
                    {
                        self.gap(
                            Some(provider.clone()),
                            Some(SchemaSubject::Slot(ActionOutputDefId::address(output))),
                            PlanGapReason::PartialDeclarations,
                        )?;
                        result.complete = false;
                        if matches!(selection, SupportActionSelection::AllDeclared) {
                            continue;
                        }
                    }
                    let action_context = ActionEndpoint {
                        provider: &provider,
                        actor: &actor,
                        output,
                        skill: skill.as_ref(),
                        admission: &admission,
                    };
                    match selection {
                        SupportActionSelection::Exact(variant) => {
                            self.receiving_action(
                                action_context,
                                variant,
                                &mut result,
                                &mut seen,
                                counts,
                            )?;
                        }
                        SupportActionSelection::AllDeclared => {
                            let n = schema
                                .parts
                                .members
                                .len()
                                .checked_mul(schema.modes.members.len())
                                .and_then(|n| n.checked_mul(schema.stat_sets.members.len()))
                                .ok_or(PlanError::Limit("receiving action expansion"))?;
                            if n > self
                                .limits
                                .max_owner_bindings
                                .saturating_sub(counts.contexts)
                            {
                                return Err(PlanError::Limit("receiving action expansion"));
                            }
                            charge(
                                &mut self.work,
                                schema.parts.members.len()
                                    + schema.modes.members.len()
                                    + schema.stat_sets.members.len()
                                    + n,
                            )?;
                            if n == 0 {
                                continue;
                            }
                            for part in &schema.parts.members {
                                for mode in &schema.modes.members {
                                    for stat_set in &schema.stat_sets.members {
                                        let variant = SupportActionVariant {
                                            part: part.clone(),
                                            mode: mode.clone(),
                                            stat_set: stat_set.clone(),
                                        };
                                        self.receiving_action(
                                            action_context,
                                            &variant,
                                            &mut result,
                                            &mut seen,
                                            counts,
                                        )?;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(result)
    }

    pub(super) fn receiving_path(
        &mut self,
        anchor: &ProviderKey,
        path: &[DeclaredSlot<GrantSlotDefId>],
    ) -> Result<ProviderKey> {
        let depth = anchor
            .grant_path
            .len()
            .checked_add(path.len())
            .ok_or(PlanError::Limit("receiving path depth"))?;
        if depth > self.limits.binding.input.max_provider_steps {
            return Err(PlanError::Limit("receiving path depth"));
        }
        charge(&mut self.work, depth + 1)?;
        let mut provider = anchor.clone();
        provider.grant_path.extend_from_slice(path);
        Ok(provider)
    }
    fn receiving_skill(
        &mut self,
        assigned: &SkillTarget,
        provider: &ProviderKey,
        exposure: &ProviderExposure<'_>,
    ) -> Result<Option<SkillTarget>> {
        charge(&mut self.work, provider.grant_path.len() + 1)?;
        match exposure {
            ProviderExposure::Skill { key, .. } => {
                if self.skill_supplies.get(key) != Some(provider) {
                    return Err(PlanError::Invalid(
                        "receiver skill does not match its discovered entering supply".into(),
                    ));
                }
                Ok(Some(SkillTarget::Generated(Box::new(key.clone()))))
            }
            ProviderExposure::Root { .. } if provider.grant_path.is_empty() => {
                if let (ProviderRoot::SkillUse(id), SkillTarget::Authored(target)) =
                    (&provider.root, assigned)
                    && id == target
                {
                    // This remains the physical Gem's authored target when applicable;
                    // no potential Skill definition becomes a supplied occurrence.
                    Ok(Some(assigned.clone()))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }
    pub(super) fn receiving_admission(
        &mut self,
        assigned: &SkillTarget,
        anchor: &ProviderKey,
        provider: &ProviderKey,
        skill: Option<&SkillTarget>,
        admission: &SupportAdmissionContext,
        complete: &mut bool,
    ) -> Result<Option<BoundSupportAdmission>> {
        charge(
            &mut self.work,
            target_depth(assigned) + skill.map_or(0, target_depth) + 1,
        )?;
        Ok(Some(match admission {
            SupportAdmissionContext::AssignedSkill => BoundSupportAdmission::AssignedSkill {
                target: assigned.clone(),
            },
            SupportAdmissionContext::ReceivingSkill { summoner_path } => {
                let Some(SkillTarget::Generated(child)) = skill else {
                    return Err(PlanError::Invalid(
                        "receiving admission has no exact generated SkillTarget".into(),
                    ));
                };
                if self.skill_supplies.get(child.as_ref()) != Some(provider) {
                    return Err(PlanError::Invalid(
                        "receiving admission has no discovered entering supply".into(),
                    ));
                }
                let summoner = match summoner_path {
                    None => None,
                    Some(path) if path.is_empty() => Some(assigned.clone()),
                    Some(path) => {
                        let source = self.receiving_path(anchor, path)?;
                        if !self.providers.contains(&source) {
                            self.gap(Some(source), None, PlanGapReason::UnresolvedTopology)?;
                            *complete = false;
                            return Ok(None);
                        }
                        let resolved = self.resolver.provider(&source)?;
                        charge(&mut self.work, resolved.work_used() + 1)?;
                        if resolved.schema() == SchemaBindingStatus::Invalid {
                            return Err(PlanError::Invalid(
                                "summoner has invalid schema bindings".into(),
                            ));
                        }
                        if resolved.schema() != SchemaBindingStatus::Valid {
                            *complete = false;
                            self.gap(
                                Some(source.clone()),
                                None,
                                PlanGapReason::UnresolvedTopology,
                            )?;
                        }
                        let Some(resolved) = resolved.into_value() else {
                            *complete = false;
                            self.gap(Some(source), None, PlanGapReason::UnresolvedTopology)?;
                            return Ok(None);
                        };
                        let Some(target @ SkillTarget::Generated(_)) =
                            self.receiving_skill(assigned, &source, resolved.exposure())?
                        else {
                            return Err(PlanError::Invalid(
                                "summoner path has no exact generated SkillTarget".into(),
                            ));
                        };
                        Some(target)
                    }
                };
                BoundSupportAdmission::ReceivingSkill {
                    target: SkillTarget::Generated(child.clone()),
                    summoner,
                }
            }
        }))
    }
    fn receiving_actor_discovered(&mut self, actor: &ActorKey) -> Result<bool> {
        charge(&mut self.work, actor_depth(actor) + 1)?;
        if !self.receiver_actors.contains(actor) {
            return Ok(false);
        }
        let ActorKey::Owned(actor) = actor else {
            return Ok(true);
        };
        if let Some(entered) = self.actor_supplies.get(actor.as_ref()) {
            return Ok(self.providers.contains(entered));
        }
        // Legacy output-only actors have no Actor definition/owned supply entry.
        // Their exact entering grant was still discovered in the actor inventory.
        Ok(
            matches!(self.index.slot(&actor.slot), SchemaLookup::Known(schema) if schema.provider_definition.is_none())
                && self.actors.contains_key(actor.as_ref()),
        )
    }
    fn receiving_action(
        &mut self,
        context: ActionEndpoint<'_>,
        variant: &SupportActionVariant,
        result: &mut RoleBinding,
        seen: &mut BTreeSet<SupportReceiverKey>,
        counts: &mut ReceivingCounts,
    ) -> Result<()> {
        let ActionEndpoint {
            provider,
            actor,
            output,
            skill,
            admission,
        } = context;
        charge(
            &mut self.work,
            provider.grant_path.len() + actor_depth(actor) + skill.map_or(0, target_depth) + 5,
        )?;
        let action = ActionSelection {
            action: ActionKey {
                actor: actor.clone(),
                provider: provider.clone(),
                output: output.clone(),
            },
            part: variant.part.clone(),
            mode: variant.mode.clone(),
            stat_set: variant.stat_set.clone(),
        };
        let resolved = self.resolver.action(&action)?;
        charge(&mut self.work, resolved.work_used() + 1)?;
        if resolved.schema() == SchemaBindingStatus::Invalid {
            return Err(PlanError::Invalid(
                "support receiving action has invalid schema bindings".into(),
            ));
        }
        if resolved.schema() != SchemaBindingStatus::Valid {
            result.complete = false;
            self.gap(
                Some(provider.clone()),
                Some(SchemaSubject::Slot(ActionOutputDefId::address(output))),
                PlanGapReason::UnresolvedTopology,
            )?;
        }
        if resolved.value().is_none() {
            result.complete = false;
            self.gap(
                Some(provider.clone()),
                Some(SchemaSubject::Slot(ActionOutputDefId::address(output))),
                PlanGapReason::UnresolvedTopology,
            )?;
            return Ok(());
        }
        if !self.actions.contains(&action) && self.actions.len() >= self.limits.max_providers {
            return Err(PlanError::Limit("receiving actions"));
        }
        self.actions.insert(action.clone());
        self.push_receiving_context(
            BoundSupportReceiverContext {
                receiver: SupportReceiverKey::Action(Box::new(action)),
                provider: provider.clone(),
                skill: skill.cloned(),
                admission: admission.clone(),
            },
            &mut result.receivers,
            seen,
            counts,
        )
    }
    fn push_receiving_context(
        &mut self,
        context: BoundSupportReceiverContext,
        out: &mut Vec<Arc<BoundSupportReceiverContext>>,
        seen: &mut BTreeSet<SupportReceiverKey>,
        counts: &mut ReceivingCounts,
    ) -> Result<()> {
        bounded_add(
            &mut counts.contexts,
            1,
            self.limits.max_owner_bindings,
            "support receiving contexts",
        )?;
        charge(
            &mut self.work,
            context.provider.grant_path.len() + context.skill.as_ref().map_or(0, target_depth) + 1,
        )?;
        if !seen.insert(context.receiver.clone()) {
            return Err(PlanError::Invalid(
                "receiving endpoint paths alias the same exact receiver".into(),
            ));
        }
        out.push(Arc::new(context));
        Ok(())
    }
}

#[cfg(test)]
#[path = "receiving_tests.rs"]
mod tests;
