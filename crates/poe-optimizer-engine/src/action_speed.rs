//! Shared action speed after source-order modifier queries and actor conditions.
use crate::actor::ActorError;
use poe_optimizer_data::game_data::ActionSpeedData;
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActionSpeedOutput {
    pub action_speed_mod: f64,
    pub minimum_action_speed: Option<f64>,
    pub maximum_action_speed_reduction: Option<f64>,
    pub action_speed_increased: f64,
    pub temporal_chains_action_speed_increased: f64,
    pub unaffected_by_slows: bool,
}
/// Literal arithmetic of CalcPerform.actionSpeedMod. Inputs are queried values;
/// MAX availability and positive-row filtering belong to the shared DB stage.
/// The minimum is applied before the maximum; contradictory caps stay observable.
pub fn calculate(
    data: &ActionSpeedData,
    minimum_action_speed: Option<f64>,
    maximum_action_speed_reduction: Option<f64>,
    action_speed_increased: f64,
    temporal_chains_action_speed_increased: f64,
    unaffected_by_slows: bool,
) -> Result<ActionSpeedOutput, ActorError> {
    let temporal = if -data.temporal_chains_effect_cap > temporal_chains_action_speed_increased {
        -data.temporal_chains_effect_cap
    } else {
        temporal_chains_action_speed_increased
    };
    let value = data.base_multiplier + (temporal + action_speed_increased) / data.percent_divisor;
    let minimum =
        minimum_action_speed.unwrap_or(data.default_minimum_percent) / data.percent_divisor;
    let mut action_speed_mod = if minimum > value { minimum } else { value };
    if let Some(maximum) = maximum_action_speed_reduction {
        let ceiling = (data.percent_divisor - maximum) / data.percent_divisor;
        action_speed_mod = if ceiling < action_speed_mod {
            ceiling
        } else {
            action_speed_mod
        };
    }
    if !action_speed_mod.is_finite() {
        return Err(ActorError("Action speed calculation must remain finite"));
    }
    Ok(ActionSpeedOutput {
        action_speed_mod,
        minimum_action_speed,
        maximum_action_speed_reduction,
        action_speed_increased,
        temporal_chains_action_speed_increased,
        unaffected_by_slows,
    })
}
