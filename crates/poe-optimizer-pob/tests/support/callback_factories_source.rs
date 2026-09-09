//! Test-only access to actual original public parser closures and createMod.
use super::public_source::PublicSource;
use mlua::{Function, Lua, MultiValue, Table, Value};

pub fn upvalue(lua: &Lua, function: &Function, requested: &str) -> Value {
    for index in 1..=i32::from(function.info().num_upvalues) {
        let (name, value): (mlua::LuaString, Value) = unsafe {
            // SAFETY: a rooted Function is the only input. Read one valid upvalue,
            // immediately copy its name, and return exactly the name/value pair.
            lua.exec_raw(function.clone(), |state| {
                let name = mlua::ffi::lua_getupvalue(state, 1, index);
                mlua::ffi::lua_pushstring(state, name);
                mlua::ffi::lua_insert(state, -2);
                mlua::ffi::lua_remove(state, 1);
            })
            .unwrap()
        };
        if name.as_bytes().as_ref() == requested.as_bytes() {
            return value;
        }
    }
    panic!("missing actual source upvalue {requested}");
}

pub struct FactorySource {
    pub public: PublicSource,
    pub special: Table,
    pub internal: Function,
    pub create_mod: Function,
}
impl FactorySource {
    pub fn new() -> Self {
        let public = PublicSource::new();
        let internal = upvalue(&public.source.lua, &public.parse, "parseMod")
            .as_function()
            .unwrap()
            .clone();
        assert_eq!(internal.info().line_defined, Some(6619));
        let special = upvalue(&public.source.lua, &internal, "specialModList")
            .as_table()
            .unwrap()
            .clone();
        let create_mod = public
            .source
            .lua
            .globals()
            .get::<Table>("modLib")
            .unwrap()
            .get::<Function>("createMod")
            .unwrap();
        let info = create_mod.info();
        assert_eq!(info.source.as_deref(), Some("@src/Modules/ModTools.lua"));
        assert_eq!(info.line_defined, Some(57));
        assert_eq!(info.last_line_defined, Some(91));
        Self {
            public,
            special,
            internal,
            create_mod,
        }
    }
    pub fn dictionary(&self, name: &str) -> Table {
        upvalue(&self.public.source.lua, &self.internal, name)
            .as_table()
            .unwrap()
            .clone()
    }
    pub fn clear_specials(&self) {
        let keys = self
            .special
            .clone()
            .pairs::<Value, Value>()
            .map(|row| row.unwrap().0)
            .collect::<Vec<_>>();
        for key in keys {
            self.special.set(key, Value::Nil).unwrap();
        }
    }
    pub fn add(&self, pattern: &str, callback: Function) {
        self.special.set(pattern, callback).unwrap();
    }
    pub fn synthetic(&self, body: &str) -> Function {
        // This labelled caller-owned function is a protocol fixture. The mod
        // binding and public dispatcher it calls are unchanged original source.
        let factory = self
            .public
            .source
            .lua
            .load(format!(
                "return function(mod) return function(num,a,b,c,d,e) {body} end end"
            ))
            .set_name("@test-only-callback-factory-fixture")
            .eval::<Function>()
            .unwrap();
        factory.call(self.create_mod.clone()).unwrap()
    }
    pub fn create(&self, values: Vec<Value>) -> Table {
        self.create_mod.call(MultiValue::from_vec(values)).unwrap()
    }
    pub fn text(&self, bytes: impl AsRef<[u8]>) -> Value {
        Value::String(self.public.source.lua.create_string(bytes).unwrap())
    }
}
