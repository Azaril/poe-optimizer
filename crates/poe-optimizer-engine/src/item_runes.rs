//! Bounded rune text and vector operations from the pinned Item.lua helpers.
//! Grammar, tolerance, candidate order and count limits are caller supplied.
//! No item identity, parser, sorting policy or native build admission lives here.
use crate::item_tools::lua_number_text;
use crate::lua_number::parse_number;
use crate::lua_pattern::{
    Capture, CompileLimits, LuaPattern, MatchBudget, MatchLimits, PatternError, PatternMatch,
};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuneError {
    Pattern(PatternError),
    Source(&'static str),
    Resource(&'static str),
}
impl From<PatternError> for RuneError {
    fn from(value: PatternError) -> Self {
        Self::Pattern(value)
    }
}
impl std::fmt::Display for RuneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pattern(e) => e.fmt(f),
            Self::Source(s) => write!(f, "rune source error: {s}"),
            Self::Resource(s) => write!(f, "rune resource bound: {s}"),
        }
    }
}
impl std::error::Error for RuneError {}
pub type Result<T> = std::result::Result<T, RuneError>;

#[derive(Clone, Copy, Debug)]
pub struct RuneLimits {
    pub matching: MatchLimits,
    pub max_output_bytes: usize,
    pub max_callbacks: u64,
    pub max_vector_components: usize,
    pub max_vector_work: u64,
    pub max_candidates: usize,
    pub max_search_depth: usize,
    pub max_search_steps: u64,
}
impl Default for RuneLimits {
    fn default() -> Self {
        Self {
            matching: MatchLimits::default(),
            max_output_bytes: 8 * 1024 * 1024,
            max_callbacks: 65536,
            max_vector_components: 4096,
            max_vector_work: 8_000_000,
            max_candidates: 4096,
            max_search_depth: 128,
            max_search_steps: 1_000_000,
        }
    }
}
/// Cumulative budget. Reuse it throughout one item operation, including failures.
#[derive(Clone, Debug)]
pub struct RuneBudget {
    limits: RuneLimits,
    matching: MatchBudget,
    output_bytes: usize,
    callbacks: u64,
    vector_work: u64,
    search_steps: u64,
}
impl Default for RuneBudget {
    fn default() -> Self {
        Self::new(RuneLimits::default())
    }
}
impl RuneBudget {
    pub fn new(limits: RuneLimits) -> Self {
        Self {
            matching: MatchBudget::new(limits.matching),
            limits,
            output_bytes: 0,
            callbacks: 0,
            vector_work: 0,
            search_steps: 0,
        }
    }
    pub fn match_steps_used(&self) -> u64 {
        self.matching.steps_used()
    }
    pub fn output_bytes_used(&self) -> usize {
        self.output_bytes
    }
    pub fn callbacks_used(&self) -> u64 {
        self.callbacks
    }
    pub fn search_steps_used(&self) -> u64 {
        self.search_steps
    }
    fn append(&mut self, out: &mut Vec<u8>, bytes: &[u8]) -> Result<()> {
        self.output_bytes = self
            .output_bytes
            .checked_add(bytes.len())
            .ok_or(RuneError::Resource("output bytes"))?;
        if self.output_bytes > self.limits.max_output_bytes {
            return Err(RuneError::Resource("output bytes"));
        }
        out.try_reserve_exact(bytes.len())
            .map_err(|_| RuneError::Resource("output allocation"))?;
        out.extend_from_slice(bytes);
        Ok(())
    }
    fn callback(&mut self) -> Result<()> {
        charge(
            &mut self.callbacks,
            1,
            self.limits.max_callbacks,
            "callbacks",
        )
    }
    fn vectors(&mut self, components: usize) -> Result<()> {
        if components > self.limits.max_vector_components {
            return Err(RuneError::Resource("vector components"));
        }
        charge(
            &mut self.vector_work,
            components as u64,
            self.limits.max_vector_work,
            "vector work",
        )
    }
    fn search(&mut self) -> Result<()> {
        charge(
            &mut self.search_steps,
            1,
            self.limits.max_search_steps,
            "search steps",
        )
    }
}
fn charge(used: &mut u64, n: u64, limit: u64, name: &'static str) -> Result<()> {
    *used = used.checked_add(n).ok_or(RuneError::Resource(name))?;
    if *used > limit {
        Err(RuneError::Resource(name))
    } else {
        Ok(())
    }
}
fn lua_index(offset: usize) -> Result<i32> {
    offset
        .checked_add(1)
        .and_then(|n| i32::try_from(n).ok())
        .ok_or(RuneError::Resource("subject index"))
}
fn number_capture(subject: &[u8], capture: Capture) -> Option<f64> {
    match capture {
        Capture::Bytes { start, end } => parse_number(&subject[start..end]),
        Capture::Position(n) => Some(n as f64),
    }
}
fn callback_capture(found: &PatternMatch) -> Capture {
    found.captures().first().copied().unwrap_or_else(|| {
        let range = found.range();
        Capture::Bytes {
            start: range.start,
            end: range.end,
        }
    })
}

