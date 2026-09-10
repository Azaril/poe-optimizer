//! Complete original Flag wrapper, constructor, call packing and public graph parity.
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
use flag_source::FlagSource;
use mlua::{Function, MultiValue, Table, Value};
use poe_optimizer_data::{
    game_data::bundled_snapshot,
    modifier_parser::{
        ModifierParserCatalog, ModifierParserData, ParserCallbackId, ParserDictionary as D,
        ParserFactoryDisposition as Disposition, ParserFactoryExpr as E, ParserFactoryField as F,
        ParserFactoryLiteral as L, ParserNonFinite as N, ParserTable, ParserValue as P,
    },
};
use poe_optimizer_engine::{
    lua_pattern::{MatchBudget, PatternError},
    modifier_parser::{CompiledModifierParser, ParserError},
    modifier_scan::ScanError,
};
use public_source::Observation;
use std::collections::BTreeSet;
fn has_flag(e: &E) -> bool {
    match e {
        E::Flag { .. } => true,
        E::Negate(v) | E::FirstToUpper { value: v, .. } | E::ToNumber { value: v } => has_flag(v),
        E::Concat { left, right } => has_flag(left) || has_flag(right),
        E::Table(fields) => fields.iter().any(|f| match f {
            F::Named { value, .. } | F::List(value) => has_flag(value),
        }),
        E::CreateMod { args } => args.iter().any(has_flag),
        _ => false,
    }
}
fn candidates(data: &ModifierParserData) -> Vec<(String, ParserCallbackId)> {
    data.tables[data.dictionaries[&D::Special].0 as usize - 1]
        .fields
        .iter()
        .filter_map(|(key, v)| {
            let P::Callback(id) = v else { return None };
            matches!(data.factories.get(id),Some(Disposition::Pure(f)) if has_flag(&f.body))
                .then_some((key.clone(), *id))
        })
        .collect()
}
fn compile(data: ModifierParserData) -> CompiledModifierParser {
    CompiledModifierParser::new(&ModifierParserCatalog::new(data).unwrap()).unwrap()
}
fn compare(source: &FlagSource, native: &CompiledModifierParser, line: &[u8]) -> bool {
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
        (source, native) => {
            panic!("flag source/native mismatch {line:?}: source={source:?}, native={native:?}")
        }
    }
}
fn text(value: &str) -> E {
    E::Literal(L::Text(value.into()))
}
fn flag(data: &ModifierParserData, args: Vec<E>) -> E {
    E::Flag {
        helper: data.helpers["flag"],
        args,
    }
}
fn upper(data: &ModifierParserData, value: E) -> E {
    E::FirstToUpper {
        helper: data.helpers["firstToUpper"],
        value: Box::new(value),
    }
}
fn list(value: E) -> E {
    E::Table(vec![F::List(value)])
}
fn tag() -> E {
    E::Table(vec![F::Named {
        key: "type".into(),
        value: text("CallerTag"),
    }])
}
fn uses_mod(e: &E) -> bool {
    match e {
        E::CreateMod { .. } => true,
        E::Flag { args, .. } => args.iter().any(uses_mod),
        E::Negate(v) | E::FirstToUpper { value: v, .. } | E::ToNumber { value: v } => uses_mod(v),
        E::Concat { left, right } => uses_mod(left) || uses_mod(right),
        E::Table(fields) => fields.iter().any(|f| match f {
            F::Named { value, .. } | F::List(value) => uses_mod(value),
        }),
        _ => false,
    }
}
fn install(
    source: &FlagSource,
    mut data: ModifierParserData,
    body: E,
    lua: &str,
) -> CompiledModifierParser {
    // This source recipe can use all three proven helper bindings. Identity is
    // chosen structurally from actual captured values, never by a callback ID.
    let id = candidates(&data)
        .into_iter()
        .map(|(_, id)| id)
        .find(|id| {
            let names = data.callbacks[id.0 as usize - 1]
                .upvalues
                .iter()
                .map(|u| u.name.as_str())
                .collect::<BTreeSet<_>>();
            names.contains("flag") && names.contains("mod") && names.contains("firstToUpper")
        })
        .unwrap();
    let direct = uses_mod(&body);
    let Disposition::Pure(factory) = data.factories.get_mut(&id).unwrap() else {
        unreachable!()
    };
    factory.parameter_count = 6;
    factory.body = body;
    if !direct {
        factory.provenance.constructor = None;
    }
    source.source.clear_specials();
    data.tables[data.dictionaries[&D::Special].0 as usize - 1] = ParserTable::default();
    let callback = source.fixture("caller", &format!("return {lua}"));
    for pattern in [
        "^__flag (.*)|(.*)|(.*)|(.*)|(.*)$",
        "^__missing$",
        "^__position ()$",
    ] {
        data.tables[data.dictionaries[&D::Special].0 as usize - 1]
            .fields
            .insert(pattern.into(), P::Callback(id));
        source.source.add(pattern, callback.clone());
    }
    compile(data)
}
#[test]
fn every_original_flag_body_is_selected_by_its_real_special_pattern() {
    let snapshot = bundled_snapshot().unwrap();
    let data = snapshot.modifier_parser().data();
    let native = compile(data.clone());
    let source = FlagSource::new(None);
    let control = FlagSource::new(None);
    let generate: Function = source
        .source
        .public
        .source
        .lua
        .load(include_str!("support/lua_pattern_witness.lua").replace("{55,97,65", "{97,55,65"))
        .eval()
        .unwrap();
    let mut covered = BTreeSet::new();
    for (key, id) in candidates(data) {
        let original: Function = source.source.special.get(key.as_str()).unwrap();
        let label = format!("real:{}", id.0);
        source
            .source
            .special
            .set(key.as_str(), source.wrap(original, &label))
            .unwrap();
        let input: mlua::LuaString = generate.call(key.as_str()).unwrap();
        let input = input.as_bytes();
        assert!(
            compare(&source, &native, input.as_ref()),
            "actual flag{id:?}"
        );
        assert!(
            source
                .calls()
                .iter()
                .any(|c| c.label == label && c.frames.iter().any(|n| [7406, 7408].contains(n))),
            "{label}: {:?}",
            source.calls()
        );
        assert_eq!(
            source.source.public.observe(input.as_ref()),
            control.observe(input.as_ref()),
            "observer changed original"
        );
        covered.insert(id);
    }
    assert_eq!(covered.len(), 71);
    eprintln!(
        "All71 original Flag bodies selected at71 actual Special call sites; every complete public graph matches"
    );
}
#[test]
fn every_original_flag_body_preserves_broad_raw_alias_arguments() {
    let snapshot = bundled_snapshot().unwrap();
    let original = snapshot.modifier_parser().data();
    let source = FlagSource::new(None);
    let mut data = original.clone();
    let mut rows = vec![];
    for (key, id) in candidates(original) {
        let callback: Function = source.source.special.get(key.as_str()).unwrap();
        let pattern = format!("^__flag_{} (.-)|(.-)|(.-)|(.-)|(.-)$", id.0);
        data.tables[data.dictionaries[&D::Special].0 as usize - 1]
            .fields
            .insert(pattern.clone(), P::Callback(id));
        source.source.add(&pattern, callback);
        rows.push(id);
    }
    let native = compile(data);
    let mut paired = 0;
    let mut errors = 0;
    for id in rows {
        for captures in [
            b"12|34|56|78|90".as_slice(),
            b"-0|-0|0|0|0",
            b"1e309|cold|mana|red|shield",
            b"name|other|third|fourth|fifth",
            b"||||",
            b"\xff\0\x80|a|b|c|d",
        ] {
            let input = [format!("__flag_{} ", id.0).as_bytes(), captures].concat();
            if compare(&source, &native, &input) {
                paired += 1
            } else {
                errors += 1
            }
        }
    }
    assert_eq!(paired + errors, 426);
    eprintln!("All71 original Flag aliases:{paired} exact full graphs,{errors} ordered errors");
}
#[test]
fn original_flag_preserves_exact_zero_and_sparse_variadic_constructor_vectors() {
    let source = FlagSource::new(None);
    let lua = &source.source.public.source.lua;
    let observer:Function=lua.load("return function(mod) return function(...) observed_flag_mod_args={count=select('#',...),...};return mod(...) end end").set_name("@test-only-flag-constructor-observer").eval::<Function>().unwrap().call(source.source.create_mod.clone()).unwrap();
    let observed = source.helper(observer, None);
    let shared = lua.create_table().unwrap();
    shared.set("type", "SharedTag").unwrap();
    let tag = Value::Table(shared);
    let cases = vec![
        vec![],
        vec![Value::Nil],
        vec![Value::Nil, Value::Nil],
        vec![Value::Nil, Value::Nil, Value::Nil],
        vec![Value::Boolean(false)],
        vec![Value::Number(-0.0)],
        vec![Value::Number(f64::INFINITY)],
        vec![source.source.text(b"\xff\0x")],
        vec![tag.clone()],
        vec![
            source.source.text("Name"),
            source.source.text("source"),
            tag.clone(),
        ],
        vec![
            source.source.text("Name"),
            Value::Nil,
            Value::Number(7.0),
            Value::Number(9.0),
            tag.clone(),
        ],
        vec![
            source.source.text("Name"),
            Value::Boolean(false),
            source.source.text("7"),
            source.source.text("9"),
            tag.clone(),
        ],
        vec![
            source.source.text("Name"),
            tag.clone(),
            tag.clone(),
            Value::Nil,
            tag.clone(),
            Value::Nil,
        ],
        vec![
            source.source.text("Name"),
            source.source.text("source"),
            Value::Number(-0.0),
            Value::Number(f64::NEG_INFINITY),
            tag.clone(),
            Value::Nil,
        ],
    ];
    for args in &cases {
        let actual: Value = source
            .flag
            .call(MultiValue::from_vec(args.clone()))
            .unwrap();
        let measured: Value = observed.call(MultiValue::from_vec(args.clone())).unwrap();
        assert_eq!(
            source
                .source
                .public
                .capture(MultiValue::from_vec(vec![actual]))
                .unwrap(),
            source
                .source
                .public
                .capture(MultiValue::from_vec(vec![measured]))
                .unwrap()
        );
        let recorded: Table = lua.globals().get("observed_flag_mod_args").unwrap();
        let count: usize = recorded.get("count").unwrap();
        let mut expected = vec![
            args.first().cloned().unwrap_or(Value::Nil),
            source.source.text("FLAG"),
            Value::Boolean(true),
        ];
        expected.extend(args.iter().skip(1).cloned());
        assert_eq!(count, expected.len());
        assert_eq!(
            source
                .source
                .public
                .capture(MultiValue::from_vec(expected))
                .unwrap(),
            source
                .source
                .public
                .capture(MultiValue::from_vec(
                    (1..=count).map(|i| recorded.get(i).unwrap()).collect()
                ))
                .unwrap()
        );
    }
    eprintln!(
        "Original Flag: {} exact absolute constructor argument vectors including zero args,nilholes,trailingnil,sharedtables,rawbytes andnonfinite names",
        cases.len()
    );
}
#[test]
fn native_flag_recipes_preserve_constructor_names_tails_and_mixed_nested_outputs() {
    let snapshot = bundled_snapshot().unwrap();
    let data = snapshot.modifier_parser().data();
    let source = FlagSource::new(None);
    let nil = E::Literal(L::Nil);
    let number = |n| E::Literal(L::Number(n));
    let mut cases = vec![
        (flag(data, vec![]), "flag()".to_string()),
        (flag(data, vec![nil.clone()]), "flag(nil)".into()),
        (list(flag(data, vec![E::Argument(1)])), "{flag(a)}".into()),
        (list(flag(data, vec![E::Argument(0)])), "{flag(num)}".into()),
        (
            list(flag(data, vec![E::Literal(L::Boolean(false))])),
            "{flag(false)}".into(),
        ),
        (
            list(flag(data, vec![tag()])),
            "{flag({type='CallerTag'})}".into(),
        ),
        (
            list(flag(
                data,
                vec![
                    E::Argument(1),
                    nil.clone(),
                    number(17.0),
                    number(31.0),
                    tag(),
                ],
            )),
            "{flag(a,nil,17,31,{type='CallerTag'})}".into(),
        ),
        (
            list(flag(
                data,
                vec![
                    E::Argument(1),
                    text("source"),
                    number(17.0),
                    number(31.0),
                    tag(),
                    nil.clone(),
                ],
            )),
            "{flag(a,'source',17,31,{type='CallerTag'},nil)}".into(),
        ),
        (
            list(flag(
                data,
                vec![
                    E::Argument(1),
                    text("source"),
                    text("17"),
                    text("31"),
                    tag(),
                ],
            )),
            "{flag(a,'source','17','31',{type='CallerTag'})}".into(),
        ),
        (
            list(flag(
                data,
                vec![
                    E::Argument(1),
                    E::Literal(L::Boolean(false)),
                    nil.clone(),
                    number(9.0),
                    nil.clone(),
                    tag(),
                ],
            )),
            "{flag(a,false,nil,9,nil,{type='CallerTag'})}".into(),
        ),
        (
            list(flag(
                data,
                vec![
                    flag(data, vec![text("Inner")]),
                    nil.clone(),
                    flag(data, vec![text("Tag")]),
                    nil.clone(),
                ],
            )),
            "{flag(flag('Inner'),nil,flag('Tag'),nil)}".into(),
        ),
        (
            list(flag(
                data,
                vec![
                    E::Argument(1),
                    nil.clone(),
                    E::Literal(L::NonFinite(N::Nan)),
                ],
            )),
            "{flag(a,nil,0/0)}".into(),
        ),
        (
            list(flag(data, vec![upper(data, nil.clone()), E::Table(vec![])])),
            "{flag(firstToUpper(nil),{})}".into(),
        ),
        (
            list(flag(
                data,
                vec![
                    text("Outer"),
                    flag(data, vec![text("Inner")]),
                    upper(data, nil.clone()),
                ],
            )),
            "{flag('Outer',flag('Inner'),firstToUpper(nil))}".into(),
        ),
        (
            E::Table(vec![
                F::List(flag(data, vec![text("First")])),
                F::List(E::CreateMod {
                    args: vec![text("Mixed"), text("BASE"), E::Argument(0)],
                }),
                F::List(flag(data, vec![upper(data, E::Argument(2))])),
            ]),
            "{flag('First'),mod('Mixed','BASE',num),flag(firstToUpper(b))}".into(),
        ),
    ];
    cases.push((
        E::Table(
            (0..6)
                .map(|i| F::List(flag(data, vec![text(&format!("Flag{i}"))])))
                .collect(),
        ),
        format!(
            "{{{}}}",
            (0..6)
                .map(|i| format!("flag('Flag{i}')"))
                .collect::<Vec<_>>()
                .join(",")
        ),
    ));
    let mut paired = 0;
    let mut errors = 0;
    for (expr, lua) in cases {
        let native = install(&source, data.clone(), expr, &lua);
        for captures in [
            b"12|BASE|tag|source|tail".as_slice(),
            b"-0|a|b|c|d",
            b"1e309|fire|cold|shield|red",
            b"name|blue|next|fourth|fifth",
            b"||||",
            b"\xff\0\x80|a|b|c|d",
        ] {
            let line = [b"__flag ", captures].concat();
            if compare(&source, &native, &line) {
                paired += 1
            } else {
                errors += 1
            }
        }
        for line in [b"__missing".as_slice(), b"__position "] {
            if compare(&source, &native, line) {
                paired += 1
            } else {
                errors += 1
            }
        }
    }
    assert_eq!(paired + errors, 128);
    eprintln!(
        "Explicit Flag recipes:16shapes,{paired} exact full public graphs,{errors} matching source errors; root/zeroargs/arbitrarynames/absoluteholes/nested6flags/mixedmod"
    );
}
#[test]
fn injected_typed_flag_prefix_changes_runtime_outputs_and_preserves_cache_copies() {
    let snapshot = bundled_snapshot().unwrap();
    let mut paired = 0;
    for (kind, value) in [
        ("FLAG", true),
        ("CallerType", false),
        ("", false),
        ("\0Donnée\nType", true),
    ] {
        let source = FlagSource::new(Some((kind, value)));
        let mut data = snapshot.modifier_parser().data().clone();
        data.policy.flag_mod_type = kind.into();
        data.policy.flag_mod_value = value;
        let expr = list(flag(
            &data,
            vec![
                E::Argument(1),
                text("source"),
                E::Literal(L::Number(7.0)),
                E::Literal(L::Number(9.0)),
                tag(),
                E::Literal(L::Nil),
            ],
        ));
        let native = install(
            &source,
            data,
            expr,
            "{flag(a,'source',7,9,{type='CallerTag'},nil)}",
        );
        for captures in [b"12|a|b|c|d".as_slice(), b"\xff\0\x80|a|b|c|d", b"||||"] {
            let line = [b"__flag ", captures].concat();
            assert!(compare(&source, &native, &line));
            let actual = native
                .parse(&line, &mut MatchBudget::default())
                .unwrap()
                .modifiers
                .unwrap();
            let m = actual.indexed[&1].as_table().unwrap();
            assert_eq!(m.fields["type"].as_bytes(), Some(kind.as_bytes()));
            assert!(
                matches!(m.fields["value"],poe_optimizer_engine::modifier_parser::ModifierValue::Boolean(v) if v==value)
            );
            paired += 1;
        }
        let line = b"__flag original|a|b|c|d";
        let raw = source.source.public.raw(line).unwrap();
        let original_graph = source.source.public.capture(raw.clone()).unwrap();
        let root = raw.front().unwrap().as_table().unwrap();
        let first: Table = root.get(1).unwrap();
        first.set("name", "mutated caller").unwrap();
        first
            .get::<Table>(1)
            .unwrap()
            .set("type", "mutated tag")
            .unwrap();
        // These calls deliberately keep the original public cache; observer helper
        // clearing would make a cache-copy regression vacuous.
        let fresh = source.source.public.raw(line).unwrap();
        assert_eq!(source.source.public.capture(fresh).unwrap(), original_graph);
        assert!(compare(&source, &native, line));
        paired += 1;
    }
    eprintln!(
        "Injected Flag string/bool prefixes:{paired} exact native/public graphs,4original warm-cache copy controls; empty/NUL/UTF8/literalfalse preserved"
    );
}
#[test]
fn opaque_flag_function_values_preserve_the_nested_constructor_boundary() {
    use poe_optimizer_import::item_loading::{
        DependencyResult, ItemLoadProvider, NativeModifierParserProvider, ParseRequest,
    };
    let snapshot = bundled_snapshot().unwrap();
    let mut data = snapshot.modifier_parser().data().clone();
    let source = FlagSource::new(None);
    let root = data.policy.mod_flags;
    let helper = data.helpers["flag"];
    data.tables[root.0 as usize - 1]
        .fields
        .insert("CallerFlagFunction".into(), P::Callback(helper));
    source
        .source
        .public
        .source
        .lua
        .globals()
        .get::<Table>("ModFlag")
        .unwrap()
        .set("CallerFlagFunction", source.flag.clone())
        .unwrap();
    let expr = list(flag(
        &data,
        vec![
            text("OpaqueFlag"),
            E::Table(vec![
                F::Named {
                    key: "type".into(),
                    value: text("CallerTag"),
                },
                F::Named {
                    key: "func".into(),
                    value: E::ConstantField {
                        table: root,
                        key: "CallerFlagFunction".into(),
                    },
                },
            ]),
        ],
    ));
    let native = std::sync::Arc::new(install(
        &source,
        data,
        expr,
        "{flag('OpaqueFlag',{type='CallerTag',func=ModFlag.CallerFlagFunction})}",
    ));
    let line = b"__flag name|a|b|c|d";
    let original = source
        .source
        .public
        .capture(source.source.public.raw(line).unwrap())
        .unwrap();
    let result = native.parse(line, &mut MatchBudget::default()).unwrap();
    let actual = native_observer::capture(&result, native.catalog()).unwrap();
    for graph in [&original, &actual] {
        let flag = graph
            .callbacks
            .iter()
            .find(|v| v.first_line == 2177)
            .unwrap();
        assert_eq!(flag.last_line, 2179);
        assert_eq!(
            flag.upvalues
                .iter()
                .map(|(n, _)| n.as_slice())
                .collect::<Vec<_>>(),
            [b"mod".as_slice()]
        );
    }
    let original_constructor = original
        .callbacks
        .iter()
        .find(|v| v.first_line == 57)
        .unwrap();
    let actual_constructor = actual
        .callbacks
        .iter()
        .find(|v| v.first_line == 57)
        .unwrap();
    assert_eq!(
        original_constructor
            .upvalues
            .iter()
            .map(|(n, _)| n.as_slice())
            .collect::<Vec<_>>(),
        [b"select".as_slice(), b"type".as_slice()]
    );
    assert!(actual_constructor.upvalues.is_empty());
    assert_ne!(
        original, actual,
        "do not normalize indirect opaque closure graphs"
    );
    let mut provider = NativeModifierParserProvider::from_compiled(native);
    let result = provider.parse_modifier(&ParseRequest {
        sequence: 1,
        line_index: Some(1),
        origin: None,
        text: String::from_utf8(line.to_vec()).unwrap(),
        combined: false,
    });
    assert!(
        matches!(result, DependencyResult::Unavailable(_)),
        "{result:?}"
    );
    eprintln!(
        "Opaque Flag value retains mod upvalue and nested full-source constructor difference; native item metadata remains explicitly unavailable"
    );
}
#[test]
fn every_original_flag_body_helper_and_constructor_run_in_live_traces() {
    let snapshot = bundled_snapshot().unwrap();
    let catalog = snapshot.modifier_parser();
    let source = FlagSource::new(None);
    let lua = &source.source.public.source.lua;
    let generate: Function = lua
        .load(include_str!("support/lua_pattern_witness.lua").replace("{55,97,65", "{97,55,65"))
        .eval()
        .unwrap();
    let copy: Function = lua.globals().get("copyTable").unwrap();
    let cases = lua.create_table().unwrap();
    let mut cold = vec![];
    for (pattern, id) in candidates(catalog.data()) {
        let original: Function = source.source.special.get(pattern.as_str()).unwrap();
        let label = format!("warm:{}", id.0);
        source
            .source
            .special
            .set(pattern.as_str(), source.wrap(original.clone(), &label))
            .unwrap();
        let input: mlua::LuaString = generate.call(pattern.as_str()).unwrap();
        let input = input.as_bytes().to_vec();
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
        source
            .source
            .special
            .set(pattern.as_str(), original.clone())
            .unwrap();
        let row = lua.create_table().unwrap();
        row.set("callback", original.clone()).unwrap();
        row.set("args", args).unwrap();
        cases.set(cold.len() + 1, row).unwrap();
        cold.push((original.info().line_defined.unwrap(), graph, input));
    }
    let observed: Table = lua
        .load(include_str!("support/flag_factories_warm.lua"))
        .set_name("@test-only-original-flag-warm")
        .call(cases)
        .unwrap();
    assert_eq!(observed.get::<usize>("executions").unwrap(), 71 * 128);
    let live = observed
        .get::<Table>("live")
        .unwrap()
        .sequence_values::<Table>()
        .map(Result::unwrap)
        .map(|r| {
            (
                r.get::<String>("source").unwrap(),
                r.get::<usize>("line").unwrap(),
            )
        })
        .collect::<BTreeSet<_>>();
    assert!(
        live.contains(&("@src/Modules/ModParser.lua".into(), 2177)),
        "Flag helper absent from live traces:{live:?}"
    );
    assert!(
        live.contains(&("@src/Modules/ModTools.lua".into(), 57)),
        "createMod absent from live traces:{live:?}"
    );
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
        assert_eq!(graph, &warm, "cold/warm line{line}");
        assert!(compare(&source, &native, input));
    }
    eprintln!(
        "All71 original Flag bodies plus Flag/helper constructor observed in completed live traces;9088 direct calls and exact cold/warm/public native graphs"
    );
}
