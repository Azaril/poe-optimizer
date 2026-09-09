//! Retained original Flag and optional labelled runtime prefix definitions.
use super::{factory_source::upvalue, runtime, string_source::StringSource};
use mlua::{Function, Value};
use std::ops::Deref;
pub struct FlagSource {
    pub string: StringSource,
    pub flag: Function,
}
impl Deref for FlagSource {
    type Target = StringSource;
    fn deref(&self) -> &Self::Target {
        &self.string
    }
}
impl FlagSource {
    pub fn new(prefix: Option<(&str, bool)>) -> Self {
        let string = StringSource::new(None);
        let owner = string
            .source
            .special
            .clone()
            .pairs::<Value, Value>()
            .map(Result::unwrap)
            .find_map(|(_, value)| match value {
                Value::Function(f) if f.info().line_defined == Some(5632) => Some(f),
                _ => None,
            })
            .unwrap();
        let flag = upvalue(&string.source.public.source.lua, &owner, "flag")
            .as_function()
            .unwrap()
            .clone();
        assert_eq!(flag.info().line_defined, Some(2177));
        assert_eq!(flag.info().last_line_defined, Some(2179));
        assert_eq!(flag.info().num_upvalues, 1);
        let constructor = upvalue(&string.source.public.source.lua, &flag, "mod")
            .as_function()
            .unwrap()
            .clone();
        assert_eq!(
            constructor.to_pointer(),
            string.source.create_mod.to_pointer()
        );
        let mut out = Self { string, flag };
        if let Some(prefix) = prefix {
            out.flag = out.helper(out.source.create_mod.clone(), Some(prefix));
        }
        out
    }
    pub fn helper(&self, constructor: Function, prefix: Option<(&str, bool)>) -> Function {
        let source = runtime::verified("src/Modules/ModParser.lua").unwrap();
        let mut body = source
            .lines()
            .skip(2176)
            .take(3)
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(body.matches("mod(name, \"FLAG\", true, ...)").count(), 1);
        if let Some((kind, value)) = prefix {
            // This explicit caller definition changes only two typed literals in
            // the complete original wrapper, after normal dictionary construction.
            body = body.replace(
                "mod(name, \"FLAG\", true, ...)",
                &format!(
                    "mod(name, \"{}\", {}, ...)",
                    kind.bytes()
                        .map(|b| format!("\\{b:03}"))
                        .collect::<String>(),
                    value
                ),
            );
        }
        let factory: Function = self
            .source
            .public
            .source
            .lua
            .load(format!(
                "return function(mod)\n{}{}\nreturn flag end",
                "\n".repeat(2175),
                body
            ))
            .set_name("@src/Modules/ModParser.lua")
            .eval()
            .unwrap();
        factory.call(constructor).unwrap()
    }
    pub fn fixture(&self, label: &str, body: &str) -> Function {
        let factory:Function=self.source.public.source.lua.load(format!("return function(mod,flag,firstToUpper) return function(num,a,b,c,d,e) {body} end end")).set_name("@test-only-flag-factory-caller-recipe").eval().unwrap();
        self.wrap(
            factory
                .call((
                    self.source.create_mod.clone(),
                    self.flag.clone(),
                    self.upper.clone(),
                ))
                .unwrap(),
            label,
        )
    }
}