#[derive(Clone, Debug)]
pub struct RuneText {
    pattern: LuaPattern,
}
#[derive(Clone, Debug, PartialEq)]
pub struct RuneLineParts {
    pub stripped: Vec<u8>,
    pub values: Vec<f64>,
}
impl RuneText {
    pub fn compiled_bytes(&self) -> usize {
        self.pattern.compiled_bytes()
    }
    pub fn compile(pattern: &[u8], limits: CompileLimits) -> Result<Self> {
        Ok(Self {
            pattern: LuaPattern::compile_with_limits(pattern, limits)?,
        })
    }
    /// Lua gsub callback semantics: first capture, or whole match when absent.
    /// Invalid tonumber values are nil table.insert values and do not add a slot.
    pub fn line_parts(
        &self,
        line: &[u8],
        marker: &[u8],
        no_number_value: f64,
        budget: &mut RuneBudget,
    ) -> Result<RuneLineParts> {
        let mut values = Vec::new();
        let stripped = self.gsub(line, budget, |found, out, budget| {
            if let Some(value) = number_capture(line, callback_capture(found)) {
                budget.vectors(
                    values
                        .len()
                        .checked_add(1)
                        .ok_or(RuneError::Resource("vector components"))?,
                )?;
                values
                    .try_reserve_exact(1)
                    .map_err(|_| RuneError::Resource("vector allocation"))?;
                values.push(value);
            }
            budget.append(out, marker)
        })?;
        if values.is_empty() {
            budget.vectors(1)?;
            values.push(no_number_value);
        }
        Ok(RuneLineParts { stripped, values })
    }
    /// Returns the completed gsub result only. On error the caller's stored text
    /// is unchanged. On success assign this result BEFORE invoking its parser.
    /// Incoming string.find supplies no synthetic whole-match capture.
    pub fn combine(
        &self,
        stored: &[u8],
        incoming: &[u8],
        budget: &mut RuneBudget,
    ) -> Result<Vec<u8>> {
        let mut start = 1;
        self.gsub(stored, budget, |found, out, budget| {
            let other = self
                .pattern
                .find(incoming, start, false, &mut budget.matching)?
                .ok_or(RuneError::Source(
                    "arithmetic on missing incoming match end",
                ))?;
            start = lua_index(other.range().end)?;
            let a = number_capture(stored, callback_capture(found))
                .ok_or(RuneError::Source("arithmetic on nonnumeric stored capture"))?;
            let b = other
                .captures()
                .first()
                .copied()
                .and_then(|c| number_capture(incoming, c))
                .ok_or(RuneError::Source(
                    "arithmetic on missing or nonnumeric incoming capture",
                ))?;
            budget.append(out, lua_number_text(a + b).as_bytes())
        })
    }
    fn gsub(
        &self,
        line: &[u8],
        budget: &mut RuneBudget,
        mut callback: impl FnMut(&PatternMatch, &mut Vec<u8>, &mut RuneBudget) -> Result<()>,
    ) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        let mut copied = 0;
        // match_captures interprets patterns even when find would use its
        // literal fast path, exactly as gsub does.
        while let Some(found) =
            self.pattern
                .match_captures(line, lua_index(copied)?, &mut budget.matching)?
        {
            let range = found.range();
            budget.append(&mut out, &line[copied..range.start])?;
            budget.callback()?;
            callback(&found, &mut out, budget)?;
            copied = range.end;
            if range.is_empty() {
                if copied == line.len() {
                    break;
                }
                budget.append(&mut out, &line[copied..copied + 1])?;
                copied += 1;
            }
            if self.pattern.source().first() == Some(&b'^') {
                break;
            }
        }
        budget.append(&mut out, &line[copied..])?;
        Ok(out)
    }
}

