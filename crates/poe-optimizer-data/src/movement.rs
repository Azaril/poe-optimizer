//! Source-owned movement formula defaults and generated armour penalty mapping.
use crate::game_data::*;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MovementData {
    pub base_multiplier: f64,
    pub minimum_multiplier: f64,
    pub rounding_precision: u32,
    pub query_stats: Vec<ActorStat>,
    /// Capture0 is the selected base's MovementPenalty ratio; generated after
    /// local consumption, preserving absence versus an explicit zero value.
    pub penalty_modifier: ActorModifierMapping,
}
impl MovementData {
    pub(crate) fn validate(&self) -> Result<(), GameDataError> {
        let invalid = |message: &str| GameDataError(message.into());
        if !self.base_multiplier.is_finite()
            || !(0.0..=1_000_000.0).contains(&self.base_multiplier)
            || !self.minimum_multiplier.is_finite()
            || !(0.0..=1_000_000.0).contains(&self.minimum_multiplier)
            || self.rounding_precision > 12
        {
            return Err(invalid(
                "movement defaults/rounding exceed bounded capabilities",
            ));
        }
        if self.query_stats != [ActorStat::MovementSpeed] {
            return Err(invalid(
                "movement query must retain exact source target/order",
            ));
        }
        let p = &self.penalty_modifier;
        if p.stat != ActorStat::MovementSpeed
            || p.flags != 0
            || p.keyword_flags != 0
            || p.tags
                != [ActorModifierTag::Condition {
                    variables: vec![ActorCondition::IgnoreMovementPenalties],
                    negated: true,
                }]
            || !matches!(p.effect,ActorRuleEffect::Numeric{operation:ActorNumericOperation::Base,value:ActorRuleValue::Capture{index:0,multiplier}} if multiplier.is_finite() && (-1_000_000.0..0.0).contains(&multiplier) && multiplier!=0.0)
        {
            return Err(invalid(
                "movement penalty requires complete source BASE capture0 and negated IgnoreMovementPenalties condition",
            ));
        }
        Ok(())
    }
}
