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

/// `Modules/Data.lua:251,261` at [`crate::UPSTREAM_REVISION`].
pub const PINNED_CONSTANTS: DefenceConstants = DefenceConstants {
    armour_ratio: 10.0,
    deflection_chance_cap: 95.0,
};

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
    if accuracy < 0.0 {
        return 5.0;
    }
    let raw_chance = (accuracy * 1.25) / (accuracy + evasion * 0.3) * 100.0;
    let rounded = round_to_integer(raw_chance);
    if uncapped {
        lua_max(rounded, 5.0)
    } else {
        lua_max(lua_min(rounded, 100.0), 5.0)
    }
}

/// `calcs.monsterHitChance`: monster hit chance, clamped to 5-100%.
pub fn monster_hit_chance(evasion: f64, accuracy: f64) -> f64 {
    if accuracy < 0.0 {
        return 5.0;
    }
    let raw_chance = (1.0 - (0.95 * evasion) / (evasion + 4.0 * accuracy)) * 100.0;
    lua_max(lua_min(round_to_integer(raw_chance), 100.0), 5.0)
}

/// `calcs.deflectChance`: deflection chance in percentage points.
///
/// A deflection rating below 1 returns 0 before inspecting accuracy/constants.
pub fn deflect_chance(deflection: f64, accuracy: f64, constants: DefenceConstants) -> f64 {
    if deflection < 1.0 {
        return 0.0;
    }
    let not_deflected = accuracy / (accuracy + deflection * 0.12) * 150.0 - 50.0;
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
