//! Hit, deflection and armour formulas from `src/Modules/CalcDefence.lua:33-69`.
//!
//! Source attribution and the upstream MIT notice are in `NOTICE.md`. Values and
//! chances use PoB's units: ratings, raw hit damage, and percentage points (0-100).
//! These are low-level numeric translations, not input validators. They retain
//! early returns, operation ordering and special floating-point results instead of
//! replacing undefined or unbounded arithmetic with an optimizer score.
//!
//! NaN clamp behavior follows the selected x64 LuaJIT oracle's ordered min/max
//! operations. Callers must validate domains before using results as constraints:
//! a finite result does not prove the inputs or a complete build were valid.

use poe_optimizer_data::game_data::DefenceData;

/// Versioned constants supplied to the translated defence kernels.
///
/// Explicit inputs keep data-version selection outside the formulas. These fields
/// intentionally accept all `f64` values for parity testing; game data loaders must
/// validate constants before exposing them as an optimization domain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DefenceConstants {
    /// Coefficient of raw hit damage in the armour reduction denominator.
    pub armour_ratio: f64,
    /// Maximum deflection chance in percentage points.
    pub deflection_chance_cap: f64,
}

/// Reviewed default constants loaded from the common game-data package.
pub fn pinned_constants() -> DefenceConstants {
    crate::data::bundled_reference()
        .expect("reviewed game-data package")
        .defence_constants()
}

/// PoB's zero-decimal `round`: nearest integer, ties toward positive infinity.
///
/// This intentionally differs from `f64::round()` for negative half-integers.
/// NaN remains NaN and infinities remain infinite. This is the exact operation
/// sequence of `Modules/Common.lua:722-728` when `dec` is absent.
pub fn round_to_integer(value: f64) -> f64 {
    (value + 0.5).floor()
}

// x64 LuaJIT min/max select their second operand for equal or unordered inputs.
// Explicit comparisons avoid Rust/WASM min/max's differing NaN/zero policies.
fn lua_min(left: f64, right: f64) -> f64 {
    if left < right { left } else { right }
}

fn lua_max(left: f64, right: f64) -> f64 {
    if left > right { left } else { right }
}

/// `calcs.hitChance`: hit chance in percentage points.
///
/// Negative accuracy returns 5 immediately. `uncapped` removes the 100% ceiling
/// but retains the 5% floor. Inputs are not clamped before the upstream formula.
pub fn hit_chance(evasion: f64, accuracy: f64, uncapped: bool) -> f64 {
    hit_chance_with_data(
        evasion,
        accuracy,
        uncapped,
        crate::data::bundled_reference()
            .expect("reviewed game-data package")
            .defence(),
    )
}

/// Explicit balance-parameter variant of `calcs.hitChance`.
pub fn hit_chance_with_data(
    evasion: f64,
    accuracy: f64,
    uncapped: bool,
    data: &DefenceData,
) -> f64 {
    if accuracy < 0.0 {
        return data.hit_chance_floor;
    }
    let raw_chance = (accuracy * data.hit_accuracy_multiplier)
        / (accuracy + evasion * data.hit_evasion_multiplier)
        * 100.0;
    let rounded = round_to_integer(raw_chance);
    if uncapped {
        lua_max(rounded, data.hit_chance_floor)
    } else {
        lua_max(lua_min(rounded, data.hit_chance_cap), data.hit_chance_floor)
    }
}

/// `calcs.monsterHitChance` using the reviewed default parameters.
pub fn monster_hit_chance(evasion: f64, accuracy: f64) -> f64 {
    monster_hit_chance_with_data(
        evasion,
        accuracy,
        crate::data::bundled_reference()
            .expect("reviewed game-data package")
            .defence(),
    )
}

/// Explicit balance-parameter variant of `calcs.monsterHitChance`.
pub fn monster_hit_chance_with_data(evasion: f64, accuracy: f64, data: &DefenceData) -> f64 {
    if accuracy < 0.0 {
        return data.hit_chance_floor;
    }
    let raw_chance = (1.0
        - (data.monster_evasion_multiplier * evasion)
            / (evasion + data.monster_accuracy_multiplier * accuracy))
        * 100.0;
    lua_max(
        lua_min(round_to_integer(raw_chance), data.hit_chance_cap),
        data.hit_chance_floor,
    )
}

/// `calcs.deflectChance` with reviewed formula parameters and explicit caps.
pub fn deflect_chance(deflection: f64, accuracy: f64, constants: DefenceConstants) -> f64 {
    deflect_chance_with_data(
        deflection,
        accuracy,
        constants,
        crate::data::bundled_reference()
            .expect("reviewed game-data package")
            .defence(),
    )
}

/// Explicit balance-parameter variant of `calcs.deflectChance`.
pub fn deflect_chance_with_data(
    deflection: f64,
    accuracy: f64,
    constants: DefenceConstants,
    data: &DefenceData,
) -> f64 {
    if deflection < data.deflection_rating_floor {
        return 0.0;
    }
    let not_deflected = accuracy / (accuracy + deflection * data.deflection_rating_multiplier)
        * data.deflection_chance_multiplier
        - data.deflection_chance_offset;
    lua_max(
        lua_min(
            100.0 - round_to_integer(not_deflected),
            constants.deflection_chance_cap,
        ),
        0.0,
    )
}

/// `calcs.armourReductionF`: fractional armour reduction in percentage points.
///
/// Both zero inputs yield 0. Negative armour uses its positive magnitude in the
/// denominator and negates the result, preserving PoB's armour-break behavior.
/// This helper applies no later mitigation caps; singular inputs can yield
/// infinity or NaN, which the caller must handle explicitly.
pub fn armour_reduction_percent(armour: f64, raw: f64, constants: DefenceConstants) -> f64 {
    if armour == 0.0 && raw == 0.0 {
        0.0
    } else if armour < 0.0 {
        let positive_armour = -armour;
        -(positive_armour / (positive_armour + raw * constants.armour_ratio) * 100.0)
    } else {
        armour / (armour + raw * constants.armour_ratio) * 100.0
    }
}

/// `calcs.armourReduction`: armour reduction rounded using PoB's rounding rule.
pub fn armour_reduction_rounded_percent(armour: f64, raw: f64, constants: DefenceConstants) -> f64 {
    round_to_integer(armour_reduction_percent(armour, raw, constants))
}
