//! Pure, injected translation of ItemTools text formatting and catalyst scaling.
//! Formatting is independent of item assembly and grants no native mechanic coverage.
//! Parser-dependent precision requires explicit ordered feedback; no parser is hidden here.
mod numeric;
mod patterns;
pub use numeric::{format_value, lua_number_text};
use patterns::{append, number_end, replace_nth, replace_ranges};
use poe_optimizer_data::game_data::ActorData;
use poe_optimizer_data::item_loading::{
    ItemCatalystDefinition, ItemMetadataTable, ItemMetadataValue,
};
use poe_optimizer_data::item_scalability::{CatalystScalingData, ItemScalabilityCatalog};
use std::collections::BTreeSet;

pub const MAX_FORMAT_TEXT: usize = 1024 * 1024;
pub const MAX_FORMAT_CAPTURES: usize = 256;
pub const MAX_FORMAT_CANDIDATES: usize = 65536;
pub const MAX_FORMAT_WORK_BYTES: usize = 64 * 1024 * 1024;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatError {
    SourceError(&'static str),
    ResourceBound(&'static str),
    BindingMismatch,
}
impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SourceError(s) => write!(f, "ItemTools source error: {s}"),
            Self::ResourceBound(s) => write!(f, "ItemTools resource bound: {s}"),
            Self::BindingMismatch => {
                f.write_str("ItemTools fallback belongs to different injected data")
            }
        }
    }
}
impl std::error::Error for FormatError {}
pub type Result<T> = std::result::Result<T, FormatError>;
#[derive(Debug, Clone, Copy)]
pub enum RangeInput<'a> {
    Missing,
    Scalar(f64),
    Values(&'a [Option<f64>]),
}
#[derive(Debug, Clone, Copy)]
pub struct FormatInput<'a> {
    pub line: &'a str,
    pub range: RangeInput<'a>,
    pub value_scalar: Option<f64>,
    pub base_value_scalar: Option<f64>,
}
#[derive(Debug)]
enum OwnedRange {
    Missing,
    Scalar(f64),
    Values(Vec<Option<f64>>),
}
impl OwnedRange {
    fn borrowed(&self) -> RangeInput<'_> {
        match self {
            Self::Missing => RangeInput::Missing,
            Self::Scalar(n) => RangeInput::Scalar(*n),
            Self::Values(v) => RangeInput::Values(v),
        }
    }
}
#[derive(Debug)]
pub struct FormatFallback<'a> {
    catalog: &'a ItemScalabilityCatalog,
    actor: &'a ActorData,
    line: String,
    range: OwnedRange,
    value_scalar: Option<f64>,
    base_value_scalar: Option<f64>,
    parser_text: String,
}
impl FormatFallback<'_> {
    pub fn parser_text(&self) -> &str {
        &self.parser_text
    }
}
#[derive(Debug)]
pub enum FormatResult<'a> {
    Complete(String),
    NeedsParser(FormatFallback<'a>),
}
/// Rows are complete parser results in the caller-observed original iteration order.
/// Extra text presence, including an empty string, disables precision discovery.
#[derive(Debug, Clone, Copy)]
pub struct ParserFeedback<'a> {
    pub modifiers: Option<&'a [ItemMetadataTable]>,
    pub extra: Option<&'a str>,
}
#[derive(Debug, Clone, Copy)]
pub struct ItemFormatter<'a> {
    catalog: &'a ItemScalabilityCatalog,
    actor: &'a ActorData,
}
impl<'a> ItemFormatter<'a> {
    pub fn new(catalog: &'a ItemScalabilityCatalog, actor: &'a ActorData) -> Self {
        Self { catalog, actor }
    }
    pub fn apply_range(&self, input: FormatInput<'_>) -> Result<FormatResult<'a>> {
        check_text(input.line)?;
        if let RangeInput::Values(v) = input.range
            && v.len() > MAX_FORMAT_CAPTURES
        {
            return Err(FormatError::ResourceBound("range table"));
        }
        let mut range_index = 0;
        let resolved = replace_ranges(input.line, true, |r| {
            let selected = match input.range {
                RangeInput::Values(v) => v
                    .get(range_index)
                    .copied()
                    .flatten()
                    .unwrap_or(self.catalog.data().missing_range_value),
                other => scalar_range(other)?,
            };
            range_index += 1;
            let mut value = r.minimum + selected * (r.maximum - r.minimum);
            if r.sign == Some(b'-') {
                value *= -1.0;
            }
            Ok(format!(
                "{}{}",
                if r.sign == Some(b'+') && value > 0.0 {
                    "+"
                } else {
                    ""
                },
                lua_number_text(value)
            ))
        })?;
        let resolved = antonyms(&resolved, true, &self.catalog.data().antonyms)?;
        let (stripped, values) = strip_numbers(&resolved)?;
        if let Some((mut line, mut values)) = self.find_scalable(&stripped, &values)? {
            let key = line.replace("+#", "#");
            let rows = self
                .catalog
                .lookup_exact(&key)
                .expect("selected catalog key");
            for (i, row) in rows.iter().enumerate() {
                let Some(value) = values.get_mut(i) else {
                    return Err(FormatError::SourceError(
                        "scalability row has no corresponding number",
                    ));
                };
                let mut precision = None;
                let mut display = None;
                let mut required = None;
                if let Some(labels) = &row.formats {
                    for label in labels {
                        if let Some(a) = self.catalog.format_assignments(label) {
                            if a.precision.is_some() {
                                precision = a.precision;
                            }
                            if a.display_precision.is_some() {
                                display = a.display_precision;
                            }
                            if a.if_required.is_some() {
                                required = a.if_required;
                            }
                        }
                    }
                }
                let (base, scalar) = if row.is_scalable {
                    (input.base_value_scalar, input.value_scalar)
                } else {
                    (Some(1.0), Some(1.0))
                };
                *value = format_value(
                    parse_number(value)?,
                    base,
                    scalar,
                    precision.unwrap_or(1.0),
                    display,
                    required.unwrap_or(false),
                )?;
            }
            for value in values {
                line = replace_nth(&line, &value, 0)?;
            }
            return Ok(FormatResult::Complete(line));
        }
        let mut precision_same = true;
        let test_line = if input.line.contains('-') {
            let test = replace_ranges(input.line, false, |r| {
                let maximum = r.minimum + scalar_range(input.range)? * (r.maximum - r.minimum);
                let minimum = (maximum + 0.5).floor();
                if minimum != maximum {
                    precision_same = false;
                }
                Ok(format!(
                    "{}{}",
                    if minimum < 0.0 {
                        ""
                    } else if r.sign == Some(b'+') {
                        "+"
                    } else {
                        ""
                    },
                    lua_number_text(minimum)
                ))
            })?;
            antonyms(&test, false, &self.catalog.data().antonyms)?
        } else {
            input.line.into()
        };
        if precision_same
            && input.value_scalar.is_none_or(|n| n == 1.0)
            && input.base_value_scalar.is_none_or(|n| n == 1.0)
        {
            return Ok(FormatResult::Complete(test_line));
        }
        Ok(FormatResult::NeedsParser(FormatFallback {
            catalog: self.catalog,
            actor: self.actor,
            line: input.line.into(),
            range: match input.range {
                RangeInput::Missing => OwnedRange::Missing,
                RangeInput::Scalar(n) => OwnedRange::Scalar(n),
                RangeInput::Values(v) => OwnedRange::Values(v.to_vec()),
            },
            value_scalar: input.value_scalar,
            base_value_scalar: input.base_value_scalar,
            parser_text: test_line,
        }))
    }
    pub fn resume(
        &self,
        fallback: FormatFallback<'_>,
        feedback: ParserFeedback<'_>,
    ) -> Result<String> {
        if !std::ptr::eq(self.catalog, fallback.catalog)
            || !std::ptr::eq(self.actor, fallback.actor)
        {
            return Err(FormatError::BindingMismatch);
        }
        if feedback.extra.is_some_and(|s| s.len() > MAX_FORMAT_TEXT) {
            return Err(FormatError::ResourceBound("parser extra"));
        }
        let mut precision = None;
        if let Some(modifiers) = feedback.modifiers.filter(|_| feedback.extra.is_none()) {
            if modifiers.len() > 65536 {
                return Err(FormatError::ResourceBound("parser modifier rows"));
            }
            for modifier in modifiers {
                let nested = modifier
                    .fields
                    .get("value")
                    .and_then(ItemMetadataValue::as_table)
                    .and_then(|v| v.fields.get("mod"));
                let modifier = match nested {
                    Some(ItemMetadataValue::Table(t)) => t,
                    Some(ItemMetadataValue::Boolean(false)) | None => modifier,
                    // Strings use Lua's string metatable; dense arrays are tables
                    // with no named value field. Neither supplies numeric precision.
                    Some(ItemMetadataValue::Text(_) | ItemMetadataValue::Array(_)) => continue,
                    Some(_) => {
                        return Err(FormatError::SourceError(
                            "nested value.mod is not an indexable modifier",
                        ));
                    }
                };
                if matches!(
                    modifier.fields.get("value"),
                    Some(ItemMetadataValue::Number(_))
                ) {
                    let Some(name) = modifier
                        .fields
                        .get("name")
                        .and_then(ItemMetadataValue::as_str)
                    else {
                        continue;
                    };
                    let Some(kind) = modifier
                        .fields
                        .get("type")
                        .and_then(ItemMetadataValue::as_str)
                    else {
                        continue;
                    };
                    if let Some(operations) = self.actor.high_precision_mods.get(name) {
                        for (operation, p) in operations {
                            if operation.upstream_name() == kind {
                                precision = Some(f64::from(*p));
                            }
                        }
                    }
                }
            }
        }
        if precision.is_none() && has_decimal(&fallback.line) {
            precision = Some(f64::from(self.catalog.data().default_high_precision));
        }
        let mut numbers = 0;
        let line = replace_ranges(&fallback.line, false, |r| {
            numbers += 1;
            let power = 10.0_f64.powf(precision.unwrap_or(0.0));
            let value = ((r.minimum
                + scalar_range(fallback.range.borrowed())? * (r.maximum - r.minimum))
                * power
                + 0.5)
                .floor()
                / power;
            Ok(format!(
                "{}{}",
                if value < 0.0 {
                    ""
                } else if r.sign == Some(b'+') {
                    "+"
                } else {
                    ""
                },
                lua_number_text(value)
            ))
        })?;
        let line = antonyms(&line, false, &self.catalog.data().antonyms)?;
        if numbers == 0 && has_scalable_number(&line) {
            numbers = 1;
        }
        apply_value_scalar(
            &line,
            fallback.value_scalar,
            fallback.base_value_scalar,
            Some(numbers),
            precision,
        )
    }
    fn find_scalable(
        &self,
        line: &str,
        values: &[String],
    ) -> Result<Option<(String, Vec<String>)>> {
        let mut work = Work::default();
        for size in (1..=values.len()).rev() {
            let mut indices: Vec<usize> = (0..size).collect();
            loop {
                let mut candidate = line.to_owned();
                for (removed, &index) in indices.iter().enumerate() {
                    work.charge(candidate.len())?;
                    candidate = replace_nth(&candidate, &values[index], index - removed)?;
                }
                work.candidate(candidate.len())?;
                if self
                    .catalog
                    .lookup_exact(&candidate.replace("+#", "#"))
                    .is_some()
                {
                    let remaining = values
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| !indices.contains(i))
                        .map(|(_, v)| v.clone())
                        .collect();
                    return Ok(Some((candidate, remaining)));
                }
                let Some(at) = (0..size)
                    .rev()
                    .find(|&i| indices[i] < values.len() - size + i)
                else {
                    break;
                };
                indices[at] += 1;
                for i in at + 1..size {
                    indices[i] = indices[i - 1] + 1;
                }
            }
        }
        if self
            .catalog
            .lookup_exact(&line.replace("+#", "#"))
            .is_some()
        {
            Ok(Some((line.into(), values.to_vec())))
        } else {
            Ok(None)
        }
    }
}
#[derive(Default)]
struct Work {
    candidates: usize,
    bytes: usize,
}
impl Work {
    fn charge(&mut self, bytes: usize) -> Result<()> {
        self.bytes = self.bytes.saturating_add(bytes);
        if self.bytes > MAX_FORMAT_WORK_BYTES {
            Err(FormatError::ResourceBound("literal specialization work"))
        } else {
            Ok(())
        }
    }
    fn candidate(&mut self, bytes: usize) -> Result<()> {
        self.candidates += 1;
        if self.candidates > MAX_FORMAT_CANDIDATES {
            return Err(FormatError::ResourceBound(
                "literal specialization candidates",
            ));
        }
        self.charge(bytes)
    }
}
fn check_text(line: &str) -> Result<()> {
    if line.len() > MAX_FORMAT_TEXT {
        Err(FormatError::ResourceBound("input text"))
    } else {
        Ok(())
    }
}
fn parse_number(text: &str) -> Result<f64> {
    text.parse()
        .map_err(|_| FormatError::SourceError("number conversion"))
}
fn scalar_range(range: RangeInput<'_>) -> Result<f64> {
    match range {
        RangeInput::Scalar(n) => Ok(n),
        RangeInput::Missing => Err(FormatError::SourceError("arithmetic on missing range")),
        RangeInput::Values(_) => Err(FormatError::SourceError(
            "arithmetic on range table in fallback",
        )),
    }
}
fn strip_numbers(line: &str) -> Result<(String, Vec<String>)> {
    let mut values = Vec::new();
    let mut result = String::new();
    let mut at = 0;
    let mut last = 0;
    while at < line.len() {
        if let Some(end) = number_end(line.as_bytes(), at, true) {
            if values.len() >= MAX_FORMAT_CAPTURES {
                return Err(FormatError::ResourceBound("number captures"));
            }
            append(&mut result, &line[last..at])?;
            append(&mut result, "#")?;
            values.push(line[at..end].into());
            at = end;
            last = end;
        } else {
            at += 1;
        }
    }
    append(&mut result, &line[last..])?;
    Ok((result, values))
}
fn antonyms(
    line: &str,
    decimal: bool,
    mapping: &std::collections::BTreeMap<String, String>,
) -> Result<String> {
    let b = line.as_bytes();
    let mut output = String::new();
    let mut at = 0;
    let mut last = 0;
    while at < b.len() {
        if b[at] == b'-'
            && let Some(mut end) = number_end(b, at + 1, false)
        {
            if !decimal && line[at + 1..end].contains('.') {
                at += 1;
                continue;
            }
            if b.get(end) == Some(&b'%') && b.get(end + 1) == Some(&b' ') {
                let number_end = end + 1;
                end += 2;
                let word_start = end;
                while b.get(end).is_some_and(u8::is_ascii_alphabetic) {
                    end += 1;
                }
                if word_start != end {
                    if let Some(word) = mapping.get(&line[word_start..end]) {
                        append(&mut output, &line[last..at])?;
                        append(&mut output, &line[at + 1..number_end])?;
                        append(&mut output, " ")?;
                        append(&mut output, word)?;
                        last = end;
                    }
                    at = end;
                    continue;
                }
            }
        }
        at += 1;
    }
    append(&mut output, &line[last..])?;
    Ok(output)
}
fn has_decimal(line: &str) -> bool {
    let b = line.as_bytes();
    b.windows(2).any(|v| v[0].is_ascii_digit() && v[1] == b'.')
}
fn has_scalable_number(line: &str) -> bool {
    let bytes = line.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        if let Some(end) = number_end(bytes, at, false) {
            if bytes.get(end) == Some(&b' ')
                || (bytes.get(end) == Some(&b'%') && bytes.get(end + 1) == Some(&b' '))
            {
                return true;
            }
            // Every later digit in this same match has the same trailing
            // delimiter. Do not rescan an arbitrarily long digit run.
            at = end;
        } else {
            at += 1;
        }
    }
    false
}
/// Original fallback scalar transform, including the integer-pattern suffix/backtracking rule.
pub fn apply_value_scalar(
    line: &str,
    value_scalar: Option<f64>,
    base_value_scalar: Option<f64>,
    numbers: Option<usize>,
    precision: Option<f64>,
) -> Result<String> {
    check_text(line)?;
    let scalar = value_scalar.unwrap_or(1.0);
    if scalar == 1.0 && base_value_scalar.is_none_or(|n| n == 1.0) {
        return Ok(line.into());
    }
    let limit = numbers.unwrap_or(usize::MAX);
    let mut count = 0;
    let mut output = String::new();
    let b = line.as_bytes();
    let mut at = 0;
    let mut last = 0;
    while at < b.len() && count < limit {
        if !b[at].is_ascii_digit() {
            at += 1;
            continue;
        }
        let (number_end, match_end) = if precision.is_some() {
            let end = number_end(b, at, false).expect("digit");
            (end, end)
        } else {
            let mut end = at;
            while b.get(end).is_some_and(u8::is_ascii_digit) {
                end += 1;
            }
            while end > at && (end == b.len() || b[end] == b'.') {
                end -= 1;
            }
            if end == at {
                at += 1;
                continue;
            }
            (
                end,
                end + line[end..].chars().next().expect("suffix").len_utf8(),
            )
        };
        let mut value = parse_number(&line[at..number_end])?;
        if let Some(precision) = precision {
            let power = 10.0_f64.powf(precision);
            if let Some(base) = base_value_scalar {
                value = (value * base * power + 0.5).floor() / power;
            }
            value = (value * scalar * power).floor() / power;
        } else {
            if let Some(base) = base_value_scalar {
                value = (value * base + 0.5).floor();
            }
            value = (value * scalar + 0.001).floor();
        }
        append(&mut output, &line[last..at])?;
        append(&mut output, &lua_number_text(value))?;
        append(&mut output, &line[number_end..match_end])?;
        count += 1;
        at = match_end;
        last = at;
    }
    append(&mut output, &line[last..])?;
    Ok(output)
}
/// Source catalyst guard and one any-tag match; flags augment only a nonempty tag list.
pub fn catalyst_scalar(
    policy: &CatalystScalingData,
    catalysts: &[ItemCatalystDefinition],
    catalyst_id: Option<f64>,
    tags: Option<&[String]>,
    flags: &BTreeSet<String>,
    unscalable: bool,
    quality: Option<f64>,
) -> Result<f64> {
    if unscalable {
        return Ok(policy.neutral_scalar);
    }
    let Some(id) = catalyst_id
        .filter(|n| n.is_finite() && *n >= 1.0 && n.fract() == 0.0 && *n <= catalysts.len() as f64)
    else {
        return Ok(policy.neutral_scalar);
    };
    let Some(tags) = tags.filter(|v| !v.is_empty()) else {
        return Ok(policy.neutral_scalar);
    };
    if tags.len() > 65536 || flags.len() > 65536 {
        return Err(FormatError::ResourceBound("catalyst tags"));
    }
    let catalyst = &catalysts[id as usize - 1];
    if catalyst.tags.iter().any(|tag| {
        tags.contains(tag) || (policy.extra_tag_flags.contains(tag) && flags.contains(tag))
    }) {
        Ok(
            (policy.percent_offset + quality.unwrap_or(policy.default_quality))
                / policy.percent_divisor,
        )
    } else {
        Ok(policy.neutral_scalar)
    }
}
/// Files entering an adapter's source fingerprint; normalized by that adapter.
pub fn implementation_sources() -> [&'static str; 3] {
    [
        include_str!("item_tools.rs"),
        include_str!("item_tools/numeric.rs"),
        include_str!("item_tools/patterns.rs"),
    ]
}
