use super::*;
use poe_optimizer_core::owned_routing::*;
#[cfg(test)]
mod deferred_tests;
mod preparation;
mod reads;
mod sources;
mod transforms;
type ModifierTransforms = BTreeMap<PlanValueKey, BTreeMap<(usize, i64), BoundModifierTransform>>;

#[derive(Clone, Copy)]
struct FinalReadSources<'a> {
    values: &'a BTreeMap<PlanValueKey, usize>,
    contributions: &'a BTreeMap<ContributionKey, Vec<usize>>,
    transforms: &'a ModifierTransforms,
}

#[derive(Clone, Debug)]
enum PendingRead {
    Ready(ReadBinding),
    Select {
        decision: usize,
        when_true: Box<PendingRead>,
        when_false: Box<PendingRead>,
    },
    Required(Box<PendingRead>),
    Value(PlanValueKey),
    Contributions(ContributionKey, ContributionReduction, ParameterValue),
    ModifierTransforms {
        key: PlanValueKey,
        initial: Box<PlanValueKey>,
    },
}
#[derive(Clone)]
struct Context {
    origin: RuleOrigin,
    provider: Option<ProviderKey>,
    actor: ActorKey,
    skill: Option<GeneratedSkillKey>,
    assigned_skill: Option<SkillTarget>,
    entity: ConcreteEntity,
}
/// Topology discovery records owner visits in their historical order. Actions
/// can be registered before these visits bind Action-context rule programs.
struct DeferredOwner {
    subject: SchemaSubject,
    provider: ProviderKey,
    actor: ActorKey,
    skill: Option<GeneratedSkillKey>,
    /// Position among topology diagnostics, before this owner's diagnostics.
    gap_offset: usize,
}
struct Builder<'a, I> {
    request: &'a OwnedEvaluationRequest,
    index: &'a I,
    rules: &'a CompiledRulePackage,
    operations: RuleOperationsVersion,
    preparation: bool,
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
    transforms: ModifierTransforms,
    actions: BTreeSet<ActionSelection>,
    providers: BTreeSet<ProviderKey>,
    actors: BTreeMap<OwnedActorKey, Vec<PlanValueKey>>,
    /// Actual discovered actors, distinct from merely declared grant targets.
    receiver_actors: BTreeSet<ActorKey>,
    unsupported_roots: BTreeSet<ProviderRoot>,
    potential_skills: BTreeSet<(ProviderKey, SkillDefId)>,
    skill_supplies: BTreeMap<GeneratedSkillKey, ProviderKey>,
    actor_supplies: BTreeMap<OwnedActorKey, ProviderKey>,
    supply_ancestors: BTreeMap<ProviderKey, BTreeSet<DefinitionAddress>>,
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
        SlotOwnerDefId::Actor(id) => id.address(),
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
fn entity(relative: RuleEntity, context: &Context) -> Result<ConcreteEntity> {
    Ok(match relative {
        RuleEntity::Current => context.entity.clone(),
        RuleEntity::SupportOrigin => match context.provider.as_ref() {
            Some(ProviderKey {
                root: ProviderRoot::SupportAssignment(id),
                grant_path,
            }) if grant_path.is_empty() => {
                ConcreteEntity::SupportOrigin(SupportOrigin::Assignment(*id))
            }
            _ => {
                return Err(PlanError::Invalid(
                    "support-origin read requires an exact assignment".into(),
                ));
            }
        },
        RuleEntity::AssignedSkill => {
            ConcreteEntity::Skill(Box::new(context.assigned_skill.clone().ok_or_else(
                || PlanError::Invalid("assigned-skill read requires an explicit target".into()),
            )?))
        }
        RuleEntity::Skill => {
            ConcreteEntity::Skill(Box::new(match &context.entity {
                ConcreteEntity::Skill(target) => target.as_ref().clone(),
                _ => exact_skill(context.provider.as_ref(), context.skill.as_ref()).ok_or_else(
                    || PlanError::Invalid("skill read requires an exact skill occurrence".into()),
                )?,
            }))
        }
        RuleEntity::Modifier => {
            let provider = context.provider.as_ref().ok_or_else(|| {
                PlanError::Invalid("modifier value requires an exact modifier provider".into())
            })?;
            let ProviderRoot::ItemModifier { equipment_use, .. } = &provider.root else {
                return Err(PlanError::Invalid(
                    "modifier value requires an item-modifier root".into(),
                ));
            };
            if !provider.grant_path.is_empty()
                || context.entity != ConcreteEntity::EquipmentUse(*equipment_use)
            {
                return Err(PlanError::Invalid(
                    "modifier value has an incompatible occurrence context".into(),
                ));
            }
            ConcreteEntity::Modifier(provider.clone())
        }
        RuleEntity::Actor => ConcreteEntity::Actor(context.actor.clone()),
        RuleEntity::Player => ConcreteEntity::Actor(ActorKey::Player),
        RuleEntity::Enemy => ConcreteEntity::Enemy,
        RuleEntity::Environment => ConcreteEntity::Environment,
    })
}
fn exact_skill(
    provider: Option<&ProviderKey>,
    skill: Option<&GeneratedSkillKey>,
) -> Option<SkillTarget> {
    if let Some(skill) = skill {
        return Some(SkillTarget::Generated(Box::new(skill.clone())));
    }
    match provider {
        Some(ProviderKey {
            root: ProviderRoot::SkillUse(id),
            grant_path,
        }) if grant_path.is_empty() => Some(SkillTarget::Authored(*id)),
        _ => None,
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
    compile_with_purpose(request, definitions, rules, routing, limits, false)
}
pub(super) fn compile_with_purpose<I: DefinitionSchemaIndex>(
    request: Arc<OwnedEvaluationRequest>,
    definitions: Arc<I>,
    rules: Arc<CompiledRulePackage>,
    routing: Arc<OwnedActionRouting>,
    limits: PlanLimits,
    preparation: bool,
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
    let operations = RuleOperationsVersion::parse(rules.input().operations_version.as_str())
        .ok_or_else(|| PlanError::Invalid("unsupported owned rule operations".into()))?;
    let identity = digest_owned(
        if preparation {
            "owned-support-input-plan-v1"
        } else {
            operations.effect_plan_domain()
        },
        &bindings,
        limits.max_wire_bytes,
    )?;
    let resolver = OwnedOccurrenceResolver::new(definitions.as_ref(), &request, limits.binding)?;
    let mut b = Builder::new(
        &request,
        definitions.as_ref(),
        &rules,
        resolver,
        operations,
        limits,
    );
    b.preparation = preparation;
    if preparation && !operations.supports_preparation_scopes() {
        return Err(PlanError::Invalid(
            "support input preparation requires operation v12".into(),
        ));
    }
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
    let owners = b.discover()?;
    // Register any explicitly declared additional receiving actions at this
    // boundary, before any Gem/Skill owner Action-context programs are bound.
    b.instantiate_discovered_owners(owners)?;
    b.check_skill_supply_coverage()?;
    if !request.build().input().payload_links.is_empty() {
        b.gap(None, None, PlanGapReason::UnsupportedRelation)?;
    }
    b.stat_receivers()?;
    b.encounter_and_usage()?;
    b.action_programs()?;
    b.routes(&routing)?;
    let query_gates = b.query_gates()?;
    let preparation_gates = if preparation {
        b.preparation_gates()?
    } else {
        BTreeMap::new()
    };
    let complete = b.gaps.is_empty();
    for (inv, reads) in b.invocations.iter_mut().zip(b.pending) {
        inv.reads = reads
            .into_iter()
            .map(|r| {
                resolve(
                    r,
                    FinalReadSources {
                        values: &b.values,
                        contributions: &b.contributions,
                        transforms: &b.transforms,
                    },
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
                    FinalReadSources {
                        values: &b.values,
                        contributions: &b.contributions,
                        transforms: &b.transforms,
                    },
                    complete,
                    &mut b.work,
                    &mut b.binding_edges,
                    limits.max_edges,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        match &mut node.operation {
            EffectOperation::Route { source } | EffectOperation::SelectSource { source } => {
                bind_completeness(source, complete, &mut b.work)?;
            }
            EffectOperation::Program { .. } => {}
        }
    }
    let query_gates = query_gates
        .into_iter()
        .map(|gates| {
            gates
                .into_iter()
                .map(|r| {
                    resolve(
                        r,
                        FinalReadSources {
                            values: &b.values,
                            contributions: &b.contributions,
                            transforms: &b.transforms,
                        },
                        complete,
                        &mut b.work,
                        &mut b.binding_edges,
                        limits.max_edges,
                    )
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    let order = dependency_order(&mut b.effects, &b.invocations, limits, &mut b.work)?;
    let preparation_gates = preparation_gates
        .into_iter()
        .map(|(target, gates)| {
            let gates = gates
                .into_iter()
                .map(|r| {
                    resolve(
                        r,
                        FinalReadSources {
                            values: &b.values,
                            contributions: &b.contributions,
                            transforms: &b.transforms,
                        },
                        complete,
                        &mut b.work,
                        &mut b.binding_edges,
                        limits.max_edges,
                    )
                })
                .collect::<Result<Vec<_>>>()?;
            Ok((target, gates))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    Ok(OwnedEffectPlan {
        request: Arc::clone(&request),
        definitions: Arc::clone(&definitions),
        rules: Arc::clone(&rules),
        routing: Arc::clone(&routing),
        identity,
        bindings,
        binding_report: report,
        limits,
        gaps: b.gaps,
        complete,
        invocations: b.invocations,
        effects: b.effects,
        order,
        values: b.values,
        query_gates,
        preparation_gates,
    })
}
fn resolve(
    read: PendingRead,
    sources: FinalReadSources<'_>,
    complete: bool,
    work: &mut usize,
    edges: &mut usize,
    max_edges: usize,
) -> Result<ReadBinding> {
    let FinalReadSources {
        values,
        contributions,
        transforms,
    } = sources;
    let expansion = match &read {
        PendingRead::Ready(_) => 0,
        PendingRead::Select { .. } => 3,
        PendingRead::Required(_) => 1,
        PendingRead::Value(_) => 1,
        PendingRead::Contributions(key, ..) => contributions.get(key).map_or(0, Vec::len),
        PendingRead::ModifierTransforms { key, .. } => transforms
            .get(key)
            .map_or(1, |steps| steps.len().saturating_add(1)),
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
        PendingRead::Select {
            decision,
            when_true,
            when_false,
        } => ReadBinding::Select {
            decision,
            when_true: Box::new(resolve(
                *when_true, sources, complete, work, edges, max_edges,
            )?),
            when_false: Box::new(resolve(
                *when_false,
                sources,
                complete,
                work,
                edges,
                max_edges,
            )?),
        },
        PendingRead::Required(source) => ReadBinding::Present {
            source: Box::new(resolve(*source, sources, complete, work, edges, max_edges)?),
        },
        PendingRead::Value(key) => ReadBinding::Final {
            effect: values.get(&key).copied(),
            complete,
        },
        PendingRead::ModifierTransforms { key, initial } => ReadBinding::ModifierTransforms {
            initial: Box::new(ReadBinding::Final {
                effect: values.get(initial.as_ref()).copied(),
                complete,
            }),
            steps: transforms
                .get(&key)
                .map(|steps| steps.values().cloned().collect())
                .unwrap_or_default(),
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
    fn new(
        request: &'a OwnedEvaluationRequest,
        index: &'a I,
        rules: &'a CompiledRulePackage,
        resolver: OwnedOccurrenceResolver<'a, I>,
        operations: RuleOperationsVersion,
        limits: PlanLimits,
    ) -> Self {
        Self {
            request,
            index,
            rules,
            operations,
            preparation: false,
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
            transforms: BTreeMap::new(),
            actions: BTreeSet::new(),
            providers: BTreeSet::new(),
            actors: BTreeMap::new(),
            receiver_actors: BTreeSet::from([ActorKey::Player]),
            unsupported_roots: BTreeSet::new(),
            potential_skills: BTreeSet::new(),
            skill_supplies: BTreeMap::new(),
            actor_supplies: BTreeMap::new(),
            supply_ancestors: BTreeMap::new(),
            supplied_skills: BTreeMap::new(),
            binding_edges: 0,
        }
    }
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
    fn discover(&mut self) -> Result<Vec<DeferredOwner>> {
        let input = self.request.build().input();
        let mut owners_to_instantiate = Vec::new();
        let mut pending = BTreeSet::new();
        if !self.preparation {
            self.unsupported_roots.extend(
                input
                    .supports
                    .iter()
                    .map(|s| ProviderRoot::SupportAssignment(s.id)),
            );
        }
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
                if !key.grant_path.is_empty() && self.operations.supports_actor_supply() {
                    // Discovery follows explicitly declared grants from an available
                    // parent. A rejected child is contradictory potential topology,
                    // not permission to silently filter the authored supply graph.
                    return Err(PlanError::Invalid(
                        "declared potential supply is unavailable in its actor/provider context"
                            .into(),
                    ));
                }
                continue;
            }
            let Some(provider) = resolution.into_value() else {
                self.gap(Some(key), None, PlanGapReason::UnresolvedTopology)?;
                continue;
            };
            if self.operations.supports_actor_supply() {
                self.check_supply_ancestors(&key, provider.exposure())?;
            }
            self.providers.insert(key.clone());
            if matches!(key.root, ProviderRoot::SupportAssignment(_)) && !self.preparation {
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
                ProviderExposure::Root {
                    owners,
                    skills,
                    implicit_passives,
                } => {
                    // Roots are class/ascendancy-owned providers, not paid allocation
                    // occurrences. Retain known owners while marking every open set.
                    for roots in implicit_passives {
                        charge(&mut self.work, 1)?;
                        if !roots.declaration().is_complete() {
                            self.gap(
                                Some(key.clone()),
                                Some(owner_subject(roots.owner())),
                                PlanGapReason::PartialDeclarations,
                            )?;
                        }
                        for (node, schema) in roots.nodes() {
                            charge(&mut self.work, 1)?;
                            let subject = Some(SchemaSubject::Definition(node.address()));
                            match schema {
                                SchemaLookup::Known(schema) => {
                                    if !schema.pools.members.is_empty() {
                                        return Err(PlanError::Invalid(
                                            "implicit passive root declares paid point pools"
                                                .into(),
                                        ));
                                    }
                                    if !schema.pools.is_complete() {
                                        self.gap(
                                            Some(key.clone()),
                                            subject,
                                            PlanGapReason::PartialDeclarations,
                                        )?;
                                    }
                                }
                                SchemaLookup::Missing | SchemaLookup::Unmapped(_) => {
                                    self.gap(
                                        Some(key.clone()),
                                        subject,
                                        PlanGapReason::UnresolvedTopology,
                                    )?;
                                }
                                SchemaLookup::NamespaceMismatch
                                | SchemaLookup::InconsistentIndex => {
                                    return Err(PlanError::Invalid(
                                        "implicit passive root has invalid schema binding".into(),
                                    ));
                                }
                            }
                        }
                    }
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
                        self.defer_owner(
                            &mut owners_to_instantiate,
                            owner.subject(),
                            &key,
                            provider.actor(),
                            None,
                        )?;
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
                    let activation = PlanValueKey::Grant {
                        provider: skill.provider.clone(),
                        slot: grant.clone(),
                    };
                    self.supplied_skills
                        .entry((skill.provider.clone(), definition.clone()))
                        .or_default()
                        .push(activation.clone());
                    // Potential membership belongs to the current actor, including
                    // abilities supplied by another ability inside that actor.
                    if let ActorKey::Owned(actor) = provider.actor()
                        && let Some(actor_provider) = self.actor_supplies.get(actor.as_ref())
                        && actor_provider != &skill.provider
                    {
                        charge(&mut self.work, actor_provider.grant_path.len() + 1)?;
                        self.supplied_skills
                            .entry((actor_provider.clone(), definition.clone()))
                            .or_default()
                            .push(activation);
                    }
                    self.declarations(&key, declarations, &mut pending)?;
                    self.defer_owner(
                        &mut owners_to_instantiate,
                        owner_subject(owner),
                        &key,
                        provider.actor(),
                        Some(skill),
                    )?;
                }
                ProviderExposure::Actor {
                    key: actor,
                    schema,
                    owner,
                    ..
                } => {
                    if owner.is_some() {
                        if !self.operations.supports_actor_supply() {
                            return Err(PlanError::Invalid(
                                "actor-owned supply requires owned-domain-operations-v11".into(),
                            ));
                        }
                        charge(&mut self.work, key.grant_path.len() + 1)?;
                        if let Some(previous) =
                            self.actor_supplies.insert(actor.clone(), key.clone())
                            && previous != key
                        {
                            return Err(PlanError::Invalid(format!(
                                "ambiguous actor supply for {actor:?}: {previous:?} and {key:?}"
                            )));
                        }
                    }
                    self.receiver_actors
                        .insert(ActorKey::Owned(Box::new(actor.clone())));
                    if !schema.skills.is_complete() || !schema.outputs.is_complete() {
                        self.gap(Some(key.clone()), None, PlanGapReason::PartialDeclarations)?;
                    }
                    for skill in &schema.skills.members {
                        charge(&mut self.work, 1)?;
                        self.potential_skills.insert((key.clone(), skill.clone()));
                        if owner.is_none() {
                            self.gap(
                                Some(key.clone()),
                                Some(SchemaSubject::Definition(skill.address())),
                                PlanGapReason::UnresolvedActivation,
                            )?;
                        }
                    }
                    self.defer_owner(
                        &mut owners_to_instantiate,
                        SchemaSubject::Slot(ActorSlotDefId::address(&actor.slot)),
                        &key,
                        provider.actor(),
                        None,
                    )?;
                    if let Some(owner) = owner {
                        self.declarations(&key, owner.declarations(), &mut pending)?;
                        self.defer_owner(
                            &mut owners_to_instantiate,
                            owner.subject(),
                            &key,
                            provider.actor(),
                            None,
                        )?;
                    }
                }
                ProviderExposure::AllocationAccess { .. } => {
                    self.gap(Some(key), None, PlanGapReason::UnsupportedRelation)?;
                }
            }
            if pending.len() + self.providers.len() > self.limits.max_providers {
                return Err(PlanError::Limit("providers"));
            }
        }
        Ok(owners_to_instantiate)
    }

    fn defer_owner(
        &mut self,
        owners: &mut Vec<DeferredOwner>,
        subject: SchemaSubject,
        provider: &ProviderKey,
        actor: &ActorKey,
        skill: Option<&GeneratedSkillKey>,
    ) -> Result<()> {
        // Owners with no programs still occupy this inventory. Bound it before
        // cloning provider paths rather than relying on later invocation counts.
        if owners.len() >= self.limits.max_owner_bindings {
            return Err(PlanError::Limit("owner bindings"));
        }
        let actor_depth = match actor {
            ActorKey::Player => 0,
            ActorKey::Owned(actor) => actor.provider.grant_path.len(),
        };
        charge(
            &mut self.work,
            provider.grant_path.len()
                + actor_depth
                + skill.map_or(0, |skill| skill.provider.grant_path.len())
                + 1,
        )?;
        owners.push(DeferredOwner {
            subject,
            provider: provider.clone(),
            actor: actor.clone(),
            skill: skill.cloned(),
            gap_offset: self.gaps.len(),
        });
        Ok(())
    }

    fn instantiate_discovered_owners(&mut self, owners: Vec<DeferredOwner>) -> Result<()> {
        if self.actions.len() > self.limits.max_providers {
            return Err(PlanError::Limit("actions"));
        }
        // Replay topology and owner diagnostics in their original encounter
        // order, including first-occurrence deduplication. Leaving future gaps
        // in self.gaps while instantiating owners would suppress the wrong copy.
        let mut topology_gaps = std::mem::take(&mut self.gaps)
            .into_iter()
            .enumerate()
            .peekable();
        for owner in owners {
            while topology_gaps
                .peek()
                .is_some_and(|(index, _)| *index < owner.gap_offset)
            {
                let (_, gap) = topology_gaps.next().expect("peeked topology gap");
                self.gap(gap.provider, gap.subject, gap.reason)?;
            }
            self.owner(
                owner.subject,
                &owner.provider,
                &owner.actor,
                owner.skill.as_ref(),
            )?;
        }
        for (_, gap) in topology_gaps {
            self.gap(gap.provider, gap.subject, gap.reason)?;
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
    /// Detect recursive potential supply independently of runtime activation.
    /// Each lineage is bounded before cloning; siblings may reuse a template.
    fn check_supply_ancestors(
        &mut self,
        key: &ProviderKey,
        exposure: &ProviderExposure<'_>,
    ) -> Result<()> {
        charge(&mut self.work, key.grant_path.len() + 1)?;
        let mut ancestors = if key.grant_path.is_empty() {
            BTreeSet::new()
        } else {
            let mut parent = key.clone();
            parent.grant_path.pop();
            let prior = self.supply_ancestors.get(&parent).ok_or_else(|| {
                PlanError::Invalid("supplied provider has no discovered parent".into())
            })?;
            charge(&mut self.work, prior.len())?;
            prior.clone()
        };
        let current = match exposure {
            ProviderExposure::Skill { owner, .. } => match owner_subject(owner) {
                SchemaSubject::Definition(address) => Some(address),
                _ => unreachable!("definition owner"),
            },
            ProviderExposure::Actor {
                owner: Some(owner), ..
            } => match owner.subject() {
                SchemaSubject::Definition(address) => Some(address),
                _ => unreachable!("definition owner"),
            },
            // Direct authored skills can also recur through their descendants.
            ProviderExposure::Root { owners, .. } => owners.iter().find_map(|owner| {
                if let SlotOwnerDefId::Skill(id) = owner.definition() {
                    Some(id.address())
                } else {
                    None
                }
            }),
            _ => None,
        };
        if let Some(current) = current
            && !ancestors.insert(current)
        {
            return Err(PlanError::Invalid(
                "recursive potential supply cycle".into(),
            ));
        }
        self.supply_ancestors.insert(key.clone(), ancestors);
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
                if self.operations.supports_actor_supply() {
                    if key.grant_path.len() >= self.limits.binding.input.max_provider_steps {
                        return Err(PlanError::Limit("provider depth"));
                    }
                    // Charge and reject before allocating any child key or occurrence.
                    if pending.len() + self.providers.len() >= self.limits.max_providers {
                        return Err(PlanError::Limit("providers"));
                    }
                }
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
            if matches!(key.root, ProviderRoot::SupportAssignment(_))
                && program.context != RuleEntityKind::SupportOrigin
            {
                // Delivery requires exact retained receiver applications. A preparation
                // plan cannot skip such programs and claim complete owner coverage.
                self.gap(
                    Some(key.clone()),
                    Some(subject.clone()),
                    PlanGapReason::UnsupportedRelation,
                )?;
                continue;
            }
            let contexts: Vec<ConcreteEntity> = match program.context {
                RuleEntityKind::SupportOrigin => match (&key.root, key.grant_path.is_empty()) {
                    (ProviderRoot::SupportAssignment(id), true) if self.preparation => {
                        vec![ConcreteEntity::SupportOrigin(SupportOrigin::Assignment(
                            *id,
                        ))]
                    }
                    _ => vec![],
                },
                RuleEntityKind::Skill => exact_skill(Some(key), skill)
                    .map(|target| ConcreteEntity::Skill(Box::new(target)))
                    .into_iter()
                    .collect(),
                RuleEntityKind::Actor => vec![ConcreteEntity::Actor(actor.clone())],
                RuleEntityKind::Modifier => {
                    return Err(PlanError::Invalid(
                        "Modifier is a relative value scope, not a program context".into(),
                    ));
                }
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
                    assigned_skill: if let ProviderRoot::SupportAssignment(id) = key.root {
                        charge(&mut self.work, self.request.build().input().supports.len())?;
                        self.request
                            .build()
                            .input()
                            .supports
                            .iter()
                            .find(|s| s.id == id)
                            .map(|s| s.target.clone())
                    } else {
                        None
                    },
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
            if matches!(
                definition.effect,
                RuleEffectKind::ProjectModifierTransform { .. }
            ) {
                self.modifier_transform(context, &key, definition, invocation, effect)?;
                continue;
            }
            let target = match &definition.effect {
                RuleEffectKind::Contribute {
                    entity: e,
                    stat,
                    contribution,
                    ..
                } => BoundEffectTarget::Contribution {
                    key: ContributionKey {
                        entity: entity(*e, context)?,
                        stat: stat.clone(),
                        kind: *contribution,
                    },
                },
                RuleEffectKind::Derive {
                    entity: e, stat, ..
                } => BoundEffectTarget::Value {
                    key: PlanValueKey::Stat {
                        entity: entity(*e, context)?,
                        stat: stat.clone(),
                    },
                },
                RuleEffectKind::Capability {
                    entity: e,
                    capability,
                    ..
                } => BoundEffectTarget::Value {
                    key: PlanValueKey::Capability {
                        entity: entity(*e, context)?,
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
                RuleEffectKind::ProjectModifierTransform { .. } => {
                    unreachable!("handled before scalar effect binding")
                }
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
    /// Resolve the same parent and generated-skill context for receiver roots and
    /// metric queries. A bare actor key would lose ancestor and required-input gates.
    fn actor_context(
        &mut self,
        actor: &ActorKey,
    ) -> Result<(SelectorBindingStatus, Option<Context>)> {
        let resolved = self.resolver.actor(actor)?;
        charge(&mut self.work, resolved.work_used())?;
        let status = resolved.status();
        if resolved.into_value().is_none() {
            return Ok((status, None));
        }
        let provider = match actor {
            ActorKey::Player => root(ProviderRoot::Character),
            ActorKey::Owned(actor) => actor.provider.clone(),
        };
        let parent = self.resolver.provider(&provider)?;
        charge(&mut self.work, parent.work_used())?;
        let parent_status = parent.status();
        let context = parent.into_value().map(|parent| Context {
            assigned_skill: None,
            origin: RuleOrigin::Provider {
                provider: provider.clone(),
            },
            skill: match parent.exposure() {
                ProviderExposure::Skill { key, .. } => Some(key.clone()),
                _ => None,
            },
            provider: Some(provider),
            actor: actor.clone(),
            entity: ConcreteEntity::Actor(actor.clone()),
        });
        Ok((parent_status, context))
    }

    fn stat_receivers(&mut self) -> Result<()> {
        let rules = self.rules.input();
        if !rules.receivers.is_complete() {
            self.gap(None, None, PlanGapReason::PartialReceivers)?;
        }
        // Index authored stat programs and exact applicability once. Neither query
        // order nor applicability declarations create actor/equipment occurrences.
        charge(&mut self.work, rules.owners.len())?;
        let mut programs = BTreeMap::new();
        for owner in &rules.owners {
            if let SchemaSubject::Definition(DefinitionAddress::Stat(stat)) = &owner.owner {
                charge(&mut self.work, owner.programs.members.len())?;
                for program in &owner.programs.members {
                    programs.insert((stat, &program.id), (program, owner.programs.is_complete()));
                }
            }
        }
        let mut applicable: BTreeMap<_, Vec<_>> = BTreeMap::new();
        let mut has_equipment = false;
        charge(&mut self.work, rules.receivers.members.len())?;
        for receiver in &rules.receivers.members {
            let (program, complete) = programs
                .get(&(&receiver.stat, &receiver.program))
                .copied()
                .ok_or_else(|| PlanError::Invalid("validated receiver program is absent".into()))?;
            charge(&mut self.work, receiver.targets.len())?;
            for target in &receiver.targets {
                has_equipment |= matches!(target, StatReceiverTarget::EquipmentTemplate { .. });
                applicable
                    .entry(target.clone())
                    .or_default()
                    .push((receiver, program, complete));
            }
        }
        let actors = std::mem::take(&mut self.receiver_actors);
        charge(&mut self.work, actors.len())?;
        for actor in actors {
            let target = match &actor {
                ActorKey::Player => ActorReceiverTarget::Player,
                ActorKey::Owned(actor) => ActorReceiverTarget::OwnedSlot {
                    slot: actor.slot.clone(),
                },
            };
            let Some(receivers) = applicable.get(&target) else {
                continue;
            };
            charge(&mut self.work, receivers.len())?;
            let (status, context) = self.actor_context(&actor)?;
            for (receiver, program, complete) in receivers {
                let subject = SchemaSubject::Definition(receiver.stat.address());
                if !complete {
                    self.gap(None, Some(subject.clone()), PlanGapReason::PartialPrograms)?;
                }
                let Some(mut context) = context.clone() else {
                    if status != SelectorBindingStatus::Unavailable {
                        self.gap(None, Some(subject), PlanGapReason::UnresolvedTopology)?;
                    }
                    continue;
                };
                context.origin = RuleOrigin::Receiver {
                    receiver: receiver.id.clone(),
                    actor: actor.clone(),
                };
                self.instantiate(subject, program, &context)?;
            }
        }
        if has_equipment {
            // Existing discovery resolves loadout and ancestor activity. Reuse
            // only its exact root occurrences, never create providers from a
            // template declaration or redirect modifiers to a receiver root.
            charge(&mut self.work, self.providers.len())?;
            let equipment: Vec<_> = self
                .providers
                .iter()
                .filter_map(|provider| {
                    if let ProviderRoot::EquipmentUse(id) = provider.root {
                        provider.grant_path.is_empty().then_some(id)
                    } else {
                        None
                    }
                })
                .collect();
            let build = self.request.build().input();
            charge(&mut self.work, build.items.len() + build.equipment.len())?;
            let items: BTreeMap<_, _> = build.items.iter().map(|item| (item.id, item)).collect();
            let uses: BTreeMap<_, _> = build
                .equipment
                .iter()
                .map(|usage| (usage.id, usage))
                .collect();
            for id in equipment {
                charge(&mut self.work, 2)?;
                let item = uses
                    .get(&id)
                    .and_then(|usage| items.get(&usage.item))
                    .ok_or_else(|| {
                        PlanError::Invalid("discovered equipment occurrence has no item".into())
                    })?;
                let target = StatReceiverTarget::EquipmentTemplate {
                    template: item.template.clone(),
                };
                let Some(receivers) = applicable.get(&target) else {
                    continue;
                };
                charge(&mut self.work, receivers.len())?;
                let provider = root(ProviderRoot::EquipmentUse(id));
                let resolved = self.resolver.provider(&provider)?;
                charge(&mut self.work, resolved.work_used())?;
                let status = resolved.status();
                let resolved = resolved.into_value();
                for (receiver, program, complete) in receivers {
                    let subject = SchemaSubject::Definition(receiver.stat.address());
                    if !complete {
                        self.gap(
                            Some(provider.clone()),
                            Some(subject.clone()),
                            PlanGapReason::PartialPrograms,
                        )?;
                    }
                    let Some(resolved) = &resolved else {
                        if status != SelectorBindingStatus::Unavailable {
                            self.gap(
                                Some(provider.clone()),
                                Some(subject),
                                PlanGapReason::UnresolvedTopology,
                            )?;
                        }
                        continue;
                    };
                    let context = Context {
                        assigned_skill: None,
                        origin: RuleOrigin::EquipmentReceiver {
                            receiver: receiver.id.clone(),
                            equipment_use: id,
                        },
                        provider: Some(provider.clone()),
                        actor: resolved.actor().clone(),
                        skill: None,
                        entity: ConcreteEntity::EquipmentUse(id),
                    };
                    self.instantiate(subject, program, &context)?;
                }
            }
        }
        Ok(())
    }

    /// Bind target existence separately from final stat production. A parent may
    /// project a useful diagnostic value into a child whose grant is false.
    fn query_gates(&mut self) -> Result<Vec<Vec<PendingRead>>> {
        let queries = &self.request.queries().input().requests;
        charge(&mut self.work, queries.len())?;
        let mut result = Vec::with_capacity(queries.len());
        for query in queries {
            let (status, context) = match &query.target {
                MetricTarget::Actor(actor) => self.actor_context(actor)?,
                MetricTarget::Action(action) => {
                    let resolved = self.resolver.action(action)?;
                    charge(&mut self.work, resolved.work_used())?;
                    let status = resolved.status();
                    let context = resolved.into_value().map(|resolved| Context {
                        assigned_skill: None,
                        origin: RuleOrigin::Provider {
                            provider: action.action.provider.clone(),
                        },
                        provider: Some(action.action.provider.clone()),
                        actor: resolved.expected_actor().clone(),
                        skill: match resolved.provider().exposure() {
                            ProviderExposure::Skill { key, .. } => Some(key.clone()),
                            _ => None,
                        },
                        entity: ConcreteEntity::Action(action.clone()),
                    });
                    (status, context)
                }
            };
            result.push(match context {
                Some(context) => self.context_gates(&context)?,
                None if status == SelectorBindingStatus::Unavailable => vec![PendingRead::Ready(
                    ReadBinding::Constant(Some(ParameterValue::Boolean(false))),
                )],
                None => vec![missing(PlanGapReason::UnresolvedTopology)],
            });
        }
        Ok(result)
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
                        assigned_skill: None,
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
                assigned_skill: None,
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
            let selected_sources =
                self.bind_source_selectors(&action, skill.as_ref(), routing, &mut sources)?;
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
                    ActionStatRouteSource::Selected { selector, stats } => {
                        let bound = selected_sources.get(selector).ok_or_else(|| {
                            PlanError::Invalid(
                                "selected route has no matching source selector".into(),
                            )
                        })?;
                        self.selected_source_read(&action, bound, stats)?
                    }

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
                    assigned_skill: None,
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
            let source = resolve(
                source,
                FinalReadSources {
                    values: &self.values,
                    contributions: &self.contributions,
                    transforms: &self.transforms,
                },
                self.gaps.is_empty(),
                &mut self.work,
                &mut self.binding_edges,
                self.limits.max_edges,
            )?;
            self.effects[index].operation = match self.effects[index].operation {
                EffectOperation::SelectSource { .. } => EffectOperation::SelectSource { source },
                _ => EffectOperation::Route { source },
            };
        }
        Ok(())
    }
}
fn bind_completeness(read: &mut ReadBinding, complete: bool, work: &mut usize) -> Result<()> {
    charge(work, 1)?;
    match read {
        ReadBinding::Final {
            complete: value, ..
        }
        | ReadBinding::Reduction {
            complete: value, ..
        } => *value = complete,
        ReadBinding::ModifierTransforms {
            initial,
            complete: value,
            ..
        } => {
            *value = complete;
            bind_completeness(initial, complete, work)?;
        }
        ReadBinding::Present { source } => bind_completeness(source, complete, work)?,
        ReadBinding::Select {
            when_true,
            when_false,
            ..
        } => {
            bind_completeness(when_true, complete, work)?;
            bind_completeness(when_false, complete, work)?;
        }
        _ => {}
    }
    Ok(())
}
pub(super) fn read_dependencies(
    read: &ReadBinding,
    out: &mut BTreeSet<usize>,
    work: &mut usize,
) -> Result<()> {
    charge(work, 1)?;
    match read {
        ReadBinding::Select {
            decision,
            when_true,
            when_false,
        } => {
            out.insert(*decision);
            read_dependencies(when_true, out, work)?;
            read_dependencies(when_false, out, work)?;
        }
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
        ReadBinding::ModifierTransforms { initial, steps, .. } => {
            read_dependencies(initial, out, work)?;
            charge(work, steps.len())?;
            out.extend(steps.iter().map(|step| step.effect));
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
            EffectOperation::Route { source } | EffectOperation::SelectSource { source } => {
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
