//! Explicit raw-table projections and an observed, owner-bound global environment.
use super::*;
use std::collections::BTreeSet;

/// Select raw fields of this actual table. Unselected present fields remain
/// unavailable. Selected nested table values are captured completely unless
/// their actual identity also has an explicit projection.
#[derive(Clone)]
pub struct SourceTableSelection {
    pub table: Table,
    pub fields: BTreeSet<String>,
    pub indexed: BTreeSet<i64>,
    /// Opt into an unrepresented __index on this raw snapshot.
    /// Its missing ordinary reads remain unavailable; no fallback is executed.
    pub allow_index_fallback: bool,
    /// Opt into an unrepresented __call; invoking this table remains unavailable.
    pub allow_call_fallback: bool,
}
#[derive(Clone)]
pub struct SourceEnvironmentSelection {
    /// Must be the actual original global table from this observer's Lua host.
    pub table: Table,
    /// Catalog root name; it does not introduce a source global variable.
    pub root_name: String,
}
#[derive(Clone, Default)]
pub struct SourceCaptureContext {
    /// Capture original Pairs/Next linkage and raw traversal order for complete
    /// plain immutable definitions, plus complete mutable session raw order and
    /// raw length observations. Disabled captures keep prior outputs.
    pub capture_iteration: bool,
    pub projections: Vec<SourceTableSelection>,
    pub environment: Option<SourceEnvironmentSelection>,
    /// Exact Lua debug name (without @) to authenticated inventory path.
    pub source_names: BTreeMap<String, String>,
}
#[derive(Debug, Clone)]
pub struct ObservedSourceContext {
    pub(super) owner: SourceProgramOwner,
    pub(super) callbacks: BTreeMap<String, SourceCallbackId>,
    pub(super) constructor_observations: Option<ObservedSourceConstructors>,
    pub(super) closure_observations: Option<ObservedSourceClosureCreations>,
}
impl ObservedSourceContext {
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
}
pub(super) fn validate_source_aliases(
    sources: &BTreeMap<String, String>,
    aliases: &BTreeMap<String, String>,
) -> Result<()> {
    if aliases.len() > 4096 {
        return Err(error("source alias count bound"));
    }
    for (name, path) in aliases {
        if name.is_empty()
            || name.len() > 4096
            || name.contains('\0')
            || !sources.contains_key(path)
            || sources.contains_key(name) && name != path
        {
            return Err(error("invalid or shadowing explicit source alias"));
        }
    }
    Ok(())
}
impl Graph<'_> {
    pub(super) fn register_projections(&mut self, request: &SourceCaptureContext) -> Result<()> {
        self.observer.verify_iteration(request.capture_iteration)?;
        if request.capture_iteration {
            self.context.iteration = Some(SourceProgramIteration::default());
        }
        if request.projections.len() > 4096 {
            return Err(error("source table projection count bound"));
        }
        if let Some(environment) = &request.environment {
            if environment.table.to_pointer() != self.observer.globals.to_pointer() {
                return Err(error(
                    "source environment is not the observed original globals",
                ));
            }
            if environment.root_name.is_empty()
                || environment.root_name.len() > 256
                || environment.root_name.contains('\0')
            {
                return Err(error("invalid source environment root name"));
            }
        }
        // Register all selected identities before recursively observing any value.
        // This preserves cycles and cross-projection aliases, including _G.
        let mut selected_keys = 0usize;
        for selection in &request.projections {
            selected_keys = selected_keys
                .checked_add(selection.fields.len() + selection.indexed.len())
                .ok_or_else(|| error("source projection selected key count overflow"))?;
            if selected_keys > MAX_VALUES {
                return Err(error("source projection aggregate selected key bound"));
            }
            if selection.fields.len() + selection.indexed.len() > 50_000 {
                return Err(error("source table projection selected key bound"));
            }
            self.check_table_capacity()?;
            for field in &selection.fields {
                if field.len() > 4096 || field.contains('\0') {
                    return Err(error("source table projection key text bound"));
                }
                self.text(field.len())?;
            }
            if selection
                .indexed
                .iter()
                .any(|key| key.unsigned_abs() > 9_007_199_254_740_991)
            {
                return Err(error("source table projection integer key precision"));
            }
            let (index_fallback, call_fallback) = self.projection_fallbacks(selection)?;
            let pointer = selection.table.to_pointer() as usize;
            let id = SourceTableId(self.tables.len() as u32 + 1);
            if self.seen_tables.insert(pointer, id).is_some() {
                return Err(error("duplicate source table projection identity"));
            }
            self.tables.push(SourceTable::default());
            self.context.tables.insert(
                id,
                SourceTableCoverage {
                    inventory: SourceTableInventory::Complete,
                    known_absent: BTreeSet::new(),
                    unavailable: BTreeSet::new(),
                    index_fallback,
                    call_fallback,
                },
            );
        }
        Ok(())
    }
    pub(super) fn projection_fallbacks(
        &mut self,
        selection: &SourceTableSelection,
    ) -> Result<(SourceTableIndexFallback, SourceTableCallFallback)> {
        let Some(metatable) = selection.table.metatable() else {
            return Ok((
                SourceTableIndexFallback::Nil,
                SourceTableCallFallback::NonCallable,
            ));
        };
        if !selection.allow_index_fallback && !selection.allow_call_fallback {
            return Err(error(
                "source projected table metatable requires explicit raw-field opt-in",
            ));
        }
        let mut count = 0usize;
        for entry in metatable.clone().pairs::<Value, Value>() {
            self.projection_row()?;
            count += 1;
            if count > 50_000 {
                return Err(error("source projected metatable row bound"));
            }
            let (key, _) = entry?;
            let key = self.projection_key(key)?;
            if let SourceTableKey::Text(key) = key {
                match key.as_str() {
                    "__index" if selection.allow_index_fallback => {}
                    "__call" if selection.allow_call_fallback => {}
                    "__index" | "__call" => {
                        return Err(error(format!(
                            "source projected metatable operation {key} requires explicit opt-in"
                        )));
                    }
                    _ if key.starts_with("__") => {
                        return Err(error(format!(
                            "source projected metatable operation {key} is not represented"
                        )));
                    }
                    _ => {}
                }
            }
        }
        Ok((
            if matches!(metatable.raw_get::<Value>("__index")?, Value::Nil) {
                SourceTableIndexFallback::Nil
            } else {
                SourceTableIndexFallback::Unavailable
            },
            if matches!(metatable.raw_get::<Value>("__call")?, Value::Nil) {
                SourceTableCallFallback::NonCallable
            } else {
                SourceTableCallFallback::Unavailable
            },
        ))
    }
    pub(super) fn projection_row(&mut self) -> Result<()> {
        self.values += 1;
        if self.values > MAX_VALUES {
            return Err(error("source projection aggregate row bound"));
        }
        Ok(())
    }
    pub(super) fn projection_key(&mut self, key: Value) -> Result<SourceTableKey> {
        match key {
            Value::String(key) => {
                if key.as_bytes().len() > 4096 {
                    return Err(error("source table projection key text bound"));
                }
                self.text(key.as_bytes().len())?;
                let key = key.to_str()?;
                if key.contains('\0') {
                    return Err(error("source table projection key contains NUL"));
                }
                Ok(SourceTableKey::Text(key.to_owned()))
            }
            Value::Integer(key) if key.unsigned_abs() <= 9_007_199_254_740_991 => {
                Ok(SourceTableKey::Integer(key))
            }
            Value::Number(key)
                if key.is_finite()
                    && key.fract() == 0.0
                    && key.abs() <= 9_007_199_254_740_991.0 =>
            {
                Ok(SourceTableKey::Integer(key as i64))
            }
            _ => Err(error(
                "source table projection has an unsupported key domain",
            )),
        }
    }
    pub(super) fn capture_projections(&mut self, request: &SourceCaptureContext) -> Result<()> {
        for selection in &request.projections {
            let id = self.seen_tables[&(selection.table.to_pointer() as usize)];
            let mut selected = BTreeMap::new();
            let mut unavailable = BTreeSet::new();
            // Enumerate the complete raw inventory, even values not selected.
            // Deterministic sorting happens only after each row has been charged.
            let entries = self.raw_table_entries(&selection.table)?;
            for (key, value) in &entries {
                self.projection_row()?;
                if selected.len() + unavailable.len() >= 50_000 {
                    return Err(error("source table projection row bound"));
                }
                let key = self.projection_key(key.clone())?;
                let wanted = match &key {
                    SourceTableKey::Text(key) => selection.fields.contains(key),
                    SourceTableKey::Integer(key) => selection.indexed.contains(key),
                };
                if wanted {
                    selected.insert(key, value.clone());
                } else {
                    unavailable.insert(key);
                }
            }
            self.store_iteration_order(&selection.table, id, &entries, unavailable.is_empty())?;
            let mut out = SourceTable::default();
            for (key, value) in selected {
                let value = self.value(value, 1)?;
                match key {
                    SourceTableKey::Text(key) => {
                        out.fields.insert(key, value);
                    }
                    SourceTableKey::Integer(key) => {
                        out.indexed.insert(key, value);
                    }
                }
            }
            self.tables[id.0 as usize - 1] = out;
            self.context
                .tables
                .get_mut(&id)
                .expect("registered projection")
                .unavailable = unavailable;
        }
        Ok(())
    }
    pub(super) fn capture_environment(
        &mut self,
        request: &SourceCaptureContext,
        roots: &mut Vec<SourceProgramRoot>,
    ) -> Result<()> {
        if let Some(environment) = &request.environment {
            if roots.len() >= 4096 {
                return Err(error("source environment/definition root count bound"));
            }
            if roots.iter().any(|root| root.name == environment.root_name) {
                return Err(error(
                    "source environment root collides with named definition root",
                ));
            }
            self.text(environment.root_name.len())?;
            let SourceValue::Table(table) =
                self.value(Value::Table(environment.table.clone()), 0)?
            else {
                unreachable!()
            };
            roots.push(SourceProgramRoot {
                name: environment.root_name.clone(),
                table,
            });
            self.context.environment = Some(SourceProgramRootId(roots.len() as u32));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
