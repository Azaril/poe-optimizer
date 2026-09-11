//! Original complete public ModParser versus explicitly admitted and packaged programs.
//! Raw G3 evidence remains unchanged; this target covers the public packing/copy boundary.
#[allow(dead_code)]
#[path = "support/callback_factories_source.rs"]
mod factory_source;
#[path = "support/mod_parser_native_observer.rs"]
mod native_observer;
#[allow(dead_code)]
#[path = "support/mod_parser_public_source.rs"]
mod public_source;
#[allow(dead_code)]
#[path = "support/parser_program_source.rs"]
mod raw_source;
#[allow(dead_code)]
#[path = "support/item_loading_runtime.rs"]
mod runtime;
#[path = "support/parser_program_public_source.rs"]
mod source;
use mlua::{Table, Value};
use poe_optimizer_data::modifier_parser::*;
use poe_optimizer_engine::{
    lua_pattern::MatchBudget,
    modifier_parser::{CompiledModifierParser, ParserError},
};
use raw_source::Target;
use source::Original;

#[test]
fn packaged_and_explicit_program_admissions_use_the_same_public_constructor() {
    let snapshot = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
    let owner = snapshot.modifier_parser();
    let admitted = ParserAdmittedProgramCatalog::new(owner).unwrap();
    assert!(admitted.is_bound_to(owner));
    for target in Target::ALL {
        let id = target.id(owner);
        assert!(
            admitted.is_admitted(id),
            "packaged target {target:?} not admitted"
        );
        assert_eq!(admitted.is_special(id), !matches!(target, Target::Grant));
    }
    assert_eq!(owner.data().programs.admissions.len(), 4);
    let packaged = CompiledModifierParser::new(owner).unwrap();
    let explicit = source::parser(source::bare_data());
    let source = Original::new(owner);
    for line in [
        b"+2 to level of all fire skills".as_slice(),
        b"grants skill: fireball",
        b"grants skill: level 7 fireball",
        b"grants skill: missing caller skill",
    ] {
        let (expected, _) = source.pair(&packaged, line);
        let (_, actual) = source.pair(&explicit, line);
        assert_eq!(
            expected,
            native_observer::capture(&actual, explicit.catalog()).unwrap()
        );
    }
    let mut unadmitted = owner.data().clone();
    unadmitted.programs.admissions.clear();
    let unadmitted =
        CompiledModifierParser::new(&ModifierParserCatalog::new(unadmitted).unwrap()).unwrap();
    assert!(matches!(
        unadmitted.parse(b"grants skill: fireball", &mut MatchBudget::default()),
        Err(ParserError::Deferred { .. })
    ));
}

#[test]
fn all_lookup_names_retain_actual_source_selection_and_public_graph_pairs() {
    let snapshot = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
    let owner = snapshot.modifier_parser();
    let native = CompiledModifierParser::new(owner).unwrap();
    let names = owner
        .dictionary(ParserDictionary::GemIdLookup)
        .fields
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    let mut rows = vec![];
    let mut paired = 0;
    for target in [Target::Property, Target::Plain, Target::Level] {
        let expected_id = target.id(owner);
        for chunk in names.chunks(256) {
            // Bounded fresh VMs keep the original source instruction deadline local.
            let source = Original::new(owner);
            for name in chunk {
                let line = match target {
                    Target::Property => format!("+2 to quality of all {name} skills"),
                    Target::Plain => format!("grants skill: {name}"),
                    Target::Level => format!("grants skill: level 7 {name}"),
                    Target::Grant => unreachable!(),
                };
                let (selected, remainder, captures) = source.selection(line.as_bytes());
                let admitted = selected == Some(expected_id) && remainder.is_empty();
                let roots = if admitted {
                    let (graph, _) = source.pair(&native, line.as_bytes());
                    paired += 1;
                    Some(graph.roots)
                } else {
                    None
                };
                rows.push(serde_json::json!({"lookup_name":name,"line":line,"requested_callback":expected_id,"selected_callback":selected,"source_remainder":remainder,"source_capture_graph":captures,"paired":admitted,"public_roots":roots}));
            }
        }
    }
    assert_eq!(rows.len(), names.len() * 3);
    assert!(
        paired >= names.len(),
        "leveled grant should express all names"
    );
    if let Some(path) = std::env::var_os("POE_PROGRAM_PUBLIC_ARTIFACT") {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        serde_json::to_writer_pretty(file,&serde_json::json!({"schema_version":1,"scope":"Original scan reports chosen callback, capture pack and remainder; it does not expose an exact winning key. Non-selected/partial lines remain observations, not parity claims.","lookup_entries":names.len(),"candidates":rows.len(),"paired":paired,"rows":rows})).unwrap();
    }
    eprintln!(
        "natural public candidates: {}; source-selected exact target pairs:{paired}; remaining observations:{}",
        names.len() * 3,
        names.len() * 3 - paired
    );
}

