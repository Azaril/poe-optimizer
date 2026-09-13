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
        ParserFactoryLiteral as L, ParserFactoryReplacement as Replacement, ParserTable,
        ParserTableId, ParserValue as P,
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
        E::Concat { .. } | E::FirstToUpper { .. } | E::Gsub { .. } => true,
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
// Exact current-package breadth gates; source call coverage is checked against
// every retained callback identity below, including all earlier string bodies.
const STRING_FAMILIES: [(D, usize); 3] = [(D::Special, 71), (D::ModTag, 11), (D::PreFlag, 5)];
fn string_breadth(data: &ModifierParserData) -> usize {
    let mut total = 0;
    for (family, expected) in STRING_FAMILIES {
        let entries = candidates(data, family);
        let ids = entries.iter().map(|(_, id)| *id).collect::<BTreeSet<_>>();
        assert_eq!(entries.len(), expected, "{family:?} string entries");
        assert_eq!(ids.len(), expected, "{family:?} distinct string bodies");
        total += ids.len();
    }
    assert_eq!(total, 87);
    total
}
fn public_input(family: D, witness: &[u8], second_tag: bool) -> Vec<u8> {
    match family {
        D::Special => {
            assert!(!second_tag);
            witness.to_vec()
        }
        D::PreFlag => {
            assert!(!second_tag);
            [witness, b"7% increased damage"].concat()
        }
        D::ModTag if second_tag => [b"+7 to maximum Life __string_first__ ", witness].concat(),
        D::ModTag => [b"+7 to maximum Life ", witness].concat(),
        _ => panic!("not a string factory public position"),
    }
}
fn public_frames(family: D, second_tag: bool) -> &'static [usize] {
    match family {
        D::Special => {
            assert!(!second_tag);
            &[7406, 7408]
        }
        D::PreFlag => {
            assert!(!second_tag);
            &[6656]
        }
        D::ModTag if second_tag => &[6684, 6686],
        D::ModTag => &[6675, 6677],
        _ => panic!("not a string factory public position"),
    }
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
    let bodies = string_breadth(catalog.data());
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
    for (family, _) in STRING_FAMILIES {
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
                if second && family != D::ModTag {
                    continue;
                }
                let input = public_input(family, witness.as_ref(), second);
                assert!(
                    compare(&source, &native, &input),
                    "actual source callback{id:?}"
                );
                let frames = public_frames(family, second);
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
    for (family, _) in STRING_FAMILIES {
        let expected = candidates(catalog.data(), family)
            .into_iter()
            .map(|(_, id)| id)
            .collect::<BTreeSet<_>>();
        assert_eq!(coverage[&(family, false)], expected);
        if family == D::ModTag {
            assert_eq!(coverage[&(family, true)], expected);
        }
    }
    assert_eq!(paired, bodies + candidates(catalog.data(), D::ModTag).len());
    assert_eq!(paired, 98);
    eprintln!(
        "String factories: all{bodies} original bodies,{paired} actual full public graphs (71Special,11firstTag,11secondTag,5PreFlag); exact reached source call positions and unwrapped controls"
    );
}
#[test]
fn all_original_string_bodies_preserve_raw_capture_aliases_and_ordered_errors() {
    let snapshot = bundled_snapshot().unwrap();
    let original = snapshot.modifier_parser().data();
    let bodies = string_breadth(original);
    let source = StringSource::new(None);
    let mut data = original.clone();
    let mut cases = vec![];
    for (family, _) in STRING_FAMILIES {
        for (key, id) in candidates(original, family) {
            let f = source
                .source
                .dictionary(family.source_name())
                .get::<Function>(key.as_str())
                .unwrap();
            let alias = format!("__string_{} (.*)|(.*)|(.*)|(.*)|(.*)", id.0);
            let pattern = match family {
                D::Special => format!("^{alias}$"),
                // A literal terminator prevents the final greedy capture from
                // consuming the real modifier form that follows this prefix.
                D::PreFlag => format!("^{alias} __string_end__ "),
                D::ModTag => format!(" {alias}$"),
                _ => unreachable!(),
            };
            let label = format!("alias:{}:{}", family.source_name(), id.0);
            row(&mut data, family, &pattern, P::Callback(id));
            source
                .source
                .dictionary(family.source_name())
                .set(pattern.as_str(), source.wrap(f, &label))
                .unwrap();
            cases.push((family, id, label));
        }
    }
    let native = compile(data);
    assert_eq!(cases.len(), bodies);
    let mut paired = 0;
    let mut errors = 0;
    for (family, id, label) in cases {
        for captures in [
            b"12|cold|fire|mana|blue".as_slice(),
            b"abc|lightning|life|red|green",
            b"-0|a\0b|\xffz|\x80a|chaos",
            b"1e309|physical|shield|x|y",
            b"||||",
        ] {
            let input = [
                if family == D::ModTag {
                    b"+7 to maximum Life ".as_slice()
                } else {
                    b""
                },
                format!("__string_{} ", id.0).as_bytes(),
                captures,
                if family == D::PreFlag {
                    b" __string_end__ 7% increased damage".as_slice()
                } else {
                    b""
                },
            ]
            .concat();
            if compare(&source, &native, &input) {
                paired += 1
            } else {
                errors += 1
            }
            let calls = source.calls();
            let call = calls.iter().find(|call| call.label == label
                && call.frames.iter().any(|line| public_frames(family, false).contains(line)))
                .unwrap_or_else(|| panic!("aliased original callback {label} not reached at its actual public position"));
            assert_eq!(call.count, if family == D::PreFlag { 5 } else { 6 });
            if family == D::PreFlag {
                assert_eq!(
                    call.arguments.roots,
                    captures
                        .split(|byte| *byte == b'|')
                        .map(|bytes| public_source::Atom::Bytes(bytes.to_vec()))
                        .collect::<Vec<_>>(),
                    "PreFlag must receive the raw capture pack without an inserted numeric argument"
                );
            }
        }
    }
    assert_eq!(paired + errors, bodies * 5);
    assert_eq!(paired + errors, 435);
    assert!(paired > 0 && errors > 0);
    eprintln!(
        "All{bodies} original string closures via labelled aliases at Special/firstTag/PreFlag positions:{paired} exact full graphs,{errors} ordered source errors; all435 probes reached the selected original callback"
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
    // This exact original ignores its declared cond parameter. The table-method
    // receiver is therefore a valid source input even as new argument-consuming
    // factories become admitted earlier in the dictionary's ordering.
    let pattern = "^enemies you've hit recently have ";
    let P::Callback(id) =
        &data.tables[data.dictionaries[&D::PreFlag].0 as usize - 1].fields[pattern]
    else {
        panic!("pinned argument-ignoring original callback")
    };
    let id = *id;
    let F::Pure(factory) = data.factories.get(&id).unwrap() else {
        panic!("pinned pure fixture")
    };
    assert_eq!(factory.parameter_count, 1);
    assert_eq!(
        factory.body,
        E::Table(vec![
            Field::Named {
                key: "playerTag".into(),
                value: E::Table(vec![
                    Field::Named {
                        key: "type".into(),
                        value: text("Condition")
                    },
                    Field::Named {
                        key: "var".into(),
                        value: text("HitRecently")
                    },
                ])
            },
            Field::Named {
                key: "applyToEnemy".into(),
                value: E::Literal(L::Boolean(true))
            },
        ])
    );
    let actual = source
        .source
        .dictionary("preFlagList")
        .get::<Function>(pattern)
        .unwrap();
    assert_eq!(
        actual.info().source.as_deref(),
        Some("@src/Modules/ModParser.lua")
    );
    assert_eq!(actual.info().line_defined, Some(1385));
    assert_eq!(actual.info().last_line_defined, Some(1387));
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
    let calls = source.calls();
    let method = calls
        .iter()
        .find(|call| call.label == "opaque_method")
        .unwrap();
    assert_eq!(method.count, 3);
    assert!(matches!(
        method.arguments.roots[0],
        public_source::Atom::Table(_)
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
    let bodies = string_breadth(catalog.data());
    let source = StringSource::new(None);
    let lua = &source.source.public.source.lua;
    let generate: Function = lua
        .load(include_str!("support/lua_pattern_witness.lua").replace("{55,97,65", "{97,55,65"))
        .eval()
        .unwrap();
    let copy: Function = lua.globals().get("copyTable").unwrap();
    let cases = lua.create_table().unwrap();
    let mut cold = vec![];
    for (family, _) in STRING_FAMILIES {
        for (pattern, id) in candidates(catalog.data(), family) {
            let dictionary = source.source.dictionary(family.source_name());
            let original: Function = dictionary.get(pattern.as_str()).unwrap();
            let label = format!("warm:{}", id.0);
            dictionary
                .set(pattern.as_str(), source.wrap(original.clone(), &label))
                .unwrap();
            let witness: mlua::LuaString = generate.call(pattern.as_str()).unwrap();
            let witness = witness.as_bytes();
            let input = public_input(family, witness.as_ref(), false);
            assert!(matches!(source.observe(&input), Observation::Returned(_)));
            assert!(
                source.calls().iter().any(|call| call.label == label
                    && call
                        .frames
                        .iter()
                        .any(|line| public_frames(family, false).contains(line))),
                "warm input callback {label} not reached at its genuine public call position"
            );
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
    assert_eq!(cold.len(), bodies);
    assert_eq!(observed.get::<usize>("executions").unwrap(), bodies * 128);
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
        "Warm string source:{bodies} original factories at Special/firstTag/PreFlag positions plus retained firstToUpper helper,{} direct executions; exact cold/warm metadata and warmed full public native graphs. Live-trace evidence covers each original function for its captured input; it does not claim every Gsub branch or string.upper replacement invocation, including digit-only inputs.",
        bodies * 128
    );
}

#[test]
fn original_weapon_ailment_flag_uses_the_complete_public_parser() {
    let snapshot = bundled_snapshot().unwrap();
    let source = StringSource::new(None);
    let control = StringSource::new(None);
    // These are unchanged real input lines. Caller-defined dictionaries and
    // test recipes are not used in this production parser comparison.
    let native = CompiledModifierParser::new(snapshot.modifier_parser()).unwrap();
    for (input, expected_name) in [
        (
            b"All damage with this Weapon causes Electrocution buildup".as_slice(),
            b"CanElectrocution".as_slice(),
        ),
        (
            b"All damage with this Weapon causes Freeze buildup".as_slice(),
            b"CanFreeze".as_slice(),
        ),
    ] {
        assert!(compare(&source, &native, input));
        assert_eq!(source.observe(input), control.observe(input));
        let parsed = native.parse(input, &mut MatchBudget::default()).unwrap();
        let modifiers = parsed.modifiers.expect("positive original modifier result");
        assert_eq!(modifiers.indexed.len(), 1);
        let modifier = modifiers.indexed_value(1).as_table().unwrap();
        assert_eq!(modifier.field("name").as_bytes(), Some(expected_name));
        assert_eq!(modifier.field("type").as_bytes(), Some(b"FLAG".as_slice()));
        let condition = modifier.indexed_value(1).as_table().unwrap();
        assert_eq!(
            condition.field("type").as_bytes(),
            Some(b"Condition".as_slice())
        );
        assert_eq!(
            condition.field("var").as_bytes(),
            Some(b"{Hand}Attack".as_slice())
        );
    }
}
fn gsub(value: E, pattern: &str, replacement: Replacement) -> E {
    E::Gsub {
        value: Box::new(value),
        pattern: pattern.into(),
        replacement,
    }
}
#[test]
fn configured_gsub_chains_match_complete_graphs_and_reached_source_errors() {
    let snapshot = bundled_snapshot().unwrap();
    let cases = [
        ("", Replacement::Text("-".into())),
        ("a*", Replacement::Text("%0".into())),
        ("(.)", Replacement::Text("<%1>%%".into())),
        ("()", Replacement::Text("%1".into())),
        ("(.)", Replacement::Text("%2".into())),
        ("[", Replacement::Text("x".into())),
        ("^z[", Replacement::Text("x".into())),
        ("^%l", Replacement::StringUpper),
        (" %l", Replacement::StringUpper),
        ("()", Replacement::StringUpper),
        ("(%l*)", Replacement::StringUpper),
        ("%f[%a]", Replacement::StringUpper),
    ];
    let mut returned = 0;
    let mut errors = 0;
    for (pattern, replacement) in cases {
        let source = StringSource::new(None);
        let replacement_text = match &replacement {
            Replacement::Text(text) => format!("[==[{text}]==]"),
            Replacement::StringUpper => "string.upper".into(),
        };
        let expr = gsub(E::Argument(1), pattern, replacement);
        // A named field is an explicit scalar consumer. The native Gsub node
        // does not claim the count return at a variadic/list-tail boundary.
        let body = format!("a:gsub([==[{pattern}]==], {replacement_text})");
        let native = injected(
            &source,
            snapshot.modifier_parser().data().clone(),
            expr,
            &body,
        );
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
                returned += 1;
            } else {
                errors += 1;
            }
        }
        for input in [b"__missing".as_slice(), b"__position "] {
            assert!(!compare(&source, &native, input));
            errors += 1;
        }
        assert!(compare(&source, &native, b"+7 to maximum Life"));
    }
    let source = StringSource::new(None);
    let expr = concat(
        text("Can"),
        gsub(
            gsub(
                gsub(E::Argument(1), "^%l", Replacement::StringUpper),
                " %l",
                Replacement::StringUpper,
            ),
            " ",
            Replacement::Text(String::new()),
        ),
    );
    let native = injected(
        &source,
        snapshot.modifier_parser().data().clone(),
        expr,
        "'Can'..a:gsub('^%l', string.upper):gsub(' %l', string.upper):gsub(' ', '')",
    );
    for input in [
        b"__string electrocution".as_slice(),
        b"__string energy shield",
        b"__string a\0b",
        b"__string ",
        b"__missing",
    ] {
        if compare(&source, &native, input) {
            returned += 1;
        } else {
            errors += 1;
        }
    }
    assert!(returned > 0 && errors > 0);
    eprintln!(
        "Configured scalar substitutions/chains: {returned} exact graphs, {errors} matching source errors; unused malformed rules remain unexecuted"
    );
}
