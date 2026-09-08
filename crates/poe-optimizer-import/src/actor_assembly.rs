//! Shared source assembly and receiving-defence evidence at the import/native seam.
use poe_optimizer_data::{
    class_tree::{PassiveAllocationSelection, ResolvedClassTree},
    game_data::{ActorModifierRecord, GameDataSnapshot},
};
use poe_optimizer_engine::actor::ReceivingOutput;
use serde_json::{Value, json};

/// Resolve legacy optional entrances through the same contextual source views as
/// arbitrary allocations. This does not compose numeric character modifiers.
pub fn legacy_passive_actor_records(
    snapshot: &GameDataSnapshot,
    tree: &ResolvedClassTree,
) -> Result<Vec<ActorModifierRecord>, String> {
    if tree
        .selection
        .resolve(snapshot.tree())
        .map_err(|e| e.to_string())?
        != *tree
    {
        return Err("Legacy resolved tree differs from selected data".into());
    }
    let allocation = PassiveAllocationSelection {
        class_id: tree.selection.class_id,
        ascendancy_id: tree.selection.ascendancy_id.clone(),
        ordinary_nodes: tree.selection.entrance_node_id.into_iter().collect(),
        ascendancy_nodes: tree.selection.ascendancy_node_id.into_iter().collect(),
        attribute_options: Default::default(),
    }
    .resolve(snapshot)
    .map_err(|e| e.to_string())?;
    Ok(allocation
        .views
        .into_iter()
        .flat_map(|view| view.actor_modifiers)
        .collect())
}

/// Stable evidence shape for the shared receiving stage. Source/data identity is
/// recorded by the enclosing native profile, equipment and passive attachments.
pub fn receiving_defence_evidence(output: ReceivingOutput) -> Value {
    let resistance = |values: poe_optimizer_engine::resistance::PlayerResistances| json!({"fire":values.fire,"cold":values.cold,"lightning":values.lightning,"chaos":values.chaos});
    json!({
        "schema_version":1,
        "armour":output.armour,"evasion":output.evasion,"energy_shield":output.energy_shield,
        "resistances":resistance(output.resistances),
        "resistance_totals":resistance(output.resistance_totals),
        "resistance_cap":output.resistance_cap,"resistance_floor":output.resistance_floor,
    })
}

/// Shared movement ratios; the public percentage metric multiplies the effective
/// ratio by 100, with 100 representing the selected baseline movement speed.
pub fn movement_evidence(output: poe_optimizer_engine::movement::MovementOutput) -> Value {
    json!({"schema_version":1,
        "movement_speed_mod":output.movement_speed_mod,
        "action_speed_mod":output.action_speed_mod,
        "effective_movement_speed_mod":output.effective_movement_speed_mod,
        "ignore_movement_penalties":output.ignore_movement_penalties,
        "cannot_be_below_base":output.cannot_be_below_base,
        "has_override":output.has_override})
}

