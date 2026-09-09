use serde::{Serialize, Serializer, ser::SerializeMap};
use std::collections::BTreeSet;
/// Lua number observations retain nil and IEEE nonfinite values explicitly.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ItemNumber {
    Nil,
    Finite(f64),
    PositiveInfinity,
    NegativeInfinity,
    NaN,
}
impl Serialize for ItemNumber {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let value = match *self {
            Self::Finite(n) => Self::new(n),
            value => value,
        };
        let mut map = serializer.serialize_map(Some(if matches!(value, Self::Finite(_)) {
            2
        } else {
            1
        }))?;
        map.serialize_entry(
            "kind",
            match value {
                Self::Nil => "nil",
                Self::Finite(_) => "finite",
                Self::PositiveInfinity => "positive_infinity",
                Self::NegativeInfinity => "negative_infinity",
                Self::NaN => "nan",
            },
        )?;
        if let Self::Finite(n) = value {
            map.serialize_entry("value", &n)?;
        }
        map.end()
    }
}
impl ItemNumber {
    pub(super) fn canonical(self) -> bool {
        !matches!(self,Self::Finite(n)if !n.is_finite())
    }
    pub fn new(value: f64) -> Self {
        if value.is_nan() {
            Self::NaN
        } else if value == f64::INFINITY {
            Self::PositiveInfinity
        } else if value == f64::NEG_INFINITY {
            Self::NegativeInfinity
        } else {
            Self::Finite(value)
        }
    }
    pub fn value(self) -> Option<f64> {
        match self {
            Self::Nil => None,
            Self::Finite(n) => Some(n),
            Self::PositiveInfinity => Some(f64::INFINITY),
            Self::NegativeInfinity => Some(f64::NEG_INFINITY),
            Self::NaN => Some(f64::NAN),
        }
    }
}
/// Source specToNumber: leading signed digits/dots only; no exponent capture.
pub fn spec_to_number(text: &str) -> ItemNumber {
    let bytes = text.as_bytes();
    let mut end = usize::from(matches!(bytes.first(), Some(b'+') | Some(b'-')));
    let begin = end;
    while bytes
        .get(end)
        .is_some_and(|b| b.is_ascii_digit() || *b == b'.')
    {
        end += 1;
    }
    if end == begin {
        return ItemNumber::Nil;
    }
    text[..end]
        .parse::<f64>()
        .map(ItemNumber::new)
        .unwrap_or(ItemNumber::Nil)
}
pub(super) fn lua_number(text: &str) -> ItemNumber {
    let text = text.trim_matches(ascii_space);
    if text.is_empty() {
        return ItemNumber::Nil;
    }
    let (negative, body) = if let Some(t) = text.strip_prefix('-') {
        (true, t)
    } else {
        (false, text.strip_prefix('+').unwrap_or(text))
    };
    if body.eq_ignore_ascii_case("inf") || body.eq_ignore_ascii_case("infinity") {
        return ItemNumber::new(if negative {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        });
    }
    if body.eq_ignore_ascii_case("nan") {
        return ItemNumber::NaN;
    }
    if let Some(hex) = body.strip_prefix("0x").or_else(|| body.strip_prefix("0X")) {
        return hex_number(hex, negative);
    }
    if !body.bytes().any(|b| b.is_ascii_digit())
        || body
            .bytes()
            .any(|b| !b.is_ascii_digit() && !matches!(b, b'.' | b'e' | b'E' | b'+' | b'-'))
    {
        return ItemNumber::Nil;
    }
    text.parse::<f64>()
        .map(ItemNumber::new)
        .unwrap_or(ItemNumber::Nil)
}
fn hex_number(text: &str, negative: bool) -> ItemNumber {
    let (mantissa, exponent) = if let Some(at) = text.find(['p', 'P']) {
        (&text[..at], Some(&text[at + 1..]))
    } else {
        (text, None)
    };
    let exponent = if let Some(e) = exponent {
        let (neg, e) = if let Some(e) = e.strip_prefix('-') {
            (true, e)
        } else {
            (false, e.strip_prefix('+').unwrap_or(e))
        };
        if e.is_empty() || !e.bytes().all(|b| b.is_ascii_digit()) {
            return ItemNumber::Nil;
        }
        let mut n = 0i64;
        for b in e.bytes() {
            n = (n * 10 + i64::from(b - b'0')).min(1_000_000_000);
        }
        if neg { -n } else { n }
    } else {
        0
    };
    let mut digits = Vec::new();
    let mut after_dot = false;
    let mut fraction = 0i64;
    for b in mantissa.bytes() {
        if b == b'.' && !after_dot {
            after_dot = true;
        } else if b.is_ascii_hexdigit() {
            digits.push((b as char).to_digit(16).unwrap_or(0) as u8);
            fraction += i64::from(after_dot);
        } else {
            return ItemNumber::Nil;
        }
    }
    if digits.is_empty() {
        return ItemNumber::Nil;
    }
    let sign = if negative { 1u64 << 63 } else { 0 };
    let Some(first) = digits.iter().position(|&n| n != 0) else {
        return ItemNumber::new(f64::from_bits(sign));
    };
    let high = 7 - digits[first].leading_zeros() as i64;
    let mut exp = exponent - fraction * 4 + high + 4 * (digits.len() - first - 1) as i64;
    if exp > 1023 {
        return ItemNumber::new(f64::from_bits(sign | 0x7ff0_0000_0000_0000));
    }
    if exp < -1075 {
        return ItemNumber::new(f64::from_bits(sign));
    }
    let precision = (exp + 1075).clamp(0, 53) as usize;
    let mut significand = 0u64;
    let mut position = 0usize;
    let mut guard = false;
    let mut sticky = false;
    for (index, &digit) in digits[first..].iter().enumerate() {
        let width = if index == 0 { high as usize + 1 } else { 4 };
        for bit in (0..width).rev() {
            let one = digit & (1 << bit) != 0;
            if position < precision {
                significand = (significand << 1) | u64::from(one);
            } else if position == precision {
                guard = one;
            } else {
                sticky |= one;
            }
            position += 1;
        }
    }
    if position < precision {
        significand <<= precision - position;
    }
    if guard && (sticky || significand & 1 != 0) {
        significand += 1;
    }
    let bits = if exp < -1022 {
        significand
    } else {
        if significand == (1u64 << 53) {
            significand >>= 1;
            exp += 1;
        }
        if exp > 1023 {
            0x7ff0_0000_0000_0000
        } else {
            ((exp + 1023) as u64) << 52 | (significand & 0x000f_ffff_ffff_ffff)
        }
    };
    ItemNumber::new(f64::from_bits(sign | bits))
}
pub(super) fn ascii_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r' | '\n' | '\x0b' | '\x0c')
}
pub(super) fn raw_lines(raw: &str) -> Vec<String> {
    raw.split('\n')
        .filter_map(|line| {
            let line = line.trim_matches(ascii_space);
            if line.is_empty() {
                None
            } else {
                Some(escape_ggg(line))
            }
        })
        .collect()
}
pub(super) fn parse_spec(line: &str) -> Option<(&str, &str)> {
    if let Some((name, value)) = line.split_once(": ") {
        if !value.is_empty()
            && !name.strip_suffix(':').unwrap_or(name).is_empty()
            && name
                .strip_suffix(':')
                .unwrap_or(name)
                .bytes()
                .all(|b| b.is_ascii_alphabetic() || matches!(b, b' ' | b'(' | b')'))
        {
            return Some((
                if name == "Class:" {
                    "Requires Class"
                } else {
                    name
                },
                value,
            ));
        }
        if name == "Class:" && !value.is_empty() {
            return Some(("Requires Class", value));
        }
    }
    let rest = line.strip_prefix("Requires ")?;
    let (name, value) = rest.split_once(' ')?;
    if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphabetic()) || value.is_empty() {
        return None;
    }
    Some((&line[..9 + name.len()], value))
}
pub(super) fn ids(text: &str, positive: bool) -> BTreeSet<u32> {
    text.split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .filter(|&n| !positive || n > 0)
        .collect()
}
fn substitute(
    mut text: String,
    open: &str,
    close: &str,
    mut value: impl FnMut(&str) -> Option<String>,
) -> String {
    let mut at = 0;
    while let Some(start) = text[at..].find(open).map(|n| at + n) {
        let Some(end) = text[start + open.len()..]
            .find(close)
            .map(|n| start + open.len() + n)
        else {
            break;
        };
        if let Some(replacement) = value(&text[start + open.len()..end]) {
            text.replace_range(start..end + close.len(), &replacement);
            at = start + replacement.len();
        } else {
            at = end + close.len();
        }
    }
    text
}
fn escape_ggg(text: &str) -> String {
    let mut out = text.to_owned();
    let mut at = 0;
    while let Some(start) = out[at..].find('<').map(|n| at + n) {
        let Some(end) = out[start + 1..].find('>').map(|n| start + 1 + n) else {
            break;
        };
        if end > start + 1
            && out[end + 1..].starts_with('{')
            && let Some(last) = out[end + 2..]
                .find('}')
                .map(|n| end + 2 + n)
                .filter(|&n| n > end + 2)
        {
            let replacement = out[end + 2..last].to_owned();
            out.replace_range(start..last + 1, &replacement);
            at = start + replacement.len();
            continue;
        }
        at = end + 1;
    }
    out = substitute(out, "[", "]", |s| {
        if !s.is_empty() && !s.contains('|') {
            Some(s.into())
        } else {
            None
        }
    });
    let mut at = 0;
    while let Some(start) = out[at..].find('[').map(|n| at + n) {
        let Some(pipe) = out[start + 1..].find('|').map(|n| start + 1 + n) else {
            break;
        };
        let limit = out[pipe + 1..]
            .find('|')
            .map_or(out.len(), |n| pipe + 1 + n);
        if pipe > start + 1
            && let Some(end) = out[pipe + 1..limit]
                .rfind(']')
                .map(|n| pipe + 1 + n)
                .filter(|&n| n > pipe + 1)
        {
            let replacement = out[pipe + 1..end].to_owned();
            out.replace_range(start..end + 1, &replacement);
            at = start + replacement.len();
            continue;
        }
        at = start + 1;
    }
    out
}
pub(super) fn remove_braces(text: &str) -> String {
    substitute(text.to_owned(), "{", "}", |_| Some(String::new()))
}
pub(super) fn strip_next(text: &str) -> String {
    let mut value = remove_braces(text);
    let mut at = 0;
    while let Some(start) = value[at..].find('(').map(|n| at + n) {
        let Some(end) = value[start + 1..].find(')').map(|n| start + 1 + n) else {
            break;
        };
        if end > start + 1
            && value[start + 1..end]
                .bytes()
                .all(|b| b.is_ascii_lowercase())
        {
            let first = if start > 0 && value.as_bytes()[start - 1] == b' ' {
                start - 1
            } else {
                start
            };
            value.replace_range(first..end + 1, "");
            at = first;
        } else {
            at = end + 1;
        }
    }
    value
}

pub(super) fn strip_name_parentheses(text: &str) -> String {
    let mut value = text.to_owned();
    let mut at = 0;
    while let Some(start) = value[at..].find(" (").map(|n| at + n) {
        let Some(end) = value[start + 2..]
            .rfind(')')
            .map(|n| start + 2 + n)
            .filter(|&n| n > start + 2)
        else {
            break;
        };
        value.replace_range(start..end + 1, "");
        at = start;
    }
    value
}

pub(super) fn lua_min(a: f64, b: f64) -> f64 {
    if a < b { a } else { b }
}
pub(super) fn lua_max(a: f64, b: f64) -> f64 {
    if a > b { a } else { b }
}
