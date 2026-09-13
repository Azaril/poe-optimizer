//! Complete original explosion helpers/public parser versus injected native programs.
//! Public dispatch uses the reviewed package; raw tests retain explicit source inputs.
#[allow(dead_code)]
#[path = "support/callback_factories_source.rs"]
mod factory_source;
#[path = "support/mod_parser_native_observer.rs"]
mod native_observer;
#[allow(dead_code)]
#[path = "support/explosion_helper_observer.rs"]
mod observer;
#[allow(dead_code)]
#[path = "support/explosion_source.rs"]
mod original;
#[allow(dead_code)]
#[path = "support/mod_parser_public_source.rs"]
mod public_source;
#[allow(dead_code)]
#[path = "support/parser_program_source.rs"]
mod raw_source;
#[allow(dead_code)]
#[path = "support/item_loading_runtime.rs"]
mod runtime;
use factory_source::upvalue;
use mlua::{Function, MultiValue, Table, Value};
use original::Original;
use poe_optimizer_data::modifier_parser::*;
use poe_optimizer_engine::{
    lua_pattern::MatchBudget, modifier_parser::CompiledModifierParser, parser_program::*,
};
use std::collections::BTreeSet;

fn callback_id(owner: &ModifierParserCatalog, function: &Function) -> ParserCallbackId {
    let info = function.info();
    let ids = owner
        .data()
        .callbacks
        .iter()
        .enumerate()
        .filter_map(|(index, c)| {
            let ParserCallbackKind::Lua { source } = &c.kind else {
                return None;
            };
            (info.source.as_deref() == Some(format!("@{}", source.path).as_str())
                && info.line_defined == Some(source.line as usize)
                && info.last_line_defined == Some(source.end_line as usize))
            .then_some(ParserCallbackId(index as u32 + 1))
        })
        .collect::<Vec<_>>();
    assert_eq!(ids.len(), 1, "unique original declaration");
    ids[0]
}
fn family(owner: &ModifierParserCatalog, helper: ParserCallbackId) -> BTreeSet<ParserCallbackId> {
    owner
        .data()
        .callbacks
        .iter()
        .enumerate()
        .filter_map(|(index, c)| {
            c.upvalues
                .iter()
                .any(|u| u.value == ParserValue::Callback(helper))
                .then_some(ParserCallbackId(index as u32 + 1))
        })
        .collect()
}
fn source_function(
    original: &Original,
    owner: &ModifierParserCatalog,
    id: ParserCallbackId,
) -> Function {
    let functions = owner
        .dictionary(ParserDictionary::Special)
        .fields
        .iter()
        .filter_map(|(key, value)| {
            if value != &ParserValue::Callback(id) {
                return None;
            }
            Some(
                original
                    .source
                    .special
                    .raw_get::<Function>(key.as_str())
                    .unwrap(),
            )
        })
        .collect::<Vec<_>>();
    let first = functions
        .first()
        .expect("original special function")
        .clone();
    assert!(functions.iter().all(|f| f == &first));
    assert_eq!(callback_id(owner, &first), id);
    assert_eq!(
        upvalue(&original.source.public.source.lua, &first, "explodeFunc").as_function(),
        Some(&original.helper)
    );
    first
}
fn plan() -> CompiledParserPrograms {
    CompiledParserPrograms::new(raw_source::extraction().catalog()).unwrap()
}
fn source_inputs(original: &Original, args: MultiValue) -> MultiValue {
    // Materialize host-supplied numbers through lua_pushnumber before observing
    // native input. LuaJIT canonicalizes NaN on this ingress; holes and shared
    // table identities survive the round trip. Returned outputs stay bit-exact.
    original
        .source
        .public
        .source
        .lua
        .load("return ...")
        .set_name("@test/explosion-input-ingress")
        .call(args)
        .unwrap()
}
fn pair(
    original: &Original,
    plan: &CompiledParserPrograms,
    function: &Function,
    args: Vec<Value>,
    label: &str,
) -> public_source::Graph {
    original.verify();
    let id = callback_id(plan.catalog().owner(), function);
    let args = source_inputs(original, MultiValue::from_vec(args));
    let input = raw_source::from_source(&observer::graph(&original.source, args.clone()));
    let source = function
        .call::<MultiValue>(args)
        .unwrap_or_else(|e| panic!("{label}: source {e}"));
    let expected = observer::graph(&original.source, source);
    let actual = plan
        .execute(id, &input, ProgramLimits::default())
        .unwrap_or_else(|e| panic!("{label}: native {e:?}"));
    assert_eq!(
        raw_source::native_graph(actual.graph()),
        expected,
        "{label}: full raw graph/arity/aliases"
    );
    original.verify();
    expected
}
fn assert_source_site(
    error: &ProgramRuntimeError,
    plan: &CompiledParserPrograms,
    helper: ParserCallbackId,
    operation: &str,
) {
    assert_eq!(error.kind, ProgramRuntimeErrorKind::Source, "{error:?}");
    assert_eq!(error.callback, Some(helper), "actual helper caller");
    let program = plan.catalog().for_callback(helper).unwrap();
    let source = runtime::verified(&program.provenance.source.path).unwrap();
    let declaration = source
        .lines()
        .skip(program.provenance.source.line as usize - 1)
        .take((program.provenance.source.end_line - program.provenance.source.line + 1) as usize)
        .collect::<Vec<_>>()
        .join("\n");
    let location = error.location.expect("original caller site");
    let start = program.provenance.function_start as usize;
    let reached = &declaration[start + location.start as usize..start + location.end as usize];
    assert_eq!(reached.trim(), operation, "source operation before failure");
}