/// Shared action-speed queries retain resolved MAX availability; authored records
/// separately preserve zero values that the source MAX query omits.
pub fn action_speed_evidence(output: poe_optimizer_engine::actor::ActionSpeedOutput) -> Value {
    json!({"schema_version":1,
        "action_speed_mod":output.action_speed_mod,
        "minimum_action_speed":output.minimum_action_speed,
        "maximum_action_speed_reduction":output.maximum_action_speed_reduction,
        "action_speed_increased":output.action_speed_increased,
        "temporal_chains_action_speed_increased":output.temporal_chains_action_speed_increased,
        "unaffected_by_slows":output.unaffected_by_slows})
}
/// CastRate includes action speed before the server cap; Speed/Time follow it.
pub fn action_timing_evidence(
    output: poe_optimizer_engine::timing::DirectActionTimingOutput,
) -> Value {
    let mut value = json!({"schema_version":1,
        "speed_multiplier":output.speed_multiplier,"base_time":output.base_time,
        "cast_rate":output.cast_rate,"speed":output.speed,"time":output.time,
        "action_speed_mod":output.action_speed_mod});
    let mut non_finite = std::collections::BTreeMap::new();
    for (name, number) in [
        ("speed_multiplier", output.speed_multiplier),
        ("base_time", output.base_time),
        ("cast_rate", output.cast_rate),
        ("speed", output.speed),
        ("time", output.time),
        ("action_speed_mod", output.action_speed_mod),
    ] {
        if let poe_optimizer_core::metrics::MeasurementValue::NonFinite { kind } =
            poe_optimizer_core::metrics::MeasurementValue::from_number(number)
        {
            non_finite.insert(name, kind);
        }
    }
    value["non_finite_values"] = json!(non_finite);
    value
}
/// Reconstruct timing from exact admitted weapon/support data without evaluating damage.
pub fn mace_action_timing(
    data: &poe_optimizer_engine::CompiledGameData,
    character: &poe_optimizer_engine::character::CharacterInput,
    weapon: &crate::mace_item::ValidatedMaceWeapon,
    supports: &[String],
    action_speed_mod: f64,
) -> Result<poe_optimizer_engine::timing::DirectActionTimingOutput, String> {
    use poe_optimizer_engine::mace::MaceWeapon;
    let slot = match weapon.weapon_key() {
        "wooden_club" => MaceWeapon::WoodenClub,
        "smithing_hammer" => MaceWeapon::SmithingHammer,
        _ => return Err("Unknown native Mace weapon capability slot".into()),
    };
    let prepared = data
        .prepare_mace_weapon(
            slot,
            weapon.quality(),
            weapon.item_level(),
            weapon.local_modifiers(),
        )
        .map_err(|error| error.to_string())?;
    let supports = data
        .mace_support_loadout(supports)
        .map_err(|error| error.to_string())?;
    poe_optimizer_engine::mace::action_timing(
        data,
        character,
        &prepared,
        supports,
        action_speed_mod,
    )
    .map_err(|error| error.to_string())
}

/// Source-bound local armour evidence; no global BASE surrogate is introduced.
pub fn local_armour_evidence<'a>(
    items: impl Iterator<
        Item = (
            &'a str,
            &'a crate::equipment::ValidatedEquipmentItem,
            &'a poe_optimizer_engine::armour::PreparedArmour,
        ),
    >,
) -> Value {
    let items: std::collections::BTreeMap<_, _> = items.map(|(slot,item,armour)| {
        let stats = armour.stats();
        (slot, json!({
            "base_id":armour.base_key(),"slot":armour.slot(),"quality":armour.quality(),"item_level":armour.item_level(),
            "source_sha256":item.source_sha256(),"consumed_modifier_count":armour.consumed_modifier_count(),
            "source_global_modifiers":armour.source_global_records(),
            "generated_global_modifiers":armour.generated_global_records(),
            "global_modifiers":armour.global_records(),
            "base_armour":stats.base_armour,"base_evasion":stats.base_evasion,"base_energy_shield":stats.base_energy_shield,
            "armour":stats.armour,"evasion":stats.evasion,"energy_shield":stats.energy_shield,
        }))
    }).collect();
    json!({"schema_version":2,"items":items})
}

#[cfg(test)]
mod tests {
    use super::*;
    use poe_optimizer_data::{class_tree::ClassTreeSelection, game_data};
    #[test]
    fn legacy_actor_records_resolve_once_from_exact_selected_source_views() {
        let data = game_data::bundled_snapshot().unwrap();
        let mut tree = ClassTreeSelection {
            class_id: 6,
            ascendancy_id: None,
            entrance_node_id: Some(38646),
            ascendancy_node_id: None,
        }
        .resolve(data.tree())
        .unwrap();
        let records = legacy_passive_actor_records(&data, &tree).unwrap();
        let key = poe_optimizer_data::passive_allocation::key_for_effective(
            tree.paid_node.as_ref().unwrap(),
        );
        assert_eq!(
            records,
            data.package()
                .passive_view_effects(&key)
                .unwrap()
                .actor_modifiers
        );
        assert!(!records.is_empty());
        tree.base_attributes.strength += 1;
        assert!(legacy_passive_actor_records(&data, &tree).is_err());
    }
}
