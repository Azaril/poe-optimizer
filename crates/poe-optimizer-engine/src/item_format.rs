//! Numeric item-source formatting before local/global modifier parsing.
//!
//! Item.lua calls ItemTools.applyRange even for literal rolls and scalar one.
//! A matching injected formatting policy therefore rounds captures before they
//! become modifiers. Configuration/passive inputs must not use this transform.
use crate::actor::ActorError;
fn round_symmetric(value: f64, decimals: u32) -> f64 {
    let factor = 10.0_f64.powi(decimals as i32);
    if value >= 0.0 {
        (value * factor + 0.5).floor() / factor
    } else {
        (value * factor - 0.5).ceil() / factor
    }
}
/// Numeric result of pinned ItemTools.formatValue with both scalars equal to one.
/// `precision` is the injected internal multiplier/divisor, not decimal places.
/// Missing scalability policy means no call: the caller retains its original
/// value. Presentation's trailing-zero option has no effect on parsed numbers.
/// This helper does not authorize ranges, catalysts, corruption or mod magnitude.
pub fn format_item_capture(
    value: f64,
    precision: f64,
    display_precision: Option<u32>,
) -> Result<f64, ActorError> {
    if !value.is_finite()
        || value.abs() > 1_000_000.0
        || !precision.is_finite()
        || !(1.0..=10_000.0).contains(&precision)
        || display_precision.is_some_and(|decimals| decimals > 2)
    {
        return Err(ActorError(
            "Item formatting requires bounded finite value, precision 1..10000 and display precision 0..2",
        ));
    }
    let value = round_symmetric(value * precision, 0) / precision;
    let decimals =
        display_precision.unwrap_or_else(|| (precision.log10() + 0.001).floor().min(2.0) as u32);
    Ok(round_symmetric(value, decimals))
}