fn packaged_parser(original: &Original) -> CompiledModifierParser {
    let snapshot = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
    let owner = snapshot.modifier_parser();
    let helper = callback_id(owner, &original.helper);
    let roles = &owner.data().programs.admissions;
    for id in family(owner, helper) {
        assert_eq!(roles[&id].role, ParserProgramRole::Special);
    }
    for id in [helper, callback_id(owner, &original.flag)] {
        assert_eq!(roles[&id].role, ParserProgramRole::Helper);
    }
    CompiledModifierParser::new(owner).unwrap()
}

#[test]
fn complete_original_helper_raw_returns_errors_and_forwarded_aliases_match_native() {
    let original = Original::new();
    let plan = plan();
    for amount in [
        Value::Number(0.0),
        Value::Number(-0.0),
        Value::Number(17.25),
        original.text(" 7e0 "),
        original.text("0x8"),
        original.text("tenth"),
        original.text("quarter"),
        Value::Number(f64::INFINITY),
        Value::Number(f64::NEG_INFINITY),
        Value::Number(f64::NAN),
    ] {
        pair(
            &original,
            &plan,
            &original.helper,
            vec![Value::Number(37.0), amount, original.text("physical")],
            "amount",
        );
    }
    for amount in [
        Value::Nil,
        Value::Boolean(false),
        original.text("Tenth"),
        original.text("nonsense"),
    ] {
        assert!(
            pair(
                &original,
                &plan,
                &original.helper,
                vec![Value::Number(100.0), amount, Value::Nil],
                "early zero-return"
            )
            .roots
            .is_empty()
        );
    }
    for kind in [
        Value::Nil,
        Value::Number(f64::NAN),
        Value::Boolean(true),
        Value::Table(original.source.public.source.lua.create_table().unwrap()),
    ] {
        let (line, message) = match &kind {
            Value::Nil => (2261, "table index is nil"),
            Value::Number(_) => (2261, "table index is NaN"),
            _ => (14, "attempt"),
        };
        let args = MultiValue::from_vec(vec![Value::Number(100.0), Value::Number(10.0), kind]);
        let args = source_inputs(&original, args);
        let input = raw_source::from_source(&observer::graph(&original.source, args.clone()));
        let error = original.helper.call::<MultiValue>(args).unwrap_err();
        assert!(
            observer::source_error_at(&error, message, line),
            "unexpected source/host error: {error}"
        );
        let native = plan
            .execute(
                callback_id(plan.catalog().owner(), &original.helper),
                &input,
                ProgramLimits::default(),
            )
            .unwrap_err();
        assert_source_site(
            &native,
            &plan,
            callback_id(plan.catalog().owner(), &original.helper),
            if line == 2261 {
                "amounts[type] = amountNumber"
            } else {
                "firstToUpper(type)"
            },
        );
    }
    let tag = original.source.public.source.lua.create_table().unwrap();
    tag.raw_set("type", "Condition").unwrap();
    tag.raw_set("var", "CallerTag").unwrap();
    pair(
        &original,
        &plan,
        &original.helper,
        vec![
            Value::Number(100.0),
            Value::Number(10.0),
            original.text("physical"),
            original.text("caller source"),
            Value::Number(7.0),
            Value::Number(9.0),
            Value::Table(tag.clone()),
            Value::Nil,
            Value::Table(tag),
        ],
        "shared tags and nil hole",
    );
}

