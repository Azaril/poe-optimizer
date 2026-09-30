//! Position-independent rule bindings for finite physical assignment/receiver pairs.
//! Selection later instantiates only retained positions; this is not effect delivery.
use super::*;

#[derive(Clone, Debug)]
pub(in crate::owned_plan) struct BoundSupportTemplate {
    pub(in crate::owned_plan) origin: SupportOrigin,
    pub(in crate::owned_plan) context: Arc<BoundSupportReceiverContext>,
    pub(in crate::owned_plan) target: SkillTarget,
    pub(in crate::owned_plan) owner: SchemaSubject,
    pub(in crate::owned_plan) applicability: BoundSupportProgram,
    pub(in crate::owned_plan) delivery: Vec<BoundSupportProgram>,
}
#[derive(Clone, Debug)]
pub(in crate::owned_plan) struct BoundSupportProgram {
    pub(in crate::owned_plan) prepared: PreparedRuleProgram,
    pub(in crate::owned_plan) program: OwnedDefinitionKey,
    pub(in crate::owned_plan) reads: Vec<PendingRead>,
    pub(in crate::owned_plan) read_ids: Vec<OwnedDefinitionKey>,
    pub(in crate::owned_plan) effects: Vec<BoundSupportEffect>,
}
#[derive(Clone, Debug)]
pub(in crate::owned_plan) struct BoundSupportEffect {
    pub(in crate::owned_plan) id: OwnedDefinitionKey,
    pub(in crate::owned_plan) target: BoundEffectTarget,
    pub(in crate::owned_plan) gates: Vec<PendingRead>,
    pub(in crate::owned_plan) effect_index: usize,
}
#[derive(Default)]
struct TemplateCounts {
    programs: usize,
    effects: usize,
    references: usize,
}
fn count(total: &mut usize, n: usize, maximum: usize, label: &'static str) -> Result<()> {
    *total = total
        .checked_add(n)
        .filter(|v| *v <= maximum)
        .ok_or(PlanError::Limit(label))?;
    Ok(())
}
fn depth(target: &SkillTarget) -> usize {
    match target {
        SkillTarget::Authored(_) => 0,
        SkillTarget::Generated(skill) => skill.provider.grant_path.len(),
    }
}
fn effect_target(effect: &RuleEffectKind, context: &Context) -> Result<BoundEffectTarget> {
    Ok(match effect {
        RuleEffectKind::Contribute {
            entity: relative,
            stat,
            contribution,
            ..
        } => BoundEffectTarget::Contribution {
            key: ContributionKey {
                entity: entity(*relative, context)?,
                stat: stat.clone(),
                kind: *contribution,
            },
        },
        RuleEffectKind::Derive {
            entity: relative,
            stat,
            ..
        } => BoundEffectTarget::Value {
            key: PlanValueKey::Stat {
                entity: entity(*relative, context)?,
                stat: stat.clone(),
            },
        },
        RuleEffectKind::Capability {
            entity: relative,
            capability,
            ..
        } => BoundEffectTarget::Value {
            key: PlanValueKey::Capability {
                entity: entity(*relative, context)?,
                capability: capability.clone(),
            },
        },
        RuleEffectKind::Requirement { code, .. } => {
            BoundEffectTarget::Requirement { code: code.clone() }
        }
        RuleEffectKind::SupportApplicability { .. } => BoundEffectTarget::Applicability,
        _ => {
            return Err(PlanError::Invalid(
                "support delivery has an unsupported projection/grant effect".into(),
            ));
        }
    })
}

