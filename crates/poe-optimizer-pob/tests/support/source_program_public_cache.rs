//! Original public-parser cache observations on the same source/native session.
//! No inner parser replacement, result injection, or cache traversal fallback.
use super::*;
use mlua::{HookTriggers, VmState};
use std::sync::{Arc, Mutex};

pub(super) fn frontier(pair: &Pair, error: &ProgramRuntimeError) -> Json {
    json!({"declaration":error.callback.and_then(|id|pair.observed.owner().callback(id)).map(|c|&c.kind),
        "kind":format!("{:?}",error.kind),"message":error.message,"callback":error.callback,"location":error.location})
}
fn lookup(pair: &mut Pair, key: &Value) -> SessionValue {
    let key = pair.args(std::slice::from_ref(key)).remove(0);
    pair.call("probe.lookup", &[pair.root("public.cache"), key])
        .unwrap()
        .remove(0)
}
fn replace(pair: &mut Pair, key: &Value, value: &Value) {
    let args = pair.args(&[key.clone(), value.clone()]);
    pair.call(
        "probe.replace",
        &[pair.root("public.cache"), args[0].clone(), args[1].clone()],
    )
    .unwrap();
}
fn same(pair: &mut Pair, left: &SessionValue, right: &SessionValue) -> bool {
    let result = pair
        .call("probe.distinct", &[left.clone(), right.clone()])
        .unwrap();
    !pair.boolean(&result)
}
fn plain(value: &Value, depth: usize, seen: &mut BTreeSet<usize>) -> bool {
    if depth > 20 {
        return false;
    }
    match value {
        Value::Nil | Value::Boolean(_) | Value::Integer(_) | Value::Number(_) => true,
        Value::String(v) => v.to_str().is_ok(),
        Value::Table(t) => {
            if t.metatable().is_some() || !seen.insert(t.to_pointer() as usize) {
                return false;
            }
            t.clone().pairs::<Value, Value>().all(|row| {
                let (key, value) = row.unwrap();
                matches!(key, Value::Integer(_) | Value::Number(_) | Value::String(_))
                    && plain(&key, depth + 1, seen)
                    && plain(&value, depth + 1, seen)
            })
        }
        _ => false,
    }
}
fn compare(pair: &mut Pair, source: &[Value], native: &[SessionValue], label: &str) -> Json {
    assert_eq!(source.len(), native.len(), "public result arity: {label}");
    let graph = pair.plain(native);
    assert_eq!(
        graph,
        observation::canonical(&observation::capture(source)),
        "public graph: {label}"
    );
    graph
}
fn paired_call(
    lua: &Lua,
    parser: &Function,
    pair: &mut Pair,
    key: &Value,
    combined: bool,
    label: &str,
) -> (Vec<Value>, Vec<SessionValue>, usize, Json) {
    let calls = Arc::new(Mutex::new(0usize));
    let count = calls.clone();
    lua.set_hook(HookTriggers::EVERY_LINE, move |_, debug| {
        if debug
            .source()
            .source
            .as_deref()
            .is_some_and(|p| p.ends_with("Modules/ModParser.lua"))
            && debug.current_line() == Some(6621)
        {
            *count.lock().unwrap() += 1;
        }
        Ok(VmState::Continue)
    })
    .unwrap();
    let result = parser.call::<MultiValue>((key.clone(), combined));
    lua.remove_hook();
    let source = result
        .unwrap_or_else(|e| panic!("original public {label}: {e}"))
        .into_vec();
    let args = pair.args(&[key.clone(), Value::Boolean(combined)]);
    let native = pair
        .call("original.parser", &args)
        .unwrap_or_else(|e| panic!("native public {label}: {e}"));
    let graph = compare(pair, &source, &native, label);
    let count = *calls.lock().unwrap();
    (source, native, count, graph)
}
fn cache_row(
    cache: &Table,
    pair: &mut Pair,
    key: &Value,
    label: &str,
) -> (Value, SessionValue, Json) {
    let source = cache.raw_get::<Value>(key.clone()).unwrap();
    let native = lookup(pair, key);
    let graph = compare(
        pair,
        std::slice::from_ref(&source),
        std::slice::from_ref(&native),
        label,
    );
    (source, native, graph)
}
pub(super) fn run(lua: &Lua, primitives: &Primitives, parser: &Function, pair: &mut Pair) -> Json {
    let cache: Table = lua
        .globals()
        .raw_get::<Table>("modLib")
        .unwrap()
        .raw_get("parseModCache")
        .unwrap();
    assert_eq!(
        primitives.captured_value(parser, "cache"),
        Value::Table(cache.clone())
    );
    // Choose an actual initialized success row with an observable plain graph.
    // The source row must predate observation: no cache result is fabricated.
    // Keep only one rooted candidate while walking the initialized cache;
    // collecting thousands of Lua table handles can exhaust mlua's ref arena.
    let mut eligible: Option<(mlua::LuaString, Table)> = None;
    for entry in cache.clone().pairs::<mlua::LuaString, Table>() {
        let (key, row) = entry.unwrap();
        if row
            .raw_get::<Value>(1)
            .unwrap()
            .as_table()
            .is_some_and(|mods| !mods.is_empty())
            && plain(&Value::Table(row.clone()), 0, &mut BTreeSet::new())
            && eligible
                .as_ref()
                .is_none_or(|(previous, _)| key.as_bytes().as_ref() < previous.as_bytes().as_ref())
        {
            eligible = Some((key, row));
        }
    }
    let (hit_key, hit_row) = eligible.expect("initialized plain successful cache row");
    let hit_key = Value::String(hit_key);
    let (_, native_row, before_graph) = cache_row(&cache, pair, &hit_key, "initialized row");
    let (first_source, first_native, first_calls, first_graph) =
        paired_call(lua, parser, pair, &hit_key, false, "initialized hit");
    assert_eq!(first_calls, 0);
    assert!(first_source[0].as_table().is_some());
    let (second_source, second_native, second_calls, _) =
        paired_call(lua, parser, pair, &hit_key, true, "combined hit");
    assert_eq!(second_calls, 0);
    assert_ne!(first_source[0], second_source[0]);
    assert!(!same(pair, &first_native[0], &second_native[0]));
    let current = lookup(pair, &hit_key);
    assert!(same(pair, &native_row, &current));
    // Compare cache plus both returns together so every nested alias relation
    // remains visible, in addition to the scalar/bit-exact standalone packs.
    let mut source_graph = vec![Value::Table(hit_row.clone())];
    source_graph.extend(first_source.clone());
    source_graph.extend(second_source.clone());
    let mut native_graph = vec![native_row.clone()];
    native_graph.extend(first_native.clone());
    native_graph.extend(second_native.clone());
    compare(
        pair,
        &source_graph,
        &native_graph,
        "cache and independent returned copies",
    );
    let marker = pair.args(&[Value::Integer(123456)]).remove(0);
    pair.call("probe.mutate", &[first_native[0].clone(), marker])
        .unwrap();
    first_source[0]
        .as_table()
        .unwrap()
        .raw_set(1, 123456)
        .unwrap();
    compare(
        pair,
        &source_graph,
        &native_graph,
        "caller mutation stays in first returned copy",
    );
    assert_eq!(
        cache_row(&cache, pair, &hit_key, "unchanged initialized row").2,
        before_graph
    );
    compare(
        pair,
        &second_source,
        &second_native,
        "second returned copy unchanged",
    );
    let mut cases = vec![
        json!({"case":"initialized_hit","key_bytes":hit_key.as_string().unwrap().as_bytes().to_vec(),
        "result":first_graph,"source_inner_calls":[first_calls,second_calls],"cache_row":before_graph,
        "copy_graph_aliases_compared":true,"caller_mutation_isolated":true}),
    ];
    let mut saved = Vec::new();
    for (text, expected_calls, empty_modifiers) in [
        ("native parser readiness sentinel never matches", 1, false),
        ("NATIVE PARSER READINESS SENTINEL NEVER MATCHES", 1, false),
        ("20% increased not a stat", 2, true),
    ] {
        let key = Value::String(lua.create_string(text).unwrap());
        assert_eq!(cache.raw_get::<Value>(key.clone()).unwrap(), Value::Nil);
        let absent = lookup(pair, &key);
        compare(pair, &[Value::Nil], &[absent], "initial cache miss");
        let (source, native, calls, result) =
            paired_call(lua, parser, pair, &key, false, "no-match miss");
        assert_eq!(calls, expected_calls);
        assert_eq!(source.len(), 2);
        if empty_modifiers {
            assert!(source[0].as_table().is_some_and(Table::is_empty));
        } else {
            assert_eq!(source[0], Value::Nil);
        }
        assert!(source[1].as_string().is_some());
        let (old_source, old_native, row) = cache_row(&cache, pair, &key, "fresh no-match row");
        let (hit_source, hit_native, hit_calls, hit_result) =
            paired_call(lua, parser, pair, &key, true, "no-match hit");
        assert_eq!(hit_calls, 0);
        assert_eq!(hit_result, result);
        compare(pair, &source, &native, "retained first miss pack");
        compare(pair, &hit_source, &hit_native, "retained repeated pack");
        assert_eq!(cache.raw_get::<Value>(key.clone()).unwrap(), old_source);
        let current = lookup(pair, &key);
        assert!(same(pair, &old_native, &current));
        cache.raw_set(key.clone(), Value::Nil).unwrap();
        replace(pair, &key, &Value::Nil);
        let (_, _, evict_calls, evict_result) =
            paired_call(lua, parser, pair, &key, false, "no-match after eviction");
        assert_eq!(evict_calls, expected_calls);
        assert_eq!(evict_result, result);
        let (new_source, new_native, new_row) =
            cache_row(&cache, pair, &key, "recreated no-match row");
        assert_ne!(new_source, old_source);
        assert!(!same(pair, &old_native, &new_native));
        assert_eq!(new_row, row);
        saved.push((key.clone(), new_source, new_native));
        cases.push(json!({"case":if empty_modifiers { "truthy_empty_retry_hit_eviction" } else { "no_match_miss_hit_eviction" },"key":text,"result":result,"row":row,
            "source_inner_calls":[calls,hit_calls,evict_calls],"same_row_on_hit":true,"fresh_row_after_eviction":true}));
    }
    assert_ne!(saved[0].1, saved[1].1);
    assert!(!same(pair, &saved[0].2, &saved[1].2));
    let mut errors = Vec::new();
    for value in [Value::Nil, Value::Boolean(false)] {
        let source = parser
            .call::<MultiValue>((value.clone(), false))
            .unwrap_err();
        let args = pair.args(&[value.clone(), Value::Boolean(false)]);
        let before = (
            pair.session.steps(),
            pair.session.pattern_steps(),
            pair.session.allocations(),
        );
        let native = pair.call("original.parser", &args).unwrap_err();
        assert_eq!(
            native.kind,
            ProgramRuntimeErrorKind::Source,
            "{source}; {native}"
        );
        assert!(pair.session.steps() > before.0 && pair.session.pattern_steps() >= before.1);
        assert!(pair.session.allocations().values >= before.2.values);
        cache_row(&cache, pair, &value, "invalid input cache prefix unchanged");
        for (key, source, native) in &saved {
            assert_eq!(cache.raw_get::<Value>(key.clone()).unwrap(), *source);
            let row = lookup(pair, key);
            assert!(same(pair, native, &row));
        }
        errors.push(json!({"input":format!("{value:?}"),"source":source.to_string(),"native":native.message,"cache_prefix_unchanged":true,"cumulative_charges_retained":true}));
    }
    // Exercise distinct positive families through the actual public parser.
    // These strings are oracle fixtures only; production still consumes data.
    // Keep each reached dependency instead of assuming successful misses fail
    // or declaring the public milestone complete from these finite cases.
    let mut success_misses = Vec::new();
    let mut success_keys = Vec::new();
    for text in [
        "+987654 to Strength",
        "123% increased maximum Life",
        "+76% to Fire Resistance",
        "37% increased Damage",
        "Adds 13 to 17 Fire Damage",
        "987653% increased Attack Speed",
    ] {
        let key = Value::String(lua.create_string(text).unwrap());
        assert_eq!(
            cache.raw_get::<Value>(key.clone()).unwrap(),
            Value::Nil,
            "positive fixture must begin as an original cache miss: {text}"
        );
        let initial_native_row = lookup(pair, &key);
        compare(
            pair,
            &[Value::Nil],
            &[initial_native_row],
            &format!("positive fixture must begin as a native cache miss: {text}"),
        );
        let source = parser
            .call::<MultiValue>((key.clone(), false))
            .unwrap_or_else(|error| panic!("original positive miss {text}: {error}"))
            .into_vec();
        assert!(
            source
                .first()
                .and_then(Value::as_table)
                .is_some_and(|mods| !mods.is_empty()),
            "original fixture must produce nonempty modifiers: {text}"
        );
        let source_row = cache.raw_get::<Value>(key.clone()).unwrap();
        assert!(
            source_row.as_table().is_some(),
            "original cache write: {text}"
        );
        let original_result = observation::canonical(&observation::capture(&source));
        let original_cache_row =
            observation::canonical(&observation::capture(std::slice::from_ref(&source_row)));
        let args = pair.args(&[key.clone(), Value::Boolean(false)]);
        let result = pair.call("original.parser", &args);
        let native_row = lookup(pair, &key);
        let prefix = pair.plain(std::slice::from_ref(&native_row));
        let absent = observation::canonical(&observation::capture(&[Value::Nil]));
        let committed = prefix != absent;
        if committed {
            compare(
                pair,
                std::slice::from_ref(&source_row),
                std::slice::from_ref(&native_row),
                &format!("positive miss committed cache prefix: {text}"),
            );
        }
        let (matched_result, dependency) = match result {
            Ok(native) => {
                assert!(
                    committed,
                    "completed positive miss must commit its cache row: {text}"
                );
                let matched = compare(
                    pair,
                    &source,
                    &native,
                    &format!("complete positive miss: {text}"),
                );
                // Compare the cache and return together to retain nested alias
                // observations, including the source's independently copied result.
                let mut source_graph = vec![source_row];
                source_graph.extend(source);
                let mut native_graph = vec![native_row];
                native_graph.extend(native);
                compare(
                    pair,
                    &source_graph,
                    &native_graph,
                    &format!("positive cache and returned copy: {text}"),
                );
                (Some(matched), None)
            }
            Err(error) => {
                assert_eq!(
                    error.kind,
                    ProgramRuntimeErrorKind::UnsupportedCapability,
                    "positive miss {text}: {error}"
                );
                (None, Some(frontier(pair, &error)))
            }
        };
        success_misses.push(json!({
            "key":text,
            "original_result":original_result,
            "original_cache_row":original_cache_row,
            "native_cache_row":prefix,
            "cache_write_reached":committed,
            "complete":matched_result.is_some(),
            "matched_result":matched_result,
            "frontier":dependency,
        }));
        success_keys.push(key);
    }
    // Restore only entries introduced by this oracle in both heaps. Keep the
    // state paired even when a native frontier preceded the cache write.
    for key in saved.into_iter().map(|(key, _, _)| key).chain(success_keys) {
        cache.raw_set(key.clone(), Value::Nil).unwrap();
        replace(pair, &key, &Value::Nil);
        cache_row(
            &cache,
            pair,
            &key,
            "introduced cache entry restored to absent",
        );
        assert_eq!(cache.raw_get::<Value>(key).unwrap(), Value::Nil);
    }
    assert_eq!(
        cache.raw_get::<Value>(hit_key.clone()).unwrap(),
        Value::Table(hit_row)
    );
    let current = lookup(pair, &hit_key);
    assert!(same(pair, &native_row, &current));
    assert_eq!(
        cache_row(&cache, pair, &hit_key, "initialized row after restoration").2,
        before_graph
    );
    assert_eq!(
        primitives.captured_value(parser, "cache"),
        Value::Table(cache.clone())
    );
    assert_eq!(
        lua.globals()
            .raw_get::<Table>("modLib")
            .unwrap()
            .raw_get::<Value>("parseModCache")
            .unwrap(),
        Value::Table(cache)
    );
    let success_miss = success_misses[0].clone();
    let dependency = success_miss["frontier"].clone();
    json!({"cases":cases,"source_errors":errors,"paired_public_calls":11,"frontier":dependency,
        "success_miss":success_miss,"success_misses":success_misses,
        "introduced_cache_entries_restored":true,"published_cache_alias_retained":true,"complete_public_parser":false})
}
