//! Independent full-original-parser and warmed primitive proof for ToNumber.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/callback_factories_source.rs"]
mod factory_source;
#[allow(dead_code)]
#[path = "support/flag_factories_source.rs"]
mod flag_source;
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
use flag_source::FlagSource as Source;
use mlua::{Function, MultiValue, Table, Value};
use poe_optimizer_data::{
    game_data::bundled_snapshot,
    modifier_parser::{
        ModifierParserCatalog, ModifierParserData, ParserCallbackId, ParserDictionary as D,
        ParserFactoryDisposition as F, ParserFactoryExpr as E, ParserFactoryField as Field,
        ParserFactoryLiteral as L, ParserNonFinite as N, ParserTable, ParserTableId,
        ParserValue as P,
    },
};
use poe_optimizer_engine::{
    lua_pattern::{MatchBudget, PatternError},
    modifier_parser::{CompiledModifierParser, ParserError},
    modifier_scan::ScanError,
};
use public_source::Observation;
use std::collections::{BTreeMap, BTreeSet};
fn has_number(e: &E) -> bool {
    match e {
        E::ToNumber { .. } => true,
        E::Negate(v) | E::FirstToUpper { value: v, .. } => has_number(v),
        E::Concat { left, right } => has_number(left) || has_number(right),
        E::Table(fields) => fields.iter().any(|f| match f {
            Field::Named { value, .. } | Field::List(value) => has_number(value),
        }),
        E::CreateMod { args } | E::Flag { args, .. } => args.iter().any(has_number),
        _ => false,
    }
}
fn candidates(data: &ModifierParserData, family: D) -> Vec<(String, ParserCallbackId)> {
    data.tables[data.dictionaries[&family].0 as usize - 1]
        .fields
        .iter()
        .filter_map(|(key, v)| {
            let P::Callback(id) = v else { return None };
            matches!(data.factories.get(id),Some(F::Pure(f)) if has_number(&f.body))
                .then_some((key.clone(), *id))
        })
        .collect()
}
fn compile(mut data: ModifierParserData) -> CompiledModifierParser {
    // These legacy-factory probes author their own dictionaries/recipes.
    data.programs = Default::default();
    CompiledModifierParser::new(&ModifierParserCatalog::new(data).unwrap()).unwrap()
}
fn row(data: &mut ModifierParserData, family: D, key: &str, value: P) {
    data.tables[data.dictionaries[&family].0 as usize - 1]
        .fields
        .insert(key.into(), value);
}
fn marker(data: &mut ModifierParserData) -> P {
    let id = ParserTableId(data.tables.len() as u32 + 1);
    data.tables.push(ParserTable::default());
    P::Table(id)
}
fn compare(source: &Source, native: &CompiledModifierParser, line: &[u8]) -> bool {
    match (
        source.observe(line),
        native.parse(line, &mut MatchBudget::default()),
    ) {
        (Observation::Returned(expected), Ok(actual)) => {
            assert_eq!(
                expected,
                native_observer::capture(&actual, native.catalog()).unwrap(),
                "{line:?}"
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
        (expected, actual) => {
            panic!("number source/native mismatch {line:?}: source={expected:?};native={actual:?}")
        }
    }
}
fn number(value: E) -> E {
    E::ToNumber {
        value: Box::new(value),
    }
}
fn text(v: &str) -> E {
    E::Literal(L::Text(v.into()))
}
fn output(value: E) -> E {
    E::Table(vec![Field::Named {
        key: "output".into(),
        value,
    }])
}
fn list(value: E) -> E {
    E::Table(vec![Field::List(value)])
}
fn uses_mod(e: &E) -> bool {
    match e {
        E::CreateMod { .. } => true,
        E::Negate(v) | E::FirstToUpper { value: v, .. } | E::ToNumber { value: v } => uses_mod(v),
        E::Flag { args, .. } => args.iter().any(uses_mod),
        E::Concat { left, right } => uses_mod(left) || uses_mod(right),
        E::Table(fields) => fields.iter().any(|f| match f {
            Field::Named { value, .. } | Field::List(value) => uses_mod(value),
        }),
        _ => false,
    }
}
fn fixture(source: &Source, body: &str) -> Function {
    let lua = &source.source.public.source.lua;
    let maker: Function = lua
        .load(format!(
            "return function(mod,flag,firstToUpper) return function(num,a,b,c,d,e) {body} end end"
        ))
        .set_name("@test-only-number-factory-caller-recipe")
        .eval()
        .unwrap();
    maker
        .call((
            source.source.create_mod.clone(),
            source.flag.clone(),
            source.upper.clone(),
        ))
        .unwrap()
}
fn install(
    source: &Source,
    mut data: ModifierParserData,
    body: E,
    lua: &str,
) -> (CompiledModifierParser, Function) {
    let id = data
        .callbacks
        .iter()
        .enumerate()
        .find_map(|(i, c)| {
            let names = c
                .upvalues
                .iter()
                .map(|u| u.name.as_str())
                .collect::<BTreeSet<_>>();
            let id = ParserCallbackId(i as u32 + 1);
            (names.contains("mod")
                && names.contains("flag")
                && names.contains("firstToUpper")
                && matches!(data.factories.get(&id), Some(F::Pure(_))))
            .then_some(id)
        })
        .unwrap();
    let direct = uses_mod(&body);
    let F::Pure(factory) = data.factories.get_mut(&id).unwrap() else {
        unreachable!()
    };
    factory.body = body;
    factory.parameter_count = 6;
    if !direct {
        factory.provenance.constructor = None
    };
    source.source.clear_specials();
    data.tables[data.dictionaries[&D::Special].0 as usize - 1] = ParserTable::default();
    let callback = fixture(source, &format!("return {lua}"));
    for pattern in [
        "^__number (.-)|(.-)|(.-)|(.-)|(.-)$",
        "^__scalar (.*)$",
        "^__missing$",
        "^__position ()$",
    ] {
        row(&mut data, D::Special, pattern, P::Callback(id));
        source.source.add(pattern, callback.clone());
    }
    (compile(data), callback)
}
fn actual_input(family: D, second: bool, witness: &[u8]) -> Vec<u8> {
    if family == D::Special {
        witness.to_vec()
    } else if family == D::PreFlag {
        [witness, b"7% increased damage"].concat()
    } else if second {
        [b"+7 to maximum Life __number_first__ ", witness].concat()
    } else {
        [b"+7 to maximum Life ", witness].concat()
    }
}
fn generator(source: &Source) -> Function {
    source
        .source
        .public
        .source
        .lua
        .load(include_str!("support/lua_pattern_witness.lua")
            .replace("{55,97,65", "{97,55,65")
            // An unbound repetition byte is a literal in original Lua patterns.
            // This only repairs generated inputs; original dictionaries are unchanged.
            .replace("elseif c=='*'or c=='+'or c=='-'or c=='?'then i=i+1", "elseif c=='*'or c=='+'or c=='-'or c=='?'then if i==1 or i==2 and pattern:sub(1,1)=='^'then emit(c)end;i=i+1"))
        .eval()
        .unwrap()
}

#[test]
fn every_original_number_factory_matches_all_actual_public_call_positions() {
    let snapshot = bundled_snapshot().unwrap();
    let catalog = snapshot.modifier_parser();
    let source = Source::new(None);
    let control = Source::new(None);
    let mut data = catalog.data().clone();
    let empty = marker(&mut data);
    row(&mut data, D::ModTag, "__number_first__", empty);
    for s in [&source, &control] {
        s.source
            .dictionary("modTagList")
            .set(
                "__number_first__",
                s.source.public.source.lua.create_table().unwrap(),
            )
            .unwrap();
    }
    let native = compile(data);
    let generate = generator(&source);
    let mut covered = BTreeMap::<(D, bool), BTreeSet<ParserCallbackId>>::new();
    let mut paired = 0;
    for family in [D::Special, D::PreFlag, D::ModTag] {
        for (pattern, id) in candidates(catalog.data(), family) {
            let table = source.source.dictionary(family.source_name());
            let original: Function = table.get(pattern.as_str()).unwrap();
            let label = format!("{}:{}", family.source_name(), id.0);
            table
                .set(pattern.as_str(), source.wrap(original, &label))
                .unwrap();
            let w: mlua::LuaString = generate.call(pattern.as_str()).unwrap();
            for second in [false, true] {
                if second && family != D::ModTag {
                    continue;
                }
                let input = actual_input(family, second, w.as_bytes().as_ref());
                assert!(compare(&source, &native, &input));
                let frames = match (family, second) {
                    (D::Special, _) => vec![7406, 7408],
                    (D::PreFlag, _) => vec![6656],
                    (_, false) => vec![6675, 6677],
                    (_, true) => vec![6684, 6686],
                };
                assert!(
                    source
                        .calls()
                        .iter()
                        .any(|c| c.label == label && c.frames.iter().any(|n| frames.contains(n))),
                    "{label} pattern={pattern:?} input={input:?}: {:?}",
                    source.calls()
                );
                assert_eq!(
                    source.source.public.observe(&input),
                    control.observe(&input)
                );
                covered.entry((family, second)).or_default().insert(id);
                paired += 1;
            }
        }
    }
    assert_eq!(covered[&(D::Special, false)].len(), 58);
    assert_eq!(covered[&(D::PreFlag, false)].len(), 1);
    for second in [false, true] {
        assert_eq!(covered[&(D::ModTag, second)].len(), 22);
    }
    assert_eq!(paired, 103);
    eprintln!(
        "All81 original ToNumber bodies selected through103 actual source call sites:58Special,1Prefix,22firstTag,22secondTag; complete public graphs equal"
    );
}
#[test]
fn every_original_number_factory_preserves_raw_aliases_in_its_actual_protocol() {
    let snapshot = bundled_snapshot().unwrap();
    let original = snapshot.modifier_parser().data();
    let source = Source::new(None);
    let mut data = original.clone();
    let empty = marker(&mut data);
    row(&mut data, D::ModTag, "__number_first__", empty);
    source
        .source
        .dictionary("modTagList")
        .set(
            "__number_first__",
            source.source.public.source.lua.create_table().unwrap(),
        )
        .unwrap();
    let mut cases = vec![];
    for family in [D::Special, D::PreFlag, D::ModTag] {
        for (key, id) in candidates(original, family) {
            let f: Function = source
                .source
                .dictionary(family.source_name())
                .get(key.as_str())
                .unwrap();
            let alias = format!("__n{} (.-)|(.-)|(.-)|(.-)|(.-)", id.0);
            let pattern = match family {
                D::Special => format!("^{alias}$"),
                D::PreFlag => format!("^{alias}; "),
                _ => format!(" {alias}$"),
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
    let (mut paired, mut errors) = (0, 0);
    for (family, id) in cases {
        for captures in [
            b"12|34|56|78|90".as_slice(),
            b"-0|-0|0|0|0",
            b"1e309|1e-324|nan|inf|-inf",
            b"name|word|other|x|y",
            b"0x10|0x1p-1074|0b11|1.2|0x1.fffffffffffff8p1023",
            b"||||",
            b"\xff\0\x80|a|b|c|d",
        ] {
            for second in [false, true] {
                if second && family != D::ModTag {
                    continue;
                }
                let raw = [
                    format!("__n{} ", id.0).as_bytes(),
                    captures,
                    if family == D::PreFlag {
                        b"; ".as_slice()
                    } else {
                        b""
                    },
                ]
                .concat();
                let input = actual_input(family, second, &raw);
                if compare(&source, &native, &input) {
                    paired += 1
                } else {
                    errors += 1
                }
            }
        }
    }
    assert_eq!(paired + errors, 103 * 7);
    eprintln!(
        "All81 ToNumber aliases in real protocols: {paired} exact graphs,{errors} ordered source errors"
    );
}

fn scalar_inputs() -> Vec<Vec<u8>> {
    let mut out = [
        "",
        " ",
        "+",
        "-",
        ".",
        "0",
        "-0",
        "+0",
        "-.0",
        "00007",
        " 7 ",
        "1.",
        ".1",
        "-1.25",
        "1e",
        "1e+",
        "1e2",
        "1.2.3",
        " 1 2 ",
        "0x",
        "0x.",
        "0x.p1",
        "0x1",
        "-0x0",
        "0x.1",
        "0x1.",
        "0x1p-1075",
        "0x1.00000000000001p-1075",
        "0x1p-1074",
        "0x1fffffffffffff",
        "0x20000000000001",
        "0x1.fffffffffffff8p1023",
        "0x1e2",
        "0x1e-2",
        "0b",
        "0b0",
        "-0b0",
        "0b.1",
        "0b1p2",
        "0b2",
        "nan",
        "-nan",
        "+nan",
        "nan()",
        "nan(1)",
        "inf",
        "infinity",
        "-infinity",
        "+inf",
        "infinite",
        "1f",
        "2u",
        "3ll",
        "0x10ull",
        "1e309",
        "-1e309",
        "1e-324",
        "-1e-324",
        "9007199254740993",
        "9007199254740995",
        "1.7976931348623157e308",
        "1.7976931348623158e308",
        "1.7976931348623159e308",
        "2.2250738585072011e-308",
        "2.2250738585072012e-308",
        "2.2250738585072013e-308",
        "4.9406564584124654e-324",
    ]
    .into_iter()
    .map(|s| s.as_bytes().to_vec())
    .collect::<Vec<_>>();
    for body in ["1e", "0e", "0x1p", "0x0p"] {
        for exponent in [
            "1048575",
            "1048576",
            "-1048575",
            "-1048576",
            "999999999999999999999999999",
        ] {
            out.push(format!("{body}{exponent}").into_bytes());
        }
    }
    for n in [31, 32, 53, 63, 64, 65] {
        out.push(format!("0b{}", "1".repeat(n)).into_bytes());
    }
    for byte in 0..=255 {
        out.push(vec![byte, b'7']);
        out.push(vec![b'7', byte]);
    }
    out
}
#[test]
fn conversions_preserve_raw_byte_scanning_rounding_and_public_nil_fields() {
    let snapshot = bundled_snapshot().unwrap();
    let source = Source::new(None);
    let (native, _) = install(
        &source,
        snapshot.modifier_parser().data().clone(),
        output(number(E::Argument(1))),
        "{output=tonumber(a)}",
    );
    let inputs = scalar_inputs();
    for value in &inputs {
        assert!(compare(
            &source,
            &native,
            &[b"__scalar ".as_slice(), value].concat()
        ));
    }
    for input in [b"__missing".as_slice(), b"__position "] {
        assert!(compare(&source, &native, input));
    }
    eprintln!(
        "{} byte/scanner factory observations plus missing and position captures; exact complete graph numbers include signedzero/nonfinite/subnormal and nil field removal",
        inputs.len()
    );
}
#[test]
fn nested_conversions_preserve_scalar_kinds_sparse_lists_and_ordered_errors() {
    let snapshot = bundled_snapshot().unwrap();
    let data = snapshot.modifier_parser().data();
    let source = Source::new(None);
    let upper = |value| E::FirstToUpper {
        helper: data.helpers["firstToUpper"],
        value: Box::new(value),
    };
    let nil = E::Literal(L::Nil);
    let cases = vec![
        (output(number(E::Argument(0))), "{output=tonumber(num)}"),
        (
            output(number(number(E::Argument(1)))),
            "{output=tonumber(tonumber(a))}",
        ),
        (output(number(nil.clone())), "{output=tonumber(nil)}"),
        (
            output(number(E::Literal(L::Boolean(false)))),
            "{output=tonumber(false)}",
        ),
        (
            output(number(E::Literal(L::Boolean(true)))),
            "{output=tonumber(true)}",
        ),
        (output(number(E::Table(vec![]))), "{output=tonumber({})}"),
        (
            output(number(E::Literal(L::Number(-0.0)))),
            "{output=tonumber(-0.0)}",
        ),
        (
            output(number(E::Literal(L::NonFinite(N::Nan)))),
            "{output=tonumber(0/0)}",
        ),
        (
            output(number(E::Literal(L::NonFinite(N::PositiveInfinity)))),
            "{output=tonumber(1/0)}",
        ),
        (
            output(number(E::Literal(L::NonFinite(N::NegativeInfinity)))),
            "{output=tonumber(-1/0)}",
        ),
        (
            output(E::Negate(Box::new(number(E::Argument(1))))),
            "{output=-tonumber(a)}",
        ),
        (
            output(number(E::Negate(Box::new(E::Argument(1))))),
            "{output=tonumber(-a)}",
        ),
        (
            output(number(upper(nil.clone()))),
            "{output=tonumber(firstToUpper(nil))}",
        ),
        (
            output(number(E::Concat {
                left: Box::new(E::Argument(1)),
                right: Box::new(text(".5")),
            })),
            "{output=tonumber(a..'.5')}",
        ),
        (
            E::Table(vec![
                Field::List(number(E::Argument(1))),
                Field::List(number(E::Argument(2))),
                Field::List(number(E::Argument(3))),
                Field::Named {
                    key: "output".into(),
                    value: number(E::Argument(4)),
                },
            ]),
            "{tonumber(a),tonumber(b),tonumber(c),output=tonumber(d)}",
        ),
        (
            list(E::CreateMod {
                args: vec![
                    number(E::Argument(1)),
                    number(E::Argument(2)),
                    number(E::Argument(3)),
                    nil.clone(),
                    number(E::Argument(4)),
                    nil.clone(),
                    number(E::Argument(5)),
                ],
            }),
            "{mod(tonumber(a),tonumber(b),tonumber(c),nil,tonumber(d),nil,tonumber(e))}",
        ),
        (
            list(E::Flag {
                helper: data.helpers["flag"],
                args: vec![
                    number(E::Argument(1)),
                    nil.clone(),
                    number(E::Argument(2)),
                    nil.clone(),
                    number(E::Argument(3)),
                ],
            }),
            "{flag(tonumber(a),nil,tonumber(b),nil,tonumber(c))}",
        ),
        (
            output(number(E::CreateMod {
                args: vec![text("Ignored"), text("BASE"), E::Argument(0)],
            })),
            "{output=tonumber(mod('Ignored','BASE',num))}",
        ),
        (
            E::Table(vec![
                Field::List(number(upper(nil.clone()))),
                Field::List(number(E::Negate(Box::new(nil.clone())))),
            ]),
            "{tonumber(firstToUpper(nil)),tonumber(-nil)}",
        ),
        (
            E::Table(vec![
                Field::List(number(E::Negate(Box::new(nil)))),
                Field::List(number(upper(E::Literal(L::Nil)))),
            ]),
            "{tonumber(-nil),tonumber(firstToUpper(nil))}",
        ),
    ];
    let (mut paired, mut errors) = (0, 0);
    let count = cases.len();
    for (expr, lua) in cases {
        let (native, _) = install(&source, data.clone(), expr, lua);
        for line in [
            b"__number 12|34|56|78|90".as_slice(),
            b"__number -0|0|-0|0|-0",
            b"__number 1e309|nan|-inf|1e-324|0x1p-1074",
            b"__number name|word|other|x|y",
            b"__number ||||",
            b"__number \xff\0\x80|a|b|c|d",
            b"__missing",
            b"__position ",
        ] {
            if compare(&source, &native, line) {
                paired += 1
            } else {
                errors += 1
            }
        }
    }
    assert_eq!(paired + errors, count * 8);
    eprintln!(
        "{count} synthetic closed ToNumber recipe shapes: {paired} full graphs and {errors} exact source-error boundaries"
    );
}
#[test]
fn global_primitive_does_not_call_tostring_or_execute_opaque_values() {
    let snapshot = bundled_snapshot().unwrap();
    let source = Source::new(None);
    let lua = &source.source.public.source.lua;
    let primitive: Function = lua.globals().get("tonumber").unwrap();
    assert_eq!(primitive.info().what, "C");
    let nil: Value = primitive.call(Value::Nil).unwrap();
    assert!(matches!(nil, Value::Nil));
    assert!(primitive.call::<Value>(()).is_err());
    let kind:Table=lua.load("return setmetatable({}, {__tostring=function() error('tostring must not run') end,__call=function() error('call must not run') end})").eval().unwrap();
    assert!(matches!(
        primitive.call::<Value>(kind.clone()).unwrap(),
        Value::Nil
    ));
    let root = snapshot.modifier_parser().data().policy.mod_flags;
    let mut table_data = snapshot.modifier_parser().data().clone();
    let empty = marker(&mut table_data);
    let function = fixture(&source, "error('opaque value must not be executed')");
    for (value, original) in [
        (
            P::Callback(snapshot.modifier_parser().data().helpers["flag"]),
            Value::Function(function),
        ),
        (empty, Value::Table(kind)),
    ] {
        let mut data = table_data.clone();
        data.tables[root.0 as usize - 1]
            .fields
            .insert("CallerNumberInput".into(), value);
        lua.globals()
            .get::<Table>("ModFlag")
            .unwrap()
            .set("CallerNumberInput", original)
            .unwrap();
        let (native, _) = install(
            &source,
            data,
            output(number(E::ConstantField {
                table: root,
                key: "CallerNumberInput".into(),
            })),
            "{output=tonumber(ModFlag.CallerNumberInput)}",
        );
        assert!(compare(&source, &native, b"__missing"));
    }
    assert_eq!(
        primitive.to_pointer(),
        lua.globals()
            .get::<Function>("tonumber")
            .unwrap()
            .to_pointer()
    );
    eprintln!(
        "Original global C tonumber identity retained; omitted argument errors, explicit nil returns nil; table metamethod and opaque callback values are never invoked or graph-normalized"
    );
}
#[test]
fn converted_values_preserve_public_cache_copies_and_item_adapter_boundaries() {
    use poe_optimizer_import::item_loading::{
        DependencyResult, ItemLoadProvider, NativeModifierParserProvider, ParseRequest,
    };
    let snapshot = bundled_snapshot().unwrap();
    let source = Source::new(None);
    let body = list(E::CreateMod {
        args: vec![
            text("Caller"),
            text("BASE"),
            number(E::Argument(1)),
            E::Table(vec![
                Field::Named {
                    key: "type".into(),
                    value: text("CallerTag"),
                },
                Field::Named {
                    key: "value".into(),
                    value: number(E::Argument(2)),
                },
            ]),
        ],
    });
    let (native, _) = install(
        &source,
        snapshot.modifier_parser().data().clone(),
        body,
        "{mod('Caller','BASE',tonumber(a),{type='CallerTag',value=tonumber(b)})}",
    );
    let native = std::sync::Arc::new(native);
    let line = b"__number 12|34|56|78|90";
    let raw = source.source.public.raw(line).unwrap();
    let expected = source.source.public.capture(raw.clone()).unwrap();
    let row = raw
        .front()
        .unwrap()
        .as_table()
        .unwrap()
        .get::<Table>(1)
        .unwrap();
    row.set("value", 999).unwrap();
    row.get::<Table>(1).unwrap().set("value", 888).unwrap();
    assert_eq!(
        source
            .source
            .public
            .capture(source.source.public.raw(line).unwrap())
            .unwrap(),
        expected
    );
    assert!(compare(&source, &native, line));
    let mut provider = NativeModifierParserProvider::from_compiled(native.clone());
    for (captures, available) in [
        ("12|34|56|78|90", true),
        ("word|word|word|word|word", true),
        ("1e309|34|56|78|90", false),
        ("nan|34|56|78|90", false),
    ] {
        let line = format!("__number {captures}");
        assert!(compare(&source, &native, line.as_bytes()));
        let result = provider.parse_modifier(&ParseRequest {
            sequence: 1,
            line_index: Some(1),
            origin: None,
            text: line,
            combined: false,
        });
        assert_eq!(
            matches!(result, DependencyResult::Available(_)),
            available,
            "{result:?}"
        );
    }
    eprintln!(
        "Original cached converted nested values are independently copied; finite/nil item metadata stays supported and nonfinite outputs remain explicitly unavailable"
    );
}
fn warm_live(observed: &Table) -> BTreeSet<(String, usize)> {
    observed
        .get::<Table>("live")
        .unwrap()
        .sequence_values::<Table>()
        .map(Result::unwrap)
        .map(|r| (r.get("source").unwrap(), r.get("line").unwrap()))
        .collect()
}
#[test]
fn every_original_number_body_and_real_primitive_execute_in_completed_live_traces() {
    let snapshot = bundled_snapshot().unwrap();
    let catalog = snapshot.modifier_parser();
    let source = Source::new(None);
    let lua = &source.source.public.source.lua;
    let generate = generator(&source);
    let copy: Function = lua.globals().get("copyTable").unwrap();
    let cases = lua.create_table().unwrap();
    let mut cold = vec![];
    for family in [D::Special, D::PreFlag, D::ModTag] {
        for (pattern, id) in candidates(catalog.data(), family) {
            let table = source.source.dictionary(family.source_name());
            let original: Function = table.get(pattern.as_str()).unwrap();
            let label = format!("warm:{}:{}", family.source_name(), id.0);
            table
                .set(pattern.as_str(), source.wrap(original.clone(), &label))
                .unwrap();
            let witness: mlua::LuaString = generate.call(pattern.as_str()).unwrap();
            let input = actual_input(family, false, witness.as_bytes().as_ref());
            assert!(matches!(source.observe(&input), Observation::Returned(_)));
            let events: Table = lua.globals().get("ordinary_events").unwrap();
            let event = events
                .sequence_values::<Table>()
                .map(Result::unwrap)
                .find(|e| e.get::<String>("label").unwrap() == label)
                .unwrap();
            let args: Table = event.get("args").unwrap();
            let count: usize = args.get("count").unwrap();
            let value: Value = original
                .call(MultiValue::from_vec(
                    (1..=count).map(|i| args.get(i).unwrap()).collect(),
                ))
                .unwrap();
            let copied: Value = copy.call(value).unwrap();
            let graph = source
                .source
                .public
                .capture(MultiValue::from_vec(vec![copied]))
                .unwrap();
            table.set(pattern.as_str(), original.clone()).unwrap();
            let row = lua.create_table().unwrap();
            row.set("callback", original.clone()).unwrap();
            row.set("args", args).unwrap();
            cases.set(cold.len() + 1, row).unwrap();
            cold.push((original.info().line_defined.unwrap(), graph, input));
        }
    }
    let observed: Table = lua
        .load(include_str!("support/number_factories_warm.lua"))
        .set_name("@test-only-number-factories-warm")
        .call(cases)
        .unwrap();
    assert_eq!(observed.get::<usize>("executions").unwrap(), 81 * 128);
    assert!(
        observed.get::<usize>("tonumber_live").unwrap() > 0,
        "No original tonumber C-function event in a completed live trace"
    );
    let live = warm_live(&observed);
    let results: Table = observed.get("results").unwrap();
    let native = CompiledModifierParser::new(catalog).unwrap();
    for (index, (line, graph, input)) in cold.iter().enumerate() {
        assert!(
            live.contains(&("@src/Modules/ModParser.lua".into(), *line)),
            "factory line{line} absent from live traces"
        );
        let warm = source
            .source
            .public
            .capture(MultiValue::from_vec(vec![results.get(index + 1).unwrap()]))
            .unwrap();
        assert_eq!(&warm, graph, "cold/warm line{line}");
        assert!(compare(&source, &native, input));
    }
    eprintln!(
        "81 retained original ToNumber bodies:10368 uncached calls; every prototype plus actual original tonumber C-function pointer observed in completed live traces; exact cold/warm/public graphs"
    );
}
#[test]
fn dynamic_warm_conversion_inputs_preserve_exact_bits_and_nil_results() {
    let snapshot = bundled_snapshot().unwrap();
    let source = Source::new(None);
    let lua = &source.source.public.source.lua;
    let (native, callback) = install(
        &source,
        snapshot.modifier_parser().data().clone(),
        output(number(E::Argument(1))),
        "{output=tonumber(a)}",
    );
    let inputs = scalar_inputs();
    let arguments = lua.create_table().unwrap();
    let mut cold = vec![];
    for (index, input) in inputs.iter().enumerate() {
        let bytes = lua.create_string(input).unwrap();
        let args = lua.create_table().unwrap();
        args.set(2, bytes.clone()).unwrap();
        arguments.set(index + 1, args).unwrap();
        let value: Value = callback.call((Value::Nil, bytes)).unwrap();
        cold.push(
            source
                .source
                .public
                .capture(MultiValue::from_vec(vec![value]))
                .unwrap(),
        );
    }
    let case = lua.create_table().unwrap();
    case.set("callback", callback).unwrap();
    case.set("inputs", arguments).unwrap();
    case.set("iterations", inputs.len() * 8).unwrap();
    let cases = lua.create_table().unwrap();
    cases.set(1, case).unwrap();
    let observed: Table = lua
        .load(include_str!("support/number_factories_warm.lua"))
        .set_name("@test-only-dynamic-tonumber-warm")
        .call(cases)
        .unwrap();
    assert!(
        observed.get::<usize>("tonumber_live").unwrap() > 0,
        "dynamic actual tonumber primitive never recorded into a completed live trace"
    );
    assert_eq!(
        observed.get::<usize>("executions").unwrap(),
        inputs.len() * 8
    );
    let results = observed
        .get::<Table>("dynamic")
        .unwrap()
        .get::<Table>(1)
        .unwrap();
    for (index, input) in inputs.iter().enumerate() {
        let warm = source
            .source
            .public
            .capture(MultiValue::from_vec(vec![results.get(index + 1).unwrap()]))
            .unwrap();
        assert_eq!(warm, cold[index], "dynamic numeric input{input:?}");
        assert!(compare(
            &source,
            &native,
            &[b"__scalar ".as_slice(), input].concat()
        ));
    }
    eprintln!(
        "{} changing byte inputs,{} warmed direct conversion calls: exact cold/warm/full native graphs and original tonumber C-function event in live trace",
        inputs.len(),
        inputs.len() * 8
    );
}

#[test]
fn numeric_passthrough_and_string_conversion_have_explicit_ieee_bit_proof() {
    use poe_optimizer_engine::modifier_parser::ModifierValue;
    let snapshot = bundled_snapshot().unwrap();
    let source = Source::new(None);
    let lua = &source.source.public.source.lua;
    let primitive: Function = lua.globals().get("tonumber").unwrap();
    let cases = [
        (-0.0, E::Literal(L::Number(-0.0)), "-0.0"),
        (0.0, E::Literal(L::Number(0.0)), "0.0"),
        (
            f64::from_bits(1),
            E::Literal(L::Number(f64::from_bits(1))),
            "0x1p-1074",
        ),
        (
            f64::INFINITY,
            E::Literal(L::NonFinite(N::PositiveInfinity)),
            "1/0",
        ),
        (
            f64::NEG_INFINITY,
            E::Literal(L::NonFinite(N::NegativeInfinity)),
            "-1/0",
        ),
        (
            f64::from_bits(0xfff8000000000000),
            E::Literal(L::NonFinite(N::Nan)),
            "0/0",
        ),
    ];
    for (value, expr, original) in cases {
        let echoed: Value = primitive.call(Value::Number(value)).unwrap();
        let actual = match echoed {
            Value::Number(n) => n,
            Value::Integer(n) => n as f64,
            v => panic!("numeric passthrough kind{}", v.type_name()),
        };
        assert_eq!(actual.to_bits(), value.to_bits());
        let (native, _) = install(
            &source,
            snapshot.modifier_parser().data().clone(),
            output(number(expr)),
            &format!("{{output=tonumber({original})}}"),
        );
        assert!(compare(&source, &native, b"__missing"));
        let state = native
            .parse(b"__missing", &mut MatchBudget::default())
            .unwrap()
            .modifiers
            .unwrap();
        let ModifierValue::Number(actual) = state.fields["output"] else {
            panic!("expected number")
        };
        assert_eq!(actual.to_bits(), value.to_bits());
    }
    let (native, _) = install(
        &source,
        snapshot.modifier_parser().data().clone(),
        output(number(E::Argument(1))),
        "{output=tonumber(a)}",
    );
    for (input, bits) in [
        ("-0", 0x8000000000000000),
        ("0", 0),
        ("-1e-324", 0x8000000000000000),
        ("0x1p-1074", 1),
        ("nan", 0xfff8000000000000),
    ] {
        let raw: Value = primitive.call(lua.create_string(input).unwrap()).unwrap();
        let source_bits = match raw {
            Value::Number(n) => n.to_bits(),
            Value::Integer(n) => (n as f64).to_bits(),
            v => panic!("unexpected {}", v.type_name()),
        };
        assert_eq!(source_bits, bits);
        let state = native
            .parse(
                format!("__scalar {input}").as_bytes(),
                &mut MatchBudget::default(),
            )
            .unwrap()
            .modifiers
            .unwrap();
        let ModifierValue::Number(n) = state.fields["output"] else {
            panic!("expected number")
        };
        assert_eq!(n.to_bits(), bits);
    }
    eprintln!(
        "11 explicit original/native IEEE-bit pairs: six numeric passthroughs and five converted zero/subnormal/nonfinite values; canonical source NaN domain retained"
    );
}
