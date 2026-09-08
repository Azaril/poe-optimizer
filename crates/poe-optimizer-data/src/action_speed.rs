//! Source-owned action-speed and ordinary direct-action timing configuration.
use crate::game_data::{ActorStat, GameDataError};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionSpeedData {
    pub base_multiplier: f64,
    pub default_minimum_percent: f64,
    pub temporal_chains_effect_cap: f64,
    pub percent_divisor: f64,
    /// Exact source order: minimum, maximum reduction, unaffected flag, ordinary
    /// ActionSpeed INC and TemporalChainsActionSpeed INC.
    pub query_stats: Vec<ActorStat>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectActionTimingEligibility {
    /// Ordinary non-channelled self-cast action; no time override, trigger,
    /// cooldown, reload, repeats or other unimplemented timing branches.
    OrdinarySelfCast,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectActionTimingData {
    pub server_tick_rate: f64,
    pub speed_multiplier_rounding_precision: u32,
    pub default_repeats: u32,
    pub eligibility: DirectActionTimingEligibility,
}
impl ActionSpeedData {
    pub(crate) fn validate(&self) -> Result<(), GameDataError> {
        let bounded = |v: f64| v.is_finite() && (0.0..=1_000_000.0).contains(&v);
        if !bounded(self.base_multiplier)
            || !bounded(self.default_minimum_percent)
            || !bounded(self.temporal_chains_effect_cap)
            || !bounded(self.percent_divisor)
            || self.percent_divisor == 0.0
        {
            return Err(GameDataError(
                "action-speed formula values exceed finite bounds".into(),
            ));
        }
        if self.query_stats
            != [
                ActorStat::MinimumActionSpeed,
                ActorStat::MaximumActionSpeedReduction,
                ActorStat::UnaffectedBySlows,
                ActorStat::ActionSpeed,
                ActorStat::TemporalChainsActionSpeed,
            ]
        {
            return Err(GameDataError(
                "action-speed queries must retain exact source target/order".into(),
            ));
        }
        Ok(())
    }
}
impl DirectActionTimingData {
    pub(crate) fn validate(&self) -> Result<(), GameDataError> {
        if !self.server_tick_rate.is_finite()
            || !(0.000001..=1_000_000.0).contains(&self.server_tick_rate)
            || self.speed_multiplier_rounding_precision > 12
            || self.default_repeats != 1
            || self.eligibility != DirectActionTimingEligibility::OrdinarySelfCast
        {
            return Err(GameDataError("direct-action timing requires finite positive tick rate, bounded precision and ordinary self-cast repeats1 eligibility".into()));
        }
        Ok(())
    }
}
