//! Exact source-owner discovery and sealed symbolic producer bindings.
//! Neither action demand nor the existence of support assignments discovers a source.
use super::*;
use poe_optimizer_core::owned_source_properties::*;
use poe_optimizer_core::owned_support_receiving::SupportTargetDefinition;

pub(in crate::owned_plan) struct BoundSourceProperties {
    pub(in crate::owned_plan) complete: bool,
    pub(in crate::owned_plan) relations: Vec<BoundSourceRelation>,
    pub(in crate::owned_plan) by_owner: BTreeMap<SkillTarget, Vec<usize>>,
}
impl Default for BoundSourceProperties {
    fn default() -> Self {
        Self {
            complete: true,
            relations: vec![],
            by_owner: BTreeMap::new(),
        }
    }
}

pub(in crate::owned_plan) struct BoundSourceRelation {
    pub(in crate::owned_plan) id: OwnedDefinitionKey,
    pub(in crate::owned_plan) owner: SkillTarget,
    pub(in crate::owned_plan) effects: Vec<BoundSupportAdmission>,
    pub(in crate::owned_plan) count: PlanValueKey,
    pub(in crate::owned_plan) census_stage: OwnedDefinitionKey,
    pub(in crate::owned_plan) complete: bool,
    pub(in crate::owned_plan) external: Vec<BoundSourceProgram>,
    pub(in crate::owned_plan) supports: BTreeMap<SupportAssignmentId, BoundSourceSupport>,
    pub(in crate::owned_plan) assembly: Vec<BoundSourceProgram>,
}
pub(in crate::owned_plan) struct BoundSourceSupport {
    pub(in crate::owned_plan) counted: bool,
    pub(in crate::owned_plan) programs: Vec<BoundSourceProgram>,
}
pub(in crate::owned_plan) struct BoundSourceProgram {
    pub(in crate::owned_plan) owner: SchemaSubject,
    pub(in crate::owned_plan) producer: ProviderKey,
    pub(in crate::owned_plan) entity: ConcreteEntity,
    pub(in crate::owned_plan) program: BoundSupportProgram,
}
impl BoundSourceRelation {
    pub(in crate::owned_plan) fn programs(&self) -> impl Iterator<Item = &BoundSourceProgram> {
        self.external
            .iter()
            .chain(self.supports.values().flat_map(|s| &s.programs))
            .chain(&self.assembly)
    }
}

fn invalid(message: &str) -> PlanError {
    PlanError::Invalid(message.into())
}
fn subject(owner: &SupportTargetDefinition) -> SchemaSubject {
    SchemaSubject::Definition(match owner {
        SupportTargetDefinition::Gem(gem) => gem.address(),
        SupportTargetDefinition::Skill(skill) => skill.address(),
    })
}

pub(super) type SourceProgramKey = (
    Option<DefinitionAddress>,
    Option<SlotAddress>,
    OwnedDefinitionKey,
);
pub(super) fn program_key(owner: &SchemaSubject, program: &OwnedDefinitionKey) -> SourceProgramKey {
    match owner {
        SchemaSubject::Definition(id) => (Some(id.clone()), None, program.clone()),
        SchemaSubject::Slot(id) => (None, Some(id.clone()), program.clone()),
    }
}

