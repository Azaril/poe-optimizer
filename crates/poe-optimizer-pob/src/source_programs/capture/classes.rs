//! Production observation of explicitly requested constructed Common classes.
//! Class projections are intentional and enumerate all omitted string fields;
//! ordinary tables are complete unless an explicit capture context selects them.
use super::*;
use std::collections::BTreeSet;
mod protocol;

#[derive(Clone)]
pub struct SourceClassSelection {
    pub table: Table,
    /// Requested methods; original constructors and inherited origins are added.
    pub methods: BTreeSet<String>,
}
#[derive(Clone)]
pub struct SourceClassCaptureRequest {
    pub classes: Vec<SourceClassSelection>,
    pub callbacks: BTreeMap<String, Function>,
    pub definition_roots: BTreeMap<String, Table>,
    /// Original Common.new, before instrumentation wrappers. Capture never calls it.
    pub allocation: Function,
    /// Exact debug source name (without @) to supplied inventory path. No suffix
    /// guessing, file reads, normalization or fallback lookup is performed.
    pub source_names: BTreeMap<String, String>,
}
#[derive(Debug, Clone)]
pub struct ObservedSourceClasses {
    owner: SourceProgramOwner,
    callbacks: BTreeMap<String, SourceCallbackId>,
    classes: BTreeMap<String, SourceClassId>,
}
impl ObservedSourceClasses {
    pub fn owner(&self) -> &SourceProgramOwner {
        &self.owner
    }
    pub fn callbacks(&self) -> &BTreeMap<String, SourceCallbackId> {
        &self.callbacks
    }
    pub fn class_ids(&self) -> &BTreeMap<String, SourceClassId> {
        &self.classes
    }
}
impl SourceClosureObserver {
    /// Observe source-created classes whose allocation caches the caller has
    /// already warmed when needed. No Lua callback or constructor is executed.
    /// All parent/superclass identities must be among the explicit class inputs.
    pub fn observe_classes(
        &self,
        lua: &Lua,
        sources: &BTreeMap<String, String>,
        source: ItemLoadingSource,
        request: SourceClassCaptureRequest,
    ) -> Result<ObservedSourceClasses> {
        self.observe_classes_with_context(
            lua,
            sources,
            source,
            request,
            SourceCaptureContext::default(),
        )
    }
    /// Observe classes plus explicit raw-table/environment projections. The closed
    /// Common protocol still requires its original global primitive bindings.
    pub fn observe_classes_with_context(
        &self,
        lua: &Lua,
        sources: &BTreeMap<String, String>,
        source: ItemLoadingSource,
        mut request: SourceClassCaptureRequest,
        context: SourceCaptureContext,
    ) -> Result<ObservedSourceClasses> {
        self.verify(lua)?;
        if request.classes.len() > 512
            || request.callbacks.len() > 4096
            || request.definition_roots.len() > 4096
            || request.source_names.len() > 4096
        {
            return Err(error("constructed source capture request count bound"));
        }
        context::validate_source_aliases(sources, &context.source_names)?;
        for (name, path) in &context.source_names {
            if request
                .source_names
                .get(name)
                .is_some_and(|existing| existing != path)
            {
                return Err(error("conflicting explicit source aliases"));
            }
            if !request.source_names.contains_key(name) && request.source_names.len() >= 4096 {
                return Err(error("source alias count bound"));
            }
            request.source_names.insert(name.clone(), path.clone());
        }
        context::validate_source_aliases(sources, &request.source_names)?;
        // Preflight bytes and declared construction spans before graph allocation.
        let empty = SourceProgramOwner::new(SourceProgramDefinitions {
            schema_version: SOURCE_PROGRAM_DEFINITIONS_SCHEMA_VERSION,
            source: source.clone(),
            tables: vec![],
            callbacks: vec![],
            roots: vec![],
            intrinsics: BTreeMap::new(),
        })
        .map_err(error)?;
        validate_sources(sources, &empty)?;
        let mut graph = Graph {
            observer: self,
            sources,
            source_names: &request.source_names,
            tables: vec![],
            callbacks: vec![],
            seen_tables: BTreeMap::new(),
            seen_callbacks: BTreeMap::new(),
            intrinsics: BTreeMap::new(),
            values: 0,
            text_bytes: 0,
            context: SourceProgramContext::default(),
        };
        graph.register_projections(&context)?;
        let policy = protocol::extract(&mut graph, &request.allocation)?;
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
            let table_id = SourceTableId(graph.tables.len() as u32 + 1);
            graph
                .seen_tables
                .insert(selection.table.to_pointer() as usize, table_id);
            graph.tables.push(SourceTable::default());
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
        graph.capture_projections(&context)?;
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
                                &graph,
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
                            descriptor.constructor = Some(constructor(&graph, &policy, callback)?);
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
        for (name, function) in request.callbacks {
            valid_name(&name)?;
            graph.text(name.len())?;
            let SourceValue::Callback(id) = graph.value(Value::Function(function), 0)? else {
                unreachable!()
            };
            if callbacks.insert(name, id).is_some() {
                return Err(error("named root callback collides with selected method"));
            }
        }
        let mut roots = Vec::new();
        for (name, table) in request.definition_roots {
            valid_name(&name)?;
            graph.text(name.len())?;
            let SourceValue::Table(table) = graph.value(Value::Table(table), 0)? else {
                unreachable!()
            };
            roots.push(SourceProgramRoot { name, table });
        }
        graph.capture_environment(&context, &mut roots)?;
        let definitions = SourceProgramDefinitions {
            schema_version: SOURCE_PROGRAM_DEFINITIONS_SCHEMA_VERSION,
            source,
            tables: graph.tables,
            callbacks: graph.callbacks,
            roots,
            intrinsics: graph.intrinsics,
        };
        let owner = SourceProgramOwner::new_with_context(
            definitions,
            Some(SourceClassDefinitions {
                schema_version: SOURCE_CLASS_DEFINITIONS_SCHEMA_VERSION,
                source: policy,
                classes: descriptors,
            }),
            graph.context,
        )
        .map_err(error)?;
        validate_sources(sources, &owner)?;
        self.verify(lua)?;
        Ok(ObservedSourceClasses {
            owner,
            callbacks,
            classes,
        })
    }
}
fn represented_field(
    key: &str,
    value: &Value,
    policy: &SourceClassConstructionPolicy,
    selected: &BTreeSet<String>,
) -> bool {
    key != policy.super_parents_field
        && (selected.contains(key)
            || [
                "__index",
                policy.parent_classes_field.as_str(),
                policy.unconstructed_meta_field.as_str(),
            ]
            .contains(&key)
            || matches!(
                value,
                Value::Nil
                    | Value::Boolean(_)
                    | Value::Number(_)
                    | Value::Integer(_)
                    | Value::String(_)
            ))
}
fn valid_name(value: &str) -> Result<()> {
    if value.is_empty() || value.len() > 256 || value.contains('\0') {
        Err(error("invalid source capture name"))
    } else {
        Ok(())
    }
}
fn class_id(value: &Table, ids: &BTreeMap<usize, SourceClassId>) -> Result<SourceClassId> {
    ids.get(&(value.to_pointer() as usize))
        .copied()
        .ok_or_else(|| error("source class dependency was not explicitly requested"))
}
fn parents(
    table: &Table,
    policy: &SourceClassConstructionPolicy,
    ids: &BTreeMap<usize, SourceClassId>,
) -> Result<Vec<SourceClassId>> {
    let value: Value = table.raw_get(policy.parent_classes_field.as_str())?;
    let Value::Table(parent_table) = value else {
        return if matches!(value, Value::Nil) {
            Ok(vec![])
        } else {
            Err(error("class parent metadata is not a table"))
        };
    };
    plain(&parent_table, "parent list")?;
    let mut parents = BTreeMap::new();
    for pair in parent_table.pairs::<Value, Table>() {
        let (key, parent) = pair?;
        let key = match key {
            Value::Integer(key) => key,
            Value::Number(key)
                if key.is_finite() && key.fract() == 0.0 && (1.0..=64.0).contains(&key) =>
            {
                key as i64
            }
            _ => return Err(error("class parent list has unrepresented keys")),
        };
        if !(1..=64).contains(&key) {
            return Err(error("class parent count bound"));
        }
        parents.insert(key, class_id(&parent, ids)?);
    }
    if parents.keys().copied().ne(1..=parents.len() as i64) {
        return Err(error("class parent list has holes"));
    }
    Ok(parents.into_values().collect())
}
fn super_parents(
    table: &Table,
    policy: &SourceClassConstructionPolicy,
    ids: &BTreeMap<usize, SourceClassId>,
) -> Result<Option<Vec<SourceClassId>>> {
    match table.raw_get::<Value>(policy.super_parents_field.as_str())? {
        Value::Nil => Ok(None),
        Value::Table(set) => {
            plain(&set, "superclass set")?;
            let mut parents = Vec::new();
            for pair in set.pairs::<Table, Value>() {
                let (parent, _) = pair?;
                if parents.len() >= 512 {
                    return Err(error("superclass count bound"));
                }
                parents.push(class_id(&parent, ids)?);
            }
            Ok(Some(parents))
        }
        _ => Err(error("class superclass metadata is not a table")),
    }
}
fn wrapper_span(
    graph: &Graph<'_>,
    policy: &SourceClassConstructionPolicy,
    function: &Function,
) -> Result<bool> {
    let span = protocol::span(graph, function)?;
    Ok(span.path == policy.wrap_constructor.path
        && span.line >= policy.wrap_constructor.line
        && span.end_line <= policy.wrap_constructor.end_line)
}
fn copied_callback(
    graph: &Graph<'_>,
    policy: &SourceClassConstructionPolicy,
    child: &Value,
    parent: &Value,
    name: &str,
    parent_name: &str,
) -> Result<bool> {
    let (Value::Function(child), Value::Function(parent)) = (child, parent) else {
        return Ok(false);
    };
    if child.to_pointer() == parent.to_pointer() {
        return Ok(true);
    }
    if name == parent_name && wrapper_span(graph, policy, parent)? {
        let captures = protocol::upvalues(graph, parent)?;
        return Ok(
            matches!(captures.get("originalFunc"),Some((_,Value::Function(original))) if original.to_pointer()==child.to_pointer()),
        );
    }
    Ok(false)
}
fn constructor(
    graph: &Graph<'_>,
    policy: &SourceClassConstructionPolicy,
    callback: SourceCallbackId,
) -> Result<SourceClassConstructor> {
    let closure = &graph.callbacks[callback.0 as usize - 1];
    let SourceCallbackKind::Lua { source } = &closure.kind else {
        return Err(error("class constructor is not a source Lua body"));
    };
    if source.path == policy.wrap_constructor.path
        && source.line >= policy.wrap_constructor.line
        && source.end_line <= policy.wrap_constructor.end_line
    {
        let slot = |name: &str| -> Result<u16> {
            closure
                .upvalues
                .iter()
                .position(|value| value.name == name)
                .map(|slot| slot as u16)
                .ok_or_else(|| error(format!("constructor wrapper capture {name} absent")))
        };
        let original_upvalue = slot("originalFunc")?;
        let SourceValue::Callback(original) = closure.upvalues[original_upvalue as usize].value
        else {
            return Err(error("constructor original capture is not a callback"));
        };
        Ok(SourceClassConstructor {
            callback: original,
            wrapper: Some(SourceClassConstructorWrapper {
                callback,
                original_upvalue,
                class_upvalue: slot("class")?,
                class_name_upvalue: slot("className")?,
                pairs_upvalue: slot("pairs")?,
            }),
        })
    } else {
        Ok(SourceClassConstructor {
            callback,
            wrapper: None,
        })
    }
}
fn declaring(
    classes: &[SourceClassDefinition],
    tables: &[SourceTable],
    index: usize,
    name: &str,
    visited: &mut BTreeSet<usize>,
) -> Result<Option<SourceClassId>> {
    if !visited.insert(index) || visited.len() > 64 {
        return Err(error("cyclic or overly deep source method ancestry"));
    }
    let class = &classes[index];
    let Some(method) = class.methods.get(name) else {
        visited.remove(&index);
        return Ok(None);
    };
    let mut owner = SourceClassId(index as u32 + 1);
    for parent in &class.parents {
        let parent_index = parent.0 as usize - 1;
        let parent_class = &classes[parent_index];
        if let Some(parent_method) = parent_class.methods.get(name) {
            if parent_method.callback == method.callback {
                owner = declaring(classes, tables, parent_index, name, visited)?
                    .expect("parent method");
            } else if parent_class.name == name
                && parent_class
                    .constructor
                    .as_ref()
                    .is_some_and(|constructor| {
                        constructor.callback == method.callback && constructor.wrapper.is_some()
                    })
            {
                owner = *parent;
            }
            break;
        } else if parent_class.unsupported_fields.contains(name)
            || tables[parent_class.table.0 as usize - 1]
                .fields
                .contains_key(name)
        {
            break;
        }
    }
    visited.remove(&index);
    Ok(Some(owner))
}
#[cfg(test)]
mod tests;
