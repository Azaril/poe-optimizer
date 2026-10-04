//! Query-independent exact source/recipient applications in the shared effect DAG.
use super::*;

type GroupKey = (ConcreteEntity, OwnedDefinitionKey, OwnedDefinitionKey);
struct Group {
    key: ContributionKey,
    candidates: Vec<usize>,
    applications: BTreeSet<OwnedDefinitionKey>,
}
impl<I: DefinitionSchemaIndex> Builder<'_, I> {
    pub(super) fn effect_applications(&mut self) -> Result<()> {
        let rules = self.rules;
        let Some(registry) = &rules.input().effect_applications else {
            return Ok(());
        };
        if !registry.is_complete() {
            self.gap(None, None, PlanGapReason::PartialEffectApplications)?;
        }
        charge(&mut self.work, registry.members.len())?;
        let mut groups: BTreeMap<GroupKey, Group> = BTreeMap::new();
        for row in &registry.members {
            let sources = self.application_sources(&row.source)?;
            let recipients = self.application_recipients(&row.targets)?;
            let pairs = sources
                .len()
                .checked_mul(recipients.len())
                .ok_or(PlanError::Limit("application pairs"))?;
            charge(&mut self.work, pairs)?;
            if pairs
                > self
                    .limits
                    .max_invocations
                    .saturating_sub(self.invocations.len())
            {
                return Err(PlanError::Limit("application pairs"));
            }
            let candidate_count = pairs
                .checked_mul(row.program.effects.len())
                .ok_or(PlanError::Limit("application candidates"))?;
            if candidate_count > self.limits.max_effects.saturating_sub(self.effects.len()) {
                return Err(PlanError::Limit("application candidates"));
            }
            for recipient in &recipients {
                for mapping in &row.stacking {
                    charge(&mut self.work, row.program.effects.len() + 1)?;
                    let effect = row
                        .program
                        .effects
                        .iter()
                        .find(|effect| effect.id == mapping.effect)
                        .ok_or_else(|| {
                            PlanError::Invalid("missing validated application effect".into())
                        })?;
                    let RuleEffectKind::Contribute {
                        stat, contribution, ..
                    } = &effect.effect
                    else {
                        return Err(PlanError::Invalid(
                            "application effect is not a contribution".into(),
                        ));
                    };
                    let key = (
                        recipient.entity.clone(),
                        mapping.family.clone(),
                        mapping.modifier.clone(),
                    );
                    if !groups.contains_key(&key) && groups.len() >= self.limits.max_effects {
                        return Err(PlanError::Limit("application groups"));
                    }
                    groups
                        .entry(key)
                        .or_insert_with(|| Group {
                            key: ContributionKey {
                                entity: recipient.entity.clone(),
                                stat: stat.clone(),
                                kind: *contribution,
                            },
                            candidates: vec![],
                            applications: BTreeSet::new(),
                        })
                        .applications
                        .insert(row.id.clone());
                }
                for source in &sources {
                    self.application(row, source, recipient, &mut groups)?;
                }
            }
        }
        for ((recipient, family, modifier), group) in groups {
            let origin = RuleOrigin::EffectApplicationGroup {
                recipient: recipient.clone(),
                family: family.clone(),
                modifier: modifier.clone(),
            };
            let key = EffectOccurrenceKey {
                invocation: ProgramOccurrenceKey {
                    origin,
                    owner: SchemaSubject::Definition(group.key.stat.address()),
                    program: family,
                    entity: recipient,
                },
                effect: modifier,
            };
            self.add_effect(
                EffectNode {
                    key,
                    target: BoundEffectTarget::Contribution { key: group.key },
                    operation: EffectOperation::ApplicationMaximum {
                        candidates: group.candidates,
                        applications: group.applications.into_iter().collect(),
                        complete: false,
                    },
                    gates: vec![],
                    dependencies: vec![],
                },
                vec![],
            )?;
        }
        Ok(())
    }

    fn application_sources(&mut self, source: &EffectApplicationSource) -> Result<Vec<Context>> {
        let mut contexts = Vec::new();
        match source {
            EffectApplicationSource::Skill { skill: definition } => {
                charge(
                    &mut self.work,
                    self.request.build().input().skills.len() + self.skill_supplies.len(),
                )?;
                let mut targets: BTreeSet<_> = self
                    .request
                    .build()
                    .input()
                    .skills
                    .iter()
                    .filter(|row| matches!(row.source, AuthoredSkillSource::Direct(_)))
                    .map(|row| SkillTarget::Authored(row.id))
                    .collect();
                targets.extend(
                    self.skill_supplies
                        .keys()
                        .cloned()
                        .map(|row| SkillTarget::Generated(Box::new(row))),
                );
                if targets.len() > self.limits.max_providers {
                    return Err(PlanError::Limit("application sources"));
                }
                for target in targets {
                    let resolved = self.resolver.skill(&target)?;
                    charge(&mut self.work, resolved.work_used() + 1)?;
                    let status = resolved.status();
                    let Some(resolved) = resolved.into_value() else {
                        if status != SelectorBindingStatus::Unavailable {
                            self.gap(None, None, PlanGapReason::UnresolvedTopology)?;
                        }
                        continue;
                    };
                    if resolved.definition() != Some(definition) {
                        continue;
                    }
                    let (provider, skill) = match &target {
                        SkillTarget::Authored(id) => (root(ProviderRoot::SkillUse(*id)), None),
                        SkillTarget::Generated(skill) => (
                            self.skill_supplies
                                .get(skill.as_ref())
                                .expect("discovered skill")
                                .clone(),
                            Some(skill.as_ref().clone()),
                        ),
                    };
                    contexts.push(Context {
                        property_owner: None,
                        origin: RuleOrigin::Provider {
                            provider: provider.clone(),
                        },
                        provider: Some(provider),
                        actor: resolved.provider().actor().clone(),
                        skill,
                        receiving_skill: None,
                        assigned_skill: None,
                        entity: ConcreteEntity::Skill(Box::new(target)),
                    });
                }
            }
            EffectApplicationSource::OwnedSlot { slot } => {
                charge(&mut self.work, self.receiver_actors.len())?;
                let actors: Vec<_> = self
                    .receiver_actors
                    .iter()
                    .filter(|actor| matches!(actor, ActorKey::Owned(key) if key.slot == *slot))
                    .cloned()
                    .collect();
                for actor in actors {
                    let (status, context) = self.actor_context(&actor)?;
                    if let Some(context) = context {
                        contexts.push(context);
                    } else if status != SelectorBindingStatus::Unavailable {
                        self.gap(None, None, PlanGapReason::UnresolvedTopology)?;
                    }
                }
            }
        }
        Ok(contexts)
    }

    fn application_recipients(
        &mut self,
        targets: &[EffectApplicationTarget],
    ) -> Result<Vec<Context>> {
        let mut result = Vec::new();
        for target in targets {
            charge(&mut self.work, 1)?;
            if matches!(target, EffectApplicationTarget::Enemy) {
                result.push(Context {
                    property_owner: None,
                    origin: RuleOrigin::Encounter,
                    provider: None,
                    actor: ActorKey::Player,
                    skill: None,
                    receiving_skill: None,
                    assigned_skill: None,
                    entity: ConcreteEntity::Enemy,
                });
                continue;
            }
            let actors = match target {
                EffectApplicationTarget::Player => vec![ActorKey::Player],
                EffectApplicationTarget::OwnedSlot { slot } => {
                    charge(&mut self.work, self.receiver_actors.len())?;
                    self.receiver_actors
                        .iter()
                        .filter(|actor| matches!(actor, ActorKey::Owned(key) if key.slot == *slot))
                        .cloned()
                        .collect()
                }
                EffectApplicationTarget::Enemy => unreachable!(),
            };
            for actor in actors {
                let (status, context) = self.actor_context(&actor)?;
                if let Some(context) = context {
                    result.push(context);
                } else if status != SelectorBindingStatus::Unavailable {
                    self.gap(None, None, PlanGapReason::UnresolvedTopology)?;
                }
                if result.len() > self.limits.max_providers {
                    return Err(PlanError::Limit("application recipients"));
                }
            }
        }
        Ok(result)
    }

    fn application_read(
        &mut self,
        read: &RuleReadSource,
        source: &Context,
        recipient: &Context,
        owner: &SchemaSubject,
    ) -> Result<PendingRead> {
        let mapped = match read {
            RuleReadSource::EffectSourceParameter { slot } => {
                Some(RuleReadSource::Parameter { slot: slot.clone() })
            }
            RuleReadSource::EffectSourceChoice { slot } => {
                Some(RuleReadSource::Choice { slot: slot.clone() })
            }
            RuleReadSource::Stat {
                entity: RuleEntity::EffectSource,
                stat,
            } => Some(RuleReadSource::Stat {
                entity: RuleEntity::Current,
                stat: stat.clone(),
            }),
            RuleReadSource::Capability {
                entity: RuleEntity::EffectSource,
                capability,
            } => Some(RuleReadSource::Capability {
                entity: RuleEntity::Current,
                capability: capability.clone(),
            }),
            RuleReadSource::External {
                entity: RuleEntity::EffectSource,
                input,
            } => Some(RuleReadSource::External {
                entity: RuleEntity::Current,
                input: input.clone(),
            }),
            RuleReadSource::Contributions {
                entity: RuleEntity::EffectSource,
                stat,
                contribution,
                reduction,
                empty,
            } => Some(RuleReadSource::Contributions {
                entity: RuleEntity::Current,
                stat: stat.clone(),
                contribution: *contribution,
                reduction: *reduction,
                empty: empty.clone(),
            }),
            _ => None,
        };
        if let Some(mapped) = mapped {
            self.read(&mapped, source, owner)
        } else {
            self.read(read, recipient, owner)
        }
    }

    fn application(
        &mut self,
        row: &EffectApplicationRule,
        source: &Context,
        recipient: &Context,
        groups: &mut BTreeMap<GroupKey, Group>,
    ) -> Result<()> {
        charge(
            &mut self.work,
            row.program.reads.len() + row.program.effects.len() + 1,
        )?;
        if self.invocations.len() >= self.limits.max_invocations {
            return Err(PlanError::Limit("invocations"));
        }
        let owner = match &row.source {
            EffectApplicationSource::Skill { skill } => SchemaSubject::Definition(skill.address()),
            EffectApplicationSource::OwnedSlot { slot } => {
                SchemaSubject::Slot(ActorSlotDefId::address(slot))
            }
        };
        let key = ProgramOccurrenceKey {
            origin: RuleOrigin::EffectApplication {
                application: row.id.clone(),
                source: source.entity.clone(),
                recipient: recipient.entity.clone(),
            },
            owner: owner.clone(),
            program: row.program.id.clone(),
            entity: recipient.entity.clone(),
        };
        let prepared = self.rules.prepare_effect_application(&row.id)?;
        let authored: BTreeMap<_, _> = row
            .program
            .reads
            .iter()
            .map(|read| (&read.id, &read.source))
            .collect();
        let mut reads = Vec::with_capacity(row.program.reads.len());
        for id in prepared.read_ids() {
            reads.push(self.application_read(authored[id], source, recipient, &owner)?);
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
        let mut gates = self.context_gates(source)?;
        gates.extend(self.context_gates(recipient)?);
        for (effect_index, effect) in row.program.effects.iter().enumerate() {
            charge(&mut self.work, row.stacking.len() + gates.len() + 1)?;
            let mapping = row
                .stacking
                .iter()
                .find(|mapping| mapping.effect == effect.id)
                .expect("validated mapping");
            let group = groups
                .get_mut(&(
                    recipient.entity.clone(),
                    mapping.family.clone(),
                    mapping.modifier.clone(),
                ))
                .expect("registered group");
            let index = self.effects.len();
            self.add_effect(
                EffectNode {
                    key: EffectOccurrenceKey {
                        invocation: key.clone(),
                        effect: effect.id.clone(),
                    },
                    target: BoundEffectTarget::ApplicationCandidate {
                        family: mapping.family.clone(),
                        modifier: mapping.modifier.clone(),
                        key: group.key.clone(),
                    },
                    operation: EffectOperation::Program {
                        invocation,
                        effect: effect_index,
                    },
                    gates: vec![],
                    dependencies: vec![],
                },
                gates.clone(),
            )?;
            group.candidates.push(index);
        }
        Ok(())
    }
}