#[test]
fn explicit_aliases_preserve_special_capture_packing_errors_and_return_arity() {
    let owner = poe_optimizer_data::game_data::bundled_snapshot()
        .unwrap()
        .modifier_parser()
        .clone();
    let mut source = Original::new(&owner);
    let mut data = source::bare_data();
    for (target, pattern) in [
        (Target::Property, "^__g4_property (.-)|(.-)|(.-)|(.-)$"),
        (Target::Plain, "^__g4_plain (.-)$"),
        (Target::Level, "^__g4_level (.-)|(.-)$"),
        (Target::Property, "^__g4_position ()(.-)|(.-)|(.-)$"),
    ] {
        let id = target.id(&owner);
        source::add_alias(&mut data, pattern, id);
        source.alias(pattern, source.raw.function(target).clone(), id);
    }
    let native = source::parser(data);
    for line in [
        b"__g4_property -0|quality|caller word|strength".as_slice(),
        b"__g4_property nan|quality|fire|intelligence",
        b"__g4_property inf|\xffp|\xffcaller|",
        b"__g4_property bad|quality||",
        b"__g4_plain fireball",
        b"__g4_plain missing",
        b"__g4_level 0x10|fireball",
        b"__g4_level nan|fireball",
        b"__g4_level invalid|fireball",
        b"__g4_level 7|missing",
        b"__g4_position quality|callerword|dexterity",
    ] {
        source.pair(&native, line);
    }
    let pattern = "^__g4_missing__$";
    let mut data = source::bare_data();
    let id = Target::Plain.id(&owner);
    source::add_alias(&mut data, pattern, id);
    source.alias(pattern, source.raw.plain.clone(), id);
    let native = source::parser(data);
    source.source_error(&native, b"__g4_missing__", "index");
}

#[test]
fn immutable_catalog_snapshots_and_original_cache_keep_return_mutations_isolated() {
    let owner = poe_optimizer_data::game_data::bundled_snapshot()
        .unwrap()
        .modifier_parser()
        .clone();
    let mut source = Original::new(&owner);
    let counts = source.counters(&owner);
    let mut data = source::bare_data();
    source::set_lookup(&mut data, "caller", ParserValue::Text("CallerOne".into()));
    source.raw.lookup.raw_set("caller", "CallerOne").unwrap();
    let native = source::parser(data.clone());
    let line = b"grants skill: level 7 caller";
    let (expected, mut first) = source.pair(&native, line);
    let returned: Table = source
        .raw
        .factory
        .public
        .raw(line)
        .unwrap()
        .front()
        .unwrap()
        .as_table()
        .unwrap()
        .clone();
    returned
        .get::<Table>(1)
        .unwrap()
        .get::<Table>("value")
        .unwrap()
        .raw_set("skillId", "caller mutation")
        .unwrap();
    first.modifiers.as_mut().unwrap().fields.insert(
        "caller mutation".into(),
        poe_optimizer_engine::modifier_parser::ModifierValue::Boolean(true),
    );
    let (again, _) = source.pair(&native, line);
    assert_eq!(again, expected);
    assert_eq!(
        counts.get::<u32>(Target::Level.id(&owner).0).unwrap(),
        1,
        "public cache avoids repeat factory calls"
    );
    source.raw.lookup.raw_set("caller", "CallerTwo").unwrap();
    let (cached, _) = source.pair(&native, line);
    assert_eq!(
        cached, expected,
        "replacement definition cannot mutate earlier cached scalar result"
    );
    source::set_lookup(&mut data, "caller", ParserValue::Text("CallerTwo".into()));
    let next = source::parser(data);
    let fresh_line = b"grants skill: level 7 caller skill";
    let (updated, _) = source.pair(&next, fresh_line);
    assert_ne!(updated, expected);
    assert_eq!(counts.get::<u32>(Target::Level.id(&owner).0).unwrap(), 2);
    // Each comparison binds one immutable definition snapshot; no live cache
    // invalidation semantics are invented for a changed native catalog.
}

