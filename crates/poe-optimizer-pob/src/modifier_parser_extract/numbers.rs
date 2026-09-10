//! Authentication of the original global one-argument number primitive.
use super::*;

pub(super) struct NumberPrimitive {
    globals: Table,
    function: Function,
}
impl NumberPrimitive {
    pub(super) fn capture(lua: &Lua) -> Result<Self> {
        let globals = lua.globals();
        let function: Function = globals.raw_get("tonumber")?;
        let this = Self { globals, function };
        this.verify(lua)?;
        Ok(this)
    }
    pub(super) fn verify(&self, lua: &Lua) -> Result<()> {
        let globals = lua.globals();
        let actual: Function = globals.raw_get("tonumber")?;
        // C functions have no environment through mlua's Function API. Prove
        // their original raw global binding here; Graph proves Lua owner environments.
        if globals.to_pointer() != self.globals.to_pointer()
            || actual.to_pointer() != self.function.to_pointer()
            || actual.info().what != "C"
        {
            return Err(error(
                "factory tonumber is not the original global C primitive",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_number_primitive_rejects_rebound_functions_and_metatable_fallback() {
        for code in [
            "tonumber = function(value) return value end",
            "tonumber = tostring",
            "tonumber = nil",
            "local original = tonumber; tonumber = nil; setmetatable(_G, {__index = {tonumber=original}})",
        ] {
            let lua = Lua::new();
            let original = NumberPrimitive::capture(&lua).unwrap();
            lua.load(code).exec().unwrap();
            assert!(original.verify(&lua).is_err(), "{code}");
        }
        let lua = Lua::new();
        lua.load("tonumber=function() end").exec().unwrap();
        assert!(NumberPrimitive::capture(&lua).is_err());
    }
    #[test]
    fn global_number_owners_require_the_actual_original_lua_environment() {
        // SAFETY: the bounded, test-owned VM retains debug only for observing upvalues.
        let lua = unsafe { Lua::unsafe_new() };
        let primitive = NumberPrimitive::capture(&lua).unwrap();
        let source = "return function(value) return tonumber(value) end\n";
        let sources = BTreeMap::from([("owner.lua".into(), source.into())]);
        let get_upvalue: Function = lua
            .globals()
            .get::<Table>("debug")
            .unwrap()
            .get("getupvalue")
            .unwrap();
        for changed in [false, true] {
            let owner: Function = lua.load(source).set_name("@owner.lua").eval().unwrap();
            if changed {
                let env = lua.create_table().unwrap();
                env.set("tonumber", primitive.function.clone()).unwrap();
                owner.set_environment(env).unwrap();
            }
            let mut graph = Graph {
                global_environment: lua.globals().to_pointer() as usize,
                sources: &sources,
                get_upvalue: get_upvalue.clone(),
                builtins: BTreeMap::new(),
                seen_tables: BTreeMap::new(),
                seen_callbacks: BTreeMap::new(),
                tables: vec![],
                callbacks: vec![],
                values: 0,
            };
            assert_eq!(graph.value(Value::Function(owner), 0).is_err(), changed);
        }
    }
}
