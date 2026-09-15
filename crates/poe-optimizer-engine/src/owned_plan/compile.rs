use super::*;
use poe_optimizer_core::owned_routing::*;
mod reads;

#[derive(Clone, Debug)]
enum PendingRead {
    Ready(ReadBinding),
    Required(Box<PendingRead>),
    Value(PlanValueKey),
    Contributions(ContributionKey, ContributionReduction, ParameterValue),
}
#[derive(Clone)]
struct Context {
    origin: RuleOrigin,
    provider: Option<ProviderKey>,
    actor: ActorKey,
    skill: Option<GeneratedSkillKey>,
    entity: ConcreteEntity,
}
struct Builder<'a, I> {
    request: &'a OwnedEvaluationRequest,
    index: &'a I,
    rules: &'a CompiledRulePackage,
    resolver: OwnedOccurrenceResolver<'a, I>,
    limits: PlanLimits,
    work: usize,
    gaps: Vec<PlanGap>,
    invocations: Vec<Invocation>,
    pending: Vec<Vec<PendingRead>>,
    effects: Vec<EffectNode>,
    gates: Vec<Vec<PendingRead>>,
    values: BTreeMap<PlanValueKey, usize>,
    contributions: BTreeMap<ContributionKey, Vec<usize>>,
    actions: BTreeSet<ActionSelection>,
    providers: BTreeSet<ProviderKey>,
    actors: BTreeMap<OwnedActorKey, Vec<PlanValueKey>>,
    unsupported_roots: BTreeSet<ProviderRoot>,
    potential_skills: BTreeSet<(ProviderKey, SkillDefId)>,
    skill_supplies: BTreeMap<GeneratedSkillKey, ProviderKey>,
    supplied_skills: BTreeMap<(ProviderKey, SkillDefId), Vec<PlanValueKey>>,
    binding_edges: usize,
}
fn owner_subject(owner: &SlotOwnerDefId) -> SchemaSubject {
    SchemaSubject::Definition(match owner {
        SlotOwnerDefId::Class(id) => id.address(),
        SlotOwnerDefId::Ascendancy(id) => id.address(),
        SlotOwnerDefId::Reward(id) => id.address(),
        SlotOwnerDefId::ItemTemplate(id) => id.address(),
        SlotOwnerDefId::Modifier(id) => id.address(),
        SlotOwnerDefId::Gem(id) => id.address(),
        SlotOwnerDefId::Skill(id) => id.address(),
        SlotOwnerDefId::PassiveNode(id) => id.address(),
        SlotOwnerDefId::UsagePolicy(id) => id.address(),
    })
}
fn root(root: ProviderRoot) -> ProviderKey {
    ProviderKey {
        root,
        grant_path: vec![],
    }
}
fn entity(relative: RuleEntity, context: &Context) -> ConcreteEntity {
    match relative {
        RuleEntity::Current => context.entity.clone(),
        RuleEntity::Actor => ConcreteEntity::Actor(context.actor.clone()),
        RuleEntity::Player => ConcreteEntity::Actor(ActorKey::Player),
        RuleEntity::Enemy => ConcreteEntity::Enemy,
        RuleEntity::Environment => ConcreteEntity::Environment,
    }
}
fn missing(reason: PlanGapReason) -> PendingRead {
    PendingRead::Ready(ReadBinding::Missing(reason))
}

