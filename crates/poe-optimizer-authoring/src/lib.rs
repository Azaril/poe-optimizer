//! Offline authoring syntax for the existing checked native rule IR.
//!
//! This crate currently parses expressions only. It does not evaluate formulas,
//! resolve names or units, authenticate reads, or admit calls as known functions.
//! Lowering must preserve association, signed zero, and lazy conditional branches
//! and must pass through the existing IR checker. Source spans are diagnostics,
//! not semantic identity.
#![forbid(unsafe_code)]

mod ast;
mod diagnostic;
mod lexer;
mod parser;

pub use ast::{
    BinaryOperator, ExprId, ExprKind, ExprNode, NumericLiteral, ParsedExpression, Span,
    UnaryOperator, UnitSuffix,
};
pub use diagnostic::{Diagnostic, DiagnosticCode, LimitKind};
pub use parser::{MAX_PARSE_DEPTH, ParseLimits, parse_expression};

#[cfg(test)]
mod tests;
