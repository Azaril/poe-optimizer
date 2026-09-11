//! Domain-neutral runtime input graphs. Engine aliases retain the existing API.
//!
//! Source observation authenticates one coherent instance/cell/state artifact.
//! These types confer no source proof, mutation admission or numerical coverage.
use super::{
    SourceCallbackId, SourceClosurePrototypeHandle, SourceProgramOwner, SourceTableCoverage,
    SourceTableId,
};
use std::collections::BTreeMap;

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
    pub cells: Vec<SourceSessionValue>,
    pub closures: Vec<SourceSessionClosure>,
}
