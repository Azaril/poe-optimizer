//! Original library identities needed by the standalone typed-program lowerer.
//! Capture runs in the fresh extraction VM before authenticated source executes;
//! this does not infer primitive authenticity from names in a caller-owned VM.
use super::*;

pub(super) struct ProgramPrimitives {
    globals: Table,
    strings: Table,
    tables: Table,
    string_gsub: Function,
    string_gmatch: Function,
    table_insert: Function,
    ipairs: Function,
    tonumber: Function,
    get_metatable: Function,
    string_metatable: Table,
}

impl ProgramPrimitives {
    pub(super) fn capture(lua: &Lua) -> Result<Self> {
        let globals = lua.globals();
        plain(&globals, "global environment")?;
        let strings = raw_table(&globals, "string")?;
        let tables = raw_table(&globals, "table")?;
        plain(&strings, "string library")?;
        plain(&tables, "table library")?;
        // Retain the original observer; never call a later global replacement.
        let get_metatable = raw_c_function(&globals, "getmetatable")?;
        let string_metatable: Table = get_metatable.call("")?;
        let this = Self {
            string_gsub: raw_c_function(&strings, "gsub")?,
            string_gmatch: raw_c_function(&strings, "gmatch")?,
            table_insert: raw_c_function(&tables, "insert")?,
            ipairs: raw_c_function(&globals, "ipairs")?,
            tonumber: raw_c_function(&globals, "tonumber")?,
            globals,
            strings,
            tables,
            get_metatable,
            string_metatable,
        };
        this.verify(lua)?;
        Ok(this)
    }

    pub(super) fn verify(&self, lua: &Lua) -> Result<()> {
        let globals = lua.globals();
        same_table(&globals, &self.globals, "global environment")?;
        let strings = raw_table(&globals, "string")?;
        let tables = raw_table(&globals, "table")?;
        same_table(&strings, &self.strings, "string library")?;
        same_table(&tables, &self.tables, "table library")?;
        for (table, name, expected) in [
            (&strings, "gsub", &self.string_gsub),
            (&strings, "gmatch", &self.string_gmatch),
            (&tables, "insert", &self.table_insert),
            (&globals, "ipairs", &self.ipairs),
            (&globals, "tonumber", &self.tonumber),
            (&globals, "getmetatable", &self.get_metatable),
        ] {
            let actual = raw_c_function(table, name)?;
            if actual.to_pointer() != expected.to_pointer() {
                return Err(error(format!(
                    "program original primitive {name} was rebound"
                )));
            }
        }
        let metatable: Table = self.get_metatable.call("")?;
        same_table(&metatable, &self.string_metatable, "string metatable")?;
        let index = raw_table(&metatable, "__index")?;
        same_table(&index, &self.strings, "string method index")?;
        // The original LuaJIT string metatable has only its library __index.
        // Additional metamethods could change generic operations outside method
        // lookup, so do not authenticate a silently enlarged effect surface.
        for entry in metatable.pairs::<Value, Value>() {
            let (key, _) = entry?;
            if !matches!(key, Value::String(ref key) if key.as_bytes().as_ref() == b"__index") {
                return Err(error("program string metatable has an unmodeled field"));
            }
        }
        Ok(())
    }
}

