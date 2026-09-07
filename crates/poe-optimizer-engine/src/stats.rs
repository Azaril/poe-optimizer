//! Explicit ordinary `ModStore:GetStat` values for a single queried actor.
//!
//! This represents output-table lookup and cfg.skillStats fallback only. It does
//! not calculate these stats or resolve reservation-specific GetStat branches.

use std::{collections::BTreeMap, error::Error, fmt};

pub type StatValues = BTreeMap<String, f64>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatError {
    UnsupportedContext(String),
    UnsupportedStat(String),
}

impl fmt::Display for StatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unsupported native stat input: {self:?}")
    }
}
impl Error for StatError {}

/// Immutable resolved numeric tables. None preserves an absent actor output table.
/// Output values win over skill-local values, including numeric zero and NaN.
/// Retain unsupported values/context features rather than dropping them at import.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedStatEnvironment {
    output: Option<StatValues>,
    skill_stats: StatValues,
}

impl ResolvedStatEnvironment {
    pub fn try_new(
        output: Option<StatValues>,
        skill_stats: StatValues,
        unsupported_features: Vec<String>,
    ) -> Result<Self, StatError> {
        if let Some(feature) = unsupported_features.into_iter().next() {
            return Err(StatError::UnsupportedContext(feature));
        }
        Ok(Self {
            output,
            skill_stats,
        })
    }

    pub fn output(&self) -> Option<&StatValues> {
        self.output.as_ref()
    }
    pub fn skill_stats(&self) -> &StatValues {
        &self.skill_stats
    }

    pub fn get_stat(&self, stat: &str) -> Result<f64, StatError> {
        validate_stat(stat)?;
        Ok(self
            .output
            .as_ref()
            .and_then(|output| output.get(stat))
            .or_else(|| self.skill_stats.get(stat))
            .copied()
            .unwrap_or(0.0))
    }
}

/// Special names can remain in an output table, but cannot be queried through
/// this ordinary lookup slice or hidden behind an early-exit program condition.
pub(crate) fn validate_stat(stat: &str) -> Result<(), StatError> {
    match stat {
        "ManaReservedPercent" | "LifeReservedPercent" | "ManaUnreserved" => {
            Err(StatError::UnsupportedStat(stat.to_owned()))
        }
        _ => Ok(()),
    }
}
