//! Shared source scanner. Parsing policy and admission remain with callers.
use crate::game_data::{GameDataExtractionError, error};
type Result<T> = std::result::Result<T, GameDataExtractionError>;

#[derive(Debug)]
pub(crate) struct Token<'a> {
    pub(crate) text: &'a str,
    pub(crate) line: usize,
    pub(crate) quoted: bool,
}
pub(crate) fn tokens(text: &str) -> Result<Vec<Token<'_>>> {
    let bytes = text.as_bytes();
    let mut i = 0;
    let mut line = 1;
    let mut out = Vec::new();
    while i < bytes.len() {
        let start = i;
        let start_line = line;
        if bytes[i].is_ascii_whitespace() {
            if bytes[i] == b'\n' {
                line += 1;
            }
            i += 1;
            continue;
        }
        let comment = bytes[i..].starts_with(b"--");
        if comment {
            i += 2;
        }
        let long_start = i;
        let long_open = if bytes.get(i) == Some(&b'[') {
            let mut j = i + 1;
            while bytes.get(j) == Some(&b'=') {
                j += 1;
            }
            (bytes.get(j) == Some(&b'[')).then_some(j)
        } else {
            None
        };
        if let Some(end) = long_open {
            let closer = format!("]{}]", "=".repeat(end - long_start - 1));
            let tail = &text[end + 1..];
            let close = tail
                .find(&closer)
                .ok_or_else(|| error("unterminated Lua long literal"))?;
            i = end + 1 + close + closer.len();
            line += text[start..i].bytes().filter(|b| *b == b'\n').count();
            if !comment {
                out.push(Token {
                    text: &text[start..i],
                    line: start_line,
                    quoted: true,
                });
            }
            continue;
        }
        if comment {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        let quoted = bytes[i] == b'\'' || bytes[i] == b'"';
        if quoted {
            let quote = bytes[i];
            i += 1;
            let mut closed = false;
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    i += 1;
                    if bytes.get(i) == Some(&b'\n') {
                        line += 1;
                    }
                    i += 1;
                } else if bytes[i] == quote {
                    i += 1;
                    closed = true;
                    break;
                } else {
                    if bytes[i] == b'\n' {
                        line += 1;
                    }
                    i += 1;
                }
            }
            if !closed {
                return Err(error("unterminated Lua quoted literal"));
            }
        } else if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
            i += 1;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
        } else {
            i += 1;
        }
        // All code outside literals in the reviewed files is ASCII. Reject an
        // unsupported future source spelling instead of slicing a UTF-8 codepoint.
        if !text.is_char_boundary(i) {
            return Err(error(
                "non-ASCII Lua code token requires reviewed scanner support",
            ));
        }
        out.push(Token {
            text: &text[start..i],
            line: start_line,
            quoted,
        });
    }
    Ok(out)
}
