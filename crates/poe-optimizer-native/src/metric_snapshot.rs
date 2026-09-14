//! Stack measurements shared by the retained native candidate evaluator.
//! This is the current diagnostic metric contract, not a complete semantic result model.
use poe_optimizer_core::metrics::*;
use poe_optimizer_engine::mace::MaceOutput;

pub(crate) const ATTACK_AVERAGE_REASON: &str =
    "Attack AverageHit is stored per hand; the current metric contract does not aggregate hands.";

/// Borrowed/inline metric value. Formatting into the shared owned measurement
/// contract happens only at the scheduler/report boundary, outside calculation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NativeMetricValue {
    Finite(f64),
    NonFinite(NonFiniteKind),
    Unavailable(&'static str),
}
impl NativeMetricValue {
    fn from_number(value: f64) -> Self {
        match MeasurementValue::from_number(value) {
            MeasurementValue::Finite { value } => Self::Finite(value),
            MeasurementValue::NonFinite { kind } => Self::NonFinite(kind),
            MeasurementValue::Unavailable { .. } => unreachable!("numeric conversion"),
        }
    }
    pub fn finite(self) -> Option<f64> {
        match self {
            Self::Finite(value) if value.is_finite() => Some(value),
            _ => None,
        }
    }
    pub fn to_owned(self) -> MeasurementValue {
        match self {
            Self::Finite(value) => MeasurementValue::Finite { value },
            Self::NonFinite(kind) => MeasurementValue::NonFinite { kind },
            Self::Unavailable(reason) => MeasurementValue::Unavailable {
                reason: reason.into(),
            },
        }
    }
}

/// Stack snapshot in native metric-catalog order, including explicit availability.
/// No strings, XML, JSON, diagnostics or cached build result are constructed.
#[derive(Debug, Clone, Copy)]
pub struct NativeMetricSnapshot {
    values: [NativeMetricValue; 14],
    elapsed_ms: f64,
}
impl NativeMetricSnapshot {
    /// Complete native metric catalog order; use the prepared selector's
    /// `snapshot_measurements` for requested metrics and owned query descriptors.
    pub fn values(&self) -> &[NativeMetricValue] {
        &self.values
    }
    pub fn elapsed_ms(&self) -> f64 {
        self.elapsed_ms
    }
    /// This bounded diagnostic profile does not certify complete game coverage.
    pub fn diagnostic_only(&self) -> bool {
        true
    }
    pub(crate) fn set_elapsed(&mut self, elapsed_ms: f64) {
        self.elapsed_ms = elapsed_ms;
    }
    pub(crate) fn from_calculation(output: crate::NativeCalculation) -> Self {
        match output {
            crate::NativeCalculation::Mace(output) => Self::from_output(output),
            crate::NativeCalculation::Spark(output) => Self {
                values: [
                    NativeMetricValue::from_number(output.life),
                    NativeMetricValue::from_number(output.mana),
                    NativeMetricValue::from_number(output.energy_shield),
                    NativeMetricValue::from_number(output.fire_resistance),
                    NativeMetricValue::from_number(output.cold_resistance),
                    NativeMetricValue::from_number(output.lightning_resistance),
                    NativeMetricValue::from_number(output.chaos_resistance),
                    NativeMetricValue::from_number(output.average_hit),
                    NativeMetricValue::from_number(output.hit_dps),
                    NativeMetricValue::from_number(output.spirit),
                    NativeMetricValue::from_number(output.armour),
                    NativeMetricValue::from_number(output.evasion),
                    NativeMetricValue::from_number(100.0 * output.effective_movement_speed_mod),
                    NativeMetricValue::from_number(100.0 * output.action_speed_mod),
                ],
                elapsed_ms: 0.0,
            },
        }
    }
    fn from_output(output: MaceOutput) -> Self {
        Self {
            values: [
                NativeMetricValue::from_number(output.life),
                NativeMetricValue::from_number(output.mana),
                NativeMetricValue::from_number(output.energy_shield),
                NativeMetricValue::from_number(output.fire_resistance),
                NativeMetricValue::from_number(output.cold_resistance),
                NativeMetricValue::from_number(output.lightning_resistance),
                NativeMetricValue::from_number(output.chaos_resistance),
                NativeMetricValue::Unavailable(ATTACK_AVERAGE_REASON),
                NativeMetricValue::from_number(output.hit_dps),
                NativeMetricValue::from_number(output.spirit),
                NativeMetricValue::from_number(output.armour),
                NativeMetricValue::from_number(output.evasion),
                NativeMetricValue::from_number(100.0 * output.effective_movement_speed_mod),
                NativeMetricValue::from_number(100.0 * output.action_speed_mod),
            ],
            elapsed_ms: 0.0,
        }
    }
}
