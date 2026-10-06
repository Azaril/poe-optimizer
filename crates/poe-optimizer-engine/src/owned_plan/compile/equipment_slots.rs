//! One cold authored slot census shared by Player reads and Action routing.
//! Occupancy does not establish attack eligibility or equipment legality.
use super::*;

#[derive(Clone, Copy)]
pub(super) enum EquipmentSlotState {
    Empty,
    Occupied(ItemSlotUseId),
    Unresolved {
        occupant: Option<ItemSlotUseId>,
        reason: PlanGapReason,
    },
}

impl<I: DefinitionSchemaIndex> Builder<'_, I> {
    pub(super) fn equipment_slot(
        &mut self,
        slot: &EquipmentSlotDefId,
    ) -> Result<EquipmentSlotState> {
        charge(&mut self.work, 1)?;
        if let Some(state) = self.equipment_slots.get(slot) {
            return Ok(*state);
        }
        if self.equipment_slots.len() >= self.limits.max_providers {
            return Err(PlanError::Limit("equipment slots"));
        }
        if !matches!(self.index.definition(slot), SchemaLookup::Known(_)) {
            return Err(PlanError::Invalid(
                "equipment slot definition is not known".into(),
            ));
        }
        let input = self.request.build().input();
        charge(&mut self.work, input.equipment.len())?;
        let mut occupant = None;
        let mut ambiguous = false;
        // Inspect every authored active occupant before checking provider success.
        // Otherwise an unknown record or one side of a collision could disappear.
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
            if active && occupant.replace(row.id).is_some() {
                ambiguous = true;
            }
        }
        let state = if ambiguous {
            EquipmentSlotState::Unresolved {
                occupant: None,
                reason: PlanGapReason::UnsupportedRelation,
            }
        } else if let Some(id) = occupant {
            let resolved = self
                .resolver
                .provider(&root(ProviderRoot::EquipmentUse(id)))?;
            charge(&mut self.work, resolved.work_used())?;
            if resolved.schema() == SchemaBindingStatus::Valid && resolved.value().is_some() {
                EquipmentSlotState::Occupied(id)
            } else {
                EquipmentSlotState::Unresolved {
                    occupant: Some(id),
                    reason: PlanGapReason::UnresolvedTopology,
                }
            }
        } else {
            EquipmentSlotState::Empty
        };
        self.equipment_slots.insert(slot.clone(), state);
        Ok(state)
    }

    pub(super) fn player_equipment_read(
        &mut self,
        slot: &EquipmentSlotDefId,
        read: &PlayerEquipmentSlotRead,
        context: &Context,
    ) -> Result<PendingRead> {
        if !self.operations.supports_player_equipment_slots()
            || !matches!(
                context.origin,
                RuleOrigin::ExistingActor {
                    actor: ActorKey::Player,
                    ..
                }
            )
            || context.actor != ActorKey::Player
            || context.entity != ConcreteEntity::Actor(ActorKey::Player)
        {
            return Err(PlanError::Invalid(
                "Player equipment slot read requires an explicit existing Player Actor invocation"
                    .into(),
            ));
        }
        let id = match self.equipment_slot(slot)? {
            EquipmentSlotState::Empty => {
                return Ok(match read {
                    PlayerEquipmentSlotRead::Occupied => PendingRead::Ready(ReadBinding::Constant(
                        Some(ParameterValue::Boolean(false)),
                    )),
                    PlayerEquipmentSlotRead::Stat { .. }
                    | PlayerEquipmentSlotRead::Capability { .. } => {
                        missing(PlanGapReason::MissingProducer)
                    }
                });
            }
            EquipmentSlotState::Occupied(id) => id,
            EquipmentSlotState::Unresolved { reason, .. } => return Ok(missing(reason)),
        };
        Ok(match read {
            PlayerEquipmentSlotRead::Occupied => {
                PendingRead::Ready(ReadBinding::Constant(Some(ParameterValue::Boolean(true))))
            }
            PlayerEquipmentSlotRead::Stat { stat } => PendingRead::Value(PlanValueKey::Stat {
                entity: ConcreteEntity::EquipmentUse(id),
                stat: stat.clone(),
            }),
            PlayerEquipmentSlotRead::Capability { capability } => {
                PendingRead::Value(PlanValueKey::Capability {
                    entity: ConcreteEntity::EquipmentUse(id),
                    capability: capability.clone(),
                })
            }
        })
    }
}
