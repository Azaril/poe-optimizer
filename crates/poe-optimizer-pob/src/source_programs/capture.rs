//! Bounded observation of constructed closure graphs in an authenticated Lua host.
//!
//! The host must capture before executing source, load exactly the supplied source
//! bytes, and supply its observed module order/construction evidence. Debug names
//! and file hashes alone cannot prove what Lua previously executed. This observer
//! never runs callbacks, constructs classes, or converts proxies into plain tables.
use super::{MAX_SOURCE_BYTES, MAX_SOURCE_FILES, Result, error, hash, validate_sources};
use mlua::{Function, Lua, Table, Value};
use poe_optimizer_data::{
    item_loading::{ItemLoadingSource, ItemSourceSpan},
    source_program::*,
};
use std::collections::BTreeMap;

mod classes;
pub(crate) mod closures;
mod constructors;
pub use closures::ObservedSourceClosureCreations;
pub use constructors::ObservedSourceConstructors;
mod context;
mod iteration;
mod session;
mod upvalues;
pub use classes::{ObservedSourceClasses, SourceClassCaptureRequest, SourceClassSelection};
pub use context::{
    ObservedSourceContext, SourceCaptureContext, SourceEnvironmentSelection, SourceTableSelection,
};
pub use session::{ObservedSourceSession, SourceSessionCaptureRequest};

const MAX_VALUES: usize = 1_000_000;
const MAX_TABLES: usize = 100_000;
const MAX_CALLBACKS: usize = 20_000;
const MAX_TEXT_BYTES: usize = 16 * 1024 * 1024;

/// Original primitive identities retained before authenticated source runs.
/// Fresh-host ownership is an adapter contract, not inferred from function names.
pub struct SourceClosureObserver {
    globals: Table,
    libraries: BTreeMap<String, Table>,
    primitives: Vec<(SourceProgramIntrinsic, Function)>,
    opaque_primitives: Vec<(String, Function)>,
    lua: Lua,
    get_metatable: Function,
    string_metatable: Table,
    iterator_primitives: iteration::Primitives,
    constructor_reflection: Option<constructors::Reflection>,
    closure_reflection: Option<closures::Reflection>,
}

