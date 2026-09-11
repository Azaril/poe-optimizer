//! Shared source-program compiler and per-build runtime, independent of parser data.
//! The parser compatibility facade delegates to this same compiler, VM and heap.
pub use crate::parser_program::{
    CompiledParserProgram as CompiledSourceProgram, CompiledProgramBinding, CompiledSourcePrograms,
    ProgramAllocationUsage, ProgramCompileError, ProgramInstruction, ProgramLimits,
    ProgramOperation, ProgramRuntimeError, ProgramRuntimeErrorKind, ProgramSession, ProgramTable,
    ProgramTableId, ProgramValue, ProgramValueGraph, RuntimeResult, SessionValue,
    SourceProgramOutput,
};
