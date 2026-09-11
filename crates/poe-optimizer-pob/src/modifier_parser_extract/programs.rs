//! Bounded whole-function lowering into source-bound typed programs.
//! This pass never executes a source body or changes a legacy factory recipe.
use super::*;
use std::collections::BTreeSet;
mod syntax;
use syntax::Lowerer;

pub(super) struct LoweredPrograms {
    pub(super) data: ParserProgramData,
    pub(super) unsupported: BTreeMap<ParserCallbackId, String>,
}

pub(super) fn lower(
    lua: &Lua,
    sources: &BTreeMap<String, String>,
    data: &ModifierParserData,
    constructor: ParserCallbackId,
) -> Result<LoweredPrograms> {
    let mut programs = BTreeMap::new();
    let mut unsupported = BTreeMap::new();
    let mut budget = Budget::default();
    for (index, callback) in data.callbacks.iter().enumerate() {
        let id = ParserCallbackId(index as u32 + 1);
        if matches!(
            data.factories.get(&id),
            Some(ParserFactoryDisposition::Pure(_))
        ) {
            continue;
        }
        let ParserCallbackKind::Lua { source: span } = &callback.kind else {
            unsupported.insert(id, "builtin callback is not a source program".into());
            continue;
        };
        let text = source(sources, &span.path)?;
        let body = text
            .split_inclusive('\n')
            .skip(span.line as usize - 1)
            .take((span.end_line - span.line + 1) as usize)
            .collect::<String>();
        if hash(body.as_bytes()) != span.sha256 {
            return Err(error("program source span mismatch"));
        }
        let result = Lowerer::new(lua, &body, id, callback, data, constructor, &mut budget)
            .and_then(|lowerer| lowerer.program(span));
        match result {
            Ok(program) => {
                programs.insert(id, program);
            }
            Err(reason) => {
                unsupported.insert(id, reason);
            }
        }
    }
    // A source call is meaningful only with its captured callee's own complete
    // body. Remove dependants transitively; never inline or invent that body.
    loop {
        let rejected = programs.iter().filter_map(|(id, program)| {
            program.bindings.iter().find_map(|binding| {
                let ParserProgramBinding::CapturedCallback { callback, .. } = binding else { return None };
                if programs.contains_key(callback) { return None; }
                let reason = if matches!(data.factories.get(callback), Some(ParserFactoryDisposition::Pure(_))) {
                    format!("captured helper {callback:?} requires the unavailable raw legacy factory bridge")
                } else {
                    format!("captured helper {callback:?} has no complete lowered program")
                };
                Some((*id, reason))
            })
        }).collect::<Vec<_>>();
        if rejected.is_empty() {
            break;
        }
        for (id, reason) in rejected {
            programs.remove(&id);
            unsupported.insert(id, reason);
        }
    }
    let callbacks = programs
        .keys()
        .enumerate()
        .map(|(index, callback)| (*callback, ParserProgramId(index as u32 + 1)))
        .collect();
    Ok(LoweredPrograms {
        data: ParserProgramData {
            schema_version: PARSER_PROGRAM_SCHEMA_VERSION,
            programs: programs.into_values().collect(),
            callbacks,
        },
        unsupported,
    })
}

type LowerResult<T> = std::result::Result<T, String>;
#[derive(Default)]
struct Budget {
    nodes: usize,
    bytes: usize,
    tokens: usize,
}
impl Budget {
    fn node(&mut self) -> LowerResult<()> {
        self.nodes += 1;
        if self.nodes > 250_000 {
            return Err("aggregate program node bound".into());
        }
        Ok(())
    }
    fn bytes(&mut self, count: usize) -> LowerResult<()> {
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
struct Lex<'a> {
    text: &'a str,
    start: usize,
    end: usize,
    quoted: bool,
    numeric: bool,
}
fn lex<'a>(body: &'a str, budget: &mut Budget) -> LowerResult<Vec<Lex<'a>>> {
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
fn identifier(value: &str) -> bool {
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

#[cfg(test)]
mod tests;
