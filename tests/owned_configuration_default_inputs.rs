//! Offline authenticated configuration inputs; committed data only.
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
#[path = "support/owned_configuration_enemy_speed_inputs.rs"]
mod interval;
#[test]
fn configured_enemy_interval_has_source_bound_units_default_and_accounting() {
    interval::check_authored();
}

#[test]
fn configured_enemy_critical_chance_has_source_bound_units_default_and_accounting() {
    chance::check_authored()
}

#[test]
fn configured_enemy_critical_damage_has_source_bound_units_default_and_accounting() {
    damage::check_authored()
}