impl<I: DefinitionSchemaIndex> Builder<'_, I> {
    pub(super) fn support_templates(
        &mut self,
        receiving: &BoundSupportReceiving,
    ) -> Result<BTreeMap<(SupportAssignmentId, SupportReceiverKey), BoundSupportTemplate>> {
        // These gates remain symbolic until every retained application producer exists.
        let target_gates = self.preparation_gates()?;
        charge(
            &mut self.work,
            self.rules.input().owners.len() + receiving.assignments.len(),
        )?;
        for owner in &self.rules.input().owners {
            charge(&mut self.work, owner.programs.members.len())?;
        }
        let programs: BTreeMap<_, BTreeMap<_, _>> = self
            .rules
            .input()
            .owners
            .iter()
            .filter_map(|owner| {
                let SchemaSubject::Definition(DefinitionAddress::Gem(gem)) = &owner.owner else {
                    return None;
                };
                Some((
                    gem,
                    owner
                        .programs
                        .members
                        .iter()
                        .map(|program| (&program.id, program))
                        .collect(),
                ))
            })
            .collect();
        let mut templates = BTreeMap::new();
        let mut counts = TemplateCounts::default();
        for (assignment, binding) in &receiving.assignments {
            charge(&mut self.work, binding.receivers.len() + 1)?;
            if !binding.complete || binding.availability != BoundReceivingAvailability::Bound {
                continue;
            }
            let owner = SchemaSubject::Definition(binding.gem.address());
            let owner_programs = programs.get(&binding.gem).ok_or_else(|| {
                PlanError::Invalid("support template has no exact Gem rule owner".into())
            })?;
            let origin_provider = root(ProviderRoot::SupportAssignment(*assignment));
            let origin = self.resolver.provider(&origin_provider)?;
            charge(&mut self.work, origin.work_used() + 1)?;
            if origin.schema() == SchemaBindingStatus::Invalid {
                return Err(PlanError::Invalid(
                    "support template origin has invalid bindings".into(),
                ));
            }
            let origin_status = origin.status();
            let origin_schema = origin.schema();
            let origin_present = origin.value().is_some();
            for receiver in &binding.receivers {
                if templates.len() >= self.limits.max_owner_bindings {
                    return Err(PlanError::Limit("support templates"));
                }
                charge(
                    &mut self.work,
                    receiver.context.provider.grant_path.len()
                        + depth(&binding.target)
                        + receiver.context.skill.as_ref().map_or(0, depth)
                        + 4,
                )?;
                let (actor, concrete) = match &receiver.context.receiver {
                    SupportReceiverKey::Actor(actor) => {
                        (actor.clone(), ConcreteEntity::Actor(actor.clone()))
                    }
                    SupportReceiverKey::Action(action) => (
                        action.action.actor.clone(),
                        ConcreteEntity::Action(action.clone()),
                    ),
                };
                // Source reads use the assignment. Exact receiver value channels use
                // receiving_skill/actor/entity; never replace the source provider.
                let source = Context {
                    origin: RuleOrigin::Provider {
                        provider: origin_provider.clone(),
                    },
                    provider: Some(origin_provider.clone()),
                    actor: actor.clone(),
                    skill: None,
                    receiving_skill: receiver.context.skill.clone(),
                    assigned_skill: Some(binding.target.clone()),
                    entity: concrete.clone(),
                };
                let mut gates = if origin_status == SelectorBindingStatus::Unavailable {
                    vec![PendingRead::Ready(ReadBinding::Constant(Some(
                        ParameterValue::Boolean(false),
                    )))]
                } else if !origin_present || origin_schema != SchemaBindingStatus::Valid {
                    self.gap(
                        Some(origin_provider.clone()),
                        Some(owner.clone()),
                        PlanGapReason::UnresolvedTopology,
                    )?;
                    vec![missing(PlanGapReason::UnresolvedTopology)]
                } else {
                    self.context_gates(&source)?
                };
                // Resolve the endpoint provider again only to retain generated required
                // inputs on Actor endpoints whose explicit path passes through a skill.
                let resolved = self.resolver.provider(&receiver.context.provider)?;
                charge(&mut self.work, resolved.work_used() + 1)?;
                if resolved.schema() == SchemaBindingStatus::Invalid {
                    return Err(PlanError::Invalid(
                        "support template receiver has invalid bindings".into(),
                    ));
                }
                if let Some(resolved) = resolved.into_value() {
                    let skill = match resolved.exposure() {
                        ProviderExposure::Skill { key, .. } => Some(key.clone()),
                        _ => None,
                    };
                    let receiving = Context {
                        origin: RuleOrigin::Provider {
                            provider: receiver.context.provider.clone(),
                        },
                        provider: Some(receiver.context.provider.clone()),
                        actor,
                        skill,
                        receiving_skill: receiver.context.skill.clone(),
                        assigned_skill: Some(binding.target.clone()),
                        entity: concrete,
                    };
                    gates.extend(self.context_gates(&receiving)?);
                } else {
                    gates.push(missing(PlanGapReason::UnresolvedTopology));
                }
                let assigned = target_gates.get(&binding.target).ok_or_else(|| {
                    PlanError::Invalid(
                        "support template assigned target has no activation gates".into(),
                    )
                })?;
                charge(&mut self.work, assigned.len() + gates.len())?;
                gates.extend_from_slice(assigned);
                let applicability = self.support_program(
                    owner_programs.get(&receiver.applicability).ok_or_else(|| {
                        PlanError::Invalid("unknown support applicability program".into())
                    })?,
                    &owner,
                    &source,
                    &gates,
                    &mut counts,
                )?;
                charge(&mut self.work, receiver.delivery.len())?;
                let mut delivery = Vec::with_capacity(receiver.delivery.len());
                for name in &receiver.delivery {
                    delivery.push(self.support_program(
                        owner_programs.get(name).ok_or_else(|| {
                            PlanError::Invalid("unknown support delivery program".into())
                        })?,
                        &owner,
                        &source,
                        &gates,
                        &mut counts,
                    )?);
                }
                let key = (*assignment, receiver.context.receiver.clone());
                if templates
                    .insert(
                        key,
                        BoundSupportTemplate {
                            origin: binding.origin.clone(),
                            context: Arc::clone(&receiver.context),
                            target: binding.target.clone(),
                            owner: owner.clone(),
                            applicability,
                            delivery,
                        },
                    )
                    .is_some()
                {
                    return Err(PlanError::Invalid(
                        "duplicate support application template".into(),
                    ));
                }
            }
        }
        Ok(templates)
    }
    fn support_program(
        &mut self,
        program: &RuleProgram,
        owner: &SchemaSubject,
        source: &Context,
        gates: &[PendingRead],
        counts: &mut TemplateCounts,
    ) -> Result<BoundSupportProgram> {
        count(
            &mut counts.programs,
            1,
            self.limits.max_invocations,
            "support template programs",
        )?;
        count(
            &mut counts.effects,
            program.effects.len(),
            self.limits.max_effects,
            "support template effects",
        )?;
        let references = gates
            .len()
            .checked_mul(program.effects.len())
            .and_then(|n| n.checked_add(program.reads.len()))
            .ok_or(PlanError::Limit("support template references"))?;
        count(
            &mut counts.references,
            references,
            self.limits.max_edges,
            "support template references",
        )?;
        let path_work = source.provider.as_ref().map_or(0, |p| p.grant_path.len())
            + source.assigned_skill.as_ref().map_or(0, depth)
            + source.receiving_skill.as_ref().map_or(0, depth)
            + 1;
        charge(
            &mut self.work,
            (references + program.effects.len() + 1)
                .checked_mul(path_work)
                .ok_or(PlanError::Limit("support template work"))?,
        )?;
        let prepared = self.rules.prepare_program(owner, &program.id)?;
        let authored: BTreeMap<_, _> = program
            .reads
            .iter()
            .map(|read| (&read.id, &read.source))
            .collect();
        let mut reads = Vec::with_capacity(program.reads.len());
        let mut read_ids = Vec::with_capacity(program.reads.len());
        for id in prepared.read_ids() {
            let read = authored.get(id).ok_or_else(|| {
                PlanError::Invalid("compiled support read is not declared".into())
            })?;
            reads.push(self.read(read, source, owner)?);
            read_ids.push(id.clone());
        }
        let mut effects = Vec::with_capacity(program.effects.len());
        for (effect_index, effect) in program.effects.iter().enumerate() {
            effects.push(BoundSupportEffect {
                id: effect.id.clone(),
                target: effect_target(&effect.effect, source)?,
                gates: gates.to_vec(),
                effect_index,
            });
        }
        Ok(BoundSupportProgram {
            prepared,
            program: program.id.clone(),
            reads,
            read_ids,
            effects,
        })
    }
}
