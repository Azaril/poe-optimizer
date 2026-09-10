//! LuaJIT 5.1 string-replacement gsub, over the shared byte matcher.
use super::{
    CaptureState, LuaPattern, MatchBudget, PatternError, RawMatch, ResourceKind, Result,
    SourcePatternError,
};

#[derive(Clone, Copy, Debug)]
pub struct GsubLimits {
    /// Maximum replacement input size, including embedded NUL. Default: 1 MiB.
    pub max_replacement_bytes: usize,
    /// Maximum retained output bytes. Default: 16 MiB.
    pub max_output_bytes: usize,
}
impl Default for GsubLimits {
    fn default() -> Self {
        Self {
            max_replacement_bytes: 1024 * 1024,
            max_output_bytes: 16 * 1024 * 1024,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GsubResult {
    pub bytes: Vec<u8>,
    pub substitutions: usize,
}
impl LuaPattern {
    /// LuaJIT 5.1 `string.gsub` with a byte-string replacement and an already
    /// converted optional integer limit. This does not implement Lua argument
    /// coercion or callable/table replacements. Negative or zero limits make no
    /// substitutions, without evaluating pattern or replacement captures.
    ///
    /// Unlike newer Lua versions, a percent before a non-digit copies that byte;
    /// a trailing percent copies the replacement's terminating NUL. Capture one
    /// denotes the whole match when there are no captures. Capture validity is
    /// checked only when that replacement reference is reached.
    pub fn gsub(
        &self,
        subject: &[u8],
        replacement: &[u8],
        max_replacements: Option<i32>,
        budget: &mut MatchBudget,
        limits: GsubLimits,
    ) -> Result<GsubResult> {
        if subject.len() > budget.limits.max_subject_bytes {
            return Err(PatternError::Resource(ResourceKind::SubjectBytes));
        }
        if replacement.len() > limits.max_replacement_bytes {
            return Err(PatternError::Resource(ResourceKind::ReplacementBytes));
        }
        budget.charge(1)?;
        let maximum =
            max_replacements.map_or_else(|| subject.len().saturating_add(1), |n| n.max(0) as usize);
        let mut output = Output {
            bytes: Vec::new(),
            maximum: limits.max_output_bytes,
        };
        let mut substitutions = 0;
        let mut cursor = 0;
        let mut frames = Vec::new();
        while substitutions < maximum {
            budget.charge(1)?;
            let found = self.run(subject, cursor, &mut frames, budget)?;
            if let Some(found) = &found {
                substitutions += 1;
                output.replace(subject, replacement, found, budget)?;
            }
            if let Some(found) = found.filter(|found| found.end > cursor) {
                cursor = found.end;
            } else if cursor < subject.len() {
                output.append(&subject[cursor..cursor + 1], budget)?;
                cursor += 1;
            } else {
                break;
            }
            if self.anchor {
                break;
            }
        }
        output.append(&subject[cursor..], budget)?;
        Ok(GsubResult {
            bytes: output.bytes,
            substitutions,
        })
    }
}
struct Output {
    bytes: Vec<u8>,
    maximum: usize,
}
impl Output {
    fn append(&mut self, bytes: &[u8], budget: &mut MatchBudget) -> Result<()> {
        let needed = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .filter(|n| *n <= self.maximum)
            .ok_or(PatternError::Resource(ResourceKind::OutputBytes))?;
        budget.charge(bytes.len() as u64)?;
        if needed > self.bytes.capacity() {
            let target = needed
                .max(self.bytes.capacity().saturating_mul(2))
                .min(self.maximum);
            self.bytes
                .try_reserve_exact(target - self.bytes.len())
                .map_err(|_| PatternError::Resource(ResourceKind::OutputBytes))?;
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    fn replace(
        &mut self,
        subject: &[u8],
        replacement: &[u8],
        found: &RawMatch,
        budget: &mut MatchBudget,
    ) -> Result<()> {
        let mut cursor = 0;
        while cursor < replacement.len() {
            budget.charge(1)?;
            let byte = replacement[cursor];
            cursor += 1;
            if byte != b'%' {
                self.append(&[byte], budget)?;
                continue;
            }
            // Lua strings have an accessible terminating NUL; add_s reads it
            // when the last authored byte is a percent escape.
            let escaped = replacement.get(cursor).copied().unwrap_or(0);
            cursor += 1;
            match escaped {
                b'0' => self.append(&subject[found.start..found.end], budget)?,
                b'1'..=b'9' => {
                    let index = usize::from(escaped - b'1');
                    if index >= found.captures.count {
                        if index == 0 {
                            self.append(&subject[found.start..found.end], budget)?;
                        } else {
                            return Err(PatternError::Source(
                                SourcePatternError::InvalidCaptureIndex,
                            ));
                        }
                    } else {
                        match found.captures.items[index] {
                            CaptureState::Open(_) => {
                                return Err(PatternError::Source(
                                    SourcePatternError::UnfinishedCapture,
                                ));
                            }
                            CaptureState::Bytes(start, end) => {
                                self.append(&subject[start..end], budget)?
                            }
                            CaptureState::Position(at) => self.position(at + 1, budget)?,
                        }
                    }
                }
                literal => self.append(&[literal], budget)?,
            }
        }
        Ok(())
    }
    fn position(&mut self, mut value: usize, budget: &mut MatchBudget) -> Result<()> {
        // At most three decimal digits per byte of usize; no numeric-formatting
        // allocation and no locale-dependent rendering of position captures.
        let mut digits = [0u8; std::mem::size_of::<usize>() * 3];
        let mut cursor = digits.len();
        loop {
            cursor -= 1;
            digits[cursor] = b'0' + (value % 10) as u8;
            value /= 10;
            if value == 0 {
                break;
            }
        }
        self.append(&digits[cursor..], budget)
    }
}
