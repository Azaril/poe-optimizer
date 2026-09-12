//! Test-only, bounded inspection of the exact original copyTable activations.
//! Does not replace functions, modify source state or provide traversal evidence
//! to native execution. Hooked calls make no JIT/warm-path claim.
#[allow(dead_code)]
#[path = "source_program_producer_witness.rs"]
pub mod producer;
use mlua::debug::DebugEvent;
use mlua::{Function, HookTriggers, Lua, MultiValue, Table, Value, VmState};
use producer::{ProducerBinding, ProducerProbe, ProducerState};
use std::{cell::RefCell, rc::Rc};
const MAX_EVENTS: usize = 256;
const MAX_DEPTH: usize = 32;
const INSPECT: &str = r#"local getinfo,getlocal = ...
return function(target)
    local frames = {}
    for level = 1, 256 do
        local info = getinfo(level, 'f')
        if not info then return frames end
        if info.func == target then
            if #frames == 32 then error('copy witness depth bound') end
            local name,tbl = getlocal(level,1)
            local second,noRecurse = getlocal(level,2)
            if name ~= 'tbl' or second ~= 'noRecurse' then error('copy witness parameter layout') end
            local row = {tbl=tbl,noRecurse=noRecurse}
            for slot=3,256 do
                local key,value = getlocal(level,slot)
                if not key then break end
                if slot == 256 then error('copy witness local bound') end
                if key == 'k' then row.key=value; row.hasKey=true end
                if key == '(for control)' then row.control=value; row.hasControl=true end
            end
            frames[#frames+1] = row
        end
    end
    error('copy witness stack scan bound')
end"#;
fn failure(message: impl Into<String>) -> mlua::Error {
    mlua::Error::RuntimeError(message.into())
}
pub struct SourceCopyWitness {
    globals: Table,
    gethook: Function,
    inspect: Function,
    producer: ProducerProbe,
}
#[derive(Debug)]
pub struct CopyActivation {
    pub ordinal: usize,
    pub observed_event: usize,
    pub parent: Option<usize>,
    pub depth: usize,
    pub table: Value,
    pub no_recurse: Value,
    pub parent_key: Option<Value>,
}
#[derive(Debug)]
pub struct CopyLoopWitness {
    pub activation: usize,
    pub source_line: usize,
    pub table: Value,
    /// Only the actual hidden local named `(for control)`, when available.
    /// This is a line-hook observation, not an intercepted Next argument.
    pub visible_control: Option<Value>,
    pub control_unavailable_reason: Option<String>,
    pub visible_key: Option<Value>,
}
#[derive(Debug)]
pub struct CopyWitness {
    /// A source execution error is distinct from a witness/collection failure.
    pub result: mlua::Result<MultiValue>,
    pub activations: Vec<CopyActivation>,
    pub loops: Vec<CopyLoopWitness>,
    pub producer: ProducerState,
    pub producer_binding: Option<ProducerBinding>,
}
#[derive(Default)]
struct State {
    activations: Vec<CopyActivation>,
    loops: Vec<CopyLoopWitness>,
    active: Vec<usize>,
    error: Option<String>,
    producer: ProducerState,
}
struct HookGuard<'a>(&'a Lua);
impl Drop for HookGuard<'_> {
    fn drop(&mut self) {
        self.0.remove_hook();
    }
}
impl SourceCopyWitness {
    pub fn before_source(lua: &Lua) -> mlua::Result<Self> {
        let globals = lua.globals();
        let debug: Table = globals.raw_get("debug")?;
        if debug.metatable().is_some() {
            return Err(failure("copy witness debug library metatable"));
        }
        let mut original = Vec::new();
        for key in ["getinfo", "getlocal", "gethook"] {
            let function: Function = debug.raw_get(key)?;
            if function.info().what != "C" {
                return Err(failure(
                    "copy witness inspection primitive is not original C",
                ));
            }
            original.push(function);
        }
        let inspect = lua
            .load(INSPECT)
            .set_name("@tests/support/source_program_copy_witness.rs#inspection")
            .call((original[0].clone(), original[1].clone()))?;
        Ok(Self {
            globals,
            gethook: original[2].clone(),
            inspect,
            producer: ProducerProbe::before_source(lua)?,
        })
    }
    pub fn bind_producer(
        &self,
        parser: &Function,
        target: &Function,
        source_line: usize,
    ) -> mlua::Result<ProducerBinding> {
        self.producer.bind(parser, target, source_line)
    }
    pub fn bind_producer_tail(
        &self,
        binding: &ProducerBinding,
        config: producer::ProducerTailConfig,
    ) -> mlua::Result<ProducerBinding> {
        self.producer.bind_tail(binding, config)
    }
    #[allow(dead_code)]
    pub fn call(
        &self,
        lua: &Lua,
        parser: &Function,
        actual_copy_table: &Function,
        args: MultiValue,
    ) -> mlua::Result<CopyWitness> {
        self.call_inner(lua, parser, actual_copy_table, None, args)
    }
    pub fn call_with_producer(
        &self,
        lua: &Lua,
        parser: &Function,
        actual_copy_table: &Function,
        producer_binding: &ProducerBinding,
        args: MultiValue,
    ) -> mlua::Result<CopyWitness> {
        self.call_inner(lua, parser, actual_copy_table, Some(producer_binding), args)
    }
    fn call_inner(
        &self,
        lua: &Lua,
        parser: &Function,
        actual_copy_table: &Function,
        producer_binding: Option<&ProducerBinding>,
        args: MultiValue,
    ) -> mlua::Result<CopyWitness> {
        if lua.globals().to_pointer() != self.globals.to_pointer() {
            return Err(failure("copy witness belongs to a different Lua host"));
        }
        let previous: MultiValue = self.gethook.call(())?;
        if previous.front().is_some_and(|v| !matches!(v, Value::Nil)) {
            return Err(failure("copy witness requires no existing hook"));
        }
        if actual_copy_table.info().what != "Lua" {
            return Err(failure("copy witness target must be actual Lua function"));
        }
        let first_line = actual_copy_table
            .info()
            .line_defined
            .ok_or_else(|| failure("copy witness target has no source lines"))?;
        if let Some(binding) = producer_binding {
            if !binding.is_parser(parser) {
                return Err(failure(
                    "producer witness belongs to a different parser wrapper",
                ));
            }
            self.producer.verify(binding)?;
        }
        let state = Rc::new(RefCell::new(State::default()));
        let target = actual_copy_table.clone();
        let inspect = self.inspect.clone();
        let producer = self.producer.clone();
        let producer_binding_for_hook = producer_binding.cloned();
        let shared = state.clone();
        lua.set_hook(
            HookTriggers::new().on_calls().on_returns().every_line(),
            move |_, debug| {
                let is_copy = debug.function().to_pointer() == target.to_pointer();
                let is_producer = producer_binding_for_hook.as_ref().is_some_and(|binding| debug.function().to_pointer() == binding.target.to_pointer());
                let is_tail = debug.event() == DebugEvent::Call && producer_binding_for_hook.as_ref()
                    .and_then(|binding| binding.tail.as_ref())
                    .is_some_and(|tail| debug.function().to_pointer() == tail.original_unpack.to_pointer());
                if !is_copy && !is_producer && !is_tail {
                    return Ok(VmState::Continue);
                }
                if let Some(error) = shared.borrow().error.clone() {
                    return Err(failure(error));
                }
                let result = (|| -> mlua::Result<()> {
                    let mut state = shared.borrow_mut();
                    if is_tail {
                        let count = state.activations.len() + state.loops.len() + state.producer.event_count();
                        return producer.observe_tail(&mut state.producer, producer_binding_for_hook.as_ref().expect("matched tail target"),
                            MAX_EVENTS.saturating_sub(count), count);
                    }
                    if is_producer {
                        let count = state.activations.len() + state.loops.len() + state.producer.event_count();
                        return producer.observe(&mut state.producer, producer_binding_for_hook.as_ref().expect("matched producer target"),
                            debug.event(), debug.current_line(), MAX_EVENTS.saturating_sub(count), count);
                    }
                    match debug.event() {
                        DebugEvent::Call | DebugEvent::Line => {
                            let line = debug.current_line();
                            // Original copyTable's for/body-entry lines. Absence of
                            // hidden control locals remains explicitly unavailable.
                            if debug.event() == DebugEvent::Line
                                && !matches!(line,Some(n) if n==first_line+2 || n==first_line+3)
                            {
                                return Ok(());
                            }
                            if state.activations.len() + state.loops.len() + state.producer.event_count() >= MAX_EVENTS {
                                return Err(failure("copy witness event bound"));
                            }
                            let frames: Table = inspect.call(target.clone())?;
                            let depth = frames.raw_len();
                            if depth == 0 || depth > MAX_DEPTH {
                                return Err(failure(
                                    "copy witness exact active frame missing/depth bound",
                                ));
                            }
                            let frame: Table = frames.raw_get(1)?;
                            let table: Value = frame.raw_get("tbl")?;
                            let no_recurse: Value = frame.raw_get("noRecurse")?;
                            let ancestors = if debug.event() == DebugEvent::Call {
                                depth - 1
                            } else {
                                depth
                            };
                            if ancestors > state.active.len() {
                                return Err(failure("copy witness missed parent activation"));
                            }
                            state.active.truncate(ancestors);
                            for ancestor in 0..ancestors {
                                let old = &state.activations[state.active[ancestor]];
                                let observed: Table = frames.raw_get(depth - ancestor)?;
                                if observed.raw_get::<Value>("tbl")? != old.table
                                    || observed.raw_get::<Value>("noRecurse")? != old.no_recurse
                                {
                                    return Err(failure(
                                        "copy witness active frame identity mismatch",
                                    ));
                                }
                            }
                            if debug.event() == DebugEvent::Call {
                                let parent = state.active.last().copied();
                                let parent_key = if depth > 1 {
                                    let parent: Table = frames.raw_get(2)?;
                                    if parent.raw_get::<bool>("hasKey")? {
                                        Some(parent.raw_get("key")?)
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                };
                                let ordinal = state.activations.len();
                                let observed_event = state.activations.len() + state.loops.len() + state.producer.event_count();
                                state.activations.push(CopyActivation {
                                    ordinal,
                                    observed_event,
                                    parent,
                                    depth,
                                    table,
                                    no_recurse,
                                    parent_key,
                                });
                                state.active.push(ordinal);
                            } else {
                                let activation = *state.active.last().ok_or_else(|| {
                                    failure("copy witness missing current activation")
                                })?;
                                let visible_key = if frame.raw_get::<bool>("hasKey")? {
                                    Some(frame.raw_get("key")?)
                                } else {
                                    None
                                };
                                let (visible_control, control_unavailable_reason) =
                                    if frame.raw_get::<bool>("hasControl")? {
                                        let value: Value = frame.raw_get("control")?;
                                        if matches!(value, Value::LightUserData(_)) {
                                            (None, Some("optimized iterator internal control is not a source Next argument".into()))
                                        } else {
                                            (Some(value), None)
                                        }
                                    } else {
                                        (None, Some("hidden control local is not visible at this source line".into()))
                                    };
                                state.loops.push(CopyLoopWitness {
                                    activation,
                                    source_line: line.unwrap(),
                                    table,
                                    visible_control,
                                    control_unavailable_reason,
                                    visible_key,
                                });
                            }
                        }
                        DebugEvent::Ret => {
                            state
                                .active
                                .pop()
                                .ok_or_else(|| failure("copy witness unmatched return"))?;
                        }
                        DebugEvent::TailCall => {
                            return Err(failure("copy witness unexpected tail-return event"));
                        }
                        _ => {}
                    }
                    Ok(())
                })();
                if let Err(error) = result {
                    let message = error.to_string();
                    shared.borrow_mut().error = Some(message.clone());
                    return Err(failure(message));
                }
                Ok(VmState::Continue)
            },
        )?;
        let guard = HookGuard(lua);
        let installed: MultiValue = self.gethook.call(())?;
        let result = parser.call(args);
        let remaining: MultiValue = self.gethook.call(())?;
        drop(guard);
        if !installed.iter().eq(remaining.iter()) {
            return Err(failure("copy witness hook was changed during source call"));
        }
        let mut state = state.borrow_mut();
        if let Some(error) = state.error.take() {
            return Err(failure(error));
        }
        if let Some(binding) = producer_binding {
            self.producer.verify(binding)?;
        }
        Ok(CopyWitness {
            result,
            activations: std::mem::take(&mut state.activations),
            loops: std::mem::take(&mut state.loops),
            producer: std::mem::take(&mut state.producer),
            producer_binding: producer_binding.cloned(),
        })
    }
}

#[cfg(test)]
mod producer_tests {
    use super::*;
    const COPY: &str = r#"function copyTable(tbl, noRecurse)
    local out = {}
    for k, v in pairs(tbl) do
        if not noRecurse and type(v) == "table" then
            out[k] = copyTable(v)
        else
            out[k] = v
        end
    end
    return out
end"#;
    const PARSER: &str = r#"local function parseMod(replace)
    local modList = {}
    for i,name in ipairs({'same','same'}) do
        modList[i] = { name=name }
        if replace then modList[i].extra=1 end
    end
    if replace then modList[1] = { child=modList[1] } end
    return modList
end
return function(replace)
    local unused = parseMod(false)
    return copyTable(parseMod(replace))
end, parseMod"#;

