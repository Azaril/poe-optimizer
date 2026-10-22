use crate::Span;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LimitKind {
    SourceBytes,
    Tokens,
    Nesting,
    Nodes,
    LoweredNodes,
    Statements,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticCode {
    LimitExceeded(LimitKind),
    InvalidLimits,
    UnexpectedCharacter,
    InvalidNumber,
    ExpectedExpression,
    ExpectedToken,
    ChainedComparison,
    TrailingInput,
    UnknownSymbol,
    UnknownUnit,
    UnknownFunction,
    InvalidArguments,
    TypeMismatch,
    DuplicateSymbol,
}

/// One bounded diagnostic at the first error. Spans always select complete UTF-8
/// characters (or an empty range at a boundary); messages never copy input text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub span: Span,
    pub message: &'static str,
}

impl Diagnostic {
    pub(crate) fn new(code: DiagnosticCode, span: Span, message: &'static str) -> Self {
        Self {
            code,
            span,
            message,
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at bytes {}..{}",
            self.message, self.span.start, self.span.end
        )
    }
}

impl std::error::Error for Diagnostic {}
