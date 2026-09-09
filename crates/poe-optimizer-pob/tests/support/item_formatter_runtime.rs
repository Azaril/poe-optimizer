//! Additional observations over the complete authenticated Item/ModParser runtime.
use super::runtime;
use mlua::{Function, Table, Value};
pub struct FormatterOracle {
    pub source: runtime::Oracle,
}
impl FormatterOracle {
    pub fn new() -> Self {
        let source = runtime::Oracle::new();
        source
            .lua
            .load(include_str!("item_formatter_observer.lua"))
            .set_name("@test-only-item-formatter-observer")
            .exec()
            .unwrap();
        Self { source }
    }
    pub fn args(&self, values: &[Value]) -> Table {
        let table = self.source.lua.create_table().unwrap();
        table.set("n", values.len()).unwrap();
        for (index, value) in values.iter().enumerate() {
            table.raw_set(index + 1, value.clone()).unwrap();
        }
        table
    }
    pub fn text(&self, text: &str) -> Value {
        Value::String(self.source.lua.create_string(text).unwrap())
    }
    pub fn observe(&self, operation: &str, args: &[Value]) -> Table {
        self.source
            .lua
            .globals()
            .get::<Function>("formatter_observe")
            .unwrap()
            .call((operation, self.args(args)))
            .unwrap()
    }
    pub fn assignment(&self, formats: &[String]) -> Table {
        let rows = self
            .source
            .lua
            .create_sequence_from(formats.to_vec())
            .unwrap();
        self.source
            .lua
            .globals()
            .get::<Function>("formatter_assignment")
            .unwrap()
            .call(rows)
            .unwrap()
    }
    pub fn warm(&self) -> Vec<String> {
        self.source
            .lua
            .globals()
            .get::<Function>("formatter_warm")
            .unwrap()
            .call::<Table>(())
            .unwrap()
            .pairs::<String, bool>()
            .map(|r| r.unwrap().0)
            .collect()
    }
}