#[test]
fn every_original_captured_explosion_wrapper_matches_raw_native_programs() {
    let original = Original::new();
    let plan = plan();
    let owner = plan.catalog().owner();
    let ids = family(owner, callback_id(owner, &original.helper));
    assert_eq!(ids.len(), 23);
    let source_text = runtime::verified("src/Modules/ModParser.lua").unwrap();
    for id in ids {
        let function = source_function(&original, owner, id);
        let declaration = source_text
            .lines()
            .nth(function.info().line_defined.unwrap() - 1)
            .unwrap();
        let parameters = declaration
            .split_once("function(")
            .unwrap()
            .1
            .split_once(')')
            .unwrap()
            .0;
        for amount in [
            original.text("10"),
            original.text("tenth"),
            original.text("quarter"),
            original.text("invalid"),
        ] {
            // Direct packs follow the original declaration, independently of native IR.
            // They are not a claim that every amount can be captured by the public pattern.
            let args = match parameters {
                "chance, _, amount, type" => vec![
                    Value::Number(37.0),
                    original.text("37"),
                    amount.clone(),
                    original.text("physical"),
                ],
                "chance, _, amount" => {
                    vec![Value::Number(37.0), original.text("37"), amount.clone()]
                }
                "amount, _, type" => {
                    vec![amount.clone(), amount.clone(), original.text("physical")]
                }
                other => panic!("unreviewed original parameter layout: {other}"),
            };
            let expected = pair(
                &original,
                &plan,
                &function,
                args,
                &format!("wrapper {}", id.0),
            );
            if amount.as_string().unwrap().as_bytes().as_ref() == b"invalid" {
                assert!(expected.roots.is_empty(), "zero-return wrapper {}", id.0);
            } else {
                assert_eq!(expected.roots.len(), 1, "one-result wrapper {}", id.0);
            }
        }
    }
}

const PUBLIC_LINES: [&str; 23] = [
    "Enemies you kill have a 25% chance to explode, dealing a tenth of their maximum life as chaos damage",
    "Enemies you kill have a 25% chance to explode, dealing 10% of their maximum life as fire damage",
    "Enemies you or your totems kill have 25% chance to explode, dealing 10% of their maximum life as fire damage",
    "Enemies you kill with empowered attacks have a 25% chance to explode, dealing a tenth of their maximum life as physical damage",
    "Enemies you kill while using Pride have 25% chance to explode, dealing a tenth of their maximum life as physical damage",
    "Enemies you kill during effect have a 25% chance to explode, dealing a tenth of their maximum life as damage of a random element",
    "Enemies you kill while affected by Glorious Madness have a 25% chance to explode, dealing a quarter of their life as chaos damage",
    "Enemies killed with wand hits have a 25% chance to explode, dealing a tenth of their life as physical damage",
    "Cursed enemies you or your minions kill have a 25% chance to explode, dealing a quarter of their maximum life as chaos damage",
    "Cursed enemies killed by you, or by allies in your presence, have a 25% chance to explode, dealing a quarter of their maximum life as chaos damage",
    "Enemies you kill explode, dealing 10% of their life as physical damage",
    "Causes enemies to explode on critical kill, for 10% of their life as physical damage",
    "Enemies on fungal ground you kill explode, dealing 10% of their life as chaos damage",
    "Enemies killed with attack or spell hits explode, dealing 10% of their life as physical damage",
    "Shocked enemies you kill explode, dealing 10% of their life as lightning damage which cannot shock",
    "Bleeding enemies you kill explode, dealing 10% of their maximum life as physical damage",
    "Burning enemies you kill have a 25% chance to explode, dealing a tenth of their maximum life as fire damage",
    "Enemies killed near corpses affected by your curses explode, dealing 10% of their life as physical damage",
    "Enemies taunted by your warcries explode on death, dealing 10% of their maximum life as physical damage",
    "Warcries Explode Corpses dealing 10% of their Life as Physical Damage",
    "Totems explode on death, dealing 10% of their life as physical damage",
    "Nearby corpses explode when you warcry, dealing 10% of their life as physical damage",
    "25% chance for enemies you kill to explode, dealing 10% of their maximum life as physical damage",
];