fn plain(table: &Table, role: &str) -> Result<()> {
    if table.metatable().is_some() {
        return Err(error(format!("program {role} cannot use metatable lookup")));
    }
    Ok(())
}
fn same_table(actual: &Table, expected: &Table, role: &str) -> Result<()> {
    if actual.to_pointer() != expected.to_pointer() {
        return Err(error(format!("program original {role} was rebound")));
    }
    plain(actual, role)
}
fn raw_table(table: &Table, key: &str) -> Result<Table> {
    match table.raw_get::<Value>(key)? {
        Value::Table(value) => Ok(value),
        _ => Err(error(format!("program raw table {key} is missing"))),
    }
}
fn raw_c_function(table: &Table, key: &str) -> Result<Function> {
    match table.raw_get::<Value>(key)? {
        Value::Function(function) if function.info().what == "C" => Ok(function),
        _ => Err(error(format!(
            "program raw primitive {key} is not an original C function"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_identity_survives_unrelated_source_definitions_and_repeated_verification() {
        let lua = Lua::new();
        let original = ProgramPrimitives::capture(&lua).unwrap();
        lua.load(
            "local words = {}; for word in ('a b'):gmatch('%w+') do table.insert(words, word) end; \
             program_auth_observed = tonumber('12'); string.program_auth_extra = 'unrelated'; \
             table.program_auth_extra = {}; program_auth_function = function(name) return name:gsub(' skill', '') end",
        )
        .exec()
        .unwrap();
        original.verify(&lua).unwrap();
        original.verify(&lua).unwrap();
        assert_eq!(
            lua.globals().get::<f64>("program_auth_observed").unwrap(),
            12.0
        );
        assert!(original.verify(&Lua::new()).is_err());
    }

    #[test]
    fn replaced_missing_lua_and_different_c_primitives_are_rejected() {
        for name in [
            "string.gsub",
            "string.gmatch",
            "table.insert",
            "ipairs",
            "tonumber",
            "getmetatable",
        ] {
            for replacement in [
                "nil",
                "false",
                "{}",
                "function(...) return ... end",
                "tostring",
            ] {
                let lua = Lua::new();
                let original = ProgramPrimitives::capture(&lua).unwrap();
                lua.load(format!("{name} = {replacement}")).exec().unwrap();
                assert!(original.verify(&lua).is_err(), "{name} = {replacement}");
            }
        }
    }

    #[test]
    fn replacement_library_tables_and_metatable_fallback_never_supply_raw_bindings() {
        for code in [
            "local original = string; string = {}; for k,v in pairs(original) do string[k]=v end",
            "local original = table; table = {}; for k,v in pairs(original) do table[k]=v end",
            "local original = string; string = nil; setmetatable(_G, {__index={string=original}})",
            "local original = table; table = nil; setmetatable(_G, {__index={table=original}})",
            "local original = ipairs; ipairs = nil; setmetatable(_G, {__index={ipairs=original}})",
            "local original = tonumber; tonumber = nil; setmetatable(_G, {__index={tonumber=original}})",
            "local original = string.gsub; string.gsub = nil; setmetatable(string, {__index={gsub=original}})",
            "local original = string.gmatch; string.gmatch = nil; setmetatable(string, {__index={gmatch=original}})",
            "local original = table.insert; table.insert = nil; setmetatable(table, {__index={insert=original}})",
            "setmetatable(_G, {})",
            "setmetatable(string, {})",
            "setmetatable(table, {})",
        ] {
            let lua = Lua::new();
            let original = ProgramPrimitives::capture(&lua).unwrap();
            lua.load(code).exec().unwrap();
            assert!(original.verify(&lua).is_err(), "{code}");
        }
    }

    #[test]
    fn string_method_lookup_requires_the_same_plain_metatable_and_raw_index() {
        for code in [
            "getmetatable('').__index = nil",
            "getmetatable('').__index = function(_, key) return string[key] end",
            "local t={}; for k,v in pairs(string) do t[k]=v end; getmetatable('').__index = t",
            "getmetatable('').__index = setmetatable({}, {__index=string})",
            "local m=getmetatable(''); m.__index=nil; setmetatable(m,{__index={__index=string}})",
            "setmetatable(getmetatable(''), {})",
            "getmetatable('').__call = function() end",
            "getmetatable('').__metatable = 'protected'",
            "debug.setmetatable('', {__index=string})",
            "debug.setmetatable('', nil)",
        ] {
            // SAFETY: this test-owned VM uses debug only to replace a string's
            // actual metatable; no untrusted source or runtime program uses it.
            let lua = unsafe { Lua::unsafe_new() };
            let original = ProgramPrimitives::capture(&lua).unwrap();
            lua.load(code).exec().unwrap();
            assert!(original.verify(&lua).is_err(), "{code}");
        }
    }

    #[test]
    fn capture_rejects_non_c_or_fallback_initial_shapes() {
        for code in [
            "string.gmatch = function() end",
            "table.insert = function() end",
            "ipairs = function() end",
            "tonumber = function() end",
            "getmetatable = function() return {__index=string} end",
            "local original=string.gsub; string.gsub=nil; setmetatable(string,{__index={gsub=original}})",
            "getmetatable('').__index = {}",
        ] {
            let lua = Lua::new();
            lua.load(code).exec().unwrap();
            assert!(ProgramPrimitives::capture(&lua).is_err(), "{code}");
        }
    }
}
