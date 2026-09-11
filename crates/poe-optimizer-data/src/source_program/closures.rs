//! Source closure prototypes contain code/layout, never a build's live values.
use super::*;

pub const SOURCE_CLOSURE_PROTOTYPES_SCHEMA_VERSION: u32 = 1;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SourceClosurePrototypeId(pub u32);
/// The callback retains the exact source span, environment and ordered upvalue
/// names. Every slot is explicitly LiveCapture; actual values/cell identities
/// belong exclusively to one coherent session input artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceClosurePrototype {
    pub callback: SourceCallbackId,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceClosurePrototypes {
    pub schema_version: u32,
    pub prototypes: Vec<SourceClosurePrototype>,
}
impl SourceClosurePrototypes {
    pub fn from_bytes(
        bytes: &[u8],
        definitions: &SourceProgramDefinitions,
    ) -> SourceProgramResult<Self> {
        if bytes.len() > 4 * 1024 * 1024 {
            return Err(failure(
                SourceProgramErrorKind::ResourceLimit,
                "closure prototype JSON byte bound",
            ));
        }
        let result: Self = serde_json::from_slice(bytes).map_err(|error| {
            failure(
                SourceProgramErrorKind::InvalidData,
                format!("closure prototype JSON: {error}"),
            )
        })?;
        result.validate(definitions)?;
        Ok(result)
    }
    pub fn validate(&self, definitions: &SourceProgramDefinitions) -> SourceProgramResult<()> {
        validate_declarations(definitions, Some(self))
    }
}
/// Checks complete marker ownership even when no prototypes were supplied.
/// Graph validation separately rejects markers in table payloads/parser graphs.
pub(super) fn validate_declarations(
    definitions: &SourceProgramDefinitions,
    prototypes: Option<&SourceClosurePrototypes>,
) -> SourceProgramResult<()> {
    if definitions.callbacks.len() > 20_000 {
        return Err(failure(
            SourceProgramErrorKind::ResourceLimit,
            "closure callback count bound",
        ));
    }
    let mut captures = 0usize;
    for callback in &definitions.callbacks {
        if callback.upvalues.len() > 128 {
            return Err(failure(
                SourceProgramErrorKind::ResourceLimit,
                "closure capture count bound",
            ));
        }
        captures = captures
            .checked_add(callback.upvalues.len())
            .ok_or_else(|| {
                failure(
                    SourceProgramErrorKind::ResourceLimit,
                    "closure capture count overflow",
                )
            })?;
        if captures > 1_000_000 {
            return Err(failure(
                SourceProgramErrorKind::ResourceLimit,
                "aggregate closure capture count bound",
            ));
        }
    }
    let mut declared = BTreeSet::new();
    if let Some(prototypes) = prototypes {
        if prototypes.schema_version != SOURCE_CLOSURE_PROTOTYPES_SCHEMA_VERSION {
            return Err(failure(
                SourceProgramErrorKind::InvalidData,
                "unsupported closure prototype schema",
            ));
        }
        if prototypes.prototypes.len() > 20_000 {
            return Err(failure(
                SourceProgramErrorKind::ResourceLimit,
                "closure prototype count bound",
            ));
        }
        for prototype in &prototypes.prototypes {
            let callback = prototype
                .callback
                .0
                .checked_sub(1)
                .and_then(|index| definitions.callbacks.get(index as usize))
                .ok_or_else(|| {
                    failure(
                        SourceProgramErrorKind::Binding,
                        "closure prototype callback is missing",
                    )
                })?;
            if !declared.insert(prototype.callback) {
                return Err(failure(
                    SourceProgramErrorKind::InvalidData,
                    "duplicate closure prototype callback",
                ));
            }
            if !matches!(callback.kind, SourceCallbackKind::Lua { .. }) {
                return Err(failure(
                    SourceProgramErrorKind::Binding,
                    "closure prototype requires a Lua source callback",
                ));
            }
            if callback
                .upvalues
                .iter()
                .any(|capture| !matches!(capture.value, SourceValue::LiveCapture {}))
            {
                return Err(failure(
                    SourceProgramErrorKind::Binding,
                    "closure prototype must declare every ordered capture as live",
                ));
            }
        }
    }
    for (index, callback) in definitions.callbacks.iter().enumerate() {
        if callback
            .upvalues
            .iter()
            .any(|capture| matches!(capture.value, SourceValue::LiveCapture {}))
            && !declared.contains(&SourceCallbackId(index as u32 + 1))
        {
            return Err(failure(
                SourceProgramErrorKind::Binding,
                "live capture has no declared source closure prototype",
            ));
        }
    }
    Ok(())
}
#[derive(Debug)]
pub(super) struct ClosureStorage {
    definitions: SourceClosurePrototypes,
    by_callback: BTreeMap<SourceCallbackId, SourceClosurePrototypeId>,
}
impl ClosureStorage {
    fn new(definitions: SourceClosurePrototypes) -> Self {
        let by_callback = definitions
            .prototypes
            .iter()
            .enumerate()
            .map(|(index, prototype)| {
                (
                    prototype.callback,
                    SourceClosurePrototypeId(index as u32 + 1),
                )
            })
            .collect();
        Self {
            definitions,
            by_callback,
        }
    }
}
impl SourceProgramOwner {
    /// Fresh identity binds immutable code/layout to optional source context and
    /// classes. No live build graph or mutable capture values enter this owner.
    pub fn new_with_closures(
        data: SourceProgramDefinitions,
        classes: Option<SourceClassDefinitions>,
        context: Option<SourceProgramContext>,
        prototypes: SourceClosurePrototypes,
    ) -> SourceProgramResult<Self> {
        data.validate()?;
        prototypes.validate(&data)?;
        if let Some(classes) = &classes {
            classes.validate(&data)?;
            super::classes::validate_shared_callbacks(&data, classes, &prototypes)?;
        }
        if let Some(context) = &context {
            context.validate(&data, classes.as_ref())?;
        } else {
            iteration::validate(&data, classes.as_ref(), None)?;
        }
        Ok(Self(OwnerStorage::Standalone {
            definitions: Arc::new(data),
            classes: classes.map(Arc::new),
            context: context.map(Arc::new),
            closures: Some(Arc::new(ClosureStorage::new(prototypes))),
        }))
    }
    pub fn closure_prototypes(&self) -> Option<&SourceClosurePrototypes> {
        match &self.0 {
            OwnerStorage::Standalone { closures, .. } => {
                closures.as_deref().map(|value| &value.definitions)
            }
            OwnerStorage::Parser(_) => None,
        }
    }
    pub fn closure_prototype(
        &self,
        id: SourceClosurePrototypeId,
    ) -> Option<&SourceClosurePrototype> {
        id.0.checked_sub(1)
            .and_then(|index| self.closure_prototypes()?.prototypes.get(index as usize))
    }
    pub fn closure_prototype_id(
        &self,
        callback: SourceCallbackId,
    ) -> Option<SourceClosurePrototypeId> {
        match &self.0 {
            OwnerStorage::Standalone { closures, .. } => {
                closures.as_ref()?.by_callback.get(&callback).copied()
            }
            OwnerStorage::Parser(_) => None,
        }
    }
    pub fn bind_closure_prototype(
        &self,
        id: SourceClosurePrototypeId,
    ) -> SourceProgramResult<SourceClosurePrototypeHandle> {
        self.closure_prototype(id).ok_or_else(|| {
            failure(
                SourceProgramErrorKind::Binding,
                "closure prototype is not bound to this owner",
            )
        })?;
        Ok(SourceClosurePrototypeHandle {
            owner: self.clone(),
            id,
        })
    }
    pub fn resolve_closure_prototype(
        &self,
        handle: &SourceClosurePrototypeHandle,
    ) -> SourceProgramResult<&SourceClosurePrototype> {
        if !self.is_same_owner(&handle.owner) {
            return Err(failure(
                SourceProgramErrorKind::Binding,
                "closure prototype belongs to another owner",
            ));
        }
        Ok(self
            .closure_prototype(handle.id)
            .expect("immutable bound closure prototype"))
    }
}
#[derive(Debug, Clone)]
pub struct SourceClosurePrototypeHandle {
    owner: SourceProgramOwner,
    id: SourceClosurePrototypeId,
}
impl SourceClosurePrototypeHandle {
    pub fn owner(&self) -> &SourceProgramOwner {
        &self.owner
    }
    pub fn id(&self) -> SourceClosurePrototypeId {
        self.id
    }
    pub fn definition(&self) -> &SourceClosurePrototype {
        self.owner
            .closure_prototype(self.id)
            .expect("immutable bound closure prototype")
    }
    pub fn capture_count(&self) -> usize {
        self.owner
            .callback(self.definition().callback)
            .expect("validated source callback")
            .upvalues
            .len()
    }
}
