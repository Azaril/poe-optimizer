//! Production observation of explicitly requested constructed Common classes.
//! Class projections are intentional and enumerate all omitted string fields;
//! ordinary tables are complete unless an explicit capture context selects them.
use super::*;
use std::collections::BTreeSet;
mod prepared;
mod protocol;
pub(super) use prepared::PreparedClasses;

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
    pub(super) constructor_observations: Option<ObservedSourceConstructors>,
    pub(super) closure_observations: Option<ObservedSourceClosureCreations>,
}
impl ObservedSourceClasses {
    pub fn closure_observations(&self) -> Option<&ObservedSourceClosureCreations> {
        self.closure_observations.as_ref()
    }
    pub fn constructor_observations(&self) -> Option<&ObservedSourceConstructors> {
        self.constructor_observations.as_ref()
    }
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
            forbidden_tables: BTreeSet::new(),
            forbidden_callbacks: BTreeSet::new(),
            forbidden_cells: BTreeSet::new(),
            immutable_capture_tables: None,
            session_tables: 0,
            context: SourceProgramContext::default(),
            constructor_observations: constructors::Pending::default(),
            closure_observations: closures::Pending::default(),
        };
        graph.register_projections(&context)?;
        let prepared = PreparedClasses::prepare(&mut graph, lua, &request)?;
        graph.capture_projections(&context)?;
        let captured = prepared.capture(&mut graph, &request)?;
        let mut roots = captured.roots;
        graph.capture_environment(&context, &mut roots)?;
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
            roots,
            intrinsics: graph.intrinsics,
        };
        let owner = if self.closure_reflection.is_some() {
            SourceProgramOwner::new_with_closures(
                definitions,
                Some(captured.definitions),
                Some(graph.context),
                prototypes,
            )
        } else {
            SourceProgramOwner::new_with_context(
                definitions,
                Some(captured.definitions),
                graph.context,
            )
        }
        .map_err(error)?;
        validate_sources(sources, &owner)?;
        self.verify(lua)?;
        self.verify_iteration(context.capture_iteration)?;
        let constructor_observations =
            self.bind_constructors(&owner, graph.constructor_observations);
        let closure_observations = self.bind_closure_creations(&owner, graph.closure_observations);
        Ok(ObservedSourceClasses {
            closure_observations,
            constructor_observations,
            owner,
            callbacks: captured.callbacks,
            classes: captured.classes,
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
