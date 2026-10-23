//! Explicit all-five publications through one shared preservation harness.
use super::{chance, damage, interval, publication};

#[test]
#[ignore = "requires authenticated prior package and fresh output directory; no source runtime"]
fn publish_configured_enemy_interval_preserving_all_five_originals() {
    publication::run(publication::Publication {
        directory: interval::directory(),
        stage: interval::stage,
        prior: std::env::var_os("POE_OPTIMIZER_TEST_ENEMY_SPEED_PRIOR")
            .expect("prior package")
            .into(),
        output: std::env::var_os("POE_OPTIMIZER_TEST_ENEMY_SPEED_OUTPUT")
            .expect("fresh output")
            .into(),
        original05_before: (35, 20, 15),
        new_definitions: 2,
        expectations: publication::cases::ignored_enemy(700.),
    });
}

#[test]
#[ignore = "requires authenticated prior package and fresh output directory; no source runtime"]
fn publish_configured_enemy_critical_chance_preserving_all_five_originals() {
    publication::run(publication::Publication {
        directory: chance::directory(),
        stage: chance::stage,
        prior: std::env::var_os("POE_OPTIMIZER_TEST_ENEMY_CRITICAL_CHANCE_PRIOR")
            .expect("prior package")
            .into(),
        output: std::env::var_os("POE_OPTIMIZER_TEST_ENEMY_CRITICAL_CHANCE_OUTPUT")
            .expect("fresh output")
            .into(),
        original05_before: (34, 19, 15),
        new_definitions: 1,
        expectations: publication::cases::ignored_enemy(5.),
    });
}

#[test]
#[ignore = "requires authenticated prior package and fresh output directory; no source runtime"]
fn publish_configured_enemy_critical_damage_preserving_all_five_originals() {
    publication::run(publication::Publication {
        directory: damage::directory(),
        stage: damage::stage,
        prior: std::env::var_os("POE_OPTIMIZER_TEST_ENEMY_CRITICAL_DAMAGE_PRIOR")
            .expect("prior package")
            .into(),
        output: std::env::var_os("POE_OPTIMIZER_TEST_ENEMY_CRITICAL_DAMAGE_OUTPUT")
            .expect("fresh output")
            .into(),
        original05_before: (33, 18, 15),
        new_definitions: 1,
        expectations: publication::cases::ignored_enemy(30.),
    });
}
