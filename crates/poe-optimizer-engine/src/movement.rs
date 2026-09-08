//! Shared source movement arithmetic over already resolved numeric queries.
//! Action speed is explicit in this raw helper. Complete actor preparation uses
//! the selected data's proven default and admits no action-speed modifiers yet.
use crate::actor::ActorError;
use poe_optimizer_data::game_data::MovementData;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MovementInput {
    pub base: f64,
    pub increased: f64,
    pub more: f64,
    pub override_value: Option<f64>,
    pub cannot_be_below_base: bool,
    pub ignore_movement_penalties: bool,
    pub action_speed_mod: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MovementOutput {
    pub has_override: bool,
    pub movement_speed_mod: f64,
    pub action_speed_mod: f64,
    pub effective_movement_speed_mod: f64,
    pub ignore_movement_penalties: bool,
    pub cannot_be_below_base: bool,
}

/// Literal CalcDefence movement branch, after source query/condition resolution.
/// An override (including zero) skips the first round and all numeric queries;
/// the effective result always rounds after multiplying by action speed.
pub fn calculate(data: &MovementData, input: MovementInput) -> Result<MovementOutput, ActorError> {
    let finite = |value: f64| {
        if value.is_finite() {
            Ok(value)
        } else {
            Err(ActorError("Movement calculation must remain finite"))
        }
    };
    if data.rounding_precision > 12 {
        return Err(ActorError(
            "Movement rounding precision exceeds supported scope",
        ));
    }
    let power = 10_u64.pow(data.rounding_precision) as f64;
    let round = |value: f64| (value * power + 0.5).floor() / power;
    let mut movement_speed_mod = match input.override_value {
        Some(value) => finite(value)?,
        None => {
            // calcLib.mod multiplies INC and MORE before the separate BASE term.
            let modifier = (1.0 + input.increased / 100.0) * input.more;
            finite(round(finite(
                (data.base_multiplier + input.base) * modifier,
            )?))?
        }
    };
    if input.cannot_be_below_base {
        // Lua m_max selects the second operand on equality, including +/- zero.
        movement_speed_mod = if movement_speed_mod > data.minimum_multiplier {
            movement_speed_mod
        } else {
            data.minimum_multiplier
        };
    }
    let action_speed_mod = finite(input.action_speed_mod)?;
    let effective_movement_speed_mod =
        finite(round(finite(movement_speed_mod * action_speed_mod)?))?;
    Ok(MovementOutput {
        has_override: input.override_value.is_some(),
        movement_speed_mod,
        action_speed_mod,
        effective_movement_speed_mod,
        ignore_movement_penalties: input.ignore_movement_penalties,
        cannot_be_below_base: input.cannot_be_below_base,
    })
}
