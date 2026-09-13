//! Retained complete original explosion functions shared by reference and native comparisons.
use super::{
    factory_source::{FactorySource, upvalue},
    observer,
};
use mlua::{Function, MultiValue, Value};
use serde_json::json;

pub const LINE: &[u8] = b"Warcries Explode Corpses dealing 10% of their Life as Physical Damage";
pub const PATTERN: &str = "^warcries explode corpses dealing (%d+)%% of their life as (.+) damage$";
pub struct Original {
    pub source: FactorySource,
    pub callback: Function,
    pub helper: Function,
    pub upper: Function,
    pub flag: Function,
}
fn declaration(f: &Function, first: usize, last: usize) {
    let info = f.info();
    assert_eq!(info.source.as_deref(), Some("@src/Modules/ModParser.lua"));
    assert_eq!(info.line_defined, Some(first));
    assert_eq!(info.last_line_defined, Some(last));
}
impl Original {
    pub fn new() -> Self {
        let source = FactorySource::new();
        let callback: Function = source.special.raw_get(PATTERN).unwrap();
        let helper = upvalue(&source.public.source.lua, &callback, "explodeFunc")
            .as_function()
            .unwrap()
            .clone();
        let upper = upvalue(&source.public.source.lua, &helper, "firstToUpper")
            .as_function()
            .unwrap()
            .clone();
        let flag = upvalue(&source.public.source.lua, &helper, "flag")
            .as_function()
            .unwrap()
            .clone();
        declaration(&callback, 2336, 2338);
        declaration(&helper, 2255, 2266);
        declaration(&upper, 13, 15);
        declaration(&flag, 2177, 2179);
        assert_eq!(
            upvalue(&source.public.source.lua, &helper, "mod").as_function(),
            Some(&source.create_mod)
        );
        assert_eq!(
            upvalue(&source.public.source.lua, &flag, "mod").as_function(),
            Some(&source.create_mod)
        );
        Self {
            source,
            callback,
            helper,
            upper,
            flag,
        }
    }
    pub fn verify(&self) {
        let lua = &self.source.public.source.lua;
        assert_eq!(
            self.source.special.raw_get::<Function>(PATTERN).unwrap(),
            self.callback
        );
        for (f, n, wanted) in [
            (&self.callback, "explodeFunc", &self.helper),
            (&self.helper, "firstToUpper", &self.upper),
            (&self.helper, "flag", &self.flag),
            (&self.helper, "mod", &self.source.create_mod),
            (&self.flag, "mod", &self.source.create_mod),
        ] {
            assert_eq!(upvalue(lua, f, n).as_function(), Some(wanted));
        }
        declaration(&self.callback, 2336, 2338);
        declaration(&self.helper, 2255, 2266);
    }
    pub fn text(&self, v: impl AsRef<[u8]>) -> Value {
        self.source.text(v)
    }
    pub fn run(
        &self,
        entry: &Function,
        args: Vec<Value>,
    ) -> (serde_json::Value, mlua::Result<MultiValue>) {
        self.verify();
        let args = MultiValue::from_vec(args);
        let supplied = observer::graph(&self.source, args.clone());
        let info = entry.info();
        let input = json!({
            "entry_source": info.source,
            "entry_first_line": info.line_defined,
            "entry_last_line": info.last_line_defined,
            "supplied_inputs": supplied,
            "internal_call_frames_observed": false
        });
        let output = entry.call::<MultiValue>(args);
        self.verify();
        (input, output)
    }
    pub fn public(&self, line: &[u8]) -> (serde_json::Value, mlua::Result<MultiValue>) {
        self.run(
            &self.source.public.parse,
            vec![self.text(line), Value::Boolean(false)],
        )
    }
    pub fn direct(&self, args: Vec<Value>) -> (serde_json::Value, mlua::Result<MultiValue>) {
        self.run(&self.helper, args)
    }
}
