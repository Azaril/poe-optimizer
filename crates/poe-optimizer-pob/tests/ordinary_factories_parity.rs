//! Whole-original-parser parity for the independently scoped ordinary factory ABI.
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
use mlua::{Function, MultiValue, Table, Value};
use ordinary_source::{OrdinarySource, clear};
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
use public_source::{Atom, Observation};
use std::collections::{BTreeMap, BTreeSet};
fn compare(source: &OrdinarySource, native: &CompiledModifierParser, text: &[u8]) -> bool {
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
            "ordinary source/native mismatch {text:?}: source={expected:?}; native={actual:?}"
        ),
    }
}
fn row(data: &mut ModifierParserData, dictionary: D, pattern: &str, value: P) {
    data.tables[data.dictionaries[&dictionary].0 as usize - 1]
        .fields
        .insert(pattern.into(), value);
}
fn empty_table(data: &mut ModifierParserData) -> P {
    let id = ParserTableId(data.tables.len() as u32 + 1);
    data.tables.push(ParserTable::default());
    P::Table(id)
}
fn pure_ids(data: &ModifierParserData, family: D) -> Vec<ParserCallbackId> {
    data.tables[data.dictionaries[&family].0 as usize - 1]
        .fields
        .values()
        .filter_map(|v| match v {
            P::Callback(id) if matches!(data.factories.get(id), Some(F::Pure(_))) => Some(*id),
            _ => None,
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
fn set_recipe(data: &mut ModifierParserData, id: ParserCallbackId, body: E) {
    let F::Pure(f) = data.factories.get_mut(&id).unwrap() else {
        panic!("Pure fixture")
    };
    f.parameter_count = 6;
    f.provenance.constructor = None;
    f.body = body;
}
fn metadata_recipe() -> E {
    E::Table(vec![Field::Named {
        key: "tag".into(),
        value: E::Table(
            std::iter::once(Field::Named {
                key: "type".into(),
                value: E::Literal(L::Text("CallerTag".into())),
            })
            .chain((0..6).map(|i| Field::Named {
                key: format!("arg{}", i + 1),
                value: E::Argument(i),
            }))
            .collect(),
        ),
    }])
}
const METADATA_BODY: &str =
    "return {tag={type='CallerTag',arg1=num,arg2=a,arg3=b,arg4=c,arg5=d,arg6=e}}";
fn ordinary_fixture() -> (OrdinarySource, ModifierParserData, Vec<ParserCallbackId>) {
    let snapshot = bundled_snapshot().unwrap();
    let mut data = snapshot.modifier_parser().data().clone();
    let ids = pure_ids(&data, D::ModTag);
    let source = OrdinarySource::new();
    for family in [D::PreFlag, D::ModTag] {
        data.tables[data.dictionaries[&family].0 as usize - 1] = ParserTable::default();
        clear(&source.source.dictionary(family.source_name()));
    }
    (source, data, ids)
}

#[test]
fn every_real_pure_prefix_and_both_tag_positions_match_the_unchanged_public_parser() {
    let snapshot = bundled_snapshot().unwrap();
    let catalog = snapshot.modifier_parser();
    let mut data = catalog.data().clone();
    let source = OrdinarySource::new();
    let control = OrdinarySource::new();
    let marker = empty_table(&mut data);
    row(&mut data, D::ModTag, "__ordinary_first__", marker);
    for original in [&source, &control] {
        original
            .source
            .dictionary("modTagList")
            .set(
                "__ordinary_first__",
                original.source.public.source.lua.create_table().unwrap(),
            )
            .unwrap();
    }
    let native = CompiledModifierParser::new(&ModifierParserCatalog::new(data).unwrap()).unwrap();
    let generate: Function = source
        .source
        .public
        .source
        .lua
        .load(include_str!("support/lua_pattern_witness.lua").replace("{55,97,65", "{97,55,65"))
        .eval()
        .unwrap();
    let mut covered = BTreeMap::<(D, bool), BTreeSet<ParserCallbackId>>::new();
    let mut paired = 0;
    for family in [D::PreFlag, D::ModTag] {
        let table = source.source.dictionary(family.source_name());
        let mut wrappers = BTreeMap::new();
        for (pattern, value) in &catalog.dictionary(family).fields {
            let P::Callback(id) = value else { continue };
            if !matches!(catalog.factory(*id), Some(F::Pure(_))) {
                continue;
            }
            let actual = table.get::<Function>(pattern.as_str()).unwrap();
            let label = format!("{}:{}", family.source_name(), id.0);
            let wrapper = wrappers
                .entry(*id)
                .or_insert_with(|| source.wrap(actual, &label))
                .clone();
            table.set(pattern.as_str(), wrapper).unwrap();
            let witness: mlua::LuaString = generate.call(pattern.as_str()).unwrap();
            let witness = witness.as_bytes();
            for second in [false, true] {
                if second && family == D::PreFlag {
                    continue;
                }
                let text = if family == D::PreFlag {
                    [witness.as_ref(), b"7% increased damage"].concat()
                } else if second {
                    [b"+7 to maximum Life __ordinary_first__ ", witness.as_ref()].concat()
                } else {
                    [b"+7 to maximum Life ", witness.as_ref()].concat()
                };
                assert!(compare(&source, &native, &text));
                let calls = source.calls();
                let expected_frames = if family == D::PreFlag {
                    vec![6656]
                } else if second {
                    vec![6684, 6686]
                } else {
                    vec![6675, 6677]
                };
                assert!(
                    calls.iter().any(|call| call.label == label
                        && call
                            .frames
                            .iter()
                            .any(|line| expected_frames.contains(line))),
                    "real callback {label}, second={second}, {calls:?}"
                );
                assert_eq!(
                    source.source.public.observe(&text),
                    control.observe(&text),
                    "observer changed original output"
                );
                covered.entry((family, second)).or_default().insert(*id);
                paired += 1;
            }
        }
    }
    assert_eq!(
        covered[&(D::PreFlag, false)]
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        pure_ids(catalog.data(), D::PreFlag)
    );
    for second in [false, true] {
        assert_eq!(
            covered[&(D::ModTag, second)]
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            pure_ids(catalog.data(), D::ModTag)
        );
    }
    eprintln!(
        "Real ordinary factories: {paired} exact full public outputs; Prefix{}, firstTag{}, secondTag{}",
        covered[&(D::PreFlag, false)].len(),
        covered[&(D::ModTag, false)].len(),
        covered[&(D::ModTag, true)].len()
    );
}

#[test]
fn caller_metadata_recipes_preserve_raw_prefix_and_leading_tag_capture_slots() {
    let (source, mut data, ids) = ordinary_fixture();
    let id = ids[0];
    set_recipe(&mut data, id, metadata_recipe());
    for (family, pattern) in [
        (D::PreFlag, "^__prefix (.-) "),
        (D::PreFlag, "^__missing "),
        (D::PreFlag, "^__position() "),
        (D::PreFlag, "^__six (.)|(.)|(.)|(.)|(.)|(.) "),
        (D::ModTag, " __tag (.*) "),
        (D::ModTag, " __missing "),
        (D::ModTag, " __position() "),
        (D::ModTag, " __six (.)|(.)|(.)|(.)|(.)|(.) "),
    ] {
        row(&mut data, family, pattern, P::Callback(id));
        source
            .source
            .dictionary(family.source_name())
            .set(pattern, source.fixture("target", METADATA_BODY))
            .unwrap();
    }
    let marker = empty_table(&mut data);
    row(&mut data, D::ModTag, " __first", marker);
    source
        .source
        .dictionary("modTagList")
        .set(
            " __first",
            source.source.public.source.lua.create_table().unwrap(),
        )
        .unwrap();
    let native = CompiledModifierParser::new(&ModifierParserCatalog::new(data).unwrap()).unwrap();
    let mut paired = 0;
    let mut errors = 0;
    for second in [false, true] {
        let prefix = if second {
            b"+7 to maximum Life __first".as_slice()
        } else {
            b"+7 to maximum Life"
        };
        for bytes in [
            b"12".as_slice(),
            b"-0",
            b"-0.5",
            b"+.5",
            b"1e2",
            b"1e309",
            b"1x",
            b"x1",
            b"0xap2",
            b"0b101",
            b"nan",
            b"inf",
            b"-inf",
            b"WORD",
            b"",
            b"\xff\0\x80",
            b"\xff1\0",
            b"12\0junk",
            b"\xd9\xa1",
            b" 12 ",
        ] {
            let text = [prefix, b" __tag ", bytes].concat();
            assert!(compare(&source, &native, &text));
            let calls = source.calls();
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0].count, 2);
            assert!(calls[0].frames.iter().any(|line| if second {
                [6684, 6686].contains(line)
            } else {
                [6675, 6677].contains(line)
            }));
            paired += 1;
        }
        for suffix in [b" __missing".as_slice(), b" __position"] {
            assert!(!compare(&source, &native, &[prefix, suffix].concat()));
            assert!(source.calls().is_empty());
            errors += 1;
        }
        assert!(compare(
            &source,
            &native,
            &[prefix, b" __six 1|2|3|4|5|6"].concat()
        ));
        let calls = source.calls();
        assert_eq!(calls[0].count, 6);
        assert_eq!(
            calls[0].arguments.roots.last(),
            Some(&Atom::Bytes(b"5".to_vec()))
        );
        paired += 1;
    }
    for text in [
        b"__prefix 12 +7 to maximum Life".as_slice(),
        b"__prefix 1e309 +7 to maximum Life",
        b"__prefix \xff\0\x80 +7 to maximum Life",
        b"__missing +7 to maximum Life",
        b"__position +7 to maximum Life",
        b"__six 1|2|3|4|5|6 +7 to maximum Life",
        b"__missing no valid modifier form",
    ] {
        assert!(compare(&source, &native, text));
        let calls = source.calls();
        assert!(calls[0].frames.contains(&6656));
        paired += 1;
    }
    eprintln!(
        "Injected ordinary argument metadata: {paired} exact full graphs and{errors} precheck errors; raw counts independently observed"
    );
}

#[test]
fn first_result_truthiness_fresh_second_captures_and_public_retries_match_source() {
    let (source, mut data, ids) = ordinary_fixture();
    for (id, name, body, recipe) in [
        (ids[0], "nil", "return nil", E::Literal(L::Nil)),
        (ids[1], "empty", "return {}", E::Table(vec![])),
    ] {
        set_recipe(&mut data, id, recipe);
        let pattern = format!(" __{name} (.-);");
        row(&mut data, D::ModTag, &pattern, P::Callback(id));
        source
            .source
            .dictionary("modTagList")
            .set(pattern, source.fixture("first", body))
            .unwrap();
    }
    set_recipe(&mut data, ids[2], metadata_recipe());
    for pattern in [" __second (.*) ", " __missing "] {
        row(&mut data, D::ModTag, pattern, P::Callback(ids[2]));
        source
            .source
            .dictionary("modTagList")
            .set(pattern, source.fixture("second", METADATA_BODY))
            .unwrap();
    }
    let native = CompiledModifierParser::new(&ModifierParserCatalog::new(data).unwrap()).unwrap();
    for name in ["nil", "empty"] {
        for suffix in [" __second 34", " __missing"] {
            let text = format!("+7 to maximum Life __{name} 12;{suffix}");
            let ok = compare(&source, &native, text.as_bytes());
            let calls = source.calls();
            if name == "nil" {
                assert!(ok);
                assert_eq!(calls.len(), 2);
                assert!(calls.iter().all(|v| v.label == "first"));
                assert!(calls[0].frames.contains(&7406));
                assert!(calls[1].frames.contains(&7408));
            } else if suffix.contains("missing") {
                assert!(!ok);
                assert_eq!(calls.len(), 1);
                assert_eq!(calls[0].label, "first");
            } else {
                assert!(ok);
                assert_eq!(calls.len(), 2);
                assert_eq!(calls[0].label, "first");
                assert_eq!(calls[1].label, "second");
                assert_eq!(calls[1].arguments.roots[0], Atom::Number(34.0f64.to_bits()));
                assert!(calls[1].frames.contains(&6684));
            }
        }
    }
    // False is not a representable top-level native factory recipe. Preserve its
    // original truthiness behavior as a source-only protocol observation.
    source
        .source
        .dictionary("modTagList")
        .set(
            " __false (.-);",
            source.variadic_fixture("false_first", "return false"),
        )
        .unwrap();
    for suffix in [" __second 34", " __missing"] {
        assert!(matches!(
            source.observe(format!("+7 to maximum Life __false 12;{suffix}").as_bytes()),
            Observation::Returned(_)
        ));
        let calls = source.calls();
        assert_eq!(calls.len(), 2);
        assert!(calls.iter().all(|c| c.label == "false_first"));
    }
}

#[test]
fn source_precheck_errors_precede_unsupported_body_and_missing_form_skips_tags() {
    let (source, mut data, ids) = ordinary_fixture();
    let id = ids[0];
    data.factories.insert(
        id,
        F::Unsupported {
            reason: "caller fixture unavailable body".into(),
        },
    );
    for (family, pattern) in [
        (D::ModTag, " __unsupported "),
        (D::ModTag, " __unsupported_position() "),
        (D::ModTag, " __unsupported_capture (.*) "),
        (D::PreFlag, "^__unavailable_prefix "),
    ] {
        row(&mut data, family, pattern, P::Callback(id));
        source
            .source
            .dictionary(family.source_name())
            .set(
                pattern,
                source.variadic_fixture("body", "error('body executed')"),
            )
            .unwrap();
    }
    let marker = empty_table(&mut data);
    row(&mut data, D::ModTag, " __first", marker);
    source
        .source
        .dictionary("modTagList")
        .set(
            " __first",
            source.source.public.source.lua.create_table().unwrap(),
        )
        .unwrap();
    let native = CompiledModifierParser::new(&ModifierParserCatalog::new(data).unwrap()).unwrap();
    for prefix in ["+7 to maximum Life", "+7 to maximum Life __first"] {
        for suffix in [" __unsupported", " __unsupported_position"] {
            let text = format!("{prefix}{suffix}");
            assert!(!compare(&source, &native, text.as_bytes()));
            assert!(source.calls().is_empty());
            assert!(
                matches!(native.parse(text.as_bytes(),&mut MatchBudget::default()),Err(ParserError::SourceError(ref why)) if why.contains("non-string"))
            );
        }
        let text = format!("{prefix} __unsupported_capture 12");
        assert!(
            matches!(source.observe(text.as_bytes()),Observation::SourceError(ref why) if why.contains("body executed"))
        );
        assert_eq!(source.calls().len(), 1);
        assert!(
            matches!(native.parse(text.as_bytes(),&mut MatchBudget::default()),Err(ParserError::Deferred{callback:Some(found),..}) if found==id)
        );
    }
    assert!(compare(&source, &native, b"not a modifier __unsupported"));
    assert!(source.calls().is_empty());
    let text = b"__unavailable_prefix no modifier form";
    assert!(matches!(source.observe(text), Observation::SourceError(_)));
    assert_eq!(source.calls().len(), 1);
    assert!(source.calls()[0].frames.contains(&6656));
    assert!(matches!(
        native.parse(text, &mut MatchBudget::default()),
        Err(ParserError::Deferred {
            stage: "prefix callback",
            ..
        })
    ));
}

#[test]
fn configured_numeric_guard_uses_lazy_original_match_return_semantics() {
    let snapshot = bundled_snapshot().unwrap();
    let original = snapshot.modifier_parser().data();
    let id = pure_ids(original, D::ModTag)[0];
    let mut paired = 0;
    let mut errors = 0;
    for guard in ["%d+", "%a+", "", "()", "^(%d*)$", "[", "%f[%d]"] {
        let source = OrdinarySource::configured(Some(guard));
        let mut data = original.clone();
        data.policy.tag_capture_numeric_pattern = guard.into();
        for family in [D::PreFlag, D::ModTag] {
            data.tables[data.dictionaries[&family].0 as usize - 1] = ParserTable::default();
            clear(&source.source.dictionary(family.source_name()));
        }
        set_recipe(&mut data, id, metadata_recipe());
        for (family, pattern) in [
            (D::ModTag, " __guard (.*) "),
            (D::ModTag, " __missing "),
            (D::ModTag, " __position() "),
            (D::PreFlag, "^__prefix "),
            (D::Special, "^__special_fixture__$"),
        ] {
            row(&mut data, family, pattern, P::Callback(id));
            source
                .source
                .dictionary(family.source_name())
                .set(pattern, source.fixture("selected", METADATA_BODY))
                .unwrap();
        }
        let static_tag = empty_table(&mut data);
        row(&mut data, D::ModTag, " __static", static_tag);
        source
            .source
            .dictionary("modTagList")
            .set(
                " __static",
                source.source.public.source.lua.create_table().unwrap(),
            )
            .unwrap();
        let native =
            CompiledModifierParser::new(&ModifierParserCatalog::new(data).unwrap()).unwrap();
        for prefix in [
            b"+7 to maximum Life".as_slice(),
            b"+7 to maximum Life __static",
        ] {
            for capture in [b"12".as_slice(), b"x1", b"word", b"", b"\xff\0"] {
                let text = [prefix, b" __guard ", capture].concat();
                let ok = compare(&source, &native, &text);
                if guard == "[" {
                    assert!(!ok);
                    assert!(source.calls().is_empty());
                    errors += 1;
                } else {
                    assert!(ok);
                    assert_eq!(source.calls()[0].count, 2);
                    paired += 1;
                }
            }
            for suffix in [b" __missing".as_slice(), b" __position"] {
                let text = [prefix, suffix].concat();
                assert!(!compare(&source, &native, &text));
                assert!(source.calls().is_empty());
                assert!(
                    matches!(native.parse(&text,&mut MatchBudget::default()),Err(ParserError::SourceError(ref why)) if why.contains("non-string")),
                    "method access must precede malformed pattern interpretation"
                );
                errors += 1;
            }
        }
        // These do not enter a tag function, so even a malformed configured guard is
        // unused. A malformed dictionary pattern would be a different earlier scan.
        for text in [
            b"+7 to maximum Life".as_slice(),
            b"+7 to maximum Life __static",
            b"__prefix +7 to maximum Life",
            b"__special_fixture__",
            b"no modifier form __missing",
        ] {
            assert!(compare(&source, &native, text));
            paired += 1;
        }
    }
    eprintln!(
        "Configured original guard variants: {paired} exact graphs,{errors} ordered errors; no translated Lua callback oracle"
    );
}

#[test]
fn every_original_ordinary_factory_runs_directly_in_live_traces_and_preserves_public_output() {
    let snapshot = bundled_snapshot().unwrap();
    let catalog = snapshot.modifier_parser();
    let source = OrdinarySource::new();
    let lua = &source.source.public.source.lua;
    let generate: Function = lua
        .load(include_str!("support/lua_pattern_witness.lua").replace("{55,97,65", "{97,55,65"))
        .eval()
        .unwrap();
    let copy: Function = lua.globals().get("copyTable").unwrap();
    let cases = lua.create_table().unwrap();
    let mut cold = vec![];
    for family in [D::PreFlag, D::ModTag] {
        for (pattern, id) in
            catalog
                .dictionary(family)
                .fields
                .iter()
                .filter_map(|(pattern, value)| {
                    let P::Callback(id) = value else { return None };
                    matches!(catalog.factory(*id), Some(F::Pure(_)))
                        .then_some((pattern.clone(), *id))
                })
        {
            let dictionary = source.source.dictionary(family.source_name());
            let original: Function = dictionary.get(pattern.as_str()).unwrap();
            let label = format!("warm:{}", id.0);
            dictionary
                .set(pattern.as_str(), source.wrap(original.clone(), &label))
                .unwrap();
            let witness: mlua::LuaString = generate.call(pattern.as_str()).unwrap();
            let witness = witness.as_bytes();
            let input = if family == D::PreFlag {
                [witness.as_ref(), b"7% increased damage"].concat()
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
        .load(include_str!("support/ordinary_factories_warm.lua"))
        .set_name("@test-only-direct-ordinary-factory-warm")
        .call(cases)
        .unwrap();
    assert_eq!(
        observed.get::<usize>("executions").unwrap(),
        cold.len() * 128
    );
    let live = observed
        .get::<Table>("live")
        .unwrap()
        .sequence_values::<Table>()
        .map(Result::unwrap)
        .map(|r| r.get::<usize>("line").unwrap())
        .collect::<BTreeSet<_>>();
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
        "Warm ordinary source:{} original factories,{} direct executions; exact actual-call cold/warm metadata and warmed full public native graphs",
        cold.len(),
        cold.len() * 128
    );
}

#[test]
fn ordinary_factory_metadata_combines_complete_enemy_wrappers_and_multiple_tag_outputs() {
    let snapshot = bundled_snapshot().unwrap();
    let native = CompiledModifierParser::new(snapshot.modifier_parser()).unwrap();
    let source = OrdinarySource::new();
    let mut paired = 0;
    for prefix in [
        "",
        "Enemies you've hit recently have ",
        "While a unique enemy is in your presence, enemies you've hit recently have ",
        "While a pinnacle atlas boss is in your presence, enemies you've hit recently have ",
    ] {
        for form in [
            "1% increased Damage",
            "+7 to Strength and Dexterity",
            "-0 to maximum Life",
            "13% more Damage",
            "Grants 3 Life per Enemy Hit",
        ] {
            for suffix in [
                " per 10 maximum Life",
                " per 10 maximum Life per 20 maximum Mana",
                " while dual wielding or holding a shield per 10 Strength",
            ] {
                let text = format!("{prefix}{form}{suffix}");
                assert!(compare(&source, &native, text.as_bytes()));
                paired += 1;
            }
        }
    }
    let result = native
        .parse(
            b"1% increased Damage per 10 maximum Life",
            &mut MatchBudget::default(),
        )
        .unwrap();
    let modifier = result.modifiers.unwrap();
    let modifier = modifier.indexed[&1].as_table().unwrap();
    assert_eq!(
        modifier.fields["name"].as_bytes(),
        Some(b"Damage".as_slice())
    );
    assert_eq!(modifier.fields["type"].as_bytes(), Some(b"INC".as_slice()));
    let tag = modifier.indexed[&1].as_table().unwrap();
    assert_eq!(tag.fields["type"].as_bytes(), Some(b"PerStat".as_slice()));
    assert_eq!(tag.fields["stat"].as_bytes(), Some(b"Life".as_slice()));
    assert!(
        matches!(tag.fields["div"],poe_optimizer_engine::modifier_parser::ModifierValue::Number(n) if n==10.0)
    );
    eprintln!(
        "Ordinary wrapper/form/two-tag combinations:{paired} exact complete public graphs, including Damage INC1 PerStat Life div10"
    );
}
