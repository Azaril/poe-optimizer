//! Explicit resolved character values for the closed Spark/Mace pipelines.
//!
//! The host resolves class identity and at most the admitted ordinary entrance
//! passive plus an admitted ascendancy passive before constructing this input.
//! This is a typed numeric boundary, not a stat-text parser or a general
//! passive/ascendancy evaluator. No modifier here
//! changes attributes, resources, conversion or critical strikes. Signed player
//! BASE resistance contributions are explicit and exclude other actors.

use std::{error::Error, fmt};

/// Already-resolved whole attribute values, before their inherent bonuses.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CharacterAttributes {
    pub strength: f64,
    pub dexterity: f64,
    pub intelligence: f64,
}

/// The admitted numeric modifier forms found on pinned owned passives.
/// Damage/speed percentages are increased modifiers within each skill query.
/// Resistance percentages are signed BASE contributions in percentage points.
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
    /// Signed player BASE contributions; elemental excludes chaos.
    pub fire_resistance_flat: f64,
    pub cold_resistance_flat: f64,
    pub lightning_resistance_flat: f64,
    pub chaos_resistance_flat: f64,
    pub elemental_resistance_flat: f64,
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
        fire_resistance_flat: 0.0,
        cold_resistance_flat: 0.0,
        lightning_resistance_flat: 0.0,
        chaos_resistance_flat: 0.0,
        elemental_resistance_flat: 0.0,
    };

    /// Compose separately admitted passive records without admitting overflow or
    /// expanding their numeric scope. Neither operand can hide an invalid value
    /// through cancellation. This does not validate allocation ownership.
    pub fn checked_add(self, rhs: Self) -> Result<Self, CharacterError> {
        self.validate()?;
        rhs.validate()?;
        let result = Self {
            armour_flat: self.armour_flat + rhs.armour_flat,
            evasion_flat: self.evasion_flat + rhs.evasion_flat,
            energy_shield_flat: self.energy_shield_flat + rhs.energy_shield_flat,
            skill_speed_increased: self.skill_speed_increased + rhs.skill_speed_increased,
            spell_damage_increased: self.spell_damage_increased + rhs.spell_damage_increased,
            attack_damage_increased: self.attack_damage_increased + rhs.attack_damage_increased,
            melee_damage_increased: self.melee_damage_increased + rhs.melee_damage_increased,
            projectile_damage_increased: self.projectile_damage_increased
                + rhs.projectile_damage_increased,
            minion_damage_increased: self.minion_damage_increased + rhs.minion_damage_increased,
            fire_resistance_flat: self.fire_resistance_flat + rhs.fire_resistance_flat,
            cold_resistance_flat: self.cold_resistance_flat + rhs.cold_resistance_flat,
            lightning_resistance_flat: self.lightning_resistance_flat
                + rhs.lightning_resistance_flat,
            chaos_resistance_flat: self.chaos_resistance_flat + rhs.chaos_resistance_flat,
            elemental_resistance_flat: self.elemental_resistance_flat
                + rhs.elemental_resistance_flat,
        };
        result.validate()?;
        Ok(result)
    }

    pub fn validate(&self) -> Result<(), CharacterError> {
        let m = self;
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
                    "Character non-resistance modifiers must be finite and in 0..1000000",
                ));
            }
        }
        for value in [
            m.fire_resistance_flat,
            m.cold_resistance_flat,
            m.lightning_resistance_flat,
            m.chaos_resistance_flat,
            m.elemental_resistance_flat,
        ] {
            if !value.is_finite() || !(-1_000_000.0..=1_000_000.0).contains(&value) {
                return Err(CharacterError(
                    "Character resistance modifiers must be finite and in -1000000..1000000",
                ));
            }
        }
        Ok(())
    }
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
        self.modifiers.validate()?;
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