#[test]
fn complete_public_parser_exercises_all_family_patterns_and_copy_history() {
    let original = Original::new();
    let native = packaged_parser(&original);
    let owner = native.catalog();
    let ids = family(owner, callback_id(owner, &original.helper));
    let mut reached = BTreeSet::new();
    let scan = upvalue(
        &original.source.public.source.lua,
        &original.source.internal,
        "scan",
    )
    .as_function()
    .unwrap()
    .clone();
    for line in PUBLIC_LINES {
        let lower = line.to_ascii_lowercase();
        let (selected, remainder, _): (Function, mlua::LuaString, Value) =
            scan.call((lower, original.source.special.clone())).unwrap();
        let id = callback_id(owner, &selected);
        assert!(ids.contains(&id));
        assert!(remainder.as_bytes().is_empty());
        reached.insert(id);
        let source = original.source.public.raw(line.as_bytes()).unwrap();
        assert_eq!(
            source.len(),
            1,
            "successful public pack without remainder: {line}"
        );
        let rows = source
            .front()
            .unwrap()
            .as_table()
            .expect("public modifier list");
        assert_eq!(rows.raw_len(), 2, "complete public effect pair: {line}");
        for (index, name, kind) in [(1, "ExplodeMod", "LIST"), (2, "CanExplode", "FLAG")] {
            let row: Table = rows.raw_get(index).unwrap();
            assert_eq!(row.raw_get::<String>("name").unwrap(), name);
            assert_eq!(row.raw_get::<String>("type").unwrap(), kind);
        }
        assert!(
            rows.raw_get::<Table>(2)
                .unwrap()
                .raw_get::<bool>("value")
                .unwrap()
        );
        let expected = observer::graph(&original.source, source.clone());
        let mut actual = native
            .parse(line.as_bytes(), &mut MatchBudget::default())
            .unwrap();
        assert_eq!(
            native_observer::capture(&actual, owner).unwrap(),
            expected,
            "public {line}"
        );
        actual.modifiers.as_mut().unwrap().fields.insert(
            "caller mutation".into(),
            poe_optimizer_engine::modifier_parser::ModifierValue::Boolean(true),
        );
        source
            .front()
            .unwrap()
            .as_table()
            .unwrap()
            .raw_set("caller mutation", true)
            .unwrap();
        let again = observer::graph(
            &original.source,
            original.source.public.raw(line.as_bytes()).unwrap(),
        );
        assert_eq!(again, expected, "original cache-copy {line}");
        let again = native
            .parse(line.as_bytes(), &mut MatchBudget::default())
            .unwrap();
        assert_eq!(
            native_observer::capture(&again, owner).unwrap(),
            expected,
            "native repeat {line}"
        );
    }
    assert_eq!(
        reached, ids,
        "every actual original family callback selected"
    );
}

