//! Shared bounded source syntax lowering; authorization and admission are adapters.
pub(crate) use super::syntax::Lowerer;
use super::tokens::tokens;
pub(super) use crate::game_data::hash;
pub(super) use mlua::Lua;
pub(super) use poe_optimizer_data::item_loading::ItemSourceSpan;
pub(super) use poe_optimizer_data::modifier_parser::*;
pub(super) use poe_optimizer_data::source_program::{
    SourceProgramDefinitionRoot, SourceProgramRootId,
};
pub(super) use std::collections::{BTreeMap, BTreeSet};

/// Bindings authenticated by the source-construction adapter. Names alone never
/// grant captured intrinsic or definition-root identities to a closure.
#[derive(Default)]
pub(crate) struct LoweringBindings {
    pub(crate) roots: BTreeMap<String, SourceProgramDefinitionRoot>,
    pub(crate) intrinsics: BTreeMap<ParserCallbackId, ParserProgramIntrinsic>,
    pub(crate) implicit_self: bool,
    /// Actual source globals; when present all ordinary names resolve here.
    pub(crate) environment: Option<SourceProgramRootId>,
    /// Standalone owners may use receiver lookup and additional language primitives.
    /// Parser extraction keeps its separately reviewed capability inventory unchanged.
    pub(crate) standalone_calls: bool,
    pub(crate) closure_functions: BTreeMap<ParserCallbackId, super::closures::BoundFunction>,
    pub(crate) closure_prototypes:
        BTreeMap<ParserCallbackId, poe_optimizer_data::source_program::SourceClosurePrototypeId>,
}

pub(crate) type LowerResult<T> = std::result::Result<T, String>;
#[derive(Default)]
pub(crate) struct Budget {
    nodes: usize,
    bytes: usize,
    tokens: usize,
}
impl Budget {
    pub(super) fn node(&mut self) -> LowerResult<()> {
        self.nodes += 1;
        if self.nodes > 250_000 {
            return Err("aggregate program node bound".into());
        }
        Ok(())
    }
    pub(super) fn bytes(&mut self, count: usize) -> LowerResult<()> {
        self.bytes = self
            .bytes
            .checked_add(count)
            .ok_or("program text overflow")?;
        if count > 4096 || self.bytes > 4 * 1024 * 1024 {
            return Err("program literal/text bound".into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Lex<'a> {
    pub(super) text: &'a str,
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) quoted: bool,
    pub(super) numeric: bool,
}
pub(super) fn lex<'a>(body: &'a str, budget: &mut Budget) -> LowerResult<Vec<Lex<'a>>> {
    if body.len() > 64 * 1024 {
        return Err("complete function source byte bound".into());
    }
    let source_tokens = tokens(body).map_err(|e| e.to_string())?;
    budget.tokens = budget
        .tokens
        .checked_add(source_tokens.len())
        .ok_or("token count overflow")?;
    if budget.tokens > 2_000_000 {
        return Err("aggregate source token bound".into());
    }
    let mut out = Vec::with_capacity(source_tokens.len());
    let mut at = 0;
    while let Some(token) = source_tokens.get(at) {
        let start = token.text.as_ptr() as usize - body.as_ptr() as usize;
        let mut end = start + token.text.len();
        let numeric = !token.quoted
            && (body.as_bytes()[start].is_ascii_digit()
                || (token.text == "." && body.as_bytes().get(end).is_some_and(u8::is_ascii_digit)));
        if numeric {
            let bytes = body.as_bytes();
            let hex = bytes[start..].starts_with(b"0x") || bytes[start..].starts_with(b"0X");
            end = start;
            while let Some(byte) = bytes.get(end) {
                if byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.') {
                    end += 1;
                    if (matches!(byte, b'e' | b'E') && !hex || matches!(byte, b'p' | b'P') && hex)
                        && bytes.get(end).is_some_and(|b| matches!(b, b'+' | b'-'))
                    {
                        end += 1;
                    }
                } else {
                    break;
                }
            }
        } else if !token.quoted {
            for compound in ["...", "..", "==", "~=", "<=", ">="] {
                if body[start..].starts_with(compound) {
                    end = start + compound.len();
                    break;
                }
            }
        }
        while source_tokens
            .get(at)
            .is_some_and(|t| (t.text.as_ptr() as usize - body.as_ptr() as usize) < end)
        {
            at += 1;
        }
        out.push(Lex {
            text: &body[start..end],
            start,
            end,
            quoted: token.quoted,
            numeric,
        });
    }
    Ok(out)
}
/// Find the closing token of one complete function in a line-based debug span.
/// The shared scanner has already removed comments and marked string literals.
/// This recognizes block boundaries only: the lowerer must still consume and
/// validate every token inside the result, including currently unsupported blocks.
/// `for`/`while` open their block at `do`; counting both would hide the outer end.
pub(super) fn complete_function_token_end(tokens: &[Lex<'_>], start: usize) -> LowerResult<usize> {
    let mut closes = vec!["end"];
    for (index, token) in tokens.iter().enumerate().skip(start + 1) {
        if token.quoted {
            continue;
        }
        match token.text {
            "function" | "if" | "do" | "repeat" => {
                if closes.len() >= 64 {
                    return Err("function extent block depth bound".into());
                }
                closes.push(if token.text == "repeat" {
                    "until"
                } else {
                    "end"
                });
            }
            "end" | "until" => {
                if closes.pop() != Some(token.text) {
                    return Err("mismatched function/control block delimiter".into());
                }
                if closes.is_empty() {
                    return Ok(index + 1);
                }
            }
            _ => {}
        }
    }
    Err("unterminated function/control block".into())
}
pub(super) fn identifier(value: &str) -> bool {
    let mut chars = value.bytes();
    chars
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && chars.all(|b| b.is_ascii_alphanumeric() || b == b'_')
        && ![
            "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "if", "in",
            "local", "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
        ]
        .contains(&value)
}
