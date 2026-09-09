//! Closed byte-string operations for proven factory expressions.
//! No generic method execution, metatables, Unicode casing or shared mutable state.
use super::{ModifierValue as V, ParserError, ParserResult, value::OutputBudget};
use crate::item_tools::lua_number_text;
use crate::lua_pattern::{Capture, LuaPattern, MatchBudget};
use std::borrow::Cow;

fn number_text(
    value: f64,
    budget: &mut MatchBudget,
    output: &mut OutputBudget,
) -> ParserResult<Vec<u8>> {
    // The shared fourteen-significant-digit renderer has bounded temporary
    // storage. Reserve its work/storage budget before any formatting allocation.
    budget.charge(128)?;
    output.charge(128)?;
    Ok(lua_number_text(value).into_bytes())
}
fn operand<'a>(
    value: &'a V,
    budget: &mut MatchBudget,
    output: &mut OutputBudget,
) -> ParserResult<Cow<'a, [u8]>> {
    match value {
        V::Bytes(bytes) => Ok(Cow::Borrowed(bytes)),
        V::Number(value) => Ok(Cow::Owned(number_text(*value, budget, output)?)),
        _ => Err(ParserError::SourceError(
            "concatenation of a non-string value".into(),
        )),
    }
}
pub(super) fn concat(
    left: &V,
    right: &V,
    budget: &mut MatchBudget,
    output: &mut OutputBudget,
) -> ParserResult<V> {
    let left = operand(left, budget, output)?;
    let right = operand(right, budget, output)?;
    let len = left
        .len()
        .checked_add(right.len())
        .ok_or(ParserError::ResourceBound("factory string bytes"))?;
    budget.charge(len as u64)?;
    output.charge(len)?;
    let mut result = Vec::new();
    result
        .try_reserve_exact(len)
        .map_err(|_| ParserError::ResourceBound("factory string allocation"))?;
    result.extend_from_slice(&left);
    result.extend_from_slice(&right);
    Ok(V::Bytes(result))
}
fn append(
    result: &mut Vec<u8>,
    bytes: &[u8],
    uppercase: bool,
    budget: &mut MatchBudget,
    output: &mut OutputBudget,
) -> ParserResult<()> {
    if bytes.is_empty() {
        return Ok(());
    }
    budget.charge(bytes.len() as u64)?;
    output.charge(bytes.len())?;
    // Amortized growth keeps per-byte replacement patterns linear in output size.
    result
        .try_reserve(bytes.len())
        .map_err(|_| ParserError::ResourceBound("factory string allocation"))?;
    if uppercase {
        result.extend(bytes.iter().map(u8::to_ascii_uppercase));
    } else {
        result.extend_from_slice(bytes);
    }
    Ok(())
}
pub(super) fn first_to_upper(
    value: &V,
    pattern: &LuaPattern,
    budget: &mut MatchBudget,
    output: &mut OutputBudget,
) -> ParserResult<V> {
    let line = match value {
        V::Bytes(line) => line,
        V::Table(table) => {
            return Err(match table.field("gsub") {
                V::Callback(id) => ParserError::Deferred {
                    stage: "firstToUpper receiver method",
                    callback: Some(*id),
                },
                _ => ParserError::SourceError("attempt to call a non-function gsub method".into()),
            });
        }
        _ => {
            return Err(ParserError::SourceError(
                "attempt to index a non-string firstToUpper receiver".into(),
            ));
        }
    };
    output.charge(0)?;
    let mut result = Vec::new();
    let mut copied = 0usize;
    loop {
        let init = copied
            .checked_add(1)
            .and_then(|n| i32::try_from(n).ok())
            .ok_or(ParserError::ResourceBound("factory string index"))?;
        let Some(found) = pattern.match_captures(line, init, budget)? else {
            break;
        };
        let range = found.range();
        append(
            &mut result,
            &line[copied..range.start],
            false,
            budget,
            output,
        )?;
        budget.charge(1)?;
        // string.upper consumes the first capture; without explicit captures the
        // pattern engine supplies the whole match. Position captures are numbers.
        match found.captures().first() {
            Some(Capture::Bytes { start, end }) => {
                append(&mut result, &line[*start..*end], true, budget, output)?
            }
            Some(Capture::Position(position)) => {
                let text = number_text(*position as f64, budget, output)?;
                append(&mut result, &text, true, budget, output)?;
            }
            None => return Err(ParserError::InvalidData("gsub replacement capture".into())),
        }
        copied = range.end;
        // Lua 5.1 consumes one unchanged byte after an empty replacement match.
        if range.is_empty() {
            if copied == line.len() {
                break;
            }
            append(
                &mut result,
                &line[copied..copied + 1],
                false,
                budget,
                output,
            )?;
            copied += 1;
        }
        if pattern.source().first() == Some(&b'^') {
            break;
        }
    }
    append(&mut result, &line[copied..], false, budget, output)?;
    Ok(V::Bytes(result))
}

