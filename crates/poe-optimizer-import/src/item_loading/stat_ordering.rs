//! Finite advanced-copy ordering over the selected immutable raw definitions.
//! Successful lookup preparation is cached per item machine, not process-wide.
use super::*;
use poe_optimizer_data::item_loading::{ItemStatOrderingPolicy, ItemStatOrderingSubstitution};
use poe_optimizer_engine::lua_pattern::{CompileLimits, MatchLimits};
use std::cmp::Ordering;

type Result<T> = std::result::Result<T, Failure>;
#[derive(Debug)]
enum Failure {
    Pattern(PatternError),
    Unsupported(&'static str),
    Resource(&'static str),
}
impl From<PatternError> for Failure {
    fn from(error: PatternError) -> Self {
        Self::Pattern(error)
    }
}
const MAX_PREPARATION_STEPS: u64 = 64_000_000;
const MAX_LOOKUP_ROWS: usize = 65_536;
const MAX_COMPILED_BYTES: usize = 4 * 1024 * 1024;

/// Includes temporary key traffic and retained lookup storage. This is a bounded
/// logical allocation charge, not allocator instrumentation or source VM cost.
struct Work<'a> {
    matching: &'a mut MatchBudget,
    bytes: usize,
    maximum: usize,
}
impl Work<'_> {
    fn charge(&mut self, bytes: usize) -> Result<()> {
        self.bytes = self
            .bytes
            .checked_add(bytes)
            .filter(|n| *n <= self.maximum)
            .ok_or(Failure::Resource("unique order preparation byte bound"))?;
        Ok(())
    }
    fn remaining(&self) -> usize {
        self.maximum.saturating_sub(self.bytes)
    }
    fn lower(&mut self, text: &[u8]) -> Result<Vec<u8>> {
        self.charge(text.len())?;
        self.matching.charge(text.len().max(1) as u64)?;
        Ok(text.iter().map(u8::to_ascii_lowercase).collect())
    }
    fn substitute(
        &mut self,
        pattern: &LuaPattern,
        text: &[u8],
        rule: &ItemStatOrderingSubstitution,
    ) -> Result<Vec<u8>> {
        let maximum = self.remaining().min(MAX_ITEM_LOADING_TEXT);
        let output = pattern
            .gsub(
                text,
                rule.replacement.as_bytes(),
                None,
                self.matching,
                GsubLimits {
                    max_replacement_bytes: MAX_ITEM_LOADING_TEXT,
                    max_output_bytes: maximum,
                },
            )?
            .bytes;
        self.charge(output.len())?;
        Ok(output)
    }
}
#[derive(Clone, Copy)]
struct Minimum {
    value: f64,
    zero_signs: u8,
}
impl Minimum {
    fn new(value: f64) -> Self {
        Self {
            value,
            zero_signs: if value == 0.0 {
                if value.is_sign_negative() { 2 } else { 1 }
            } else {
                0
            },
        }
    }
    fn add(&mut self, value: f64) {
        if value < self.value {
            *self = Self::new(value);
        } else if value == 0.0 && self.value == 0.0 {
            self.zero_signs |= Self::new(value).zero_signs;
        }
    }
}
pub(super) struct StatOrderingPrograms {
    numbers: LuaPattern,
    ranges: LuaPattern,
    flatten: LuaPattern,
    exact: BTreeMap<Vec<u8>, Minimum>,
    normalised: BTreeMap<Vec<u8>, Minimum>,
}
impl StatOrderingPrograms {
    fn exact_key(
        &self,
        text: &[u8],
        policy: &ItemStatOrderingPolicy,
        work: &mut Work<'_>,
    ) -> Result<Vec<u8>> {
        let lower = work.lower(text)?;
        work.substitute(&self.flatten, &lower, &policy.flatten_newlines)
    }
    fn normalised_key(
        &self,
        text: &[u8],
        policy: &ItemStatOrderingPolicy,
        work: &mut Work<'_>,
    ) -> Result<Vec<u8>> {
        let numbers = work.substitute(&self.numbers, text, &policy.normalize_numbers)?;
        let ranges = work.substitute(&self.ranges, &numbers, &policy.normalize_ranges)?;
        let lower = work.lower(&ranges)?;
        work.substitute(&self.flatten, &lower, &policy.flatten_newlines)
    }
    fn prepare(
        policy: &ItemStatOrderingPolicy,
        definitions: Option<&ItemMetadataTable>,
        work: &mut Work<'_>,
    ) -> Result<Self> {
        policy
            .validate()
            .map_err(|_| Failure::Unsupported("invalid unique order policy"))?;
        let mut compiled = 0usize;
        let mut compile = |rule: &ItemStatOrderingSubstitution| -> Result<LuaPattern> {
            let pattern = LuaPattern::compile_with_limits(
                rule.pattern.as_bytes(),
                CompileLimits {
                    max_compiled_bytes: work.remaining().min(MAX_COMPILED_BYTES - compiled),
                    ..CompileLimits::default()
                },
            )?;
            compiled += pattern.compiled_bytes();
            work.charge(pattern.compiled_bytes())?;
            Ok(pattern)
        };
        let mut out = Self {
            numbers: compile(&policy.normalize_numbers)?,
            ranges: compile(&policy.normalize_ranges)?,
            flatten: compile(&policy.flatten_newlines)?,
            exact: BTreeMap::new(),
            normalised: BTreeMap::new(),
        };
        let definitions = definitions.ok_or(Failure::Unsupported(
            "unique order raw definition family is unavailable",
        ))?;
        let mut rows = 0;
        // This is not a source pairs order. Admission requires every consumed row
        // to succeed and the complete minimum reductions to be order-independent.
        for definition in definitions
            .fields
            .values()
            .chain(definitions.indexed.values())
        {
            work.matching.charge(1)?;
            if !matches!(
                definition,
                ItemMetadataValue::Table(_) | ItemMetadataValue::Array(_)
            ) {
                return Err(Failure::Unsupported(
                    "unique order raw record has unrepresented pairs/error-order effects",
                ));
            }
            for index in 1.. {
                let Some(line) = indexed(definition, index) else {
                    break;
                };
                rows += 1;
                if rows > MAX_LOOKUP_ROWS {
                    return Err(Failure::Resource("unique order row bound"));
                }
                let line = line.as_str().ok_or(Failure::Unsupported(
                    "unique order raw line has unrepresented pairs/error-order effects",
                ))?;
                let exact = out.exact_key(line.as_bytes(), policy, work)?;
                let normalised = out.normalised_key(line.as_bytes(), policy, work)?;
                let order = definition
                    .as_table()
                    .and_then(|t| t.fields.get(&policy.stat_order_field))
                    .and_then(|orders| indexed(orders, index))
                    .and_then(|value| match value {
                        ItemMetadataValue::Number(n) => Some(*n),
                        ItemMetadataValue::Text(s) => syntax::lua_number(s).value(),
                        _ => None,
                    })
                    .filter(|n| n.is_finite())
                    .ok_or(Failure::Unsupported(
                        "unique order raw order has unrepresented pairs/error-order effects",
                    ))?;
                insert(&mut out.exact, exact, order, work)?;
                insert(&mut out.normalised, normalised, order, work)?;
            }
        }
        if out
            .exact
            .values()
            .chain(out.normalised.values())
            .any(|v| v.zero_signs == 3)
        {
            return Err(Failure::Unsupported(
                "unique order minimum has source pairs-dependent signed zero",
            ));
        }
        Ok(out)
    }
    fn lookup(
        &self,
        text: &str,
        policy: &ItemStatOrderingPolicy,
        work: &mut Work<'_>,
    ) -> Result<Option<ItemNumber>> {
        let exact = self.exact_key(text.as_bytes(), policy, work)?;
        if let Some(order) = self.exact.get(&exact) {
            return Ok(Some(ItemNumber::new(order.value)));
        }
        let normalised = self.normalised_key(text.as_bytes(), policy, work)?;
        Ok(self
            .normalised
            .get(&normalised)
            .map(|order| ItemNumber::new(order.value)))
    }
}
fn indexed(value: &ItemMetadataValue, index: usize) -> Option<&ItemMetadataValue> {
    match value {
        ItemMetadataValue::Array(values) => values.get(index - 1),
        ItemMetadataValue::Table(table) => table.indexed.get(&(index as i64)),
        _ => None,
    }
}
fn insert(
    map: &mut BTreeMap<Vec<u8>, Minimum>,
    key: Vec<u8>,
    value: f64,
    work: &mut Work<'_>,
) -> Result<()> {
    work.matching.charge(key.len().max(1) as u64)?;
    if let Some(entry) = map.get_mut(&key) {
        entry.add(value);
    } else {
        work.charge(128)?;
        map.insert(key, Minimum::new(value));
    }
    Ok(())
}
fn sort_lines(
    lines: &mut [LoadedModLine],
    policy: &ItemStatOrderingPolicy,
    work: &mut Work<'_>,
) -> Result<()> {
    let n = lines.len();
    if n <= 1 {
        return Ok(());
    }
    if n > MAX_ITEM_LOADING_LINES {
        return Err(Failure::Resource("unique order line bound"));
    }
    work.charge(n * std::mem::size_of::<usize>() * 2)?;
    let mut indices: Vec<_> = (0..n).collect();
    for line in lines.iter() {
        work.matching.charge(1)?;
        if line
            .order
            .is_some_and(|n| !matches!(n, ItemNumber::Finite(v) if v.is_finite()))
        {
            return Err(Failure::Unsupported(
                "advanced-copy sort order is outside finite numeric domain",
            ));
        }
    }
    let group = |line: &LoadedModLine| {
        if line.flags.contains("crafted") || line.flags.contains("custom") {
            policy.groups.crafted_custom
        } else if line.flags.contains("fractured") {
            policy.groups.fractured
        } else {
            policy.groups.ordinary
        }
    };
    let compare = |a: usize, b: usize| -> Ordering {
        let (left, right) = (&lines[a], &lines[b]);
        let (ag, bg) = (group(left), group(right));
        if ag != bg {
            return ag.partial_cmp(&bg).expect("validated finite groups");
        }
        if ag < policy.groups.compare_order_below && left.order != right.order {
            return left
                .order
                .and_then(ItemNumber::value)
                .unwrap_or(f64::INFINITY)
                .partial_cmp(
                    &right
                        .order
                        .and_then(ItemNumber::value)
                        .unwrap_or(f64::INFINITY),
                )
                .expect("finite orders or missing infinity");
        }
        a.cmp(&b)
    };
    // Fallible merge only touches index scratch. Budget failure leaves all source
    // row identities/order intact; no inconsistent comparator enters Rust sort.
    let mut destinations = vec![0; n];
    let mut width = 1;
    while width < n {
        for start in (0..n).step_by(width * 2) {
            let middle = (start + width).min(n);
            let end = (start + width * 2).min(n);
            let (mut left, mut right) = (start, middle);
            for slot in &mut destinations[start..end] {
                work.matching.charge(1)?;
                if left < middle
                    && (right == end || compare(indices[left], indices[right]) != Ordering::Greater)
                {
                    *slot = indices[left];
                    left += 1;
                } else {
                    *slot = indices[right];
                    right += 1;
                }
            }
        }
        std::mem::swap(&mut indices, &mut destinations);
        width *= 2;
    }
    work.matching.charge((n * 2) as u64)?;
    for (destination, &origin) in indices.iter().enumerate() {
        destinations[origin] = destination;
    }
    for index in 0..n {
        while destinations[index] != index {
            let next = destinations[index];
            lines.swap(index, next);
            destinations.swap(index, next);
        }
    }
    Ok(())
}
impl ItemLoadMachine<'_> {
    pub(super) fn finish_stat_ordering(&mut self) -> std::result::Result<bool, ItemLoadError> {
        if !self.flag("advancedCopy") {
            return Ok(true);
        }
        let policy = &self.catalog.policy().stat_ordering;
        let assign = (self.state.rarity == policy.unique_rarity
            || self.state.rarity == policy.relic_rarity)
            && !self.state.variants.uses_versioned_or_grouped();
        let mut work = Work {
            matching: &mut self.stat_order_budget,
            bytes: 0,
            maximum: MAX_ITEM_LOADING_EVIDENCE_BYTES.saturating_sub(self.work_bytes),
        };
        let result = (|| {
            if assign {
                if self.stat_order_programs.is_none() {
                    // Source publishes a partial global cache before a failure.
                    // No successful item order writes occur until this finishes.
                    // Do not forge that unrepresented failed-cache history.
                    self.stat_order_programs = Some(StatOrderingPrograms::prepare(policy, self.catalog.modifier_table(&policy.modifier_table), &mut work)
                        .map_err(|failure| match failure {
                            Failure::Pattern(PatternError::Source(_)) => Failure::Unsupported("unique order cache pattern failed with unrepresented pairs/error-order effects"),
                            other => other,
                        })?);
                }
                let program = self
                    .stat_order_programs
                    .as_ref()
                    .expect("prepared unique order");
                for line in &mut self.state.explicit_mod_lines {
                    line.order = program.lookup(&line.line, policy, &mut work)?;
                }
            }
            sort_lines(&mut self.state.explicit_mod_lines, policy, &mut work)
        })();
        let spent = work.bytes;
        self.charge(spent)?;
        match result {
            Ok(()) => Ok(true),
            Err(Failure::Unsupported(message)) => {
                self.stop(DependencyKind::AdvancedCopyAffixes, None, message)?;
                Ok(false)
            }
            Err(Failure::Pattern(PatternError::Source(error))) => {
                self.reject_dependency(format!("unique order source pattern: {error:?}"), true)
            }
            Err(Failure::Pattern(error)) => Err(ItemLoadError(error.to_string())),
            Err(Failure::Resource(message)) => Err(ItemLoadError(message.into())),
        }
    }
}
pub(super) fn budget() -> MatchBudget {
    MatchBudget::new(MatchLimits {
        max_steps: MAX_PREPARATION_STEPS,
        ..MatchLimits::default()
    })
}

#[cfg(test)]
#[path = "stat_ordering_tests.rs"]
mod tests;
