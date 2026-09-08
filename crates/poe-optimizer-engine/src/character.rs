//! Explicit resolved character values for the closed Spark/Mace pipelines.
//!
//! The host resolves class identity and at most the admitted ordinary entrance
//! passive before constructing this input. This is a typed numeric boundary, not
//! a stat-text parser or a general passive/ascendancy evaluator. No modifier here
//! changes attributes, resources, conversion, critical strikes or resistance.

use std::{error::Error, fmt};

/// Already-resolved whole attribute values, before their inherent bonuses.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CharacterAttributes {
    pub strength: f64,
    pub dexterity: f64,
    pub intelligence: f64,
}

/// The exact numeric modifier forms found on the pinned ordinary entrances.
/// Percentages are increased modifiers, added within each matching skill query.
/// Minion damage is explicit evidence but has no target in either zero-minion
/// profile. Armour/evasion affect their own outputs; no EHP metric is claimed.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CharacterModifiers {
    pub armour_flat: f64,
    pub evasion_flat: f64,
    pub energy_shield_flat: f64,
    pub skill_speed_increased: f64,
    pub spell_damage_increased: f64,
    pub attack_damage_increased: f64,
    pub melee_damage_increased: f64,
    pub projectile_damage_increased: f64,
    pub minion_damage_increased: f64,
}
impl CharacterModifiers {
    pub const NONE: Self = Self {
        armour_flat: 0.0,
        evasion_flat: 0.0,
        energy_shield_flat: 0.0,
        skill_speed_increased: 0.0,
        spell_damage_increased: 0.0,
        attack_damage_increased: 0.0,
        melee_damage_increased: 0.0,
        projectile_damage_increased: 0.0,
        minion_damage_increased: 0.0,
    };
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CharacterInput {
    pub attributes: CharacterAttributes,
    pub modifiers: CharacterModifiers,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterError(pub &'static str);
impl fmt::Display for CharacterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}
impl Error for CharacterError {}

/// Numeric scope guard, not build legality. The conservative ceiling prevents
/// overflow throughout the admitted formulas and is not a game stat maximum.
/// The host separately validates source forms and admitted allocation/mechanics.
/// Search callers enforce explicit point budgets; diagnostic evaluation alone
/// does not certify available passive points or item/gem requirements.
impl CharacterInput {
    pub fn validate(&self) -> Result<(), CharacterError> {
        for value in [
            self.attributes.strength,
            self.attributes.dexterity,
            self.attributes.intelligence,
        ] {
            if !value.is_finite() || !(0.0..=1_000_000.0).contains(&value) || value.fract() != 0.0 {
                return Err(CharacterError(
                    "Resolved character attributes must be finite integers in 0..1000000",
                ));
            }
        }
        let m = self.modifiers;
        for value in [
            m.armour_flat,
            m.evasion_flat,
            m.energy_shield_flat,
            m.skill_speed_increased,
            m.spell_damage_increased,
            m.attack_damage_increased,
            m.melee_damage_increased,
            m.projectile_damage_increased,
            m.minion_damage_increased,
        ] {
            if !value.is_finite() || !(0.0..=1_000_000.0).contains(&value) {
                return Err(CharacterError(
                    "Character entrance modifiers must be finite and in 0..1000000",
                ));
            }
        }
        Ok(())
    }
}

/// Reviewed default base evasion; injected kernels read their supplied snapshot.
pub fn base_evasion() -> f64 {
    crate::data::bundled_reference()
        .expect("reviewed game-data package")
        .snapshot()
        .package()
        .character
        .base_evasion
}