#[test]
fn original_program_definition_aliases_are_broken_by_each_public_copy() {
    use poe_optimizer_engine::modifier_parser::ModifierValue;
    use std::{collections::BTreeMap, sync::Arc};
    let owner = poe_optimizer_data::game_data::bundled_snapshot()
        .unwrap()
        .modifier_parser()
        .clone();
    let source = Original::new(&owner);
    let mut data = source::bare_data();
    let child = ParserTableId(data.tables.len() as u32 + 1);
    data.tables.push(ParserTable {
        fields: BTreeMap::from([("x".into(), ParserValue::Number(-0.0))]),
        indexed: BTreeMap::new(),
    });
    let parent = ParserTableId(data.tables.len() as u32 + 1);
    data.tables.push(ParserTable {
        fields: BTreeMap::from([
            ("left".into(), ParserValue::Table(child)),
            ("right".into(), ParserValue::Table(child)),
        ]),
        indexed: BTreeMap::new(),
    });
    source::set_lookup(&mut data, "caller aliases", ParserValue::Table(parent));
    let shared = source.raw.table();
    shared.raw_set("x", -0.0).unwrap();
    let definition = source.raw.table();
    definition.raw_set("left", shared.clone()).unwrap();
    definition.raw_set("right", shared).unwrap();
    source
        .raw
        .lookup
        .raw_set("caller aliases", definition)
        .unwrap();
    let native = source::parser(data);
    let line = b"grants skill: caller aliases";
    let (expected, actual) = source.pair(&native, line);
    let source_mods: Table = source
        .raw
        .factory
        .public
        .raw(line)
        .unwrap()
        .front()
        .unwrap()
        .as_table()
        .unwrap()
        .clone();
    let source_payload: Table = source_mods
        .get::<Table>(1)
        .unwrap()
        .get::<Table>("value")
        .unwrap()
        .get("skillId")
        .unwrap();
    let left: Table = source_payload.get("left").unwrap();
    let right: Table = source_payload.get("right").unwrap();
    assert_ne!(left.to_pointer(), right.to_pointer());
    left.raw_set("x", 99).unwrap();
    assert_eq!(
        right.get::<f64>("x").unwrap().to_bits(),
        (-0.0f64).to_bits()
    );
    let cached: Table = source
        .raw
        .factory
        .public
        .cache
        .get::<Table>(source.raw.text(line))
        .unwrap()
        .get(1)
        .unwrap();
    let cached_payload: Table = cached
        .get::<Table>(1)
        .unwrap()
        .get::<Table>("value")
        .unwrap()
        .get("skillId")
        .unwrap();
    assert_eq!(
        cached_payload.get::<Table>("left").unwrap().to_pointer(),
        cached_payload.get::<Table>("right").unwrap().to_pointer()
    );
    let root = actual.modifiers.as_ref().unwrap();
    let value = root.indexed[&1].as_table().unwrap().fields["value"]
        .as_table()
        .unwrap()
        .fields["skillId"]
        .as_table()
        .unwrap();
    let (ModifierValue::Table(left), ModifierValue::Table(right)) =
        (&value.fields["left"], &value.fields["right"])
    else {
        panic!("native alias fields")
    };
    assert!(!Arc::ptr_eq(left, right));
    let (again, _) = source.pair(&native, line);
    assert_eq!(expected, again);
}

