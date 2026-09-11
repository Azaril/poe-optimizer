//! Test-only warming and exact-function trace evidence. No arithmetic is replaced.
use mlua::{Function, Lua, Table, Value};
const DRIVER: &str = r#"local jit, util = require('jit'), require('jit.util')
local function run(f, args)
    local ok, value
    for iteration = 1, 128 do
        ok, value = pcall(f, unpack(args, 1, args.n))
    end
    return ok, value
end
local function warm(f, args, error_seed)
    jit.off()
    jit.flush()
    jit.opt.start('hotloop=2', 'hotexit=2')
    local pending, completed = {}, {}
    local function record(id, callback)
        if pending[id] then pending[id][callback] = true end
    end
    local function lifecycle(event, id)
        if event == 'start' then
            pending[id], completed[id] = {}, nil
        elseif event == 'stop' then
            if util.traceinfo(id) then completed[id] = pending[id] end
            pending[id] = nil
        elseif event == 'abort' then
            pending[id], completed[id] = nil, nil
        elseif event == 'flush' then
            pending, completed = {}, {}
        end
    end
    jit.off(record, true)
    jit.off(lifecycle, true)
    jit.attach(record, 'record')
    jit.attach(lifecycle, 'trace')
    jit.on()
    local seed_calls = 0
    if error_seed then
        -- Source type errors can exit or abort traces. Establish that the same
        -- original function has a live trace on valid nonliteral inputs first.
        local ok = run(f, error_seed)
        assert(ok)
        seed_calls = 128
    end
    local ok, value = run(f, args)
    jit.off()
    jit.attach(record)
    jit.attach(lifecycle)
    local live, target_live = 0, 0
    for id, callbacks in pairs(completed) do
        if util.traceinfo(id) then
            live = live + 1
            if callbacks[f] then target_live = target_live + 1 end
        end
    end
    return {ok=ok, value=value, calls=128, seed_calls=seed_calls,
        live=live, target_live=target_live}
end
jit.off(warm)
return warm
"#;

pub struct SourceWarmDriver {
    run: Function,
}
pub struct SourceWarmResult {
    pub success: bool,
    pub value: Value,
    pub calls: usize,
    pub seed_calls: usize,
    pub live_traces: usize,
    pub target_live_traces: usize,
}
impl SourceWarmDriver {
    pub fn new(lua: &Lua) -> mlua::Result<Self> {
        Ok(Self {
            run: lua
                .load(DRIVER)
                .set_name("@tests/support/source_program_warm.lua")
                .eval()?,
        })
    }
    /// Invoke the exact supplied function 128 times with the exact argument pack.
    /// Expected errors may use a caller-supplied valid seed to establish a live
    /// trace of that same function first; this does not claim errors ran in JIT.
    pub fn run(
        &self,
        lua: &Lua,
        function: &Function,
        arguments: &[Value],
        error_seed: Option<&[Value]>,
    ) -> mlua::Result<SourceWarmResult> {
        fn args(lua: &Lua, values: &[Value]) -> mlua::Result<Table> {
            let table = lua.create_table()?;
            table.raw_set("n", values.len())?;
            for (index, value) in values.iter().enumerate() {
                table.raw_set(index + 1, value.clone())?;
            }
            Ok(table)
        }
        let seed = error_seed.map(|values| args(lua, values)).transpose()?;
        let output: Table = self
            .run
            .call((function.clone(), args(lua, arguments)?, seed))?;
        let result = SourceWarmResult {
            success: output.raw_get("ok")?,
            value: output.raw_get("value")?,
            calls: output.raw_get("calls")?,
            seed_calls: output.raw_get("seed_calls")?,
            live_traces: output.raw_get("live")?,
            target_live_traces: output.raw_get("target_live")?,
        };
        if result.target_live_traces == 0 || result.live_traces < result.target_live_traces {
            return Err(mlua::Error::RuntimeError(
                "actual source function absent from completed still-live warm traces".into(),
            ));
        }
        Ok(result)
    }
}