#[cfg(test)]
mod tests {
    use super::super::{ModifierTable, value::MAX_OUTPUT_BYTES};
    use super::*;
    use crate::lua_pattern::MatchLimits;
    use poe_optimizer_data::modifier_parser::ParserCallbackId;
    use std::sync::Arc;

    fn upper(pattern: &str, bytes: &[u8]) -> ParserResult<V> {
        first_to_upper(
            &V::Bytes(bytes.to_vec()),
            &LuaPattern::compile(pattern.as_bytes()).unwrap(),
            &mut MatchBudget::default(),
            &mut OutputBudget::default(),
        )
    }
    #[test]
    fn helper_preserves_all_bytes_and_uses_first_capture() {
        let bytes: Vec<u8> = (0..=255).collect();
        assert_eq!(
            upper("(.)", &bytes).unwrap(),
            V::Bytes(bytes.iter().map(u8::to_ascii_uppercase).collect())
        );
        assert_eq!(upper("()(.)", b"ab").unwrap(), V::Bytes(b"12".to_vec()));
        assert_eq!(upper("(.)()", b"ab").unwrap(), V::Bytes(b"AB".to_vec()));
        assert_eq!(upper("()", b"ab").unwrap(), V::Bytes(b"1a2b3".to_vec()));
        assert_eq!(upper("", b"ab").unwrap(), V::Bytes(b"ab".to_vec()));
        assert_eq!(upper("^%l", b"a\0z").unwrap(), V::Bytes(b"A\0z".to_vec()));
        assert_eq!(upper("a*", b"ab").unwrap(), V::Bytes(b"Ab".to_vec()));
    }
    #[test]
    fn malformed_pattern_is_lazy_and_receiver_lookup_precedes_matching() {
        assert_eq!(upper("^z[", b"ab").unwrap(), V::Bytes(b"ab".to_vec()));
        assert!(matches!(upper("[", b"ab"), Err(ParserError::Scan(_))));
        let pattern = LuaPattern::compile(b"[").unwrap();
        for value in [
            V::Nil,
            V::Number(3.0),
            V::Boolean(false),
            V::Callback(ParserCallbackId(1)),
        ] {
            assert!(matches!(
                first_to_upper(
                    &value,
                    &pattern,
                    &mut MatchBudget::default(),
                    &mut OutputBudget::default()
                ),
                Err(ParserError::SourceError(_))
            ));
        }
    }
    #[test]
    fn opaque_receiver_method_retains_actual_callback_identity() {
        let mut table = ModifierTable::default();
        let pattern = LuaPattern::compile(b"[").unwrap();
        table
            .fields
            .insert("gsub".into(), V::Callback(ParserCallbackId(42)));
        assert_eq!(
            first_to_upper(
                &V::Table(Arc::new(table.clone())),
                &pattern,
                &mut MatchBudget::default(),
                &mut OutputBudget::default()
            ),
            Err(ParserError::Deferred {
                stage: "firstToUpper receiver method",
                callback: Some(ParserCallbackId(42))
            })
        );
        for method in [
            V::Nil,
            V::Boolean(false),
            V::Number(1.0),
            V::Bytes(b"gsub".to_vec()),
            V::Table(Arc::new(ModifierTable::default())),
        ] {
            table.fields.insert("gsub".into(), method);
            assert!(matches!(
                first_to_upper(
                    &V::Table(Arc::new(table.clone())),
                    &pattern,
                    &mut MatchBudget::default(),
                    &mut OutputBudget::default()
                ),
                Err(ParserError::SourceError(_))
            ));
        }
    }
    #[test]
    fn concatenation_keeps_number_rendering_and_raw_bytes() {
        for (number, text) in [
            (-0.0, "-0"),
            (f64::INFINITY, "inf"),
            (f64::NEG_INFINITY, "-inf"),
            (f64::NAN, "nan"),
            (1e14, "1e+14"),
            (1e-5, "1e-05"),
            (123.5, "123.5"),
        ] {
            let actual = concat(
                &V::Bytes(vec![0xff, 0]),
                &V::Number(number),
                &mut MatchBudget::default(),
                &mut OutputBudget::default(),
            )
            .unwrap();
            let mut expected = vec![0xff, 0];
            expected.extend_from_slice(text.as_bytes());
            assert_eq!(actual, V::Bytes(expected));
        }
        assert!(matches!(
            concat(
                &V::Boolean(false),
                &V::Bytes(Vec::new()),
                &mut MatchBudget::default(),
                &mut OutputBudget::default()
            ),
            Err(ParserError::SourceError(_))
        ));
    }
    #[test]
    fn cumulative_output_bound_rejects_before_concatenation_allocation() {
        let value = V::Bytes(vec![b'x'; MAX_OUTPUT_BYTES / 2 + 1]);
        assert!(matches!(
            concat(
                &value,
                &value,
                &mut MatchBudget::new(MatchLimits {
                    max_steps: u64::MAX,
                    ..MatchLimits::default()
                }),
                &mut OutputBudget::default()
            ),
            Err(ParserError::ResourceBound(_))
        ));
        let mut output = OutputBudget::default();
        output.charge(MAX_OUTPUT_BYTES - 1).unwrap();
        assert!(matches!(
            concat(
                &V::Number(7.0),
                &V::Bytes(Vec::new()),
                &mut MatchBudget::default(),
                &mut output
            ),
            Err(ParserError::ResourceBound(_))
        ));
    }
    #[test]
    fn string_work_uses_the_shared_request_budget() {
        let mut budget = MatchBudget::new(MatchLimits {
            max_steps: 4,
            ..MatchLimits::default()
        });
        assert!(
            concat(
                &V::Bytes(b"ab".to_vec()),
                &V::Bytes(b"cd".to_vec()),
                &mut budget,
                &mut OutputBudget::default()
            )
            .is_ok()
        );
        assert!(matches!(
            upper_with_budget(&mut budget),
            Err(ParserError::Scan(_))
        ));
        fn upper_with_budget(budget: &mut MatchBudget) -> ParserResult<V> {
            first_to_upper(
                &V::Bytes(b"a".to_vec()),
                &LuaPattern::compile(b"(.)").unwrap(),
                budget,
                &mut OutputBudget::default(),
            )
        }
    }
    #[test]
    fn concurrent_patterns_do_not_share_policy_or_request_state() {
        let first = Arc::new(LuaPattern::compile(b"(.)").unwrap());
        let second = Arc::new(LuaPattern::compile(b"()(.)").unwrap());
        std::thread::scope(|scope| {
            for (pattern, expected) in [(first, b"AB"), (second, b"12")] {
                scope.spawn(move || {
                    for _ in 0..32 {
                        let actual = first_to_upper(
                            &V::Bytes(b"ab".to_vec()),
                            &pattern,
                            &mut MatchBudget::default(),
                            &mut OutputBudget::default(),
                        )
                        .unwrap();
                        assert_eq!(actual, V::Bytes(expected.to_vec()));
                    }
                });
            }
        });
    }
}
