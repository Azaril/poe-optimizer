//! Independent original-source factory, call-packing and public output graphs.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/callback_factories_source.rs"]
mod factory_source;
#[allow(dead_code)]
#[path = "support/mod_parser_native_observer.rs"]
mod native_observer;
#[allow(dead_code)]
#[path = "support/mod_parser_public_source.rs"]
mod public_source;
#[allow(dead_code)]
#[path = "support/item_loading_runtime.rs"]
mod runtime;
use factory_source::FactorySource;
use mlua::{Function, MultiValue, Table, Value};
use public_source::Atom;

#[test]
fn original_constructor_keeps_absolute_vararg_positions_holes_and_table_aliases() {
    let source = FactorySource::new();
    let tag = source.public.source.lua.create_table().unwrap();
    tag.set("type", "CallerTag").unwrap();
    let t = Value::Table(tag.clone());
    let cases = [
        (vec![Value::Number(7.0)], 0.0f64, 0.0f64, vec![1]),
        (vec![Value::Nil, Value::Number(7.0)], 7.0, 0.0, vec![]),
        (vec![t.clone(), Value::Number(7.0)], 7.0, 0.0, vec![]),
        (
            vec![t.clone(), t.clone(), Value::Number(9.0)],
            0.0,
            9.0,
            vec![],
        ),
        (vec![Value::Nil, source.text("7")], 0.0, 0.0, vec![2]),
        (
            vec![Value::Nil, Value::Nil, Value::Nil, t.clone()],
            0.0,
            0.0,
            vec![4],
        ),
        (vec![source.text("source"), t.clone()], 0.0, 0.0, vec![1]),
        (
            vec![source.text("source"), Value::Number(7.0), t.clone()],
            7.0,
            0.0,
            vec![1],
        ),
    ];
    for (args, flags, keywords, keys) in cases {
        let mut values = vec![source.text("Name"), source.text("BASE"), Value::Nil];
        values.extend(args);
        let result = source.create(values);
        assert!(matches!(result.get::<Value>("value").unwrap(), Value::Nil));
        assert_eq!(
            result.get::<f64>("flags").unwrap().to_bits(),
            flags.to_bits()
        );
        assert_eq!(
            result.get::<f64>("keywordFlags").unwrap().to_bits(),
            keywords.to_bits()
        );
        let mut numeric = result
            .clone()
            .pairs::<Value, Value>()
            .map(Result::unwrap)
            .filter_map(|(k, _)| match k {
                Value::Integer(n) => Some(n),
                Value::Number(n) => Some(n as i64),
                _ => None,
            })
            .collect::<Vec<_>>();
        numeric.sort();
        assert_eq!(numeric, keys);
    }
    let result = source.create(vec![
        Value::Nil,
        Value::Boolean(false),
        t.clone(),
        t.clone(),
        t,
    ]);
    assert!(matches!(result.get::<Value>("name").unwrap(), Value::Nil));
    assert!(!result.get::<bool>("type").unwrap());
    for key in [1, 2] {
        assert_eq!(
            result.get::<Table>(key).unwrap().to_pointer(),
            tag.to_pointer()
        );
    }
    assert_eq!(
        result.get::<Table>("value").unwrap().to_pointer(),
        tag.to_pointer()
    );
}

#[test]
fn original_special_dispatcher_preserves_numeric_prefix_raw_captures_and_five_capture_limit() {
    let source = FactorySource::new();
    let callback: Function = source
        .public
        .source
        .lua
        .load("return function(...) factory_args={count=select('#',...),...}; return {} end")
        .set_name("@test-only-call-packing-observer")
        .eval()
        .unwrap();
    for pattern in [
        "^__pack__$",
        "^__pack (.*)$",
        "^__position ()(.+)$",
        "^__six (.) (.) (.) (.) (.) (.)$",
    ] {
        source.add(pattern, callback.clone());
    }
    for (line, expected) in [
        (b"__pack__".to_vec(), vec![Value::Nil]),
        (
            b"__pack 12".to_vec(),
            vec![Value::Number(12.0), source.text("12")],
        ),
        (
            b"__pack Blue".to_vec(),
            vec![Value::Nil, source.text("blue")],
        ),
        (b"__pack ".to_vec(), vec![Value::Nil, source.text("")]),
        (
            b"__pack \xff\0\x80".to_vec(),
            vec![Value::Nil, source.text(b"\xff\0\x80")],
        ),
        (
            b"__position abc".to_vec(),
            vec![Value::Number(12.0), Value::Number(12.0), source.text("abc")],
        ),
        (
            b"__six 1 2 3 4 5 6".to_vec(),
            vec![
                Value::Number(1.0),
                source.text("1"),
                source.text("2"),
                source.text("3"),
                source.text("4"),
                source.text("5"),
            ],
        ),
    ] {
        source.public.raw(&line).unwrap();
        let observed = source
            .public
            .source
            .lua
            .globals()
            .get::<Table>("factory_args")
            .unwrap();
        assert_eq!(
            observed.get::<usize>("count").unwrap(),
            expected.len(),
            "{line:?}"
        );
        let actual = (1..=expected.len())
            .map(|i| observed.get::<Value>(i).unwrap())
            .collect();
        assert_eq!(
            source.public.capture(MultiValue::from_vec(actual)).unwrap(),
            source
                .public
                .capture(MultiValue::from_vec(expected))
                .unwrap(),
            "{line:?}"
        );
    }
}

