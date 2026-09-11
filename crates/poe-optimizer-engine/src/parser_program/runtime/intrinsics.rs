//! Original primitive calls over invocation-local values; no graph conversion.
use super::value::{Heap, V};
use super::{ProgramLimits, ProgramRuntimeError as Error, RuntimeResult};
use crate::item_tools::lua_number_text;
use crate::lua_number::parse_number;
use crate::lua_pattern::{Capture, CompileLimits, GsubLimits, LuaPattern, MatchBudget};
use poe_optimizer_data::modifier_parser::{ParserCallbackId, ParserProgramIntrinsic};
use std::sync::Arc;

/// The value found by method lookup, retained across argument evaluation.
#[derive(Debug, Clone, Copy)]
pub(super) enum MethodTarget {
    StringPrimitive,
    NonCallable,
    OpaqueCallback(ParserCallbackId),
    OpaqueClosure,
}

/// Resolve the method before argument expressions, but do not attempt to call
/// the resolved value yet. Only indexing failures happen at this stage.
pub(super) fn precheck_method(
    operation: ParserProgramIntrinsic,
    receiver: &V,
    heap: &mut Heap,
) -> RuntimeResult<MethodTarget> {
    let key: &[u8] = match operation {
        ParserProgramIntrinsic::StringGsub => b"gsub",
        ParserProgramIntrinsic::StringGmatch => b"gmatch",
        _ => return Err(Error::unsupported("intrinsic has no string method form")),
    };
    match receiver {
        V::Bytes(_) => Ok(MethodTarget::StringPrimitive),
        V::Table(_) => {
            let key = heap.bytes(key)?;
            Ok(match heap.get(receiver, &key)? {
                V::Callback(callback) => MethodTarget::OpaqueCallback(callback),
                V::Closure(_) => MethodTarget::OpaqueClosure,
                // Every modeled table is plain: imported definition graphs
                // reject metatables, and argument/owned graphs cannot add one.
                _ => MethodTarget::NonCallable,
            })
        }
        _ => Err(Error::source(
            "attempt to index a non-table method receiver",
        )),
    }
}

/// Called only after every argument expression/result-pack expansion finishes.
/// Reusing this token preserves the original target if arguments mutate a table.
pub(super) fn finish_method(target: MethodTarget) -> RuntimeResult<()> {
    match target {
        MethodTarget::StringPrimitive => Ok(()),
        MethodTarget::NonCallable => Err(Error::source(
            "attempt to call a non-function receiver method",
        )),
        MethodTarget::OpaqueClosure => Err(Error::unsupported(
            "live receiver method requires dynamic dispatch",
        )),
        MethodTarget::OpaqueCallback(callback) => Err(Error::unsupported(format!(
            "opaque receiver method callback {callback:?}"
        ))),
    }
}

/// The receiver, if any, has already been checked and prepended to arguments.
/// Extra argument and raw result cardinalities remain source-visible.
pub(super) fn call(
    operation: ParserProgramIntrinsic,
    arguments: &[V],
    heap: &mut Heap,
    patterns: &mut MatchBudget,
    limits: &ProgramLimits,
) -> RuntimeResult<Vec<V>> {
    match operation {
        ParserProgramIntrinsic::ToNumber => tonumber(arguments, heap, patterns, limits),
        ParserProgramIntrinsic::ToString => tostring(arguments, heap, limits),
        ParserProgramIntrinsic::MathMin | ParserProgramIntrinsic::MathMax => {
            minmax(operation, arguments, heap, patterns, limits)
        }
        ParserProgramIntrinsic::StringMatch => string_match(arguments, heap, patterns, limits),
        ParserProgramIntrinsic::Type => {
            let value = arguments
                .first()
                .ok_or_else(|| Error::source("type requires a value"))?;
            let name: &[u8] = match value {
                V::Nil => b"nil",
                V::Boolean(_) => b"boolean",
                V::Number(_) => b"number",
                V::Bytes(_) => b"string",
                V::Table(_) => b"table",
                V::Callback(_) | V::Closure(_) => b"function",
            };
            result_space(1, heap, limits)?;
            Ok(vec![heap.bytes(name)?])
        }
        ParserProgramIntrinsic::Select => select(arguments, heap, patterns, limits),
        ParserProgramIntrinsic::StringGsub => gsub(arguments, heap, patterns, limits),
        ParserProgramIntrinsic::CreateMod => {
            let value = create_mod(arguments, heap)?;
            result_space(1, heap, limits)?;
            Ok(vec![value])
        }
        ParserProgramIntrinsic::TableInsert => {
            table_insert(arguments, heap, patterns)?;
            Ok(Vec::new())
        }
        ParserProgramIntrinsic::StringGmatch => {
            // Check arguments in the same order as creation, but do not invent a
            // callback ID for a closure whose lifetime/identity is not modeled.
            let _ = string_argument(arguments.first(), heap)?;
            let _ = string_argument(arguments.get(1), heap)?;
            Err(Error::unsupported("escaped string.gmatch iterator"))
        }
        ParserProgramIntrinsic::Ipairs => {
            check_table(arguments.first())?;
            Err(Error::unsupported("escaped ipairs iterator"))
        }
    }
}

