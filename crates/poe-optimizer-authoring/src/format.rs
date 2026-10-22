//! Syntax-only formatting: original tokens and comment bodies remain intact.
//!
//! This module never resolves symbols, lowers formulas or raises rule IR. Its
//! only edits are to whitespace outside tokens and line-comment bodies.
use crate::{
    Diagnostic, DiagnosticCode, ExprKind, LimitKind, ProgramParseLimits, Span, UnaryOperator,
    lexer::{Token, TokenKind, lex},
    parse_program,
};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FormatLimits {
    pub parse: ProgramParseLimits,
    /// Generated text must fit this cap and the parser's source byte cap so
    /// every successful result remains admissible to the same formatter.
    pub max_output_bytes: usize,
}

impl Default for FormatLimits {
    fn default() -> Self {
        Self {
            parse: ProgramParseLimits::default(),
            max_output_bytes: 1024 * 1024,
        }
    }
}

/// Format one syntactically valid program using four-space indentation and LF
/// structural line breaks. Preserve numeric spellings, names, keys, grouping,
/// comments and the adjacency of unit suffixes. Output ends with one structural
/// newline. Existing `//` bodies end only at LF: any CR, Unicode whitespace or
/// trailing spaces inside a comment remain literal comment bytes.
///
/// Parse errors are returned before considering the independent output budget.
/// A zero output budget therefore produces OutputBytes for valid input.
/// Successful output also fits the parser's source byte cap. Exceeding that
/// smaller cap reports SourceBytes for generated output; equal caps report
/// OutputBytes. No additional parsing or semantic evaluation is performed.
pub fn format_program(source: &str, limits: FormatLimits) -> Result<String, Diagnostic> {
    let parsed = parse_program(source, limits.parse)?;
    let unary_minus = parsed
        .nodes()
        .iter()
        .filter_map(|node| match node.kind {
            ExprKind::Unary {
                operator: UnaryOperator::Negate,
                operator_span,
                ..
            } => Some(operator_span.start),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    // The formatter needs only unary punctuation positions, not a second AST.
    drop(parsed);
    let tokens = lex(source, limits.parse.expressions)?;
    let source_maximum = limits.parse.expressions.max_source_bytes;
    let mut output = Output {
        text: String::new(),
        maximum: limits.max_output_bytes.min(source_maximum),
        limit_kind: if source_maximum < limits.max_output_bytes {
            LimitKind::SourceBytes
        } else {
            LimitKind::OutputBytes
        },
        indentation: 0,
        line_start: true,
        pending_newline: false,
    };
    let mut previous = None;
    let mut cursor = 0;
    for token in tokens {
        gap(
            source,
            cursor,
            token.span.start,
            previous.is_some(),
            &mut output,
        )?;
        output.token(source, token, previous, &unary_minus)?;
        cursor = token.span.end;
        previous = Some(token);
    }
    gap(
        source,
        cursor,
        source.len(),
        previous.is_some(),
        &mut output,
    )?;
    output.newline(Span {
        start: source.len(),
        end: source.len(),
    })?;
    Ok(output.text)
}

struct Output {
    text: String,
    maximum: usize,
    limit_kind: LimitKind,
    indentation: usize,
    line_start: bool,
    // Delayed so an inline comment after ';' or '{' stays with its token.
    pending_newline: bool,
}

impl Output {
    fn append(&mut self, text: &str, span: Span) -> Result<(), Diagnostic> {
        if self
            .text
            .len()
            .checked_add(text.len())
            .is_none_or(|end| end > self.maximum)
        {
            return Err(Diagnostic::new(
                DiagnosticCode::LimitExceeded(self.limit_kind),
                span,
                if self.limit_kind == LimitKind::SourceBytes {
                    "formatted source exceeds the parser source byte limit"
                } else {
                    "formatted source exceeds the output byte limit"
                },
            ));
        }
        self.text.push_str(text);
        Ok(())
    }

    fn newline(&mut self, span: Span) -> Result<(), Diagnostic> {
        if !self.line_start {
            self.append("\n", span)?;
            self.line_start = true;
        }
        self.pending_newline = false;
        Ok(())
    }

    fn indent(&mut self, span: Span) -> Result<(), Diagnostic> {
        if self.line_start {
            for _ in 0..self.indentation {
                self.append("    ", span)?;
            }
        }
        Ok(())
    }

    fn comment(&mut self, source: &str, span: Span, inline: bool) -> Result<(), Diagnostic> {
        if inline && !self.line_start {
            self.append(" ", span)?;
        } else {
            self.newline(span)?;
            self.indent(span)?;
        }
        self.append(&source[span.start..span.end], span)?;
        self.line_start = false;
        // Do not trim CR or trailing spaces: they belong to this lexer-defined
        // comment body. LF terminates both the original and formatted comment.
        self.newline(span)
    }

    fn token(
        &mut self,
        source: &str,
        token: Token,
        previous: Option<Token>,
        unary_minus: &BTreeSet<usize>,
    ) -> Result<(), Diagnostic> {
        if token.kind == TokenKind::RightBrace {
            // Parsing has already checked balanced program/conditional braces.
            self.indentation = self
                .indentation
                .checked_sub(1)
                .expect("parsed braces balance");
            self.newline(token.span)?;
        } else if self.pending_newline {
            self.newline(token.span)?;
        }
        if self.line_start {
            self.indent(token.span)?;
        } else if previous.is_some_and(|previous| needs_space(previous, token, unary_minus)) {
            self.append(" ", token.span)?;
        }
        self.append(&source[token.span.start..token.span.end], token.span)?;
        self.line_start = false;
        match token.kind {
            TokenKind::LeftBrace => {
                self.indentation += 1;
                self.pending_newline = true;
            }
            TokenKind::Semicolon => self.pending_newline = true,
            _ => {}
        }
        Ok(())
    }
}

/// Recover discarded trivia directly from the bounded input. Token gaps contain
/// only whitespace and comments because the same lexer has already succeeded.
/// Scan comments iteratively and copy each borrowed slice once; no trivia arena.
fn gap(
    source: &str,
    start: usize,
    end: usize,
    has_previous_token: bool,
    output: &mut Output,
) -> Result<(), Diagnostic> {
    let mut cursor = start;
    let mut line_break = false;
    while cursor < end {
        if source[cursor..end].starts_with("//") {
            let comment_end = source[cursor..end]
                .find('\n')
                .map_or(end, |offset| cursor + offset);
            output.comment(
                source,
                Span {
                    start: cursor,
                    end: comment_end,
                },
                has_previous_token && !line_break,
            )?;
            cursor = comment_end;
            if cursor < end {
                cursor += 1;
            }
            line_break = true;
        } else {
            let character = source[cursor..end]
                .chars()
                .next()
                .expect("nonempty token gap");
            debug_assert!(
                character.is_whitespace(),
                "successful lexer left only trivia"
            );
            // Use the lexer's LF boundary for comment affinity too. Bare CR
            // and Unicode whitespace must not silently move an inline comment
            // onto a new lexical line. CRLF already contains an LF boundary.
            line_break |= character == '\n';
            cursor += character.len_utf8();
        }
    }
    Ok(())
}

fn needs_space(previous: Token, token: Token, unary_minus: &BTreeSet<usize>) -> bool {
    use TokenKind as T;
    if matches!(
        token.kind,
        T::Dot | T::Comma | T::Semicolon | T::Colon | T::RightParen
    ) || previous.kind == T::Dot
        || previous.kind == T::LeftParen
    {
        return false;
    }
    if previous.kind == T::Number
        && matches!(token.kind, T::Identifier | T::Percent)
        && previous.span.end == token.span.start
    {
        return false;
    }
    if previous.kind == T::Minus && unary_minus.contains(&previous.span.start) {
        return false;
    }
    if token.kind == T::LeftParen && previous.kind == T::Identifier {
        return false;
    }
    true
}
