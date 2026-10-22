use crate::{
    BinaryOperator, Diagnostic, DiagnosticCode, ExprId, ExprKind, ExprNode, LimitKind,
    NumericLiteral, ParsedExpression, Span, UnaryOperator, UnitSuffix,
    lexer::{Token, TokenKind, lex},
};

/// A hard ceiling on recursive parser calls even when caller limits are untrusted.
/// The flat arena can represent longer left-associated chains without recursion.
pub const MAX_PARSE_DEPTH: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseLimits {
    pub max_source_bytes: usize,
    /// Non-whitespace, non-comment tokens; no synthetic EOF token is counted.
    pub max_tokens: usize,
    /// Recursive expression nesting, including unary operators and call arguments.
    /// Must be no greater than MAX_PARSE_DEPTH. Zero permits no expression.
    pub max_depth: usize,
    pub max_nodes: usize,
}

impl Default for ParseLimits {
    fn default() -> Self {
        Self {
            max_source_bytes: 256 * 1024,
            max_tokens: 16 * 1024,
            max_depth: MAX_PARSE_DEPTH,
            max_nodes: 16 * 1024,
        }
    }
}

/// Parse one complete expression. This accepts unresolved ASCII symbolic paths
/// and calls, decimal spellings, adjacent `%`/named unit suffixes, and `//` comments.
/// Bare numbers leave units unresolved. No source constructs execute here.
///
/// Decimal literals allow `.5` and scientific notation. An `e`/`E` followed by
/// a digit, sign, or identifier boundary starts an exponent and requires digits;
/// otherwise it may start a unit name (`2energy`, `2e_unit`). A second
/// exponent-looking suffix is rejected. Unit validity and numeric range are
/// deliberately left to the lowerer.
pub fn parse_expression(
    source: &str,
    limits: ParseLimits,
) -> Result<ParsedExpression<'_>, Diagnostic> {
    if limits.max_depth > MAX_PARSE_DEPTH {
        return Err(Diagnostic::new(
            DiagnosticCode::InvalidLimits,
            Span { start: 0, end: 0 },
            "parser depth limit exceeds its stack safety ceiling",
        ));
    }
    let tokens = lex(source, limits)?;
    let mut parser = Parser {
        source,
        tokens,
        cursor: 0,
        nodes: Vec::new(),
        limits,
    };
    let root = parser.expression(0, 1)?;
    if parser.peek().is_some() {
        return Err(parser.error(
            DiagnosticCode::TrailingInput,
            "unexpected input after expression",
        ));
    }
    Ok(ParsedExpression {
        source,
        nodes: parser.nodes,
        root,
    })
}

struct Parser<'source> {
    source: &'source str,
    tokens: Vec<Token>,
    cursor: usize,
    nodes: Vec<ExprNode>,
    limits: ParseLimits,
}