fn tostring(arguments: &[V], heap: &mut Heap, limits: &ProgramLimits) -> RuntimeResult<Vec<V>> {
    let value = arguments
        .first()
        .ok_or_else(|| Error::source("tostring requires a value"))?;
    result_space(1, heap, limits)?;
    let value = match value {
        V::Nil => heap.bytes(b"nil")?,
        V::Boolean(false) => heap.bytes(b"false")?,
        V::Boolean(true) => heap.bytes(b"true")?,
        V::Number(_) => V::Bytes(string_argument(Some(value), heap)?),
        V::Bytes(_) => value.clone(),
        V::Table(_) | V::Callback(_) | V::Closure(_) => {
            return Err(Error::unsupported(
                "identity-bearing tostring requires original metamethod/address semantics",
            ));
        }
    };
    Ok(vec![value])
}
fn number_argument(value: Option<&V>, patterns: &mut MatchBudget) -> RuntimeResult<f64> {
    match value {
        Some(V::Number(value)) => Ok(*value),
        Some(V::Bytes(value)) => {
            patterns.charge(value.len() as u64)?;
            parse_number(value).ok_or_else(|| Error::source("number argument expected"))
        }
        _ => Err(Error::source("number argument expected")),
    }
}
fn minmax(
    operation: ParserProgramIntrinsic,
    arguments: &[V],
    heap: &mut Heap,
    patterns: &mut MatchBudget,
    limits: &ProgramLimits,
) -> RuntimeResult<Vec<V>> {
    let mut result = number_argument(arguments.first(), patterns)?;
    for value in &arguments[1..] {
        let next = number_argument(Some(value), patterns)?;
        // The source x64 LuaJIT MINSD/MAXSD path selects the second operand for
        // unordered/equal doubles. Rust f64::min/max would change NaN/zero bits.
        result = if operation == ParserProgramIntrinsic::MathMin {
            if result < next { result } else { next }
        } else if result > next {
            result
        } else {
            next
        };
    }
    result_space(1, heap, limits)?;
    Ok(vec![V::Number(result)])
}
fn string_match(
    arguments: &[V],
    heap: &mut Heap,
    patterns: &mut MatchBudget,
    limits: &ProgramLimits,
) -> RuntimeResult<Vec<V>> {
    // Source validates both strings and optional init before interpreting any
    // pattern instruction. Extra values have no role after argument effects.
    let subject = string_argument(arguments.first(), heap)?;
    let pattern = string_argument(arguments.get(1), heap)?;
    let init = optional_integer(arguments.get(2), patterns)?.unwrap_or(1);
    let pattern = compile(&pattern, heap)?;
    let Some(found) = pattern.match_captures(&subject, init, patterns)? else {
        result_space(1, heap, limits)?;
        return Ok(vec![V::Nil]);
    };
    result_space(found.captures().len(), heap, limits)?;
    let mut values = Vec::with_capacity(found.captures().len());
    for capture in found.captures() {
        values.push(match capture {
            Capture::Bytes { start, end } => heap.bytes(&subject[*start..*end])?,
            Capture::Position(position) => V::Number(*position as f64),
        });
    }
    Ok(values)
}

fn select(
    arguments: &[V],
    heap: &mut Heap,
    patterns: &mut MatchBudget,
    limits: &ProgramLimits,
) -> RuntimeResult<Vec<V>> {
    // LuaJIT recognizes any string beginning with '#', before numeric coercion.
    if matches!(arguments.first(), Some(V::Bytes(value)) if value.first() == Some(&b'#')) {
        result_space(1, heap, limits)?;
        return Ok(vec![V::Number(arguments.len().saturating_sub(1) as f64)]);
    }
    let index = optional_integer(arguments.first(), patterns)?
        .ok_or_else(|| Error::source("select requires an index"))?;
    let count = i64::try_from(arguments.len()).map_err(|_| Error::resource("select arguments"))?;
    let index = i64::from(index);
    let index = if index < 0 {
        count + index
    } else {
        index.min(count)
    };
    if index < 1 {
        return Err(Error::source("select index out of range"));
    }
    let tail = &arguments[index as usize..];
    result_space(tail.len(), heap, limits)?;
    Ok(tail.to_vec())
}

