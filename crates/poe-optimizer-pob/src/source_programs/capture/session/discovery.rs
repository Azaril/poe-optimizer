//! Bounded identity discovery precedes immutable-definition capture.
use super::*;
struct LiveTable {
    table: Table,
    filled: bool,
    entries: Vec<(SourceTableKey, Value)>,
    coverage: SourceTableCoverage,
}
struct LiveClosure {
    function: Function,
    source: ItemSourceSpan,
    names: Vec<String>,
    captures: Vec<SourceSessionCellId>,
}
pub(super) struct CapturedLive {
    tables: Vec<LiveTable>,
    table_ids: BTreeMap<usize, SourceSessionTableId>,
    closures: Vec<LiveClosure>,
    closure_ids: BTreeMap<usize, SourceSessionClosureId>,
    cells: Vec<Value>,
    cell_identities: BTreeSet<usize>,
    class_bindings: BTreeMap<SourceSessionTableId, SourceClassId>,
}
pub(super) struct LiveGraph<'a, 'b> {
    definitions: &'b mut Graph<'a>,
    immutable: &'b BTreeSet<usize>,
    selections: BTreeMap<usize, &'b SourceTableSelection>,
    shared_functions: &'b BTreeMap<usize, Function>,
    instance_classes: &'b BTreeMap<usize, classes::Instance>,
    cell_ids: BTreeMap<usize, SourceSessionCellId>,
    captured: CapturedLive,
}
pub(super) struct Converted {
    pub(super) state: SourceSessionValueGraph,
    pub(super) coverage: SourceSessionCoverage,
    pub(super) cells: Vec<SourceSessionValue>,
    pub(super) closures: Vec<(SourceCallbackId, Vec<SourceSessionCellId>)>,
    pub(super) prototypes: SourceClosurePrototypes,
    pub(super) class_bindings: BTreeMap<SourceSessionTableId, SourceClassId>,
}
impl<'a, 'b> LiveGraph<'a, 'b> {
    pub(super) fn new(
        definitions: &'b mut Graph<'a>,
        immutable: &'b BTreeSet<usize>,
        projections: &'b [SourceTableSelection],
        shared_functions: &'b BTreeMap<usize, Function>,
        instance_classes: &'b BTreeMap<usize, classes::Instance>,
    ) -> Result<Self> {
        let mut result = Self {
            definitions,
            immutable,
            shared_functions,
            instance_classes,
            selections: BTreeMap::new(),
            cell_ids: BTreeMap::new(),
            captured: CapturedLive {
                tables: vec![],
                table_ids: BTreeMap::new(),
                closures: vec![],
                closure_ids: BTreeMap::new(),
                cells: vec![],
                cell_identities: BTreeSet::new(),
                class_bindings: BTreeMap::new(),
            },
        };
        let mut selected = 0usize;
        for projection in projections {
            let pointer = projection.table.to_pointer() as usize;
            if immutable.contains(&pointer) {
                return Err(error("state projection is also classified immutable"));
            }
            selected = selected
                .checked_add(projection.fields.len() + projection.indexed.len())
                .ok_or_else(|| error("session projection selected key overflow"))?;
            if selected > MAX_VALUES || projection.fields.len() + projection.indexed.len() > 50_000
            {
                return Err(error("session projection selected key bound"));
            }
            for key in &projection.fields {
                if key.len() > 4096 || key.contains('\0') {
                    return Err(error("session projection key text bound"));
                }
                result.definitions.text(key.len())?;
            }
            if projection
                .indexed
                .iter()
                .any(|key| key.unsigned_abs() > 9_007_199_254_740_991)
            {
                return Err(error("session projection integer key precision"));
            }
            if result.selections.insert(pointer, projection).is_some() {
                return Err(error("duplicate session projection identity"));
            }
            result.register_table(projection.table.clone())?;
        }
        for instance in instance_classes.values() {
            if immutable.contains(&(instance.table.to_pointer() as usize)) {
                return Err(error("class instance is also classified immutable"));
            }
            result.register_table(instance.table.clone())?;
        }
        Ok(result)
    }
    fn register_table(&mut self, table: Table) -> Result<SourceSessionTableId> {
        let pointer = table.to_pointer() as usize;
        if let Some(id) = self.captured.table_ids.get(&pointer) {
            return Ok(*id);
        }
        if self.captured.tables.len() + self.definitions.tables.len() >= MAX_TABLES {
            return Err(error("session observation table count bound"));
        }
        let plain = SourceTableSelection {
            table: table.clone(),
            fields: BTreeSet::new(),
            indexed: BTreeSet::new(),
            allow_index_fallback: false,
            allow_call_fallback: false,
        };
        let selection = self.selections.get(&pointer).copied().unwrap_or(&plain);
        let instance = self.instance_classes.get(&pointer);
        let (index_fallback, call_fallback) = if let Some(instance) = instance {
            (SourceTableIndexFallback::ClassResolved, instance.call)
        } else {
            self.definitions.projection_fallbacks(selection)?
        };
        let id = SourceSessionTableId(self.captured.tables.len() as u32 + 1);
        self.captured.table_ids.insert(pointer, id);
        if let Some(instance) = instance {
            self.captured.class_bindings.insert(id, instance.class);
        }
        self.definitions.session_tables = self.captured.tables.len() + 1;
        self.captured.tables.push(LiveTable {
            table,
            filled: false,
            entries: vec![],
            coverage: SourceTableCoverage {
                inventory: SourceTableInventory::Complete,
                known_absent: BTreeSet::new(),
                unavailable: BTreeSet::new(),
                index_fallback,
                call_fallback,
            },
        });
        Ok(id)
    }
    pub(super) fn discover(&mut self, value: Value, depth: usize) -> Result<()> {
        self.definitions.projection_row()?;
        if depth > 64 {
            return Err(error("session observation recursion bound"));
        }
        match value {
            Value::Nil | Value::Boolean(_) | Value::Number(_) | Value::Integer(_) => Ok(()),
            Value::String(value) => self.definitions.text(value.as_bytes().len()),
            Value::Table(table) => {
                let pointer = table.to_pointer() as usize;
                if self.immutable.contains(&pointer) {
                    return Ok(());
                }
                let id = self.register_table(table.clone())?;
                let index = id.0 as usize - 1;
                if self.captured.tables[index].filled {
                    return Ok(());
                }
                self.captured.tables[index].filled = true;
                let selection = self.selections.get(&pointer).copied();
                let mut entries = BTreeMap::new();
                let mut unavailable = BTreeSet::new();
                for entry in table.pairs::<Value, Value>() {
                    self.definitions.projection_row()?;
                    if entries.len() + unavailable.len() >= 50_000 {
                        return Err(error("session table raw row bound"));
                    }
                    let (key, value) = entry?;
                    let key = self.definitions.projection_key(key)?;
                    let selected = selection.is_none_or(|selection| match &key {
                        SourceTableKey::Text(key) => selection.fields.contains(key),
                        SourceTableKey::Integer(key) => selection.indexed.contains(key),
                    });
                    if selected {
                        entries.insert(key, value);
                    } else {
                        unavailable.insert(key);
                    }
                }
                for value in entries.values() {
                    self.discover(value.clone(), depth + 1)?;
                }
                self.captured.tables[index].entries = entries.into_iter().collect();
                self.captured.tables[index].coverage.unavailable = unavailable;
                Ok(())
            }
            Value::Function(function) => {
                if self
                    .shared_functions
                    .contains_key(&(function.to_pointer() as usize))
                {
                    return Ok(());
                }
                if function.info().what == "C" {
                    self.definitions.value(Value::Function(function), depth)?;
                    return Ok(());
                }
                let pointer = function.to_pointer() as usize;
                if self.captured.closure_ids.contains_key(&pointer) {
                    return Ok(());
                }
                if self.captured.closures.len() >= MAX_CALLBACKS {
                    return Err(error("session closure count bound"));
                }
                let source = self.definitions.lua_source(&function)?;
                let id = SourceSessionClosureId(self.captured.closures.len() as u32 + 1);
                self.captured.closure_ids.insert(pointer, id);
                self.captured.closures.push(LiveClosure {
                    function: function.clone(),
                    source,
                    names: vec![],
                    captures: vec![],
                });
                let mut names = Vec::new();
                let mut captures = Vec::new();
                for slot in 1..=129 {
                    let Some(observed) =
                        upvalues::read(&self.definitions.observer.lua, &function, slot)?
                    else {
                        break;
                    };
                    if slot > 128 {
                        return Err(error("session closure capture count bound"));
                    }
                    self.definitions.text(observed.name.len())?;
                    self.definitions.projection_row()?;
                    names.push(observed.name);
                    let cell = if let Some(id) = self.cell_ids.get(&observed.identity) {
                        if !same_value(&self.captured.cells[id.0 as usize - 1], &observed.value) {
                            return Err(error("shared source cell changed during observation"));
                        }
                        *id
                    } else {
                        if self.captured.cells.len() >= MAX_VALUES {
                            return Err(error("session capture cell count bound"));
                        }
                        let id = SourceSessionCellId(self.captured.cells.len() as u32 + 1);
                        self.cell_ids.insert(observed.identity, id);
                        self.captured.cell_identities.insert(observed.identity);
                        self.captured.cells.push(observed.value.clone());
                        self.discover(observed.value, depth + 1)?;
                        id
                    };
                    captures.push(cell);
                }
                self.captured.closures[id.0 as usize - 1].names = names;
                self.captured.closures[id.0 as usize - 1].captures = captures;
                Ok(())
            }
            _ => Err(error("session observation value is not represented")),
        }
    }
    pub(super) fn finish_projections(&mut self) -> Result<()> {
        let pending = self
            .captured
            .tables
            .iter()
            .filter(|table| !table.filled)
            .map(|table| table.table.clone())
            .collect::<Vec<_>>();
        for table in pending {
            self.discover(Value::Table(table), 0)?;
        }
        Ok(())
    }
    pub(super) fn finish(self) -> CapturedLive {
        self.captured
    }
}
impl CapturedLive {
    pub(super) fn cell_identities(&self) -> BTreeSet<usize> {
        self.cell_identities.clone()
    }
    pub(super) fn table_pointers(&self) -> BTreeSet<usize> {
        self.table_ids.keys().copied().collect()
    }
    pub(super) fn closure_pointers(&self) -> BTreeSet<usize> {
        self.closure_ids.keys().copied().collect()
    }
    pub(super) fn convert(
        &self,
        definitions: &mut Graph<'_>,
        roots: Vec<Value>,
    ) -> Result<Converted> {
        let mut prototypes = SourceClosurePrototypes {
            schema_version: SOURCE_CLOSURE_PROTOTYPES_SCHEMA_VERSION,
            prototypes: vec![],
        };
        let mut interned = BTreeMap::new();
        let mut closures = Vec::new();
        for closure in &self.closures {
            let source = &closure.source;
            let key = (
                source.path.clone(),
                source.line,
                source.end_line,
                source.sha256.clone(),
                closure.names.clone(),
            );
            let callback = if let Some(id) = interned.get(&key) {
                *id
            } else {
                if definitions.callbacks.len() >= MAX_CALLBACKS {
                    return Err(error("shared prototype callback bound"));
                }
                let id = SourceCallbackId(definitions.callbacks.len() as u32 + 1);
                definitions.callbacks.push(SourceCallback {
                    kind: SourceCallbackKind::Lua {
                        source: source.clone(),
                    },
                    upvalues: closure
                        .names
                        .iter()
                        .map(|name| SourceUpvalue {
                            name: name.clone(),
                            value: SourceValue::LiveCapture {},
                        })
                        .collect(),
                    environment: SourceEnvironment::OriginalGlobals,
                });
                prototypes
                    .prototypes
                    .push(SourceClosurePrototype { callback: id });
                interned.insert(key, id);
                id
            };
            // Retaining the actual Function keeps its observed cells rooted until
            // conversion ends; it never escapes in the pure-Rust result.
            let _keep_alive = &closure.function;
            closures.push((callback, closure.captures.clone()));
        }
        let values = roots
            .iter()
            .map(|value| self.value(definitions, value))
            .collect::<Result<Vec<_>>>()?;
        let mut tables = Vec::new();
        let mut coverage = BTreeMap::new();
        for (index, table) in self.tables.iter().enumerate() {
            let mut entries = Vec::new();
            for (key, value) in &table.entries {
                let key = match key {
                    SourceTableKey::Text(key) => SourceSessionValue::Bytes(key.as_bytes().to_vec()),
                    SourceTableKey::Integer(key) => SourceSessionValue::Number(*key as f64),
                };
                entries.push((key, self.value(definitions, value)?));
            }
            tables.push(SourceSessionTable { entries });
            coverage.insert(
                SourceSessionTableId(index as u32 + 1),
                table.coverage.clone(),
            );
        }
        let cells = self
            .cells
            .iter()
            .map(|value| self.value(definitions, value))
            .collect::<Result<Vec<_>>>()?;
        Ok(Converted {
            state: SourceSessionValueGraph { values, tables },
            coverage,
            cells,
            closures,
            prototypes,
            class_bindings: self.class_bindings.clone(),
        })
    }
    fn value(&self, definitions: &mut Graph<'_>, value: &Value) -> Result<SourceSessionValue> {
        use SourceSessionValue as Out;
        definitions.projection_row()?;
        Ok(match value {
            Value::Nil => Out::Nil,
            Value::Boolean(value) => Out::Boolean(*value),
            Value::Integer(value) => Out::Number(*value as f64),
            Value::Number(value) => Out::Number(*value),
            Value::String(value) => {
                definitions.text(value.as_bytes().len())?;
                Out::Bytes(value.as_bytes().to_vec())
            }
            Value::Table(table) => {
                let pointer = table.to_pointer() as usize;
                if let Some(id) = self.table_ids.get(&pointer) {
                    Out::Table(*id)
                } else {
                    Out::DefinitionTable(*definitions.seen_tables.get(&pointer).ok_or_else(
                        || error("session table lacks its explicitly observed definition"),
                    )?)
                }
            }
            Value::Function(function) => {
                let pointer = function.to_pointer() as usize;
                if let Some(id) = self.closure_ids.get(&pointer) {
                    Out::Closure(*id)
                } else {
                    Out::Callback(*definitions.seen_callbacks.get(&pointer).ok_or_else(|| {
                        error("session primitive lacks its observed callback identity")
                    })?)
                }
            }
            _ => return Err(error("session conversion value is not represented")),
        })
    }
}
fn same_value(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Nil, Value::Nil) => true,
        (Value::Boolean(a), Value::Boolean(b)) => a == b,
        (Value::Integer(a), Value::Integer(b)) => a == b,
        (Value::Number(a), Value::Number(b)) => a.to_bits() == b.to_bits(),
        (Value::String(a), Value::String(b)) => a.as_bytes() == b.as_bytes(),
        (Value::Table(a), Value::Table(b)) => a.to_pointer() == b.to_pointer(),
        (Value::Function(a), Value::Function(b)) => a.to_pointer() == b.to_pointer(),
        _ => false,
    }
}
