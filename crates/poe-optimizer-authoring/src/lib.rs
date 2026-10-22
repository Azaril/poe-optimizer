//! Offline authoring syntax for the existing checked native rule IR.
//!
//! Program syntax names explicit reads, formulas and effects. Exact schema
//! symbols resolve definitions and declared slots. This crate does not evaluate
//! formulas or authenticate read/effect authority. Emitted IR
//! must pass through the existing Engine compiler as part of a complete package.
//! Source spans are diagnostics, not semantic identity.
#![forbid(unsafe_code)]

mod allocation;
mod ast;
mod bindings;
mod diagnostic;
mod format;
mod lexer;
mod lower;
mod manifest;
mod manifest_parse;
mod packet;
mod parser;
mod program;
mod raise;
mod release;
mod symbols;

pub use allocation::{
    AllocationError, AllocationErrorKind, AllocationLimits, NAME_BINDINGS_VERSION, NameBinding,
    NameRequest, NamedIdRegistry, NamesInput,
};
pub use ast::{
    BinaryOperator, EffectDeclaration, ExprId, ExprKind, ExprNode, LetStatement, NumericLiteral,
    ParsedBlock, ParsedExpression, ParsedProgram, ReadDeclaration, Span, UnaryOperator, UnitSuffix,
};
pub use bindings::{
    BindingError, BindingErrorKind, BindingKind, BindingLimits, ExpressionBindings, UnitBinding,
    ValueBinding,
};
pub use diagnostic::{Diagnostic, DiagnosticCode, LimitKind};
pub use format::{FormatLimits, format_program};
pub use lower::{
    LowerLimits, LoweredBinding, LoweredBlock, LoweredExpression, lower_block, lower_expression,
};
pub use manifest::{NamesLimits, load_names, parse_packet};
pub use manifest_parse::{MAX_MANIFEST_DEPTH, ManifestError, ManifestErrorKind, ManifestLimits};
pub use packet::{
    LoweredPacket, LoweredPacketAppend, LoweredPacketProgram, PacketAppend, PacketError,
    PacketErrorKind, PacketLimit, PacketLimits, PacketLocation, PacketManifest, PacketProgram,
    RuleSource, lower_packet,
};
pub use parser::{
    BlockParseLimits, MAX_PARSE_DEPTH, ParseLimits, ProgramParseLimits, parse_block,
    parse_expression, parse_program,
};
pub use program::{LoweredProgram, ProgramLowerLimits, ProgramSpans, lower_program};
pub use raise::{
    RaiseError, RaiseErrorKind, RaiseLimit, RaiseLimits, RaiseLocation, RaisedNode, RaisedProgram,
    raise_program,
};
pub use release::{
    BuiltRelease, ReleaseBuildError, ReleaseBuildLimits, ReleaseManifest, ReleaseSource,
    ReleaseSourceSpec, SourceEncoding, build_release, parse_release_manifest,
};
pub use symbols::{SymbolError, SymbolErrorKind, SymbolLimits, SymbolRegistry};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod binding_tests;

#[cfg(test)]
mod lowering_tests;

#[cfg(test)]
mod block_tests;

#[cfg(test)]
mod symbol_tests;

#[cfg(test)]
mod program_parse_tests;

#[cfg(test)]
mod program_tests;

#[cfg(test)]
mod operation_tests;

#[cfg(test)]
mod raise_tests;

#[cfg(test)]
mod format_tests;

#[cfg(test)]
mod manifest_tests;

#[cfg(test)]
mod manifest_parse_tests;

#[cfg(test)]
mod packet_tests;

#[cfg(test)]
mod allocation_tests;