fn configured_pattern(pattern: &str) -> (Original, CompiledParserPrograms) {
    let mut original = Original::new();
    original.verify();
    let lua = &original.source.public.source.lua;
    assert!(pattern.len() <= 4096, "configured fixture pattern bound");
    let quoted: String = lua
        .globals()
        .raw_get::<Table>("string")
        .unwrap()
        .raw_get::<Function>("format")
        .unwrap()
        .call(("%q", pattern))
        .unwrap();
    let text = runtime::verified("src/Modules/ModParser.lua").unwrap();
    let declaration = text
        .split_inclusive('\n')
        .skip(12)
        .take(3)
        .collect::<String>();
    assert!(declaration.starts_with("local function firstToUpper(str)\n"));
    assert!(declaration.trim_end().ends_with("\nend"));
    let old = "str:gsub(\"^%l\", string.upper)";
    assert_eq!(declaration.matches(old).count(), 1);
    assert_eq!(text.matches(old).count(), 1);
    let configured = declaration.replacen(old, &format!("str:gsub({quoted}, string.upper)"), 1);
    assert_eq!(configured.lines().count(), declaration.lines().count());
    // Isolated configured-helper evidence: the complete verified declaration is
    // retained with only its authored pattern literal changed. The original
    // module is not reexecuted, so this does not claim malformed patterns can
    // survive its earlier initialization uses of firstToUpper.
    let upper: Function = lua
        .load(format!(
            "{}{}\nreturn firstToUpper",
            "\n".repeat(12),
            configured
        ))
        .set_name("@src/Modules/ModParser.lua")
        .set_environment(lua.globals())
        .eval()
        .unwrap();
    let info = upper.info();
    assert_eq!(info.source.as_deref(), Some("@src/Modules/ModParser.lua"));
    assert_eq!(info.line_defined, Some(13));
    assert_eq!(info.last_line_defined, Some(15));
    assert_eq!(info.num_upvalues, 0);
    assert_eq!(
        upper.environment().unwrap().to_pointer(),
        lua.globals().to_pointer()
    );
    assert_ne!(upper, original.upper);
    let count = original.helper.info().num_upvalues;
    assert!(count <= 16, "retained helper capture bound");
    let mut target = None;
    for index in 1..=i32::from(count) {
        let (mut stack_ready, mut named) = (false, false);
        // SAFETY: the retained original Function is the only argument. Reserve
        // stack space before reading a bounded existing slot, copy only a
        // non-null name, and leave exactly name/value for mlua conversion.
        let result: mlua::Result<(mlua::LuaString, Value)> = unsafe {
            lua.exec_raw(original.helper.clone(), |state| {
                stack_ready = mlua::ffi::lua_checkstack(state, 3) != 0;
                if !stack_ready {
                    return;
                }
                let name = mlua::ffi::lua_getupvalue(state, 1, index);
                named = !name.is_null();
                if !named {
                    return;
                }
                mlua::ffi::lua_pushstring(state, name);
                mlua::ffi::lua_insert(state, -2);
                mlua::ffi::lua_remove(state, 1);
            })
        };
        assert!(stack_ready && named, "retained capture read failed");
        let (name, value) = result.unwrap();
        if name.as_bytes().as_ref() == b"firstToUpper" {
            assert_eq!(value.as_function(), Some(&original.upper));
            assert!(target.replace(index).is_none(), "ambiguous actual capture");
        }
    }
    let index = target.expect("actual firstToUpper capture");
    let (mut stack_ready, mut named) = (false, false);
    // SAFETY: these are the retained original helper and the authenticated
    // configured helper. lua_setupvalue is the C operation behind debug.setupvalue;
    // only the just-verified capture slot is changed, without loading a debug
    // library or changing any original function body/global environment.
    let result: mlua::Result<mlua::LuaString> = unsafe {
        lua.exec_raw((original.helper.clone(), upper.clone()), |state| {
            stack_ready = mlua::ffi::lua_checkstack(state, 2) != 0;
            if !stack_ready {
                return;
            }
            let name = mlua::ffi::lua_setupvalue(state, 1, index);
            named = !name.is_null();
            if !named {
                return;
            }
            mlua::ffi::lua_pushstring(state, name);
            mlua::ffi::lua_remove(state, 1);
        })
    };
    assert!(stack_ready && named, "retained capture write failed");
    assert_eq!(result.unwrap().as_bytes().as_ref(), b"firstToUpper");
    assert_eq!(
        upvalue(lua, &original.helper, "firstToUpper").as_function(),
        Some(&upper)
    );
    original.upper = upper;
    original.verify();
    let extraction = raw_source::extraction();
    let mut data = extraction.catalog().owner().data().clone();
    data.programs = ParserProgramPayload::default();
    data.policy.first_to_upper_pattern = pattern.into();
    let owner = ModifierParserCatalog::new(data).unwrap();
    let catalog = ParserProgramCatalog::new(extraction.catalog().data().clone(), owner).unwrap();
    (original, CompiledParserPrograms::new(&catalog).unwrap())
}

#[test]
fn injected_pattern_captures_bytes_empty_matches_and_errors_match_complete_configured_source() {
    for pattern in [
        "^%l", "%l", "(.)", "()", "()%l", "(%l)()", "", ".", "^", "$", "a*", "%f[%l]", "[", "(%l",
    ] {
        let (original, plan) = configured_pattern(pattern);
        for kind in [b"ab cd".as_slice(), b"", b"a\0z", b"\xce\xbb\xffa"] {
            let args = MultiValue::from_vec(vec![
                Value::Number(37.0),
                Value::Number(10.0),
                original.text(kind),
            ]);
            let args = source_inputs(&original, args);
            let input = raw_source::from_source(&observer::graph(&original.source, args.clone()));
            let source = original.helper.call::<MultiValue>(args);
            let native = plan.execute(
                callback_id(plan.catalog().owner(), &original.helper),
                &input,
                ProgramLimits::default(),
            );
            match source {
                Ok(values) => assert_eq!(
                    raw_source::native_graph(native.unwrap().graph()),
                    observer::graph(&original.source, values),
                    "pattern {pattern:?}, kind {kind:?}"
                ),
                Err(error) => {
                    assert!(
                        observer::source_error_at(&error, "", 14),
                        "unexpected source/host error: {error}"
                    );
                    assert_eq!(
                        native.unwrap_err().kind,
                        ProgramRuntimeErrorKind::Source,
                        "pattern {pattern:?}"
                    );
                }
            }
        }
    }
}
