//! Offline authenticated configuration inputs; committed data only.
use super::{chance, damage, interval};

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
