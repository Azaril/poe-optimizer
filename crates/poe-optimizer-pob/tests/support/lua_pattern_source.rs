//! Independent byte-string observations from the bundled LuaJIT C pattern engine.
use mlua::{Function, HookTriggers, Lua, MultiValue, Table, Value, VmState};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Capture {
    Bytes(Vec<u8>),
    Position(usize),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Find {
    Absent,
    Match {
        start: usize,
        end: usize,
        captures: Vec<Capture>,
    },
    Error(String),
}
pub struct PatternSource {
    pub lua: Lua,
}
impl PatternSource {
    pub fn new() -> Self {
        let lua = Lua::new();
        lua.set_memory_limit(64 * 1024 * 1024).unwrap();
        let start = Instant::now();
        lua.set_hook(
            HookTriggers::new().every_nth_instruction(10_000),
            move |_, _| {
                if start.elapsed() > Duration::from_secs(30) {
                    Err(mlua::Error::RuntimeError("pattern oracle deadline".into()))
                } else {
                    Ok(VmState::Continue)
                }
            },
        )
        .unwrap();
        lua.load("jit.off(); jit.flush()").exec().unwrap();
        Self { lua }
    }
    pub fn find(&self, subject: &[u8], pattern: &[u8], init: i32, plain: bool) -> Find {
        let function = self
            .lua
            .globals()
            .get::<Table>("string")
            .unwrap()
            .get::<Function>("find")
            .unwrap();
        let result = function.call::<MultiValue>((
            self.lua.create_string(subject).unwrap(),
            self.lua.create_string(pattern).unwrap(),
            init,
            plain,
        ));
        match result {
            Err(error) => Find::Error(error.to_string()),
            Ok(values) => {
                if matches!(values.front(), Some(Value::Nil) | None) {
                    return Find::Absent;
                }
                let values = values.into_iter().collect::<Vec<_>>();
                let number = |value: &Value| match value {
                    Value::Integer(v) => usize::try_from(*v).unwrap(),
                    Value::Number(v) => *v as usize,
                    other => panic!("original find returned unexpected offset {other:?}"),
                };
                let captures = values[2..]
                    .iter()
                    .map(|v| match v {
                        Value::String(s) => Capture::Bytes(s.as_bytes().to_vec()),
                        Value::Integer(_) | Value::Number(_) => Capture::Position(number(v)),
                        other => panic!("original find returned unexpected capture {other:?}"),
                    })
                    .collect();
                Find::Match {
                    start: number(&values[0]) - 1,
                    end: number(&values[1]),
                    captures,
                }
            }
        }
    }
    pub fn match_captures(
        &self,
        subject: &[u8],
        pattern: &[u8],
        init: i32,
    ) -> Result<Option<Vec<Capture>>, String> {
        let function = self
            .lua
            .globals()
            .get::<Table>("string")
            .unwrap()
            .get::<Function>("match")
            .unwrap();
        let values = function
            .call::<MultiValue>((
                self.lua.create_string(subject).unwrap(),
                self.lua.create_string(pattern).unwrap(),
                init,
            ))
            .map_err(|e| e.to_string())?;
        if matches!(values.front(), Some(Value::Nil) | None) {
            return Ok(None);
        }
        Ok(Some(
            values
                .into_iter()
                .map(|v| match v {
                    Value::String(s) => Capture::Bytes(s.as_bytes().to_vec()),
                    Value::Integer(n) => Capture::Position(n as usize),
                    Value::Number(n) => Capture::Position(n as usize),
                    other => panic!("unexpected source match capture {other:?}"),
                })
                .collect(),
        ))
    }
    pub fn warm(&self) -> Table {
        self.lua
            .load(include_str!("lua_pattern_warm.lua"))
            .set_name("@test-only-pattern-warm")
            .eval()
            .unwrap()
    }
}
