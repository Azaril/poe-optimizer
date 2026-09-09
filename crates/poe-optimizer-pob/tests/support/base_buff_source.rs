//! Instrument unchanged original Item methods, exposing their existing snapshot
//! observer to compare successive parses before and after actual assembly.
use super::runtime;
use mlua::{Function, Table, Value};
pub struct Source {
    pub oracle: runtime::Oracle,
    snapshot: Function,
}
impl Source {
    pub fn new() -> Self {
        let oracle = runtime::Oracle::new();
        let class = oracle
            .lua
            .globals()
            .get::<Table>("common")
            .unwrap()
            .get::<Table>("classes")
            .unwrap()
            .get::<Table>("Item")
            .unwrap();
        let build = class.get::<Function>("BuildModList").unwrap();
        let snapshot = (1..=i32::from(build.info().num_upvalues))
            .find_map(|index| {
                // SAFETY: read one existing upvalue of a validated rooted Function.
                // Return exactly name/value; no source bytecode or upvalue mutation.
                let (name, value): (String, Value) = unsafe {
                    oracle.lua.exec_raw(build.clone(), |state| {
                        let name = mlua::ffi::lua_getupvalue(state, 1, index);
                        mlua::ffi::lua_pushstring(state, name);
                        mlua::ffi::lua_insert(state, -2);
                        mlua::ffi::lua_remove(state, 1);
                    })
                }
                .unwrap();
                (name == "snapshot").then(|| value.as_function().unwrap().clone())
            })
            .expect("existing observer snapshot upvalue");
        oracle
            .lua
            .globals()
            .set("buff_snapshot", snapshot.clone())
            .unwrap();
        oracle
            .lua
            .load(include_str!("base_buff_observer.lua"))
            .set_name("@test-only-base-buff-observer")
            .exec()
            .unwrap();
        Self { oracle, snapshot }
    }
    pub fn clear(&self) {
        for key in ["buff_stages", "buff_calls", "buff_attempts"] {
            self.oracle
                .lua
                .globals()
                .set(key, self.oracle.lua.create_table().unwrap())
                .unwrap();
        }
    }
    pub fn try_parse(&self, raw: &str) -> (Table, Option<mlua::Error>) {
        let item = self.oracle.parse("");
        self.clear();
        let error = item
            .get::<Function>("ParseRaw")
            .unwrap()
            .call::<()>((item.clone(), raw))
            .err();
        (item, error)
    }
    pub fn attempts(&self) -> Vec<Table> {
        self.oracle
            .lua
            .globals()
            .get::<Table>("buff_attempts")
            .unwrap()
            .sequence_values::<Table>()
            .map(Result::unwrap)
            .collect()
    }
    pub fn parse(&self, raw: &str) -> Table {
        self.clear();
        self.oracle.parse(raw)
    }
    pub fn reparse(&self, item: &Table, raw: &str) {
        self.clear();
        item.get::<Function>("ParseRaw")
            .unwrap()
            .call::<()>((item.clone(), raw))
            .unwrap();
    }
    pub fn snapshot(&self, item: &Table) -> Table {
        self.snapshot.call(item.clone()).unwrap()
    }
    pub fn stages(&self) -> Vec<Table> {
        self.oracle
            .lua
            .globals()
            .get::<Table>("buff_stages")
            .unwrap()
            .sequence_values::<Table>()
            .map(Result::unwrap)
            .filter(|stage| {
                !stage
                    .get::<Table>("before")
                    .unwrap()
                    .get::<String>("raw")
                    .unwrap()
                    .is_empty()
            })
            .collect()
    }
    pub fn before(&self) -> Table {
        self.stages()
            .first()
            .expect("original assembly stage")
            .get("before")
            .unwrap()
    }
    pub fn calls(&self) -> Vec<(String, bool)> {
        self.oracle
            .lua
            .globals()
            .get::<Table>("buff_calls")
            .unwrap()
            .sequence_values::<Table>()
            .map(|row| {
                let row = row.unwrap();
                (row.get("text").unwrap(), row.get("combined").unwrap())
            })
            .collect()
    }
    pub fn base(&self, name: &str) -> Option<Table> {
        self.oracle
            .lua
            .globals()
            .get::<Table>("data")
            .unwrap()
            .get::<Table>("itemBases")
            .unwrap()
            .get(name)
            .unwrap()
    }
    pub fn copy_base(&self, name: &str, template: &str) {
        let copy = self
            .oracle
            .lua
            .globals()
            .get::<Function>("copyTable")
            .unwrap()
            .call::<Table>(self.base(template).unwrap())
            .unwrap();
        self.oracle
            .lua
            .globals()
            .get::<Table>("data")
            .unwrap()
            .get::<Table>("itemBases")
            .unwrap()
            .set(name, copy)
            .unwrap();
    }
}