fn result_space(count: usize, heap: &mut Heap, limits: &ProgramLimits) -> RuntimeResult<()> {
    if count > limits.max_results {
        return Err(Error::resource("intrinsic result pack"));
    }
    heap.charge_values(count)
}

fn tonumber(
    arguments: &[V],
    heap: &mut Heap,
    patterns: &mut MatchBudget,
    limits: &ProgramLimits,
) -> RuntimeResult<Vec<V>> {
    // LuaJIT checks the optional base before the required first argument.
    let base = optional_integer(arguments.get(1), patterns)?;
    if let Some(base) = base
        && base != 10
    {
        if !matches!(arguments.first(), Some(V::Bytes(_) | V::Number(_))) {
            return Err(Error::source(
                "string argument expected for explicit tonumber base",
            ));
        }
        if !(2..=36).contains(&base) {
            return Err(Error::source("tonumber base out of range"));
        }
        return Err(Error::unsupported("tonumber explicit non-decimal base"));
    }
    let value = arguments
        .first()
        .ok_or_else(|| Error::source("tonumber requires a value"))?;
    let result = match value {
        V::Number(value) => V::Number(*value),
        V::Bytes(value) => {
            patterns.charge(value.len() as u64)?;
            parse_number(value).map_or(V::Nil, V::Number)
        }
        _ => V::Nil,
    };
    result_space(1, heap, limits)?;
    Ok(vec![result])
}

fn string_argument(value: Option<&V>, heap: &mut Heap) -> RuntimeResult<Arc<[u8]>> {
    match value {
        Some(V::Bytes(value)) => Ok(value.clone()),
        Some(V::Number(value)) => {
            // The reused 14-significant-digit formatter has at most three short
            // temporary strings for an f64. Reserve their bounded cost before
            // formatting, then let Heap charge the retained byte value.
            heap.charge_bytes(128)?;
            let text = lua_number_text(*value);
            let V::Bytes(value) = heap.bytes(text.as_bytes())? else {
                unreachable!("Heap::bytes returns bytes")
            };
            Ok(value)
        }
        _ => Err(Error::source("string argument expected")),
    }
}

fn optional_integer(value: Option<&V>, patterns: &mut MatchBudget) -> RuntimeResult<Option<i32>> {
    let number = match value {
        None | Some(V::Nil) => return Ok(None),
        Some(V::Number(value)) => *value,
        Some(V::Bytes(value)) => {
            patterns.charge(value.len() as u64)?;
            parse_number(value).ok_or_else(|| Error::source("number argument expected"))?
        }
        _ => return Err(Error::source("number argument expected")),
    };
    // lj_lib_checkint truncates in range. Its out-of-range/nonfinite conversion
    // depends on the original host architecture; do not invent portable results.
    let truncated = number.trunc();
    if !truncated.is_finite() || truncated < f64::from(i32::MIN) || truncated > f64::from(i32::MAX)
    {
        return Err(Error::unsupported(
            "host-dependent integer argument conversion",
        ));
    }
    Ok(Some(truncated as i32))
}

fn compile(pattern: &[u8], heap: &mut Heap) -> RuntimeResult<LuaPattern> {
    let defaults = CompileLimits::default();
    let pattern = LuaPattern::compile_with_limits(
        pattern,
        CompileLimits {
            max_pattern_bytes: defaults.max_pattern_bytes.min(heap.remaining_bytes()),
            max_compiled_bytes: defaults.max_compiled_bytes.min(heap.remaining_bytes()),
        },
    )?;
    heap.charge_bytes(pattern.compiled_bytes())?;
    Ok(pattern)
}

