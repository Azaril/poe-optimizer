//! Scalar DOUBLED results compared with the complete authenticated public parser.
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
use factory_source::{FactorySource, upvalue};
use mlua::{Function, Table, Value};
use poe_optimizer_data::{
    game_data::bundled_snapshot,
    modifier_parser::{ModifierParserCatalog, ModifierParserData, ParserDictionary, ParserValue},
};
use poe_optimizer_engine::{
    lua_pattern::MatchBudget,
    modifier_parser::{CompiledModifierParser, ParseOutcome, ParserError},
};

fn compile(data: ModifierParserData) -> CompiledModifierParser {
    CompiledModifierParser::new(&ModifierParserCatalog::new(data).unwrap()).unwrap()
}
fn source_concat_error(error: &mlua::Error) -> bool {
    match error {
        mlua::Error::RuntimeError(message) => {
            message.contains("attempt to concatenate") && !message.contains("oracle deadline")
        }
        mlua::Error::CallbackError { cause, .. } => source_concat_error(cause),
        _ => false,
    }
}
fn compare(
    source: &FactorySource,
    native: &CompiledModifierParser,
    text: &[u8],
) -> Option<ParseOutcome> {
    let expected = source.public.raw(text);
    let actual = native.parse(text, &mut MatchBudget::default());
    match (expected, actual) {
        (Ok(expected), Ok(actual)) => {
            assert_eq!(
                source.public.capture(expected).unwrap(),
                native_observer::capture(&actual, native.catalog()).unwrap(),
                "{text:?}"
            );
            Some(actual)
        }
        (Err(expected), Err(ParserError::SourceError(actual))) => {
            // Only the expected original scalar concatenation failure counts.
            // Host deadlines, memory/stack errors and observer failures must fail.
            assert!(
                source_concat_error(&expected),
                "unexpected source/host error: {expected}"
            );
            assert_eq!(actual, "concatenation of a non-string value");
            None
        }
        (expected, actual) => {
            panic!("DOUBLED mismatch {text:?}: source={expected:?}, native={actual:?}")
        }
    }
}
fn name(source: &FactorySource, data: &mut ModifierParserData, value: ParserValue) {
    let lua_value = match &value {
        ParserValue::Text(v) => source.text(v),
        ParserValue::Number(v) => Value::Number(*v),
        ParserValue::Boolean(v) => Value::Boolean(*v),
        ParserValue::Nil => Value::Nil,
        _ => panic!("scalar fixture"),
    };
    source
        .dictionary("modNameList")
        .set("__caller_stat__", lua_value)
        .unwrap();
    let fields =
        &mut data.tables[data.dictionaries[&ParserDictionary::ModName].0 as usize - 1].fields;
    if matches!(value, ParserValue::Nil) {
        fields.remove("__caller_stat__");
    } else {
        fields.insert("__caller_stat__".into(), value);
    }
    // These cases author dictionary state directly. They exercise the native
    // ordinary parser with original full-source control, not program acquisition.
    data.programs = Default::default();
}
fn returned(source: &FactorySource, text: &[u8]) -> Table {
    source
        .public
        .raw(text)
        .unwrap()
        .front()
        .unwrap()
        .as_table()
        .unwrap()
        .clone()
}

#[test]
fn four_actual_rune_lines_match_full_public_graphs_and_cache_copy_history() {
    let snapshot = bundled_snapshot().unwrap();
    let native = CompiledModifierParser::new(snapshot.modifier_parser()).unwrap();
    let source = FactorySource::new();
    let names = source.dictionary("modNameList");
    let cases: [(&[u8], &str, &str); 4] = [
        (
            b"Causes Double Stun Buildup",
            "stun buildup",
            "EnemyHeavyStunBuildup",
        ),
        (
            b"Flammability Magnitude is doubled",
            "flammability magnitude",
            "EnemyIgniteChance",
        ),
        (
            b"Double Stun Threshold while Shield is Raised",
            "stun threshold",
            "StunThreshold",
        ),
        (
            b"Runic Ward Regeneration Rate is doubled",
            "runic ward regeneration rate",
            "WardRegen",
        ),
    ];
    for (text, pattern, expected_name) in cases {
        assert_eq!(
            names.get::<String>(pattern).unwrap(),
            expected_name,
            "actual scalar dictionary entry"
        );
        let first = compare(&source, &native, text).unwrap();
        assert_eq!(first.modifiers.as_ref().unwrap().indexed.len(), 2);
        // A caller may edit the recursively copied public return. It cannot
        // change the source cache, dictionary, or a subsequent native result.
        let rows = returned(&source, text);
        let first_row: Table = rows.get(1).unwrap();
        first_row.set("name", "__caller_mutation__").unwrap();
        let limit: Table = first_row.get(1).unwrap();
        limit.set("var", "__caller_limit_mutation__").unwrap();
        assert_eq!(compare(&source, &native, text).unwrap(), first);
        assert_eq!(names.get::<String>(pattern).unwrap(), expected_name);
        source
            .public
            .cache
            .set(
                source.public.source.lua.create_string(text).unwrap(),
                Value::Nil,
            )
            .unwrap();
        assert_eq!(
            compare(&source, &native, text).unwrap(),
            first,
            "uncached reparse"
        );
    }
}

