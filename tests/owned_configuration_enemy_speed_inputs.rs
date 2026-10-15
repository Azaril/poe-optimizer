//! Offline source and data checks; adapter execution is covered in Import's
//! existing normalization fixture and the explicit all-five release test.
#[allow(dead_code)]
#[path = "support/owned_configuration_enemy_speed_inputs.rs"]
mod family;

#[test]
fn configured_enemy_interval_has_source_bound_units_default_and_accounting() {
    family::check_authored();
}
