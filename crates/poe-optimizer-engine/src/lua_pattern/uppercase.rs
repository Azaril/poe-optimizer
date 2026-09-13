//! Closed string.upper replacement traversal, shared by both native parser paths.
//! The caller owns output storage and charges it before allocation; no callbacks
//! or function values are evaluated by this byte-pattern kernel.
use super::{Capture, LuaPattern, MatchBudget, PatternError};
use crate::item_tools::lua_number_text;

pub(crate) trait UppercaseOutput {
    type Error: From<PatternError>;
    fn append(&mut self, bytes: &[u8], uppercase: bool) -> Result<(), Self::Error>;
    fn scratch(&mut self, bytes: usize) -> Result<(), Self::Error>;
    fn resource(message: &'static str) -> Self::Error;
    fn invalid(message: &'static str) -> Self::Error;
}
fn append<O: UppercaseOutput>(
    output: &mut O,
    bytes: &[u8],
    uppercase: bool,
    work: &mut MatchBudget,
) -> Result<(), O::Error> {
    if !bytes.is_empty() {
        work.charge(bytes.len() as u64)?;
        output.append(bytes, uppercase)?;
    }
    Ok(())
}
pub(crate) fn replace_upper<O: UppercaseOutput>(
    line: &[u8],
    pattern: &LuaPattern,
    work: &mut MatchBudget,
    output: &mut O,
) -> Result<(), O::Error> {
    let mut copied = 0usize;
    loop {
        let init = copied
            .checked_add(1)
            .and_then(|n| i32::try_from(n).ok())
            .ok_or_else(|| O::resource("uppercase string index"))?;
        let Some(found) = pattern.match_captures(line, init, work)? else {
            break;
        };
        let range = found.range();
        append(output, &line[copied..range.start], false, work)?;
        work.charge(1)?;
        // A function replacement receives captures, or the full match when no
        // explicit captures exist. string.upper consumes only its first argument.
        match found.captures().first() {
            Some(Capture::Bytes { start, end }) => append(output, &line[*start..*end], true, work)?,
            Some(Capture::Position(position)) => {
                work.charge(128)?;
                output.scratch(128)?;
                let text = lua_number_text(*position as f64);
                append(output, text.as_bytes(), true, work)?;
            }
            None => return Err(O::invalid("gsub replacement capture")),
        }
        copied = range.end;
        // Lua 5.1 consumes one unchanged byte after an empty match.
        if range.is_empty() {
            if copied == line.len() {
                break;
            }
            append(output, &line[copied..copied + 1], false, work)?;
            copied += 1;
        }
        if pattern.source().first() == Some(&b'^') {
            break;
        }
    }
    append(output, &line[copied..], false, work)
}
