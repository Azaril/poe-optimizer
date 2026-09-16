//! Cold binding of shared action source decisions. Source absence is authored
//! policy, never inferred from a missing numerical producer.
use super::*;

pub(super) struct BoundSourceSelector {
    decision: usize,
    when_true: Option<NamedActionSource>,
    when_false: Option<NamedActionSource>,
    equipment: Option<ItemSlotUseId>,
}
fn applies(selection: &ActionRouteSelection, action: &ActionSelection) -> bool {
    match selection {
        ActionRouteSelection::All => true,
        ActionRouteSelection::Exact(s) => {
            s.part == action.part && s.mode == action.mode && s.stat_set == action.stat_set
        }
    }
}
fn named(selector: &ActionSourceSelector, id: &OwnedDefinitionKey) -> Result<NamedActionSource> {
    selector
        .sources
        .iter()
        .find(|s| &s.id == id)
        .cloned()
        .ok_or_else(|| PlanError::Invalid("source policy references an absent named source".into()))
}
fn alternative(
    selector: &ActionSourceSelector,
    outcome: &ActionSourceOutcome,
) -> Result<Option<NamedActionSource>> {
    match outcome {
        ActionSourceOutcome::Use { source } => Ok(Some(named(selector, source)?)),
        ActionSourceOutcome::Unavailable => Ok(None),
    }
}
impl<I: DefinitionSchemaIndex> Builder<'_, I> {
    pub(super) fn bind_source_selectors(
        &mut self,
        action: &ActionSelection,
        skill: Option<&GeneratedSkillKey>,
        routing: &OwnedActionRouting,
        pending: &mut Vec<(usize, PendingRead)>,
    ) -> Result<BTreeMap<OwnedDefinitionKey, BoundSourceSelector>> {
        let mut bound = BTreeMap::new();
        let Some(selectors) = routing.source_selectors_for(&action.action.output) else {
            return Ok(bound);
        };
        charge(&mut self.work, selectors.members.len())?;
        if selectors.members.len() > self.limits.max_effects {
            return Err(PlanError::Limit("source selectors"));
        }
        if !selectors.is_complete() {
            self.gap(
                Some(action.action.provider.clone()),
                Some(SchemaSubject::Slot(ActionOutputDefId::address(
                    &action.action.output,
                ))),
                PlanGapReason::PartialRouting,
            )?;
        }
        for selector in &selectors.members {
            if !applies(&selector.selection, action) {
                continue;
            }
            charge(&mut self.work, selector.sources.len() + 1)?;
            let (source, when_true, when_false, equipment) = match &selector.policy {
                ActionSourcePolicy::Fixed { source } => (
                    PendingRead::Ready(ReadBinding::Constant(Some(ParameterValue::Boolean(true)))),
                    Some(named(selector, source)?),
                    None,
                    None,
                ),
                ActionSourcePolicy::EquipmentEligibility {
                    source,
                    capability,
                    when_empty,
                    when_ineligible,
                } => {
                    let primary = named(selector, source)?;
                    let ActionSourceOrigin::PlayerEquipment { slot } = &primary.origin else {
                        return Err(PlanError::Invalid(
                            "equipment selector has non-equipment origin".into(),
                        ));
                    };
                    let input = self.request.build().input();
                    charge(&mut self.work, input.equipment.len())?;
                    let mut occupied = None;
                    let mut ambiguous = false;
                    // Inspect authored occupancy before consulting discovered providers.
                    // An unresolved provider must not disappear into the empty branch.
                    for row in &input.equipment {
                        if row.destination != EquipmentDestination::CharacterSlot(slot.clone()) {
                            continue;
                        }
                        let active = match &row.scope {
                            LoadoutScope::Shared => true,
                            LoadoutScope::Selected { loadouts } => {
                                charge(&mut self.work, loadouts.len())?;
                                loadouts.contains(&input.active_weapon_loadout)
                            }
                        };
                        if active && occupied.replace(row.id).is_some() {
                            ambiguous = true;
                        }
                    }
                    if ambiguous {
                        (
                            missing(PlanGapReason::UnsupportedRelation),
                            Some(primary),
                            alternative(selector, when_ineligible)?,
                            None,
                        )
                    } else if let Some(equipment) = occupied {
                        let provider = root(ProviderRoot::EquipmentUse(equipment));
                        let resolution = self.resolver.provider(&provider)?;
                        charge(&mut self.work, resolution.work_used())?;
                        let source = if resolution.into_value().is_some() {
                            PendingRead::Value(PlanValueKey::Capability {
                                entity: ConcreteEntity::EquipmentUse(equipment),
                                capability: capability.clone(),
                            })
                        } else {
                            missing(PlanGapReason::UnresolvedTopology)
                        };
                        (
                            source,
                            Some(primary),
                            alternative(selector, when_ineligible)?,
                            Some(equipment),
                        )
                    } else {
                        (
                            PendingRead::Ready(ReadBinding::Constant(Some(
                                ParameterValue::Boolean(true),
                            ))),
                            alternative(selector, when_empty)?,
                            None,
                            None,
                        )
                    }
                }
            };
            let context = Context {
                origin: RuleOrigin::SourceSelection {
                    action: Box::new(action.clone()),
                    selector: selector.id.clone(),
                },
                provider: Some(action.action.provider.clone()),
                actor: action.action.actor.clone(),
                skill: skill.cloned(),
                entity: ConcreteEntity::Action(Box::new(action.clone())),
            };
            let decision = self.effects.len();
            let gates = self.context_gates(&context)?;
            self.add_effect(
                EffectNode {
                    key: EffectOccurrenceKey {
                        invocation: ProgramOccurrenceKey {
                            origin: context.origin.clone(),
                            owner: SchemaSubject::Slot(ActionOutputDefId::address(
                                &action.action.output,
                            )),
                            program: selector.id.clone(),
                            entity: context.entity.clone(),
                        },
                        effect: selector.id.clone(),
                    },
                    target: BoundEffectTarget::SourceSelection {
                        action: Box::new(action.clone()),
                        selector: selector.id.clone(),
                    },
                    operation: EffectOperation::SelectSource {
                        source: ReadBinding::Missing(PlanGapReason::MissingProducer),
                    },
                    gates: vec![],
                    dependencies: vec![],
                },
                gates,
            )?;
            pending.push((decision, source));
            if bound
                .insert(
                    selector.id.clone(),
                    BoundSourceSelector {
                        decision,
                        when_true,
                        when_false,
                        equipment,
                    },
                )
                .is_some()
            {
                return Err(PlanError::Invalid("duplicate bound source selector".into()));
            }
        }
        Ok(bound)
    }
    pub(super) fn selected_source_read(
        &mut self,
        action: &ActionSelection,
        selector: &BoundSourceSelector,
        stats: &[ActionSourceStat],
    ) -> Result<PendingRead> {
        charge(&mut self.work, stats.len() + 3)?;
        let branch = |source: Option<&NamedActionSource>| -> Result<PendingRead> {
            let Some(source) = source else {
                return Ok(PendingRead::Ready(ReadBinding::Inactive));
            };
            let stat = stats
                .iter()
                .find(|s| s.source == source.id)
                .ok_or_else(|| {
                    PlanError::Invalid("selected source lacks its explicit stat binding".into())
                })?;
            let entity = match &source.origin {
                ActionSourceOrigin::ActionActor => {
                    ConcreteEntity::Actor(action.action.actor.clone())
                }
                ActionSourceOrigin::CurrentAction => {
                    ConcreteEntity::Action(Box::new(action.clone()))
                }
                ActionSourceOrigin::PlayerEquipment { .. } => match selector.equipment {
                    Some(id) => ConcreteEntity::EquipmentUse(id),
                    None => return Ok(missing(PlanGapReason::UnresolvedTopology)),
                },
            };
            Ok(PendingRead::Value(PlanValueKey::Stat {
                entity,
                stat: stat.stat.clone(),
            }))
        };
        Ok(PendingRead::Select {
            decision: selector.decision,
            when_true: Box::new(branch(selector.when_true.as_ref())?),
            when_false: Box::new(branch(selector.when_false.as_ref())?),
        })
    }
}