#[test]
fn authored_return_controls_preserve_public_zero_nil_empty_extra_and_retry() {
    use poe_optimizer_engine::lua_pattern::MatchLimits;
    use public_source::Atom;
    let owner = poe_optimizer_data::game_data::bundled_snapshot()
        .unwrap()
        .modifier_parser()
        .clone();
    let id = Target::Property.id(&owner);
    let mut source = Original::new(&owner);
    let pattern = "^__g4_return__$";
    let line = b"__g4_return__";
    let mut data = source::bare_data();
    source::add_alias(&mut data, pattern, id);
    let cases = [
        ("return", vec![], 0usize, 1u32),
        ("return nil", vec![source::nil()], 0, 1),
        ("return {}", vec![source::empty_table()], 1, 1),
        (
            "return {},nil",
            vec![source::empty_table(), source::nil()],
            1,
            1,
        ),
        (
            "return nil,'extra'",
            vec![source::nil(), source::bytes(b"extra")],
            2,
            1,
        ),
        (
            "return {},nil,'ignored'",
            vec![
                source::empty_table(),
                source::nil(),
                source::bytes(b"ignored"),
            ],
            1,
            1,
        ),
        (
            "return {},''",
            vec![source::empty_table(), source::bytes(b"")],
            2,
            2,
        ),
    ];
    let lua = &source.raw.factory.public.source.lua;
    let counts = lua.create_table().unwrap();
    let make:mlua::Function=lua.load("return function(body,counts) return function(...) counts.n=(counts.n or 0)+1;return body(...) end end").set_name("@test-only-public-return-count").eval().unwrap();
    let mut one_pass = 0;
    for (body, returns, arity, calls) in cases {
        let function = source.raw.factory.synthetic(body);
        let wrapper: mlua::Function = make.call((function, counts.clone())).unwrap();
        source.alias(pattern, wrapper, id);
        source
            .raw
            .factory
            .public
            .cache
            .raw_set(source.raw.text(line), Value::Nil)
            .unwrap();
        counts.raw_set("n", 0).unwrap();
        let native = CompiledModifierParser::new(&source::rebind(
            data.clone(),
            source::returning(source::original_programs(), id, returns),
        ))
        .unwrap();
        let (graph, _) = source.pair(&native, line);
        assert_eq!(graph.roots.len(), arity, "{body}");
        assert_eq!(
            counts.get::<u32>("n").unwrap(),
            calls,
            "source retry count {body}"
        );
        if body == "return {},nil" {
            let mut budget = MatchBudget::default();
            native.parse(line, &mut budget).unwrap();
            one_pass = budget.steps_used();
        }
        if body == "return {},''" {
            assert_eq!(graph.roots, [Atom::Table(0), Atom::Bytes(vec![])]);
            let mut budget = MatchBudget::default();
            native.parse(line, &mut budget).unwrap();
            assert!(
                budget.steps_used() > one_pass * 3 / 2,
                "retry executes additional parser/program work"
            );
            let mut limited = MatchBudget::new(MatchLimits {
                max_steps: one_pass * 3 / 2,
                ..MatchLimits::default()
            });
            assert!(
                native.parse(line, &mut limited).is_err(),
                "single-pass budget cannot silently skip retry"
            );
        }
    }
    // These shapes are legal Lua returns but outside the public Rust DTO.
    // They must be deferred, not collapsed into absent/nil results.
    for (body, returns) in [
        (
            "return false",
            vec![source::expr(ParserProgramExprKind::Literal {
                value: ParserFactoryLiteral::Boolean(false),
            })],
        ),
        (
            "return {},false",
            vec![
                source::empty_table(),
                source::expr(ParserProgramExprKind::Literal {
                    value: ParserFactoryLiteral::Boolean(false),
                }),
            ],
        ),
        (
            "return {},0",
            vec![
                source::empty_table(),
                source::expr(ParserProgramExprKind::Literal {
                    value: ParserFactoryLiteral::Number(0.0),
                }),
            ],
        ),
    ] {
        source.alias(pattern, source.raw.factory.synthetic(body), id);
        source
            .raw
            .factory
            .public
            .cache
            .raw_set(source.raw.text(line), Value::Nil)
            .unwrap();
        let result = source.raw.factory.public.raw(line).unwrap();
        assert!(!result.is_empty());
        let native = CompiledModifierParser::new(&source::rebind(
            data.clone(),
            source::returning(source::original_programs(), id, returns),
        ))
        .unwrap();
        assert!(
            matches!(
                native.parse(line, &mut MatchBudget::default()),
                Err(ParserError::Deferred { .. })
            ),
            "{body}"
        );
    }
}