/// Concatenation uses Lua number text; the caller provides the entire textual
/// prefix (including type/bonded delimiters). Distinct doubles may share a key.
pub fn number_order_key(prefix: &[u8], order: f64, budget: &mut RuneBudget) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    budget.append(&mut out, prefix)?;
    budget.append(&mut out, lua_number_text(order).as_bytes())?;
    Ok(out)
}
#[derive(Clone, Copy, Debug)]
pub struct VectorPolicy {
    pub missing_value: f64,
    pub epsilon: f64,
}
fn component(values: &[f64], i: usize, policy: VectorPolicy) -> f64 {
    values.get(i).copied().unwrap_or(policy.missing_value)
}
/// Original strict descending lexicographic predicate; not a total ordering for NaN.
pub fn compare_vectors(
    a: &[f64],
    b: &[f64],
    policy: VectorPolicy,
    budget: &mut RuneBudget,
) -> Result<bool> {
    let n = a.len().max(b.len());
    budget.vectors(n)?;
    for i in 0..n {
        let (a, b) = (component(a, i, policy), component(b, i, policy));
        if a != b {
            return Ok(a > b);
        }
    }
    Ok(false)
}
pub fn equal_vectors(
    a: &[f64],
    b: &[f64],
    policy: VectorPolicy,
    budget: &mut RuneBudget,
) -> Result<bool> {
    let n = a.len().max(b.len());
    budget.vectors(n)?;
    for i in 0..n {
        if (component(a, i, policy) - component(b, i, policy)).abs() > policy.epsilon {
            return Ok(false);
        }
    }
    Ok(true)
}
pub fn add_vectors(
    a: &[f64],
    b: &[f64],
    policy: VectorPolicy,
    budget: &mut RuneBudget,
) -> Result<Vec<f64>> {
    let n = a.len().max(b.len());
    budget.vectors(n)?;
    let mut out = Vec::new();
    out.try_reserve_exact(n)
        .map_err(|_| RuneError::Resource("vector allocation"))?;
    for i in 0..n {
        out.push(component(a, i, policy) + component(b, i, policy));
    }
    Ok(out)
}
pub fn exceeds_vector(
    a: &[f64],
    target: &[f64],
    policy: VectorPolicy,
    budget: &mut RuneBudget,
) -> Result<bool> {
    let n = a.len().max(target.len());
    budget.vectors(n)?;
    for i in 0..n {
        if component(a, i, policy) > component(target, i, policy) + policy.epsilon {
            return Ok(true);
        }
    }
    Ok(false)
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuneCombination {
    /// One-based candidate indices; preserve visited zero keys copied by source.
    pub counts: BTreeMap<usize, usize>,
    pub count: usize,
    /// Additional proof, not a source field. Distinct positive-count vectors at
    /// the same minimum count exist; retained counts still use source first best.
    pub ambiguous_minimum: bool,
}
struct SearchFrame {
    next: usize,
    sum: Vec<f64>,
    chosen: Option<usize>,
    entered: bool,
}
/// Enumerates source DFS order without sorting or canonicalizing candidates.
/// Missing per-index limits default to zero when a cap table is supplied. The
/// caller resolves name-keyed caps before entry. NaN, fractional and negative
/// bounds retain Lua comparisons; a resource stop is never reported as no match.
pub fn find_combination(
    candidates: &[&[f64]],
    target: &[f64],
    max_runes: f64,
    max_counts: Option<&[f64]>,
    policy: VectorPolicy,
    budget: &mut RuneBudget,
) -> Result<Option<RuneCombination>> {
    if candidates.len() > budget.limits.max_candidates {
        return Err(RuneError::Resource("candidates"));
    }
    budget.vectors(target.len())?;
    for v in candidates {
        budget.vectors(v.len())?;
    }
    let mut counts = vec![0usize; candidates.len()];
    let mut touched = vec![false; candidates.len()];
    let mut stack = vec![SearchFrame {
        next: 0,
        sum: Vec::new(),
        chosen: None,
        entered: false,
    }];
    let mut best: Option<RuneCombination> = None;
    loop {
        budget.search()?;
        let count = stack.len() - 1;
        let frame = stack.last_mut().expect("root search frame");
        let mut finish = false;
        if !frame.entered {
            frame.entered = true;
            if equal_vectors(&frame.sum, target, policy, budget)? {
                // Copying touched keys and comparing alternative count vectors
                // also consume work, independently of their value dimension.
                charge(
                    &mut budget.vector_work,
                    counts.len() as u64,
                    budget.limits.max_vector_work,
                    "vector work",
                )?;
                let previous_count = best.as_ref().map(|b| b.count);
                if previous_count.is_none_or(|n| count < n) {
                    best = Some(RuneCombination {
                        counts: counts
                            .iter()
                            .enumerate()
                            .filter(|(i, _)| touched[*i])
                            .map(|(i, n)| (i + 1, *n))
                            .collect(),
                        count,
                        ambiguous_minimum: false,
                    });
                } else if previous_count == Some(count) {
                    let retained = best.as_mut().expect("existing best");
                    if counts
                        .iter()
                        .enumerate()
                        .any(|(i, n)| *n != retained.counts.get(&(i + 1)).copied().unwrap_or(0))
                    {
                        retained.ambiguous_minimum = true;
                    }
                }
                finish = true;
            } else if count as f64 >= max_runes || best.as_ref().is_some_and(|b| count >= b.count) {
                finish = true;
            }
        }
        if finish || frame.next == candidates.len() {
            if let Some(index) = stack.pop().expect("search frame").chosen {
                counts[index] -= 1;
            }
            if stack.is_empty() {
                break;
            }
            continue;
        }
        let index = frame.next;
        frame.next += 1;
        // Keep the exact '<' predicate: negating '>=' changes NaN behavior.
        let within_cap = max_counts
            .is_none_or(|caps| (counts[index] as f64) < caps.get(index).copied().unwrap_or(0.0));
        if !within_cap {
            continue;
        }
        let next = add_vectors(&frame.sum, candidates[index], policy, budget)?;
        if exceeds_vector(&next, target, policy, budget)? {
            continue;
        }
        if count >= budget.limits.max_search_depth {
            return Err(RuneError::Resource("search depth"));
        }
        counts[index] = counts[index]
            .checked_add(1)
            .ok_or(RuneError::Resource("candidate count"))?;
        touched[index] = true;
        stack
            .try_reserve_exact(1)
            .map_err(|_| RuneError::Resource("search allocation"))?;
        stack.push(SearchFrame {
            next: index,
            sum: next,
            chosen: Some(index),
            entered: false,
        });
    }
    Ok(best)
}
/// Files entering an adapter source fingerprint; normalized by the caller.
pub fn implementation_sources() -> [&'static str; 4] {
    [
        include_str!("item_runes.rs"),
        include_str!("lua_pattern.rs"),
        include_str!("lua_number.rs"),
        include_str!("item_tools/numeric.rs"),
    ]
}