impl Parser<'_> {
    fn peek(&self) -> Option<Token> {
        self.tokens.get(self.cursor).copied()
    }

    fn span(&self) -> Span {
        self.peek().map_or(
            Span {
                start: self.source.len(),
                end: self.source.len(),
            },
            |token| token.span,
        )
    }

    fn error(&self, code: DiagnosticCode, message: &'static str) -> Diagnostic {
        Diagnostic::new(code, self.span(), message)
    }

    fn eat(&mut self, kind: TokenKind) -> Option<Token> {
        if self.peek().is_some_and(|token| token.kind == kind) {
            let token = self.peek();
            self.cursor += 1;
            token
        } else {
            None
        }
    }

    fn expect(&mut self, kind: TokenKind, message: &'static str) -> Result<Token, Diagnostic> {
        self.eat(kind)
            .ok_or_else(|| self.error(DiagnosticCode::ExpectedToken, message))
    }

    fn push(&mut self, span: Span, kind: ExprKind) -> Result<ExprId, Diagnostic> {
        if self.nodes.len() == self.limits.max_nodes {
            return Err(Diagnostic::new(
                DiagnosticCode::LimitExceeded(LimitKind::Nodes),
                span,
                "expression exceeds the AST node limit",
            ));
        }
        let id = ExprId(self.nodes.len());
        self.nodes.push(ExprNode { span, kind });
        Ok(id)
    }

    fn expression(&mut self, minimum: u8, depth: usize) -> Result<ExprId, Diagnostic> {
        if depth > self.limits.max_depth {
            return Err(self.error(
                DiagnosticCode::LimitExceeded(LimitKind::Nesting),
                "expression exceeds the parser nesting limit",
            ));
        }
        let mut left = self.prefix(depth)?;
        while let Some((operator, precedence)) = self.peek().and_then(|token| binary(token.kind)) {
            if precedence < minimum {
                break;
            }
            if operator.is_comparison()
                && matches!(self.nodes[left.0].kind, ExprKind::Binary { operator: previous, .. } if previous.is_comparison())
            {
                return Err(self.error(
                    DiagnosticCode::ChainedComparison,
                    "comparisons cannot be chained; use explicit Boolean operators or parentheses",
                ));
            }
            let operator_span = self.peek().unwrap().span;
            self.cursor += 1;
            let right = self.expression(precedence + 1, depth + 1)?;
            left = self.push(
                self.nodes[left.0].span.cover(self.nodes[right.0].span),
                ExprKind::Binary {
                    operator,
                    operator_span,
                    left,
                    right,
                },
            )?;
        }
        Ok(left)
    }

    fn prefix(&mut self, depth: usize) -> Result<ExprId, Diagnostic> {
        let token = self
            .peek()
            .ok_or_else(|| self.error(DiagnosticCode::ExpectedExpression, "expected expression"))?;
        self.cursor += 1;
        match token.kind {
            TokenKind::Number => {
                let suffix = self.peek().filter(|next| next.span.start == token.span.end);
                let unit = match suffix {
                    Some(Token {
                        kind: TokenKind::Percent,
                        span,
                    }) => Some(UnitSuffix::Percent(span)),
                    Some(Token {
                        kind: TokenKind::Identifier,
                        span,
                    }) => Some(UnitSuffix::Named(span)),
                    _ => None,
                };
                let span = if unit.is_some() {
                    self.cursor += 1;
                    token.span.cover(suffix.unwrap().span)
                } else {
                    token.span
                };
                self.push(
                    span,
                    ExprKind::Number(NumericLiteral {
                        spelling: token.span,
                        unit,
                    }),
                )
            }
            TokenKind::True | TokenKind::False => {
                self.push(token.span, ExprKind::Boolean(token.kind == TokenKind::True))
            }
            TokenKind::Identifier => self.path_or_call(token, depth),
            TokenKind::Minus | TokenKind::Not => {
                let operand = self.expression(11, depth + 1)?;
                self.push(
                    token.span.cover(self.nodes[operand.0].span),
                    ExprKind::Unary {
                        operator: if token.kind == TokenKind::Minus {
                            UnaryOperator::Negate
                        } else {
                            UnaryOperator::Not
                        },
                        operator_span: token.span,
                        operand,
                    },
                )
            }
            TokenKind::LeftParen => {
                let inner = self.expression(0, depth + 1)?;
                let end = self.expect(
                    TokenKind::RightParen,
                    "expected ')' after grouped expression",
                )?;
                self.push(token.span.cover(end.span), ExprKind::Group { inner })
            }
            TokenKind::If => self.conditional(token, depth),
            _ => Err(Diagnostic::new(
                DiagnosticCode::ExpectedExpression,
                token.span,
                "expected expression",
            )),
        }
    }

    fn path_or_call(&mut self, first: Token, depth: usize) -> Result<ExprId, Diagnostic> {
        let mut segments = vec![first.span];
        let mut span = first.span;
        while self.eat(TokenKind::Dot).is_some() {
            let segment = self.expect(TokenKind::Identifier, "expected symbolic name after '.'")?;
            segments.push(segment.span);
            span = span.cover(segment.span);
        }
        let callee = self.push(span, ExprKind::Path { segments })?;
        if self.eat(TokenKind::LeftParen).is_none() {
            return Ok(callee);
        }
        let mut arguments = Vec::new();
        if !self
            .peek()
            .is_some_and(|token| token.kind == TokenKind::RightParen)
        {
            loop {
                arguments.push(self.expression(0, depth + 1)?);
                if self.eat(TokenKind::Comma).is_none() {
                    break;
                }
            }
        }
        let end = self.expect(TokenKind::RightParen, "expected ')' after call arguments")?;
        self.push(
            first.span.cover(end.span),
            ExprKind::Call { callee, arguments },
        )
    }

    fn block(&mut self, depth: usize) -> Result<(ExprId, Span), Diagnostic> {
        let start = self.expect(
            TokenKind::LeftBrace,
            "expected '{' before conditional branch",
        )?;
        let branch = self.expression(0, depth + 1)?;
        let end = self.expect(
            TokenKind::RightBrace,
            "expected '}' after conditional branch",
        )?;
        Ok((branch, start.span.cover(end.span)))
    }

    fn conditional(&mut self, start: Token, depth: usize) -> Result<ExprId, Diagnostic> {
        let condition = self.expression(0, depth + 1)?;
        let (then_branch, _) = self.block(depth)?;
        self.expect(
            TokenKind::Else,
            "conditional expression requires an 'else' branch",
        )?;
        let (else_branch, end) = if self.peek().is_some_and(|token| token.kind == TokenKind::If) {
            // Enter through expression so an else-if consumes the same depth budget.
            // Limit precedence to prefixes: operators after its final block belong
            // to the surrounding expression, not silently inside the else branch.
            let branch = self.expression(11, depth + 1)?;
            (branch, self.nodes[branch.0].span)
        } else {
            self.block(depth)?
        };
        self.push(
            start.span.cover(end),
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            },
        )
    }
}

fn binary(kind: TokenKind) -> Option<(BinaryOperator, u8)> {
    use BinaryOperator as B;
    use TokenKind as T;
    Some(match kind {
        T::Or => (B::Or, 1),
        T::And => (B::And, 3),
        T::Equal => (B::Equal, 5),
        T::NotEqual => (B::NotEqual, 5),
        T::Less => (B::Less, 5),
        T::LessEqual => (B::LessEqual, 5),
        T::Greater => (B::Greater, 5),
        T::GreaterEqual => (B::GreaterEqual, 5),
        T::Plus => (B::Add, 7),
        T::Minus => (B::Subtract, 7),
        T::Star => (B::Multiply, 9),
        T::Slash => (B::Divide, 9),
        _ => return None,
    })
}