#[test]
fn original_public_factory_nil_empty_false_and_cache_copy_are_distinct() {
    let source = FactorySource::new();
    for (name, body) in [
        ("nil", "return nil"),
        ("empty", "return {}"),
        ("false", "return false"),
    ] {
        source.add(&format!("^__return_{name}__$"), source.synthetic(body));
    }
    assert!(
        source
            .public
            .capture(source.public.raw(b"__return_nil__").unwrap())
            .unwrap()
            .roots
            .is_empty()
    );
    let empty = source
        .public
        .capture(source.public.raw(b"__return_empty__").unwrap())
        .unwrap();
    assert_eq!(empty.roots, vec![Atom::Table(0)]);
    assert!(empty.tables[0].is_empty());
    assert_eq!(
        source
            .public
            .capture(source.public.raw(b"__return_false__").unwrap())
            .unwrap()
            .roots,
        vec![Atom::Boolean(false)]
    );
    source.add(
        "^__alias__$",
        source.synthetic("local t={value=7}; return {mod('Name','LIST',t,t,t)}"),
    );
    let result = source
        .public
        .raw(b"__alias__")
        .unwrap()
        .front()
        .unwrap()
        .as_table()
        .unwrap()
        .get::<Table>(1)
        .unwrap();
    let payload = result.get::<Table>("value").unwrap();
    assert_ne!(
        payload.to_pointer(),
        result.get::<Table>(1).unwrap().to_pointer()
    );
    assert_ne!(
        result.get::<Table>(1).unwrap().to_pointer(),
        result.get::<Table>(2).unwrap().to_pointer()
    );
    let cached = source
        .public
        .cache
        .get::<Table>("__alias__")
        .unwrap()
        .get::<Table>(1)
        .unwrap()
        .get::<Table>(1)
        .unwrap();
    assert_eq!(
        cached.get::<Table>("value").unwrap().to_pointer(),
        cached.get::<Table>(1).unwrap().to_pointer()
    );
    payload.set("value", 99).unwrap();
    let next = source
        .public
        .raw(b"__alias__")
        .unwrap()
        .front()
        .unwrap()
        .as_table()
        .unwrap()
        .get::<Table>(1)
        .unwrap();
    assert_eq!(
        next.get::<Table>("value")
            .unwrap()
            .get::<i64>("value")
            .unwrap(),
        7
    );
}

use poe_optimizer_data::game_data::bundled_snapshot;
use poe_optimizer_data::modifier_parser::{
    ModifierParserCatalog, ModifierParserData, ParserCallbackId, ParserDictionary as D,
    ParserFactoryDisposition as Disposition, ParserFactoryExpr as E, ParserFactoryField as F,
    ParserFactoryLiteral as L, ParserTable, ParserTableId, ParserValue as P,
};
use poe_optimizer_engine::lua_pattern::MatchBudget;
use poe_optimizer_engine::modifier_parser::{CompiledModifierParser, ParserError};
use std::collections::BTreeMap;

fn clear_dictionaries(data: &mut ModifierParserData) {
    for id in data.dictionaries.values() {
        data.tables[id.0 as usize - 1] = ParserTable::default();
    }
}
fn alias(data: &mut ModifierParserData, pattern: &str, callback: ParserCallbackId) {
    let id = data.dictionaries[&D::Special];
    data.tables[id.0 as usize - 1]
        .fields
        .insert(pattern.into(), P::Callback(callback));
}
fn pattern(index: usize) -> String {
    format!("^__factory_{index:05} (.-)|(.-)|(.-)|(.-)|(.-)$")
}
fn input(index: usize, captures: &[u8]) -> Vec<u8> {
    [format!("__factory_{index:05} ").as_bytes(), captures].concat()
}
fn compare(source: &FactorySource, native: &CompiledModifierParser, line: &[u8]) -> bool {
    let original = source.public.raw(line);
    let result = native.parse(line, &mut MatchBudget::default());
    match (original, result) {
        (Ok(expected), Ok(actual)) => {
            assert_eq!(
                source.public.capture(expected).unwrap(),
                native_observer::capture(&actual, native.catalog()).unwrap(),
                "{line:?}"
            );
            true
        }
        (Err(_), Err(ParserError::SourceError(_))) => false,
        (original, actual) => panic!(
            "factory source/native mismatch {line:?}: original={original:?}, native={actual:?}"
        ),
    }
}

