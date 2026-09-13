//! Closed byte-string operations for proven factory expressions.
//! No generic method execution, metatables, Unicode casing or shared mutable state.
use super::{ModifierValue as V, ParserError, ParserResult, value::OutputBudget};
use crate::item_tools::lua_number_text;
use crate::lua_pattern::{
    GsubLimits, LuaPattern, MatchBudget,
    uppercase::{self, UppercaseOutput},
};
use poe_optimizer_data::modifier_parser::ParserFactoryReplacement;
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
fn method_subject<'a>(value: &'a V, stage: &'static str) -> ParserResult<&'a [u8]> {
    match value {
        V::Bytes(line) => Ok(line),
        V::Table(table) => Err(match table.field("gsub") {
            V::Callback(id) => ParserError::Deferred {
                stage,
                callback: Some(*id),
            },
            _ => ParserError::SourceError("attempt to call a non-function gsub method".into()),
        }),
        _ => Err(ParserError::SourceError(
            if stage == "firstToUpper receiver method" {
                "attempt to index a non-string firstToUpper receiver"
            } else {
                "attempt to index a non-string gsub receiver"
            }
            .into(),
        )),
    }
}
pub(super) fn first_to_upper(
    value: &V,
    pattern: &LuaPattern,
    budget: &mut MatchBudget,
    output: &mut OutputBudget,
) -> ParserResult<V> {
    let line = method_subject(value, "firstToUpper receiver method")?;
    replace_upper(line, pattern, budget, output)
}
/// Only the first return is represented. The source lowerer proves scalar use.
pub(super) fn gsub(
    value: &V,
    pattern: &LuaPattern,
    replacement: &ParserFactoryReplacement,
    budget: &mut MatchBudget,
    output: &mut OutputBudget,
) -> ParserResult<V> {
    // Colon lookup occurs before pattern matching and replacement processing.
    let line = method_subject(value, "factory gsub receiver method")?;
    match replacement {
        ParserFactoryReplacement::StringUpper => replace_upper(line, pattern, budget, output),
        ParserFactoryReplacement::Text(replacement) => {
            output.charge(0)?;
            let result = pattern.gsub(
                line,
                replacement.as_bytes(),
                None,
                budget,
                GsubLimits {
                    max_replacement_bytes: 4096,
                    max_output_bytes: output.remaining_bytes(),
                },
            )?;
            output.charge(result.bytes.len())?;
            Ok(V::Bytes(result.bytes))
        }
    }
}
fn replace_upper(
    line: &[u8],
    pattern: &LuaPattern,
    budget: &mut MatchBudget,
    output: &mut OutputBudget,
) -> ParserResult<V> {
    struct Buffer<'a> {
        bytes: Vec<u8>,
        output: &'a mut OutputBudget,
    }
    impl UppercaseOutput for Buffer<'_> {
        type Error = ParserError;
        fn append(&mut self, bytes: &[u8], uppercase: bool) -> ParserResult<()> {
            self.output.charge(bytes.len())?;
            // Retain this facade's existing logical output charging and growth.
            self.bytes
                .try_reserve(bytes.len())
                .map_err(|_| ParserError::ResourceBound("factory string allocation"))?;
            if uppercase {
                self.bytes.extend(bytes.iter().map(u8::to_ascii_uppercase));
            } else {
                self.bytes.extend_from_slice(bytes);
            }
            Ok(())
        }
        fn scratch(&mut self, bytes: usize) -> ParserResult<()> {
            self.output.charge(bytes)
        }
        fn resource(message: &'static str) -> ParserError {
            ParserError::ResourceBound(message)
        }
        fn invalid(message: &'static str) -> ParserError {
            ParserError::InvalidData(message.into())
        }
    }
    output.charge(0)?;
    let mut buffer = Buffer {
        bytes: Vec::new(),
        output,
    };
    uppercase::replace_upper(line, pattern, budget, &mut buffer)?;
    Ok(V::Bytes(buffer.bytes))
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
    #[test]
    fn gsub_replacements_preserve_lua_captures_and_empty_matches() {
        for (pattern, replacement, input, expected) in [
            ("(.)", "%1%0", b"a\0".as_slice(), b"aa\0\0".as_slice()),
            ("()(.)", "%2%1", b"ab".as_slice(), b"a1b2".as_slice()),
            ("", "_", b"ab".as_slice(), b"_a_b_".as_slice()),
            (" ", "", b"a  b\tc".as_slice(), b"ab\tc".as_slice()),
        ] {
            let actual = gsub(
                &V::Bytes(input.to_vec()),
                &LuaPattern::compile(pattern.as_bytes()).unwrap(),
                &ParserFactoryReplacement::Text(replacement.into()),
                &mut MatchBudget::default(),
                &mut OutputBudget::default(),
            )
            .unwrap();
            assert_eq!(actual, V::Bytes(expected.to_vec()), "{pattern}");
        }
        let actual = gsub(
            &V::Bytes(b"a b\tc".to_vec()),
            &LuaPattern::compile(b" %l").unwrap(),
            &ParserFactoryReplacement::StringUpper,
            &mut MatchBudget::default(),
            &mut OutputBudget::default(),
        )
        .unwrap();
        assert_eq!(actual, V::Bytes(b"a B\tc".to_vec()));
    }
    #[test]
    fn gsub_method_errors_precede_lazy_pattern_and_replacement_errors() {
        let pattern = LuaPattern::compile(b"[").unwrap();
        for value in [
            V::Nil,
            V::Boolean(false),
            V::Number(3.0),
            V::Callback(ParserCallbackId(7)),
        ] {
            assert!(matches!(
                gsub(
                    &value,
                    &pattern,
                    &ParserFactoryReplacement::Text("%2".into()),
                    &mut MatchBudget::default(),
                    &mut OutputBudget::default()
                ),
                Err(ParserError::SourceError(_))
            ));
        }
        let mut table = ModifierTable::default();
        table
            .fields
            .insert("gsub".into(), V::Callback(ParserCallbackId(7)));
        assert!(matches!(
            gsub(
                &V::Table(Arc::new(table)),
                &pattern,
                &ParserFactoryReplacement::StringUpper,
                &mut MatchBudget::default(),
                &mut OutputBudget::default()
            ),
            Err(ParserError::Deferred {
                stage: "factory gsub receiver method",
                callback: Some(ParserCallbackId(7))
            })
        ));
        for (pattern, replacement, input, succeeds) in [
            ("^z[", "%2", "abc", true),
            ("[", "", "abc", false),
            ("z", "%2", "abc", true),
            ("a", "%2", "abc", false),
        ] {
            let result = gsub(
                &V::Bytes(input.as_bytes().to_vec()),
                &LuaPattern::compile(pattern.as_bytes()).unwrap(),
                &ParserFactoryReplacement::Text(replacement.into()),
                &mut MatchBudget::default(),
                &mut OutputBudget::default(),
            );
            assert_eq!(result.is_ok(), succeeds, "{pattern}/{replacement}");
        }
    }
    #[test]
    fn gsub_output_allocation_and_work_share_cumulative_request_limits() {
        let mut output = OutputBudget::default();
        output.charge(MAX_OUTPUT_BYTES - 2).unwrap();
        let pattern = LuaPattern::compile(b".").unwrap();
        assert!(matches!(
            gsub(
                &V::Bytes(b"a".to_vec()),
                &pattern,
                &ParserFactoryReplacement::Text("xxx".into()),
                &mut MatchBudget::default(),
                &mut output
            ),
            Err(ParserError::Scan(_))
        ));
        let mut budget = MatchBudget::new(MatchLimits {
            max_steps: 2,
            ..Default::default()
        });
        assert!(matches!(
            gsub(
                &V::Bytes(b"abc".to_vec()),
                &pattern,
                &ParserFactoryReplacement::StringUpper,
                &mut budget,
                &mut OutputBudget::default()
            ),
            Err(ParserError::Scan(_))
        ));
    }
}