impl<'a, I: DefinitionSchemaIndex> Builder<'a, I> {
    pub(super) fn source_properties(
        &mut self,
        owners: &[DeferredOwner],
    ) -> Result<BoundSourceProperties> {
        let Some(input) = self
            .receiving
            .and_then(|p| p.input().source_properties.as_ref())
        else {
            return Ok(BoundSourceProperties::default());
        };
        if !self.operations.supports_source_properties() || self.stages.is_none() {
            return Err(invalid(
                "source properties require checked operation/stage authority",
            ));
        }
        if !input.relations.is_complete() {
            return Err(invalid("source property relations are incomplete"));
        }
        // Declaration authority determines invocation kind, even when no source
        // owner is present in this build. A class/item producer must not fall
        // through to ordinary invocation merely because its relation is empty.
        for row in &input.relations.members {
            charge(&mut self.work, 1)?;
            for external in &row.external.members {
                self.defer_source_program(&external.owner, &external.program)?;
            }
            for support in &row.supports.members {
                let owner = SchemaSubject::Definition(support.gem.address());
                charge(&mut self.work, 1)?;
                for program in &support.programs.members {
                    self.defer_source_program(&owner, program)?;
                }
            }
            let owner = subject(&row.owner);
            for program in &row.assembly.members {
                self.defer_source_program(&owner, program)?;
            }
        }
        let build = self.request.build().input();
        charge(
            &mut self.work,
            build.skills.len() + build.gems.len() + build.supports.len() + owners.len(),
        )?;
        let gems: BTreeMap<_, _> = build.gems.iter().map(|g| (g.id, g)).collect();
        let mut skill_uses = BTreeMap::<GemInstanceId, usize>::new();
        let mut support_uses = BTreeMap::<GemInstanceId, usize>::new();
        for skill in &build.skills {
            if let AuthoredSkillSource::Gem(gem) = &skill.source {
                *skill_uses.entry(*gem).or_default() += 1;
            }
        }
        for support in &build.supports {
            *support_uses.entry(support.support).or_default() += 1;
        }
        let target_gates = self.preparation_gates()?;
        let mut result = BoundSourceProperties::default();
        let mut counts = support_templates::TemplateCounts::default();
        let mut bound_owners = BTreeSet::new();
        for row in &input.relations.members {
            charge(&mut self.work, build.skills.len() + 1)?;
            for skill in &build.skills {
                let declaration = match &skill.source {
                    AuthoredSkillSource::Direct(id) => SupportTargetDefinition::Skill(id.clone()),
                    AuthoredSkillSource::Gem(id) => SupportTargetDefinition::Gem(
                        gems.get(id)
                            .ok_or_else(|| invalid("source owner has no exact Gem"))?
                            .definition
                            .clone(),
                    ),
                };
                if declaration != row.owner {
                    continue;
                }
                if let AuthoredSkillSource::Gem(gem) = &skill.source
                    && (skill_uses.get(gem) != Some(&1) || support_uses.contains_key(gem))
                {
                    return Err(invalid(
                        "source property owner has ambiguous backing-Gem aliases",
                    ));
                }
                if result.relations.len() >= self.limits.max_owner_bindings {
                    return Err(PlanError::Limit("source property relations"));
                }
                let target = SkillTarget::Authored(skill.id);
                if !bound_owners.insert(target.clone()) {
                    return Err(invalid("competing source relations for one input owner"));
                }
                let provider = root(ProviderRoot::SkillUse(skill.id));
                let resolved = self.resolver.skill(&target)?;
                charge(&mut self.work, resolved.work_used() + 1)?;
                if resolved.schema() == SchemaBindingStatus::Invalid {
                    return Err(invalid("source property owner has invalid bindings"));
                }
                let inactive = resolved.status() == SelectorBindingStatus::Unavailable;
                let complete = inactive
                    || (resolved.schema() == SchemaBindingStatus::Valid
                        && resolved.value().is_some());
                let owner_gates = target_gates
                    .get(&target)
                    .ok_or_else(|| invalid("source owner has no readiness gates"))?;
                let mut bound = BoundSourceRelation {
                    id: row.id.clone(),
                    owner: target.clone(),
                    effects: vec![],
                    count: PlanValueKey::Stat {
                        entity: ConcreteEntity::Skill(Box::new(target.clone())),
                        stat: row.non_hidden_count.clone(),
                    },
                    census_stage: row.census_stage.clone(),
                    complete,
                    external: vec![],
                    supports: BTreeMap::new(),
                    assembly: vec![],
                };
                charge(&mut self.work, row.effects.members.len())?;
                for effect in &row.effects.members {
                    // Inactive roots have no discovered children. Their validated
                    // declarations still bind every potential producer below.
                    if inactive {
                        continue;
                    }
                    let (effect_target, entered) = match &effect.endpoint {
                        SourcePropertyEffectEndpoint::DirectOwner {} => {
                            (target.clone(), provider.clone())
                        }
                        SourcePropertyEffectEndpoint::Generated { path, skill_supply } => {
                            let declaring = self.receiving_path(&provider, path)?;
                            let key = GeneratedSkillKey {
                                provider: declaring,
                                slot: skill_supply.clone(),
                            };
                            let Some(entered) = self.skill_supplies.get(&key).cloned() else {
                                bound.complete = false;
                                self.gap(
                                    Some(key.provider),
                                    Some(subject(&row.owner)),
                                    PlanGapReason::UnresolvedTopology,
                                )?;
                                continue;
                            };
                            (SkillTarget::Generated(Box::new(key)), entered)
                        }
                    };
                    let resolution = self.resolver.skill(&effect_target)?;
                    charge(&mut self.work, resolution.work_used() + 1)?;
                    if resolution.schema() == SchemaBindingStatus::Invalid {
                        return Err(invalid("source effect has invalid bindings"));
                    }
                    if resolution.schema() != SchemaBindingStatus::Valid
                        || resolution.value().is_none()
                    {
                        bound.complete = false;
                        self.gap(
                            Some(entered),
                            Some(subject(&row.owner)),
                            PlanGapReason::UnresolvedTopology,
                        )?;
                        continue;
                    }
                    if resolution
                        .value()
                        .is_some_and(|s| s.provider().actor() != &ActorKey::Player)
                    {
                        return Err(invalid(
                            "source effect is outside the declared Player context",
                        ));
                    }
                    if let Some(admission) = self.receiving_admission(
                        &target,
                        &provider,
                        &entered,
                        Some(&effect_target),
                        &effect.admission,
                        &mut bound.complete,
                    )? {
                        charge(&mut self.work, bound.effects.len())?;
                        if bound.effects.contains(&admission) {
                            return Err(invalid("duplicate concrete source effect admission"));
                        }
                        bound.effects.push(admission);
                    }
                }
                for external in &row.external.members {
                    charge(&mut self.work, owners.len() + 1)?;
                    for occurrence in owners.iter().filter(|o| o.subject == external.owner) {
                        if occurrence.actor != ActorKey::Player {
                            return Err(invalid(
                                "external source producer is outside the Player context",
                            ));
                        }
                        let program =
                            self.source_program_definition(&external.owner, &external.program)?;
                        let entity = match program.context {
                            RuleEntityKind::Actor => ConcreteEntity::Actor(ActorKey::Player),
                            RuleEntityKind::EquipmentUse => match (
                                &occurrence.provider.root,
                                occurrence.provider.grant_path.is_empty(),
                            ) {
                                (
                                    ProviderRoot::EquipmentUse(id)
                                    | ProviderRoot::ItemModifier {
                                        equipment_use: id, ..
                                    },
                                    true,
                                ) => ConcreteEntity::EquipmentUse(*id),
                                _ => {
                                    return Err(invalid(
                                        "external property requires an exact equipment producer",
                                    ));
                                }
                            },
                            _ => {
                                return Err(invalid(
                                    "unsupported source external producer context",
                                ));
                            }
                        };
                        let context = Context {
                            property_owner: Some(target.clone()),
                            origin: RuleOrigin::Provider {
                                provider: occurrence.provider.clone(),
                            },
                            provider: Some(occurrence.provider.clone()),
                            actor: occurrence.actor.clone(),
                            skill: occurrence.skill.clone(),
                            receiving_skill: None,
                            assigned_skill: None,
                            entity,
                        };
                        bound.external.push(self.source_program(
                            row,
                            &external.owner,
                            program,
                            &context,
                            owner_gates,
                            &mut counts,
                        )?);
                    }
                }
                charge(&mut self.work, build.supports.len())?;
                for assignment in build.supports.iter().filter(|a| a.target == target) {
                    let gem = gems
                        .get(&assignment.support)
                        .ok_or_else(|| invalid("source support has no exact Gem"))?;
                    if support_uses.get(&assignment.support) != Some(&1)
                        || skill_uses.contains_key(&assignment.support)
                    {
                        return Err(invalid("source support has ambiguous backing-Gem aliases"));
                    }
                    charge(&mut self.work, row.supports.members.len() + 1)?;
                    let Some(declared) = row
                        .supports
                        .members
                        .iter()
                        .find(|s| s.gem == gem.definition)
                    else {
                        bound.complete = false;
                        self.gap(
                            Some(root(ProviderRoot::SupportAssignment(assignment.id))),
                            Some(SchemaSubject::Definition(gem.definition.address())),
                            PlanGapReason::IncompleteContributors,
                        )?;
                        continue;
                    };
                    let owner = SchemaSubject::Definition(gem.definition.address());
                    let origin = root(ProviderRoot::SupportAssignment(assignment.id));
                    let context = Context {
                        property_owner: Some(target.clone()),
                        origin: RuleOrigin::Provider {
                            provider: origin.clone(),
                        },
                        provider: Some(origin),
                        actor: ActorKey::Player,
                        skill: None,
                        receiving_skill: None,
                        assigned_skill: Some(target.clone()),
                        entity: ConcreteEntity::SupportOrigin(SupportOrigin::Assignment(
                            assignment.id,
                        )),
                    };
                    let mut support = BoundSourceSupport {
                        counted: declared.counted,
                        programs: vec![],
                    };
                    charge(&mut self.work, declared.programs.members.len())?;
                    for name in &declared.programs.members {
                        let program = self.source_program_definition(&owner, name)?;
                        if program.context != RuleEntityKind::SupportOrigin {
                            return Err(invalid(
                                "source support program must retain its origin context",
                            ));
                        }
                        support.programs.push(self.source_program(
                            row,
                            &owner,
                            program,
                            &context,
                            owner_gates,
                            &mut counts,
                        )?);
                    }
                    bound.supports.insert(assignment.id, support);
                }
                let owner = subject(&row.owner);
                charge(&mut self.work, row.assembly.members.len())?;
                for name in &row.assembly.members {
                    let program = self.source_program_definition(&owner, name)?;
                    let entity = match program.context {
                        RuleEntityKind::Actor => ConcreteEntity::Actor(ActorKey::Player),
                        RuleEntityKind::Skill => ConcreteEntity::Skill(Box::new(target.clone())),
                        _ => return Err(invalid("unsupported source assembly context")),
                    };
                    let context = Context {
                        property_owner: Some(target.clone()),
                        origin: RuleOrigin::Provider {
                            provider: provider.clone(),
                        },
                        provider: Some(provider.clone()),
                        actor: ActorKey::Player,
                        skill: None,
                        receiving_skill: None,
                        assigned_skill: None,
                        entity,
                    };
                    bound.assembly.push(self.source_program(
                        row,
                        &owner,
                        program,
                        &context,
                        owner_gates,
                        &mut counts,
                    )?);
                }
                charge(&mut self.work, 1)?;
                result
                    .by_owner
                    .entry(target)
                    .or_default()
                    .push(result.relations.len());
                result.relations.push(bound);
            }
        }
        Ok(result)
    }

