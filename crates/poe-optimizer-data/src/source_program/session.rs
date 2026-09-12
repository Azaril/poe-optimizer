//! Domain-neutral runtime input graphs. Engine aliases retain the existing API.
//!
//! Source observation authenticates one coherent instance/cell/state artifact.
//! These types confer no source proof, mutation admission or numerical coverage.
use super::{
    SourceCallbackId, SourceClassHandle, SourceClosurePrototypeHandle, SourceProgramErrorKind,
    SourceProgramOwner, SourceProgramResult, SourceTableCoverage, SourceTableId,
    SourceTableIndexFallback, failure,
};
use std::collections::BTreeMap;
mod traversal;
pub use traversal::*;

/// One-based table reference in a single input/output graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceSessionTableId(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceSessionClosureId(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceSessionCellId(pub u32);
/// Raw Lua values. Numbers retain IEEE bits; strings need not be UTF-8.
/// Callback IDs refer to the retained owner. The last two forms are accepted
/// only inside a coherent owner-bound SourceSessionInput, never plain imports.
#[derive(Debug, Clone, PartialEq)]
pub enum SourceSessionValue {
    Nil,
    Boolean(bool),
    Number(f64),
    Bytes(Vec<u8>),
    Table(SourceSessionTableId),
    Callback(SourceCallbackId),
    Closure(SourceSessionClosureId),
    DefinitionTable(SourceTableId),
}
/// Unordered Lua entries, including arbitrary scalar/table/function keys. Import
/// rejects nil values, nil/NaN keys and duplicates after Lua normalization.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SourceSessionTable {
    pub entries: Vec<(SourceSessionValue, SourceSessionValue)>,
}
/// Existing raw argument/result transport. All table IDs are graph-local.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SourceSessionValueGraph {
    pub values: Vec<SourceSessionValue>,
    pub tables: Vec<SourceSessionTable>,
}
pub type SourceSessionCoverage = BTreeMap<SourceSessionTableId, SourceTableCoverage>;
/// Actual class metatables attached to private state tables, without replaying
/// constructors or copying inherited members into raw instance fields. Handles
/// retain immutable owner data; this map contains no mutable class definitions.
///
/// This does not admit session-owned class methods. Selected shared methods need
/// complete definition-owned captures; live-captured class members remain an
/// unsupported frontier. The observing domain authenticates that distinction,
/// including shared upvalue-cell identities, rather than inferring immutability.
pub type SourceSessionClassBindings = BTreeMap<SourceSessionTableId, SourceClassHandle>;
/// A distinct runtime function instance. Reusing a prototype does not alias
/// closures; reusing a cell ID does alias its upvalue storage, even across them.
#[derive(Debug, Clone)]
pub struct SourceSessionClosure {
    pub prototype: SourceClosurePrototypeHandle,
    pub captures: Vec<SourceSessionCellId>,
}
/// All identities belong to this one artifact. Table/cell/closure cycles can be
/// declared before any initial values are resolved. Hosts authenticate actual
/// closure and upvalue-cell identities, never infer sharing from equal values.
/// State tables are private writable state; DefinitionTable refers to shared
/// immutable data. Prototype handles and even an empty instance list retain the
/// explicit owner so raw definition IDs cannot be rebound to another library.
#[derive(Debug, Clone)]
pub struct SourceSessionInput {
    pub owner: SourceProgramOwner,
    pub state: SourceSessionValueGraph,
    pub coverage: SourceSessionCoverage,
    /// Optional source-observed raw layout facts, private to this input.
    pub traversal: Option<SourceSessionTraversal>,
    pub class_bindings: SourceSessionClassBindings,
    pub cells: Vec<SourceSessionValue>,
    pub closures: Vec<SourceSessionClosure>,
}

impl SourceSessionInput {
    /// Validate bounded, owner-bound class/coverage associations without copying
    /// values or constructing a second definition graph. `max_tables` is the
    /// importing engine's available table budget, not a source-data preset.
    ///
    /// The engine separately verifies this input's owner against its library,
    /// all raw graph entries, and the admitted class metatable protocol. This
    /// association does not prove source construction or numerical admission.
    pub fn validate_class_bindings(&self, max_tables: usize) -> SourceProgramResult<()> {
        if self.state.tables.len() > max_tables
            || self.coverage.len() > max_tables
            || self.class_bindings.len() > max_tables
        {
            return Err(failure(
                SourceProgramErrorKind::ResourceLimit,
                "session class association table bound",
            ));
        }
        let valid_table = |id: SourceSessionTableId| {
            id.0.checked_sub(1)
                .is_some_and(|index| (index as usize) < self.state.tables.len())
        };
        for (id, class) in &self.class_bindings {
            self.owner.resolve_class(class)?;
            if !valid_table(*id) {
                return Err(failure(
                    SourceProgramErrorKind::Binding,
                    "session class binding has no local state table",
                ));
            }
            if self
                .coverage
                .get(id)
                .map(|coverage| coverage.index_fallback)
                != Some(SourceTableIndexFallback::ClassResolved)
            {
                return Err(failure(
                    SourceProgramErrorKind::Binding,
                    "session class binding lacks resolved-class index coverage",
                ));
            }
        }
        for (id, coverage) in &self.coverage {
            if !valid_table(*id) {
                return Err(failure(
                    SourceProgramErrorKind::Binding,
                    "session coverage has no local state table",
                ));
            }
            if coverage.index_fallback == SourceTableIndexFallback::ClassResolved
                && !self.class_bindings.contains_key(id)
            {
                return Err(failure(
                    SourceProgramErrorKind::Binding,
                    "resolved-class index coverage lacks a session class binding",
                ));
            }
        }
        Ok(())
    }
}
