//! Shared admitted offence operations; broader critical mechanics remain outside
//! the closed profiles. Callers validate finite data and resolve other modifiers.
use crate::defence::round_to_integer;

/// Pinned CalcOffence: two-decimal chance, actor cap, then nonnegative floor.
/// The attack pipeline applies its second accuracy check after this operation.
pub(crate) fn capped_critical_chance(chance: f64, cap: f64) -> f64 {
    (round_to_integer(chance * 100.0) / 100.0).min(cap).max(0.0)
}
