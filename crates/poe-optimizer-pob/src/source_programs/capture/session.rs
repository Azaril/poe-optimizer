//! One authenticated construction/state observation, separated from shared code/data.
use super::*;
use std::collections::BTreeSet;
mod classes;
mod discovery;
use discovery::LiveGraph;

#[derive(Clone, Default)]
pub struct SourceSessionCaptureRequest {
    /// Named actual Lua closures (including zero-capture methods), never recipes.
    pub callbacks: BTreeMap<String, Function>,
    pub state_roots: BTreeMap<String, Value>,
    pub definition_roots: BTreeMap<String, Table>,
    pub definitions: SourceCaptureContext,
    pub state_projections: Vec<SourceTableSelection>,
}
#[derive(Debug, Clone)]
pub struct ObservedSourceSession {
    input: SourceSessionInput,
    roots: BTreeMap<String, usize>,
}
impl ObservedSourceSession {
    pub fn owner(&self) -> &SourceProgramOwner {
        &self.input.owner
    }
    pub fn input(&self) -> &SourceSessionInput {
        &self.input
    }
    /// Zero-based positions in input.state.values and the resulting session roots.
    /// These are not raw callback, table, closure or cell IDs.
    pub fn roots(&self) -> &BTreeMap<String, usize> {
        &self.roots
    }
    pub fn root_index(&self, name: &str) -> Option<usize> {
        self.roots.get(name).copied()
    }
    pub fn into_parts(self) -> (SourceSessionInput, BTreeMap<String, usize>) {
        (self.input, self.roots)
    }
}
impl SourceClosureObserver {
    /// Capture the actual paused source state and its original closures together.
    /// The caller authenticates source execution and this lifecycle occurrence.
    /// No caller can supply a replacement capture, cell, table ID or prototype.
    pub fn observe_session(
        &self,
        lua: &Lua,
        sources: &BTreeMap<String, String>,
        source: ItemLoadingSource,
        request: SourceSessionCaptureRequest,
    ) -> Result<ObservedSourceSession> {
        self.observe_session_inner(lua, sources, source, request, None, Vec::new())
    }
    /// Observe selected original class tables and their existing instances in the
    /// same code/data/session graph. Class membership comes from actual metatable
    /// identity, never a supplied class name. This does not execute constructors.
    /// Named session callbacks must not also be selected shared class functions;
    /// put a shared method alias in state_roots instead. Shared function table
    /// captures must belong to explicit definition roots/projections/class tables
    /// or their represented table subgraphs; hidden state captures are rejected.
    #[allow(clippy::too_many_arguments)]
    pub fn observe_session_with_classes(
        &self,
        lua: &Lua,
        sources: &BTreeMap<String, String>,
        source: ItemLoadingSource,
        request: SourceSessionCaptureRequest,
        class_request: SourceClassCaptureRequest,
        instances: Vec<Table>,
    ) -> Result<ObservedSourceSession> {
        self.observe_session_inner(
            lua,
            sources,
            source,
            request,
            Some(class_request),
            instances,
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn observe_session_inner(
        &self,
        lua: &Lua,
        sources: &BTreeMap<String, String>,
        source: ItemLoadingSource,
        mut request: SourceSessionCaptureRequest,
        mut class_request: Option<SourceClassCaptureRequest>,
        instances: Vec<Table>,
    ) -> Result<ObservedSourceSession> {
        if instances.len() > 4096 {
            return Err(error("session class instance count bound"));
        }
        if let Some(classes) = &mut class_request {
            self.verify(lua)?;
            context::validate_source_aliases(sources, &classes.source_names)?;
            context::validate_source_aliases(sources, &request.definitions.source_names)?;
            classes::merge_request(&mut request, classes)?;
        } else {
            self.verify_capture_context(lua, request.definitions.environment.is_some())?;
        }
        if request.callbacks.len() + request.state_roots.len() > 4096
            || request.definition_roots.len() > 4096
            || request.state_projections.len() > 4096
        {
            return Err(error("source session request count bound"));
        }
        context::validate_source_aliases(sources, &request.definitions.source_names)?;
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
        let mut definitions = Graph {
            observer: self,
            sources,
            source_names: &request.definitions.source_names,
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
        };
        definitions.register_projections(&request.definitions)?;
        let prepared = class_request
            .as_ref()
            .map(|classes| super::classes::PreparedClasses::prepare(&mut definitions, lua, classes))
            .transpose()?;
        let shared_functions = if let Some(classes) = &prepared {
            classes.shared_functions(
                &mut definitions,
                class_request.as_ref().expect("class request"),
            )?
        } else {
            BTreeMap::new()
        };
        let instance_classes = if let Some(classes) = &prepared {
            classes::instances(&mut definitions, classes, instances)?
        } else {
            BTreeMap::new()
        };
        let mut immutable = request
            .definitions
            .projections
            .iter()
            .map(|selection| selection.table.to_pointer() as usize)
            .collect::<BTreeSet<_>>();
        for (name, table) in &request.definition_roots {
            valid_name(name)?;
            definitions.text(name.len())?;
            immutable.insert(table.to_pointer() as usize);
        }
        if let Some(environment) = &request.definitions.environment {
            immutable.insert(environment.table.to_pointer() as usize);
        }
        if let Some(classes) = &prepared {
            immutable = classes::immutable_tables(
                &mut definitions,
                &request,
                class_request.as_ref().expect("class request"),
                classes,
            )?;
            definitions.immutable_capture_tables = Some(immutable.clone());
        }
        let mut roots = BTreeMap::new();
        for (name, function) in &request.callbacks {
            valid_name(name)?;
            if function.info().what == "C" {
                return Err(error("session callback root is not a Lua closure"));
            }
            if shared_functions.contains_key(&(function.to_pointer() as usize)) {
                return Err(error(
                    "explicit session callback is also a selected shared class function",
                ));
            }
            roots.insert(name.clone(), Value::Function(function.clone()));
        }
        for (name, value) in &request.state_roots {
            valid_name(name)?;
            if let Value::Table(table) = value
                && immutable.contains(&(table.to_pointer() as usize))
            {
                return Err(error("session state root is also classified immutable"));
            }
            if roots.insert(name.clone(), value.clone()).is_some() {
                return Err(error("duplicate session callback/state root name"));
            }
        }
        for name in roots.keys() {
            definitions.text(name.len())?;
        }
        let named_roots = roots
            .keys()
            .enumerate()
            .map(|(index, name)| (name.clone(), index))
            .collect();
        let mut live = LiveGraph::new(
            &mut definitions,
            &immutable,
            &request.state_projections,
            &shared_functions,
            &instance_classes,
        )?;
        for value in roots.values() {
            live.discover(value.clone(), 0)?;
        }
        // Projections also preserve explicitly selected but currently unreachable
        // state, rather than silently dropping the producer's declared snapshot.
        live.finish_projections()?;
        let live = live.finish();
        definitions.forbidden_tables = live.table_pointers();
        definitions.forbidden_callbacks = live.closure_pointers();
        definitions.forbidden_cells = live.cell_identities();
        classes::audit_shared(&definitions, &shared_functions)?;
        definitions.capture_projections(&request.definitions)?;
        let captured_classes = prepared
            .map(|classes| {
                classes.capture(
                    &mut definitions,
                    class_request.as_ref().expect("class request"),
                )
            })
            .transpose()?;
        let mut definition_roots = Vec::new();
        for (name, table) in &request.definition_roots {
            let SourceValue::Table(table) = definitions.value(Value::Table(table.clone()), 0)?
            else {
                unreachable!()
            };
            definition_roots.push(SourceProgramRoot {
                name: name.clone(),
                table,
            });
        }
        definitions.capture_environment(&request.definitions, &mut definition_roots)?;
        // Every explicitly immutable identity must have its actual captured table;
        // session values refer to these graph IDs without copying their data.
        let conversion = live.convert(&mut definitions, roots.into_values().collect())?;
        let context = definitions.context;
        let owner = SourceProgramOwner::new_with_closures(
            SourceProgramDefinitions {
                schema_version: SOURCE_PROGRAM_DEFINITIONS_SCHEMA_VERSION,
                source,
                tables: definitions.tables,
                callbacks: definitions.callbacks,
                roots: definition_roots,
                intrinsics: definitions.intrinsics,
            },
            captured_classes.map(|classes| classes.definitions),
            Some(context),
            conversion.prototypes,
        )
        .map_err(error)?;
        validate_sources(sources, &owner)?;
        let closures = conversion
            .closures
            .into_iter()
            .map(|(callback, captures)| {
                let id = owner
                    .closure_prototype_id(callback)
                    .expect("observed prototype");
                Ok(SourceSessionClosure {
                    prototype: owner.bind_closure_prototype(id).map_err(error)?,
                    captures,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let class_bindings = conversion
            .class_bindings
            .into_iter()
            .map(|(table, class)| Ok((table, owner.bind_class(class).map_err(error)?)))
            .collect::<Result<_>>()?;
        let input = SourceSessionInput {
            owner,
            class_bindings,
            state: conversion.state,
            coverage: conversion.coverage,
            cells: conversion.cells,
            closures,
        };
        if class_request.is_some() {
            self.verify(lua)?;
        } else {
            self.verify_capture_context(lua, request.definitions.environment.is_some())?;
        }
        Ok(ObservedSourceSession {
            input,
            roots: named_roots,
        })
    }
}
fn valid_name(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > 256 || name.contains('\0') {
        return Err(error("invalid source session root name"));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
