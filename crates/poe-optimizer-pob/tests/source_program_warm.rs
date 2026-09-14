//! The warming oracle must bind evidence to the exact requested function.
#[path = "support/source_program_warm.rs"]
mod warm;
use mlua::{Function, Lua, Table, Value};

#[test]
fn invoked_wrapper_and_exact_nested_trace_target_are_distinct_obligations() {
    let lua = Lua::new();
    let functions: Table = lua
        .load(
            r#"
local function target(value)
    local total = value
    for i=1,8 do total = total + i end
    return total, nil, 'tail'
end
local function wrapper(value)
    local first, second, third = target(value)
    return {n=3, first, second, third}
end
local function unrelated(value)
    local total = value
    for i=1,8 do total = total + i end
    return total, nil, 'tail'
end
return {target=target, wrapper=wrapper, unrelated=unrelated}
"#,
        )
        .set_name("@tests/source_program_warm_fixture.lua")
        .eval()
        .unwrap();
    let target: Function = functions.raw_get("target").unwrap();
    let wrapper: Function = functions.raw_get("wrapper").unwrap();
    let unrelated: Function = functions.raw_get("unrelated").unwrap();
    let driver = warm::SourceWarmDriver::new(&lua).unwrap();
    let args = [Value::Integer(7)];
    let direct = driver.run(&lua, &target, &args, None).unwrap();
    assert!(direct.success);
    assert_eq!(lua.coerce_number(direct.value.clone()).unwrap(), Some(43.0));
    assert_eq!(direct.calls, 128);
    assert_eq!(direct.seed_calls, 0);
    assert!(direct.target_live_traces > 0);
    let packed = driver
        .run_with_target(&lua, &wrapper, &target, &args, None)
        .unwrap();
    assert!(packed.success);
    assert_eq!(packed.calls, 128);
    assert_eq!(packed.seed_calls, 0);
    assert!(packed.target_live_traces > 0);
    assert!(packed.live_traces >= packed.target_live_traces);
    let result = packed.value.as_table().unwrap();
    assert_eq!(result.raw_get::<i32>("n").unwrap(), 3);
    assert_eq!(result.raw_get::<i32>(1).unwrap(), 43);
    assert_eq!(result.raw_get::<Value>(2).unwrap(), Value::Nil);
    assert_eq!(result.raw_get::<String>(3).unwrap(), "tail");
    // A same-shaped, uncalled function must never borrow another function's
    // evidence, including traces established by preceding successful runs.
    let error = driver
        .run_with_target(&lua, &wrapper, &unrelated, &args, None)
        .err()
        .expect("unrelated function is absent");
    assert!(
        error
            .to_string()
            .contains("absent from completed still-live warm traces")
    );
    // The original API remains self-targeted after explicit-target calls.
    let direct = driver.run(&lua, &unrelated, &args, None).unwrap();
    assert!(direct.success && direct.target_live_traces > 0);
}

// The pinned LuaJIT attach implementation removes its exact callbacks from this
// registry table on detach. These fresh fixtures have no foreign VM observers.
fn assert_success_lane_clean(lua: &Lua) {
    lua.load("assert(not jit.status())").exec().unwrap();
    if let Some(events) = lua
        .named_registry_value::<Option<Table>>("_VMEVENTS")
        .unwrap()
    {
        assert_eq!(events.pairs::<Value, Value>().count(), 0);
    }
}

fn success_fixture(lua: &Lua) -> Table {
    lua.load(
        r#"
local function target(value, alias, hole, tail)
    local total = value
    for i=1,8 do total = total + i end
    return total, alias, hole, tail, alias
end
local function unrelated(value, alias, hole, tail)
    local total = value
    for i=1,8 do total = total + i end
    return total, alias, hole, tail, alias
end
local function pack(...) return { n=select('#',...), ... } end
local function wrapper(original, rows, ...)
    local result = pack(original(...))
    local index = rows.calls + 1
    rows.calls = index
    rows[index] = result
    return result
end
local function failing(rows)
    rows.calls = rows.calls + 1
    if rows.calls == 7 then error('deliberate source failure',0) end
    return rows
end
return {target=target, unrelated=unrelated, wrapper=wrapper, failing=failing}
"#,
    )
    .set_name("@tests/source_program_warm_success_fixture.lua")
    .eval()
    .unwrap()
}

