//! Ordinary direct-action timing over explicit numeric inputs. No skill or build
//! identifiers enter this reusable kernel. Other action modes need their own
//! complete source branches before callers can select them.
use poe_optimizer_data::game_data::DirectActionTimingData;
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
    let power = 10_f64.powi(data.speed_multiplier_rounding_precision as i32);
    let speed_multiplier =
        (((1.0 + input.increased / 100.0) * input.more) * power + 0.5).floor() / power;
    let ordinary_speed = 1.0
        / (input.base_time / speed_multiplier
            + input.additional_attack_time
            + input.additional_cast_time);
    let cast_rate = ordinary_speed * input.action_speed_mod;
    let ceiling = data.server_tick_rate * input.repeats;
    // Lua m_min chooses its second argument on equality (and unordered inputs).
    let speed = if cast_rate < ceiling {
        cast_rate
    } else {
        ceiling
    };
    let time = if speed == 0.0 { 0.0 } else { 1.0 / speed };
    DirectActionTimingOutput {
        base_time: input.base_time,
        speed_multiplier,
        cast_rate,
        speed,
        time,
        action_speed_mod: input.action_speed_mod,
    }
}
