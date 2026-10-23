//! Equipment and defence source laws and their explicit retained publication cases.
//! Shared release and preservation helpers are registered once for this suite.
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[allow(dead_code)]
#[path = "support/owned_support_delivery_publication.rs"]
mod publication;
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_selected_request.rs"]
mod selected;

// These helpers use the same sibling families and replay inputs in every child.
#[allow(dead_code)]
#[path = "support/owned_player_evasion_equipment.rs"]
mod equipment;
#[allow(dead_code)]
#[path = "support/owned_player_evasion_equipment_replay.rs"]
mod equipment_native;
#[allow(dead_code)]
#[path = "support/owned_player_evasion_final.rs"]
mod evasion_family;
#[allow(dead_code)]
#[path = "support/owned_player_evasion_final_replay.rs"]
mod evasion_native;
#[allow(dead_code)]
#[path = "support/owned_item_evasion_property.rs"]
mod family;
#[allow(dead_code)]
#[path = "support/owned_identity_correspondence.rs"]
mod identity;
#[allow(dead_code)]
#[path = "support/owned_item_evasion_property_native.rs"]
mod item_native;
#[allow(dead_code)]
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[allow(dead_code)]
#[path = "support/owned_plan_replay.rs"]
mod replay;

// Crown and Leggings previously supplied identical SHA256 and manifest-root
// helpers; the evidence still receives its explicit ItemWitness at each call.
use family::{hash, root};
#[path = "support/owned_crown_declarations_evidence.rs"]
mod declaration_evidence;

#[path = "equipment_sources/charm_capacity.rs"]
mod charm_capacity;
#[path = "equipment_sources/cold_item_delivery.rs"]
mod cold_item_delivery;
#[path = "equipment_sources/crown_declarations.rs"]
mod crown_declarations;
#[path = "equipment_sources/fine_belt_modifiers.rs"]
mod fine_belt_modifiers;
#[path = "equipment_sources/fixed_spirit.rs"]
mod fixed_spirit;
#[path = "equipment_sources/global_energy_shield_inputs.rs"]
mod global_energy_shield_inputs;
#[path = "equipment_sources/item_evasion_property.rs"]
mod item_evasion_property;
#[path = "equipment_sources/leggings_declarations.rs"]
mod leggings_declarations;
#[path = "equipment_sources/leggings_placement.rs"]
mod leggings_placement;
#[path = "equipment_sources/local_defence_composition.rs"]
mod local_defence_composition;
#[path = "equipment_sources/player_evasion_equipment.rs"]
mod player_evasion_equipment;
#[path = "equipment_sources/player_evasion_final.rs"]
mod player_evasion_final;
#[path = "equipment_sources/player_evasion_override.rs"]
mod player_evasion_override;
#[path = "equipment_sources/solar_declarations.rs"]
mod solar_declarations;
