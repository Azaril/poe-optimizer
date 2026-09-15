//! Legacy scalar BASE resistance adapter for explicit Spark/Mace numeric callers.
//! Complete source preparation uses actor_receiving with ordered BASE/INC records.
//!
//! CalcSetup applies the penalty only to elemental types. CalcDefence sums each
//! type with ElementalResist only for elemental types, truncates the total and
//! configured limits toward zero, then applies maximum and minimum. INC/MORE,
//! overrides, conditional/actor modifiers, maximum-resistance modifiers and
//! conversions must be rejected by the caller before this numeric boundary.
use crate::character::CharacterModifiers;
use poe_optimizer_data::game_data::DefenceData;

pub mod ordinary;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerResistances {
    pub fire: f64,
    pub cold: f64,
    pub lightning: f64,
    pub chaos: f64,
}

/// Calculate from already validated numeric contributions and injected rules.
/// `quest_rewards` is ordered fire, cold, lightning. Values are added to their
/// individual BASE bucket before its elemental bucket, matching ModDB::Sum.
/// Both inputs and rules are validated by the profile/data boundary; conservative
/// numeric limits keep every intermediate finite.
pub(crate) fn calculate(
    modifiers: &CharacterModifiers,
    penalty: f64,
    quest_rewards: [f64; 3],
    rules: &DefenceData,
) -> PlayerResistances {
    let parameters = ordinary::OrdinaryResistanceParameters {
        maximum_cap: rules.resistance_maximum_cap,
        resistance_cap: rules.player_resistance_cap,
        floor: rules.resistance_floor,
    };
    let finish = |base: f64| {
        ordinary::calculate(
            parameters,
            ordinary::OrdinaryResistanceInput {
                base,
                // This compatibility adapter admits BASE contributions only.
                increased_percent: 0.0,
                more_multiplier: 1.0,
            },
        )
        .resistance
    };
    let elemental = |individual: f64, quest: f64| {
        // The per-type bucket contains admitted passive, then base penalty and
        // quest reward; ElementalResist is a separate final ModDB query name.
        finish(((individual + penalty) + quest) + modifiers.elemental_resistance_flat)
    };
    PlayerResistances {
        fire: elemental(modifiers.fire_resistance_flat, quest_rewards[0]),
        cold: elemental(modifiers.cold_resistance_flat, quest_rewards[1]),
        lightning: elemental(modifiers.lightning_resistance_flat, quest_rewards[2]),
        chaos: finish(0.0 + modifiers.chaos_resistance_flat),
    }
}
