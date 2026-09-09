//! Original full parser with optional labelled policy substitution and call observers.
use super::{
    factory_source::{FactorySource, upvalue},
    public_source::{Graph, Observation},
    runtime,
};
use mlua::{Function, MultiValue, Table, Value};
#[derive(Debug)]
pub struct Call {
    pub label: String,
    pub frames: Vec<usize>,
    pub count: usize,
    pub arguments: Graph,
}
pub fn clear(table: &Table) {
    for key in table
        .clone()
        .pairs::<Value, Value>()
        .map(|v| v.unwrap().0)
        .collect::<Vec<_>>()
    {
        table.set(key, Value::Nil).unwrap();
    }
}
pub struct OrdinarySource {
    pub source: FactorySource,
}
impl OrdinarySource {
    pub fn new() -> Self {
        Self::configured(None)
    }
    pub fn configured(pattern: Option<&str>) -> Self {
        let mut source = FactorySource::new();
        if let Some(pattern) = pattern {
            // Explicit configured-definition fixture: only these two data literals differ.
            // Other original module bytes, call order and callback execution are unchanged.
            assert!(!pattern.contains(['"', '\\', '\n']));
            let text = runtime::verified("src/Modules/ModParser.lua").unwrap();
            let original = "tagCap[1]:match(\"%d+\")";
            assert_eq!(text.matches(original).count(), 2);
            let text = text.replace(original, &format!("tagCap[1]:match(\"{pattern}\")"));
            let (parse, cache): (Function, Table) = source
                .public
                .source
                .lua
                .load(text)
                .set_name("@src/Modules/ModParser.lua")
                .eval()
                .unwrap();
            source.public.parse = parse;
            source.public.cache = cache;
            source.internal = upvalue(&source.public.source.lua, &source.public.parse, "parseMod")
                .as_function()
                .unwrap()
                .clone();
            source.special = upvalue(
                &source.public.source.lua,
                &source.internal,
                "specialModList",
            )
            .as_table()
            .unwrap()
            .clone();
        }
        let lua = &source.public.source.lua;
        lua.globals()
            .set("ordinary_events", lua.create_table().unwrap())
            .unwrap();
        lua.globals()
            .set(
                "ordinary_record",
                lua.create_function(|lua, row: Table| {
                    let frames = (0..12)
                        .filter_map(|level| {
                            lua.inspect_stack(level, |d| {
                                (d.source().source.as_deref() == Some("@src/Modules/ModParser.lua"))
                                    .then(|| d.current_line())
                                    .flatten()
                            })
                            .flatten()
                        })
                        .collect::<Vec<_>>();
                    row.set("source_frames", frames)?;
                    let events: Table = lua.globals().get("ordinary_events")?;
                    events.set(events.raw_len() + 1, row)?;
                    Ok(())
                })
                .unwrap(),
            )
            .unwrap();
        Self { source }
    }
    pub fn wrap(&self, callback: Function, label: &str) -> Function {
        self.source.public.source.lua.load("return function(callback,label) return function(...) ordinary_record({label=label,args={count=select('#',...),...}}); return callback(...) end end").set_name("@test-only-ordinary-callback-observer").eval::<Function>().unwrap().call((callback,label)).unwrap()
    }
    pub fn fixture(&self, label: &str, body: &str) -> Function {
        let callback = self
            .source
            .public
            .source
            .lua
            .load(format!("return function(num,a,b,c,d,e) {body} end"))
            .set_name("@test-only-ordinary-caller-recipe")
            .eval()
            .unwrap();
        self.wrap(callback, label)
    }
    pub fn variadic_fixture(&self, label: &str, body: &str) -> Function {
        let callback = self
            .source
            .public
            .source
            .lua
            .load(format!("return function(...) {body} end"))
            .set_name("@test-only-ordinary-protocol-control")
            .eval()
            .unwrap();
        self.wrap(callback, label)
    }
    pub fn observe(&self, text: &[u8]) -> Observation {
        self.source.public.cache.clear().unwrap();
        self.source
            .public
            .source
            .lua
            .globals()
            .get::<Table>("ordinary_events")
            .unwrap()
            .clear()
            .unwrap();
        self.source.public.observe(text)
    }
    pub fn calls(&self) -> Vec<Call> {
        self.source
            .public
            .source
            .lua
            .globals()
            .get::<Table>("ordinary_events")
            .unwrap()
            .sequence_values::<Table>()
            .map(|row| {
                let row = row.unwrap();
                let args: Table = row.get("args").unwrap();
                let count: usize = args.get("count").unwrap();
                let values = (1..=count).map(|i| args.get::<Value>(i).unwrap()).collect();
                Call {
                    label: row.get("label").unwrap(),
                    frames: row.get("source_frames").unwrap(),
                    count,
                    arguments: self
                        .source
                        .public
                        .capture(MultiValue::from_vec(values))
                        .unwrap(),
                }
            })
            .collect()
    }
}