fn gsub(
    arguments: &[V],
    heap: &mut Heap,
    patterns: &mut MatchBudget,
    limits: &ProgramLimits,
) -> RuntimeResult<Vec<V>> {
    let subject = string_argument(arguments.first(), heap)?;
    let pattern = string_argument(arguments.get(1), heap)?;
    // Original C obtains the replacement type, then checks the optional count
    // before rejecting an invalid replacement. Do not reorder those failures.
    let maximum = optional_integer(arguments.get(3), patterns)?;
    let replacement = match arguments.get(2) {
        Some(V::Table(_) | V::Callback(_) | V::Closure(_)) => {
            return Err(Error::unsupported("dynamic string.gsub replacement"));
        }
        value => string_argument(value, heap)?,
    };
    let pattern = compile(&pattern, heap)?;
    // The primitive owns a Vec before the returned byte value owns an Arc.
    // Bound both simultaneously live allocations by the remaining heap budget.
    let result = pattern.gsub(
        &subject,
        &replacement,
        maximum,
        patterns,
        GsubLimits {
            max_replacement_bytes: limits.max_bytes,
            max_output_bytes: heap.remaining_bytes() / 2,
        },
    )?;
    heap.charge_bytes(result.bytes.capacity())?;
    let output = heap.bytes(&result.bytes)?;
    result_space(2, heap, limits)?;
    Ok(vec![output, V::Number(result.substitutions as f64)])
}

fn check_table(value: Option<&V>) -> RuntimeResult<&V> {
    match value {
        Some(value @ V::Table(_)) => Ok(value),
        _ => Err(Error::source("table argument expected")),
    }
}

fn table_insert(arguments: &[V], heap: &mut Heap, patterns: &mut MatchBudget) -> RuntimeResult<()> {
    let table = check_table(arguments.first())?;
    match arguments.len() {
        2 => heap.append(table, arguments[1].clone()),
        3 => {
            if optional_integer(arguments.get(1), patterns)?.is_none() {
                return Err(Error::source("table.insert position requires a number"));
            }
            Err(Error::unsupported("table.insert positional shift"))
        }
        _ => Err(Error::source("wrong number of arguments to table.insert")),
    }
}

/// Original ModTools.createMod: raw first-three values plus a type-sensitive
/// vararg prefix. The source constructor never clones tag/value table objects.
fn create_mod(arguments: &[V], heap: &mut Heap) -> RuntimeResult<V> {
    let mut source = V::Nil;
    let mut flags = V::Number(0.0);
    let mut keywords = V::Number(0.0);
    let mut tag_start = 3;
    if let Some(value @ V::Bytes(_)) = arguments.get(3) {
        source = value.clone();
        tag_start = 4;
    }
    if let Some(value @ V::Number(_)) = arguments.get(4) {
        flags = value.clone();
        tag_start = 5;
    }
    if let Some(value @ V::Number(_)) = arguments.get(5) {
        keywords = value.clone();
        tag_start = 6;
    }
    let table = heap.new_table()?;
    for (key, value) in [
        (
            b"name".as_slice(),
            arguments.first().cloned().unwrap_or(V::Nil),
        ),
        (
            b"type".as_slice(),
            arguments.get(1).cloned().unwrap_or(V::Nil),
        ),
        (
            b"value".as_slice(),
            arguments.get(2).cloned().unwrap_or(V::Nil),
        ),
        (b"source".as_slice(), source),
        (b"flags".as_slice(), flags),
        (b"keywordFlags".as_slice(), keywords),
    ] {
        let key = heap.bytes(key)?;
        heap.set(&table, key, value)?;
    }
    for (index, value) in arguments.iter().skip(tag_start).enumerate() {
        // Nil consumes an index just as select(...) does in a table constructor.
        heap.set(&table, V::Number((index + 1) as f64), value.clone())?;
    }
    Ok(table)
}

/// A native gmatch closure kept inside one invocation's generic-for state.
#[derive(Debug)]
pub(super) struct Gmatch {
    subject: Arc<[u8]>,
    pattern: LuaPattern,
    cursor: usize,
    exhausted: bool,
    max_results: usize,
}
impl Gmatch {
    pub(super) fn new(
        arguments: &[V],
        heap: &mut Heap,
        limits: &ProgramLimits,
    ) -> RuntimeResult<Self> {
        let subject = string_argument(arguments.first(), heap)?;
        let pattern = string_argument(arguments.get(1), heap)?;
        let compiled = if pattern.first() == Some(&b'^') {
            // gmatch starts match() directly; '^' has no anchor special case.
            // Preserve that literal byte while reusing the compiled matcher.
            let length = pattern
                .len()
                .checked_add(1)
                .ok_or_else(|| Error::resource("gmatch pattern length"))?;
            heap.charge_bytes(length)?;
            let mut escaped = Vec::with_capacity(length);
            escaped.push(b'%');
            escaped.extend_from_slice(&pattern);
            compile(&escaped, heap)?
        } else {
            compile(&pattern, heap)?
        };
        Ok(Self {
            subject,
            pattern: compiled,
            cursor: 0,
            exhausted: false,
            max_results: limits.max_results,
        })
    }