    #[test]
    fn actual_producer_events_keep_equal_distinct_repeated_and_replaced_rows() {
        let lua = unsafe { Lua::unsafe_new() };
        let witness = SourceCopyWitness::before_source(&lua).unwrap();
        lua.load(COPY).exec().unwrap();
        let copy: Function = lua.globals().raw_get("copyTable").unwrap();
        let (parser, inner): (Function, Function) = lua.load(PARSER).eval().unwrap();
        let binding = witness.bind_producer(&parser, &inner, 5).unwrap();
        let observed = witness
            .call_with_producer(
                &lua,
                &parser,
                &copy,
                &binding,
                MultiValue::from_vec(vec![Value::Boolean(true)]),
            )
            .unwrap();
        assert!(observed.result.is_ok());
        assert!(
            observed
                .producer_binding
                .as_ref()
                .unwrap()
                .is_parser(&parser)
        );
        assert_eq!(observed.producer.activations.len(), 2);
        assert_eq!(observed.producer.stores.len(), 4);
        let stores = &observed.producer.stores;
        assert_eq!(stores[0].name, stores[1].name);
        assert_ne!(
            stores[0].table, stores[1].table,
            "equal rows are distinct source objects"
        );
        for (index, store) in stores.iter().enumerate() {
            assert_eq!(store.activation, index / 2);
            assert_eq!(store.index, Value::Integer((index % 2 + 1) as i64));
            let copies = observed
                .activations
                .iter()
                .filter(|a| a.table == Value::Table(store.table.clone()))
                .collect::<Vec<_>>();
            if index < 2 {
                assert!(copies.is_empty(), "first parse attempt is discarded");
            } else {
                assert_eq!(copies.len(), 1);
                assert!(store.observed_event < copies[0].observed_event);
            }
        }
        assert_ne!(
            stores[2].list.raw_get::<Table>(1).unwrap(),
            stores[2].table,
            "a later wrapper replacement cannot erase the observed old identity"
        );
        assert_eq!(stores[3].list.raw_get::<Table>(2).unwrap(), stores[3].table);
        assert!(
            witness
                .gethook
                .call::<MultiValue>(())
                .unwrap()
                .front()
                .is_some_and(|v| matches!(v, Value::Nil))
        );
    }

