//! Compiled, byte-oriented selection from the original ModParser.scan procedure.
//!
//! The table contains patterns only. The caller supplies iteration order and resolves
//! the selected payload afterward, including Lua false/nil behavior. Exact-rank ties
//! are exposed so a dictionary loaded without an authoritative iteration order can
//! reject ambiguous payloads instead of silently selecting an arbitrary result.

use crate::lua_pattern::{Capture, LuaPattern, MatchBudget, PatternError, PatternMatch};
use std::ops::Range;

pub const MAX_SCAN_ROWS: usize = 65_536;
pub const MAX_SCAN_PATTERN_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_SCAN_COMPILED_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_SCAN_TEXT_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanError {
    Pattern(PatternError),
    ResourceBound(&'static str),
}
impl From<PatternError> for ScanError {
    fn from(error: PatternError) -> Self {
        Self::Pattern(error)
    }
}
impl std::fmt::Display for ScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pattern(error) => error.fmt(f),
            Self::ResourceBound(bound) => write!(f, "ModParser scan resource bound: {bound}"),
        }
    }
}
impl std::error::Error for ScanError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanCapture {
    /// Captures are from the lowercased byte string, as in ModParser.scan.
    Bytes(Vec<u8>),
    /// Lua position captures are one-based, including the end position length + 1.
    Position(usize),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanMatch<'a> {
    pub row_index: usize,
    /// Zero-based half-open interval in the original and lowercased line.
    pub range: Range<usize>,
    /// Only the first five source captures are retained by scan.
    pub captures: Vec<ScanCapture>,
    /// Other rows with the final winner's start, end and pattern byte length.
    /// The first winner is `row_index`; all ties preserve supplied row order.
    pub tied_rows: Vec<usize>,
    original: &'a [u8],
}
impl ScanMatch<'_> {
    /// Remove the matched bytes while preserving the original line's case.
    pub fn remainder(&self) -> Vec<u8> {
        let mut line = Vec::with_capacity(self.original.len() - self.range.len());
        line.extend_from_slice(&self.original[..self.range.start]);
        line.extend_from_slice(&self.original[self.range.end..]);
        line
    }
}

#[derive(Debug, Clone)]
struct Row {
    pattern: LuaPattern,
    byte_len: usize,
}
/// Immutable compiled patterns can be shared between evaluation workers. Matching
/// state and work budgets belong to each call, with no shared cache or Lua state.
#[derive(Debug, Clone)]
pub struct ScanTable {
    rows: Vec<Row>,
}
impl ScanTable {
    pub fn compile<I, P>(patterns: I) -> Result<Self, ScanError>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<[u8]>,
    {
        let mut rows = Vec::new();
        let mut bytes = 0usize;
        let mut compiled_bytes = 0usize;
        for pattern in patterns {
            if rows.len() == MAX_SCAN_ROWS {
                return Err(ScanError::ResourceBound("pattern rows"));
            }
            let pattern = pattern.as_ref();
            bytes = bytes
                .checked_add(pattern.len())
                .filter(|&total| total <= MAX_SCAN_PATTERN_BYTES)
                .ok_or(ScanError::ResourceBound("total pattern bytes"))?;
            let compiled = LuaPattern::compile(pattern)?;
            compiled_bytes = compiled_bytes
                .checked_add(compiled.compiled_bytes())
                .filter(|&total| total <= MAX_SCAN_COMPILED_BYTES)
                .ok_or(ScanError::ResourceBound("total compiled bytes"))?;
            rows.push(Row {
                pattern: compiled,
                byte_len: pattern.len(),
            });
        }
        Ok(Self { rows })
    }
    pub fn len(&self) -> usize {
        self.rows.len()
    }
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
    /// Read a compiled row without recompiling or copying its pattern.
    pub fn pattern(&self, row: usize) -> Option<&LuaPattern> {
        self.rows.get(row).map(|row| &row.pattern)
    }
    pub fn compiled_bytes(&self) -> usize {
        self.rows.capacity() * std::mem::size_of::<Row>()
            + self
                .rows
                .iter()
                .map(|r| r.pattern.compiled_bytes())
                .sum::<usize>()
    }
    /// Scan every row, including rows after an existing winner. Errors in later
    /// pattern attempts remain observable, as they do in the original source.
    ///
    /// Exact ties keep the first row. This is equivalent to the original only
    /// for the supplied iteration order. A higher-level parser must resolve or
    /// report tied payload ambiguity when original iteration order is unknown.
    pub fn scan<'a>(
        &self,
        line: &'a [u8],
        plain: bool,
        budget: &mut MatchBudget,
    ) -> Result<Option<ScanMatch<'a>>, ScanError> {
        if line.len() > MAX_SCAN_TEXT_BYTES {
            return Err(ScanError::ResourceBound("input bytes"));
        }
        budget.charge(line.len() as u64)?;
        // LuaJIT uses fixed ASCII case tables; Unicode case folding would change
        // both matching and byte positions, including those in source captures.
        let lower: Vec<u8> = line.iter().map(u8::to_ascii_lowercase).collect();
        let mut best: Option<(usize, PatternMatch)> = None;
        let mut tied_rows = Vec::new();
        for (index, row) in self.rows.iter().enumerate() {
            let Some(found) = row.pattern.find(&lower, 1, plain, budget)? else {
                continue;
            };
            let Some((best_index, best_match)) = &best else {
                best = Some((index, found));
                continue;
            };
            let range = found.range();
            let best_range = best_match.range();
            let rank = best_range
                .start
                .cmp(&range.start)
                .then_with(|| range.end.cmp(&best_range.end))
                .then_with(|| row.byte_len.cmp(&self.rows[*best_index].byte_len));
            match rank {
                std::cmp::Ordering::Greater => {
                    best = Some((index, found));
                    tied_rows.clear();
                }
                std::cmp::Ordering::Equal => tied_rows.push(index),
                std::cmp::Ordering::Less => {}
            }
        }
        Ok(best.map(|(row_index, found)| {
            let captures = found
                .captures()
                .iter()
                .take(5)
                .map(|capture| match *capture {
                    Capture::Bytes { start, end } => ScanCapture::Bytes(lower[start..end].to_vec()),
                    Capture::Position(position) => ScanCapture::Position(position),
                })
                .collect();
            ScanMatch {
                row_index,
                range: found.range(),
                captures,
                tied_rows,
                original: line,
            }
        }))
    }
}
