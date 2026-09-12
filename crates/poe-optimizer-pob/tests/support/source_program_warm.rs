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
local function warm(f, args, error_seed, target, observe_aborts)
    jit.off()
    jit.flush()
    jit.opt.start('hotloop=2', 'hotexit=2')
    local pending, completed = {}, {}
    local aborts, aborts_complete = observe_aborts and {} or nil, true
    local target_aborts = observe_aborts and {} or nil
    local function record(id, callback)
        if pending[id] then pending[id][callback] = true end
    end
    local function lifecycle(event, id, callback, pc, error_code)
        if event == 'start' then
            pending[id], completed[id] = {}, nil
        elseif event == 'stop' then
            if util.traceinfo(id) then completed[id] = pending[id] end
            pending[id] = nil
        elseif event == 'abort' then
            local had_target = pending[id] and pending[id][target]
            pending[id], completed[id] = nil, nil
            if aborts then
                -- Retain bounded source diagnostics, not an unbounded trace log.
                if #aborts < 128 and type(error_code) == 'number' and
                   error_code >= 0 and error_code <= 255 and error_code % 1 == 0 then
                    aborts[#aborts + 1] = error_code
                    if had_target then target_aborts[#target_aborts + 1] = error_code end
                else
                    aborts_complete = false
                end
            end
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
            if callbacks[target] then target_live = target_live + 1 end
        end
    end
    return {ok=ok, value=value, calls=128, seed_calls=seed_calls,
        live=live, target_live=target_live, aborts=aborts, target_aborts=target_aborts,
        aborts_complete=aborts_complete}
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
    /// Ordered source abort codes, retained only by the optional observation API.
    pub trace_aborts: Vec<u32>,
    /// Aborts whose pending trace had recorded the exact requested target.
    pub target_trace_aborts: Vec<u32>,
    /// False when an abort could not be represented or the 128-event bound was hit.
    pub trace_aborts_complete: bool,
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
        self.run_with_target(lua, function, function, arguments, error_seed)
    }
    /// Invoke `function` while requiring the separately supplied exact `target`
    /// to occur in a completed, still-live trace from this run. This supports
    /// wrappers that preserve full result packs while their nested source target
    /// is traced. It does not imply the wrapper or every target branch compiled.
    /// An error seed still invokes the same wrapper and checks the same target.
    pub fn run_with_target(
        &self,
        lua: &Lua,
        function: &Function,
        target: &Function,
        arguments: &[Value],
        error_seed: Option<&[Value]>,
    ) -> mlua::Result<SourceWarmResult> {
        let result = self.invoke(lua, function, target, arguments, error_seed, false)?;
        if result.target_live_traces == 0 {
            return Err(mlua::Error::RuntimeError(
                "actual source function absent from completed still-live warm traces".into(),
            ));
        }
        Ok(result)
    }
    /// Observe the same 128 calls without asserting that `target` compiled.
    /// A zero target count is evidence of a warm attempt only. Callers must keep
    /// any explicitly diagnosed compiler limit separate from compiled parity.
    #[allow(dead_code)] // Opt-in diagnostics are used by selected test targets only.
    pub fn observe_with_target(
        &self,
        lua: &Lua,
        function: &Function,
        target: &Function,
        arguments: &[Value],
        error_seed: Option<&[Value]>,
    ) -> mlua::Result<SourceWarmResult> {
        self.invoke(lua, function, target, arguments, error_seed, true)
    }
    fn invoke(
        &self,
        lua: &Lua,
        function: &Function,
        target: &Function,
        arguments: &[Value],
        error_seed: Option<&[Value]>,
        observe_aborts: bool,
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
        let output: Table = self.run.call((
            function.clone(),
            args(lua, arguments)?,
            seed,
            target.clone(),
            observe_aborts,
        ))?;
        let trace_aborts = output
            .raw_get::<Option<Table>>("aborts")?
            .map(|values| {
                values
                    .sequence_values::<u32>()
                    .collect::<mlua::Result<Vec<_>>>()
            })
            .transpose()?
            .unwrap_or_default();
        let target_trace_aborts = output
            .raw_get::<Option<Table>>("target_aborts")?
            .map(|values| {
                values
                    .sequence_values::<u32>()
                    .collect::<mlua::Result<Vec<_>>>()
            })
            .transpose()?
            .unwrap_or_default();
        if trace_aborts.len() > 128
            || target_trace_aborts.len() > trace_aborts.len()
            || trace_aborts
                .iter()
                .chain(&target_trace_aborts)
                .any(|code| *code > 255)
        {
            return Err(mlua::Error::RuntimeError(
                "invalid bounded trace abort report".into(),
            ));
        }
        let result = SourceWarmResult {
            success: output.raw_get("ok")?,
            value: output.raw_get("value")?,
            calls: output.raw_get("calls")?,
            seed_calls: output.raw_get("seed_calls")?,
            live_traces: output.raw_get("live")?,
            target_live_traces: output.raw_get("target_live")?,
            trace_aborts,
            target_trace_aborts,
            trace_aborts_complete: output.raw_get("aborts_complete")?,
        };
        if result.live_traces < result.target_live_traces {
            return Err(mlua::Error::RuntimeError(format!(
                "inconsistent source trace report: aborts={:?}, target_aborts={:?}, complete={}",
                result.trace_aborts, result.target_trace_aborts, result.trace_aborts_complete
            )));
        }
        Ok(result)
    }
}