#[test]
fn injected_scalar_names_keep_lua_coercion_fallthrough_and_errors() {
    let snapshot = bundled_snapshot().unwrap();
    let cases = [
        ParserValue::Text("CallerStat".into()),
        ParserValue::Text("".into()),
        ParserValue::Text("Unicode-\u{03bb}-tail".into()),
        ParserValue::Number(17.25),
        ParserValue::Number(-0.0),
        ParserValue::Boolean(false),
        ParserValue::Nil,
        ParserValue::Boolean(true),
    ];
    let source = FactorySource::new();
    for value in cases {
        let mut data = snapshot.modifier_parser().data().clone();
        name(&source, &mut data, value.clone());
        let native = compile(data);
        // Original dictionary state changes require explicit source-cache eviction.
        for text in [
            b"__caller_stat__ is doubled".as_slice(),
            b"Causes Double __caller_stat__",
            b"Double __caller_stat__",
        ] {
            source
                .public
                .cache
                .set(
                    source.public.source.lua.create_string(text).unwrap(),
                    Value::Nil,
                )
                .unwrap();
            let result = compare(&source, &native, text);
            assert_eq!(
                result.is_none(),
                matches!(value, ParserValue::Boolean(true))
            );
        }
    }
}

#[test]
fn injected_doubled_policy_matches_full_source_with_only_declared_operands_changed() {
    let snapshot = bundled_snapshot().unwrap();
    let mut data = snapshot.modifier_parser().data().clone();
    let mut source = FactorySource::new();
    let original = runtime::verified("src/Modules/ModParser.lua").unwrap();
    let begin = original.find("elseif modForm == \"DOUBLED\" then").unwrap();
    let end = begin + original[begin..].find("\n\tif not modName then").unwrap();
    let mut branch = original[begin..end].to_owned();
    for (old, new, count) in [
        ("\"Multiplier:\"", "\"CallerCounter:\"", 2),
        ("\"Doubled\"", "\"Scaled\"", 3),
        ("\"DoubledLimit\"", "\"ScaleCap\"", 1),
        ("modValue = { 100, 1 }", "modValue = { 125, 2 }", 1),
        ("globalLimit = 100", "globalLimit = 55", 1),
    ] {
        assert_eq!(
            branch.matches(old).count(),
            count,
            "configured operand {old}"
        );
        branch = branch.replace(old, new);
    }
    let configured = format!("{}{}{}", &original[..begin], branch, &original[end..]);
    // This labelled configuration fixture evaluates the full module. No branch,
    // emitter, public wrapper or callback implementation is substituted.
    let (parse, cache): (Function, Table) = source
        .public
        .source
        .lua
        .load(configured)
        .set_name("@src/Modules/ModParser.lua")
        .eval()
        .unwrap();
    source.public.parse = parse;
    source.public.cache = cache;
    source.internal = upvalue(&source.public.source.lua, &source.public.parse, "parseMod")
        .as_function()
        .unwrap()
        .clone();
    source.special = source.dictionary("specialModList");
    data.policy.doubled_multiplier_prefix = "CallerCounter:".into();
    data.policy.doubled_name_suffix = "Scaled".into();
    data.policy.doubled_limit_suffix = "ScaleCap".into();
    data.policy.doubled_more = 125.0;
    data.policy.doubled_override = 2.0;
    data.policy.doubled_global_limit = 55.0;
    name(&source, &mut data, ParserValue::Text("InjectedStat".into()));
    let native = compile(data);
    let result = compare(
        &source,
        &native,
        b"__caller_stat__ is doubled while on Full Life",
    )
    .unwrap();
    assert_eq!(result.modifiers.as_ref().unwrap().indexed.len(), 2);
    assert!(
        result.extra.is_none(),
        "condition must be represented on both emitted rows"
    );
}

#[test]
fn shared_table_name_mutation_is_observed_but_native_admission_stays_deferred() {
    let snapshot = bundled_snapshot().unwrap();
    let native = CompiledModifierParser::new(snapshot.modifier_parser()).unwrap();
    let source = FactorySource::new();
    let alias: Table = source
        .dictionary("modNameList")
        .get("strength and dexterity")
        .unwrap();
    assert_eq!(alias.get::<String>(2).unwrap(), "Dex");
    let line = b"Strength and Dexterity is doubled";
    let rows = returned(&source, line);
    assert_eq!(alias.get::<String>(2).unwrap(), "Multiplier:StrDoubled");
    assert_eq!(
        rows.get::<Table>(2).unwrap().get::<String>("name").unwrap(),
        "Multiplier:StrDoubled"
    );
    assert!(matches!(
        native.parse(line, &mut MatchBudget::default()),
        Err(ParserError::Deferred {
            stage: "shared dictionary mutation in doubled form",
            callback: None
        })
    ));
    // Existing mod_parser_public_parity/mod_parser_session_audit targets cover
    // the original cache/history and post-write-error behavior beyond this stop.
}
