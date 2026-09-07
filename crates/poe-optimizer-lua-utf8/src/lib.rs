//! Statically linked luautf8 0.1.6 for the Path of Building LuaJIT host.
//!
//! This module uses the same vendored LuaJIT as `mlua`. It does not load a Lua
//! runtime or a native UTF-8 module from the filesystem.

use mlua::{Lua, Result};
use std::ffi::c_int;

unsafe extern "C-unwind" {
    fn luaopen_utf8(state: *mut mlua::ffi::lua_State) -> c_int;
}

/// Registers the native loader for `require("lua-utf8")`.
///
/// The state's package library must already be loaded. Registration is lazy:
/// normal Lua `require` initializes and caches the module on first use.
pub fn register(lua: &Lua) -> Result<()> {
    // SAFETY: luaopen_utf8 is a statically linked Lua C module, compiled against
    // the exact public LuaJIT headers pinned for mlua's vendored runtime. mlua
    // invokes it through Lua's protected call boundary with a valid Lua state.
    let loader = unsafe { lua.create_c_function(luaopen_utf8) }?;
    lua.preload_module("lua-utf8", loader)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unicode_operations_load_without_dynamic_modules() -> Result<()> {
        let lua = Lua::new();
        register(&lua)?;
        lua.load(
            r##"
            package.cpath = ""
            local utf8 = require("lua-utf8")
            assert(utf8 == require("lua-utf8"))
            assert(utf8.len("é中a") == 3)
            assert(utf8.reverse("é中a") == "a中é")
            assert(utf8.sub("é中a", 2, 3) == "中a")
            local first, last = utf8.find("aé中", "中")
            assert(first == 3 and last == 3)
            local replaced, count = utf8.gsub("12élan34", "%d+", "#")
            assert(replaced == "#élan#" and count == 2)
            assert(utf8.upper("élan") == "ÉLAN")
            assert(not utf8.isvalid(string.char(255)))
            "##,
        )
        .exec()
    }

    #[test]
    fn native_errors_are_protected_and_state_remains_usable() -> Result<()> {
        let lua = Lua::new();
        register(&lua)?;
        let error = lua
            .load(r#"return require("lua-utf8").find("text", "[")"#)
            .eval::<mlua::Value>()
            .expect_err("malformed pattern must fail through Lua's protected call");
        assert!(error.to_string().contains("malformed pattern"));
        assert_eq!(
            lua.load(r#"return require("lua-utf8").len("résumé")"#)
                .eval::<u32>()?,
            6
        );
        Ok(())
    }
}
