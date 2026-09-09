//! Independent complete public ModParser source and lossless returned-value graph.
use super::runtime;
use mlua::{Function, MultiValue, Table, Value};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Atom {
    Nil,
    Boolean(bool),
    /// Lua numbers are IEEE doubles even when mlua exposes an integer variant.
    Number(u64),
    Bytes(Vec<u8>),
    Table(usize),
    Function(usize),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Callback {
    pub source: Vec<u8>,
    pub first_line: i64,
    pub last_line: i64,
    pub kind: Vec<u8>,
    pub upvalues: Vec<(Vec<u8>, Atom)>,
    pub builtin: Option<String>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Graph {
    pub roots: Vec<Atom>,
    pub tables: Vec<Vec<(Atom, Atom)>>,
    pub callbacks: Vec<Callback>,
}
struct Capture<'a> {
    source: &'a PublicSource,
    graph: Graph,
    tables: BTreeMap<usize, usize>,
    functions: BTreeMap<usize, usize>,
    bytes: usize,
    values: usize,
    named_primitives: bool,
}
impl Capture<'_> {
    fn value(&mut self, value: Value, depth: usize) -> Result<Atom, String> {
        self.values += 1;
        if depth > 256 || self.values > 1_000_000 || self.bytes > 16 * 1024 * 1024 {
            return Err("observer graph resource bound".into());
        }
        Ok(match value {
            Value::Nil => Atom::Nil,
            Value::Boolean(v) => Atom::Boolean(v),
            Value::Integer(v) => Atom::Number((v as f64).to_bits()),
            Value::Number(v) => Atom::Number(v.to_bits()),
            Value::String(v) => {
                self.bytes += v.as_bytes().len();
                Atom::Bytes(v.as_bytes().to_vec())
            }
            Value::Table(table) => {
                let pointer = table.to_pointer() as usize;
                if let Some(id) = self.tables.get(&pointer) {
                    return Ok(Atom::Table(*id));
                }
                let id = self.graph.tables.len();
                self.tables.insert(pointer, id);
                self.graph.tables.push(vec![]);
                let mut entries = vec![];
                for entry in table.pairs::<Value, Value>() {
                    let (key, value) = entry.map_err(|e| e.to_string())?;
                    if !matches!(
                        key,
                        Value::Boolean(_) | Value::Integer(_) | Value::Number(_) | Value::String(_)
                    ) {
                        return Err(format!(
                            "observer cannot canonically order table key: {}",
                            key.type_name()
                        ));
                    }
                    entries.push((self.value(key, depth + 1)?, value));
                }
                entries.sort_by(|a, b| a.0.cmp(&b.0));
                let mut out = vec![];
                for (key, value) in entries {
                    out.push((key, self.value(value, depth + 1)?));
                }
                self.graph.tables[id] = out;
                Atom::Table(id)
            }
            Value::Function(function) => {
                let pointer = function.to_pointer() as usize;
                if let Some(id) = self.functions.get(&pointer) {
                    return Ok(Atom::Function(*id));
                }
                let id = self.graph.callbacks.len();
                self.functions.insert(pointer, id);
                self.graph.callbacks.push(Callback {
                    source: vec![],
                    first_line: 0,
                    last_line: 0,
                    kind: vec![],
                    upvalues: vec![],
                    builtin: None,
                });
                let descriptor: Table = self
                    .source
                    .describe
                    .call(function)
                    .map_err(|e| e.to_string())?;
                let bytes = |key: &str| {
                    descriptor
                        .get::<mlua::LuaString>(key)
                        .map(|v| v.as_bytes().to_vec())
                        .map_err(|e| e.to_string())
                };
                let mut callback = Callback {
                    source: bytes("source")?,
                    first_line: descriptor.get("first_line").map_err(|e| e.to_string())?,
                    last_line: descriptor.get("last_line").map_err(|e| e.to_string())?,
                    kind: bytes("kind")?,
                    upvalues: vec![],
                    builtin: self.source.builtins.get(&pointer).cloned(),
                };
                // A named C primitive is the explicit portable operation seam.
                // Raw capture still observes its private C closure upvalues.
                if !(self.named_primitives && callback.builtin.is_some()) {
                    for row in descriptor
                        .get::<Table>("upvalues")
                        .map_err(|e| e.to_string())?
                        .sequence_values::<Table>()
                    {
                        let row = row.map_err(|e| e.to_string())?;
                        let name = row
                            .get::<mlua::LuaString>(1)
                            .map_err(|e| e.to_string())?
                            .as_bytes()
                            .to_vec();
                        self.bytes += name.len();
                        callback.upvalues.push((
                            name,
                            self.value(row.get(2).map_err(|e| e.to_string())?, depth + 1)?,
                        ));
                    }
                }
                self.graph.callbacks[id] = callback;
                Atom::Function(id)
            }
            other => return Err(format!("observer does not serialize {}", other.type_name())),
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", content = "value", rename_all = "snake_case")]
pub enum Observation {
    Returned(Graph),
    SourceError(String),
    ObserverError(String),
}
pub struct PublicSource {
    pub source: runtime::Oracle,
    pub parse: Function,
    pub cache: Table,
    describe: Function,
    builtins: BTreeMap<usize, String>,
}
impl PublicSource {
    pub fn new() -> Self {
        let source = runtime::Oracle::new();
        // Evaluate every byte of the authenticated original module, including its
        // real dictionary construction and unchanged public cache/retry wrapper.
        // The host's prior Item loads use a different module instance and cache.
        let (parse, cache): (Function, Table) = source
            .lua
            .load(runtime::verified("src/Modules/ModParser.lua").unwrap())
            .set_name("@src/Modules/ModParser.lua")
            .eval()
            .unwrap();
        assert_eq!(parse.info().line_defined, Some(7404));
        assert_eq!(parse.info().last_line_defined, Some(7423));
        let describe = source
            .lua
            .create_function(|lua, function: Function| {
                let info = function.info();
                let descriptor = lua.create_table()?;
                descriptor.set("source", info.source.unwrap_or_default())?;
                descriptor.set("first_line", info.line_defined.map_or(-1, |v| v as i64))?;
                descriptor.set("last_line", info.last_line_defined.map_or(-1, |v| v as i64))?;
                descriptor.set("kind", info.what)?;
                let upvalues = lua.create_table()?;
                for index in 1..=i32::from(info.num_upvalues) {
                    // SAFETY: mlua places exactly one Function on the protected stack.
                    // lua_getupvalue only reads that validated function. The name is
                    // copied immediately while its function is rooted; leave exactly
                    // name and value for mlua conversion. No debug mutation is exposed.
                    let (name, value): (mlua::LuaString, Value) = unsafe {
                        lua.exec_raw(function.clone(), |state| {
                            let name = mlua::ffi::lua_getupvalue(state, 1, index);
                            mlua::ffi::lua_pushstring(state, name);
                            mlua::ffi::lua_insert(state, -2);
                            mlua::ffi::lua_remove(state, 1);
                        })?
                    };
                    let row = lua.create_table()?;
                    row.raw_set(1, name)?;
                    row.raw_set(2, value)?;
                    upvalues.raw_set(index, row)?;
                }
                descriptor.set("upvalues", upvalues)?;
                Ok(descriptor)
            })
            .unwrap();
        Self {
            source,
            parse,
            cache,
            describe,
            builtins: BTreeMap::new(),
        }
    }
    pub fn bind_builtin_symbols<'a>(&mut self, symbols: impl IntoIterator<Item = &'a str>) {
        for symbol in symbols {
            let mut value = Value::Table(self.source.lua.globals());
            for part in symbol.split('.') {
                value = value
                    .as_table()
                    .expect("original builtin path parent")
                    .get::<Value>(part)
                    .unwrap();
            }
            let function = value.as_function().expect("original builtin function");
            assert_eq!(
                function.info().what,
                "C",
                "catalog builtin is not a source builtin"
            );
            self.builtins
                .insert(function.to_pointer() as usize, symbol.to_owned());
        }
    }
    pub fn warm(&self) -> Table {
        self.source
            .lua
            .globals()
            .set("oracle_public_parse", self.parse.clone())
            .unwrap();
        self.source
            .lua
            .globals()
            .set("oracle_public_cache", self.cache.clone())
            .unwrap();
        self.source
            .lua
            .load(include_str!("mod_parser_public_warm.lua"))
            .set_name("@test-only-original-public-parser-warm")
            .eval()
            .unwrap()
    }
    pub fn raw(&self, line: &[u8]) -> mlua::Result<MultiValue> {
        self.parse
            .call((self.source.lua.create_string(line)?, false))
    }
    pub fn capture(&self, values: MultiValue) -> Result<Graph, String> {
        self.capture_at_boundary(values, false)
    }
    fn capture_at_boundary(
        &self,
        values: MultiValue,
        named_primitives: bool,
    ) -> Result<Graph, String> {
        let mut capture = Capture {
            source: self,
            graph: Graph::default(),
            tables: BTreeMap::new(),
            functions: BTreeMap::new(),
            bytes: 0,
            values: 0,
            named_primitives,
        };
        for value in values {
            let atom = capture.value(value, 0)?;
            capture.graph.roots.push(atom);
        }
        Ok(capture.graph)
    }
    pub fn observe_primitives(&self, line: &[u8]) -> Observation {
        match self.raw(line) {
            Ok(values) => match self.capture_at_boundary(values, true) {
                Ok(graph) => Observation::Returned(graph),
                Err(error) => Observation::ObserverError(error),
            },
            Err(error) => Observation::SourceError(error.to_string()),
        }
    }
    pub fn observe(&self, line: &[u8]) -> Observation {
        match self.raw(line) {
            Ok(values) => match self.capture(values) {
                Ok(graph) => Observation::Returned(graph),
                Err(error) => Observation::ObserverError(error),
            },
            Err(error) => Observation::SourceError(error.to_string()),
        }
    }
}
