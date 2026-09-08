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
