//! Offline authoring syntax for the existing checked native rule IR.
//!
//! Program syntax names explicit reads, formulas and effects. Exact schema
//! symbols resolve definitions and declared slots. This crate does not evaluate
//! formulas or authenticate read/effect authority. Emitted IR
//! must pass through the existing Engine compiler as part of a complete package.
//! Source spans are diagnostics, not semantic identity.
#![forbid(unsafe_code)]

mod ast;
mod bindings;
mod diagnostic;
mod lexer;
mod lower;
mod parser;
mod program;
mod symbols;

pub use ast::{
    BinaryOperator, EffectDeclaration, ExprId, ExprKind, ExprNode, LetStatement, NumericLiteral,
    ParsedBlock, ParsedExpression, ParsedProgram, ReadDeclaration, Span, UnaryOperator, UnitSuffix,
};
pub use bindings::{
    BindingError, BindingErrorKind, BindingLimits, ExpressionBindings, UnitBinding, ValueBinding,
};
pub use diagnostic::{Diagnostic, DiagnosticCode, LimitKind};
pub use lower::{
    LowerLimits, LoweredBinding, LoweredBlock, LoweredExpression, lower_block, lower_expression,
};
pub use parser::{
    BlockParseLimits, MAX_PARSE_DEPTH, ParseLimits, ProgramParseLimits, parse_block,
    parse_expression, parse_program,
};
pub use program::{LoweredProgram, ProgramLowerLimits, ProgramSpans, lower_program};
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
