//! Complete immediate-prototype table inventory in source allocation order.
use crate::source_programs::lowering::{Lex, complete_function_token_end, identifier};

pub(super) struct TableSite {
    pub start: usize,
    pub end: usize,
    /// bcemit_INS records lastline before expr_table consumes the opening brace.
    pub previous_token_end: usize,
    pub list_fields: Option<usize>,
}
pub(super) fn inventory(tokens: &[Lex<'_>]) -> std::result::Result<Vec<TableSite>, String> {
    let mut starts = Vec::new();
    let mut pairs = Vec::new();
    let mut index = 1; // Complete outer function token is index zero.
    while index < tokens.len() {
        let token = tokens[index];
        if !token.quoted {
            match token.text {
                "function" => {
                    index = complete_function_token_end(tokens, index)?;
                    continue;
                }
                "{" => {
                    if pairs.len() >= 4096 || starts.len() >= 32 {
                        return Err("constructor source occurrence/depth bound".into());
                    }
                    starts.push((index, pairs.len()));
                    pairs.push((index, 0));
                }
                "}" => {
                    let (_, pair) = starts.pop().ok_or("unmatched constructor delimiter")?;
                    pairs[pair].1 = index;
                }
                _ => {}
            }
        }
        index += 1;
    }
    if !starts.is_empty() {
        return Err("unterminated constructor source occurrence".into());
    }
    pairs
        .into_iter()
        .map(|(start, end)| {
            Ok(TableSite {
                start: tokens[start].start,
                end: tokens[end].end,
                previous_token_end: tokens[start - 1].end,
                list_fields: list_fields(tokens, start, end)?,
            })
        })
        .collect()
}
fn list_fields(
    tokens: &[Lex<'_>],
    start: usize,
    end: usize,
) -> std::result::Result<Option<usize>, String> {
    let mut count = 0;
    let mut field_start = true;
    let mut delimiters = Vec::new();
    let mut index = start + 1;
    while index < end {
        let token = tokens[index];
        if field_start && delimiters.is_empty() {
            if !token.quoted
                && (token.text == "["
                    || identifier(token.text)
                        && tokens.get(index + 1).is_some_and(|next| next.text == "="))
            {
                return Ok(None);
            }
            count += 1;
            field_start = false;
        }
        if !token.quoted {
            match token.text {
                "function" => {
                    index = complete_function_token_end(tokens, index)?;
                    continue;
                }
                "(" => delimiters.push(")"),
                "[" => delimiters.push("]"),
                "{" => delimiters.push("}"),
                ")" | "]" | "}" => {
                    if delimiters.pop() != Some(token.text) {
                        return Err("constructor field delimiter mismatch".into());
                    }
                }
                "," | ";" if delimiters.is_empty() => field_start = true,
                _ => {}
            }
        }
        index += 1;
    }
    if !delimiters.is_empty() {
        return Err("unterminated constructor field delimiter".into());
    }
    Ok(Some(count))
}