#[test]
fn public_misses_and_cache_hits_have_live_original_trace_evidence() {
    use mlua::Function;
    let owner = poe_optimizer_data::game_data::bundled_snapshot()
        .unwrap()
        .modifier_parser()
        .clone();
    let native = CompiledModifierParser::new(&owner).unwrap();
    for (target, line, misses) in [
        (
            Target::Property,
            b"+2 to level of all fire skills".as_slice(),
            true,
        ),
        (Target::Plain, b"grants skill: fireball", true),
        (Target::Level, b"grants skill: level 7 fireball", true),
        (Target::Property, b"+2 to level of all fire skills", false),
    ] {
        let source = Original::new(&owner);
        // Populate the exact cache entry before the independent hit control.
        // Miss controls explicitly invalidate that key on every measured call.
        source.pair(&native, line);
        let lua = &source.raw.factory.public.source.lua;
        let driver: Table = lua
            .load(include_str!("support/parser_program_public_warm.lua"))
            .set_name("@test-only-public-program-live-driver")
            .eval()
            .unwrap();
        let required = lua.create_table().unwrap();
        required
            .raw_set("public", source.raw.factory.public.parse.clone())
            .unwrap();
        required
            .raw_set("copy", lua.globals().get::<Function>("copyTable").unwrap())
            .unwrap();
        if misses {
            required
                .raw_set("internal", source.raw.factory.internal.clone())
                .unwrap();
            required
                .raw_set("body", source.raw.function(target).clone())
                .unwrap();
            required
                .raw_set("constructor", source.raw.factory.create_mod.clone())
                .unwrap();
            if matches!(target, Target::Plain | Target::Level) {
                required
                    .raw_set("helper", source.raw.grant.clone())
                    .unwrap();
            }
        }
        driver
            .get::<Function>("begin")
            .unwrap()
            .call::<()>(())
            .unwrap();
        let values: mlua::MultiValue = driver
            .get::<Function>("run")
            .unwrap()
            .call((
                source.raw.factory.public.parse.clone(),
                source.raw.factory.public.cache.clone(),
                source.raw.text(line),
                misses,
            ))
            .unwrap();
        let evidence: Table = driver
            .get::<Function>("finish")
            .unwrap()
            .call(required.clone())
            .unwrap();
        let found: Table = evidence.get("found").unwrap();
        assert!(evidence.get::<usize>("live").unwrap() > 0);
        for row in required.pairs::<String, Function>() {
            let (key, _) = row.unwrap();
            assert!(
                found.get::<bool>(key.as_str()).unwrap_or(false),
                "{target:?}/misses={misses}: original {key} absent from completed live traces"
            );
        }
        let expected = source.raw.factory.public.capture(values).unwrap();
        let actual = native.parse(line, &mut MatchBudget::default()).unwrap();
        assert_eq!(
            expected,
            native_observer::capture(&actual, native.catalog()).unwrap()
        );
        eprintln!(
            "live public {target:?}/misses={misses}: {} live traces, {} stops, {} aborts",
            evidence.get::<usize>("live").unwrap(),
            evidence.get::<usize>("stop").unwrap(),
            evidence.get::<usize>("abort").unwrap()
        );
    }
}
