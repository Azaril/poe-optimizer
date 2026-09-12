//! Test-only, bounded inspection of the exact original copyTable activations.
//! Does not replace functions, modify source state or provide traversal evidence
//! to native execution. Hooked calls make no JIT/warm-path claim.
use mlua::debug::DebugEvent;
use mlua::{Function, HookTriggers, Lua, MultiValue, Table, Value, VmState};
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
}
#[derive(Debug)]
pub struct CopyActivation {
    pub ordinal: usize,
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
}
#[derive(Default)]
struct State {
    activations: Vec<CopyActivation>,
    loops: Vec<CopyLoopWitness>,
    active: Vec<usize>,
    error: Option<String>,
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
        })
    }
    pub fn call(
        &self,
        lua: &Lua,
        parser: &Function,
        actual_copy_table: &Function,
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
        let state = Rc::new(RefCell::new(State::default()));
        let target = actual_copy_table.clone();
        let inspect = self.inspect.clone();
        let shared = state.clone();
        lua.set_hook(
            HookTriggers::new().on_calls().on_returns().every_line(),
            move |_, debug| {
                if debug.function().to_pointer() != target.to_pointer() {
                    return Ok(VmState::Continue);
                }
                if let Some(error) = shared.borrow().error.clone() {
                    return Err(failure(error));
                }
                let result = (|| -> mlua::Result<()> {
                    let mut state = shared.borrow_mut();
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
                            if state.activations.len() + state.loops.len() >= MAX_EVENTS {
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
                                state.activations.push(CopyActivation {
                                    ordinal,
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
        Ok(CopyWitness {
            result,
            activations: std::mem::take(&mut state.activations),
            loops: std::mem::take(&mut state.loops),
        })
    }
}