#[test]
fn every_pure_closure_matches_original_through_labelled_special_aliases() {
    let snapshot = bundled_snapshot().unwrap();
    let catalog = snapshot.modifier_parser();
    let source = FactorySource::new();
    let mut originals = BTreeMap::new();
    let mut families = BTreeMap::<String, usize>::new();
    // These are the original callback call-site tables, not a callback/item ID
    // whitelist. The final equality requires coverage of EVERY Pure disposition.
    for family in [D::Special, D::ModTag, D::PreFlag] {
        let original = source.dictionary(family.source_name());
        for (key, value) in &catalog.dictionary(family).fields {
            if let P::Callback(id) = value
                && matches!(catalog.factory(*id), Some(Disposition::Pure(_)))
            {
                let function = original.get::<Function>(key.as_str()).unwrap();
                let info = function.info();
                let poe_optimizer_data::modifier_parser::ParserCallbackKind::Lua { source: span } =
                    &catalog.callback(*id).unwrap().kind
                else {
                    panic!("Pure builtin")
                };
                assert_eq!(
                    info.source.as_deref(),
                    Some(format!("@{}", span.path).as_str())
                );
                assert_eq!(info.line_defined, Some(span.line as usize));
                assert_eq!(info.last_line_defined, Some(span.end_line as usize));
                if let Some(prior) = originals.insert(*id, function.clone()) {
                    assert_eq!(prior.to_pointer(), function.to_pointer());
                } else {
                    *families.entry(family.source_name().into()).or_default() += 1;
                }
            }
        }
    }
    let all_pure = catalog
        .data()
        .factories
        .iter()
        .filter_map(|(id, f)| matches!(f, Disposition::Pure(_)).then_some(*id))
        .collect::<Vec<_>>();
    assert_eq!(
        originals.keys().copied().collect::<Vec<_>>(),
        all_pure,
        "complete source closure coverage"
    );
    let mut data = catalog.data().clone();
    clear_dictionaries(&mut data);
    source.clear_specials();
    for (index, (id, callback)) in originals.iter().enumerate() {
        let pattern = pattern(index);
        alias(&mut data, &pattern, *id);
        source.add(&pattern, callback.clone());
    }
    let native = CompiledModifierParser::new(&ModifierParserCatalog::new(data).unwrap()).unwrap();
    let captures = [
        b"12|34|56|78|90".as_slice(),
        b"-0|-0|0|0|0",
        b"1e309|2|3|4|5",
        b"word|other|third|fourth|fifth",
        b"||||",
        b"\xff\0\x80|a|b|c|d",
    ];
    let mut successes = 0;
    let mut errors = 0;
    for (index, (id, _)) in originals.iter().enumerate() {
        for captures in captures {
            if compare(&source, &native, &input(index, captures)) {
                successes += 1;
            } else {
                errors += 1;
            }
        }
        assert!(matches!(
            native.catalog().factory(*id),
            Some(Disposition::Pure(_))
        ));
    }
    eprintln!(
        "All original Pure closures: {} across {families:?}, {successes} exact public graphs, {errors} matching source errors; aliases are test-owned selection controls",
        originals.len()
    );
}

#[derive(Clone)]
struct Operand {
    lua: String,
    expr: E,
}
impl Operand {
    fn literal(lua: &str, value: L) -> Self {
        Self {
            lua: lua.into(),
            expr: E::Literal(value),
        }
    }
    fn argument(lua: &str, index: u16) -> Self {
        Self {
            lua: lua.into(),
            expr: E::Argument(index),
        }
    }
}
fn constructed(arguments: &[Operand]) -> Operand {
    Operand {
        lua: format!(
            "mod({})",
            arguments
                .iter()
                .map(|v| v.lua.as_str())
                .collect::<Vec<_>>()
                .join(",")
        ),
        expr: E::CreateMod {
            args: arguments.iter().map(|v| v.expr.clone()).collect(),
        },
    }
}
fn list(value: Operand) -> Operand {
    Operand {
        lua: format!("{{{}}}", value.lua),
        expr: E::Table(vec![F::List(value.expr)]),
    }
}
fn uses_constructor(expr: &E) -> bool {
    match expr {
        E::CreateMod { .. } => true,
        E::Negate(v) => uses_constructor(v),
        E::Table(fields) => fields.iter().any(|v| {
            uses_constructor(match v {
                F::List(v) | F::Named { value: v, .. } => v,
            })
        }),
        _ => false,
    }
}
fn install_fixture(data: &mut ModifierParserData, id: ParserCallbackId, body: E) {
    let Disposition::Pure(factory) = data.factories.get_mut(&id).unwrap() else {
        panic!("Pure fixture binding")
    };
    factory.parameter_count = 6;
    if !uses_constructor(&body) {
        factory.provenance.constructor = None;
    }
    factory.body = body;
}
fn constructor_ids(data: &ModifierParserData) -> Vec<ParserCallbackId> {
    data.factories
        .iter()
        .filter_map(|(id, disposition)| {
            matches!(disposition,Disposition::Pure(f) if f.provenance.constructor.is_some())
                .then_some(*id)
        })
        .collect()
}

