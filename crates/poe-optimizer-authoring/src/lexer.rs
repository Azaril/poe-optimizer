use crate::{Diagnostic, DiagnosticCode, LimitKind, ParseLimits, Span};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TokenKind {
    Identifier,
    Number,
    True,
    False,
    If,
    Else,
    Not,
    And,
    Or,
    Plus,
    Minus,
    Star,
    Slash,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Dot,
    Percent,
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

/// Numbers use decimal digits, an optional nonempty fractional part, and an
/// optional exponent. `.5` is allowed; `1.` and numeric separators are not.
/// `e`/`E` starts an exponent when followed by a digit/sign or when it ends the
/// identifier. Otherwise it can start a named suffix, as in `2energy`.
/// A second exponent-looking suffix is rejected, so `1e2e3` cannot silently
/// become a numeric value with an unresolved `e3` unit.
pub(crate) fn lex(source: &str, limits: ParseLimits) -> Result<Vec<Token>, Diagnostic> {
    if source.len() > limits.max_source_bytes {
        return Err(Diagnostic::new(
            DiagnosticCode::LimitExceeded(LimitKind::SourceBytes),
            Span {
                start: 0,
                end: source.len(),
            },
            "expression exceeds the source byte limit",
        ));
    }
    let mut lexer = Lexer {
        source,
        cursor: 0,
        tokens: Vec::new(),
    };
    while lexer.cursor < source.len() {
        let character = source[lexer.cursor..].chars().next().unwrap();
        if character.is_whitespace() {
            lexer.cursor += character.len_utf8();
            continue;
        }
        if source[lexer.cursor..].starts_with("//") {
            while lexer.cursor < source.len() && source.as_bytes()[lexer.cursor] != b'\n' {
                lexer.cursor += 1;
            }
            continue;
        }
        let start = lexer.cursor;
        if lexer.tokens.len() == limits.max_tokens {
            return Err(Diagnostic::new(
                DiagnosticCode::LimitExceeded(LimitKind::Tokens),
                Span {
                    start,
                    end: start + character.len_utf8(),
                },
                "expression exceeds the token limit",
            ));
        }
        let kind = if character.is_ascii_digit()
            || (character == '.' && lexer.byte(1).is_some_and(|c| c.is_ascii_digit()))
        {
            lexer.number()?;
            TokenKind::Number
        } else if character.is_ascii_alphabetic() || character == '_' {
            lexer.cursor += 1;
            while lexer.byte(0).is_some_and(identifier_continue) {
                lexer.cursor += 1;
            }
            match &source[start..lexer.cursor] {
                "true" => TokenKind::True,
                "false" => TokenKind::False,
                "if" => TokenKind::If,
                "else" => TokenKind::Else,
                "not" => TokenKind::Not,
                "and" => TokenKind::And,
                "or" => TokenKind::Or,
                _ => TokenKind::Identifier,
            }
        } else {
            lexer.cursor += character.len_utf8();
            match character {
                '+' => TokenKind::Plus,
                '-' => TokenKind::Minus,
                '*' => TokenKind::Star,
                '/' => TokenKind::Slash,
                '.' => TokenKind::Dot,
                '%' => TokenKind::Percent,
                '(' => TokenKind::LeftParen,
                ')' => TokenKind::RightParen,
                '{' => TokenKind::LeftBrace,
                '}' => TokenKind::RightBrace,
                ',' => TokenKind::Comma,
                '=' if lexer.eat(b'=') => TokenKind::Equal,
                '!' if lexer.eat(b'=') => TokenKind::NotEqual,
                '<' if lexer.eat(b'=') => TokenKind::LessEqual,
                '>' if lexer.eat(b'=') => TokenKind::GreaterEqual,
                '<' => TokenKind::Less,
                '>' => TokenKind::Greater,
                _ => {
                    return Err(Diagnostic::new(
                        DiagnosticCode::UnexpectedCharacter,
                        Span {
                            start,
                            end: lexer.cursor,
                        },
                        "unexpected character in expression",
                    ));
                }
            }
        };
        lexer.tokens.push(Token {
            kind,
            span: Span {
                start,
                end: lexer.cursor,
            },
        });
    }
    Ok(lexer.tokens)
}

fn identifier_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

struct Lexer<'source> {
    source: &'source str,
    cursor: usize,
    tokens: Vec<Token>,
}

impl Lexer<'_> {
    fn byte(&self, offset: usize) -> Option<u8> {
        self.source.as_bytes().get(self.cursor + offset).copied()
    }

    fn eat(&mut self, byte: u8) -> bool {
        if self.byte(0) == Some(byte) {
            self.cursor += 1;
            true
        } else {
            false
        }
    }

    fn digits(&mut self) -> usize {
        let start = self.cursor;
        while self.byte(0).is_some_and(|byte| byte.is_ascii_digit()) {
            self.cursor += 1;
        }
        self.cursor - start
    }

    fn number(&mut self) -> Result<(), Diagnostic> {
        let start = self.cursor;
        self.digits();
        if self.eat(b'.') && self.digits() == 0 {
            return Err(self.invalid_number(start, "decimal point must be followed by digits"));
        }
        if matches!(self.byte(0), Some(b'e' | b'E')) {
            let exponent = self.byte(1).is_none_or(|next| {
                next.is_ascii_digit() || matches!(next, b'+' | b'-') || !identifier_continue(next)
            });
            if exponent {
                self.cursor += 1;
                if matches!(self.byte(0), Some(b'+' | b'-')) {
                    self.cursor += 1;
                }
                if self.digits() == 0 {
                    return Err(self.invalid_number(start, "exponent must contain decimal digits"));
                }
                if matches!(self.byte(0), Some(b'e' | b'E'))
                    && self.byte(1).is_none_or(|next| {
                        next.is_ascii_digit()
                            || matches!(next, b'+' | b'-')
                            || !identifier_continue(next)
                    })
                {
                    return Err(
                        self.invalid_number(start, "numeric literal contains another exponent")
                    );
                }
            }
        }
        if self.byte(0) == Some(b'_') {
            return Err(self.invalid_number(start, "numeric separators are not supported"));
        }
        if self.byte(0) == Some(b'.') {
            return Err(
                self.invalid_number(start, "numeric literal contains another decimal point")
            );
        }
        Ok(())
    }

    fn invalid_number(&self, start: usize, message: &'static str) -> Diagnostic {
        Diagnostic::new(
            DiagnosticCode::InvalidNumber,
            Span {
                start,
                end: self.cursor,
            },
            message,
        )
    }
}
