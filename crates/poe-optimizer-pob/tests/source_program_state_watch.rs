//! Test-only guard mechanics; actual original-parser breadth is a separate gate.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/source_program_state_watch.rs"]
mod watch;
use mlua::{Function, Lua, Table, Value};
use watch::{StateWatchFactory, StateWatchLimits};

fn host() -> Lua {
    // Tests need original debug APIs to exercise changed cells/environments.
    unsafe { Lua::unsafe_new() }
}
fn fixture(lua: &Lua) -> (Function, Table, Table) {
    lua.load(
        r#"
        local cache = {}
        local dictionary = {modNameList={attributes={'Str','Dex'}}}
        local function parser(key)
            if cache[key] then return cache[key] end
            return dictionary.modNameList[key]
        end
        return parser, cache, dictionary
        "#,
    )
    .eval()
    .unwrap()
}

#[test]
fn shared_dictionary_mutation_is_detected_after_gc_without_invoking_the_parser() {
    let lua = host();
    let factory = StateWatchFactory::before_source(&lua).unwrap();
    let (parser, cache, dictionary) = fixture(&lua);
    let guard = factory.watch(&parser, &cache).unwrap();
    lua.gc_collect().unwrap();
    assert_eq!(guard.changed().unwrap(), None);
    let names: Table = dictionary
        .raw_get::<Table>("modNameList")
        .unwrap()
        .raw_get("attributes")
        .unwrap();
    // Same alias write as original ModParser's DOUBLED branch, not a root swap.
    names.raw_set(2, "Multiplier:StrDoubled").unwrap();
    assert!(guard.changed().unwrap().unwrap().contains("raw entry"));
    names.raw_set(2, "Dex").unwrap();
    assert_eq!(guard.changed().unwrap(), None);
    names.raw_set(3, false).unwrap();
    assert!(guard.changed().unwrap().unwrap().contains("raw length"));
}

#[test]
fn cache_bindings_restore_but_shared_descendants_and_second_cache_routes_are_watched() {
    let lua = host();
    let factory = StateWatchFactory::before_source(&lua).unwrap();
    let (parser, cache, dictionary) = fixture(&lua);
    let names: Table = dictionary
        .raw_get::<Table>("modNameList")
        .unwrap()
        .raw_get("attributes")
        .unwrap();
    let original = lua.create_table().unwrap();
    original.raw_set(1, names.clone()).unwrap();
    cache.raw_set("saved", original.clone()).unwrap();
    cache.raw_set("false", false).unwrap();
    let guard = factory.watch(&parser, &cache).unwrap();
    cache.raw_set("saved", 42).unwrap();
    cache.raw_set("false", Value::Nil).unwrap();
    cache.raw_set("new", lua.create_table().unwrap()).unwrap();
    guard.restore_cache_bindings().unwrap();
    assert_eq!(cache.raw_get::<Table>("saved").unwrap(), original);
    assert!(!cache.raw_get::<bool>("false").unwrap());
    assert!(matches!(cache.raw_get::<Value>("new").unwrap(), Value::Nil));
    assert_eq!(guard.changed().unwrap(), None);
    names.raw_set(1, "changed through the cache row").unwrap();
    guard.restore_cache_bindings().unwrap();
    assert!(guard.changed().unwrap().is_some());

    // Only parser's named cache upvalue edge is skipped; this second path is not.
    dictionary
        .raw_set("other_cache_route", cache.clone())
        .unwrap();
    let guard = factory.watch(&parser, &cache).unwrap();
    cache.raw_set("saved", 55).unwrap();
    assert!(guard.changed().unwrap().is_some());
}

#[test]
fn scalar_capture_and_equal_value_cell_rebinding_are_distinct_changes() {
    let lua = host();
    let factory = StateWatchFactory::before_source(&lua).unwrap();
    let (parser, cache, setter, second, distinct): (Function, Table, Function, Function, Function) =
        lua.load(
            r#"
        local cache, value = {}, 4
        local function first() return value end
        local function second() return value end
        local function factory(x) return function() return x end end
        local function parser(key) return cache[key], first, second end
        return parser, cache, function(x) value=x end, second, factory(4)
    "#,
        )
        .eval()
        .unwrap();
    let guard = factory.watch(&parser, &cache).unwrap();
    setter.call::<()>(5).unwrap();
    assert!(guard.changed().unwrap().unwrap().contains("upvalue"));
    setter.call::<()>(4).unwrap();
    assert_eq!(guard.changed().unwrap(), None);
    let join: Function = lua
        .globals()
        .raw_get::<Table>("debug")
        .unwrap()
        .raw_get("upvaluejoin")
        .unwrap();
    join.call::<()>((second, 1, distinct, 1)).unwrap();
    assert!(guard.changed().unwrap().unwrap().contains("upvalue"));
}