#[test]
fn caller_recipe_matrix_matches_original_constructor_and_public_copy_without_normalizing_values() {
    let source = FactorySource::new();
    source.clear_specials();
    let snapshot = bundled_snapshot().unwrap();
    let mut data = snapshot.modifier_parser().data().clone();
    clear_dictionaries(&mut data);
    let ids = constructor_ids(&data);
    let root = data.policy.mod_flags;
    let shared_id = ParserTableId(data.tables.len() as u32 + 1);
    data.tables.push(ParserTable {
        fields: BTreeMap::from([
            ("type".into(), P::Text("CallerTag".into())),
            ("value".into(), P::Number(7.0)),
        ]),
        ..Default::default()
    });
    data.tables[root.0 as usize - 1]
        .fields
        .insert("__FactoryShared".into(), P::Table(shared_id));
    let shared = source.public.source.lua.create_table().unwrap();
    shared.set("type", "CallerTag").unwrap();
    shared.set("value", 7).unwrap();
    source
        .public
        .source
        .lua
        .globals()
        .get::<Table>("ModFlag")
        .unwrap()
        .set("__FactoryShared", shared)
        .unwrap();
    let nil = Operand::literal("nil", L::Nil);
    let false_ = Operand::literal("false", L::Boolean(false));
    let seven = Operand::literal("7", L::Number(7.0));
    let zero = Operand::literal("-0.0", L::Number(-0.0));
    let numeric_string = Operand::literal("'7'", L::Text("7".into()));
    let source_text = Operand::literal("'source'", L::Text("source".into()));
    let tag = Operand {
        lua: "ModFlag.__FactoryShared".into(),
        expr: E::ConstantField {
            table: root,
            key: "__FactoryShared".into(),
        },
    };
    let missing = Operand {
        lua: "ModFlag.__FactoryMissing".into(),
        expr: E::ConstantField {
            table: root,
            key: "__FactoryMissing".into(),
        },
    };
    let num = Operand::argument("num", 0);
    let raw = Operand::argument("a", 1);
    let tails = vec![
        vec![],
        vec![nil.clone()],
        vec![seven.clone()],
        vec![nil.clone(), seven.clone()],
        vec![tag.clone(), seven.clone()],
        vec![tag.clone(), tag.clone(), seven.clone()],
        vec![nil.clone(), numeric_string.clone()],
        vec![nil.clone(), nil.clone(), nil.clone(), tag.clone()],
        vec![source_text.clone(), tag.clone()],
        vec![source_text.clone(), seven.clone(), tag.clone()],
        vec![
            source_text.clone(),
            nil.clone(),
            false_.clone(),
            tag.clone(),
        ],
        vec![nil.clone(), nil.clone(), seven.clone(), tag.clone()],
        vec![
            false_.clone(),
            zero.clone(),
            tag.clone(),
            nil.clone(),
            tag.clone(),
        ],
        vec![
            source_text.clone(),
            zero.clone(),
            num.clone(),
            nil.clone(),
            false_.clone(),
            tag.clone(),
        ],
        vec![
            raw.clone(),
            num.clone(),
            num.clone(),
            tag.clone(),
            nil.clone(),
            numeric_string.clone(),
        ],
        vec![
            nil.clone(),
            tag.clone(),
            tag.clone(),
            nil.clone(),
            tag.clone(),
            nil.clone(),
        ],
    ];
    let mut cases = vec![
        ("no arguments", constructed(&[])),
        (
            "explicit absent fields",
            constructed(&[nil.clone(), nil.clone(), nil.clone()]),
        ),
        (
            "boolean fields",
            constructed(&[false_.clone(), false_.clone(), false_.clone()]),
        ),
        (
            "raw generic fields",
            constructed(&[raw.clone(), Operand::argument("b", 2), num.clone()]),
        ),
        (
            "generic shared tables",
            constructed(&[
                tag.clone(),
                tag.clone(),
                tag.clone(),
                tag.clone(),
                tag.clone(),
            ]),
        ),
        (
            "missing constant field",
            constructed(&[
                missing.clone(),
                missing.clone(),
                missing.clone(),
                missing.clone(),
                tag.clone(),
            ]),
        ),
        ("nil public result", nil.clone()),
        (
            "empty public result",
            Operand {
                lua: "{}".into(),
                expr: E::Table(vec![]),
            },
        ),
        (
            "ordered named and sparse list",
            Operand {
                lua: "{a,named=false,nil,kind='X',b,nil}".into(),
                expr: E::Table(vec![
                    F::List(raw.expr.clone()),
                    F::Named {
                        key: "named".into(),
                        value: false_.expr.clone(),
                    },
                    F::List(nil.expr.clone()),
                    F::Named {
                        key: "kind".into(),
                        value: E::Literal(L::Text("X".into())),
                    },
                    F::List(E::Argument(2)),
                    F::List(nil.expr.clone()),
                ]),
            },
        ),
        (
            "unary numeric argument",
            list(constructed(&[
                raw.clone(),
                false_.clone(),
                Operand {
                    lua: "-num".into(),
                    expr: E::Negate(Box::new(num.expr.clone())),
                },
            ])),
        ),
        (
            "unary raw coercion",
            list(constructed(&[
                raw.clone(),
                false_.clone(),
                Operand {
                    lua: "-a".into(),
                    expr: E::Negate(Box::new(raw.expr.clone())),
                },
            ])),
        ),
    ];
    for tail in tails {
        let mut args = vec![raw.clone(), Operand::argument("b", 2), num.clone()];
        args.extend(tail);
        cases.push(("absolute vararg positions", list(constructed(&args))));
    }
    let inner = constructed(&[
        source_text.clone(),
        false_.clone(),
        num.clone(),
        tag.clone(),
    ]);
    cases.push((
        "nested factory",
        list(constructed(&[
            raw.clone(),
            false_.clone(),
            inner,
            tag.clone(),
        ])),
    ));
    for (lua, sentinel) in [
        (
            "(1/0)",
            poe_optimizer_data::modifier_parser::ParserNonFinite::PositiveInfinity,
        ),
        (
            "(-1/0)",
            poe_optimizer_data::modifier_parser::ParserNonFinite::NegativeInfinity,
        ),
        (
            "(0/0)",
            poe_optimizer_data::modifier_parser::ParserNonFinite::Nan,
        ),
    ] {
        let value = Operand::literal(lua, L::NonFinite(sentinel));
        cases.push((
            "nonfinite generic fields",
            list(constructed(&[
                value.clone(),
                value.clone(),
                value.clone(),
                nil.clone(),
                value.clone(),
                value,
                tag.clone(),
            ])),
        ));
    }
    for (index, (_, case)) in cases.iter().enumerate() {
        install_fixture(&mut data, ids[index], case.expr.clone());
        let alias_pattern = pattern(index);
        alias(&mut data, &alias_pattern, ids[index]);
        source.add(
            &alias_pattern,
            source.synthetic(&format!("return {}", case.lua)),
        );
    }
    // These explicitly edited recipes are caller-definition fixtures. Their Lua
    // expressions use the genuine original constructor and unchanged public call
    // protocol; they are not claimed to be extracted source recipes/build lines.
    let native = CompiledModifierParser::new(&ModifierParserCatalog::new(data).unwrap()).unwrap();
    let mut successes = 0;
    let mut errors = 0;
    for (index, (label, _)) in cases.iter().enumerate() {
        for captures in [
            b"12|BASE|value|source|tag".as_slice(),
            b"-0|BASE|0|0|0",
            b"1e309|BASE|a|b|c",
            b"word|TYPE|a|b|c",
            b"||||",
            b"\xff\0\x80|\x80\0\xff|a|b|c",
        ] {
            let _ = label;
            if compare(&source, &native, &input(index, captures)) {
                successes += 1;
            } else {
                errors += 1;
            }
        }
    }
    let nil = native
        .parse(&input(6, b"||||"), &mut MatchBudget::default())
        .unwrap();
    let empty = native
        .parse(&input(7, b"||||"), &mut MatchBudget::default())
        .unwrap();
    assert!(nil.modifiers.is_none());
    assert_eq!(nil.extra, None);
    let empty = empty.modifiers.unwrap();
    assert!(empty.fields.is_empty() && empty.indexed.is_empty());
    eprintln!(
        "{} explicit caller recipes: {successes} exact graphs, {errors} matching source errors",
        cases.len()
    );
}