    fn source_program_definition(
        &mut self,
        owner: &SchemaSubject,
        name: &OwnedDefinitionKey,
    ) -> Result<&'a RuleProgram> {
        charge(&mut self.work, self.rules.input().owners.len() + 1)?;
        let rules: &'a CompiledRulePackage = self.rules;
        let row = rules
            .input()
            .owners
            .iter()
            .find(|o| &o.owner == owner)
            .ok_or_else(|| invalid("source producer owner is missing"))?;
        if !row.programs.is_complete() {
            return Err(invalid("source producer owner programs are incomplete"));
        }
        charge(&mut self.work, row.programs.members.len())?;
        row.programs
            .members
            .iter()
            .find(|p| &p.id == name)
            .ok_or_else(|| invalid("source producer program is missing"))
    }

    fn source_program(
        &mut self,
        relation: &SourcePropertyRelation,
        owner: &SchemaSubject,
        program: &RuleProgram,
        context: &Context,
        owner_gates: &[PendingRead],
        counts: &mut support_templates::TemplateCounts,
    ) -> Result<BoundSourceProgram> {
        charge(&mut self.work, program.reads.len() + program.effects.len())?;
        for read in &program.reads {
            match &read.source {
                RuleReadSource::Stat {
                    entity: RuleEntity::PropertyOwner,
                    stat,
                } => {
                    charge(&mut self.work, relation.inputs.len() + 1)?;
                    if stat != &relation.non_hidden_count && !relation.inputs.contains(stat) {
                        return Err(invalid(
                            "property-owner stat read is not declared by this relation",
                        ));
                    }
                }
                RuleReadSource::OrderedContributions {
                    entity: RuleEntity::PropertyOwner,
                    query,
                    ..
                } => {
                    let registry = self
                        .rules
                        .input()
                        .ordered_contributions
                        .as_ref()
                        .ok_or_else(|| invalid("ordered contribution registry is absent"))?;
                    charge(
                        &mut self.work,
                        registry.members.len() + relation.channels.members.len(),
                    )?;
                    let query = registry
                        .members
                        .iter()
                        .find(|row| &row.id == query)
                        .ok_or_else(|| invalid("ordered contribution query is absent"))?;
                    if !relation
                        .channels
                        .members
                        .iter()
                        .any(|c| c.stat == query.stat && c.contribution == query.contribution)
                    {
                        return Err(invalid(
                            "property-owner ordered contribution read is not declared",
                        ));
                    }
                }
                RuleReadSource::Contributions {
                    entity: RuleEntity::PropertyOwner,
                    stat,
                    contribution,
                    ..
                } => {
                    charge(&mut self.work, relation.channels.members.len())?;
                    if !relation
                        .channels
                        .members
                        .iter()
                        .any(|c| &c.stat == stat && &c.contribution == contribution)
                    {
                        return Err(invalid("property-owner contribution read is not declared"));
                    }
                }
                RuleReadSource::External {
                    entity: RuleEntity::PropertyOwner,
                    ..
                }
                | RuleReadSource::Capability {
                    entity: RuleEntity::PropertyOwner,
                    ..
                } => return Err(invalid("unsupported property-owner read")),
                _ => {}
            }
        }
        let producer = context
            .provider
            .clone()
            .ok_or_else(|| invalid("source program has no exact producer"))?;
        let resolution = self.resolver.provider(&producer)?;
        charge(&mut self.work, resolution.work_used() + 1)?;
        let mut gates = if resolution.status() == SelectorBindingStatus::Unavailable {
            vec![PendingRead::Ready(ReadBinding::Constant(Some(
                ParameterValue::Boolean(false),
            )))]
        } else {
            self.program_gates(owner, &program.id, context)?
        };
        charge(&mut self.work, owner_gates.len())?;
        gates.extend_from_slice(owner_gates);
        let bound = self.support_program(program, owner, context, &gates, counts)?;
        charge(&mut self.work, producer.grant_path.len() + 1)?;
        Ok(BoundSourceProgram {
            owner: owner.clone(),
            producer,
            entity: context.entity.clone(),
            program: bound,
        })
    }

    fn defer_source_program(
        &mut self,
        owner: &SchemaSubject,
        program: &OwnedDefinitionKey,
    ) -> Result<()> {
        charge(&mut self.work, 1)?;
        let classification = program_key(owner, program);
        if self.deferred_source_programs.len() >= self.limits.max_owner_bindings
            && !self.deferred_source_programs.contains(&classification)
        {
            return Err(PlanError::Limit("source program classifications"));
        }
        self.deferred_source_programs.insert(classification);
        Ok(())
    }
}
