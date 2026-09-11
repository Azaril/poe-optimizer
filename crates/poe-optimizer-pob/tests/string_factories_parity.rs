//! Independent full-original-parser proof of closed byte-string factory operations.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/callback_factories_source.rs"]
mod factory_source;
#[allow(dead_code)]
#[path = "support/mod_parser_native_observer.rs"]
mod native_observer;
#[allow(dead_code)]
#[path = "support/ordinary_factories_source.rs"]
mod ordinary_source;
#[allow(dead_code)]
#[path = "support/mod_parser_public_source.rs"]
mod public_source;
#[allow(dead_code)]
#[path = "support/item_loading_runtime.rs"]
mod runtime;
#[allow(dead_code)]
#[path = "support/string_factories_source.rs"]
mod string_source;
use mlua::{Function, MultiValue, Table, Value};
use poe_optimizer_data::{
    game_data::bundled_snapshot,
    modifier_parser::{
        ModifierParserCatalog, ModifierParserData, ParserCallbackId, ParserDictionary as D,
        ParserFactoryDisposition as F, ParserFactoryExpr as E, ParserFactoryField as Field,
        ParserFactoryLiteral as L, ParserTable, ParserTableId, ParserValue as P,
    },
};
use poe_optimizer_engine::{
    lua_pattern::{MatchBudget, PatternError},
    modifier_parser::{CompiledModifierParser, ParserError},
    modifier_scan::ScanError,
};
use public_source::Observation;
use std::collections::{BTreeMap, BTreeSet};
use string_source::StringSource;
fn has_string(expr: &E) -> bool {
    match expr {
        E::Concat { .. } | E::FirstToUpper { .. } => true,
        E::Negate(v) | E::ToNumber { value: v } => has_string(v),
        E::Table(fields) => fields.iter().any(|f| match f {
            Field::Named { value, .. } | Field::List(value) => has_string(value),
        }),
        E::CreateMod { args } | E::Flag { args, .. } => args.iter().any(has_string),
        _ => false,
    }
}
fn candidates(data: &ModifierParserData, family: D) -> Vec<(String, ParserCallbackId)> {
    data.tables[data.dictionaries[&family].0 as usize - 1]
        .fields
        .iter()
        .filter_map(|(key, v)| {
            let P::Callback(id) = v else { return None };
            matches!(data.factories.get(id),Some(F::Pure(f)) if has_string(&f.body))
                .then_some((key.clone(), *id))
        })
        .collect()
}
fn row(data: &mut ModifierParserData, family: D, key: &str, value: P) {
    data.tables[data.dictionaries[&family].0 as usize - 1]
        .fields
        .insert(key.into(), value);
}
fn empty(data: &mut ModifierParserData) -> P {
    let id = ParserTableId(data.tables.len() as u32 + 1);
    data.tables.push(ParserTable::default());
    P::Table(id)
}
fn compile(mut data: ModifierParserData) -> CompiledModifierParser {
    // These legacy-factory probes author their own dictionaries/recipes.
    data.programs = Default::default();
    CompiledModifierParser::new(&ModifierParserCatalog::new(data).unwrap()).unwrap()
}
fn compare(source: &StringSource, native: &CompiledModifierParser, text: &[u8]) -> bool {
    let expected = source.observe(text);
    let actual = native.parse(text, &mut MatchBudget::default());
    match (expected, actual) {
        (Observation::Returned(expected), Ok(actual)) => {
            assert_eq!(
                expected,
                native_observer::capture(&actual, native.catalog()).unwrap(),
                "{text:?}"
            );
            true
        }
        (
            Observation::SourceError(_),
            Err(
                ParserError::SourceError(_)
                | ParserError::Scan(ScanError::Pattern(PatternError::Source(_))),
            ),
        ) => false,
        (expected, actual) => panic!(
            "string source/native mismatch {text:?}: expected={expected:?}; actual={actual:?}"
        ),
    }
}
fn text(s: &str) -> E {
    E::Literal(L::Text(s.into()))
}
fn upper(data: &ModifierParserData, value: E) -> E {
    E::FirstToUpper {
        helper: data.helpers["firstToUpper"],
        value: Box::new(value),
    }
}
fn concat(left: E, right: E) -> E {
    E::Concat {
        left: Box::new(left),
        right: Box::new(right),
    }
}
fn output(value: E) -> E {
    E::Table(vec![Field::Named {
        key: "output".into(),
        value,
    }])
}
fn recipe(data: &mut ModifierParserData, id: ParserCallbackId, value: E) {
    let F::Pure(factory) = data.factories.get_mut(&id).unwrap() else {
        panic!("Pure fixture")
    };
    factory.parameter_count = 6;
    factory.provenance.constructor = None;
    factory.body = value;
}
fn injected(
    source: &StringSource,
    mut data: ModifierParserData,
    expr: E,
    body: &str,
) -> CompiledModifierParser {
    let id = candidates(&data, D::Special)[0].1;
    recipe(&mut data, id, output(expr));
    source.source.clear_specials();
    data.tables[data.dictionaries[&D::Special].0 as usize - 1] = ParserTable::default();
    let callback = source.fixture("caller", &format!("return {{output={body}}}"));
    for pattern in ["^__string (.*)$", "^__missing$", "^__position ()$"] {
        row(&mut data, D::Special, pattern, P::Callback(id));
        source.source.add(pattern, callback.clone());
    }
    compile(data)
}
#[test]
fn every_original_string_factory_matches_at_all_actual_public_call_positions() {
    let snapshot = bundled_snapshot().unwrap();
    let catalog = snapshot.modifier_parser();
    let source = StringSource::new(None);
    let control = StringSource::new(None);
    let mut data = catalog.data().clone();
    let marker = empty(&mut data);
    row(&mut data, D::ModTag, "__string_first__", marker);
    for src in [&source, &control] {
        src.source
            .dictionary("modTagList")
            .set(
                "__string_first__",
                src.source.public.source.lua.create_table().unwrap(),
            )
            .unwrap();
    }
    let native = compile(data);
    let generate: Function = source
        .source
        .public
        .source
        .lua
        .load(include_str!("support/lua_pattern_witness.lua").replace("{55,97,65", "{97,55,65"))
        .eval()
        .unwrap();
    let mut coverage = BTreeMap::<(D, bool), BTreeSet<ParserCallbackId>>::new();
    let mut paired = 0;
    for family in [D::Special, D::ModTag] {
        for (pattern, id) in candidates(catalog.data(), family) {
            let table = source.source.dictionary(family.source_name());
            let original = table.get::<Function>(pattern.as_str()).unwrap();
            let label = format!("{}:{}", family.source_name(), id.0);
            table
                .set(pattern.as_str(), source.wrap(original, &label))
                .unwrap();
            let witness: mlua::LuaString = generate.call(pattern.as_str()).unwrap();
            let witness = witness.as_bytes();
            for second in [false, true] {
                if second && family == D::Special {
                    continue;
                }
                let input = if family == D::Special {
                    witness.to_vec()
                } else if second {
                    [b"+7 to maximum Life __string_first__ ", witness.as_ref()].concat()
                } else {
                    [b"+7 to maximum Life ", witness.as_ref()].concat()
                };
                assert!(
                    compare(&source, &native, &input),
                    "actual source callback{id:?}"
                );
                let frames = if family == D::Special {
                    vec![7406, 7408]
                } else if second {
                    vec![6684, 6686]
                } else {
                    vec![6675, 6677]
                };
                assert!(
                    source.calls().iter().any(|call| call.label == label
                        && call.frames.iter().any(|line| frames.contains(line))),
                    "callback{label} second{second}: {:?}",
                    source.calls()
                );
                assert_eq!(
                    source.source.public.observe(&input),
                    control.observe(&input),
                    "observer changed original"
                );
                coverage.entry((family, second)).or_default().insert(id);
                paired += 1;
            }
        }
    }
    assert_eq!(coverage[&(D::Special, false)].len(), 44);
    assert_eq!(coverage[&(D::ModTag, false)].len(), 10);
    assert_eq!(coverage[&(D::ModTag, true)].len(), 10);
    assert_eq!(paired, 64);
    eprintln!(
        "String factories: all54 original bodies,64 actual full public graphs (44Special,10firstTag,10secondTag)"
    );
}
#[test]
fn all_original_string_bodies_preserve_raw_capture_aliases_and_ordered_errors() {
    let snapshot = bundled_snapshot().unwrap();
    let original = snapshot.modifier_parser().data();
    let source = StringSource::new(None);
    let mut data = original.clone();
    let mut cases = vec![];
    for family in [D::Special, D::ModTag] {
        for (key, id) in candidates(original, family) {
            let f = source
                .source
                .dictionary(family.source_name())
                .get::<Function>(key.as_str())
                .unwrap();
            let alias = format!("__string_{} (.*)|(.*)|(.*)|(.*)|(.*)", id.0);
            let pattern = if family == D::Special {
                format!("^{alias}$")
            } else {
                format!(" {alias}$")
            };
            row(&mut data, family, &pattern, P::Callback(id));
            source
                .source
                .dictionary(family.source_name())
                .set(pattern.as_str(), f)
                .unwrap();
            cases.push((family, id));
        }
    }
    let native = compile(data);
    let mut paired = 0;
    let mut errors = 0;
    for (family, id) in cases {
        for captures in [
            b"12|cold|fire|mana|blue".as_slice(),
            b"abc|lightning|life|red|green",
            b"-0|a\0b|\xffz|\x80a|chaos",
            b"1e309|physical|shield|x|y",
            b"||||",
        ] {
            let input = [
                if family == D::Special {
                    b"".as_slice()
                } else {
                    b"+7 to maximum Life "
                },
                format!("__string_{} ", id.0).as_bytes(),
                captures,
            ]
            .concat();
            if compare(&source, &native, &input) {
                paired += 1
            } else {
                errors += 1
            }
        }
    }
    assert_eq!(paired + errors, 270);
    eprintln!(
        "All54 original string closures via labelled aliases:{paired} exact full graphs,{errors} ordered source errors"
    );
}
#[test]
fn configured_original_gsub_patterns_preserve_capture_and_empty_match_semantics() {
    let snapshot = bundled_snapshot().unwrap();
    let mut paired = 0;
    let mut errors = 0;
    for pattern in [
        "^%l", "", "^", "$", "a*", "()", "()(.)", "(.)()", "(%l*)", "(%l-)", "^()", "^^", "%f[%a]",
        "^z[", "[",
    ] {
        let source = StringSource::new(Some(pattern));
        let mut data = snapshot.modifier_parser().data().clone();
        data.policy.first_to_upper_pattern = pattern.into();
        let expr = upper(&data, E::Argument(1));
        let native = injected(&source, data, expr, "firstToUpper(a)");
        for value in [
            b"ab".as_slice(),
            b"",
            b" abc",
            b"12",
            b"\xffa\0b",
            b"aaaa",
            b"z",
        ] {
            if compare(&source, &native, &[b"__string ", value].concat()) {
                paired += 1
            } else {
                errors += 1
            }
        }
        for input in [b"__missing".as_slice(), b"__position "] {
            assert!(!compare(&source, &native, input));
            errors += 1;
        }
        // The guard is a runtime operation; it stays unused when no callback reaches it.
        assert!(compare(&source, &native, b"+7 to maximum Life"));
        paired += 1;
    }
    eprintln!(
        "Configured original-helper runtime patterns:{paired} exact graphs,{errors} source errors; capture,empty,anchored,lazy malformed controls"
    );
}
#[test]
fn concat_number_rendering_and_source_grouping_preserve_complete_outputs_and_failures() {
    let snapshot = bundled_snapshot().unwrap();
    let source = StringSource::new(None);
    let data = snapshot.modifier_parser().data();
    let cases = vec![
        (concat(text("prefix"), E::Argument(0)), "'prefix'..num"),
        (concat(E::Argument(0), text("suffix")), "num..'suffix'"),
        (
            concat(E::Argument(1), concat(text(":"), E::Argument(0))),
            "a..':'..num",
        ),
        (
            concat(E::Negate(Box::new(E::Argument(0))), text("x")),
            "-num..'x'",
        ),
        (
            E::Negate(Box::new(concat(E::Argument(1), text("x")))),
            "-(a..'x')",
        ),
        (
            concat(upper(data, E::Argument(1)), text("Tail")),
            "firstToUpper(a)..'Tail'",
        ),
        (
            concat(
                text("Head"),
                concat(upper(data, E::Argument(1)), text("Tail")),
            ),
            "'Head'..firstToUpper(a)..'Tail'",
        ),
        (
            concat(
                E::Literal(L::Nil),
                concat(text("x"), E::Literal(L::Boolean(false))),
            ),
            "nil..'x'..false",
        ),
        (
            concat(
                concat(E::Literal(L::Nil), text("x")),
                E::Literal(L::Boolean(false)),
            ),
            "(nil..'x')..false",
        ),
        (
            concat(E::Literal(L::Nil), upper(data, E::Literal(L::Nil))),
            "nil..firstToUpper(nil)",
        ),
        (
            concat(
                upper(data, E::Literal(L::Nil)),
                E::Literal(L::Boolean(false)),
            ),
            "firstToUpper(nil)..false",
        ),
    ];
    let values = [
        b"0".as_slice(),
        b"-0",
        b"0.1",
        b"1.2345678901234567",
        b"99999999999999",
        b"1e-5",
        b"1e-4",
        b"1e14",
        b"1e13",
        b"5e-324",
        b"1e309",
        b"-1e309",
        b"nan",
        b"inf",
        b"-inf",
        b"0xap2",
        b"abc",
        b"",
        b"a\0b",
        b"\xffa",
    ];
    let mut paired = 0;
    let mut errors = 0;
    for (expr, body) in cases {
        let native = injected(&source, data.clone(), expr, body);
        for value in values {
            if compare(&source, &native, &[b"__string ", value].concat()) {
                paired += 1
            } else {
                errors += 1
            }
        }
    }
    // An independent original-language observer distinguishes operand evaluation
    // from concat reduction. The native expressions above retain both AST shapes.
    let control:Function=source.source.public.source.lua.load("return function(grouped) local t={} local function f(k,v)t[#t+1]=k return v end local ok,e=pcall(function() if grouped then return (f('a',nil)..f('b','x'))..f('c',false) else return f('a',nil)..f('b','x')..f('c',false) end end) return ok,e,table.concat(t,',') end").set_name("@test-only-concat-evaluation-observer").eval().unwrap();
    let (ok, error, trace): (bool, String, String) = control.call(false).unwrap();
    assert!(!ok && error.contains("boolean"));
    assert_eq!(trace, "a,b,c");
    let (ok, error, trace): (bool, String, String) = control.call(true).unwrap();
    assert!(!ok && error.contains("nil"));
    assert_eq!(trace, "a,b");
    assert_eq!(paired + errors, 220);
    eprintln!(
        "Concat/source grouping:{paired} exact public graphs,{errors} source errors; independent original operand traces distinguish grouped reduction"
    );
}
#[test]
fn uppercase_uses_all_byte_mapping_and_rejects_nonstring_receivers_at_source_boundaries() {
    let snapshot = bundled_snapshot().unwrap();
    let data = snapshot.modifier_parser().data();
    let source = StringSource::new(Some("(.)"));
    let lua = &source.source.public.source.lua;
    let direct: Function = lua
        .globals()
        .get::<Table>("string")
        .unwrap()
        .get("upper")
        .unwrap();
    let mut injected_data = data.clone();
    injected_data.policy.first_to_upper_pattern = "(.)".into();
    // Literal values avoid the original public parser's earlier lowercase pass.
    let table = data.policy.mod_flags;
    for n in 0u16..=255 {
        let raw = [n as u8];
        let result: mlua::LuaString = direct.call(source.source.text(raw)).unwrap();
        let helper: mlua::LuaString = source.upper.call(source.source.text(raw)).unwrap();
        assert_eq!(result.as_bytes().as_ref(), helper.as_bytes().as_ref());
        assert_eq!(result.as_bytes().as_ref(), [raw[0].to_ascii_uppercase()]);
    }
    // One raw byte table field per byte is injected into the actual shared policy
    // table and consumed through ConstantField; no UTF-8 normalization is possible.
    let globals: Table = lua.globals().get("ModFlag").unwrap();
    let mut outputs = vec![];
    let mut source_fields = vec![];
    for n in 0u16..=255 {
        let key = format!("CallerByte{n}");
        // ParserValue text excludes NUL/invalid UTF-8. All256 raw bytes are
        // covered through captured runtime values separately below.
        if n > 0 && n < 128 {
            globals
                .set(key.as_str(), lua.create_string([n as u8]).unwrap())
                .unwrap();
            injected_data.tables[table.0 as usize - 1].fields.insert(
                key.clone(),
                P::Text(String::from_utf8(vec![n as u8]).unwrap()),
            );
            outputs.push(Field::Named {
                key: key.clone(),
                value: upper(
                    &injected_data,
                    E::ConstantField {
                        table,
                        key: key.clone(),
                    },
                ),
            });
            source_fields.push(format!("{key}=firstToUpper(ModFlag.{key})"));
        }
    }
    let native = injected(
        &source,
        injected_data,
        E::Table(outputs),
        &format!("{{{}}}", source_fields.join(",")),
    );
    assert!(compare(&source, &native, b"__string fixture"));
    let mut raw = data.clone();
    raw.policy.first_to_upper_pattern = "(.)".into();
    let raw_expr = upper(&raw, E::Argument(1));
    let native = injected(&source, raw, raw_expr, "firstToUpper(a)");
    for n in 0u16..=255 {
        let input = [b"__string ".as_slice(), &[n as u8]].concat();
        assert!(compare(&source, &native, &input));
        let result = native
            .parse(&input, &mut MatchBudget::default())
            .unwrap()
            .modifiers
            .unwrap();
        assert_eq!(
            result.fields["output"].as_bytes(),
            Some([n as u8].map(|b| b.to_ascii_uppercase()).as_slice())
        );
    }
    for (expr, body) in [
        (E::Literal(L::Nil), "nil"),
        (E::Literal(L::Boolean(false)), "false"),
        (E::Literal(L::Number(13.0)), "13"),
        (E::Table(vec![]), "{}"),
        (
            E::Table(vec![Field::Named {
                key: "gsub".into(),
                value: text("bad"),
            }]),
            "{gsub='bad'}",
        ),
    ] {
        let value = upper(data, expr);
        let native = injected(
            &source,
            data.clone(),
            value,
            &format!("firstToUpper({body})"),
        );
        assert!(!compare(&source, &native, b"__string fixture"));
    }
    eprintln!(
        "Byte uppercase:256 direct original upper/helper mappings and256 native/public byte pairs,127 injected non-NUL ASCII metadata fields; nil/boolean/number/table receiver source errors"
    );
}
#[test]
fn callable_table_methods_remain_deferred_without_claiming_opaque_function_execution() {
    let snapshot = bundled_snapshot().unwrap();
    let source = StringSource::new(None);
    let mut data = snapshot.modifier_parser().data().clone();
    let (pattern, id) = data.tables[data.dictionaries[&D::PreFlag].0 as usize - 1]
        .fields
        .iter()
        .find_map(|(key, v)| match v {
            P::Callback(id) if matches!(data.factories.get(id), Some(F::Pure(_))) => {
                Some((key.clone(), *id))
            }
            _ => None,
        })
        .unwrap();
    let actual = source
        .source
        .dictionary("preFlagList")
        .get::<Function>(pattern.as_str())
        .unwrap();
    let table = data.policy.mod_flags;
    data.tables[table.0 as usize - 1]
        .fields
        .insert("CallerOpaqueMethod".into(), P::Callback(id));
    source
        .source
        .public
        .source
        .lua
        .globals()
        .get::<Table>("ModFlag")
        .unwrap()
        .set("CallerOpaqueMethod", source.wrap(actual, "opaque_method"))
        .unwrap();
    let receiver = E::Table(vec![Field::Named {
        key: "gsub".into(),
        value: E::ConstantField {
            table,
            key: "CallerOpaqueMethod".into(),
        },
    }]);
    let expr = upper(&data, receiver);
    let native = injected(
        &source,
        data.clone(),
        expr.clone(),
        "firstToUpper({gsub=ModFlag.CallerOpaqueMethod})",
    );
    assert!(matches!(
        source.observe(b"__string x"),
        Observation::Returned(_)
    ));
    assert!(
        matches!(
            native.parse(b"__string x", &mut MatchBudget::default()),
            Err(ParserError::Deferred { .. })
        ),
        "opaque callback method must remain explicitly unavailable"
    );
    let early = injected(
        &source,
        data.clone(),
        concat(concat(E::Literal(L::Nil), text("x")), expr.clone()),
        "(nil..'x')..firstToUpper({gsub=ModFlag.CallerOpaqueMethod})",
    );
    assert!(matches!(
        source.observe(b"__string x"),
        Observation::SourceError(_)
    ));
    assert!(
        !source
            .calls()
            .iter()
            .any(|call| call.label == "opaque_method")
    );
    assert!(
        matches!(early.parse(b"__string x", &mut MatchBudget::default()), Err(ParserError::SourceError(ref why)) if why.contains("concatenation"))
    );
    let later = injected(
        &source,
        data,
        concat(E::Literal(L::Nil), concat(text("x"), expr)),
        "nil..'x'..firstToUpper({gsub=ModFlag.CallerOpaqueMethod})",
    );
    assert!(matches!(
        source.observe(b"__string x"),
        Observation::SourceError(_)
    ));
    assert!(
        source
            .calls()
            .iter()
            .any(|call| call.label == "opaque_method")
    );
    assert!(
        matches!(later.parse(b"__string x", &mut MatchBudget::default()), Err(ParserError::Deferred {stage:"firstToUpper receiver method",callback:Some(actual)}) if actual == id)
    );
    // The primitive call is closed to one source argument. Data-owned lowering
    // tests reject other arities; this independently observes why dropping an
    // ignored extra expression would be unsound.
    let control:Function=source.source.public.source.lua.load("return function(firstToUpper) local a,b=pcall(firstToUpper) local c,d=pcall(function() return firstToUpper('abc',error('extra evaluated')) end) return a,tostring(b),c,tostring(d) end").set_name("@test-only-helper-arity-control").eval().unwrap();
    let (missing, error, extra, message): (bool, String, bool, String) =
        control.call(source.upper.clone()).unwrap();
    assert!(!missing && error.contains("nil"));
    assert!(!extra && message.contains("extra evaluated"));
}
#[test]
fn every_original_string_factory_and_helper_execute_directly_in_observed_live_traces() {
    let snapshot = bundled_snapshot().unwrap();
    let catalog = snapshot.modifier_parser();
    let source = StringSource::new(None);
    let lua = &source.source.public.source.lua;
    let generate: Function = lua
        .load(include_str!("support/lua_pattern_witness.lua").replace("{55,97,65", "{97,55,65"))
        .eval()
        .unwrap();
    let copy: Function = lua.globals().get("copyTable").unwrap();
    let cases = lua.create_table().unwrap();
    let mut cold = vec![];
    for family in [D::Special, D::ModTag] {
        for (pattern, id) in candidates(catalog.data(), family) {
            let dictionary = source.source.dictionary(family.source_name());
            let original: Function = dictionary.get(pattern.as_str()).unwrap();
            let label = format!("warm:{}", id.0);
            dictionary
                .set(pattern.as_str(), source.wrap(original.clone(), &label))
                .unwrap();
            let witness: mlua::LuaString = generate.call(pattern.as_str()).unwrap();
            let witness = witness.as_bytes();
            let input = if family == D::Special {
                witness.to_vec()
            } else {
                [b"+7 to maximum Life ", witness.as_ref()].concat()
            };
            assert!(matches!(source.observe(&input), Observation::Returned(_)));
            let events: Table = lua.globals().get("ordinary_events").unwrap();
            let event = events
                .sequence_values::<Table>()
                .map(Result::unwrap)
                .find(|r| r.get::<String>("label").unwrap() == label)
                .unwrap();
            let args: Table = event.get("args").unwrap();
            let count: usize = args.get("count").unwrap();
            let result: Value = original
                .call(MultiValue::from_vec(
                    (1..=count).map(|i| args.get(i).unwrap()).collect(),
                ))
                .unwrap();
            let copied: Value = copy.call(result).unwrap();
            let graph = source
                .source
                .public
                .capture(MultiValue::from_vec(vec![copied]))
                .unwrap();
            dictionary.set(pattern.as_str(), original.clone()).unwrap();
            let row = lua.create_table().unwrap();
            row.set("callback", original.clone()).unwrap();
            row.set("args", args).unwrap();
            cases.set(cold.len() + 1, row).unwrap();
            cold.push((original.info().line_defined.unwrap(), graph, input));
        }
    }
    let observed: Table = lua
        .load(include_str!("support/string_factories_warm.lua"))
        .set_name("@test-only-original-string-factory-warm")
        .call(cases)
        .unwrap();
    assert_eq!(observed.get::<usize>("executions").unwrap(), 54 * 128);
    let live = observed
        .get::<Table>("live")
        .unwrap()
        .sequence_values::<Table>()
        .map(Result::unwrap)
        .map(|r| r.get::<usize>("line").unwrap())
        .collect::<BTreeSet<_>>();
    assert!(
        live.contains(&13),
        "original firstToUpper did not enter a completed live trace: {live:?}"
    );
    let native = CompiledModifierParser::new(catalog).unwrap();
    let results: Table = observed.get("results").unwrap();
    for (index, (line, graph, input)) in cold.iter().enumerate() {
        assert!(
            live.contains(line),
            "original factory line{line} absent from live traces:{live:?}"
        );
        let warmed = source
            .source
            .public
            .capture(MultiValue::from_vec(vec![results.get(index + 1).unwrap()]))
            .unwrap();
        assert_eq!(graph, &warmed, "cold/warm line{line}");
        assert!(compare(&source, &native, input));
    }
    eprintln!(
        "Warm string source:54 original factories plus helper,6912 direct executions; exact cold/warm metadata and warmed full public native graphs"
    );
}
