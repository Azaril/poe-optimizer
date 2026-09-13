use super::{Arena, AssemblyError, AssemblyLimits, Result};
use poe_optimizer_engine::lua_pattern::{
    Capture, CompileLimits, GsubLimits, LuaPattern, MatchBudget, MatchLimits, PatternError,
};
use std::collections::BTreeMap;

pub(super) struct Patterns {
    compiled: BTreeMap<String, LuaPattern>,
    budget: MatchBudget,
    accounted_work: u64,
}
impl Patterns {
    pub(super) fn new(limits: AssemblyLimits) -> Self {
        Self {
            compiled: BTreeMap::new(),
            budget: MatchBudget::new(MatchLimits {
                max_subject_bytes: limits.max_bytes.min(1024 * 1024),
                max_steps: limits.max_steps,
                max_backtrack_frames: 256,
            }),
            accounted_work: 0,
        }
    }
    fn begin(&mut self, arena: &Arena) -> Result<u64> {
        self.budget
            .charge(arena.usage().steps.saturating_sub(self.accounted_work))
            .map_err(error)?;
        Ok(self.budget.steps_used())
    }
    fn end(&mut self, arena: &mut Arena, before: u64) -> Result<()> {
        arena.work(self.budget.steps_used().saturating_sub(before))?;
        self.accounted_work = arena.usage().steps;
        Ok(())
    }
    fn prepare(&mut self, arena: &mut Arena, pattern: &str) -> Result<()> {
        if self.compiled.contains_key(pattern) {
            return Ok(());
        }
        // Explicit conservative storage allowance, passed as the compiler's hard
        // bound; the shared compiler performs its own allocation-free preflight.
        let bytes = pattern
            .len()
            .checked_add(1)
            .and_then(|n| n.checked_mul(128))
            .ok_or_else(|| AssemblyError::resource("item pattern storage"))?;
        arena.reserve_bytes(
            bytes
                .checked_add(pattern.len() + 128)
                .ok_or_else(|| AssemblyError::resource("item pattern storage"))?,
        )?;
        arena.work(pattern.len() as u64 + 1)?;
        let compiled = LuaPattern::compile_with_limits(
            pattern.as_bytes(),
            CompileLimits {
                max_pattern_bytes: 4096,
                max_compiled_bytes: bytes,
            },
        )
        .map_err(error)?;
        self.compiled.insert(pattern.to_owned(), compiled);
        Ok(())
    }
    pub(super) fn find(&mut self, arena: &mut Arena, subject: &str, pattern: &str) -> Result<bool> {
        self.prepare(arena, pattern)?;
        let before = self.begin(arena)?;
        let result = self.compiled[pattern].find(subject.as_bytes(), 1, false, &mut self.budget);
        self.end(arena, before)?;
        result.map(|v| v.is_some()).map_err(error)
    }
    pub(super) fn capture(
        &mut self,
        arena: &mut Arena,
        subject: &str,
        pattern: &str,
    ) -> Result<Option<String>> {
        self.prepare(arena, pattern)?;
        let before = self.begin(arena)?;
        let result = self.compiled[pattern].match_captures(subject.as_bytes(), 1, &mut self.budget);
        self.end(arena, before)?;
        let Some(found) = result.map_err(error)? else {
            return Ok(None);
        };
        let range = match found.captures().first() {
            Some(Capture::Bytes { start, end }) => *start..*end,
            None => found.range(),
            Some(Capture::Position(_)) => {
                return Err(AssemblyError::unsupported(
                    "item class pattern returned a position",
                ));
            }
        };
        let value = subject
            .get(range)
            .ok_or_else(|| AssemblyError::unsupported("item pattern split UTF-8 text"))?;
        arena.reserve_bytes(value.len())?;
        Ok(Some(value.to_owned()))
    }
    pub(super) fn replace(
        &mut self,
        arena: &mut Arena,
        subject: &str,
        pattern: &str,
        replacement: &str,
    ) -> Result<String> {
        self.prepare(arena, pattern)?;
        let escapes = replacement.bytes().filter(|b| *b == b'%').count();
        let bound = subject
            .len()
            .checked_add(1)
            .and_then(|n| n.checked_mul(replacement.len()))
            .and_then(|n| {
                subject
                    .len()
                    .checked_mul(escapes + 1)
                    .and_then(|m| n.checked_add(m))
            })
            .ok_or_else(|| AssemblyError::resource("item replacement storage"))?;
        arena.reserve_bytes(bound)?;
        let before = self.begin(arena)?;
        let result = self.compiled[pattern].gsub(
            subject.as_bytes(),
            replacement.as_bytes(),
            None,
            &mut self.budget,
            GsubLimits {
                max_replacement_bytes: 4096,
                max_output_bytes: bound,
            },
        );
        self.end(arena, before)?;
        String::from_utf8(result.map_err(error)?.bytes)
            .map_err(|_| AssemblyError::unsupported("item replacement produced non-UTF8 bytes"))
    }
    pub(super) fn zero_line(&mut self, arena: &mut Arena, line: &str) -> Result<bool> {
        // The authenticated isZeroValueLine lexical grammar; these are numeric
        // text rules, not stat identities or item-family rules.
        Ok(self.find(arena, line, "^%+?0%%? ")?
            || (self.find(arena, line, " %+?0%%? ")?
                && !self.find(arena, line, "0 to [1-9]")?
                && !self.find(arena, line, "0%% to %d+%%")?)
            || self.find(arena, line, " 0%-0 ")?
            || self.find(arena, line, " 0 to 0 ")?)
    }
}
fn error(error: PatternError) -> AssemblyError {
    match error {
        PatternError::Source(e) => AssemblyError::source(e.message()),
        PatternError::Resource(_) => AssemblyError::resource(error.to_string()),
    }
}
