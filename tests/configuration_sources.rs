//! Configuration source proofs and their explicit retained publication cases.
//! Shared helpers are registered once, including their independent preservation laws.
#[allow(dead_code)]
#[path = "support/owned_configuration_enemy_critical_chance_inputs.rs"]
mod chance;
#[path = "support/owned_configuration_default_sources.rs"]
mod configuration_sources;
#[allow(dead_code)]
#[path = "support/owned_configuration_default_stage.rs"]
mod configuration_stage;
#[allow(dead_code)]
#[path = "support/owned_configuration_enemy_critical_damage_inputs.rs"]
mod damage;
#[allow(dead_code)]
#[path = "support/owned_configuration_distance_inputs.rs"]
mod distance;
#[allow(dead_code)]
#[path = "support/owned_configuration_enemy_speed_inputs.rs"]
mod interval;
#[path = "support/owned_configuration_default_publication.rs"]
mod publication;
#[allow(dead_code)]
#[path = "support/owned_selected_request.rs"]
mod selected;
#[allow(dead_code)]
#[path = "support/owned_configuration_player_withered_input.rs"]
mod withered;

#[path = "configuration_sources/default_inputs.rs"]
mod default_inputs;
#[path = "configuration_sources/default_publications.rs"]
mod default_publications;
#[path = "configuration_sources/distance_inputs.rs"]
mod distance_inputs;
#[path = "configuration_sources/distance_publication.rs"]
mod distance_publication;
#[path = "configuration_sources/player_withered_input.rs"]
mod player_withered_input;
#[path = "configuration_sources/player_withered_publication.rs"]
mod player_withered_publication;