#[test]
fn original_special_patterns_select_pure_factories_with_exact_public_graphs() {
    let snapshot = bundled_snapshot().unwrap();
    let catalog = snapshot.modifier_parser();
    let native = CompiledModifierParser::new(catalog).unwrap();
    let source = FactorySource::new();
    let generate: Function = source
        .public
        .source
        .lua
        .load(include_str!("support/lua_pattern_witness.lua"))
        .eval()
        .unwrap();
    let scan = factory_source::upvalue(&source.public.source.lua, &source.internal, "scan")
        .as_function()
        .unwrap()
        .clone();
    let mut by_pointer = BTreeMap::new();
    let mut cases = std::collections::BTreeSet::new();
    for (pattern, value) in &catalog.dictionary(D::Special).fields {
        if let P::Callback(id) = value {
            let function = source.special.get::<Function>(pattern.as_str()).unwrap();
            by_pointer.insert(function.to_pointer() as usize, *id);
            if matches!(catalog.factory(*id), Some(Disposition::Pure(_))) {
                let witness: mlua::LuaString = generate.call(pattern.as_str()).unwrap();
                cases.insert(witness.as_bytes().to_vec());
            }
        }
    }
    let mut selected = std::collections::BTreeSet::new();
    let mut paired = 0;
    let mut source_errors = 0;
    let mut skipped = BTreeMap::<String, usize>::new();
    for line in &cases {
        let (selected_value, remainder, _): (Value, mlua::LuaString, Value) = scan
            .call((source.text(line), source.special.clone()))
            .unwrap();
        let Value::Function(function) = selected_value else {
            *skipped.entry("non-callback selection".into()).or_default() += 1;
            continue;
        };
        let id = by_pointer[&(function.to_pointer() as usize)];
        if !remainder.as_bytes().is_empty()
            || !matches!(catalog.factory(id), Some(Disposition::Pure(_)))
        {
            *skipped
                .entry("partial or unsupported callback selection".into())
                .or_default() += 1;
            continue;
        }
        if matches!(
            native.parse(line, &mut MatchBudget::default()),
            Err(ParserError::Ambiguous { .. })
        ) {
            *skipped
                .entry("explicit ambiguous dictionary winner".into())
                .or_default() += 1;
            continue;
        }
        if compare(&source, &native, line) {
            paired += 1;
        } else {
            source_errors += 1;
        }
        selected.insert(id);
    }
    eprintln!(
        "Actual original Special patterns: {} generated inputs; {} distinct actual Pure selections, {paired} exact public outputs, {source_errors} matching errors; unpaired {skipped:?}",
        cases.len(),
        selected.len()
    );
    assert!(
        selected.len() > 850,
        "actual dictionary selection coverage regressed"
    );
}

#[test]
fn caller_captured_scalars_are_lazy_and_unrepresentable_selected_shapes_stay_deferred() {
    use poe_optimizer_data::modifier_parser::{ParserNonFinite as N, ParserUpvalue};
    let snapshot = bundled_snapshot().unwrap();
    let mut data = snapshot.modifier_parser().data().clone();
    let ids = constructor_ids(&data);
    clear_dictionaries(&mut data);
    let source = FactorySource::new();
    source.clear_specials();
    let values = vec![
        (P::Nil, Value::Nil),
        (P::Boolean(false), Value::Boolean(false)),
        (P::Number(-0.0), Value::Number(-0.0)),
        (P::Number(17.25), Value::Number(17.25)),
        (
            P::Text("captured value λ".into()),
            source.text("captured value λ"),
        ),
        (
            P::NonFinite(N::PositiveInfinity),
            Value::Number(f64::INFINITY),
        ),
        (
            P::NonFinite(N::NegativeInfinity),
            Value::Number(f64::NEG_INFINITY),
        ),
        (
            P::NonFinite(N::Nan),
            Value::Number(f64::from_bits(0xfff8_0000_0000_0000)),
        ),
        (
            P::Table(data.policy.mod_flags),
            Value::Table(
                source
                    .public
                    .source
                    .lua
                    .globals()
                    .get::<Table>("ModFlag")
                    .unwrap(),
            ),
        ),
        (
            P::Callback(ids[0]),
            Value::Function(source.synthetic("return {}")),
        ),
    ];
    let mut selected_cases = vec![];
    for (index, (native_value, source_value)) in values.iter().enumerate() {
        for used in [false, true] {
            let case = index * 2 + usize::from(used);
            let id = ids[case];
            let callback = &mut data.callbacks[id.0 as usize - 1];
            let slot = callback.upvalues.len() as u16;
            callback.upvalues.push(ParserUpvalue {
                name: "caller_fixture_scalar".into(),
                value: native_value.clone(),
            });
            let body = if used {
                E::Table(vec![F::List(E::CapturedScalar { upvalue: slot })])
            } else {
                E::Table(vec![])
            };
            install_fixture(&mut data, id, body);
            alias(&mut data, &pattern(case), id);
            let code = if used {
                "return function(captured) return function(...) return {captured} end end"
            } else {
                "return function(captured) return function(...) return {} end end"
            };
            let make: Function = source
                .public
                .source
                .lua
                .load(code)
                .set_name("@caller-captured-scalar-fixture")
                .eval()
                .unwrap();
            source.add(&pattern(case), make.call(source_value.clone()).unwrap());
            selected_cases.push((
                case,
                used && matches!(native_value, P::Table(_) | P::Callback(_)),
            ));
        }
    }
    let native = CompiledModifierParser::new(&ModifierParserCatalog::new(data).unwrap()).unwrap();
    for (case, deferred) in selected_cases {
        let line = input(case, b"bad\xff\0|a|b|c|d");
        if deferred {
            let original = source.public.raw(&line).unwrap();
            assert!(original.front().unwrap().is_table());
            assert!(matches!(
                native.parse(&line, &mut MatchBudget::default()),
                Err(ParserError::Deferred {
                    stage: "special factory captured value shape",
                    ..
                })
            ));
        } else {
            assert!(compare(&source, &native, &line));
        }
    }
}

