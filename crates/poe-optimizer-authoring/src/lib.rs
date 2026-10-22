//! Offline authoring syntax for the existing checked native rule IR.
//!
//! Expression names bind to declared reads and exact schema units. This crate
//! does not evaluate formulas or authenticate read/effect authority. Emitted IR
//! must pass through the existing Engine compiler as part of a complete package.
//! Source spans are diagnostics, not semantic identity.
#![forbid(unsafe_code)]

mod ast;
mod bindings;
mod diagnostic;
mod lexer;
mod lower;
mod parser;

pub use ast::{
    BinaryOperator, ExprId, ExprKind, ExprNode, NumericLiteral, ParsedExpression, Span,
    UnaryOperator, UnitSuffix,
};
pub use bindings::{
    BindingError, BindingErrorKind, BindingLimits, ExpressionBindings, UnitBinding, ValueBinding,
};
pub use diagnostic::{Diagnostic, DiagnosticCode, LimitKind};
pub use lower::{LowerLimits, LoweredExpression, lower_expression};
pub use parser::{MAX_PARSE_DEPTH, ParseLimits, parse_expression};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod binding_tests;

#[cfg(test)]
mod lowering_tests;
