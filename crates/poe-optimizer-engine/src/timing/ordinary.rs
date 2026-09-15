//! Pure ordinary nonchannel action timing over already resolved numeric inputs.
//!
//! Callers choose the supported timing mode, prove input/contribution availability,
//! and validate dimensional relationships and any required parameter bounds. All time inputs
//! use one canonical time unit, and rate parameters/outputs use its reciprocal.
//! This module neither looks up game data nor supplies defaults. It deliberately
//! retains the historical floating-point order and rounding contract; it is not
//! an algebraic simplification or a general action-mode resolver.

/// Explicit operation parameters, independent of any game-data package.
///
/// This raw kernel preserves all existing numerical behavior. Semantic callers
/// own precision bounds; injected schemas and requirement rules decide whether
/// a finite rate/repeat combination is valid for the selected game mechanic.
/// Numerical computability alone does not establish that gameplay validity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrdinaryTimingParameters {
    pub server_tick_rate: f64,
    pub speed_multiplier_rounding_precision: u32,
}

/// Already resolved inputs; no missing input is represented by a numeric default.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrdinaryTimingInput {
    pub base_time: f64,
    /// Percentage points, so 25 means a 25% increase.
    pub increased_percent: f64,
    /// Multiplicative factor, after the caller's complete contribution reduction.
    pub more_multiplier: f64,
    pub additional_attack_time: f64,
    pub additional_cast_time: f64,
    pub action_speed_multiplier: f64,
    /// Explicit repeat count as a raw number. Semantic callers own its domain.
    pub repeats: f64,
}

/// Individually inspectable raw channels, including signed zero and IEEE results.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrdinaryTimingOutput {
    pub base_time: f64,
    pub speed_multiplier: f64,
    /// Action-speed-adjusted rate before the server cap.
    pub pre_cap_rate: f64,
    pub action_rate: f64,
    pub action_time: f64,
    pub action_speed_multiplier: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimingChannel {
    BaseTime,
    SpeedMultiplier,
    PreCapRate,
    ActionRate,
    ActionTime,
    ActionSpeedMultiplier,
}

/// Numerical classification only: this does not establish metric availability.
/// A finite value retains its exact bits, including negative zero.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimingNumber {
    Finite { value: f64 },
    PositiveInfinity,
    NegativeInfinity,
    NotANumber,
}
impl TimingNumber {
    fn from_number(value: f64) -> Self {
        if value.is_finite() {
            Self::Finite { value }
        } else if value.is_nan() {
            Self::NotANumber
        } else if value.is_sign_positive() {
            Self::PositiveInfinity
        } else {
            Self::NegativeInfinity
        }
    }
}
impl OrdinaryTimingOutput {
    /// Classify only the requested channel. An infinite pre-cap rate cannot
    /// erase a finite capped action rate or its finite action time.
    pub fn channel(&self, channel: TimingChannel) -> TimingNumber {
        TimingNumber::from_number(match channel {
            TimingChannel::BaseTime => self.base_time,
            TimingChannel::SpeedMultiplier => self.speed_multiplier,
            TimingChannel::PreCapRate => self.pre_cap_rate,
            TimingChannel::ActionRate => self.action_rate,
            TimingChannel::ActionTime => self.action_time,
            TimingChannel::ActionSpeedMultiplier => self.action_speed_multiplier,
        })
    }
}

/// Preserve reciprocal/addition order, floor(x + 0.5) rounding, action speed
/// before the cap, and the separate zero-time case. Do not return one aggregate
/// finite/nonfinite result: each output has its own numerical classification.
pub fn calculate(
    parameters: OrdinaryTimingParameters,
    input: OrdinaryTimingInput,
) -> OrdinaryTimingOutput {
    let power = 10_f64.powi(parameters.speed_multiplier_rounding_precision as i32);
    let speed_multiplier =
        (((1.0 + input.increased_percent / 100.0) * input.more_multiplier) * power + 0.5).floor()
            / power;
    let ordinary_rate = 1.0
        / (input.base_time / speed_multiplier
            + input.additional_attack_time
            + input.additional_cast_time);
    let pre_cap_rate = ordinary_rate * input.action_speed_multiplier;
    let ceiling = parameters.server_tick_rate * input.repeats;
    // Preserve second-argument selection on equality and unordered comparison.
    let action_rate = if pre_cap_rate < ceiling {
        pre_cap_rate
    } else {
        ceiling
    };
    let action_time = if action_rate == 0.0 {
        0.0
    } else {
        1.0 / action_rate
    };
    OrdinaryTimingOutput {
        base_time: input.base_time,
        speed_multiplier,
        pre_cap_rate,
        action_rate,
        action_time,
        action_speed_multiplier: input.action_speed_multiplier,
    }
}
