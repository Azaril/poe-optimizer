//! Source authentication for existing instances; no construction is replayed.
use super::*;
use crate::source_programs::capture::classes::PreparedClasses;
mod ownership;
pub(super) use ownership::discover as immutable_tables;
#[derive(Clone)]
pub(super) struct Instance {
    pub(super) table: Table,
    pub(super) class: SourceClassId,
    pub(super) call: SourceTableCallFallback,
}
pub(super) fn merge_request(
    request: &mut SourceSessionCaptureRequest,
    classes: &mut SourceClassCaptureRequest,
) -> Result<()> {
    if classes.classes.len() > 512
        || classes.callbacks.len() > 4096
        || classes.definition_roots.len() > 4096
        || classes.source_names.len() > 4096
    {
        return Err(error("session class request count bound"));
    }
    for name in classes
        .callbacks
        .keys()
        .chain(classes.definition_roots.keys())
    {
        valid_name(name)?;
    }
    for (name, path) in &classes.source_names {
        if request
            .definitions
            .source_names
            .get(name)
            .is_some_and(|value| value != path)
        {
            return Err(error("conflicting class/session source aliases"));
        }
        request
            .definitions
            .source_names
            .insert(name.clone(), path.clone());
    }
    for (name, table) in std::mem::take(&mut classes.definition_roots) {
        if request.definition_roots.insert(name, table).is_some() {
            return Err(error("duplicate class/session definition root"));
        }
    }
    for (name, function) in &classes.callbacks {
        if request
            .state_roots
            .insert(name.clone(), Value::Function(function.clone()))
            .is_some()
        {
            return Err(error("duplicate class/session shared callback root"));
        }
    }
    Ok(())
}
pub(super) fn instances(
    graph: &mut Graph<'_>,
    classes: &PreparedClasses,
    tables: Vec<Table>,
) -> Result<BTreeMap<usize, Instance>> {
    if tables.len() > 4096 {
        return Err(error("session class instance count bound"));
    }
    let mut out = BTreeMap::new();
    let mut checked = BTreeMap::new();
    for table in tables {
        graph.projection_row()?;
        let metatable = table
            .metatable()
            .ok_or_else(|| error("class-bound session table has no actual metatable"))?;
        let pointer = metatable.to_pointer() as usize;
        let class = *classes
            .ids
            .get(&pointer)
            .ok_or_else(|| error("instance metatable is not a selected original class table"))?;
        let call = if let Some(call) = checked.get(&pointer) {
            *call
        } else {
            if !matches!(metatable.raw_get::<Value>("__index")?,Value::Table(ref value) if value.to_pointer()==metatable.to_pointer())
            {
                return Err(error("actual class metatable is not a captured self index"));
            }
            for (index, row) in metatable.pairs::<Value, Value>().enumerate() {
                graph.projection_row()?;
                if index >= 50_000 {
                    return Err(error("instance class metatable row bound"));
                }
                let (key, _) = row?;
                if let SourceTableKey::Text(key) = graph.projection_key(key)?
                    && key.starts_with("__")
                    && key != "__index"
                    && key != "__call"
                {
                    return Err(error(format!(
                        "instance class metamethod {key} is not represented"
                    )));
                }
            }
            let call = if matches!(metatable.raw_get::<Value>("__call")?, Value::Nil) {
                SourceTableCallFallback::NonCallable
            } else {
                SourceTableCallFallback::Unavailable
            };
            checked.insert(pointer, call);
            call
        };
        let pointer = table.to_pointer() as usize;
        if out
            .insert(pointer, Instance { table, class, call })
            .is_some()
        {
            return Err(error("duplicate observed class instance identity"));
        }
    }
    Ok(out)
}
pub(super) fn audit_shared(graph: &Graph<'_>, functions: &BTreeMap<usize, Function>) -> Result<()> {
    for function in functions.values() {
        if function.info().what == "C" {
            continue;
        }
        for slot in 1..=129 {
            let (name, value) = graph.upvalue(function, slot)?;
            if name.is_none() {
                break;
            }
            if slot > 128 {
                return Err(error("shared class capture count bound"));
            }
            match value {
                Value::Table(table)
                    if graph
                        .forbidden_tables
                        .contains(&(table.to_pointer() as usize)) =>
                {
                    return Err(error("shared class function captures a live session table"));
                }
                Value::Function(function)
                    if graph
                        .forbidden_callbacks
                        .contains(&(function.to_pointer() as usize)) =>
                {
                    return Err(error(
                        "shared class function captures a live session closure",
                    ));
                }
                _ => {}
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