    pub(super) fn next(
        &mut self,
        heap: &mut Heap,
        patterns: &mut MatchBudget,
    ) -> RuntimeResult<Option<Vec<V>>> {
        if self.exhausted || self.cursor > self.subject.len() {
            return Ok(None);
        }
        let init = self
            .cursor
            .checked_add(1)
            .and_then(|n| i32::try_from(n).ok())
            .ok_or_else(|| Error::resource("gmatch position"))?;
        let Some(found) = self.pattern.match_captures(&self.subject, init, patterns)? else {
            self.exhausted = true;
            return Ok(None);
        };
        let range = found.range();
        self.cursor = if range.end == range.start {
            range.end + 1
        } else {
            range.end
        };
        if found.captures().len() > self.max_results {
            return Err(Error::resource("gmatch result pack"));
        }
        heap.charge_values(found.captures().len())?;
        let mut values = Vec::with_capacity(found.captures().len());
        for capture in found.captures() {
            values.push(match capture {
                Capture::Bytes { start, end } => heap.bytes(&self.subject[*start..*end])?,
                Capture::Position(position) => V::Number(*position as f64),
            });
        }
        Ok(Some(values))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser_program::runtime::ProgramRuntimeErrorKind;
    use crate::parser_program::runtime::value::{
        ProgramTable, ProgramTableId, ProgramValue, ProgramValueGraph,
    };
    use poe_optimizer_data::game_data::bundled_snapshot;

    fn bytes(value: &[u8]) -> V {
        V::Bytes(Arc::from(value))
    }
    fn fixture() -> (Heap<'static>, MatchBudget, ProgramLimits) {
        let limits = ProgramLimits::default();
        let (heap, _) = Heap::new(
            bundled_snapshot().unwrap().modifier_parser(),
            &ProgramValueGraph::default(),
            &limits,
        )
        .unwrap();
        (heap, MatchBudget::new(limits.pattern), limits)
    }
    fn field(heap: &mut Heap, table: &V, key: &[u8]) -> V {
        heap.get(table, &bytes(key)).unwrap()
    }

    #[test]
    fn tonumber_preserves_raw_arity_nil_and_numeric_bits() {
        let (mut heap, mut patterns, limits) = fixture();
        assert_eq!(
            call(
                ParserProgramIntrinsic::ToNumber,
                &[],
                &mut heap,
                &mut patterns,
                &limits
            )
            .unwrap_err()
            .kind,
            ProgramRuntimeErrorKind::Source
        );
        for input in [
            V::Nil,
            V::Boolean(false),
            bytes(b"nonnumeric"),
            bytes(b"1\0"),
        ] {
            let result = call(
                ParserProgramIntrinsic::ToNumber,
                &[input],
                &mut heap,
                &mut patterns,
                &limits,
            )
            .unwrap();
            assert_eq!(result.len(), 1);
            assert!(matches!(result[0], V::Nil));
        }
        for (input, expected) in [
            (bytes(b" -0 "), -0.0),
            (bytes(b"0x1.8p2"), 6.0),
            (V::Number(f64::INFINITY), f64::INFINITY),
        ] {
            let result = call(
                ParserProgramIntrinsic::ToNumber,
                &[input],
                &mut heap,
                &mut patterns,
                &limits,
            )
            .unwrap();
            let V::Number(number) = result[0] else {
                panic!("expected one number")
            };
            assert_eq!(number.to_bits(), expected.to_bits());
        }
    }

    #[test]
    fn gsub_retains_count_and_c_coercion_but_does_not_coerce_method_receivers() {
        let (mut heap, mut patterns, limits) = fixture();
        let result = call(
            ParserProgramIntrinsic::StringGsub,
            &[
                V::Number(12.0),
                bytes(b"(%d)"),
                bytes(b"[%1]"),
                V::Number(1.9),
            ],
            &mut heap,
            &mut patterns,
            &limits,
        )
        .unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].as_bytes(), Some(b"[1]2".as_slice()));
        assert!(matches!(result[1], V::Number(1.0)));
        assert_eq!(
            precheck_method(
                ParserProgramIntrinsic::StringGsub,
                &V::Number(12.0),
                &mut heap
            )
            .unwrap_err()
            .kind,
            ProgramRuntimeErrorKind::Source
        );
        precheck_method(ParserProgramIntrinsic::StringGsub, &bytes(b"12"), &mut heap).unwrap();
        let result = call(
            ParserProgramIntrinsic::StringGsub,
            &[bytes(b"a"), bytes(b"["), bytes(b"%9"), V::Number(0.0)],
            &mut heap,
            &mut patterns,
            &limits,
        )
        .unwrap();
        assert_eq!(result[0].as_bytes(), Some(b"a".as_slice()));
        assert!(matches!(result[1], V::Number(0.0)));
    }

    #[test]
    fn method_lookup_retains_target_and_defers_call_errors_until_arguments_finish() {
        let (mut heap, _, _) = fixture();
        let receiver = heap.new_table().unwrap();
        let callback = *bundled_snapshot()
            .unwrap()
            .modifier_parser()
            .data()
            .factories
            .keys()
            .next()
            .unwrap();
        let table_target = heap.new_table().unwrap();
        for value in [
            V::Nil,
            V::Boolean(false),
            V::Number(3.0),
            bytes(b"text"),
            table_target,
        ] {
            heap.set(&receiver, bytes(b"gsub"), value).unwrap();
            let target =
                precheck_method(ParserProgramIntrinsic::StringGsub, &receiver, &mut heap).unwrap();
            // A later argument can mutate the receiver, but not the target
            // that lookup already returned. No call occurs during lookup.
            heap.set(&receiver, bytes(b"gsub"), V::Callback(callback))
                .unwrap();
            assert_eq!(
                finish_method(target).unwrap_err().kind,
                ProgramRuntimeErrorKind::Source
            );
        }
        let target =
            precheck_method(ParserProgramIntrinsic::StringGsub, &receiver, &mut heap).unwrap();
        heap.set(&receiver, bytes(b"gsub"), V::Nil).unwrap();
        assert_eq!(
            finish_method(target).unwrap_err().kind,
            ProgramRuntimeErrorKind::UnsupportedCapability
        );
        finish_method(
            precheck_method(
                ParserProgramIntrinsic::StringGsub,
                &bytes(b"subject"),
                &mut heap,
            )
            .unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn intrinsic_result_packs_respect_result_and_cumulative_value_limits() {
        let limits = ProgramLimits {
            max_values: 1,
            ..ProgramLimits::default()
        };
        let (mut heap, _) = Heap::new(
            bundled_snapshot().unwrap().modifier_parser(),
            &ProgramValueGraph::default(),
            &limits,
        )
        .unwrap();
        let mut patterns = MatchBudget::new(limits.pattern);
        let result = call(
            ParserProgramIntrinsic::ToNumber,
            &[V::Nil],
            &mut heap,
            &mut patterns,
            &limits,
        )
        .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(heap.stats().values, 1);
        assert_eq!(
            call(
                ParserProgramIntrinsic::ToNumber,
                &[V::Nil],
                &mut heap,
                &mut patterns,
                &limits
            )
            .unwrap_err()
            .kind,
            ProgramRuntimeErrorKind::ResourceBound
        );
        assert_eq!(heap.stats().values, 1);
        let (mut heap, mut patterns, mut limits) = fixture();
        limits.max_results = 0;
        assert_eq!(
            call(
                ParserProgramIntrinsic::ToNumber,
                &[V::Nil],
                &mut heap,
                &mut patterns,
                &limits
            )
            .unwrap_err()
            .kind,
            ProgramRuntimeErrorKind::ResourceBound
        );
        // A source argument failure occurs before any return pack exists.
        assert_eq!(
            call(
                ParserProgramIntrinsic::StringGsub,
                &[V::Nil],
                &mut heap,
                &mut patterns,
                &limits
            )
            .unwrap_err()
            .kind,
            ProgramRuntimeErrorKind::Source
        );
    }

    #[test]
    fn optional_count_errors_precede_dynamic_replacement_defer() {
        let (mut heap, mut patterns, limits) = fixture();
        let replacement = heap.new_table().unwrap();
        let mut arguments = vec![bytes(b"a"), bytes(b"a"), replacement, V::Boolean(false)];
        assert_eq!(
            call(
                ParserProgramIntrinsic::StringGsub,
                &arguments,
                &mut heap,
                &mut patterns,
                &limits
            )
            .unwrap_err()
            .kind,
            ProgramRuntimeErrorKind::Source
        );
        arguments[3] = V::Nil;
        assert_eq!(
            call(
                ParserProgramIntrinsic::StringGsub,
                &arguments,
                &mut heap,
                &mut patterns,
                &limits
            )
            .unwrap_err()
            .kind,
            ProgramRuntimeErrorKind::UnsupportedCapability
        );
        arguments[2] = bytes(b"b");
        arguments[3] = V::Number(f64::INFINITY);
        assert_eq!(
            call(
                ParserProgramIntrinsic::StringGsub,
                &arguments,
                &mut heap,
                &mut patterns,
                &limits
            )
            .unwrap_err()
            .kind,
            ProgramRuntimeErrorKind::UnsupportedCapability
        );
    }

    #[test]
    fn create_mod_preserves_alias_identity_nil_holes_and_independent_prefix_types() {
        let (mut heap, mut patterns, limits) = fixture();
        let child = heap.new_table().unwrap();
        let result = call(
            ParserProgramIntrinsic::CreateMod,
            &[
                V::Number(7.0),
                V::Boolean(false),
                child.clone(),
                V::Nil,
                V::Boolean(false),
                child.clone(),
            ],
            &mut heap,
            &mut patterns,
            &limits,
        )
        .unwrap();
        let table = &result[0];
        assert!(field(&mut heap, table, b"name").lua_equal(&V::Number(7.0)));
        assert!(field(&mut heap, table, b"type").lua_equal(&V::Boolean(false)));
        assert!(field(&mut heap, table, b"value").lua_equal(&child));
        assert!(matches!(heap.get(table, &V::Number(1.0)).unwrap(), V::Nil));
        assert!(
            heap.get(table, &V::Number(2.0))
                .unwrap()
                .lua_equal(&V::Boolean(false))
        );
        assert!(heap.get(table, &V::Number(3.0)).unwrap().lua_equal(&child));
        let result = call(
            ParserProgramIntrinsic::CreateMod,
            &[
                V::Nil,
                V::Nil,
                V::Nil,
                child.clone(),
                V::Number(-0.0),
                child,
            ],
            &mut heap,
            &mut patterns,
            &limits,
        )
        .unwrap();
        let V::Number(flags) = field(&mut heap, &result[0], b"flags") else {
            panic!("numeric flags")
        };
        assert_eq!(flags.to_bits(), (-0.0f64).to_bits());
        assert!(matches!(
            heap.get(&result[0], &V::Number(1.0)).unwrap(),
            V::Table(_)
        ));
    }

    #[test]
    fn table_insert_has_no_return_values_and_does_not_write_borrowed_tables() {
        let (mut heap, mut patterns, limits) = fixture();
        let table = heap.new_table().unwrap();
        let returned = call(
            ParserProgramIntrinsic::TableInsert,
            &[table.clone(), bytes(b"x")],
            &mut heap,
            &mut patterns,
            &limits,
        )
        .unwrap();
        assert!(returned.is_empty());
        assert_eq!(
            heap.get(&table, &V::Number(1.0)).unwrap().as_bytes(),
            Some(b"x".as_slice())
        );
        let input = ProgramValueGraph {
            values: vec![ProgramValue::Table(ProgramTableId(1))],
            tables: vec![ProgramTable::default()],
        };
        let (mut heap, input_values) = Heap::new(
            bundled_snapshot().unwrap().modifier_parser(),
            &input,
            &limits,
        )
        .unwrap();
        assert_eq!(
            call(
                ParserProgramIntrinsic::TableInsert,
                &[input_values[0].clone(), bytes(b"x")],
                &mut heap,
                &mut patterns,
                &limits
            )
            .unwrap_err()
            .kind,
            ProgramRuntimeErrorKind::UnsupportedCapability
        );
        assert!(matches!(
            heap.get(&input_values[0], &V::Number(1.0)).unwrap(),
            V::Nil
        ));
    }

    #[test]
    fn iterator_creation_keeps_pattern_errors_lazy_and_capture_packs_bounded() {
        let (mut heap, mut patterns, mut limits) = fixture();
        let mut iterator = Gmatch::new(&[bytes(b"a"), bytes(b"[")], &mut heap, &limits).unwrap();
        assert_eq!(
            iterator.next(&mut heap, &mut patterns).unwrap_err().kind,
            ProgramRuntimeErrorKind::Source
        );
        let mut iterator = Gmatch::new(&[bytes(b"^a"), bytes(b"^()")], &mut heap, &limits).unwrap();
        let values = iterator.next(&mut heap, &mut patterns).unwrap().unwrap();
        assert_eq!(values.len(), 1);
        assert!(matches!(values[0], V::Number(2.0)));
        assert!(iterator.next(&mut heap, &mut patterns).unwrap().is_none());
        limits.max_results = 0;
        let mut iterator = Gmatch::new(&[bytes(b"a"), bytes(b".")], &mut heap, &limits).unwrap();
        assert_eq!(
            iterator.next(&mut heap, &mut patterns).unwrap_err().kind,
            ProgramRuntimeErrorKind::ResourceBound
        );
    }

    #[test]
    fn compiled_patterns_and_replacement_expansion_use_the_shared_byte_budget() {
        let limits = ProgramLimits {
            max_bytes: 1024,
            ..ProgramLimits::default()
        };
        let (mut heap, _) = Heap::new(
            bundled_snapshot().unwrap().modifier_parser(),
            &ProgramValueGraph::default(),
            &limits,
        )
        .unwrap();
        let mut patterns = MatchBudget::new(limits.pattern);
        let replacement = bytes(&[b'x'; 200]);
        let error = call(
            ParserProgramIntrinsic::StringGsub,
            &[bytes(b"aaaaaaaaaaaaaaaa"), bytes(b"a"), replacement],
            &mut heap,
            &mut patterns,
            &limits,
        )
        .unwrap_err();
        assert_eq!(error.kind, ProgramRuntimeErrorKind::ResourceBound);
        assert!(heap.stats().bytes <= limits.max_bytes);
    }
    #[test]
    fn type_and_select_preserve_nil_arity_aliases_and_luajit_index_rules() {
        let (mut heap, mut patterns, limits) = fixture();
        let table = heap.new_table().unwrap();
        let cases = [
            (V::Nil, "nil"),
            (V::Boolean(false), "boolean"),
            (V::Number(f64::NAN), "number"),
            (bytes(b"a"), "string"),
            (table.clone(), "table"),
            (V::Callback(ParserCallbackId(1)), "function"),
        ];
        for (input, expected) in cases {
            let result = call(
                ParserProgramIntrinsic::Type,
                &[input],
                &mut heap,
                &mut patterns,
                &limits,
            )
            .unwrap();
            assert_eq!(result[0].as_bytes(), Some(expected.as_bytes()));
        }
        assert_eq!(
            call(
                ParserProgramIntrinsic::Type,
                &[],
                &mut heap,
                &mut patterns,
                &limits
            )
            .unwrap_err()
            .kind,
            ProgramRuntimeErrorKind::Source
        );
        let result = call(
            ParserProgramIntrinsic::Select,
            &[bytes(b"#anything"), table.clone(), V::Nil],
            &mut heap,
            &mut patterns,
            &limits,
        )
        .unwrap();
        assert!(result[0].lua_equal(&V::Number(2.0)));
        let result = call(
            ParserProgramIntrinsic::Select,
            &[V::Number(-2.9), table.clone(), V::Nil],
            &mut heap,
            &mut patterns,
            &limits,
        )
        .unwrap();
        assert_eq!(result.len(), 2);
        assert!(result[0].lua_equal(&table));
        assert!(matches!(result[1], V::Nil));
        let result = call(
            ParserProgramIntrinsic::Select,
            &[bytes(b"2"), table.clone(), V::Number(-0.0)],
            &mut heap,
            &mut patterns,
            &limits,
        )
        .unwrap();
        assert!(matches!(result[0], V::Number(v) if v.to_bits() == (-0.0f64).to_bits()));
        assert!(
            call(
                ParserProgramIntrinsic::Select,
                &[V::Number(99.0), table],
                &mut heap,
                &mut patterns,
                &limits
            )
            .unwrap()
            .is_empty()
        );
        for index in [V::Number(0.0), V::Number(-3.0), V::Nil, V::Boolean(true)] {
            assert_eq!(
                call(
                    ParserProgramIntrinsic::Select,
                    &[index, V::Nil],
                    &mut heap,
                    &mut patterns,
                    &limits
                )
                .unwrap_err()
                .kind,
                ProgramRuntimeErrorKind::Source
            );
        }
        assert_eq!(
            call(
                ParserProgramIntrinsic::Select,
                &[V::Number(f64::INFINITY)],
                &mut heap,
                &mut patterns,
                &limits
            )
            .unwrap_err()
            .kind,
            ProgramRuntimeErrorKind::UnsupportedCapability
        );
        assert_eq!(
            call(
                ParserProgramIntrinsic::Select,
                &[V::Number(1.0), V::Nil, V::Nil],
                &mut heap,
                &mut patterns,
                &ProgramLimits {
                    max_results: 1,
                    ..limits
                }
            )
            .unwrap_err()
            .kind,
            ProgramRuntimeErrorKind::ResourceBound
        );
    }
}
