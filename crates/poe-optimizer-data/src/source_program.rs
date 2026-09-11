//! Domain-neutral immutable source definitions and verified programs.
//!
//! Compilation proves graph/binding/language validity only. A domain must
//! separately admit its effects and establish original-source parity. Parser
//! compatibility retains the original Arc; no graph is cloned or synthesized.
use crate::{item_loading::ItemLoadingSource, modifier_parser::*};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
mod classes;
pub(crate) mod graph;
pub use classes::*;

// Public neutral names deliberately re-export the same graph and IR types. Old
// parser names and their wire formats remain valid, including tuple constructors.
pub use crate::modifier_parser::{
    ParserCallback as SourceCallback, ParserCallbackId as SourceCallbackId,
    ParserCallbackKind as SourceCallbackKind, ParserEnvironment as SourceEnvironment,
    ParserNonFinite as SourceNonFinite, ParserProgram as SourceProgram,
    ParserProgramBinary as SourceProgramBinary, ParserProgramBinding as SourceProgramBinding,
    ParserProgramBranch as SourceProgramBranch, ParserProgramCall as SourceProgramCall,
    ParserProgramCapability as SourceProgramCapability, ParserProgramData as SourceProgramData,
    ParserProgramError as SourceProgramError, ParserProgramErrorKind as SourceProgramErrorKind,
    ParserProgramExpr as SourceProgramExpr, ParserProgramExprKind as SourceProgramExprKind,
    ParserProgramField as SourceProgramField, ParserProgramId as SourceProgramId,
    ParserProgramIntrinsic as SourceProgramIntrinsic,
    ParserProgramIntrinsicSource as SourceProgramIntrinsicSource,
    ParserProgramIterator as SourceProgramIterator, ParserProgramLocation as SourceProgramLocation,
    ParserProgramPack as SourceProgramPack, ParserProgramProvenance as SourceProgramProvenance,
    ParserProgramResult as SourceProgramResult, ParserProgramStatement as SourceProgramStatement,
    ParserProgramStatementKind as SourceProgramStatementKind,
    ParserProgramUnary as SourceProgramUnary, ParserProgramValueList as SourceProgramValueList,
    ParserTable as SourceTable, ParserTableId as SourceTableId, ParserUpvalue as SourceUpvalue,
    ParserValue as SourceValue,
};
pub const SOURCE_PROGRAM_SCHEMA_VERSION: u32 = PARSER_PROGRAM_SCHEMA_VERSION;
pub const SOURCE_PROGRAM_DEFINITIONS_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SourceProgramRootId(pub u32);
/// Neutral root reference; parser compatibility keeps its original four-value
/// enum and total lookup API. Named indices belong only to standalone owners.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceProgramDefinitionRoot {
    ModFlags,
    KeywordFlags,
    SkillTypes,
    GemIdLookup,
    Named(SourceProgramRootId),
}
impl From<ParserProgramDefinitionRoot> for SourceProgramDefinitionRoot {
    fn from(root: ParserProgramDefinitionRoot) -> Self {
        match root {
            ParserProgramDefinitionRoot::ModFlags => Self::ModFlags,
            ParserProgramDefinitionRoot::KeywordFlags => Self::KeywordFlags,
            ParserProgramDefinitionRoot::SkillTypes => Self::SkillTypes,
            ParserProgramDefinitionRoot::GemIdLookup => Self::GemIdLookup,
        }
    }
}
impl SourceProgramDefinitionRoot {
    pub fn legacy(self) -> Option<ParserProgramDefinitionRoot> {
        match self {
            Self::ModFlags => Some(ParserProgramDefinitionRoot::ModFlags),
            Self::KeywordFlags => Some(ParserProgramDefinitionRoot::KeywordFlags),
            Self::SkillTypes => Some(ParserProgramDefinitionRoot::SkillTypes),
            Self::GemIdLookup => Some(ParserProgramDefinitionRoot::GemIdLookup),
            Self::Named(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceProgramRoot {
    pub name: String,
    pub table: SourceTableId,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceProgramDefinitions {
    pub schema_version: u32,
    pub source: ItemLoadingSource,
    pub tables: Vec<SourceTable>,
    pub callbacks: Vec<SourceCallback>,
    /// Named one-based owner-local roots; distinct names may alias one table.
    pub roots: Vec<SourceProgramRoot>,
    /// Explicit original runtime primitive identities. These are structural
    /// source claims; the importing domain must authenticate the extraction.
    #[serde(deserialize_with = "crate::modifier_parser::unique_map")]
    pub intrinsics: BTreeMap<SourceCallbackId, SourceProgramIntrinsic>,
}
impl SourceProgramDefinitions {
    pub fn validate(&self) -> SourceProgramResult<()> {
        if self.schema_version != SOURCE_PROGRAM_DEFINITIONS_SCHEMA_VERSION {
            return Err(failure(
                SourceProgramErrorKind::InvalidData,
                "unsupported source definition schema",
            ));
        }
        if self.roots.len() > 4096 || self.intrinsics.len() > 256 {
            return Err(failure(
                SourceProgramErrorKind::ResourceLimit,
                "source root/intrinsic count bound",
            ));
        }
        let graph = graph::GraphValidation::new(&self.source, &self.tables, &self.callbacks)
            .map_err(graph_error)?;
        let mut names = BTreeSet::new();
        for root in &self.roots {
            graph.charge(root.name.len()).map_err(graph_error)?;
            if root.name.is_empty()
                || root.name.len() > 256
                || root.name.contains('\0')
                || !names.insert(&root.name)
            {
                return Err(failure(
                    SourceProgramErrorKind::InvalidData,
                    "invalid or duplicate source root name",
                ));
            }
            graph.table(root.table).map_err(graph_error)?;
        }
        for (id, intrinsic) in &self.intrinsics {
            graph.callback(*id).map_err(graph_error)?;
            let callback = &self.callbacks[id.0 as usize - 1];
            let Some(path) = intrinsic.global_path() else {
                return Err(failure(
                    SourceProgramErrorKind::Binding,
                    "captured source helper is not a builtin intrinsic",
                ));
            };
            if callback.kind
                != (SourceCallbackKind::Builtin {
                    symbol: path.join("."),
                })
                || !callback.upvalues.is_empty()
            {
                return Err(failure(
                    SourceProgramErrorKind::Binding,
                    "intrinsic differs from original builtin descriptor",
                ));
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone)]
enum OwnerStorage {
    Parser(ModifierParserCatalog),
    Standalone {
        definitions: Arc<SourceProgramDefinitions>,
        classes: Option<Arc<SourceClassDefinitions>>,
    },
}
#[derive(Debug, Clone)]
pub struct SourceProgramOwner(OwnerStorage);
impl SourceProgramOwner {
    pub fn new(data: SourceProgramDefinitions) -> SourceProgramResult<Self> {
        data.validate()?;
        Ok(Self(OwnerStorage::Standalone {
            definitions: Arc::new(data),
            classes: None,
        }))
    }
    /// Deserializes bounded untrusted authoring data; no source authentication or
    /// execution admission is conferred by successful structural validation.
    pub fn from_bytes(bytes: &[u8]) -> SourceProgramResult<Self> {
        if bytes.len() > 64 * 1024 * 1024 {
            return Err(failure(
                SourceProgramErrorKind::ResourceLimit,
                "source definition JSON byte bound",
            ));
        }
        Self::new(serde_json::from_slice(bytes).map_err(|e| {
            failure(
                SourceProgramErrorKind::InvalidData,
                format!("source definition JSON: {e}"),
            )
        })?)
    }
    pub fn from_parser(owner: ModifierParserCatalog) -> Self {
        Self(OwnerStorage::Parser(owner))
    }
    pub fn parser(&self) -> Option<&ModifierParserCatalog> {
        match &self.0 {
            OwnerStorage::Parser(owner) => Some(owner),
            _ => None,
        }
    }
    pub fn definitions(&self) -> Option<&SourceProgramDefinitions> {
        match &self.0 {
            OwnerStorage::Standalone {
                definitions: owner, ..
            } => Some(owner),
            _ => None,
        }
    }
    pub fn source(&self) -> &ItemLoadingSource {
        match &self.0 {
            OwnerStorage::Parser(owner) => &owner.data().source,
            OwnerStorage::Standalone {
                definitions: owner, ..
            } => &owner.source,
        }
    }
    pub fn tables(&self) -> &[SourceTable] {
        match &self.0 {
            OwnerStorage::Parser(owner) => &owner.data().tables,
            OwnerStorage::Standalone {
                definitions: owner, ..
            } => &owner.tables,
        }
    }
    pub fn callbacks(&self) -> &[SourceCallback] {
        match &self.0 {
            OwnerStorage::Parser(owner) => &owner.data().callbacks,
            OwnerStorage::Standalone {
                definitions: owner, ..
            } => &owner.callbacks,
        }
    }
    pub fn roots(&self) -> &[SourceProgramRoot] {
        match &self.0 {
            OwnerStorage::Standalone {
                definitions: owner, ..
            } => &owner.roots,
            _ => &[],
        }
    }
    pub fn is_same_owner(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (OwnerStorage::Parser(left), OwnerStorage::Parser(right)) => left.is_same_owner(right),
            (
                OwnerStorage::Standalone {
                    definitions: left, ..
                },
                OwnerStorage::Standalone {
                    definitions: right, ..
                },
            ) => Arc::ptr_eq(left, right),
            _ => false,
        }
    }
    /// Raw graph IDs are owner-local serialization indices. Use a bound handle
    /// when retaining or transporting a reference across API boundaries.
    pub fn table(&self, id: SourceTableId) -> Option<&SourceTable> {
        id.0.checked_sub(1)
            .and_then(|i| self.tables().get(i as usize))
    }
    pub fn callback(&self, id: SourceCallbackId) -> Option<&SourceCallback> {
        id.0.checked_sub(1)
            .and_then(|i| self.callbacks().get(i as usize))
    }
    pub fn factory(&self, id: SourceCallbackId) -> Option<&ParserFactoryDisposition> {
        self.parser()?.factory(id)
    }
    pub fn intrinsic(&self, id: SourceCallbackId) -> Option<SourceProgramIntrinsic> {
        self.definitions()?.intrinsics.get(&id).copied()
    }
    pub fn root_id(&self, name: &str) -> Option<SourceProgramRootId> {
        self.roots()
            .iter()
            .position(|root| root.name == name)
            .map(|i| SourceProgramRootId(i as u32 + 1))
    }
    pub fn definition_id(&self, root: SourceProgramDefinitionRoot) -> Option<SourceTableId> {
        match root {
            SourceProgramDefinitionRoot::Named(id) => {
                id.0.checked_sub(1)
                    .and_then(|i| self.roots().get(i as usize))
                    .map(|root| root.table)
            }
            legacy => self
                .parser()
                .and_then(|owner| parser_definition_id(owner.data(), legacy)),
        }
    }
    pub fn definition(&self, root: SourceProgramDefinitionRoot) -> Option<&SourceTable> {
        self.table(self.definition_id(root)?)
    }
    pub fn bind_root(
        &self,
        root: SourceProgramDefinitionRoot,
    ) -> SourceProgramResult<SourceProgramRootHandle> {
        let table = self.definition_id(root).ok_or_else(|| {
            failure(
                SourceProgramErrorKind::Binding,
                "definition root is not bound to this owner",
            )
        })?;
        Ok(SourceProgramRootHandle {
            owner: self.clone(),
            root,
            table,
        })
    }
    pub fn resolve_root(
        &self,
        handle: &SourceProgramRootHandle,
    ) -> SourceProgramResult<&SourceTable> {
        if !self.is_same_owner(&handle.owner) {
            return Err(failure(
                SourceProgramErrorKind::Binding,
                "definition root belongs to another owner",
            ));
        }
        Ok(self
            .table(handle.table)
            .expect("bound immutable root table"))
    }
}
/// An immutable bound definition root cannot be constructed from an untrusted
/// numeric ID, serialized into a token, or silently transferred to another owner.
#[derive(Debug, Clone)]
pub struct SourceProgramRootHandle {
    owner: SourceProgramOwner,
    root: SourceProgramDefinitionRoot,
    table: SourceTableId,
}
impl SourceProgramRootHandle {
    pub fn owner(&self) -> &SourceProgramOwner {
        &self.owner
    }
    pub fn root(&self) -> SourceProgramDefinitionRoot {
        self.root
    }
    pub fn table_id(&self) -> SourceTableId {
        self.table
    }
    pub fn table(&self) -> &SourceTable {
        self.owner
            .table(self.table)
            .expect("bound immutable root table")
    }
}
#[derive(Debug, Clone)]
enum ProgramStorage {
    Authored(Arc<SourceProgramData>),
    ParserPayload(ModifierParserCatalog),
}
#[derive(Debug, Clone)]
pub struct SourceProgramCatalog {
    data: ProgramStorage,
    owner: SourceProgramOwner,
    required: BTreeSet<SourceProgramCapability>,
}
impl SourceProgramCatalog {
    pub fn new(data: SourceProgramData, owner: SourceProgramOwner) -> SourceProgramResult<Self> {
        let required = crate::modifier_parser::programs::validate::validate(&data, &owner)?;
        Ok(Self {
            data: ProgramStorage::Authored(Arc::new(data)),
            owner,
            required,
        })
    }
    /// Called only after the parser payload verifier has checked the same owner.
    /// The immutable owner retains the IR; this view neither clones its recursive
    /// programs nor creates an owner/catalog Arc cycle.
    pub(crate) fn from_verified_parser_payload(
        owner: ModifierParserCatalog,
        required: BTreeSet<SourceProgramCapability>,
    ) -> Self {
        Self {
            data: ProgramStorage::ParserPayload(owner.clone()),
            owner: SourceProgramOwner::from_parser(owner),
            required,
        }
    }
    pub fn from_bytes(bytes: &[u8], owner: SourceProgramOwner) -> SourceProgramResult<Self> {
        if bytes.len() > 16 * 1024 * 1024 {
            return Err(failure(
                SourceProgramErrorKind::ResourceLimit,
                "program JSON byte bound",
            ));
        }
        Self::new(
            serde_json::from_slice(bytes).map_err(|e| {
                failure(
                    SourceProgramErrorKind::InvalidData,
                    format!("program JSON: {e}"),
                )
            })?,
            owner,
        )
    }
    pub fn data(&self) -> &SourceProgramData {
        match &self.data {
            ProgramStorage::Authored(data) => data,
            ProgramStorage::ParserPayload(owner) => &owner.data().programs.data,
        }
    }
    pub fn owner(&self) -> &SourceProgramOwner {
        &self.owner
    }
    pub fn is_bound_to(&self, owner: &SourceProgramOwner) -> bool {
        self.owner.is_same_owner(owner)
    }
    pub fn program(&self, id: SourceProgramId) -> Option<&SourceProgram> {
        id.0.checked_sub(1)
            .and_then(|i| self.data().programs.get(i as usize))
    }
    pub fn program_id(&self, callback: SourceCallbackId) -> Option<SourceProgramId> {
        self.data().callbacks.get(&callback).copied()
    }
    pub fn for_callback(&self, callback: SourceCallbackId) -> Option<&SourceProgram> {
        self.program(self.program_id(callback)?)
    }
    pub fn required_capabilities(&self) -> &BTreeSet<SourceProgramCapability> {
        &self.required
    }
    pub fn check_capabilities(
        &self,
        available: &BTreeSet<SourceProgramCapability>,
    ) -> SourceProgramResult<()> {
        if let Some(missing) = self.required.difference(available).next() {
            return Err(failure(
                SourceProgramErrorKind::UnsupportedCapability,
                format!("program capability unavailable: {missing:?}"),
            ));
        }
        Ok(())
    }
    pub fn definition(&self, root: SourceProgramDefinitionRoot) -> Option<&SourceTable> {
        self.owner.definition(root)
    }
}
fn parser_definition_id(
    owner: &ModifierParserData,
    root: SourceProgramDefinitionRoot,
) -> Option<SourceTableId> {
    match root {
        SourceProgramDefinitionRoot::ModFlags => Some(owner.policy.mod_flags),
        SourceProgramDefinitionRoot::KeywordFlags => Some(owner.policy.keyword_flags),
        SourceProgramDefinitionRoot::SkillTypes => Some(owner.policy.skill_types),
        SourceProgramDefinitionRoot::GemIdLookup => owner
            .dictionaries
            .get(&ParserDictionary::GemIdLookup)
            .copied(),
        SourceProgramDefinitionRoot::Named(_) => None,
    }
}
// Private borrowed view allows the validator to share the existing parser data
// before its Arc is constructed; it avoids cloning either graph or program IR.
pub(crate) trait ProgramOwnerView {
    fn callback(&self, id: SourceCallbackId) -> Option<&SourceCallback>;
    fn factory(&self, id: SourceCallbackId) -> Option<&ParserFactoryDisposition>;
    fn source(&self) -> &ItemLoadingSource;
    fn has_definition(&self, root: SourceProgramDefinitionRoot) -> bool;
    fn intrinsic(&self, id: SourceCallbackId) -> Option<SourceProgramIntrinsic>;
    fn supports_dynamic_methods(&self) -> bool {
        false
    }
    fn supports_dynamic_calls(&self) -> bool {
        false
    }
}
impl ProgramOwnerView for SourceProgramOwner {
    fn supports_dynamic_calls(&self) -> bool {
        self.parser().is_none()
    }
    fn supports_dynamic_methods(&self) -> bool {
        self.parser().is_none()
    }
    fn callback(&self, id: SourceCallbackId) -> Option<&SourceCallback> {
        self.callback(id)
    }
    fn factory(&self, id: SourceCallbackId) -> Option<&ParserFactoryDisposition> {
        self.factory(id)
    }
    fn source(&self) -> &ItemLoadingSource {
        self.source()
    }
    fn has_definition(&self, root: SourceProgramDefinitionRoot) -> bool {
        self.definition_id(root).is_some()
    }
    fn intrinsic(&self, id: SourceCallbackId) -> Option<SourceProgramIntrinsic> {
        self.intrinsic(id)
    }
}
impl ProgramOwnerView for ModifierParserData {
    fn callback(&self, id: SourceCallbackId) -> Option<&SourceCallback> {
        id.0.checked_sub(1)
            .and_then(|i| self.callbacks.get(i as usize))
    }
    fn factory(&self, id: SourceCallbackId) -> Option<&ParserFactoryDisposition> {
        self.factories.get(&id)
    }
    fn source(&self) -> &ItemLoadingSource {
        &self.source
    }
    fn has_definition(&self, root: SourceProgramDefinitionRoot) -> bool {
        parser_definition_id(self, root).is_some()
    }
    fn intrinsic(&self, _id: SourceCallbackId) -> Option<SourceProgramIntrinsic> {
        None
    }
}
fn failure(kind: SourceProgramErrorKind, message: impl Into<String>) -> SourceProgramError {
    SourceProgramError {
        kind,
        program: None,
        location: None,
        message: message.into(),
    }
}
fn graph_error(error: graph::GraphError) -> SourceProgramError {
    failure(error.kind, error.message)
}