#[test]
fn metatable_contents_and_projected_global_bindings_are_checked_without_behavior_calls() {
    let lua = host();
    let factory = StateWatchFactory::before_source(&lua).unwrap();
    let (parser, cache, dictionary) = fixture(&lua);
    let (meta, effects): (Table, Table) = lua
        .load(
            r#"
        local effects={calls=0}
        return {__metatable='protected', marker=1,
            __index=function() effects.calls=effects.calls+1; error('must not run') end,
            __len=function() effects.calls=effects.calls+1; error('must not run') end,
            __tostring=function() effects.calls=effects.calls+1; error('must not run') end}, effects
    "#,
        )
        .eval()
        .unwrap();
    dictionary.set_metatable(Some(meta.clone())).unwrap();
    let guard = factory.watch(&parser, &cache).unwrap();
    assert_eq!(guard.changed().unwrap(), None);
    assert_eq!(effects.raw_get::<i32>("calls").unwrap(), 0);
    meta.raw_set("marker", 2).unwrap();
    assert!(guard.changed().unwrap().is_some());
    meta.raw_set("marker", 1).unwrap();
    assert_eq!(guard.changed().unwrap(), None);
    lua.globals().raw_set("itemSlotName", "main hand").unwrap();
    assert_eq!(
        guard.changed().unwrap(),
        Some("binding itemSlotName".into())
    );
    assert_eq!(effects.raw_get::<i32>("calls").unwrap(), 0);
}

#[test]
fn global_data_projection_and_function_environment_changes_cannot_pass_as_unchanged() {
    let lua = host();
    let factory = StateWatchFactory::before_source(&lua).unwrap();
    let (parser, cache, _) = fixture(&lua);
    let data: Table = lua
        .load("return {gems={g={grantedEffectId='a'}},unconsumed={}}")
        .eval()
        .unwrap();
    lua.globals().raw_set("data", data.clone()).unwrap();
    let guard = factory.watch(&parser, &cache).unwrap();
    data.raw_get::<Table>("unconsumed")
        .unwrap()
        .raw_set("ignored", 3)
        .unwrap();
    assert_eq!(guard.changed().unwrap(), None);
    let gem: Table = data.raw_get::<Table>("gems").unwrap().raw_get("g").unwrap();
    gem.raw_set("grantedEffectId", "b").unwrap();
    assert!(guard.changed().unwrap().is_some());
    gem.raw_set("grantedEffectId", "a").unwrap();
    assert_eq!(guard.changed().unwrap(), None);
    parser.set_environment(lua.create_table().unwrap()).unwrap();
    assert!(guard.changed().is_err());
}

#[test]
fn bounds_fail_closed_and_cache_restore_preflights_before_any_write() {
    let lua = host();
    let factory = StateWatchFactory::before_source(&lua).unwrap();
    let (parser, cache, dictionary) = fixture(&lua);
    let limits = StateWatchLimits {
        max_nodes: 1,
        ..StateWatchLimits::default()
    };
    assert!(factory.watch_with_limits(&parser, &cache, limits).is_err());
    let limits = StateWatchLimits {
        max_bytes: 1,
        ..StateWatchLimits::default()
    };
    assert!(factory.watch_with_limits(&parser, &cache, limits).is_err());
    let limits = StateWatchLimits {
        max_entries: 1,
        ..StateWatchLimits::default()
    };
    assert!(factory.watch_with_limits(&parser, &cache, limits).is_err());
    cache.raw_set("original", 1).unwrap();
    let limits = StateWatchLimits {
        max_cache_entries: 1,
        ..StateWatchLimits::default()
    };
    let guard = factory.watch_with_limits(&parser, &cache, limits).unwrap();
    cache.raw_set("original", 2).unwrap();
    cache.raw_set("extra", 3).unwrap();
    assert!(guard.restore_cache_bindings().is_err());
    assert_eq!(cache.raw_get::<i32>("original").unwrap(), 2);
    assert_eq!(cache.raw_get::<i32>("extra").unwrap(), 3);
    let object: Value = lua.load("return newproxy(true)").eval().unwrap();
    dictionary.raw_set("opaque", object).unwrap();
    assert!(factory.watch(&parser, &cache).is_err());
    lua.globals().raw_set("foo", true).unwrap();
    assert!(factory.watch(&parser, &cache).is_err());
}

#[test]
fn primitive_rebinding_cannot_make_cache_restoration_silently_do_nothing() {
    let lua = host();
    let factory = StateWatchFactory::before_source(&lua).unwrap();
    let (parser, cache, _) = fixture(&lua);
    cache.raw_set("saved", 1).unwrap();
    let original: Function = lua.globals().raw_get("rawset").unwrap();
    let wrong: Function = lua.globals().raw_get("rawget").unwrap();
    lua.globals().raw_set("rawset", wrong.clone()).unwrap();
    assert!(factory.watch(&parser, &cache).is_err());
    lua.globals().raw_set("rawset", original).unwrap();
    let guard = factory.watch(&parser, &cache).unwrap();
    cache.raw_set("saved", 2).unwrap();
    cache.raw_set("extra", 3).unwrap();
    lua.globals().raw_set("rawset", wrong).unwrap();
    // An existing watch uses its retained actual primitive, never the rebound one.
    guard.restore_cache_bindings().unwrap();
    assert_eq!(cache.raw_get::<i32>("saved").unwrap(), 1);
    assert!(matches!(
        cache.raw_get::<Value>("extra").unwrap(),
        Value::Nil
    ));
    assert_eq!(guard.changed().unwrap(), Some("binding rawset".into()));
}
