//! Read-only LuaJIT upvalue-cell identity observation through mlua's protected host.
use super::*;
use std::ffi::{c_int, c_void};

// LuaJIT 2.1 exports this public Lua 5.2-compatible API (pinned luajit-src
// lua.h / lj_api.c). mlua-sys's Lua 5.1 bindings currently omit its declaration.
// No state layout, debug library, bytecode inspection or capture mutation is used.
unsafe extern "C-unwind" {
    fn lua_upvalueid(state: *mut mlua::ffi::lua_State, function: c_int, slot: c_int)
    -> *mut c_void;
}

pub(super) struct ObservedUpvalue {
    pub(super) name: String,
    pub(super) value: Value,
    /// Ephemeral identity only; callers assign bounded artifact IDs, never persist
    /// this address or treat it as authority outside the one observation.
    pub(super) identity: usize,
}

pub(super) fn read(lua: &Lua, function: &Function, slot: i32) -> Result<Option<ObservedUpvalue>> {
    if !(1..=129).contains(&slot) {
        return Err(error("source upvalue observation slot bound"));
    }
    if function
        .environment()
        .is_none_or(|environment| environment.to_pointer() != lua.globals().to_pointer())
    {
        return Err(error(
            "source upvalue observation requires same-host original Lua globals",
        ));
    }
    // SAFETY: mlua locks the same host, roots/pushes the function, protects errors
    // and restores its stack. lua_getupvalue proves the slot exists before the
    // LuaJIT identity API is called; the latter otherwise requires a valid slot.
    // Neither operation invokes source code or mutates a capture or global.
    let (name, value, identity): (Option<String>, Value, Option<mlua::LightUserData>) = unsafe {
        lua.exec_raw(function.clone(), |state| {
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
        })?
    };
    match (name, identity) {
        (None, None) => Ok(None),
        (Some(name), Some(identity)) if !identity.0.is_null() => Ok(Some(ObservedUpvalue {
            name,
            value,
            identity: identity.0 as usize,
        })),
        _ => Err(error("source upvalue identity observation is inconsistent")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_shared_cells_differ_from_equal_values_without_exposing_debug() {
        let lua = Lua::new();
        let closures: Table = lua
            .load(
                r#"
            local shared = {value=3}
            local function first() return shared end
            local function second() return shared end
            local function distinct(value) return function() return value end end
            return {first, second, distinct(shared)}
        "#,
            )
            .eval()
            .unwrap();
        let first: Function = closures.raw_get(1).unwrap();
        let second: Function = closures.raw_get(2).unwrap();
        let third: Function = closures.raw_get(3).unwrap();
        let a = read(&lua, &first, 1).unwrap().unwrap();
        let b = read(&lua, &second, 1).unwrap().unwrap();
        let c = read(&lua, &third, 1).unwrap().unwrap();
        assert_eq!(a.identity, b.identity);
        assert_ne!(a.identity, c.identity);
        assert_eq!(a.name, "shared");
        let pointers = [&a.value, &b.value, &c.value].map(|value| match value {
            Value::Table(table) => table.to_pointer(),
            _ => panic!("captured table"),
        });
        assert_eq!(pointers, [pointers[0]; 3]);
        assert!(read(&lua, &first, 2).unwrap().is_none());
        assert!(read(&lua, &first, 129).unwrap().is_none());
        lua.gc_collect().unwrap();
        let after = read(&lua, &first, 1).unwrap().unwrap();
        assert_eq!(a.identity, after.identity);
        assert!(matches!(
            lua.globals().raw_get::<Value>("debug").unwrap(),
            Value::Nil
        ));
        let loaded: Table = lua.named_registry_value("_LOADED").unwrap();
        assert!(matches!(
            loaded.raw_get::<Value>("debug").unwrap(),
            Value::Nil
        ));
        assert_eq!(
            first
                .call::<Table>(())
                .unwrap()
                .raw_get::<i32>("value")
                .unwrap(),
            3
        );
    }
    #[test]
    fn source_cell_observation_rejects_foreign_builtin_and_out_of_bound_inputs() {
        let lua = Lua::new();
        let foreign = Lua::new();
        let foreign_function: Function = foreign
            .load("return function() return 1 end")
            .eval()
            .unwrap();
        assert!(read(&lua, &foreign_function, 1).is_err());
        let builtin: Function = lua.globals().raw_get("type").unwrap();
        assert!(read(&lua, &builtin, 1).is_err());
        let local: Function = lua.load("return function() return 1 end").eval().unwrap();
        assert!(read(&lua, &local, 0).is_err());
        assert!(read(&lua, &local, 130).is_err());
    }
}