pub(super) fn compile<I: DefinitionSchemaIndex>(
    request: Arc<OwnedEvaluationRequest>,
    definitions: Arc<I>,
    rules: Arc<CompiledRulePackage>,
    routing: Arc<OwnedActionRouting>,
    limits: PlanLimits,
) -> Result<OwnedEffectPlan<I>> {
    limits.validate()?;
    if rules.input().definitions != *definitions.identity()
        || rules.input().namespace != *definitions.namespace()
    {
        return Err(PlanError::Invalid(
            "rule and definition bindings differ".into(),
        ));
    }
    routing
        .verify_bindings(definitions.as_ref())
        .map_err(|e| PlanError::Invalid(e.to_string()))?;
    let report = bind_owned_request(definitions.as_ref(), &request, limits.binding)?;
    if report.schema() == SchemaBindingStatus::Invalid {
        return Err(PlanError::Invalid(
            "owned request has invalid schema bindings".into(),
        ));
    }
    let bindings = PlanIdentity {
        request: report.request_digest(),
        definitions: definitions.identity().clone(),
        rules: rules.identity(),
        routing: *routing.identity(),
    };
    let identity = digest_owned("owned-effect-plan-v2", &bindings, limits.max_wire_bytes)?;
    let resolver = OwnedOccurrenceResolver::new(definitions.as_ref(), &request, limits.binding)?;
    let mut b = Builder {
        request: &request,
        index: definitions.as_ref(),
        rules: &rules,
        resolver,
        limits,
        work: limits.max_work,
        gaps: vec![],
        invocations: vec![],
        pending: vec![],
        effects: vec![],
        gates: vec![],
        values: BTreeMap::new(),
        contributions: BTreeMap::new(),
        actions: BTreeSet::new(),
        providers: BTreeSet::new(),
        actors: BTreeMap::new(),
        unsupported_roots: BTreeSet::new(),
        potential_skills: BTreeSet::new(),
        skill_supplies: BTreeMap::new(),
        supplied_skills: BTreeMap::new(),
        binding_edges: 0,
    };
    // Whole-request binding is independently bounded; reserve its entire allowance.
    charge(&mut b.work, limits.binding.max_work)?;
    if report.schema() == SchemaBindingStatus::Unresolved {
        b.gap(None, None, PlanGapReason::SchemaUnresolved)?;
    }
    for q in &request.queries().input().requests {
        if let MetricTarget::Action(action) = &q.target {
            b.actions.insert(action.as_ref().clone());
        }
    }
    for u in &request.scenario().input().usage {
        if let UsageTarget::Action(action) = &u.target {
            b.actions.insert(action.as_ref().clone());
        }
    }
    for c in &request.build().input().choices {
        if let ChoiceOwner::Action(action) = &c.owner {
            b.actions.insert(action.as_ref().clone());
        }
    }
    if b.actions.len() > limits.max_providers {
        return Err(PlanError::Limit("actions"));
    }
    b.discover()?;
    b.encounter_and_usage()?;
    b.action_programs()?;
    b.routes(&routing)?;
    let complete = b.gaps.is_empty();
    for (inv, reads) in b.invocations.iter_mut().zip(b.pending) {
        inv.reads = reads
            .into_iter()
            .map(|r| {
                resolve(
                    r,
                    &b.values,
                    &b.contributions,
                    complete,
                    &mut b.work,
                    &mut b.binding_edges,
                    limits.max_edges,
                )
            })
            .collect::<Result<Vec<_>>>()?;
    }
    for (node, gates) in b.effects.iter_mut().zip(b.gates) {
        node.gates = gates
            .into_iter()
            .map(|r| {
                resolve(
                    r,
                    &b.values,
                    &b.contributions,
                    complete,
                    &mut b.work,
                    &mut b.binding_edges,
                    limits.max_edges,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        if let EffectOperation::Route {
            source: ReadBinding::Final { complete: c, .. },
        } = &mut node.operation
        {
            *c = complete;
        }
    }
    let order = dependency_order(&mut b.effects, &b.invocations, limits, &mut b.work)?;
    Ok(OwnedEffectPlan {
        request: Arc::clone(&request),
        definitions: Arc::clone(&definitions),
        rules: Arc::clone(&rules),
        routing: Arc::clone(&routing),
        identity,
        bindings,
        limits,
        gaps: b.gaps,
        complete,
        invocations: b.invocations,
        effects: b.effects,
        order,
        values: b.values,
    })
}
fn resolve(
    read: PendingRead,
    values: &BTreeMap<PlanValueKey, usize>,
    contributions: &BTreeMap<ContributionKey, Vec<usize>>,
    complete: bool,
    work: &mut usize,
    edges: &mut usize,
    max_edges: usize,
) -> Result<ReadBinding> {
    let expansion = match &read {
        PendingRead::Ready(_) => 0,
        PendingRead::Required(_) => 1,
        PendingRead::Value(_) => 1,
        PendingRead::Contributions(key, ..) => contributions.get(key).map_or(0, Vec::len),
    };
    charge(work, expansion + 1)?;
    *edges = edges
        .checked_add(expansion)
        .ok_or(PlanError::Limit("binding edges"))?;
    if *edges > max_edges {
        return Err(PlanError::Limit("binding edges"));
    }
    Ok(match read {
        PendingRead::Ready(v) => v,
        PendingRead::Required(source) => ReadBinding::Present {
            source: Box::new(resolve(
                *source,
                values,
                contributions,
                complete,
                work,
                edges,
                max_edges,
            )?),
        },
        PendingRead::Value(key) => ReadBinding::Final {
            effect: values.get(&key).copied(),
            complete,
        },
        PendingRead::Contributions(key, reduction, empty) => ReadBinding::Reduction {
            effects: contributions.get(&key).cloned().unwrap_or_default(),
            reduction,
            empty,
            complete,
        },
    })
}
impl<'a, I: DefinitionSchemaIndex> Builder<'a, I> {
    fn gap(
        &mut self,
        provider: Option<ProviderKey>,
        subject: Option<SchemaSubject>,
        reason: PlanGapReason,
    ) -> Result<()> {
        charge(&mut self.work, self.gaps.len() + 1)?;
        let gap = PlanGap {
            provider,
            subject,
            reason,
        };
        if !self.gaps.contains(&gap) {
            if self.gaps.len() >= self.limits.binding.max_issues {
                return Err(PlanError::Limit("gaps"));
            }
            self.gaps.push(gap);
        }
        Ok(())
    }
    fn discover(&mut self) -> Result<()> {
        let input = self.request.build().input();
        let mut pending = BTreeSet::new();
        self.unsupported_roots.extend(
            input
                .supports
                .iter()
                .map(|s| ProviderRoot::SupportAssignment(s.id)),
        );
        self.unsupported_roots.extend(
            input
                .allocations
                .iter()
                .filter(|a| matches!(a.access, AllocationAccess::Granted(_)))
                .map(|a| ProviderRoot::Allocation(a.id)),
        );
        pending.insert(root(ProviderRoot::Character));
        for e in &input.equipment {
            pending.insert(root(ProviderRoot::EquipmentUse(e.id)));
            charge(&mut self.work, input.items.len() + 1)?;
            if let Some(item) = input.items.iter().find(|item| item.id == e.item) {
                charge(&mut self.work, item.modifiers.len())?;
                for m in &item.modifiers {
                    pending.insert(root(ProviderRoot::ItemModifier {
                        equipment_use: e.id,
                        modifier: m.id,
                    }));
                }
            }
            if pending.len() > self.limits.max_providers {
                return Err(PlanError::Limit("providers"));
            }
        }
        pending.extend(
            input
                .skills
                .iter()
                .map(|s| root(ProviderRoot::SkillUse(s.id))),
        );
        pending.extend(
            input
                .supports
                .iter()
                .map(|s| root(ProviderRoot::SupportAssignment(s.id))),
        );
        pending.extend(
            input
                .allocations
                .iter()
                .map(|s| root(ProviderRoot::Allocation(s.id))),
        );
        pending.extend(
            input
                .character
                .rewards
                .iter()
                .map(|s| root(ProviderRoot::Reward(s.id))),
        );
        if pending.len() > self.limits.max_providers {
            return Err(PlanError::Limit("providers"));
        }
        while let Some(key) = pending.pop_first() {
            charge(&mut self.work, 1)?;
            if self.providers.contains(&key) {
                continue;
            }
            if self.providers.len() >= self.limits.max_providers {
                return Err(PlanError::Limit("providers"));
            }
            let resolution = self.resolver.provider(&key)?;
            charge(&mut self.work, resolution.work_used())?;
            if resolution.status() == SelectorBindingStatus::Unavailable {
                continue;
            }
            let Some(provider) = resolution.into_value() else {
                self.gap(Some(key), None, PlanGapReason::UnresolvedTopology)?;
                continue;
            };
            self.providers.insert(key.clone());
            if matches!(key.root, ProviderRoot::SupportAssignment(_)) {
                self.unsupported_roots.insert(key.root.clone());
                self.gap(Some(key), None, PlanGapReason::UnsupportedRelation)?;
                continue;
            }
            if let ProviderRoot::Allocation(id) = key.root {
                charge(&mut self.work, input.allocations.len())?;
                if input
                    .allocations
                    .iter()
                    .any(|v| v.id == id && matches!(v.access, AllocationAccess::Granted(_)))
                {
                    self.unsupported_roots.insert(key.root.clone());
                    self.gap(Some(key), None, PlanGapReason::UnsupportedRelation)?;
                    continue;
                }
            }
            match provider.exposure() {
                ProviderExposure::Root { owners, skills } => {
                    // Register potential addresses before instantiating Gem-owned action
                    // programs, so their gates do not depend on owner visitation order.
                    if let Some(skills) = skills {
                        if !skills.is_complete() {
                            self.gap(Some(key.clone()), None, PlanGapReason::PartialDeclarations)?;
                        }
                        for skill in &skills.members {
                            charge(&mut self.work, 1)?;
                            // Membership permits symbolic binding, but only an explicit
                            // supplied occurrence and its activation producer account for it.
                            self.potential_skills.insert((key.clone(), skill.clone()));
                        }
                    }
                    for owner in owners {
                        self.declarations(&key, owner.declarations(), &mut pending)?;
                        self.owner(owner.subject(), &key, provider.actor(), None)?;
                    }
                }
                ProviderExposure::Skill {
                    key: skill,
                    owner,
                    declarations,
                    ..
                } => {
                    // A grant path is a traversal address. Two paths entering the same
                    // parent/skill slot identify one occurrence, not two copies. Alternative
                    // grant combination needs explicit semantics; do not choose a path.
                    charge(&mut self.work, key.grant_path.len() + 1)?;
                    if let Some(previous) = self.skill_supplies.insert(skill.clone(), key.clone())
                        && previous != key
                    {
                        return Err(PlanError::Invalid(format!(
                            "ambiguous skill supply for {skill:?}: {previous:?} and {key:?}"
                        )));
                    }
                    let SlotOwnerDefId::Skill(definition) = owner else {
                        return Err(PlanError::Invalid(
                            "skill occurrence has no skill owner".into(),
                        ));
                    };
                    let grant = key.grant_path.last().ok_or_else(|| {
                        PlanError::Invalid("supplied skill has no entering grant".into())
                    })?;
                    self.supplied_skills
                        .entry((skill.provider.clone(), definition.clone()))
                        .or_default()
                        .push(PlanValueKey::Grant {
                            provider: skill.provider.clone(),
                            slot: grant.clone(),
                        });
                    self.declarations(&key, declarations, &mut pending)?;
                    self.owner(owner_subject(owner), &key, provider.actor(), Some(skill))?;
                }
                ProviderExposure::Actor {
                    key: actor, schema, ..
                } => {
                    if !schema.skills.is_complete() || !schema.outputs.is_complete() {
                        self.gap(Some(key.clone()), None, PlanGapReason::PartialDeclarations)?;
                    }
                    for skill in &schema.skills.members {
                        charge(&mut self.work, 1)?;
                        self.potential_skills.insert((key.clone(), skill.clone()));
                        self.gap(
                            Some(key.clone()),
                            Some(SchemaSubject::Definition(skill.address())),
                            PlanGapReason::UnresolvedActivation,
                        )?;
                    }
                    self.owner(
                        SchemaSubject::Slot(ActorSlotDefId::address(&actor.slot)),
                        &key,
                        provider.actor(),
                        None,
                    )?;
                }
                ProviderExposure::AllocationAccess { .. } => {
                    self.gap(Some(key), None, PlanGapReason::UnsupportedRelation)?;
                }
            }
            if pending.len() + self.providers.len() > self.limits.max_providers {
                return Err(PlanError::Limit("providers"));
            }
        }
        self.check_skill_supply_coverage()?;
        if !input.payload_links.is_empty() {
            self.gap(None, None, PlanGapReason::UnsupportedRelation)?;
        }
        Ok(())
    }
    fn check_skill_supply_coverage(&mut self) -> Result<()> {
        // Wait until every provider has instantiated its rules. Looking at source
        // program declarations alone would accept an unavailable/wrong-context producer.
        charge(&mut self.work, self.potential_skills.len())?;
        let potentials = std::mem::take(&mut self.potential_skills);
        for (provider, skill) in &potentials {
            charge(&mut self.work, provider.grant_path.len() + 1)?;
            let supplies = self.supplied_skills.get(&(provider.clone(), skill.clone()));
            charge(&mut self.work, supplies.map_or(0, Vec::len) + 1)?;
            let covered = supplies.is_some_and(|grants| {
                !grants.is_empty() && grants.iter().all(|grant| self.values.contains_key(grant))
            });
            if !covered {
                self.gap(
                    Some(provider.clone()),
                    Some(SchemaSubject::Definition(skill.address())),
                    PlanGapReason::UnresolvedActivation,
                )?;
            }
        }
        self.potential_skills = potentials;
        Ok(())
    }
    fn declarations(
        &mut self,
        key: &ProviderKey,
        d: &DeclaredSlots,
        pending: &mut BTreeSet<ProviderKey>,
    ) -> Result<()> {
        if !d.parameters.is_complete()
            || !d.choices.is_complete()
            || !d.grants.is_complete()
            || !d.actors.is_complete()
            || !d.skill_grants.is_complete()
            || !d.outputs.is_complete()
            || !d.sockets.is_complete()
        {
            self.gap(Some(key.clone()), None, PlanGapReason::PartialDeclarations)?;
        }
        for grant in &d.grants.members {
            charge(&mut self.work, key.grant_path.len() + 1)?;
            if let SchemaLookup::Known(schema) = self.index.slot(grant) {
                if let GrantTarget::Actor(actor) = &schema.target {
                    let target = OwnedActorKey {
                        provider: key.clone(),
                        slot: actor.clone(),
                    };
                    self.actors
                        .entry(target)
                        .or_default()
                        .push(PlanValueKey::Grant {
                            provider: key.clone(),
                            slot: grant.clone(),
                        });
                }
                let mut child = key.clone();
                child.grant_path.push(grant.clone());
                pending.insert(child);
            } else {
                self.gap(
                    Some(key.clone()),
                    Some(SchemaSubject::Slot(GrantSlotDefId::address(grant))),
                    PlanGapReason::UnresolvedTopology,
                )?;
            }
            if pending.len() + self.providers.len() > self.limits.max_providers {
                return Err(PlanError::Limit("providers"));
            }
        }
        Ok(())
    }

    fn owner(
        &mut self,
        subject: SchemaSubject,
        key: &ProviderKey,
        actor: &ActorKey,
        skill: Option<&GeneratedSkillKey>,
    ) -> Result<()> {
        charge(&mut self.work, self.rules.input().owners.len() + 1)?;
        let Some(row) = self
            .rules
            .input()
            .owners
            .iter()
            .find(|r| r.owner == subject)
        else {
            return self.gap(
                Some(key.clone()),
                Some(subject),
                PlanGapReason::MissingPrograms,
            );
        };
        if !row.programs.is_complete() {
            self.gap(
                Some(key.clone()),
                Some(subject.clone()),
                PlanGapReason::PartialPrograms,
            )?;
        }
        for program in &row.programs.members {
            let contexts: Vec<ConcreteEntity> = match program.context {
                RuleEntityKind::Actor => vec![ConcreteEntity::Actor(actor.clone())],
                RuleEntityKind::EquipmentUse => match (&key.root, key.grant_path.is_empty()) {
                    (
                        ProviderRoot::EquipmentUse(id)
                        | ProviderRoot::ItemModifier {
                            equipment_use: id, ..
                        },
                        true,
                    ) => vec![ConcreteEntity::EquipmentUse(*id)],
                    _ => vec![],
                },
                RuleEntityKind::Enemy => vec![ConcreteEntity::Enemy],
                RuleEntityKind::Environment => vec![ConcreteEntity::Environment],
                RuleEntityKind::Action => {
                    charge(&mut self.work, self.actions.len() + 1)?;
                    self.actions
                        .iter()
                        .filter(|a| {
                            a.action.provider == *key
                                && match &subject {
                                    SchemaSubject::Definition(DefinitionAddress::Skill(id)) => {
                                        a.action.output.declaration
                                            == SlotOwnerDefId::Skill(id.clone())
                                    }
                                    SchemaSubject::Definition(DefinitionAddress::Gem(_)) => true,
                                    SchemaSubject::Slot(SlotAddress::Actor(_)) => {
                                        a.action.actor == *actor
                                    }
                                    _ => false,
                                }
                        })
                        .cloned()
                        .map(|a| ConcreteEntity::Action(Box::new(a)))
                        .collect()
                }
            };
            if contexts.is_empty() {
                self.gap(
                    Some(key.clone()),
                    Some(subject.clone()),
                    PlanGapReason::UnsupportedContext,
                )?;
            }
            for entity in contexts {
                let mut context = Context {
                    origin: RuleOrigin::Provider {
                        provider: key.clone(),
                    },
                    provider: Some(key.clone()),
                    actor: actor.clone(),
                    skill: skill.cloned(),
                    entity,
                };
                if let ConcreteEntity::Action(a) = &context.entity {
                    let resolution = self.resolver.action(a)?;
                    charge(&mut self.work, resolution.work_used())?;
                    if resolution.value().is_none() {
                        self.gap(
                            Some(key.clone()),
                            Some(subject.clone()),
                            PlanGapReason::UnresolvedTopology,
                        )?;
                        continue;
                    }
                    context.actor = a.action.actor.clone();
                }
                self.instantiate(subject.clone(), program, &context)?;
            }
        }
        Ok(())
    }
    fn instantiate(
        &mut self,
        owner: SchemaSubject,
        program: &RuleProgram,
        context: &Context,
    ) -> Result<()> {
        if self.invocations.len() >= self.limits.max_invocations {
            return Err(PlanError::Limit("invocations"));
        }
        charge(
            &mut self.work,
            program.reads.len() + program.effects.len() + self.invocations.len() + 1,
        )?;
        let key = ProgramOccurrenceKey {
            origin: context.origin.clone(),
            owner: owner.clone(),
            program: program.id.clone(),
            entity: context.entity.clone(),
        };
        if self.invocations.iter().any(|v| v.key == key) {
            return Ok(());
        }
        let prepared = self.rules.prepare_program(&owner, &program.id)?;
        let authored: BTreeMap<_, _> = program.reads.iter().map(|r| (&r.id, &r.source)).collect();
        let mut reads = Vec::with_capacity(program.reads.len());
        for id in prepared.read_ids() {
            reads.push(self.read(authored[id], context, &owner)?);
        }
        let read_ids = prepared.read_ids().cloned().collect();
        let invocation = self.invocations.len();
        self.invocations.push(Invocation {
            key: key.clone(),
            program: prepared,
            reads: vec![],
            read_ids,
        });
        self.pending.push(reads);
        for (effect, definition) in program.effects.iter().enumerate() {
            let target = match &definition.effect {
                RuleEffectKind::Contribute {
                    entity: e,
                    stat,
                    contribution,
                    ..
                } => BoundEffectTarget::Contribution {
                    key: ContributionKey {
                        entity: entity(*e, context),
                        stat: stat.clone(),
                        kind: *contribution,
                    },
                },
                RuleEffectKind::Derive {
                    entity: e, stat, ..
                } => BoundEffectTarget::Value {
                    key: PlanValueKey::Stat {
                        entity: entity(*e, context),
                        stat: stat.clone(),
                    },
                },
                RuleEffectKind::Capability {
                    entity: e,
                    capability,
                    ..
                } => BoundEffectTarget::Value {
                    key: PlanValueKey::Capability {
                        entity: entity(*e, context),
                        capability: capability.clone(),
                    },
                },
                RuleEffectKind::ActivateGrant { slot, .. } => BoundEffectTarget::Value {
                    key: PlanValueKey::Grant {
                        provider: context.provider.clone().ok_or_else(|| {
                            PlanError::Invalid("grant requires provider occurrence".into())
                        })?,
                        slot: slot.clone(),
                    },
                },
                RuleEffectKind::ProjectSkillParameter {
                    skill, parameter, ..
                } => BoundEffectTarget::Value {
                    key: PlanValueKey::SkillParameter {
                        skill: Box::new(GeneratedSkillKey {
                            provider: context.provider.clone().ok_or_else(|| {
                                PlanError::Invalid(
                                    "skill projection requires provider occurrence".into(),
                                )
                            })?,
                            slot: skill.clone(),
                        }),
                        parameter: parameter.clone(),
                    },
                },
                RuleEffectKind::ProjectActorStat { actor, stat, .. } => BoundEffectTarget::Value {
                    key: PlanValueKey::Stat {
                        entity: ConcreteEntity::Actor(ActorKey::Owned(Box::new(OwnedActorKey {
                            provider: context.provider.clone().ok_or_else(|| {
                                PlanError::Invalid(
                                    "actor projection requires provider occurrence".into(),
                                )
                            })?,
                            slot: actor.clone(),
                        }))),
                        stat: stat.clone(),
                    },
                },
                RuleEffectKind::Requirement { code, .. } => {
                    BoundEffectTarget::Requirement { code: code.clone() }
                }
                RuleEffectKind::SupportApplicability { .. } => BoundEffectTarget::Applicability,
            };
            let gates = self.context_gates(context)?;
            self.add_effect(
                EffectNode {
                    key: EffectOccurrenceKey {
                        invocation: key.clone(),
                        effect: definition.id.clone(),
                    },
                    target,
                    operation: EffectOperation::Program { invocation, effect },
                    gates: vec![],
                    dependencies: vec![],
                },
                gates,
            )?;
        }
        Ok(())
    }
    fn add_effect(&mut self, node: EffectNode, gates: Vec<PendingRead>) -> Result<()> {
        if self.effects.len() >= self.limits.max_effects {
            return Err(PlanError::Limit("effects"));
        }
        let i = self.effects.len();
        match &node.target {
            BoundEffectTarget::Value { key } => {
                if let Some(previous) = self.values.get(key) {
                    return Err(PlanError::Invalid(format!(
                        "competing final producers for {key:?}: {:?} and {:?}",
                        self.effects[*previous].key, node.key
                    )));
                }
                self.values.insert(key.clone(), i);
            }
            BoundEffectTarget::Contribution { key } => {
                self.contributions.entry(key.clone()).or_default().push(i)
            }
            _ => {}
        }
        self.effects.push(node);
        self.gates.push(gates);
        Ok(())
    }
    fn context_gates(&mut self, context: &Context) -> Result<Vec<PendingRead>> {
        let depth = context.provider.as_ref().map_or(0, |p| p.grant_path.len());
        charge(
            &mut self.work,
            depth
                .checked_mul(depth + 1)
                .ok_or(PlanError::Limit("gate expansion"))?
                + 1,
        )?;
        let mut gates = Vec::new();
        if let Some(provider) = &context.provider {
            let mut ancestors = vec![provider.root.clone()];
            let mut seen = BTreeSet::new();
            while let Some(ancestor) = ancestors.pop() {
                charge(&mut self.work, 1)?;
                if !seen.insert(ancestor.clone()) {
                    continue;
                }
                if self.unsupported_roots.contains(&ancestor) {
                    gates.push(missing(PlanGapReason::UnsupportedRelation));
                    break;
                }
                if let ProviderRoot::EquipmentUse(id)
                | ProviderRoot::ItemModifier {
                    equipment_use: id, ..
                } = ancestor
                {
                    let equipment = &self.request.build().input().equipment;
                    charge(&mut self.work, equipment.len())?;
                    if let Some(row) = equipment.iter().find(|e| e.id == id) {
                        match row.destination {
                            EquipmentDestination::ItemSocket { container, .. } => {
                                ancestors.push(ProviderRoot::EquipmentUse(container))
                            }
                            EquipmentDestination::PassiveSocket { allocation, .. } => {
                                ancestors.push(ProviderRoot::Allocation(allocation))
                            }
                            EquipmentDestination::CharacterSlot(_) => {}
                        }
                    }
                }
            }
            if let ConcreteEntity::Action(action) = &context.entity
                && let SlotOwnerDefId::Skill(skill) = &action.action.output.declaration
                && self
                    .potential_skills
                    .contains(&(provider.clone(), skill.clone()))
            {
                // A root selector remains a different address from an explicitly
                // supplied child. Do not silently redirect saved queries or supports.
                self.gap(
                    Some(provider.clone()),
                    Some(SchemaSubject::Definition(skill.address())),
                    PlanGapReason::UnresolvedActivation,
                )?;
                gates.push(missing(PlanGapReason::UnresolvedActivation));
            }
            for (i, slot) in provider.grant_path.iter().enumerate() {
                gates.push(PendingRead::Value(PlanValueKey::Grant {
                    provider: ProviderKey {
                        root: provider.root.clone(),
                        grant_path: provider.grant_path[..i].to_vec(),
                    },
                    slot: slot.clone(),
                }));
            }
        }
        if let ActorKey::Owned(actor) = &context.actor {
            match self.actors.get(actor.as_ref()).map(Vec::as_slice) {
                Some([grant]) => gates.push(PendingRead::Value(grant.clone())),
                _ => gates.push(missing(PlanGapReason::UnresolvedActivation)),
            }
        }
        if let Some(skill) = &context.skill {
            charge(&mut self.work, 2)?;
            if let SchemaLookup::Known(grant) = self.index.slot(&skill.slot)
                && let SchemaLookup::Known(definition) = self.index.definition(&grant.skill)
            {
                charge(
                    &mut self.work,
                    definition.declarations.parameters.members.len(),
                )?;
                if !definition.declarations.parameters.is_complete() {
                    gates.push(missing(PlanGapReason::PartialDeclarations));
                }
                for parameter in &definition.declarations.parameters.members {
                    charge(&mut self.work, 1)?;
                    match self.index.slot(parameter) {
                        SchemaLookup::Known(schema)
                            if schema.presence == SlotPresence::RequiredOnce =>
                        {
                            gates.push(PendingRead::Required(Box::new(PendingRead::Value(
                                PlanValueKey::SkillParameter {
                                    skill: Box::new(skill.clone()),
                                    parameter: parameter.clone(),
                                },
                            ))));
                        }
                        SchemaLookup::Known(_) => {}
                        _ => gates.push(missing(PlanGapReason::SchemaUnresolved)),
                    }
                }
            } else {
                gates.push(missing(PlanGapReason::SchemaUnresolved));
            }
        }
        Ok(gates)
    }
    fn encounter_and_usage(&mut self) -> Result<()> {
        let scenario = self.request.scenario().input();
        let subject = SchemaSubject::Definition(scenario.enemy.encounter.address());
        charge(&mut self.work, self.rules.input().owners.len() + 1)?;
        if let Some(row) = self
            .rules
            .input()
            .owners
            .iter()
            .find(|r| r.owner == subject)
        {
            if !row.programs.is_complete() {
                self.gap(None, Some(subject.clone()), PlanGapReason::PartialPrograms)?;
            }
            for program in &row.programs.members {
                let entity = match program.context {
                    RuleEntityKind::Enemy => ConcreteEntity::Enemy,
                    RuleEntityKind::Environment => ConcreteEntity::Environment,
                    _ => {
                        self.gap(
                            None,
                            Some(subject.clone()),
                            PlanGapReason::UnsupportedContext,
                        )?;
                        continue;
                    }
                };
                self.instantiate(
                    subject.clone(),
                    program,
                    &Context {
                        origin: RuleOrigin::Encounter,
                        provider: None,
                        actor: ActorKey::Player,
                        skill: None,
                        entity,
                    },
                )?;
            }
        } else {
            self.gap(None, Some(subject), PlanGapReason::MissingPrograms)?;
        }
        // Usage selection is retained but relation/activation semantics are not inferred.
        for u in &scenario.usage {
            self.gap(
                None,
                Some(SchemaSubject::Definition(u.policy.address())),
                PlanGapReason::UnsupportedRelation,
            )?;
        }
        Ok(())
    }
    fn action_programs(&mut self) -> Result<()> {
        let actions: Vec<_> = self.actions.iter().cloned().collect();
        for action in actions {
            let resolution = self.resolver.action(&action)?;
            charge(&mut self.work, resolution.work_used())?;
            let Some(resolved) = resolution.into_value() else {
                self.gap(
                    Some(action.action.provider.clone()),
                    Some(SchemaSubject::Slot(ActionOutputDefId::address(
                        &action.action.output,
                    ))),
                    PlanGapReason::UnresolvedTopology,
                )?;
                continue;
            };
            let subject = resolved.subject();
            let skill = match resolved.provider().exposure() {
                ProviderExposure::Skill { key, .. } => Some(key.clone()),
                _ => None,
            };
            let context = Context {
                origin: RuleOrigin::Provider {
                    provider: action.action.provider.clone(),
                },
                provider: Some(action.action.provider.clone()),
                actor: resolved.expected_actor().clone(),
                skill,
                entity: ConcreteEntity::Action(Box::new(action.clone())),
            };
            charge(&mut self.work, self.rules.input().owners.len() + 1)?;
            if let Some(row) = self
                .rules
                .input()
                .owners
                .iter()
                .find(|r| r.owner == subject)
            {
                if !row.programs.is_complete() {
                    self.gap(
                        context.provider.clone(),
                        Some(subject.clone()),
                        PlanGapReason::PartialPrograms,
                    )?;
                }
                for program in &row.programs.members {
                    if program.context == RuleEntityKind::Action {
                        self.instantiate(subject.clone(), program, &context)?;
                    } else {
                        self.gap(
                            context.provider.clone(),
                            Some(subject.clone()),
                            PlanGapReason::UnsupportedContext,
                        )?;
                    }
                }
            } else {
                self.gap(
                    context.provider.clone(),
                    Some(subject),
                    PlanGapReason::MissingPrograms,
                )?;
            }
        }
        Ok(())
    }

    fn routes(&mut self, routing: &OwnedActionRouting) -> Result<()> {
        let mut sources = Vec::new();
        for action in self.actions.clone() {
            let resolution = self.resolver.action(&action)?;
            charge(&mut self.work, resolution.work_used())?;
            let Some(resolved) = resolution.into_value() else {
                continue;
            };
            let skill = match resolved.provider().exposure() {
                ProviderExposure::Skill { key, .. } => Some(key.clone()),
                _ => None,
            };
            let Some(routes) = routing.routes_for(&action.action.output) else {
                self.gap(
                    Some(action.action.provider.clone()),
                    None,
                    PlanGapReason::MissingRouting,
                )?;
                continue;
            };
            if !routes.is_complete() {
                self.gap(
                    Some(action.action.provider.clone()),
                    None,
                    PlanGapReason::PartialRouting,
                )?;
            }
            for route in &routes.members {
                charge(&mut self.work, 1)?;
                if let ActionRouteSelection::Exact(selector) = &route.selection
                    && (selector.part != action.part
                        || selector.mode != action.mode
                        || selector.stat_set != action.stat_set)
                {
                    continue;
                }
                let source = match &route.source {
                    ActionStatRouteSource::ActionActor { stat } => {
                        PendingRead::Value(PlanValueKey::Stat {
                            entity: ConcreteEntity::Actor(action.action.actor.clone()),
                            stat: stat.clone(),
                        })
                    }
                    ActionStatRouteSource::PlayerEquipment { slot, stat } => {
                        charge(&mut self.work, self.request.build().input().equipment.len())?;
                        let selected: Vec<_> = self
                            .request
                            .build()
                            .input()
                            .equipment
                            .iter()
                            .filter(|e| {
                                e.destination == EquipmentDestination::CharacterSlot(slot.clone())
                                    && self
                                        .providers
                                        .contains(&root(ProviderRoot::EquipmentUse(e.id)))
                            })
                            .collect();
                        match selected.as_slice() {
                            [e] => PendingRead::Value(PlanValueKey::Stat {
                                entity: ConcreteEntity::EquipmentUse(e.id),
                                stat: stat.clone(),
                            }),
                            _ => missing(PlanGapReason::UnsupportedRelation),
                        }
                    }
                };
                let subject =
                    SchemaSubject::Slot(ActionOutputDefId::address(&action.action.output));
                let context = Context {
                    origin: RuleOrigin::Route {
                        action: Box::new(action.clone()),
                        route: route.id.clone(),
                    },
                    provider: Some(action.action.provider.clone()),
                    actor: action.action.actor.clone(),
                    skill: skill.clone(),
                    entity: ConcreteEntity::Action(Box::new(action.clone())),
                };
                sources.push((self.effects.len(), source));
                let gates = self.context_gates(&context)?;
                self.add_effect(
                    EffectNode {
                        key: EffectOccurrenceKey {
                            invocation: ProgramOccurrenceKey {
                                origin: context.origin.clone(),
                                owner: subject,
                                program: route.id.clone(),
                                entity: context.entity.clone(),
                            },
                            effect: route.id.clone(),
                        },
                        target: BoundEffectTarget::Value {
                            key: PlanValueKey::Stat {
                                entity: context.entity.clone(),
                                stat: route.target.clone(),
                            },
                        },
                        operation: EffectOperation::Route {
                            source: ReadBinding::Missing(PlanGapReason::MissingProducer),
                        },
                        gates: vec![],
                        dependencies: vec![],
                    },
                    gates,
                )?;
            }
        }
        for (index, source) in sources {
            self.effects[index].operation = EffectOperation::Route {
                source: resolve(
                    source,
                    &self.values,
                    &self.contributions,
                    self.gaps.is_empty(),
                    &mut self.work,
                    &mut self.binding_edges,
                    self.limits.max_edges,
                )?,
            };
        }
        Ok(())
    }
}
fn read_dependencies(
    read: &ReadBinding,
    out: &mut BTreeSet<usize>,
    work: &mut usize,
) -> Result<()> {
    match read {
        ReadBinding::Present { source } => read_dependencies(source, out, work)?,
        ReadBinding::Final {
            effect: Some(i), ..
        } => {
            charge(work, 1)?;
            out.insert(*i);
        }
        ReadBinding::Reduction { effects, .. } => {
            charge(work, effects.len())?;
            out.extend(effects);
        }
        _ => {}
    }
    Ok(())
}
fn dependency_order(
    effects: &mut [EffectNode],
    invocations: &[Invocation],
    limits: PlanLimits,
    work: &mut usize,
) -> Result<Vec<usize>> {
    let mut outgoing = vec![Vec::new(); effects.len()];
    let mut edges = 0usize;
    for (index, node) in effects.iter_mut().enumerate() {
        let mut dependencies = BTreeSet::new();
        for gate in &node.gates {
            read_dependencies(gate, &mut dependencies, work)?;
        }
        match &node.operation {
            EffectOperation::Program { invocation, effect } => {
                let inv = &invocations[*invocation];
                for read in inv.program.effect_read_indices(*effect)? {
                    read_dependencies(&inv.reads[*read], &mut dependencies, work)?;
                }
            }
            EffectOperation::Route { source } => {
                read_dependencies(source, &mut dependencies, work)?
            }
        }
        edges = edges
            .checked_add(dependencies.len())
            .ok_or(PlanError::Limit("edges"))?;
        if edges > limits.max_edges {
            return Err(PlanError::Limit("edges"));
        }
        for d in &dependencies {
            outgoing[*d].push(index);
        }
        node.dependencies = dependencies.into_iter().collect();
    }
    let mut remaining: Vec<_> = effects.iter().map(|n| n.dependencies.len()).collect();
    let mut ready: BTreeSet<_> = remaining
        .iter()
        .enumerate()
        .filter_map(|(i, n)| (*n == 0).then_some(i))
        .collect();
    let mut order = Vec::with_capacity(effects.len());
    while let Some(i) = ready.pop_first() {
        charge(work, 1 + outgoing[i].len())?;
        order.push(i);
        for next in &outgoing[i] {
            remaining[*next] -= 1;
            if remaining[*next] == 0 {
                ready.insert(*next);
            }
        }
    }
    if order.len() != effects.len() {
        let i = remaining.iter().position(|n| *n > 0).expect("cycle member");
        return Err(PlanError::Invalid(format!(
            "effect dependency cycle at {:?}",
            effects[i].key
        )));
    }
    Ok(order)
}