/// Complete observed dependency graph. Named callback roots are not executable
/// admission; lower every body and inspect unsupported dependencies separately.
#[derive(Debug, Clone)]
pub struct ObservedSourceClosures {
    definitions: SourceProgramDefinitions,
    callbacks: BTreeMap<String, SourceCallbackId>,
}
impl ObservedSourceClosures {
    pub fn definitions(&self) -> &SourceProgramDefinitions {
        &self.definitions
    }
    pub fn callbacks(&self) -> &BTreeMap<String, SourceCallbackId> {
        &self.callbacks
    }
    pub fn into_parts(self) -> (SourceProgramDefinitions, BTreeMap<String, SourceCallbackId>) {
        (self.definitions, self.callbacks)
    }
}
impl SourceClosureObserver {
    pub fn capture_before_source(lua: &Lua) -> Result<Self> {
        let globals = lua.globals();
        plain(&globals, "global environment")?;
        let mut libraries = BTreeMap::new();
        let mut primitives = Vec::new();
        for operation in [
            SourceProgramIntrinsic::Unpack,
            SourceProgramIntrinsic::Type,
            SourceProgramIntrinsic::Select,
            SourceProgramIntrinsic::ToNumber,
            SourceProgramIntrinsic::Ipairs,
            SourceProgramIntrinsic::TableInsert,
            SourceProgramIntrinsic::StringGsub,
            SourceProgramIntrinsic::StringGmatch,
            SourceProgramIntrinsic::ToString,
            SourceProgramIntrinsic::MathFloor,
            SourceProgramIntrinsic::MathMin,
            SourceProgramIntrinsic::MathMax,
            SourceProgramIntrinsic::StringMatch,
            SourceProgramIntrinsic::StringLower,
            SourceProgramIntrinsic::StringFind,
            SourceProgramIntrinsic::StringSub,
            SourceProgramIntrinsic::BitBand,
            SourceProgramIntrinsic::BitBor,
            SourceProgramIntrinsic::BitBxor,
            SourceProgramIntrinsic::BitBnot,
        ] {
            let path = operation.global_path().expect("language primitive");
            let table = if path.len() == 2 {
                let library: Table = globals.raw_get(path[0])?;
                plain(&library, "primitive library")?;
                libraries.insert(path[0].into(), library.clone());
                library
            } else {
                globals.clone()
            };
            let function: Function = table.raw_get(path[path.len() - 1])?;
            if function.info().what != "C" {
                return Err(error(format!(
                    "source observer primitive {} is not an original C function",
                    path.join(".")
                )));
            }
            primitives.push((operation, function));
        }
        let mut opaque_primitives = Vec::new();
        for symbol in [
            "pairs",
            "string.format",
            "rawget",
            "setmetatable",
            "error",
            "assert",
        ] {
            let path = symbol.split('.').collect::<Vec<_>>();
            let table = if path.len() == 2 {
                let library: Table = globals.raw_get(path[0])?;
                plain(&library, "primitive library")?;
                libraries.insert(path[0].into(), library.clone());
                library
            } else {
                globals.clone()
            };
            let function: Function = table.raw_get(path[path.len() - 1])?;
            if function.info().what != "C" {
                return Err(error(format!(
                    "source observer primitive {symbol} is not an original C function"
                )));
            }
            opaque_primitives.push((symbol.into(), function));
        }
        let iterator_primitives =
            iteration::Primitives::capture(lua, &globals, &opaque_primitives)?;
        let get_metatable: Function = globals.raw_get("getmetatable")?;
        if get_metatable.info().what != "C" {
            return Err(error(
                "source observer getmetatable is not an original C function",
            ));
        }
        let string_metatable: Table = get_metatable.call("")?;
        let observer = Self {
            globals,
            libraries,
            primitives,
            opaque_primitives,
            lua: lua.clone(),
            get_metatable,
            string_metatable,
            iterator_primitives,
            constructor_reflection: None,
            closure_reflection: None,
        };
        observer.verify(lua)?;
        Ok(observer)
    }
    /// Observe all actual captures of the supplied named functions. Metatables,
    /// unknown builtins, non-global environments and out-of-inventory source
    /// dependencies are errors, including branches that may not execute now.
    pub fn observe(
        &self,
        lua: &Lua,
        sources: &BTreeMap<String, String>,
        source: ItemLoadingSource,
        roots: &BTreeMap<String, Function>,
    ) -> Result<ObservedSourceClosures> {
        let observed = self.observe_with_context(
            lua,
            sources,
            source,
            roots,
            SourceCaptureContext::default(),
        )?;
        Ok(ObservedSourceClosures {
            definitions: observed
                .owner
                .definitions()
                .expect("standalone observed owner")
                .clone(),
            callbacks: observed.callbacks,
        })
    }
    /// Capture explicit projections and, optionally, the actual global environment.
    /// Global values are then dispatched from their captured table, so names may
    /// have changed since startup. Captured primitive identities and the original
    /// string-method contract remain authenticated independently.
    pub fn observe_with_context(
        &self,
        lua: &Lua,
        sources: &BTreeMap<String, String>,
        source: ItemLoadingSource,
        roots: &BTreeMap<String, Function>,
        context: SourceCaptureContext,
    ) -> Result<ObservedSourceContext> {
        self.verify_capture_context(lua, context.environment.is_some())?;
        context::validate_source_aliases(sources, &context.source_names)?;
        if roots.len() > 4096
            || sources.len() > MAX_SOURCE_FILES
            || sources.len() != source.files.len()
        {
            return Err(error("source observer root/file inventory bound"));
        }
        let mut bytes = 0usize;
        for (path, expected) in &source.files {
            let text = sources
                .get(path)
                .ok_or_else(|| error(format!("source observer missing file {path}")))?;
            bytes = bytes
                .checked_add(text.len())
                .ok_or_else(|| error("source observer byte overflow"))?;
            if bytes > MAX_SOURCE_BYTES || hash(text.as_bytes()) != *expected {
                return Err(error(format!(
                    "source observer file hash mismatch or byte bound: {path}"
                )));
            }
        }
        let mut graph = Graph {
            observer: self,
            sources,
            tables: vec![],
            callbacks: vec![],
            seen_tables: BTreeMap::new(),
            seen_callbacks: BTreeMap::new(),
            intrinsics: BTreeMap::new(),
            values: 0,
            text_bytes: 0,
            forbidden_tables: std::collections::BTreeSet::new(),
            forbidden_callbacks: std::collections::BTreeSet::new(),
            forbidden_cells: std::collections::BTreeSet::new(),
            immutable_capture_tables: None,
            session_tables: 0,
            source_names: &context.source_names,
            context: SourceProgramContext::default(),
            constructor_observations: constructors::Pending::default(),
            closure_observations: closures::Pending::default(),
        };
        graph.register_projections(&context)?;
        graph.capture_projections(&context)?;
        let mut callbacks = BTreeMap::new();
        for (name, function) in roots {
            graph.text(name.len())?;
            if name.is_empty() || name.len() > 256 || name.contains('\0') {
                return Err(error("invalid observed callback root name"));
            }
            let SourceValue::Callback(id) = graph
                .value(Value::Function(function.clone()), 0)
                .map_err(|e| error(format!("source callback root {name}: {e}")))?
            else {
                unreachable!()
            };
            callbacks.insert(name.clone(), id);
        }
        let mut definition_roots = Vec::new();
        graph.capture_environment(&context, &mut definition_roots)?;
        let mut prototypes = SourceClosurePrototypes {
            schema_version: SOURCE_CLOSURE_PROTOTYPES_SCHEMA_VERSION,
            prototypes: vec![],
        };
        graph.finish_closure_creations(&mut prototypes)?;
        let definitions = SourceProgramDefinitions {
            schema_version: SOURCE_PROGRAM_DEFINITIONS_SCHEMA_VERSION,
            source,
            tables: graph.tables,
            callbacks: graph.callbacks,
            roots: definition_roots,
            intrinsics: graph.intrinsics,
        };
        let owner = if self.closure_reflection.is_some() {
            SourceProgramOwner::new_with_closures(
                definitions,
                None,
                Some(graph.context),
                prototypes,
            )
        } else {
            SourceProgramOwner::new_with_context(definitions, None, graph.context)
        }
        .map_err(error)?;
        validate_sources(sources, &owner)?;
        self.verify_capture_context(lua, context.environment.is_some())?;
        self.verify_iteration(context.capture_iteration)?;
        let constructor_observations =
            self.bind_constructors(&owner, graph.constructor_observations);
        let closure_observations = self.bind_closure_creations(&owner, graph.closure_observations);
        Ok(ObservedSourceContext {
            closure_observations,
            owner,
            callbacks,
            constructor_observations,
        })
    }
    fn verify_capture_context(&self, lua: &Lua, explicit_environment: bool) -> Result<()> {
        if !explicit_environment {
            return self.verify(lua);
        }
        if lua.globals().to_pointer() != self.globals.to_pointer() {
            return Err(error("source observer belongs to another Lua host"));
        }
        plain(&self.globals, "global environment")?;
        let actual: Table = self.get_metatable.call("")?;
        if actual.to_pointer() != self.string_metatable.to_pointer() {
            return Err(error("source observer string metatable was rebound"));
        }
        plain(&actual, "string metatable")?;
        let index: Table = actual.raw_get("__index")?;
        if index.to_pointer() != self.libraries["string"].to_pointer() {
            return Err(error("source observer string method index was rebound"));
        }
        for entry in actual.pairs::<Value, Value>() {
            let (key, _) = entry?;
            if !matches!(key, Value::String(key) if key.as_bytes().as_ref()==b"__index") {
                return Err(error(
                    "source observer string metatable has an unmodeled field",
                ));
            }
        }
        for (operation, original) in &self.primitives {
            if operation.is_string_method() {
                let path = operation.global_path().expect("string primitive");
                let actual: Function = self.libraries["string"].raw_get(path[1])?;
                if actual.to_pointer() != original.to_pointer() {
                    return Err(error("source observer original string method was rebound"));
                }
            }
        }
        Ok(())
    }
    fn verify(&self, lua: &Lua) -> Result<()> {
        let globals = lua.globals();
        if globals.to_pointer() != self.globals.to_pointer() {
            return Err(error("source observer belongs to another Lua host"));
        }
        plain(&globals, "global environment")?;
        for (name, original) in &self.libraries {
            let actual: Table = globals.raw_get(name.as_str())?;
            plain(&actual, "primitive library")?;
            if actual.to_pointer() != original.to_pointer() {
                return Err(error(format!(
                    "source observer original library {name} was rebound"
                )));
            }
        }
        for (operation, original) in &self.primitives {
            let path = operation.global_path().expect("language primitive");
            let table = if path.len() == 2 {
                &self.libraries[path[0]]
            } else {
                &self.globals
            };
            let actual: Function = table.raw_get(path[path.len() - 1])?;
            if actual.to_pointer() != original.to_pointer() {
                return Err(error(format!(
                    "source observer original primitive {} was rebound",
                    path.join(".")
                )));
            }
        }
        for (symbol, original) in &self.opaque_primitives {
            let path = symbol.split('.').collect::<Vec<_>>();
            let table = if path.len() == 2 {
                &self.libraries[path[0]]
            } else {
                &self.globals
            };
            let actual: Function = table.raw_get(path[path.len() - 1])?;
            if actual.to_pointer() != original.to_pointer() {
                return Err(error(format!(
                    "source observer original primitive {symbol} was rebound"
                )));
            }
        }
        let original: Function = globals.raw_get("getmetatable")?;
        if original.to_pointer() != self.get_metatable.to_pointer() {
            return Err(error("source observer original getmetatable was rebound"));
        }
        let actual: Table = self.get_metatable.call("")?;
        if actual.to_pointer() != self.string_metatable.to_pointer() {
            return Err(error("source observer string metatable was rebound"));
        }
        plain(&actual, "string metatable")?;
        let index: Table = actual.raw_get("__index")?;
        if index.to_pointer() != self.libraries["string"].to_pointer() {
            return Err(error("source observer string method index was rebound"));
        }
        for entry in actual.pairs::<Value, Value>() {
            let (key, _) = entry?;
            if !matches!(key, Value::String(key) if key.as_bytes().as_ref()==b"__index") {
                return Err(error(
                    "source observer string metatable has an unmodeled field",
                ));
            }
        }
        Ok(())
    }
}
fn plain(table: &Table, role: &str) -> Result<()> {
    if table.metatable().is_some() {
        return Err(error(format!(
            "source observer {role} metatable/proxy is not represented"
        )));
    }
    Ok(())
}
struct Graph<'a> {
    observer: &'a SourceClosureObserver,
    sources: &'a BTreeMap<String, String>,
    source_names: &'a BTreeMap<String, String>,
    tables: Vec<SourceTable>,
    callbacks: Vec<SourceCallback>,
    seen_tables: BTreeMap<usize, SourceTableId>,
    seen_callbacks: BTreeMap<usize, SourceCallbackId>,
    intrinsics: BTreeMap<SourceCallbackId, SourceProgramIntrinsic>,
    values: usize,
    text_bytes: usize,
    forbidden_tables: std::collections::BTreeSet<usize>,
    forbidden_callbacks: std::collections::BTreeSet<usize>,
    forbidden_cells: std::collections::BTreeSet<usize>,
    context: SourceProgramContext,
    constructor_observations: constructors::Pending,
    closure_observations: closures::Pending,
    // Only the class/session observer requires positive immutable ownership.
    immutable_capture_tables: Option<std::collections::BTreeSet<usize>>,
    // Live tables reserve their share before late definition graph expansion.
    session_tables: usize,
}
impl Graph<'_> {
    fn check_table_capacity(&self) -> Result<()> {
        if self.tables.len() + self.session_tables >= MAX_TABLES {
            return Err(error("source closure/session combined table count bound"));
        }
        Ok(())
    }
    fn text(&mut self, count: usize) -> Result<()> {
        self.text_bytes = self
            .text_bytes
            .checked_add(count)
            .ok_or_else(|| error("source closure text overflow"))?;
        if self.text_bytes > MAX_TEXT_BYTES {
            return Err(error("source closure text bound"));
        }
        Ok(())
    }
    fn value(&mut self, value: Value, depth: usize) -> Result<SourceValue> {
        self.values += 1;
        if self.values > MAX_VALUES || depth > 64 {
            return Err(error("source closure graph resource bound"));
        }
        Ok(match value {
            Value::Nil => SourceValue::Nil,
            Value::Boolean(value) => SourceValue::Boolean(value),
            Value::Integer(value) => SourceValue::Number(value as f64),
            Value::Number(value) if value.is_nan() => SourceValue::NonFinite(SourceNonFinite::Nan),
            Value::Number(value) if value == f64::INFINITY => {
                SourceValue::NonFinite(SourceNonFinite::PositiveInfinity)
            }
            Value::Number(value) if value == f64::NEG_INFINITY => {
                SourceValue::NonFinite(SourceNonFinite::NegativeInfinity)
            }
            Value::Number(value) => SourceValue::Number(value),
            Value::String(value) => {
                self.text(value.as_bytes().len())?;
                SourceValue::Text(value.to_str()?.to_owned())
            }
            Value::Table(table) => self.table(table, depth)?,
            Value::Function(function) => self.function(function, depth)?,
            _ => return Err(error("source closure capture value is not represented")),
        })
    }
    fn table(&mut self, table: Table, depth: usize) -> Result<SourceValue> {
        let pointer = table.to_pointer() as usize;
        if self.forbidden_tables.contains(&pointer) {
            return Err(error("immutable definition reaches a live session table"));
        }
        if let Some(id) = self.seen_tables.get(&pointer) {
            // Only preflighted projections/classes or already checked plain tables
            // enter this map; explicit raw __index snapshots retain their aliases.
            return Ok(SourceValue::Table(*id));
        }
        plain(&table, "captured table")?;
        self.check_table_capacity()?;
        let id = SourceTableId(self.tables.len() as u32 + 1);
        self.seen_tables.insert(pointer, id);
        self.tables.push(SourceTable::default());
        let (mut indexed, mut fields) = (BTreeMap::new(), BTreeMap::new());
        let entries = self.raw_table_entries(&table)?;
        self.store_iteration_order(&table, id, &entries, true)?;
        for (key, value) in entries {
            match key {
                Value::String(key) => {
                    self.text(key.as_bytes().len())?;
                    fields.insert(key.to_str()?.to_owned(), value);
                }
                Value::Integer(key) => {
                    indexed.insert(key, value);
                }
                Value::Number(key)
                    if key.is_finite()
                        && key.fract() == 0.0
                        && key.abs() <= 9_007_199_254_740_991.0 =>
                {
                    indexed.insert(key as i64, value);
                }
                _ => return Err(error("source closure table key is not represented")),
            }
            if fields.len() + indexed.len() > 50_000 {
                return Err(error("source closure table row bound"));
            }
        }
        let mut out = SourceTable::default();
        for (key, value) in indexed {
            out.indexed.insert(key, self.value(value, depth + 1)?);
        }
        for (key, value) in fields {
            out.fields.insert(key, self.value(value, depth + 1)?);
        }
        self.tables[id.0 as usize - 1] = out;
        Ok(SourceValue::Table(id))
    }
    fn upvalue(&self, function: &Function, index: i32) -> Result<(Option<String>, Value)> {
        Ok(match upvalues::read(&self.observer.lua, function, index)? {
            Some(observed) => {
                if self.forbidden_cells.contains(&observed.identity) {
                    return Err(error(
                        "immutable definition shares a live session capture cell",
                    ));
                }
                if let Value::Table(table) = &observed.value
                    && self
                        .immutable_capture_tables
                        .as_ref()
                        .is_some_and(|tables| !tables.contains(&(table.to_pointer() as usize)))
                {
                    if self
                        .forbidden_tables
                        .contains(&(table.to_pointer() as usize))
                    {
                        return Err(error("shared definition captures a live session table"));
                    }
                    return Err(error(
                        "shared definition capture table is not explicitly classified immutable",
                    ));
                }
                (Some(observed.name), observed.value)
            }
            None => (None, Value::Nil),
        })
    }
    fn lua_source(&self, function: &Function) -> Result<ItemSourceSpan> {
        if function
            .environment()
            .is_none_or(|env| env.to_pointer() != self.observer.globals.to_pointer())
        {
            return Err(error(
                "source closure uses a non-original global environment",
            ));
        }
        let info = function.info();
        let path = info
            .source
            .as_deref()
            .and_then(|source| source.strip_prefix('@'))
            .ok_or_else(|| error("source closure lacks an authenticated file name"))?;
        let path = self
            .source_names
            .get(path)
            .map(String::as_str)
            .unwrap_or(path);
        let text = self
            .sources
            .get(path)
            .ok_or_else(|| error(format!("source closure dependency absent: {path}")))?;
        let line = info
            .line_defined
            .ok_or_else(|| error("source closure missing first line"))?;
        let end_line = info
            .last_line_defined
            .ok_or_else(|| error("source closure missing last line"))?;
        if line == 0 || end_line < line || end_line > text.lines().count() {
            return Err(error("source closure line span is outside source"));
        }
        let body = text
            .split_inclusive('\n')
            .skip(line - 1)
            .take(end_line - line + 1)
            .collect::<String>();
        Ok(ItemSourceSpan {
            path: path.into(),
            line: line.try_into().map_err(error)?,
            end_line: end_line.try_into().map_err(error)?,
            sha256: hash(body.as_bytes()),
        })
    }
    fn function(&mut self, function: Function, depth: usize) -> Result<SourceValue> {
        let pointer = function.to_pointer() as usize;
        if self.forbidden_callbacks.contains(&pointer) {
            return Err(error("immutable definition reaches a live session closure"));
        }
        if let Some(id) = self.seen_callbacks.get(&pointer) {
            return Ok(SourceValue::Callback(*id));
        }
        if self.callbacks.len() >= MAX_CALLBACKS {
            return Err(error("source closure callback count bound"));
        }
        let id = SourceCallbackId(self.callbacks.len() as u32 + 1);
        self.seen_callbacks.insert(pointer, id);
        let info = function.info();
        let kind = if info.what == "C" {
            if let Some((operation, _)) = self
                .observer
                .primitives
                .iter()
                .find(|(_, original)| original.to_pointer() == function.to_pointer())
            {
                self.intrinsics.insert(id, *operation);
                SourceCallbackKind::Builtin {
                    symbol: operation
                        .global_path()
                        .expect("language primitive")
                        .join("."),
                }
            } else if let Some(operation) = self.iterator_intrinsic(&function) {
                self.intrinsics.insert(id, operation);
                SourceCallbackKind::Builtin {
                    symbol: operation.builtin_symbol().expect("iterator primitive"),
                }
            } else {
                let (symbol, _) = self
                    .observer
                    .opaque_primitives
                    .iter()
                    .find(|(_, original)| original.to_pointer() == function.to_pointer())
                    .ok_or_else(|| {
                        error("source closure builtin has no observed primitive identity")
                    })?;
                SourceCallbackKind::Builtin {
                    symbol: symbol.clone(),
                }
            }
        } else {
            SourceCallbackKind::Lua {
                source: self.lua_source(&function)?,
            }
        };
        if let SourceCallbackKind::Lua { source } = &kind {
            self.observe_constructors(id, &function, source)?;
        }
        self.callbacks.push(SourceCallback {
            kind: kind.clone(),
            upvalues: vec![],
            environment: SourceEnvironment::OriginalGlobals,
        });
        let mut upvalues = Vec::new();
        if matches!(kind, SourceCallbackKind::Lua { .. }) {
            for index in 1..=129 {
                let (name, value) = self.upvalue(&function, index)?;
                let Some(name) = name else { break };
                if index > 128 {
                    return Err(error("source closure upvalue bound"));
                }
                self.text(name.len())?;
                let value = self
                    .value(value, depth + 1)
                    .map_err(|e| error(format!("source callback {id:?} capture {name}: {e}")))?;
                upvalues.push(SourceUpvalue { name, value });
            }
        }
        self.callbacks[id.0 as usize - 1].upvalues = upvalues;
        self.capture_iterator_auxiliary(id, depth)?;
        Ok(SourceValue::Callback(id))
    }
}

#[cfg(test)]
mod tests;