#[test]
fn pure_factories_do_not_enable_jewel_protocol_or_unsupported_source() {
    let snapshot = bundled_snapshot().unwrap();
    let mut data = snapshot.modifier_parser().data().clone();
    let source = FactorySource::new();
    let mut cases = vec![];
    // Ordinary Prefix/ModTag protocols have their own full source parity target.
    let unsupported = source
        .special
        .clone()
        .pairs::<mlua::LuaString, Value>()
        .map(Result::unwrap)
        .find_map(|(pattern, value)| {
            let Value::Function(callback) = value else {
                return None;
            };
            let id = match snapshot
                .modifier_parser()
                .dictionary(D::Special)
                .fields
                .get(pattern.to_str().unwrap().as_ref())?
            {
                P::Callback(id) => *id,
                _ => return None,
            };
            matches!(
                data.factories.get(&id),
                Some(Disposition::Unsupported { .. })
            )
            .then_some((id, callback))
        })
        .unwrap();
    alias(&mut data, "^__unsupported__$", unsupported.0);
    source.add("^__unsupported__$", unsupported.1);
    cases.push((
        b"__unsupported__".to_vec(),
        "special callback",
        Some(unsupported.0),
    ));
    let pure = constructor_ids(&data)[0];
    // A selected jewel capture is a different call convention, even if its raw
    // injected value happens to carry an admitted Special factory identity.
    data.tables[data.dictionaries[&D::Jewel].0 as usize - 1]
        .fields
        .insert("^__jewel (.+)$".into(), P::Callback(pure));
    cases.push((b"__jewel captured".to_vec(), "jewel capture factory", None));
    let native = CompiledModifierParser::new(&ModifierParserCatalog::new(data).unwrap()).unwrap();
    for (text, expected_stage, id) in cases {
        assert!(
            matches!(native.parse(&text,&mut MatchBudget::default()),Err(ParserError::Deferred{stage,callback}) if stage==expected_stage&&callback==id),
            "{text:?}"
        );
    }
    assert!(
        native
            .parse(b"+7 to maximum Life", &mut MatchBudget::default())
            .unwrap()
            .modifiers
            .is_some()
    );
}

fn recipe_operations(expr: &E, operations: &mut std::collections::BTreeSet<&'static str>) {
    match expr {
        E::Literal(L::Nil) => {
            operations.insert("nil literal");
        }
        E::Literal(L::Boolean(_)) => {
            operations.insert("boolean literal");
        }
        E::Literal(L::Number(_)) => {
            operations.insert("number literal");
        }
        E::Literal(L::Text(_)) => {
            operations.insert("text literal");
        }
        E::Literal(L::NonFinite(_)) => {
            operations.insert("nonfinite literal");
        }
        E::Argument(_) => {
            operations.insert("argument");
        }
        E::CapturedScalar { .. } => {
            operations.insert("captured scalar");
        }
        E::ConstantField { .. } => {
            operations.insert("constant field");
        }
        E::Negate(v) => {
            operations.insert("negate");
            recipe_operations(v, operations);
        }
        E::Concat { left, right } => {
            operations.insert("concat");
            recipe_operations(left, operations);
            recipe_operations(right, operations);
        }
        E::FirstToUpper { value, .. } => {
            operations.insert("firstToUpper");
            recipe_operations(value, operations);
        }
        E::Table(fields) => {
            operations.insert("table");
            for field in fields {
                match field {
                    F::Named { value, .. } => {
                        operations.insert("named field");
                        recipe_operations(value, operations);
                    }
                    F::List(value) => {
                        operations.insert("list field");
                        recipe_operations(value, operations);
                    }
                }
            }
        }
        E::CreateMod { args } => {
            operations.insert("createMod");
            for arg in args {
                recipe_operations(arg, operations);
            }
        }
    }
}

