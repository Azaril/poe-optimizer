//! Class projection is prepared before live identity discovery, then captured
//! after live cells/tables are known. This also serves the legacy class observer.
use super::*;
pub(in crate::source_programs::capture) struct PreparedClasses {
    policy: SourceClassConstructionPolicy,
    pub(in crate::source_programs::capture) ids: BTreeMap<usize, SourceClassId>,
    classes: BTreeMap<String, SourceClassId>,
    selected: Vec<BTreeSet<String>>,
    descriptors: Vec<SourceClassDefinition>,
}
pub(in crate::source_programs::capture) struct CapturedClasses {
    pub(in crate::source_programs::capture) definitions: SourceClassDefinitions,
    pub(in crate::source_programs::capture) roots: Vec<SourceProgramRoot>,
    pub(in crate::source_programs::capture) callbacks: BTreeMap<String, SourceCallbackId>,
    pub(in crate::source_programs::capture) classes: BTreeMap<String, SourceClassId>,
}
impl PreparedClasses {
    pub(in crate::source_programs::capture) fn prepare(
        graph: &mut Graph<'_>,
        lua: &Lua,
        request: &SourceClassCaptureRequest,
    ) -> Result<Self> {
        let policy = protocol::extract(graph, &request.allocation)?;
        let common: Table = lua.globals().raw_get("common")?;
        plain(&common, "Common module")?;
        let registry: Table = common.raw_get("classes")?;
        plain(&registry, "Common class registry")?;
        let mut names = Vec::new();
        let mut ids = BTreeMap::new();
        let mut classes = BTreeMap::new();
        let mut selected = Vec::new();
        let mut total_methods = 0usize;
        for selection in &request.classes {
            if selection.methods.len() > 4096 {
                return Err(error("selected class method count bound"));
            }
            plain(&selection.table, "class table")?;
            let name: mlua::LuaString =
                selection.table.raw_get(policy.class_name_field.as_str())?;
            if name.as_bytes().len() > 256 {
                return Err(error("invalid source class name bound"));
            }
            let name = name.to_str()?.to_owned();
            valid_name(&name)?;
            graph.text(name.len())?;
            let actual: Table = registry.raw_get(name.as_str())?;
            if actual.to_pointer() != selection.table.to_pointer() {
                return Err(error(format!(
                    "requested class {name} differs from original Common registry"
                )));
            }
            let id = SourceClassId(names.len() as u32 + 1);
            if classes.insert(name.clone(), id).is_some()
                || ids
                    .insert(selection.table.to_pointer() as usize, id)
                    .is_some()
            {
                return Err(error("duplicate requested class identity"));
            }
            if graph
                .seen_tables
                .contains_key(&(selection.table.to_pointer() as usize))
            {
                return Err(error(
                    "class table cannot also be a generic table projection",
                ));
            }
            graph.check_table_capacity()?;
            let table_id = SourceTableId(graph.tables.len() as u32 + 1);
            graph
                .seen_tables
                .insert(selection.table.to_pointer() as usize, table_id);
            graph.tables.push(SourceTable::default());
            for method in &selection.methods {
                valid_name(method)?;
            }
            let mut methods = selection.methods.clone();
            methods.insert(name.clone());
            for method in &methods {
                valid_name(method)?;
            }
            total_methods = total_methods
                .checked_add(methods.len())
                .ok_or_else(|| error("selected method count overflow"))?;
            if methods.len() > 4096 || total_methods > 32768 {
                return Err(error("selected class method count bound"));
            }
            names.push(name);
            selected.push(methods);
        }
        let mut descriptors = Vec::new();
        for (index, selection) in request.classes.iter().enumerate() {
            let table = &selection.table;
            let parents = parents(table, &policy, &ids)?;
            let super_parents = super_parents(table, &policy, &ids)?;
            descriptors.push(SourceClassDefinition {
                name: names[index].clone(),
                table: graph.seen_tables[&(table.to_pointer() as usize)],
                parents,
                super_parents,
                unsupported_fields: BTreeSet::new(),
                methods: BTreeMap::new(),
                constructor: None,
            });
        }
        // Preserve copied method origin without depending on fixture names or
        // synthetic class inheritance. Include parent entries that explain an
        // actual matching copied callback, including the original before wrapping.
        loop {
            let mut additions = Vec::new();
            for (index, methods) in selected.iter().enumerate() {
                for name in methods {
                    let value: Value = request.classes[index].table.raw_get(name.as_str())?;
                    for parent in &descriptors[index].parents {
                        let parent_index = parent.0 as usize - 1;
                        let candidate: Value =
                            request.classes[parent_index].table.raw_get(name.as_str())?;
                        if !matches!(candidate, Value::Nil) {
                            if copied_callback(
                                graph,
                                &policy,
                                &value,
                                &candidate,
                                name,
                                &names[parent_index],
                            )? && !selected[parent_index].contains(name)
                            {
                                additions.push((parent_index, name.clone()));
                            }
                            break;
                        }
                    }
                }
            }
            if additions.is_empty() {
                break;
            }
            for (index, name) in additions {
                if selected[index].insert(name) {
                    total_methods += 1;
                }
            }
            if total_methods > 32768 {
                return Err(error("inherited method count bound"));
            }
        }
        Ok(Self {
            policy,
            ids,
            classes,
            selected,
            descriptors,
        })
    }
    pub(in crate::source_programs::capture) fn represented_value(
        &self,
        pointer: usize,
        key: &SourceTableKey,
        value: &Value,
    ) -> Option<bool> {
        let index = self.ids.get(&pointer)?.0 as usize - 1;
        Some(match key {
            SourceTableKey::Text(key) => {
                represented_field(key, value, &self.policy, &self.selected[index])
            }
            SourceTableKey::Integer(_) => true,
        })
    }
    pub(in crate::source_programs::capture) fn shared_functions(
        &self,
        graph: &mut Graph<'_>,
        request: &SourceClassCaptureRequest,
    ) -> Result<BTreeMap<usize, Function>> {
        fn visit(
            graph: &mut Graph<'_>,
            function: Function,
            seen: &mut BTreeMap<usize, Function>,
            depth: usize,
        ) -> Result<()> {
            let pointer = function.to_pointer() as usize;
            if seen.contains_key(&pointer) {
                return Ok(());
            }
            graph.projection_row()?;
            if depth > 64 || seen.len() >= MAX_CALLBACKS {
                return Err(error("shared class function graph bound"));
            }
            seen.insert(pointer, function.clone());
            if function.info().what == "C" {
                return Ok(());
            }
            graph.lua_source(&function)?;
            for slot in 1..=129 {
                let Some(capture) = upvalues::read(&graph.observer.lua, &function, slot)? else {
                    break;
                };
                if slot > 128 {
                    return Err(error("shared class function capture count bound"));
                }
                graph.projection_row()?;
                graph.text(capture.name.len())?;
                // Tables follow their explicit definition projections during the
                // actual capture. Never walk omitted fields to classify live code.
                if let Value::Function(function) = capture.value {
                    visit(graph, function, seen, depth + 1)?;
                }
            }
            Ok(())
        }
        let mut functions = BTreeMap::new();
        for (index, selection) in request.classes.iter().enumerate() {
            for name in &self.selected[index] {
                if let Value::Function(function) = selection.table.raw_get(name.as_str())? {
                    visit(graph, function, &mut functions, 0)?;
                }
            }
        }
        for function in request.callbacks.values() {
            visit(graph, function.clone(), &mut functions, 0)?;
        }
        // Closed allocation policy callbacks were captured before class table
        // registration. Retain their actual functions for the same live-cell
        // overlap audit; do not create invented empty callback captures.
        let mut protocol = BTreeMap::new();
        visit(graph, request.allocation.clone(), &mut protocol, 0)?;
        for (pointer, function) in protocol {
            if graph.seen_callbacks.contains_key(&pointer) {
                functions.insert(pointer, function);
            }
        }
        Ok(functions)
    }
    pub(in crate::source_programs::capture) fn capture(
        self,
        graph: &mut Graph<'_>,
        request: &SourceClassCaptureRequest,
    ) -> Result<CapturedClasses> {
        let Self {
            policy,
            ids: _,
            classes,
            selected,
            mut descriptors,
        } = self;
        let mut callbacks = BTreeMap::new();
        for (index, selection) in request.classes.iter().enumerate() {
            let descriptor = &mut descriptors[index];
            let mut fields = BTreeMap::new();
            let mut indexed = BTreeMap::new();
            let mut entries = BTreeMap::new();
            for entry in selection.table.clone().pairs::<Value, Value>() {
                graph.values += 1;
                if graph.values > MAX_VALUES {
                    return Err(error("source class projection aggregate row bound"));
                }
                let (key, value) = entry?;
                match key {
                    Value::String(key) => {
                        graph.text(key.as_bytes().len())?;
                        let key = key.to_str()?.to_owned();
                        if !represented_field(&key, &value, &policy, &selected[index]) {
                            descriptor.unsupported_fields.insert(key);
                            if descriptor.unsupported_fields.len() > 4096 {
                                return Err(error("source class omitted field count bound"));
                            }
                        } else {
                            entries.insert(key, value);
                        }
                    }
                    Value::Integer(key) => {
                        indexed.insert(key, graph.value(value, 0)?);
                    }
                    Value::Number(key)
                        if key.is_finite()
                            && key.fract() == 0.0
                            && key.abs() <= 9_007_199_254_740_991.0 =>
                    {
                        indexed.insert(key as i64, graph.value(value, 0)?);
                    }
                    _ => {
                        return Err(error(
                            "class projection has an unrepresented non-string/noninteger key",
                        ));
                    }
                }
                if entries.len() + indexed.len() > 50_000 {
                    return Err(error("class projection row bound"));
                }
            }
            for name in &selection.methods {
                if !matches!(entries.get(name), Some(Value::Function(_))) {
                    return Err(error(format!(
                        "requested class method {}.{name} is missing or not a function",
                        descriptor.name
                    )));
                }
            }
            for (key, value) in entries {
                let captured = graph.value(value.clone(), 0)?;
                if selected[index].contains(&key) {
                    if let SourceValue::Callback(callback) = captured {
                        if callbacks
                            .insert(format!("{}.{}", descriptor.name, key), callback)
                            .is_some()
                        {
                            return Err(error("duplicate selected callback key"));
                        }
                        descriptor.methods.insert(
                            key.clone(),
                            SourceClassMethod {
                                callback,
                                declared_by: SourceClassId(index as u32 + 1),
                            },
                        );
                        if key == descriptor.name {
                            descriptor.constructor = Some(constructor(graph, &policy, callback)?);
                        }
                    } else if key == descriptor.name
                        && !matches!(captured, SourceValue::Nil | SourceValue::Boolean(false))
                    {
                        return Err(error(
                            "source class constructor is not represented by a callback",
                        ));
                    }
                }
                fields.insert(key, captured);
            }
            graph.tables[descriptor.table.0 as usize - 1] = SourceTable { fields, indexed };
        }
        // Determine copied declarations after every actual method has been captured.
        let mut resolved = BTreeMap::new();
        for (index, methods) in selected.iter().enumerate() {
            for name in methods {
                let owner = declaring(
                    &descriptors,
                    &graph.tables,
                    index,
                    name,
                    &mut BTreeSet::new(),
                )?;
                if let Some(owner) = owner {
                    resolved.insert((index, name.clone()), owner);
                }
            }
        }
        for ((index, name), owner) in resolved {
            descriptors[index]
                .methods
                .get_mut(&name)
                .expect("resolved method")
                .declared_by = owner;
        }
        for (name, function) in &request.callbacks {
            valid_name(name)?;
            graph.text(name.len())?;
            let SourceValue::Callback(id) = graph.value(Value::Function(function.clone()), 0)?
            else {
                unreachable!()
            };
            if callbacks.insert(name.clone(), id).is_some() {
                return Err(error("named root callback collides with selected method"));
            }
        }
        let mut roots = Vec::new();
        for (name, table) in &request.definition_roots {
            valid_name(name)?;
            graph.text(name.len())?;
            let SourceValue::Table(table) = graph.value(Value::Table(table.clone()), 0)? else {
                unreachable!()
            };
            roots.push(SourceProgramRoot {
                name: name.clone(),
                table,
            });
        }
        Ok(CapturedClasses {
            definitions: SourceClassDefinitions {
                schema_version: SOURCE_CLASS_DEFINITIONS_SCHEMA_VERSION,
                source: policy,
                classes: descriptors,
            },
            roots,
            callbacks,
            classes,
        })
    }
}
