//! Ordinary direct-action timing over explicit numeric inputs. No skill or build
//! identifiers enter this reusable kernel. Other action modes need their own
//! complete source branches before callers can select them.
use poe_optimizer_data::game_data::DirectActionTimingData;

use super::ordinary;
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectActionTimingInput {
    pub base_time: f64,
    pub increased: f64,
    pub more: f64,
    pub additional_attack_time: f64,
    pub additional_cast_time: f64,
    pub action_speed_mod: f64,
    pub repeats: f64,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectActionTimingOutput {
    pub base_time: f64,
    pub speed_multiplier: f64,
    /// Post-action-speed rate before nonchannel server cap (PoB CastRate).
    pub cast_rate: f64,
    /// Final nonchannel rate used by Time and ordinary hit DPS (PoB Speed).
    pub speed: f64,
    pub time: f64,
    pub action_speed_mod: f64,
}
/// Preserve reciprocal/addition order, the two-decimal skill multiplier, the
/// post-action CastRate assignment, and the separate server-tick cap. This raw
/// kernel deliberately preserves IEEE zero/negative/infinite boundary results;
/// complete callers validate selected data and preserve source result availability.
pub fn calculate(
    data: &DirectActionTimingData,
    input: DirectActionTimingInput,
) -> DirectActionTimingOutput {
    let output = ordinary::calculate(
        ordinary::OrdinaryTimingParameters {
            server_tick_rate: data.server_tick_rate,
            speed_multiplier_rounding_precision: data.speed_multiplier_rounding_precision,
        },
        ordinary::OrdinaryTimingInput {
            base_time: input.base_time,
            increased_percent: input.increased,
            more_multiplier: input.more,
            additional_attack_time: input.additional_attack_time,
            additional_cast_time: input.additional_cast_time,
            action_speed_multiplier: input.action_speed_mod,
            repeats: input.repeats,
        },
    );
    DirectActionTimingOutput {
        base_time: output.base_time,
        speed_multiplier: output.speed_multiplier,
        cast_rate: output.pre_cap_rate,
        speed: output.action_rate,
        time: output.action_time,
        action_speed_mod: output.action_speed_multiplier,
    }
}
