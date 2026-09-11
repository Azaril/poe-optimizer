//! Source-bound class projections and modeled construction metadata.
//!
//! Descriptors identify actual source-created tables and callback identities.
//! Validation establishes graph consistency only; the extracting domain must
//! authenticate source observations and separately admit numerical effects.
use super::*;
use crate::item_loading::ItemSourceSpan;

pub const SOURCE_CLASS_DEFINITIONS_SCHEMA_VERSION: u32 = 1;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SourceClassId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceClassConstructionPolicy {
    pub allocation: ItemSourceSpan,
    pub parent_call: ItemSourceSpan,
    pub parent_index: ItemSourceSpan,
    pub wrap_constructor: ItemSourceSpan,
    pub parent_call_callback: SourceCallbackId,
    pub parent_call_format_upvalue: u16,
    pub parent_index_callback: SourceCallbackId,
    pub object_alias: String,
    pub parent_init: String,
    pub proxy_parent: String,
    pub proxy_object: String,
    pub proxy_class_name: String,
    pub class_name_field: String,
    pub parent_classes_field: String,
    pub super_parents_field: String,
    pub unconstructed_meta_field: String,
    pub constructor_initialized_field: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceClassMethod {
    pub callback: SourceCallbackId,
    /// The original declaring class, retained across copied inheritance.
    pub declared_by: SourceClassId,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceClassConstructor {
    /// Original constructor body, before the source wrapper is installed.
    pub callback: SourceCallbackId,
    /// Actual constructed class table entry, when the source wrapped the body.
    #[serde(deserialize_with = "required_option")]
    pub wrapper: Option<SourceClassConstructorWrapper>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceClassConstructorWrapper {
    pub callback: SourceCallbackId,
    pub original_upvalue: u16,
    pub class_upvalue: u16,
    pub class_name_upvalue: u16,
    pub pairs_upvalue: u16,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceClassDefinition {
    pub name: String,
    /// Source-created table projection. This is not an arbitrary Lua metatable.
    pub table: SourceTableId,
    /// Ordered direct parents. Source newClass copies their fields in this order.
    pub parents: Vec<SourceClassId>,
    /// Actual source pairs order over the class-keyed superclass set. None is
    /// absent; Some(empty) is a present, empty table. This is not derived order.
    #[serde(deserialize_with = "required_option")]
    pub super_parents: Option<Vec<SourceClassId>>,
    /// Known fields omitted from this projection (for example table-key sets).
    /// Reached reads must report an unsupported frontier, never invented Nil.
    pub unsupported_fields: BTreeSet<String>,
    #[serde(deserialize_with = "crate::modifier_parser::unique_map")]
    pub methods: BTreeMap<String, SourceClassMethod>,
    #[serde(deserialize_with = "required_option")]
    pub constructor: Option<SourceClassConstructor>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceClassDefinitions {
    pub schema_version: u32,
    pub source: SourceClassConstructionPolicy,
    pub classes: Vec<SourceClassDefinition>,
}
impl SourceClassDefinitions {
    pub fn from_bytes(
        bytes: &[u8],
        definitions: &SourceProgramDefinitions,
    ) -> SourceProgramResult<Self> {
        if bytes.len() > 16 * 1024 * 1024 {
            return Err(failure(
                SourceProgramErrorKind::ResourceLimit,
                "source class JSON byte bound",
            ));
        }
        let classes: Self = serde_json::from_slice(bytes).map_err(|e| {
            failure(
                SourceProgramErrorKind::InvalidData,
                format!("source class JSON: {e}"),
            )
        })?;
        classes.validate(definitions)?;
        Ok(classes)
    }
    pub fn validate(&self, definitions: &SourceProgramDefinitions) -> SourceProgramResult<()> {
        if self.schema_version != SOURCE_CLASS_DEFINITIONS_SCHEMA_VERSION {
            return Err(failure(
                SourceProgramErrorKind::InvalidData,
                "unsupported source class schema",
            ));
        }
        if self.classes.len() > 512 {
            return Err(failure(
                SourceProgramErrorKind::ResourceLimit,
                "source class count bound",
            ));
        }
        let graph = graph::GraphValidation::new(
            &definitions.source,
            &definitions.tables,
            &definitions.callbacks,
        )
        .map_err(graph_error)?;
        for span in [
            &self.source.allocation,
            &self.source.parent_call,
            &self.source.parent_index,
            &self.source.wrap_constructor,
        ] {
            graph.span(span).map_err(graph_error)?;
        }
        for (id, span) in [
            (self.source.parent_call_callback, &self.source.parent_call),
            (self.source.parent_index_callback, &self.source.parent_index),
        ] {
            graph.callback(id).map_err(graph_error)?;
            if definitions.callbacks[id.0 as usize - 1].kind
                != (SourceCallbackKind::Lua {
                    source: span.clone(),
                })
            {
                return Err(failure(
                    SourceProgramErrorKind::Binding,
                    "source proxy callback differs from policy span",
                ));
            }
        }
        let parent_call = &definitions.callbacks[self.source.parent_call_callback.0 as usize - 1];
        let parent_index = &definitions.callbacks[self.source.parent_index_callback.0 as usize - 1];
        if parent_call.upvalues.len() != 1
            || !captured_builtin(
                definitions,
                parent_call,
                self.source.parent_call_format_upvalue,
                "string.format",
            )
            || !parent_index.upvalues.is_empty()
        {
            return Err(failure(
                SourceProgramErrorKind::UnsupportedCapability,
                "source parent proxy protocol has unrepresented captures",
            ));
        }
        let fields = [
            &self.source.object_alias,
            &self.source.parent_init,
            &self.source.proxy_parent,
            &self.source.proxy_object,
            &self.source.proxy_class_name,
        ];
        let mut field_names = BTreeSet::new();
        for key in fields {
            check_name(key)?;
            graph.charge(key.len()).map_err(graph_error)?;
            if !field_names.insert(key)
                || ["__index", "__newindex", "__call"].contains(&key.as_str())
            {
                return Err(failure(
                    SourceProgramErrorKind::Binding,
                    "source construction fields collide",
                ));
            }
        }
        let mut metadata_names = BTreeSet::new();
        for key in [
            &self.source.class_name_field,
            &self.source.parent_classes_field,
            &self.source.super_parents_field,
            &self.source.unconstructed_meta_field,
            &self.source.constructor_initialized_field,
        ] {
            check_name(key)?;
            graph.charge(key.len()).map_err(graph_error)?;
            if !metadata_names.insert(key)
                || ["__index", "__newindex", "__call"].contains(&key.as_str())
            {
                return Err(failure(
                    SourceProgramErrorKind::Binding,
                    "source class metadata collides with metamethod field",
                ));
            }
        }
        let mut names = BTreeSet::new();
        let mut tables = BTreeSet::new();
        let mut method_count = 0usize;
        // Later inheritance checks can inspect parents that appear after their
        // child in serialization, so authenticate every table index first.
        for class in &self.classes {
            graph.table(class.table).map_err(graph_error)?;
        }
        for (index, class) in self.classes.iter().enumerate() {
            let id = SourceClassId(index as u32 + 1);
            check_name(&class.name)?;
            graph.charge(class.name.len()).map_err(graph_error)?;
            if !names.insert(&class.name) || !tables.insert(class.table) {
                return Err(failure(
                    SourceProgramErrorKind::Binding,
                    "duplicate source class name or table identity",
                ));
            }
            graph.table(class.table).map_err(graph_error)?;
            if class.parents.len() > 64 || class.methods.len() > 4096 {
                return Err(failure(
                    SourceProgramErrorKind::ResourceLimit,
                    "source class parent or method bound",
                ));
            }
            method_count = method_count
                .checked_add(class.methods.len())
                .ok_or_else(|| {
                    failure(
                        SourceProgramErrorKind::ResourceLimit,
                        "source class method count overflow",
                    )
                })?;
            if method_count > 32768 {
                return Err(failure(
                    SourceProgramErrorKind::ResourceLimit,
                    "aggregate source class method bound",
                ));
            }
            for parent in &class.parents {
                self.get(*parent)?;
                if *parent == id {
                    return Err(failure(
                        SourceProgramErrorKind::Binding,
                        "source class self parent",
                    ));
                }
            }
            let table = &definitions.tables[class.table.0 as usize - 1];
            if table.fields.get(&self.source.class_name_field)
                != Some(&SourceValue::Text(class.name.clone()))
            {
                return Err(failure(
                    SourceProgramErrorKind::Binding,
                    "source class name differs from table identity",
                ));
            }
            match table.fields.get(&self.source.parent_classes_field) {
                None if class.parents.is_empty() => {}
                Some(SourceValue::Table(parent_table)) => {
                    let parents = &definitions.tables[parent_table.0 as usize - 1];
                    if !parents.fields.is_empty()
                        || parents.indexed.len() != class.parents.len()
                        || class.parents.iter().enumerate().any(|(i, id)| {
                            parents.indexed.get(&(i as i64 + 1))
                                != Some(&SourceValue::Table(self.classes[id.0 as usize - 1].table))
                        })
                    {
                        return Err(failure(
                            SourceProgramErrorKind::Binding,
                            "source class parents differ from ordered source table",
                        ));
                    }
                }
                _ => {
                    return Err(failure(
                        SourceProgramErrorKind::Binding,
                        "source class parent table is absent or invalid",
                    ));
                }
            }
            if class.unsupported_fields.len() > 4096 {
                return Err(failure(
                    SourceProgramErrorKind::ResourceLimit,
                    "source class unsupported field bound",
                ));
            }
            for key in &class.unsupported_fields {
                check_name(key)?;
                graph.charge(key.len()).map_err(graph_error)?;
                if table.fields.contains_key(key) || class.methods.contains_key(key) {
                    return Err(failure(
                        SourceProgramErrorKind::Binding,
                        "source class field is both represented and unsupported",
                    ));
                }
            }
            if let Some(super_parents) = &class.super_parents {
                if super_parents.len() > 512 {
                    return Err(failure(
                        SourceProgramErrorKind::ResourceLimit,
                        "source superclass set bound",
                    ));
                }
                let mut observed = BTreeSet::new();
                for parent in super_parents {
                    self.get(*parent)?;
                    if !observed.insert(*parent) || *parent == id {
                        return Err(failure(
                            SourceProgramErrorKind::Binding,
                            "source superclass set repeats an identity or contains self",
                        ));
                    }
                }
                match table.fields.get(&self.source.super_parents_field) {
                    None if class
                        .unsupported_fields
                        .contains(&self.source.super_parents_field) => {}
                    Some(SourceValue::Table(set))
                        if super_parents.is_empty()
                            && definitions.tables[set.0 as usize - 1].fields.is_empty()
                            && definitions.tables[set.0 as usize - 1].indexed.is_empty() => {}
                    _ => {
                        return Err(failure(
                            SourceProgramErrorKind::Binding,
                            "source superclass observation differs from projected set presence",
                        ));
                    }
                }
            } else if table.fields.contains_key(&self.source.super_parents_field)
                || class
                    .unsupported_fields
                    .contains(&self.source.super_parents_field)
            {
                return Err(failure(
                    SourceProgramErrorKind::Binding,
                    "source superclass observation omits a present set",
                ));
            }
            for (key, method) in &class.methods {
                check_name(key)?;
                graph.charge(key.len()).map_err(graph_error)?;
                self.get(method.declared_by)?;
                graph.callback(method.callback).map_err(graph_error)?;
                if table.fields.get(key) != Some(&SourceValue::Callback(method.callback)) {
                    return Err(failure(
                        SourceProgramErrorKind::Binding,
                        "source class method differs from actual table entry",
                    ));
                }
                if method.declared_by != id {
                    let inherited = class.parents.iter().find_map(|parent| {
                        let parent = &self.classes[parent.0 as usize - 1];
                        let parent_table = &definitions.tables[parent.table.0 as usize - 1];
                        (parent_table.fields.contains_key(key)
                            || parent.unsupported_fields.contains(key))
                        .then(|| parent.methods.get(key))
                    });
                    if inherited != Some(Some(method)) {
                        // newClass copies callback identities. A subsequently
                        // wrapped parent constructor does not rewrite that old
                        // child entry; the wrapper's authenticated capture gives
                        // the original identity without inventing dynamic inheritance.
                        let before_wrap = class.parents.iter().find_map(|parent| {
                            let parent_id = *parent;
                            let parent = &self.classes[parent.0 as usize - 1];
                            let table = &definitions.tables[parent.table.0 as usize - 1];
                            (table.fields.contains_key(key)
                                || parent.unsupported_fields.contains(key))
                            .then(|| {
                                parent.name == *key
                                    && method.declared_by == parent_id
                                    && parent.constructor.as_ref().is_some_and(|constructor| {
                                        constructor.callback == method.callback
                                            && constructor.wrapper.is_some()
                                    })
                            })
                        });
                        if before_wrap != Some(true) {
                            return Err(failure(
                                SourceProgramErrorKind::UnsupportedCapability,
                                "inherited callback needs an unrepresented parent mutation or copied-field observation",
                            ));
                        }
                    }
                }
            }
            if class.constructor.is_none() {
                if class.unsupported_fields.contains(&class.name) {
                    return Err(failure(
                        SourceProgramErrorKind::UnsupportedCapability,
                        "source class constructor presence is unrepresented",
                    ));
                }
                match table.fields.get(&class.name) {
                    None | Some(SourceValue::Boolean(false)) => {}
                    Some(SourceValue::Callback(_)) => {
                        return Err(failure(
                            SourceProgramErrorKind::Binding,
                            "source class constructor callback lacks a constructor descriptor",
                        ));
                    }
                    Some(_) => {
                        return Err(failure(
                            SourceProgramErrorKind::UnsupportedCapability,
                            "source class has an unrepresented callable or invalid constructor value",
                        ));
                    }
                }
            }
            if let Some(constructor) = &class.constructor {
                graph.callback(constructor.callback).map_err(graph_error)?;
                if let Some(wrapper) = &constructor.wrapper {
                    graph.callback(wrapper.callback).map_err(graph_error)?;
                    let closure = &definitions.callbacks[wrapper.callback.0 as usize - 1];
                    let slots = BTreeSet::from([
                        wrapper.original_upvalue,
                        wrapper.class_upvalue,
                        wrapper.class_name_upvalue,
                        wrapper.pairs_upvalue,
                    ]);
                    if closure.upvalues.len() != 4
                        || slots.len() != 4
                        || !captured_builtin(definitions, closure, wrapper.pairs_upvalue, "pairs")
                    {
                        return Err(failure(
                            SourceProgramErrorKind::UnsupportedCapability,
                            "source constructor wrapper has unrepresented captures",
                        ));
                    }
                    let capture =
                        |index: u16| closure.upvalues.get(index as usize).map(|v| &v.value);
                    if capture(wrapper.original_upvalue)
                        != Some(&SourceValue::Callback(constructor.callback))
                        || capture(wrapper.class_upvalue) != Some(&SourceValue::Table(class.table))
                        || capture(wrapper.class_name_upvalue)
                            != Some(&SourceValue::Text(class.name.clone()))
                    {
                        return Err(failure(
                            SourceProgramErrorKind::Binding,
                            "source constructor wrapper differs from captured original or class identity",
                        ));
                    }
                    let SourceCallbackKind::Lua { source } = &closure.kind else {
                        return Err(failure(
                            SourceProgramErrorKind::Binding,
                            "source constructor wrapper is not Lua",
                        ));
                    };
                    let enclosing = &self.source.wrap_constructor;
                    if source.path != enclosing.path
                        || source.line < enclosing.line
                        || source.end_line > enclosing.end_line
                    {
                        return Err(failure(
                            SourceProgramErrorKind::Binding,
                            "source constructor wrapper outside policy span",
                        ));
                    }
                }
                if table.fields.get(&class.name)
                    != Some(&SourceValue::Callback(
                        constructor
                            .wrapper
                            .as_ref()
                            .map(|w| w.callback)
                            .unwrap_or(constructor.callback),
                    ))
                {
                    return Err(failure(
                        SourceProgramErrorKind::Binding,
                        "source constructor differs from actual class table entry",
                    ));
                }
            }
        }
        // Iterative postorder bounds the longest ancestry path without host
        // recursion; memoization cannot hide a long path through a shared base.
        let mut states = vec![0u8; self.classes.len()];
        let mut depths = vec![0usize; self.classes.len()];
        for start in 0..self.classes.len() {
            if states[start] == 2 {
                continue;
            }
            let mut pending = vec![(start, 0usize)];
            states[start] = 1;
            while let Some((index, next_parent)) = pending.last_mut() {
                if let Some(parent) = self.classes[*index].parents.get(*next_parent) {
                    *next_parent += 1;
                    let parent = parent.0 as usize - 1;
                    if states[parent] == 1 {
                        return Err(failure(
                            SourceProgramErrorKind::Binding,
                            "cyclic source class inheritance",
                        ));
                    }
                    if states[parent] == 0 {
                        states[parent] = 1;
                        pending.push((parent, 0));
                    }
                } else {
                    let depth = self.classes[*index]
                        .parents
                        .iter()
                        .map(|p| depths[p.0 as usize - 1])
                        .max()
                        .unwrap_or(0)
                        + 1;
                    if depth > 64 {
                        return Err(failure(
                            SourceProgramErrorKind::ResourceLimit,
                            "source class inheritance depth bound",
                        ));
                    }
                    depths[*index] = depth;
                    states[*index] = 2;
                    pending.pop();
                }
            }
        }
        for class in &self.classes {
            let mut expected = BTreeSet::new();
            let mut pending = class.parents.clone();
            while let Some(id) = pending.pop() {
                if expected.insert(id) {
                    pending.extend_from_slice(&self.get(id)?.parents);
                }
            }
            let observed = class
                .super_parents
                .as_ref()
                .map(|parents| parents.iter().copied().collect::<BTreeSet<_>>());
            if observed
                .as_ref()
                .is_some_and(|observed| observed != &expected)
                || observed.is_none() && !expected.is_empty()
            {
                return Err(failure(
                    SourceProgramErrorKind::UnsupportedCapability,
                    "observed superclass set differs from supported source ancestry",
                ));
            }
        }
        Ok(())
    }
    fn get(&self, id: SourceClassId) -> SourceProgramResult<&SourceClassDefinition> {
        id.0.checked_sub(1)
            .and_then(|index| self.classes.get(index as usize))
            .ok_or_else(|| {
                failure(
                    SourceProgramErrorKind::Binding,
                    "dangling source class reference",
                )
            })
    }
}
fn check_name(value: &str) -> SourceProgramResult<()> {
    if value.is_empty() || value.len() > 256 || value.contains('\0') {
        return Err(failure(
            SourceProgramErrorKind::InvalidData,
            "invalid source class name or key",
        ));
    }
    Ok(())
}
impl SourceProgramOwner {
    pub fn new_with_classes(
        data: SourceProgramDefinitions,
        classes: SourceClassDefinitions,
    ) -> SourceProgramResult<Self> {
        data.validate()?;
        classes.validate(&data)?;
        Ok(Self(OwnerStorage::Standalone {
            definitions: Arc::new(data),
            classes: Some(Arc::new(classes)),
        }))
    }
    pub fn classes(&self) -> Option<&SourceClassDefinitions> {
        match &self.0 {
            OwnerStorage::Standalone { classes, .. } => classes.as_deref(),
            _ => None,
        }
    }
    pub fn class(&self, id: SourceClassId) -> Option<&SourceClassDefinition> {
        self.classes()?.get(id).ok()
    }
    pub fn class_id(&self, name: &str) -> Option<SourceClassId> {
        self.classes()?
            .classes
            .iter()
            .position(|class| class.name == name)
            .map(|index| SourceClassId(index as u32 + 1))
    }
    pub fn bind_class(&self, id: SourceClassId) -> SourceProgramResult<SourceClassHandle> {
        self.class(id).ok_or_else(|| {
            failure(
                SourceProgramErrorKind::Binding,
                "class is not bound to this owner",
            )
        })?;
        Ok(SourceClassHandle {
            owner: self.clone(),
            id,
        })
    }
    pub fn resolve_class(
        &self,
        handle: &SourceClassHandle,
    ) -> SourceProgramResult<&SourceClassDefinition> {
        if !self.is_same_owner(&handle.owner) {
            return Err(failure(
                SourceProgramErrorKind::Binding,
                "source class belongs to another owner",
            ));
        }
        Ok(self.class(handle.id).expect("immutable bound class"))
    }
}
#[derive(Debug, Clone)]
pub struct SourceClassHandle {
    owner: SourceProgramOwner,
    id: SourceClassId,
}
impl SourceClassHandle {
    pub fn owner(&self) -> &SourceProgramOwner {
        &self.owner
    }
    pub fn id(&self) -> SourceClassId {
        self.id
    }
    pub fn definition(&self) -> &SourceClassDefinition {
        self.owner.class(self.id).expect("immutable bound class")
    }
}

fn required_option<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    Option::<T>::deserialize(deserializer)
}

fn captured_builtin(
    definitions: &SourceProgramDefinitions,
    callback: &SourceCallback,
    slot: u16,
    symbol: &str,
) -> bool {
    let Some(SourceValue::Callback(id)) = callback.upvalues.get(slot as usize).map(|v| &v.value)
    else {
        return false;
    };
    let Some(captured) =
        id.0.checked_sub(1)
            .and_then(|index| definitions.callbacks.get(index as usize))
    else {
        return false;
    };
    matches!(&captured.kind, SourceCallbackKind::Builtin {symbol: actual} if actual == symbol)
        && captured.upvalues.is_empty()
}
