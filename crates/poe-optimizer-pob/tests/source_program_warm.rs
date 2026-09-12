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
