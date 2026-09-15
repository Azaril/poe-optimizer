//! Pure resistance arithmetic over explicit, already resolved numeric inputs.
//!
//! Callers establish contribution completeness, units, domains and availability.
//! A reduced MORE factor is numeric input, not admission of any source modifier
//! family. Override presence and maximum-override bypass belong to the caller;
//! `finish` accepts the actual selected total and maximum, including numeric zero.
//! No game data, actor selection, source query or gameplay default enters here.

/// Limits for the ordinary maximum branch, in resistance percentage points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrdinaryResistanceParameters {
    pub maximum_cap: f64,
    pub resistance_cap: f64,
    pub floor: f64,
}
impl OrdinaryResistanceParameters {
    pub(crate) fn truncated_limits(self) -> (f64, f64) {
        truncated_limits(lua_min(self.maximum_cap, self.resistance_cap), self.floor)
    }
}
/// Already reduced BASE/INC/MORE values. The multiplier is clamped only after
/// INC and MORE combine; BASE itself is not clamped to zero.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrdinaryResistanceInput {
    pub base: f64,
    /// Percentage points: 25 means a 25% increase.
    pub increased_percent: f64,
    /// Already reduced multiplicative factor; no modifier reduction occurs here.
    pub more_multiplier: f64,
}
/// Selected values before truncation. `maximum` is already resolved: this stage
/// applies no source maximum cap and does not infer an override from truthiness.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResistanceFinishInput {
    pub total: f64,
    pub maximum: f64,
    pub floor: f64,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResistanceFinishOutput {
    pub total: f64,
    pub cap: f64,
    pub floor: f64,
    pub resistance: f64,
}
/// Raw channels retain IEEE results and signed zero independently. A finite
/// capped result does not establish that the total was available or finite.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrdinaryResistanceOutput {
    pub multiplier: f64,
    pub pre_truncation_total: f64,
    pub total: f64,
    pub cap: f64,
    pub floor: f64,
    pub resistance: f64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResistanceChannel {
    Multiplier,
    PreTruncationTotal,
    Total,
    Cap,
    Floor,
    Resistance,
}
/// Numerical classification only. Finite values preserve their exact bits;
/// semantic callers separately determine presence, coverage and gameplay validity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResistanceNumber {
    Finite { value: f64 },
    PositiveInfinity,
    NegativeInfinity,
    NotANumber,
}
impl ResistanceNumber {
    pub fn classify(value: f64) -> Self {
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
impl OrdinaryResistanceOutput {
    pub fn channel(&self, channel: ResistanceChannel) -> ResistanceNumber {
        ResistanceNumber::classify(match channel {
            ResistanceChannel::Multiplier => self.multiplier,
            ResistanceChannel::PreTruncationTotal => self.pre_truncation_total,
            ResistanceChannel::Total => self.total,
            ResistanceChannel::Cap => self.cap,
            ResistanceChannel::Floor => self.floor,
            ResistanceChannel::Resistance => self.resistance,
        })
    }
}
// Preserve Lua's second-operand selection on equality, including signed zero,
// and on unordered comparison. f64::min/max need not retain these operand bits.
fn lua_min(a: f64, b: f64) -> f64 {
    if a < b { a } else { b }
}
fn lua_max(a: f64, b: f64) -> f64 {
    if a > b { a } else { b }
}
fn truncated_limits(maximum: f64, floor: f64) -> (f64, f64) {
    (maximum.trunc(), floor.trunc())
}
/// Truncate total and each selected limit toward zero, cap, then floor. A floor
/// above the maximum wins; whether those limits are legal is a separate concern.
/// Raw nonfinite channels are retained, never promoted to semantic availability.
pub fn finish(input: ResistanceFinishInput) -> ResistanceFinishOutput {
    let total = input.total.trunc();
    let (cap, floor) = truncated_limits(input.maximum, input.floor);
    ResistanceFinishOutput {
        total,
        cap,
        floor,
        resistance: lua_max(lua_min(total, cap), floor),
    }
}
/// Preserve literal BASE * max((1 + INC / 100) * MORE, 0), followed by the shared
/// finishing law. Do not distribute multiplication or clamp INC before MORE.
pub fn calculate(
    parameters: OrdinaryResistanceParameters,
    input: OrdinaryResistanceInput,
) -> OrdinaryResistanceOutput {
    let multiplier = lua_max(
        (1.0 + input.increased_percent / 100.0) * input.more_multiplier,
        0.0,
    );
    let pre_truncation_total = input.base * multiplier;
    let output = finish(ResistanceFinishInput {
        total: pre_truncation_total,
        maximum: lua_min(parameters.maximum_cap, parameters.resistance_cap),
        floor: parameters.floor,
    });
    OrdinaryResistanceOutput {
        multiplier,
        pre_truncation_total,
        total: output.total,
        cap: output.cap,
        floor: output.floor,
        resistance: output.resistance,
    }
}
