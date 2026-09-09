//! Original full parser with optional, labelled runtime-helper definition injection.
use super::{factory_source::upvalue, ordinary_source::OrdinarySource, runtime};
use mlua::{Function, Value};
use std::ops::Deref;
pub struct StringSource {
    pub ordinary: OrdinarySource,
    pub upper: Function,
}
impl Deref for StringSource {
    type Target = OrdinarySource;
    fn deref(&self) -> &Self::Target {
        &self.ordinary
    }
}
impl StringSource {
    pub fn new(pattern: Option<&str>) -> Self {
        // Dictionary construction always executes the unchanged original module.
        // Its helper is also used at construction time (for example line6331).
        let ordinary = OrdinarySource::new();
        let callback = ordinary
            .source
            .special
            .clone()
            .pairs::<Value, Value>()
            .map(Result::unwrap)
            .find_map(|(_, value)| match value {
                Value::Function(f) if f.info().line_defined == Some(2712) => Some(f),
                _ => None,
            })
            .unwrap();
        let mut upper = upvalue(
            &ordinary.source.public.source.lua,
            &callback,
            "firstToUpper",
        )
        .as_function()
        .unwrap()
        .clone();
        if let Some(pattern) = pattern {
            let source = runtime::verified("src/Modules/ModParser.lua").unwrap();
            let helper = source
                .lines()
                .skip(12)
                .take(3)
                .collect::<Vec<_>>()
                .join("\n");
            let literal = "str:gsub(\"^%l\", string.upper)";
            assert_eq!(helper.matches(literal).count(), 1);
            let replacement = format!(
                "str:gsub({}, string.upper)",
                serde_json::to_string(pattern).unwrap()
            );
            // This is the complete original helper body with one explicit caller
            // pattern replacement, used only by the selected injected fixture.
            // It is not a claim that an altered full-module construction succeeds.
            upper = ordinary
                .source
                .public
                .source
                .lua
                .load(format!(
                    "{}{}\nreturn firstToUpper",
                    "\n".repeat(12),
                    helper.replace(literal, &replacement)
                ))
                .set_name("@src/Modules/ModParser.lua")
                .eval()
                .unwrap();
        }
        assert_eq!(upper.info().line_defined, Some(13));
        assert_eq!(upper.info().last_line_defined, Some(15));
        assert_eq!(upper.info().num_upvalues, 0);
        Self { ordinary, upper }
    }
    pub fn fixture(&self, label: &str, body: &str) -> Function {
        let factory: Function = self
            .source
            .public
            .source
            .lua
            .load(format!(
                "return function(mod,firstToUpper) return function(num,a,b,c,d,e) {body} end end"
            ))
            .set_name("@test-only-string-factory-caller-recipe")
            .eval()
            .unwrap();
        self.wrap(
            factory
                .call((self.source.create_mod.clone(), self.upper.clone()))
                .unwrap(),
            label,
        )
    }
}
