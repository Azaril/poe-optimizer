//! Units and availability are shared contracts; raw backend field names are not.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorScope {
    Player,
    SelectedMinion,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricQuery {
    pub actor: ActorScope,
    pub id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricUnit {
    PoolPoints,
    Percent,
    Damage,
    DamagePerSecond,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDefinition {
    pub id: String,
    pub unit: MetricUnit,
    pub actors: Vec<ActorScope>,
    pub description: String,
    /// Metric definitions are versioned independently of backend implementation.
    pub schema_version: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NonFiniteKind {
    PositiveInfinity,
    NegativeInfinity,
    NotANumber,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum MeasurementValue {
    Finite { value: f64 },
    NonFinite { kind: NonFiniteKind },
    Unavailable { reason: String },
}

impl MeasurementValue {
    pub fn from_number(value: f64) -> Self {
        if value.is_finite() {
            Self::Finite { value }
        } else {
            Self::NonFinite {
                kind: if value.is_nan() {
                    NonFiniteKind::NotANumber
                } else if value.is_sign_positive() {
                    NonFiniteKind::PositiveInfinity
                } else {
                    NonFiniteKind::NegativeInfinity
                },
            }
        }
    }

    /// Scoring must opt into any infinity interpretation rather than receiving it as a number.
    pub fn finite(&self) -> Option<f64> {
        match self {
            Self::Finite { value } if value.is_finite() => Some(*value),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricMeasurement {
    pub query: MetricQuery,
    pub unit: MetricUnit,
    pub value: MeasurementValue,
    pub schema_version: u32,
}
