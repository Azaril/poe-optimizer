use super::{FormatError, MAX_FORMAT_TEXT, Result};
use std::ops::Range;

pub(super) fn number_end(bytes: &[u8], start: usize, signed: bool) -> Option<usize> {
    let mut end = start;
    if signed && bytes.get(end) == Some(&b'-') {
        end += 1;
    }
    let first = end;
    while bytes.get(end).is_some_and(u8::is_ascii_digit) {
        end += 1;
    }
    if first == end {
        return None;
    }
    if bytes.get(end) == Some(&b'.') {
        end += 1;
        while bytes.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
    }
    Some(end)
}
#[derive(Debug)]
pub(super) struct RangeMatch {
    pub span: Range<usize>,
    pub sign: Option<u8>,
    pub minimum: f64,
    pub maximum: f64,
}
pub(super) fn range_at(line: &str, start: usize, minus_sign: bool) -> Option<RangeMatch> {
    let b = line.as_bytes();
    let mut at = start;
    let sign = match b.get(at) {
        Some(b'+') => {
            at += 1;
            Some(b'+')
        }
        Some(b'-') if minus_sign => {
            at += 1;
            Some(b'-')
        }
        _ => None,
    };
    if b.get(at) != Some(&b'(') {
        return None;
    }
    at += 1;
    let min_start = at;
    at = number_end(b, at, true)?;
    let minimum = line[min_start..at].parse().ok()?;
    if b.get(at) != Some(&b'-') {
        return None;
    }
    at += 1;
    let max_start = at;
    at = number_end(b, at, true)?;
    let maximum = line[max_start..at].parse().ok()?;
    if b.get(at) != Some(&b')') {
        return None;
    }
    Some(RangeMatch {
        span: start..at + 1,
        sign,
        minimum,
        maximum,
    })
}
pub(super) fn replace_ranges(
    line: &str,
    minus_sign: bool,
    mut replacement: impl FnMut(&RangeMatch) -> Result<String>,
) -> Result<String> {
    let mut output = String::new();
    let mut at = 0;
    let mut last = 0;
    while at < line.len() {
        if let Some(found) = range_at(line, at, minus_sign) {
            append(&mut output, &line[last..at])?;
            append(&mut output, &replacement(&found)?)?;
            at = found.span.end;
            last = at;
        } else {
            at += 1;
        }
    }
    append(&mut output, &line[last..])?;
    Ok(output)
}
pub(super) fn append(output: &mut String, text: &str) -> Result<()> {
    if output.len().saturating_add(text.len()) > MAX_FORMAT_TEXT {
        return Err(FormatError::ResourceBound("output text"));
    }
    output.push_str(text);
    Ok(())
}
pub(super) fn replace_nth(line: &str, replacement: &str, n: usize) -> Result<String> {
    let Some((at, _)) = line.match_indices('#').nth(n) else {
        return Ok(line.into());
    };
    let mut result = String::new();
    append(&mut result, &line[..at])?;
    append(&mut result, replacement)?;
    append(&mut result, &line[at + 1..])?;
    Ok(result)
}