#[test]
fn success_only_keeps_128_actual_packs_aliases_holes_and_exact_target() {
    let lua = Lua::new();
    let fixture = success_fixture(&lua);
    let target: Function = fixture.raw_get("target").unwrap();
    let wrapper: Function = fixture.raw_get("wrapper").unwrap();
    let unrelated: Function = fixture.raw_get("unrelated").unwrap();
    let alias = lua.create_table().unwrap();
    let rows = lua.create_table().unwrap();
    rows.raw_set("calls", 0).unwrap();
    let arguments = [
        Value::Function(target.clone()),
        Value::Table(rows.clone()),
        Value::Integer(7),
        Value::Table(alias.clone()),
        Value::Nil,
        Value::String(lua.create_string("tail").unwrap()),
    ];
    let driver = warm::SourceWarmDriver::new(&lua).unwrap();
    let result = driver
        .run_success_with_target(&lua, &wrapper, &target, &arguments)
        .unwrap();
    assert!(result.success && result.target_live_traces > 0);
    assert_eq!(result.calls, 128);
    assert_eq!(result.seed_calls, 0);
    assert_eq!(rows.raw_get::<usize>("calls").unwrap(), 128);
    assert_eq!(result.value, rows.raw_get::<Value>(128).unwrap());
    for index in 1..=128 {
        let pack: Table = rows.raw_get(index).unwrap();
        assert_eq!(pack.raw_get::<usize>("n").unwrap(), 5);
        assert_eq!(pack.raw_get::<i32>(1).unwrap(), 43);
        assert_eq!(pack.raw_get::<Table>(2).unwrap(), alias);
        assert_eq!(pack.raw_get::<Value>(3).unwrap(), Value::Nil);
        assert_eq!(pack.raw_get::<String>(4).unwrap(), "tail");
        assert_eq!(pack.raw_get::<Table>(5).unwrap(), alias);
        if index > 1 {
            assert_ne!(pack, rows.raw_get::<Table>(index - 1).unwrap());
        }
    }
    assert_success_lane_clean(&lua);
    rows.raw_set("calls", 0).unwrap();
    let error = driver
        .run_success_with_target(&lua, &wrapper, &unrelated, &arguments)
        .err()
        .expect("uncalled equal-shaped target has no evidence");
    assert!(error.to_string().contains("exact source target absent"));
    assert_eq!(rows.raw_get::<usize>("calls").unwrap(), 128);
    assert_success_lane_clean(&lua);
}

#[test]
fn success_only_stops_on_unexpected_error_cleans_up_and_allows_reuse() {
    let lua = Lua::new();
    let fixture = success_fixture(&lua);
    let failing: Function = fixture.raw_get("failing").unwrap();
    let target: Function = fixture.raw_get("target").unwrap();
    let rows = lua.create_table().unwrap();
    rows.raw_set("calls", 0).unwrap();
    let driver = warm::SourceWarmDriver::new(&lua).unwrap();
    let error = driver
        .run_success_with_target(&lua, &failing, &failing, &[Value::Table(rows.clone())])
        .err()
        .expect("unexpected original error fails immediately");
    assert!(error.to_string().contains("after 7 actual calls"));
    assert!(error.to_string().contains("deliberate source failure"));
    assert_eq!(rows.raw_get::<usize>("calls").unwrap(), 7);
    assert_success_lane_clean(&lua);
    let result = driver
        .run_success_with_target(&lua, &target, &target, &[Value::Integer(7)])
        .unwrap();
    assert!(result.success && result.target_live_traces > 0);
    assert_eq!(result.calls, 128);
    assert_eq!(lua.coerce_number(result.value).unwrap(), Some(43.0));
    assert_success_lane_clean(&lua);
}
