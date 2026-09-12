//! Bounded, test-only semantic-state guard for sequential original-parser probes.
//! Snapshots stay in Lua; no per-object Rust handles escape an inspection call.
//! Cache restoration restores raw bindings, never allocation/traversal history.
use mlua::{Function, LightUserData, Lua, Table, Value};
use std::ffi::{c_int, c_void};

unsafe extern "C-unwind" {
    fn lua_upvalueid(state: *mut mlua::ffi::lua_State, function: c_int, slot: c_int)
    -> *mut c_void;
}

#[derive(Clone, Copy)]
pub struct StateWatchLimits {
    pub max_nodes: usize,
    pub max_entries: usize,
    pub max_bytes: usize,
    pub max_depth: usize,
    pub max_cache_entries: usize,
}
impl Default for StateWatchLimits {
    fn default() -> Self {
        Self {
            max_nodes: 250_000,
            max_entries: 2_000_000,
            max_bytes: 256 * 1024 * 1024,
            max_depth: 128,
            max_cache_entries: 65_536,
        }
    }
}

/// Acquire before loading the original source. This is the authenticity boundary
/// for inspection primitives; a C-shaped function found later is not sufficient.
pub struct StateWatchFactory {
    lua: Lua,
    globals: Table,
    primitives: Table,
    factory: Function,
}
impl StateWatchFactory {
    pub fn before_source(lua: &Lua) -> mlua::Result<Self> {
        let primitives = lua.create_table()?;
        for name in ["next", "rawget", "rawset", "type", "rawequal", "error"] {
            let function: Function = lua.globals().raw_get(name)?;
            if function.info().what != "C" {
                return Err(failure("state watch requires original raw primitives"));
            }
            primitives.raw_set(name, function)?;
        }
        primitives.raw_set("metadata", lua.create_function(metadata)?)?;
        primitives.raw_set("lua_function", lua.create_function(lua_function)?)?;
        primitives.raw_set("upvalue", lua.create_function(upvalue)?)?;
        primitives.raw_set(
            "number_bits",
            lua.create_function(|lua, value: f64| {
                lua.create_string(value.to_bits().to_le_bytes())
            })?,
        )?;
        let factory = lua
            .load(include_str!("source_program_state_watch.lua"))
            .set_name("@tests/support/source_program_state_watch.lua")
            .eval()?;
        Ok(Self {
            lua: lua.clone(),
            globals: lua.globals(),
            primitives,
            factory,
        })
    }
    pub fn watch(&self, parser: &Function, cache: &Table) -> mlua::Result<StateWatch> {
        self.watch_with_limits(parser, cache, StateWatchLimits::default())
    }
    pub fn watch_with_limits(
        &self,
        parser: &Function,
        cache: &Table,
        limits: StateWatchLimits,
    ) -> mlua::Result<StateWatch> {
        let maximum = StateWatchLimits::default();
        let fields = [
            ("max_nodes", limits.max_nodes, maximum.max_nodes),
            ("max_entries", limits.max_entries, maximum.max_entries),
            ("max_bytes", limits.max_bytes, maximum.max_bytes),
            ("max_depth", limits.max_depth, maximum.max_depth),
            (
                "max_cache_entries",
                limits.max_cache_entries,
                maximum.max_cache_entries,
            ),
        ];
        let bounds = self.lua.create_table()?;
        for (name, value, cap) in fields {
            if value == 0 || value > cap {
                return Err(failure("state watch invalid limit"));
            }
            bounds.raw_set(name, value)?;
        }
        let (changed, restore_cache) = self.factory.call((
            self.globals.clone(),
            parser.clone(),
            cache.clone(),
            self.primitives.clone(),
            bounds,
        ))?;
        Ok(StateWatch {
            changed,
            restore_cache,
        })
    }
}
pub struct StateWatch {
    changed: Function,
    restore_cache: Function,
}
impl StateWatch {
    /// Some(reason) means the next probe needs a coherent owner/input/catalog
    /// recapture. An error means guard coverage failed, never "unchanged".
    pub fn changed(&self) -> mlua::Result<Option<String>> {
        self.changed.call(())
    }
    pub fn restore_cache_bindings(&self) -> mlua::Result<()> {
        self.restore_cache.call(())
    }
}

fn failure(message: &str) -> mlua::Error {
    mlua::Error::RuntimeError(message.into())
}
fn lua_function(lua: &Lua, function: Function) -> mlua::Result<bool> {
    if function.info().what == "C" {
        return Ok(false);
    }
    if function
        .environment()
        .is_none_or(|env| env.to_pointer() != lua.globals().to_pointer())
    {
        return Err(failure("state watch Lua function has foreign environment"));
    }
    Ok(true)
}
fn metadata(lua: &Lua, value: Value) -> mlua::Result<(Value, usize)> {
    // SAFETY: mlua roots the argument and protects/restores the stack. Both API
    // reads are raw: neither invokes __len, __index nor protected __metatable.
    unsafe {
        lua.exec_raw(value, |state| {
            let length = if mlua::ffi::lua_type(state, 1) == mlua::ffi::LUA_TTABLE {
                mlua::ffi::lua_objlen(state, 1)
            } else {
                0
            };
            if mlua::ffi::lua_getmetatable(state, 1) == 0 {
                mlua::ffi::lua_pushnil(state);
            }
            mlua::ffi::lua_remove(state, 1);
            mlua::ffi::lua_pushinteger(state, length as mlua::ffi::lua_Integer);
        })
    }
}
fn upvalue(
    lua: &Lua,
    (function, slot): (Function, i32),
) -> mlua::Result<(Option<String>, Value, Option<LightUserData>)> {
    if !(1..=129).contains(&slot) || !lua_function(lua, function.clone())? {
        return Err(failure("state watch invalid Lua upvalue slot/function"));
    }
    // SAFETY: identical public-API protocol to capture/upvalues.rs. Prove the
    // slot exists before lua_upvalueid (invalid slots violate its precondition).
    unsafe {
        lua.exec_raw(function, |state| {
            let name = mlua::ffi::lua_getupvalue(state, 1, slot);
            if name.is_null() {
                mlua::ffi::lua_settop(state, 0);
                mlua::ffi::lua_pushnil(state);
                mlua::ffi::lua_pushnil(state);
                mlua::ffi::lua_pushnil(state);
            } else {
                let identity = lua_upvalueid(state, 1, slot);
                mlua::ffi::lua_remove(state, 1);
                mlua::ffi::lua_pushstring(state, name);
                mlua::ffi::lua_insert(state, 1);
                mlua::ffi::lua_pushlightuserdata(state, identity);
            }
        })
    }
}
