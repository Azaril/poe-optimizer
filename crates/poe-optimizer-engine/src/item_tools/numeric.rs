use super::{FormatError, Result};

pub(super) fn round_symmetric(value: f64, decimals: Option<f64>) -> f64 {
    if let Some(decimals) = decimals {
        let factor = 10.0_f64.powf(decimals);
        if value >= 0.0 {
            (value * factor + 0.5).floor() / factor
        } else {
            (value * factor - 0.5).ceil() / factor
        }
    } else if value >= 0.0 {
        (value + 0.5).floor()
    } else {
        (value - 0.5).ceil()
    }
}

/// LuaJIT's default numeric tostring uses fourteen significant decimal digits.
/// The second argument to Lua tostring does not change this formatting.
pub fn lua_number_text(value: f64) -> String {
    if value.is_nan() {
        return "nan".into();
    }
    if value == f64::INFINITY {
        return "inf".into();
    }
    if value == f64::NEG_INFINITY {
        return "-inf".into();
    }
    if value == 0.0 {
        return if value.is_sign_negative() { "-0" } else { "0" }.into();
    }
    let scientific = format!("{value:.13e}");
    let (mantissa, exponent) = scientific.split_once('e').expect("Rust scientific number");
    let exponent: i32 = exponent.parse().expect("Rust scientific exponent");
    if !(-4..14).contains(&exponent) {
        let mantissa = mantissa.trim_end_matches('0').trim_end_matches('.');
        format!(
            "{mantissa}e{}{abs:02}",
            if exponent < 0 { '-' } else { '+' },
            abs = exponent.unsigned_abs()
        )
    } else {
        let fixed = format!("{value:.precision$}", precision = (13 - exponent) as usize);
        if fixed.contains('.') {
            fixed.trim_end_matches('0').trim_end_matches('.').into()
        } else {
            fixed
        }
    }
}

/// Original formatValue operation and rounding order, including optional zero scalars.
pub fn format_value(
    value: f64,
    base_value_scalar: Option<f64>,
    value_scalar: Option<f64>,
    precision: f64,
    display_precision: Option<u8>,
    if_required: bool,
) -> Result<String> {
    if display_precision.is_some_and(|n| n > 99) {
        return Err(FormatError::ResourceBound("display precision"));
    }
    let mut value = round_symmetric(value * precision, None);
    if let Some(base) = base_value_scalar.filter(|v| *v != 1.0) {
        value = (value * base + 0.5).trunc();
    }
    if let Some(scalar) = value_scalar.filter(|v| *v != 1.0) {
        value = (value * scalar).trunc();
    }
    value /= precision;
    if let Some(decimals) = display_precision {
        value = round_symmetric(value, Some(f64::from(decimals)));
    }
    if let Some(decimals) = display_precision {
        if !if_required {
            if !value.is_finite() {
                return Ok(lua_number_text(value));
            }
            return Ok(format!(
                "{value:.precision$}",
                precision = usize::from(decimals)
            ));
        }
        Ok(lua_number_text(value))
    } else {
        let decimals = lua_min(2.0, (precision.log10() + 0.001).floor());
        Ok(lua_number_text(round_symmetric(value, Some(decimals))))
    }
}
pub(super) fn lua_min(a: f64, b: f64) -> f64 {
    if a < b { a } else { b }
}
