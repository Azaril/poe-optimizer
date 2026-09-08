//! Bounded unit-scalar, rangeless ItemTools preprocessing for already-admitted lines.
//! Exact source keys and formatting parameters come from the injected package.
use poe_optimizer_data::game_data::GameDataPackage;
use poe_optimizer_engine::item_format::format_item_capture;
use std::ops::Range;

/// Preserve captured source values separately from effective item values. Admission
/// occurs before this helper, so unsupported ranges, scalars and numeric syntax do
/// not become supported merely because the formatter can recognize their digits.
pub(crate) fn effective_values(
    line: &str,
    values: &[f64],
    data: &GameDataPackage,
    integer_captures: &[bool],
) -> Result<Vec<f64>, String> {
    if line.len() > 256 || values.len() > 2 || integer_captures.len() != values.len() {
        return Err("item formatting exceeds bounded admitted line shape".into());
    }
    let bytes = line.as_bytes();
    let mut ranges: Vec<Range<usize>> = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at].is_ascii_digit()
            || (bytes[at] == b'-' && bytes.get(at + 1).is_some_and(u8::is_ascii_digit))
        {
            let start = at;
            at += usize::from(bytes[at] == b'-');
            while bytes.get(at).is_some_and(u8::is_ascii_digit) {
                at += 1;
            }
            if bytes.get(at) == Some(&b'.') {
                at += 1;
                while bytes.get(at).is_some_and(u8::is_ascii_digit) {
                    at += 1;
                }
            }
            ranges.push(start..at);
            if ranges.len() > 2 {
                return Err("item formatting found unsupported literal numeric tokens".into());
            }
        } else {
            at += 1;
        }
    }
    if ranges.len() != values.len() {
        return Err("item formatting numeric tokens differ from admitted captures".into());
    }
    let numbers: Vec<f64> = ranges
        .iter()
        .map(|range| {
            line[range.clone()]
                .parse::<f64>()
                .map_err(|_| "invalid item formatting source number".to_owned())
        })
        .collect::<Result<_, _>>()?;
    if numbers
        .iter()
        .zip(values)
        .any(|(number, value)| !number.is_finite() || number.abs() != value.abs())
    {
        return Err("item formatting source numbers differ from admitted captures".into());
    }
    // ItemTools checks all literals first, then left-to-right combinations with
    // fewer literal substitutions, and finally the entirely generic key.
    let masks: &[u8] = match ranges.len() {
        0 => &[0],
        1 => &[1, 0],
        _ => &[3, 1, 2, 0],
    };
    for &mask in masks {
        let mut key = String::with_capacity(line.len());
        let mut end = 0;
        for (index, range) in ranges.iter().enumerate() {
            key.push_str(&line[end..range.start]);
            if mask & (1 << index) != 0 {
                key.push_str(&line[range.clone()]);
            } else {
                key.push('#');
            }
            end = range.end;
        }
        key.push_str(&line[end..]);
        let key = key.replace("+#", "#");
        let mut matching = data
            .item_formatting
            .rules
            .iter()
            .filter(|rule| rule.template == key);
        let Some(rule) = matching.next() else {
            continue;
        };
        if matching.next().is_some() {
            return Err("ambiguous item formatting source key".into());
        }
        if rule.captures.len() != ranges.len() - mask.count_ones() as usize {
            return Err(
                "item formatting policy capture count differs from matched source key".into(),
            );
        }
        let mut output = values.to_vec();
        let mut formats = rule.captures.iter();
        for (index, number) in numbers.iter().enumerate() {
            if mask & (1 << index) != 0 {
                continue;
            }
            let format = formats.next().expect("validated matching capture count");
            let formatted =
                format_item_capture(*number, format.precision, format.display_precision)
                    .map_err(|error| error.to_string())?;
            // The source reparses formatted text. An integer INC pattern rejects
            // forced decimal spelling (even "20.0"), while if-required formatting
            // removes those trailing zeros. Preserve this distinction before IR.
            if integer_captures[index]
                && (formatted.fract() != 0.0
                    || (format.display_precision.is_some_and(|digits| digits > 0)
                        && !format.trim_trailing_zeroes))
            {
                return Err(
                    "item formatting produces text outside the admitted integer capture grammar"
                        .into(),
                );
            }
            // A selected grammar may place its sign in a literal instead of the
            // capture. The whole source number is formatted before that projection.
            output[index] = if number.is_sign_negative() == values[index].is_sign_negative() {
                formatted
            } else {
                -formatted
            };
        }
        return Ok(output);
    }
    Ok(values.to_vec())
}

#[cfg(test)]
#[path = "item_formatting_tests.rs"]
mod tests;
