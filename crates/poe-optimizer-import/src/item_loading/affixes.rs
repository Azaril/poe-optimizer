//! Compiled source affix grammar. Definitions remain in the selected data catalog.
use super::{ItemAffix, ItemAffixRange, ItemNumber, syntax};
use poe_optimizer_data::item_loading::{ItemAffixLoadingPolicy, ItemAffixSide};
use poe_optimizer_engine::lua_pattern::{
    Capture, LuaPattern, MatchBudget, PatternError, PatternMatch,
};
use std::borrow::Cow;

pub(super) const MAX_AFFIX_RANGE_VALUES: usize = 8192;
const MAX_COMPILED_BYTES: usize = 4 * 1024 * 1024;
#[derive(Debug)]
pub(super) enum AffixError {
    Pattern(PatternError),
    Source(&'static str),
    Unsupported(&'static str),
    Resource(&'static str),
}
impl From<PatternError> for AffixError {
    fn from(value: PatternError) -> Self {
        Self::Pattern(value)
    }
}
impl std::fmt::Display for AffixError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pattern(value) => value.fmt(f),
            Self::Source(value) | Self::Unsupported(value) | Self::Resource(value) => {
                f.write_str(value)
            }
        }
    }
}
struct LimitProgram {
    side: ItemAffixSide,
    matches: LuaPattern,
    positive: LuaPattern,
    negative: LuaPattern,
}
pub(super) struct AffixPrograms {
    fractured: LuaPattern,
    fractured_remove: LuaPattern,
    range: LuaPattern,
    range_values: LuaPattern,
    limits: Vec<LimitProgram>,
    pub compiled_bytes: usize,
}
impl AffixPrograms {
    pub fn compile(policy: &ItemAffixLoadingPolicy) -> Result<Self, AffixError> {
        let mut bytes = 0usize;
        let mut compile = |text: &str| -> Result<LuaPattern, AffixError> {
            let pattern = LuaPattern::compile(text.as_bytes())?;
            bytes = bytes
                .checked_add(pattern.compiled_bytes())
                .filter(|n| *n <= MAX_COMPILED_BYTES)
                .ok_or(AffixError::Resource(
                    "affix compiled pattern aggregate bound",
                ))?;
            Ok(pattern)
        };
        let reservations = policy
            .other_header_patterns
            .iter()
            .map(|text| compile(text))
            .collect::<Result<Vec<_>, _>>()?;
        let mut reservation_budget = MatchBudget::default();
        for header in policy.headers.keys() {
            for pattern in &reservations {
                let matched =
                    match pattern.match_captures(header.as_bytes(), 1, &mut reservation_budget) {
                        Ok(matched) => matched,
                        // This is definition preflight, not an executed Item branch.
                        // A malformed reservation cannot establish safe dispatch.
                        Err(PatternError::Source(_)) => {
                            return Err(AffixError::Unsupported(
                                "reserved source header pattern cannot be validated",
                            ));
                        }
                        Err(error) => return Err(error.into()),
                    };
                if matched.is_some() {
                    return Err(AffixError::Unsupported(
                        "affix header overlaps a reserved source header pattern",
                    ));
                }
            }
        }
        let fractured = compile(&policy.fractured_pattern)?;
        let fractured_remove = compile(&policy.fractured_remove_pattern)?;
        let range = compile(&policy.range_pattern)?;
        // gmatch does not interpret a leading caret as a start anchor.
        let values = if policy.range_value_pattern.starts_with('^') {
            format!("%{}", policy.range_value_pattern)
        } else {
            policy.range_value_pattern.clone()
        };
        let range_values = compile(&values)?;
        let limits = policy
            .limit_rules
            .iter()
            .map(|rule| {
                Ok(LimitProgram {
                    side: rule.side,
                    matches: compile(&rule.match_pattern)?,
                    positive: compile(&rule.positive_pattern)?,
                    negative: compile(&rule.negative_pattern)?,
                })
            })
            .collect::<Result<Vec<_>, AffixError>>()?;
        Ok(Self {
            fractured,
            fractured_remove,
            range,
            range_values,
            limits,
            compiled_bytes: bytes,
        })
    }
    pub fn header(
        &self,
        value: &str,
        policy: &ItemAffixLoadingPolicy,
        default_quality: f64,
        budget: &mut MatchBudget,
    ) -> Result<ItemAffix, AffixError> {
        let fractured = self
            .fractured
            .match_captures(value.as_bytes(), 1, budget)?
            .map(|_| true);
        let value = remove_matches(&self.fractured_remove, value.as_bytes(), budget)?;
        let matched = self.range.match_captures(&value, 1, budget)?;
        let captures = matched.as_ref().map_or(&[][..], PatternMatch::captures);
        let range_text = match captures.first() {
            Some(Capture::Bytes { start, end }) => Some(&value[*start..*end]),
            Some(Capture::Position(_)) => {
                return Err(AffixError::Source("attempt to index numeric affix range"));
            }
            None => None,
        };
        let id = match captures.get(1) {
            Some(Capture::Bytes { start, end }) => &value[*start..*end],
            Some(Capture::Position(_)) => {
                return Err(AffixError::Unsupported(
                    "numeric affix identity capture is not represented",
                ));
            }
            None => value.as_ref(),
        };
        let mod_id = std::str::from_utf8(id)
            .map_err(|_| AffixError::Unsupported("affix identity is not UTF-8"))?
            .to_owned();
        let mut range = None;
        if let Some(text) = range_text {
            let separator = policy.range_separator.as_bytes();
            if separator.is_empty() || text.windows(separator.len()).any(|part| part == separator) {
                let mut values = Vec::new();
                let mut at = 0usize;
                while at <= text.len() {
                    let Some(found) = self.range_values.match_captures(text, index(at)?, budget)?
                    else {
                        break;
                    };
                    let number = capture_number(found.captures().first(), text);
                    if number != ItemNumber::Nil {
                        if values.len() >= MAX_AFFIX_RANGE_VALUES {
                            return Err(AffixError::Resource(
                                "independent affix range count bound",
                            ));
                        }
                        values.push(number);
                    }
                    let span = found.range();
                    at = if span.start == span.end {
                        span.end + 1
                    } else {
                        span.end
                    };
                }
                range = Some(ItemAffixRange::Independent(values));
            } else {
                let value = bytes_number(text);
                if value != ItemNumber::Nil {
                    range = Some(ItemAffixRange::Scalar(value));
                }
            }
        }
        if range.is_none() && mod_id != policy.none_mod_id {
            range = Some(ItemAffixRange::Scalar(ItemNumber::new(default_quality)));
        }
        Ok(ItemAffix {
            mod_id,
            range,
            fractured,
        })
    }
    /// Return the first branch and its independent positive/negative operands.
    /// Caller applies (old or default) + positive - negative in source order.
    pub fn limit_effect(
        &self,
        text: &str,
        default: f64,
        budget: &mut MatchBudget,
    ) -> Result<Option<(ItemAffixSide, f64, f64)>, AffixError> {
        for rule in &self.limits {
            if rule
                .matches
                .match_captures(text.as_bytes(), 1, budget)?
                .is_none()
            {
                continue;
            }
            let positive = rule.positive.match_captures(text.as_bytes(), 1, budget)?;
            let positive = capture_number(
                positive.as_ref().and_then(|m| m.captures().first()),
                text.as_bytes(),
            )
            .value()
            .unwrap_or(default);
            let negative = rule.negative.match_captures(text.as_bytes(), 1, budget)?;
            let negative = capture_number(
                negative.as_ref().and_then(|m| m.captures().first()),
                text.as_bytes(),
            )
            .value()
            .unwrap_or(default);
            return Ok(Some((rule.side, positive, negative)));
        }
        Ok(None)
    }
}
fn index(at: usize) -> Result<i32, AffixError> {
    at.checked_add(1)
        .and_then(|n| i32::try_from(n).ok())
        .ok_or(AffixError::Resource("affix pattern index bound"))
}
fn bytes_number(bytes: &[u8]) -> ItemNumber {
    std::str::from_utf8(bytes).map_or(ItemNumber::Nil, syntax::lua_number)
}
fn capture_number(capture: Option<&Capture>, text: &[u8]) -> ItemNumber {
    match capture {
        Some(Capture::Bytes { start, end }) => bytes_number(&text[*start..*end]),
        Some(Capture::Position(index)) => ItemNumber::new(*index as f64),
        None => ItemNumber::Nil,
    }
}
/// Lua gsub with an empty replacement. It preserves unmatched bytes, advances
/// one byte after an empty match, and applies an anchored pattern at most once.
fn remove_matches<'a>(
    pattern: &LuaPattern,
    text: &'a [u8],
    budget: &mut MatchBudget,
) -> Result<Cow<'a, [u8]>, AffixError> {
    let Some(mut found) = pattern.match_captures(text, 1, budget)? else {
        return Ok(Cow::Borrowed(text));
    };
    let mut out = Vec::with_capacity(text.len());
    let mut copied = 0;
    loop {
        let span = found.range();
        out.extend_from_slice(&text[copied..span.start]);
        copied = span.end;
        if span.start == span.end {
            if copied == text.len() {
                break;
            }
            out.push(text[copied]);
            copied += 1;
        }
        if pattern.source().first() == Some(&b'^') {
            break;
        }
        let Some(next) = pattern.match_captures(text, index(copied)?, budget)? else {
            break;
        };
        found = next;
    }
    out.extend_from_slice(&text[copied..]);
    Ok(Cow::Owned(out))
}
