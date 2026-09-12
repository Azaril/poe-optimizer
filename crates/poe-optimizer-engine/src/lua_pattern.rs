//! Compiled, bounded LuaJIT 5.1 byte patterns, independent of Lua and game data.
//!
//! Source syntax failures are retained as instructions and raised only when
//! reached. `find` preserves the source fixed-string fast path; `match_captures`
//! always interprets the pattern. Results borrow no subject and contain byte
//! spans rather than copies. All indices in spans are zero-based, half-open;
//! position captures use Lua's one-based indices.
use std::ops::Range;

mod substitution;
pub use substitution::{GsubLimits, GsubResult};

pub const LUA_MAX_CAPTURES: usize = 32;
pub const LUA_MAX_DEPTH: usize = 200;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourcePatternError {
    MalformedEscape,
    MissingBracket,
    MissingFrontierClass,
    UnbalancedPattern,
    InvalidCaptureIndex,
    InvalidCaptureClose,
    UnfinishedCapture,
    TooManyCaptures,
    PatternTooComplex,
}
impl SourcePatternError {
    pub fn message(self) -> &'static str {
        match self {
            Self::MalformedEscape => "malformed pattern (ends with '%')",
            Self::MissingBracket => "malformed pattern (missing ']')",
            Self::MissingFrontierClass => "missing '[' after '%f' in pattern",
            Self::UnbalancedPattern => "unbalanced pattern",
            Self::InvalidCaptureIndex => "invalid capture index",
            Self::InvalidCaptureClose => "invalid pattern capture",
            Self::UnfinishedCapture => "unfinished capture",
            Self::TooManyCaptures => "too many captures",
            Self::PatternTooComplex => "pattern too complex",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceKind {
    PatternBytes,
    CompiledBytes,
    SubjectBytes,
    MatchSteps,
    BacktrackFrames,
    ReplacementBytes,
    OutputBytes,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatternError {
    Source(SourcePatternError),
    Resource(ResourceKind),
}
impl std::fmt::Display for PatternError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(error) => f.write_str(error.message()),
            Self::Resource(kind) => write!(f, "Lua pattern resource bound: {kind:?}"),
        }
    }
}
impl std::error::Error for PatternError {}
type Result<T> = std::result::Result<T, PatternError>;
#[derive(Clone, Copy, Debug)]
pub struct CompileLimits {
    /// Input pattern bytes, including bytes after embedded NUL. Default: 64 KiB.
    pub max_pattern_bytes: usize,
    /// Retained dynamic storage, including plain-search prefix data. Default: 8 MiB.
    pub max_compiled_bytes: usize,
}
impl Default for CompileLimits {
    fn default() -> Self {
        Self {
            max_pattern_bytes: 64 * 1024,
            max_compiled_bytes: 8 * 1024 * 1024,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct MatchLimits {
    /// Maximum bytes in one subject. Default: 1 MiB.
    pub max_subject_bytes: usize,
    /// Cumulative execution work across every call sharing a budget. Default: 2,000,000.
    pub max_steps: u64,
    /// Outstanding alternatives; independent of Lua's source depth limit. Default: 256.
    pub max_backtrack_frames: usize,
}
impl Default for MatchLimits {
    fn default() -> Self {
        Self {
            max_subject_bytes: 1024 * 1024,
            max_steps: 2_000_000,
            max_backtrack_frames: 256,
        }
    }
}
/// Per-request scratch budget; reuse across all candidates in a parser scan.
#[derive(Clone, Debug, Default)]
pub struct MatchBudget {
    limits: MatchLimits,
    steps: u64,
}
impl MatchBudget {
    pub fn new(limits: MatchLimits) -> Self {
        Self { limits, steps: 0 }
    }
    pub fn steps_used(&self) -> u64 {
        self.steps
    }
    pub fn charge(&mut self, steps: u64) -> Result<()> {
        self.steps = self
            .steps
            .checked_add(steps)
            .ok_or(PatternError::Resource(ResourceKind::MatchSteps))?;
        if self.steps > self.limits.max_steps {
            Err(PatternError::Resource(ResourceKind::MatchSteps))
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Capture {
    Bytes { start: usize, end: usize },
    Position(usize),
}
const EMPTY_CAPTURE: Capture = Capture::Bytes { start: 0, end: 0 };
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternMatch {
    start: usize,
    end: usize,
    captures: [Capture; LUA_MAX_CAPTURES],
    count: usize,
}
impl PatternMatch {
    pub fn range(&self) -> Range<usize> {
        self.start..self.end
    }
    pub fn captures(&self) -> &[Capture] {
        &self.captures[..self.count]
    }
    /// One-based inclusive Lua endpoints; an empty match has end = start - 1.
    pub fn lua_indices(&self) -> (usize, usize) {
        (self.start + 1, self.end)
    }
}
#[derive(Clone, Copy, Debug)]
struct ByteSet([u64; 4]);
impl ByteSet {
    fn empty() -> Self {
        Self([0; 4])
    }
    fn add(&mut self, byte: u8) {
        self.0[usize::from(byte) / 64] |= 1 << (byte % 64);
    }
    fn has(self, byte: u8) -> bool {
        self.0[usize::from(byte) / 64] & (1 << (byte % 64)) != 0
    }
    fn from_predicate(predicate: impl Fn(u8) -> bool) -> Self {
        let mut result = Self::empty();
        for byte in 0..=255 {
            if predicate(byte) {
                result.add(byte);
            }
        }
        result
    }
}
#[derive(Clone, Copy, Debug)]
enum Repeat {
    One,
    Optional,
    GreedyZero,
    GreedyOne,
    Minimal,
}
#[derive(Clone, Copy, Debug)]
enum Instruction {
    Byte { set: ByteSet, repeat: Repeat },
    Open,
    Position,
    Close,
    Backref(u8),
    Balance(u8, u8),
    Frontier(ByteSet),
    EndAnchor,
    End,
    Trap(SourcePatternError),
}
#[derive(Clone, Debug)]
pub struct LuaPattern {
    source: Vec<u8>,
    instructions: Vec<Instruction>,
    prefix: Vec<usize>,
    anchor: bool,
    has_pattern: bool,
}
impl LuaPattern {
    /// Compile once. Malformed syntax becomes a lazy source-error instruction;
    /// compilation itself rejects only implementation resource limits.
    pub fn compile(pattern: &[u8]) -> Result<Self> {
        Self::compile_with_limits(pattern, CompileLimits::default())
    }
    pub fn compile_with_limits(pattern: &[u8], limits: CompileLimits) -> Result<Self> {
        if pattern.len() > limits.max_pattern_bytes {
            return Err(PatternError::Resource(ResourceKind::PatternBytes));
        }
        let fixed_bytes = pattern
            .len()
            .checked_mul(1 + std::mem::size_of::<usize>())
            .ok_or(PatternError::Resource(ResourceKind::CompiledBytes))?;
        if fixed_bytes > limits.max_compiled_bytes {
            return Err(PatternError::Resource(ResourceKind::CompiledBytes));
        }
        let anchor = pattern.first() == Some(&b'^');
        // Determine the full retained allocation before allocating any buffer.
        // A resource failure must not discard unaccounted temporary storage in
        // a caller that shares one cumulative allocation budget across calls.
        let mut at = usize::from(anchor);
        let mut count = 0usize;
        loop {
            let (instruction, next) = compile_item(pattern, at);
            count += 1;
            let bytes = count
                .checked_mul(std::mem::size_of::<Instruction>())
                .and_then(|n| n.checked_add(fixed_bytes))
                .ok_or(PatternError::Resource(ResourceKind::CompiledBytes))?;
            if bytes > limits.max_compiled_bytes {
                return Err(PatternError::Resource(ResourceKind::CompiledBytes));
            }
            if matches!(instruction, Instruction::End | Instruction::Trap(_)) {
                break;
            }
            at = next;
        }
        let mut instructions = Vec::with_capacity(count);
        at = usize::from(anchor);
        loop {
            let (instruction, next) = compile_item(pattern, at);
            instructions.push(instruction);
            if matches!(instruction, Instruction::End | Instruction::Trap(_)) {
                break;
            }
            at = next;
        }
        let mut prefix = vec![0; pattern.len()];
        for i in 1..pattern.len() {
            let mut matched = prefix[i - 1];
            while matched > 0 && pattern[i] != pattern[matched] {
                matched = prefix[matched - 1];
            }
            if pattern[i] == pattern[matched] {
                matched += 1;
            }
            prefix[i] = matched;
        }
        Ok(Self {
            source: pattern.to_vec(),
            instructions,
            prefix,
            anchor,
            has_pattern: pattern.iter().any(|b| b"^$*+?.([%-".contains(b)),
        })
    }
    pub fn source(&self) -> &[u8] {
        &self.source
    }
    pub fn compiled_bytes(&self) -> usize {
        self.source.capacity()
            + self.prefix.capacity() * std::mem::size_of::<usize>()
            + self.instructions.capacity() * std::mem::size_of::<Instruction>()
    }
    /// The source's complete byte-string magic scan, including bytes after NUL.
    pub(crate) fn has_pattern_bytes(pattern: &[u8], budget: &mut MatchBudget) -> Result<bool> {
        for byte in pattern {
            budget.charge(1)?;
            if b"^$*+?.([%-".contains(byte) {
                return Ok(true);
            }
        }
        Ok(false)
    }
    /// Fixed-string search over borrowed bytes, without compiling a pattern.
    /// Mirrors lj_str_find's first-byte scan and suffix comparison; every
    /// comparison is charged, including failed candidates on repetitive input.
    pub(crate) fn find_literal(
        subject: &[u8],
        needle: &[u8],
        init: i32,
        budget: &mut MatchBudget,
    ) -> Result<Option<PatternMatch>> {
        let start = Self::search_start(subject, init, budget)?;
        if needle.len() > subject.len() - start {
            return Ok(None);
        }
        for candidate in start..=subject.len() - needle.len() {
            let mut matches = true;
            for (offset, byte) in needle.iter().enumerate() {
                budget.charge(1)?;
                if subject[candidate + offset] != *byte {
                    matches = false;
                    break;
                }
            }
            if matches {
                return Ok(Some(PatternMatch {
                    start: candidate,
                    end: candidate + needle.len(),
                    captures: [EMPTY_CAPTURE; LUA_MAX_CAPTURES],
                    count: 0,
                }));
            }
        }
        Ok(None)
    }
    /// LuaJIT 5.1 string.find semantics, including its automatic plain fast path.
    /// `init` is a one-based signed Lua index; negative indices are relative to
    /// the end and out-of-range positive indices clamp to the end. `^` anchors
    /// to that initial search position, not necessarily byte zero.
    pub fn find(
        &self,
        subject: &[u8],
        init: i32,
        plain: bool,
        budget: &mut MatchBudget,
    ) -> Result<Option<PatternMatch>> {
        self.search(subject, init, plain || !self.has_pattern, false, budget)
    }
    /// Lua string.match semantics. The pattern is always interpreted. When it
    /// has no explicit captures, the returned capture is the whole match.
    pub fn match_captures(
        &self,
        subject: &[u8],
        init: i32,
        budget: &mut MatchBudget,
    ) -> Result<Option<PatternMatch>> {
        self.search(subject, init, false, true, budget)
    }
    fn search(
        &self,
        subject: &[u8],
        init: i32,
        plain: bool,
        whole_capture: bool,
        budget: &mut MatchBudget,
    ) -> Result<Option<PatternMatch>> {
        let start = Self::search_start(subject, init, budget)?;
        if plain {
            return self.plain_find(subject, start, budget);
        }
        self.search_raw(subject, start, budget)?
            .map(|raw| raw.into_captures(whole_capture))
            .transpose()
    }
    /// Stateful gmatch commits its position after matching, before capture
    /// export can report an unfinished capture. Keep those operations separate.
    pub(crate) fn match_before_captures(
        &self,
        subject: &[u8],
        init: i32,
        budget: &mut MatchBudget,
    ) -> Result<Option<RawMatch>> {
        let start = Self::search_start(subject, init, budget)?;
        self.search_raw(subject, start, budget)
    }
    fn search_start(subject: &[u8], init: i32, budget: &mut MatchBudget) -> Result<usize> {
        if subject.len() > budget.limits.max_subject_bytes {
            return Err(PatternError::Resource(ResourceKind::SubjectBytes));
        }
        budget.charge(1)?;
        let raw_start = if init < 0 {
            subject.len() as i128 + i128::from(init)
        } else {
            i128::from(init) - 1
        };
        // This project uses LuaJIT 5.1, whose out-of-range positive init clamps.
        Ok(raw_start.clamp(0, subject.len() as i128) as usize)
    }
    fn search_raw(
        &self,
        subject: &[u8],
        start: usize,
        budget: &mut MatchBudget,
    ) -> Result<Option<RawMatch>> {
        let mut scratch = Vec::new();
        for candidate in start..=subject.len() {
            budget.charge(1)?;
            if let Some(raw) = self.run(subject, candidate, &mut scratch, budget)? {
                return Ok(Some(raw));
            }
            if self.anchor {
                break;
            }
        }
        Ok(None)
    }
    fn plain_find(
        &self,
        subject: &[u8],
        start: usize,
        budget: &mut MatchBudget,
    ) -> Result<Option<PatternMatch>> {
        let end = if self.source.is_empty() {
            Some(start)
        } else {
            let mut matched = 0;
            let mut found = None;
            for (index, byte) in subject.iter().copied().enumerate().skip(start) {
                loop {
                    budget.charge(1)?;
                    if byte == self.source[matched] {
                        matched += 1;
                        break;
                    }
                    if matched == 0 {
                        break;
                    }
                    matched = self.prefix[matched - 1];
                }
                if matched == self.source.len() {
                    found = Some(index + 1);
                    break;
                }
            }
            found
        };
        Ok(end.map(|end| PatternMatch {
            start: end - self.source.len(),
            end,
            captures: [EMPTY_CAPTURE; LUA_MAX_CAPTURES],
            count: 0,
        }))
    }
}
fn byte(pattern: &[u8], at: usize) -> u8 {
    pattern.get(at).copied().unwrap_or(0)
}
fn class_matches(value: u8, class: u8) -> bool {
    let result = match class.to_ascii_lowercase() {
        b'a' => value.is_ascii_alphabetic(),
        b'c' => value.is_ascii_control(),
        b'd' => value.is_ascii_digit(),
        b'g' => value.is_ascii_graphic(),
        b'l' => value.is_ascii_lowercase(),
        b'p' => value.is_ascii_punctuation(),
        b's' => matches!(value, b' ' | b'\t' | b'\n' | b'\r' | 11 | 12),
        b'u' => value.is_ascii_uppercase(),
        b'w' => value.is_ascii_alphanumeric(),
        b'x' => value.is_ascii_hexdigit(),
        b'z' => value == 0,
        _ => return value == class,
    };
    if class.is_ascii_uppercase() {
        !result
    } else {
        result
    }
}
fn bracket(pattern: &[u8], at: usize) -> std::result::Result<(ByteSet, usize), SourcePatternError> {
    let inverted = byte(pattern, at + 1) == b'^';
    let first = at + 1 + usize::from(inverted);
    let mut end = first;
    loop {
        if byte(pattern, end) == 0 {
            return Err(SourcePatternError::MissingBracket);
        }
        if byte(pattern, end) == b'%' && byte(pattern, end + 1) != 0 {
            end += 1;
        }
        end += 1;
        if byte(pattern, end) == b']' {
            break;
        }
    }
    let set = ByteSet::from_predicate(|value| {
        let mut cursor = first;
        let mut matched = false;
        while cursor < end {
            if pattern[cursor] == b'%' {
                cursor += 1;
                matched |= class_matches(value, pattern[cursor]);
            } else if byte(pattern, cursor + 1) == b'-' && cursor + 2 < end {
                matched |= pattern[cursor] <= value && value <= pattern[cursor + 2];
                cursor += 2;
            } else {
                matched |= pattern[cursor] == value;
            }
            cursor += 1;
        }
        matched != inverted
    });
    Ok((set, end + 1))
}
fn compile_item(pattern: &[u8], at: usize) -> (Instruction, usize) {
    let (set, end) = match byte(pattern, at) {
        0 => return (Instruction::End, at),
        b'(' if byte(pattern, at + 1) == b')' => return (Instruction::Position, at + 2),
        b'(' => return (Instruction::Open, at + 1),
        b')' => return (Instruction::Close, at + 1),
        b'$' if byte(pattern, at + 1) == 0 => return (Instruction::EndAnchor, at + 1),
        b'%' => match byte(pattern, at + 1) {
            0 => return (Instruction::Trap(SourcePatternError::MalformedEscape), at),
            b'b' => {
                let open = byte(pattern, at + 2);
                let close = byte(pattern, at + 3);
                return if open == 0 || close == 0 {
                    (Instruction::Trap(SourcePatternError::UnbalancedPattern), at)
                } else {
                    (Instruction::Balance(open, close), at + 4)
                };
            }
            b'f' => {
                if byte(pattern, at + 2) != b'[' {
                    return (
                        Instruction::Trap(SourcePatternError::MissingFrontierClass),
                        at,
                    );
                }
                return match bracket(pattern, at + 2) {
                    Ok((set, end)) => (Instruction::Frontier(set), end),
                    Err(error) => (Instruction::Trap(error), at),
                };
            }
            digit @ b'0'..=b'9' => return (Instruction::Backref(digit), at + 2),
            class => (ByteSet::from_predicate(|v| class_matches(v, class)), at + 2),
        },
        b'[' => match bracket(pattern, at) {
            Ok(result) => result,
            Err(error) => return (Instruction::Trap(error), at),
        },
        b'.' => (ByteSet([u64::MAX; 4]), at + 1),
        literal => {
            let mut set = ByteSet::empty();
            set.add(literal);
            (set, at + 1)
        }
    };
    let repeat = match byte(pattern, end) {
        b'?' => Repeat::Optional,
        b'*' => Repeat::GreedyZero,
        b'+' => Repeat::GreedyOne,
        b'-' => Repeat::Minimal,
        _ => {
            return (
                Instruction::Byte {
                    set,
                    repeat: Repeat::One,
                },
                end,
            );
        }
    };
    (Instruction::Byte { set, repeat }, end + 1)
}

#[derive(Clone, Copy, Debug)]
enum CaptureState {
    Open(usize),
    Bytes(usize, usize),
    Position(usize),
}
#[derive(Clone, Copy, Debug)]
struct Captures {
    items: [CaptureState; LUA_MAX_CAPTURES],
    count: usize,
}
/// Matching itself does not consume captures. In particular, gsub only rejects
/// an unfinished capture if its replacement actually asks for that capture.
pub(crate) struct RawMatch {
    start: usize,
    end: usize,
    captures: Captures,
}
impl RawMatch {
    pub(crate) fn range(&self) -> Range<usize> {
        self.start..self.end
    }
    pub(crate) fn into_captures(self, whole_capture: bool) -> Result<PatternMatch> {
        let mut result = self.captures.result(self.start, self.end)?;
        if whole_capture && result.count == 0 {
            result.captures[0] = Capture::Bytes {
                start: result.start,
                end: result.end,
            };
            result.count = 1;
        }
        Ok(result)
    }
}
impl Default for Captures {
    fn default() -> Self {
        Self {
            items: [CaptureState::Open(0); LUA_MAX_CAPTURES],
            count: 0,
        }
    }
}
impl Captures {
    fn push(&mut self, value: CaptureState) -> Result<()> {
        if self.count == LUA_MAX_CAPTURES {
            return Err(PatternError::Source(SourcePatternError::TooManyCaptures));
        }
        self.items[self.count] = value;
        self.count += 1;
        Ok(())
    }
    fn result(self, start: usize, end: usize) -> Result<PatternMatch> {
        let mut result = PatternMatch {
            start,
            end,
            captures: [EMPTY_CAPTURE; LUA_MAX_CAPTURES],
            count: self.count,
        };
        for (index, value) in self.items[..self.count].iter().enumerate() {
            result.captures[index] = match *value {
                CaptureState::Open(_) => {
                    return Err(PatternError::Source(SourcePatternError::UnfinishedCapture));
                }
                CaptureState::Bytes(start, end) => Capture::Bytes { start, end },
                CaptureState::Position(index) => Capture::Position(index + 1),
            };
        }
        Ok(result)
    }
}
#[derive(Clone, Copy, Debug)]
enum Alternative {
    Once,
    Greedy { minimum: usize },
    Minimal { set: ByteSet },
}
#[derive(Clone, Copy, Debug)]
struct Frame {
    instruction: usize,
    subject: usize,
    captures: Captures,
    depth: usize,
    alternative: Alternative,
}
fn push_frame(frames: &mut Vec<Frame>, frame: Frame, budget: &MatchBudget) -> Result<()> {
    if frames.len() >= budget.limits.max_backtrack_frames {
        return Err(PatternError::Resource(ResourceKind::BacktrackFrames));
    }
    frames.push(frame);
    Ok(())
}
impl LuaPattern {
    fn run(
        &self,
        subject: &[u8],
        start: usize,
        frames: &mut Vec<Frame>,
        budget: &mut MatchBudget,
    ) -> Result<Option<RawMatch>> {
        frames.clear();
        let mut pc = 0;
        let mut at = start;
        let mut captures = Captures::default();
        let mut depth = 1;
        'execute: loop {
            if depth > LUA_MAX_DEPTH {
                return Err(PatternError::Source(SourcePatternError::PatternTooComplex));
            }
            budget.charge(1)?;
            match self.instructions[pc] {
                Instruction::End => {
                    return Ok(Some(RawMatch {
                        start,
                        end: at,
                        captures,
                    }));
                }
                Instruction::EndAnchor => {
                    if at == subject.len() {
                        return Ok(Some(RawMatch {
                            start,
                            end: at,
                            captures,
                        }));
                    }
                }
                Instruction::Trap(error) => return Err(PatternError::Source(error)),
                Instruction::Open | Instruction::Position => {
                    captures.push(if matches!(self.instructions[pc], Instruction::Open) {
                        CaptureState::Open(at)
                    } else {
                        CaptureState::Position(at)
                    })?;
                    pc += 1;
                    depth += 1;
                    continue;
                }
                Instruction::Close => {
                    let Some(index) = captures.items[..captures.count]
                        .iter()
                        .rposition(|v| matches!(v, CaptureState::Open(_)))
                    else {
                        return Err(PatternError::Source(
                            SourcePatternError::InvalidCaptureClose,
                        ));
                    };
                    let CaptureState::Open(begin) = captures.items[index] else {
                        unreachable!()
                    };
                    captures.items[index] = CaptureState::Bytes(begin, at);
                    pc += 1;
                    depth += 1;
                    continue;
                }
                Instruction::Backref(reference) => {
                    let index = usize::from(reference - b'0');
                    if index == 0 || index > captures.count {
                        return Err(PatternError::Source(
                            SourcePatternError::InvalidCaptureIndex,
                        ));
                    }
                    match captures.items[index - 1] {
                        CaptureState::Open(_) => {
                            return Err(PatternError::Source(
                                SourcePatternError::InvalidCaptureIndex,
                            ));
                        }
                        CaptureState::Position(_) => {} // source size_t(-2) cannot fit
                        CaptureState::Bytes(begin, end) => {
                            let length = end - begin;
                            if length <= subject.len() - at {
                                budget.charge(length as u64)?;
                                if subject[begin..end] == subject[at..at + length] {
                                    at += length;
                                    pc += 1;
                                    continue;
                                }
                            }
                        }
                    }
                }
                Instruction::Balance(open, close) => {
                    if subject.get(at) == Some(&open) {
                        let mut nested = 1usize;
                        let mut cursor = at + 1;
                        while cursor < subject.len() {
                            budget.charge(1)?;
                            if subject[cursor] == close {
                                nested -= 1;
                            } else if subject[cursor] == open {
                                nested += 1;
                            }
                            cursor += 1;
                            if nested == 0 {
                                at = cursor;
                                pc += 1;
                                continue 'execute;
                            }
                        }
                    }
                }
                Instruction::Frontier(set) => {
                    let previous = if at == 0 { 0 } else { subject[at - 1] };
                    let current = subject.get(at).copied().unwrap_or(0);
                    if !set.has(previous) && set.has(current) {
                        pc += 1;
                        continue;
                    }
                }
                Instruction::Byte { set, repeat } => {
                    let matched = subject.get(at).is_some_and(|v| set.has(*v));
                    match repeat {
                        Repeat::One => {
                            if matched {
                                at += 1;
                                pc += 1;
                                continue;
                            }
                        }
                        Repeat::Optional => {
                            if matched {
                                push_frame(
                                    frames,
                                    Frame {
                                        instruction: pc + 1,
                                        subject: at,
                                        captures,
                                        depth,
                                        alternative: Alternative::Once,
                                    },
                                    budget,
                                )?;
                                at += 1;
                                depth += 1;
                            }
                            pc += 1;
                            continue;
                        }
                        Repeat::GreedyZero | Repeat::GreedyOne => {
                            if matches!(repeat, Repeat::GreedyOne) && !matched { /* fail */
                            } else {
                                let minimum = at + usize::from(matches!(repeat, Repeat::GreedyOne));
                                let mut maximum = minimum;
                                while subject.get(maximum).is_some_and(|v| set.has(*v)) {
                                    budget.charge(1)?;
                                    maximum += 1;
                                }
                                if maximum > minimum {
                                    push_frame(
                                        frames,
                                        Frame {
                                            instruction: pc + 1,
                                            subject: maximum - 1,
                                            captures,
                                            depth: depth + 1,
                                            alternative: Alternative::Greedy { minimum },
                                        },
                                        budget,
                                    )?;
                                }
                                at = maximum;
                                pc += 1;
                                depth += 1;
                                continue;
                            }
                        }
                        Repeat::Minimal => {
                            push_frame(
                                frames,
                                Frame {
                                    instruction: pc + 1,
                                    subject: at,
                                    captures,
                                    depth: depth + 1,
                                    alternative: Alternative::Minimal { set },
                                },
                                budget,
                            )?;
                            pc += 1;
                            depth += 1;
                            continue;
                        }
                    }
                }
            }
            // A failed path restores the complete source capture state and the
            // virtual Lua call depth, without using the Rust call stack.
            while let Some(mut frame) = frames.pop() {
                budget.charge(1)?;
                let resume = match frame.alternative {
                    Alternative::Once => frame.subject,
                    Alternative::Greedy { minimum } => {
                        let next = frame.subject;
                        if next > minimum {
                            frame.subject -= 1;
                            push_frame(frames, frame, budget)?;
                        }
                        next
                    }
                    Alternative::Minimal { set } => {
                        if !subject.get(frame.subject).is_some_and(|v| set.has(*v)) {
                            continue;
                        }
                        frame.subject += 1;
                        push_frame(frames, frame, budget)?;
                        frame.subject
                    }
                };
                at = resume;
                pc = frame.instruction;
                captures = frame.captures;
                depth = frame.depth;
                continue 'execute;
            }
            return Ok(None);
        }
    }
}
