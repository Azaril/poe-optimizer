//! Shared native execution of compiled source programs and persistent build sessions.
mod classes;
mod execute;
mod session;
pub use session::{ProgramSession, SessionValue};
mod intrinsics;
#[cfg(test)]
mod tests;
mod value;
use crate::lua_pattern::{MatchLimits, PatternError};
use poe_optimizer_data::modifier_parser::{
    ModifierParserCatalog, ParserCallbackId, ParserProgramLocation,
};
use poe_optimizer_data::source_program::SourceProgramOwner;
pub use value::{
    ProgramTable, ProgramTableCoverage, ProgramTableId, ProgramValue, ProgramValueGraph,
};

#[derive(Debug, Clone, Copy)]
pub struct ProgramLimits {
    pub max_steps: u64,
    pub max_values: usize,
    pub max_bytes: usize,
    pub max_tables: usize,
    pub max_call_depth: usize,
    pub max_results: usize,
    pub pattern: MatchLimits,
}
impl Default for ProgramLimits {
    fn default() -> Self {
        Self {
            max_steps: 1_000_000,
            max_values: 65_536,
            max_bytes: 8 * 1024 * 1024,
            max_tables: 16_384,
            max_call_depth: 32,
            max_results: 4_096,
            pattern: MatchLimits::default(),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramRuntimeErrorKind {
    InvalidInput,
    Source,
    ResourceBound,
    UnsupportedCapability,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramRuntimeError {
    pub kind: ProgramRuntimeErrorKind,
    pub callback: Option<ParserCallbackId>,
    pub location: Option<ParserProgramLocation>,
    pub message: String,
}
pub type RuntimeResult<T> = std::result::Result<T, ProgramRuntimeError>;
impl ProgramRuntimeError {
    fn new(kind: ProgramRuntimeErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            callback: None,
            location: None,
            message: message.into(),
        }
    }
    pub(crate) fn input(message: impl Into<String>) -> Self {
        Self::new(ProgramRuntimeErrorKind::InvalidInput, message)
    }
    pub(crate) fn source(message: impl Into<String>) -> Self {
        Self::new(ProgramRuntimeErrorKind::Source, message)
    }
    pub(crate) fn resource(message: impl Into<String>) -> Self {
        Self::new(ProgramRuntimeErrorKind::ResourceBound, message)
    }
    pub(crate) fn unsupported(message: impl Into<String>) -> Self {
        Self::new(ProgramRuntimeErrorKind::UnsupportedCapability, message)
    }
    fn at(mut self, callback: ParserCallbackId, location: ParserProgramLocation) -> Self {
        if self.callback.is_none() {
            self.callback = Some(callback);
        }
        if self.location.is_none() {
            self.location = Some(location);
        }
        self
    }
}
impl std::fmt::Display for ProgramRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "parser program {:?} at {:?}/{:?}: {}",
            self.kind, self.callback, self.location, self.message
        )
    }
}
impl std::error::Error for ProgramRuntimeError {}
impl From<PatternError> for ProgramRuntimeError {
    fn from(value: PatternError) -> Self {
        match value {
            PatternError::Source(_) => Self::source(value.to_string()),
            PatternError::Resource(_) => Self::resource(value.to_string()),
        }
    }
}
/// Cumulative allocation accounting, including temporary pack storage and graph
/// export. These counters are not a claim of measured peak resident memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgramAllocationUsage {
    pub values: usize,
    pub bytes: usize,
    pub tables: usize,
}
/// A raw return pack plus reachable table graph, before parser call-site coercion
/// or recursive copying. Callback identities are contextual to the retained owner.
#[derive(Debug, Clone)]
pub struct SourceProgramOutput {
    graph: ProgramValueGraph,
    owner: SourceProgramOwner,
    steps: u64,
    pattern_steps: u64,
    allocations: ProgramAllocationUsage,
}
impl SourceProgramOutput {
    pub fn graph(&self) -> &ProgramValueGraph {
        &self.graph
    }
    pub fn allocations(&self) -> ProgramAllocationUsage {
        self.allocations
    }
    pub fn owner(&self) -> &SourceProgramOwner {
        &self.owner
    }
    pub fn steps(&self) -> u64 {
        self.steps
    }
    pub fn pattern_steps(&self) -> u64 {
        self.pattern_steps
    }
}

/// Parser-compatible output retaining the original parser owner.
#[derive(Debug, Clone)]
pub struct ProgramOutput {
    source: SourceProgramOutput,
    owner: ModifierParserCatalog,
}
impl ProgramOutput {
    pub fn graph(&self) -> &ProgramValueGraph {
        self.source.graph()
    }
    pub fn owner(&self) -> &ModifierParserCatalog {
        &self.owner
    }
    pub fn allocations(&self) -> ProgramAllocationUsage {
        self.source.allocations()
    }
    pub fn steps(&self) -> u64 {
        self.source.steps()
    }
    pub fn pattern_steps(&self) -> u64 {
        self.source.pattern_steps()
    }
    pub fn source_output(&self) -> &SourceProgramOutput {
        &self.source
    }
}

/// One request's cumulative program work and allocations. The public standalone
/// executor creates a fresh instance; parser dispatch retains one across calls
/// and retries. Failed input validation, execution and export keep their charges.
#[derive(Debug)]
pub(crate) struct ProgramRequestAccounting {
    limits: ProgramLimits,
    steps: u64,
    allocations: value::HeapStats,
}
impl ProgramRequestAccounting {
    pub(crate) fn new(limits: ProgramLimits) -> Self {
        Self {
            limits,
            steps: 0,
            allocations: value::HeapStats::default(),
        }
    }
    pub(crate) fn steps(&self) -> u64 {
        self.steps
    }
    pub(crate) fn allocation_usage(&self) -> ProgramAllocationUsage {
        ProgramAllocationUsage {
            values: self.allocations.values,
            bytes: self.allocations.bytes,
            tables: self.allocations.tables,
        }
    }
}