    #[test]
    fn producer_binding_rejects_an_equal_distinct_function_and_hook_cleans_up() {
        let lua = unsafe { Lua::unsafe_new() };
        let witness = SourceCopyWitness::before_source(&lua).unwrap();
        lua.load(COPY).exec().unwrap();
        let copy: Function = lua.globals().raw_get("copyTable").unwrap();
        let (parser, inner): (Function, Function) = lua.load(PARSER).eval().unwrap();
        let (_, other): (Function, Function) = lua.load(PARSER).eval().unwrap();
        assert!(witness.bind_producer(&parser, &other, 5).is_err());
        let distinct_wrapper: Function = lua
            .load("local parseMod=...; return function(v) return parseMod(v) end")
            .call(inner.clone())
            .unwrap();
        let exact = witness.bind_producer(&parser, &inner, 5).unwrap();
        assert!(
            witness
                .call_with_producer(&lua, &distinct_wrapper, &copy, &exact, MultiValue::new())
                .is_err(),
            "an equal inner function cannot bind a different outer wrapper"
        );
        let binding = witness.bind_producer(&parser, &inner, 4).unwrap();
        // Before the store, local modList[i] is absent. This collection failure
        // must remain visible and still remove the shared hook.
        assert!(
            witness
                .call_with_producer(
                    &lua,
                    &parser,
                    &copy,
                    &binding,
                    MultiValue::from_vec(vec![Value::Boolean(false)])
                )
                .is_err()
        );
        assert!(
            witness
                .gethook
                .call::<MultiValue>(())
                .unwrap()
                .front()
                .is_some_and(|v| matches!(v, Value::Nil))
        );
        let good = witness.bind_producer(&parser, &inner, 5).unwrap();
        assert!(
            witness
                .call_with_producer(
                    &lua,
                    &parser,
                    &copy,
                    &good,
                    MultiValue::from_vec(vec![Value::Boolean(false)])
                )
                .unwrap()
                .result
                .is_ok()
        );
    }
}