#[test]
fn original_factory_operations_execute_in_live_jit_traces_without_public_cache_hits() {
    let snapshot = bundled_snapshot().unwrap();
    let catalog = snapshot.modifier_parser();
    let source = FactorySource::new();
    let mut candidates = BTreeMap::new();
    let mut all_operations = std::collections::BTreeSet::new();
    for (pattern, value) in &catalog.dictionary(D::Special).fields {
        if let P::Callback(id) = value
            && let Some(Disposition::Pure(factory)) = catalog.factory(*id)
        {
            let mut operations = std::collections::BTreeSet::new();
            recipe_operations(&factory.body, &mut operations);
            all_operations.extend(operations.iter().copied());
            candidates.insert(
                *id,
                (
                    source.special.get::<Function>(pattern.as_str()).unwrap(),
                    operations,
                ),
            );
        }
    }
    let mut uncovered = all_operations.clone();
    let mut selected = vec![];
    while !uncovered.is_empty() {
        let id = *candidates
            .iter()
            .max_by_key(|(_, (_, ops))| ops.intersection(&uncovered).count())
            .unwrap()
            .0;
        let (callback, operations) = candidates.remove(&id).unwrap();
        assert!(!operations.is_disjoint(&uncovered));
        for operation in &operations {
            uncovered.remove(operation);
        }
        selected.push((id, callback, operations));
    }
    source.clear_specials();
    let mut data = catalog.data().clone();
    clear_dictionaries(&mut data);
    let callbacks = source.public.source.lua.create_table().unwrap();
    let mut cold = vec![];
    for (index, (id, callback, _)) in selected.iter().enumerate() {
        callbacks.set(index + 1, callback.clone()).unwrap();
        source.add(&pattern(index), callback.clone());
        alias(&mut data, &pattern(index), *id);
        cold.push(
            source
                .public
                .capture(source.public.raw(&input(index, b"12|34|56|78|90")).unwrap())
                .unwrap(),
        );
    }
    let native = CompiledModifierParser::new(&ModifierParserCatalog::new(data).unwrap()).unwrap();
    let observed: Table = source
        .public
        .source
        .lua
        .load(include_str!("support/callback_factories_warm.lua"))
        .set_name("@test-only-direct-factory-warm-driver")
        .call(callbacks)
        .unwrap();
    assert_eq!(
        observed.get::<usize>("executions").unwrap(),
        selected.len() * 256
    );
    let results = observed.get::<Table>("results").unwrap();
    let traces = observed
        .get::<Table>("live")
        .unwrap()
        .sequence_values::<Table>()
        .map(Result::unwrap)
        .map(|v| {
            (
                v.get::<String>("source").unwrap(),
                v.get::<usize>("line").unwrap(),
            )
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        traces.contains(&("@src/Modules/ModTools.lua".into(), 57)),
        "original createMod not in a live trace: {traces:?}"
    );
    for (index, (_, callback, operations)) in selected.iter().enumerate() {
        let info = callback.info();
        assert!(
            traces.contains(&(info.source.unwrap(), info.line_defined.unwrap())),
            "factory operations {operations:?} absent from live trace: {traces:?}"
        );
        let warmed = source
            .public
            .capture(MultiValue::from_vec(vec![
                results.get::<Value>(index + 1).unwrap(),
            ]))
            .unwrap();
        assert_eq!(warmed, cold[index], "cold/warm operations {operations:?}");
        let native_result = native
            .parse(
                &input(index, b"12|34|56|78|90"),
                &mut MatchBudget::default(),
            )
            .unwrap();
        assert_eq!(
            warmed,
            native_observer::capture(&native_result, native.catalog()).unwrap(),
            "warm/native operations {operations:?}"
        );
    }
    eprintln!(
        "Direct original warm factories: {} representatives, {} actual factory calls, {} live source-function trace identities; operation coverage {all_operations:?}",
        selected.len(),
        selected.len() * 256,
        traces.len()
    );
}

#[test]
fn native_special_capture_packing_and_factory_item_metadata_boundaries_match_source() {
    use poe_optimizer_import::item_loading::{
        DependencyResult, ItemLoadProvider, NativeModifierParserProvider, ParseRequest,
    };
    let snapshot = bundled_snapshot().unwrap();
    let mut data = snapshot.modifier_parser().data().clone();
    let ids = constructor_ids(&data);
    clear_dictionaries(&mut data);
    let mut source = FactorySource::new();
    source.clear_specials();
    source
        .public
        .bind_builtin_symbols(data.callbacks.iter().filter_map(|row| match &row.kind {
            poe_optimizer_data::modifier_parser::ParserCallbackKind::Builtin { symbol } => {
                Some(symbol.as_str())
            }
            _ => None,
        }));
    install_fixture(
        &mut data,
        ids[0],
        E::Table((0..6).map(|i| F::List(E::Argument(i))).collect()),
    );
    for pattern in [
        "^__packed__$",
        "^__packed (.*)$",
        "^__packed_position ()(.+)$",
        "^__packed_six (.) (.) (.) (.) (.) (.)$",
    ] {
        alias(&mut data, pattern, ids[0]);
        source.add(pattern, source.synthetic("return {num,a,b,c,d,e}"));
    }
    let root = data.policy.mod_flags;
    let constructor = match data.factories.get(&ids[1]).unwrap() {
        Disposition::Pure(f) => f.provenance.constructor.unwrap(),
        _ => unreachable!(),
    };
    data.tables[root.0 as usize - 1]
        .fields
        .insert("__FactoryFunction".into(), P::Callback(constructor));
    source
        .public
        .source
        .lua
        .globals()
        .get::<Table>("ModFlag")
        .unwrap()
        .set("__FactoryFunction", source.create_mod.clone())
        .unwrap();
    let cases = [
        ("nil", E::Literal(L::Nil), "nil"),
        ("empty", E::Table(vec![]), "{}"),
        (
            "number",
            E::Table(vec![F::List(E::CreateMod {
                args: vec![
                    E::Literal(L::Text("Caller".into())),
                    E::Literal(L::Text("BASE".into())),
                    E::Argument(0),
                ],
            })]),
            "{mod('Caller','BASE',num)}",
        ),
        (
            "function",
            E::Table(vec![F::List(E::CreateMod {
                args: vec![
                    E::Literal(L::Text("Caller".into())),
                    E::Literal(L::Text("LIST".into())),
                    E::ConstantField {
                        table: root,
                        key: "__FactoryFunction".into(),
                    },
                ],
            })]),
            "{mod('Caller','LIST',ModFlag.__FactoryFunction)}",
        ),
    ];
    for (i, (name, body, lua)) in cases.into_iter().enumerate() {
        install_fixture(&mut data, ids[i + 1], body);
        let pattern = format!("^__metadata_{name} (.*)$");
        alias(&mut data, &pattern, ids[i + 1]);
        source.add(&pattern, source.synthetic(&format!("return {lua}")));
    }
    let native = std::sync::Arc::new(
        CompiledModifierParser::new(&ModifierParserCatalog::new(data).unwrap()).unwrap(),
    );
    for line in [
        b"__packed__".as_slice(),
        b"__packed 12",
        b"__packed ",
        b"__packed \xff\0\x80",
        b"__packed_position abc",
        b"__packed_six 1 2 3 4 5 6",
    ] {
        assert!(compare(&source, &native, line));
    }
    let mut provider = NativeModifierParserProvider::from_compiled(native.clone());
    for (line, expected) in [
        ("__metadata_nil ignored", "nil"),
        ("__metadata_empty ignored", "empty"),
        ("__metadata_number 12", "number"),
        ("__metadata_number 1e309", "nonfinite"),
        ("__metadata_function ignored", "function"),
    ] {
        if expected == "function" {
            // Opaque function values refer to the extraction-instance closure.
            // Full ModTools captures select/type; the preserved isolated helper
            // descriptor has no lexical upvalues. Do not normalize those graphs.
            let original = source
                .public
                .capture(source.public.raw(line.as_bytes()).unwrap())
                .unwrap();
            let actual = native
                .parse(line.as_bytes(), &mut MatchBudget::default())
                .unwrap();
            let actual = native_observer::capture(&actual, native.catalog()).unwrap();
            assert_eq!(original.callbacks[0].first_line, 57);
            assert_eq!(original.callbacks[0].last_line, 91);
            assert_eq!(
                original.callbacks[0]
                    .upvalues
                    .iter()
                    .map(|(name, _)| name.as_slice())
                    .collect::<Vec<_>>(),
                vec![b"select".as_slice(), b"type"]
            );
            assert!(actual.callbacks[0].upvalues.is_empty());
            assert_ne!(
                original, actual,
                "opaque full-module and extraction-instance closure graphs must remain distinct"
            );
        } else {
            assert!(compare(&source, &native, line.as_bytes()));
        }
        let request = ParseRequest {
            sequence: 1,
            line_index: Some(1),
            origin: None,
            text: line.into(),
            combined: false,
        };
        let result = provider.parse_modifier(&request);
        match expected {
            "nil" => {
                let DependencyResult::Available(result) = result else {
                    panic!("{result:?}")
                };
                assert!(result.modifiers.is_none());
                assert!(result.extra.is_none());
            }
            "empty" => {
                let DependencyResult::Available(result) = result else {
                    panic!("{result:?}")
                };
                assert_eq!(result.modifiers, Some(vec![]));
                assert!(result.extra.is_none());
            }
            "number" => {
                let DependencyResult::Available(result) = result else {
                    panic!("{result:?}")
                };
                assert_eq!(
                    result.modifiers.unwrap()[0].fields["value"].as_f64(),
                    Some(12.0)
                );
            }
            "nonfinite" => assert!(
                matches!(result,DependencyResult::Unavailable(ref why) if why.contains("non-finite"))
            ),
            "function" => assert!(
                matches!(result,DependencyResult::Unavailable(ref why) if why.contains("captured-state"))
            ),
            _ => unreachable!(),
        }
    }
    // Independently reconstruct the same authenticated standalone function block
    // to observe the descriptor's true extraction-instance boundary. This uses
    // the original body, not a translated constructor, and preserves module line
    // numbers while restoring the host's original modLib immediately afterward.
    let text = runtime::verified("src/Modules/ModTools.lua").unwrap();
    let block = text
        .lines()
        .skip(56)
        .take(35)
        .collect::<Vec<_>>()
        .join("\n");
    let globals = source.public.source.lua.globals();
    let prior: Table = globals.get("modLib").unwrap();
    globals
        .set("modLib", source.public.source.lua.create_table().unwrap())
        .unwrap();
    source
        .public
        .source
        .lua
        .load(format!("{}{block}", "\n".repeat(56)))
        .set_name("@src/Modules/ModTools.lua")
        .exec()
        .unwrap();
    let isolated = globals
        .get::<Table>("modLib")
        .unwrap()
        .get::<Function>("createMod")
        .unwrap();
    globals.set("modLib", prior).unwrap();
    assert_eq!(isolated.info().num_upvalues, 0);
    globals
        .get::<Table>("ModFlag")
        .unwrap()
        .set("__FactoryFunction", isolated)
        .unwrap();
    let text = "__metadata_function extraction-instance";
    assert!(compare(&source, &native, text.as_bytes()));
    let request = ParseRequest {
        sequence: 2,
        line_index: Some(1),
        origin: None,
        text: text.into(),
        combined: false,
    };
    assert!(
        matches!(provider.parse_modifier(&request),DependencyResult::Unavailable(ref why) if why.contains("captured-state"))
    );
    eprintln!(
        "Opaque constructor observation: full ModTools captures select/type; preserved isolated descriptor has zero upvalues; both function-valued item results remain unavailable"
    );
}
