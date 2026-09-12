//! Bounded, opt-in failure witnesses. These records never grant execution capability.
use super::{
    ProgramRuntimeError as Error, RuntimeResult as Result,
    session::SessionValue,
    value::{Heap, TableRef, V},
};
use crate::{lua_pattern::MatchBudget, parser_program::CompiledSourcePrograms};
use poe_optimizer_data::{
    modifier_parser::{ParserCallbackId, ParserProgramLocation},
    source_program::{SourceProgramCatalog, SourceProgramOwner},
};
use std::sync::Arc;

/// Bounds retained native table-allocation records across invocations while enabled.
/// Capacity is charged and reserved at enable time, independently of traversal evidence.
#[derive(Debug, Clone, Copy)]
pub struct AllocationDiagnosticLimits {
    pub max_records: usize,
}
impl Default for AllocationDiagnosticLimits {
    fn default() -> Self {
        Self { max_records: 4096 }
    }
}

/// Diagnostic origin of one same-session table. Missing evidence never implies an
/// imported table, a particular constructor opcode, or a lost traversal proof.
#[derive(Debug, Clone)]
pub enum TableAllocationOrigin {
    /// Diagnostics were disabled, restarted, or did not observe this allocation.
    NotObserved,
    Expression(TableExpressionOrigin),
}

/// An executed native Table expression, bound to its exact compiled library and
/// allocated session table. This is not an original Lua allocation/producer witness
/// and grants no table-layout or traversal capability.
#[derive(Debug, Clone)]
pub struct TableExpressionOrigin {
    /// Allocation order within this enabled diagnostic interval. Not a source
    /// pointer, global identifier, or identity shared by other sessions/intervals.
    pub ordinal: u64,
    pub callback: ParserCallbackId,
    pub location: ParserProgramLocation,
    library: CompiledSourcePrograms,
    table: SessionValue,
}
impl TableExpressionOrigin {
    /// Exact catalog used by the native allocation, including its optional facets.
    pub fn catalog(&self) -> &SourceProgramCatalog {
        self.library.catalog()
    }
    /// Check the retained compiled library, not merely its definition owner.
    pub fn is_bound_to(&self, library: &CompiledSourcePrograms) -> bool {
        Arc::ptr_eq(&self.library.0, &library.0)
    }
    /// Retains the actual allocated table identity after diagnostics are disabled.
    pub fn table(&self) -> &SessionValue {
        &self.table
    }
}

#[derive(Clone, Copy)]
struct AllocationRecord {
    table: TableRef,
    callback: ParserCallbackId,
    location: ParserProgramLocation,
}

pub(super) struct AllocationDiagnostics {
    records: Vec<AllocationRecord>,
    limit: usize,
    library: CompiledSourcePrograms,
    identity: Arc<()>,
}
impl AllocationDiagnostics {
    pub(super) fn new(
        limits: AllocationDiagnosticLimits,
        library: &CompiledSourcePrograms,
        heap: &mut Heap,
        identity: Arc<()>,
    ) -> Result<Self> {
        if limits.max_records == 0 || limits.max_records > 1_000_000 {
            return Err(Error::input(
                "allocation diagnostic limits require 1..1000000 records",
            ));
        }
        let bytes = limits
            .max_records
            .checked_mul(std::mem::size_of::<AllocationRecord>())
            .ok_or_else(|| Error::resource("allocation diagnostic record storage"))?;
        heap.charge_values(limits.max_records)?;
        heap.charge_bytes(bytes)?;
        let mut records = Vec::new();
        records
            .try_reserve_exact(limits.max_records)
            .map_err(|_| Error::resource("allocation diagnostic record storage"))?;
        Ok(Self {
            records,
            limit: limits.max_records,
            library: library.clone(),
            identity,
        })
    }
    /// Runs before allocating the table, so a full diagnostic arena cannot allow
    /// field effects or publish an allocation without its promised origin record.
    pub(super) fn prepare(&self, work: &mut MatchBudget) -> Result<()> {
        if self.records.len() == self.limit {
            return Err(Error::resource("allocation diagnostic record bound"));
        }
        work.charge(1)?;
        Ok(())
    }
    /// Allocation succeeded; no allocation or fallible work remains before fields.
    pub(super) fn record(
        &mut self,
        table: &V,
        callback: ParserCallbackId,
        location: ParserProgramLocation,
    ) {
        let V::Table(table @ TableRef::Heap(_)) = table else {
            unreachable!("a Table expression allocated a heap table")
        };
        // Heap IDs are never reused. Imports may introduce gaps, and an outer
        // constructor is recorded before nested field expressions allocate.
        debug_assert!(self.records.last().is_none_or(|last| last.table < *table));
        debug_assert!(self.records.len() < self.limit);
        self.records.push(AllocationRecord {
            table: *table,
            callback,
            location,
        });
    }
    pub(super) fn origin(&self, table: TableRef) -> TableAllocationOrigin {
        let Ok(index) = self.records.binary_search_by_key(&table, |row| row.table) else {
            return TableAllocationOrigin::NotObserved;
        };
        let row = self.records[index];
        TableAllocationOrigin::Expression(TableExpressionOrigin {
            ordinal: index as u64 + 1,
            callback: row.callback,
            location: row.location,
            library: self.library.clone(),
            table: SessionValue::retained(self.identity.clone(), V::Table(table)),
        })
    }
}

/// Bounds enabled diagnostic storage and observed program activations per public
/// invocation. No completed-activation history is retained.
#[derive(Debug, Clone, Copy)]
pub struct TraversalDiagnosticLimits {
    pub max_frames: usize,
    pub max_activations: u64,
}
impl Default for TraversalDiagnosticLimits {
    fn default() -> Self {
        Self {
            max_frames: 64,
            max_activations: 100_000,
        }
    }
}
/// One active compiled source program. Ordinals are diagnostic identifiers local
/// to this native invocation, not source pointers or cross-runtime call IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraversalActivation {
    pub activation: u64,
    pub parent_activation: Option<u64>,
    pub callback: ParserCallbackId,
    pub location: Option<ParserProgramLocation>,
}
/// Exact arguments of the first unsupported Next reached in one invocation.
/// Handles keep their session identity after diagnostics are disabled or taken.
/// They reference live values, not frozen table snapshots; later writes remain visible.
#[derive(Debug)]
pub struct TraversalFailureWitness {
    pub table: SessionValue,
    pub control: SessionValue,
    pub callback: Option<ParserCallbackId>,
    pub location: Option<ParserProgramLocation>,
    pub frames: Vec<TraversalActivation>,
    owner: SourceProgramOwner,
}
impl TraversalFailureWitness {
    pub fn owner(&self) -> &SourceProgramOwner {
        &self.owner
    }
}
pub(super) struct TraversalDiagnostics {
    limits: TraversalDiagnosticLimits,
    frames: Vec<TraversalActivation>,
    activations: u64,
    pub(super) witness: Option<TraversalFailureWitness>,
    owner: SourceProgramOwner,
    identity: Arc<()>,
}
impl TraversalDiagnostics {
    pub(super) fn new(
        limits: TraversalDiagnosticLimits,
        heap: &mut Heap,
        identity: Arc<()>,
    ) -> Result<Self> {
        if limits.max_frames == 0
            || limits.max_frames > 128
            || limits.max_activations == 0
            || limits.max_activations > 1_000_000
        {
            return Err(Error::input(
                "traversal diagnostic limits require 1..128 frames and 1..1000000 activations",
            ));
        }
        heap.charge_values(limits.max_frames)?;
        heap.charge_bytes(
            limits
                .max_frames
                .checked_mul(std::mem::size_of::<TraversalActivation>())
                .ok_or_else(|| Error::resource("traversal diagnostic frame allocation"))?,
        )?;
        let mut frames = Vec::new();
        frames
            .try_reserve_exact(limits.max_frames)
            .map_err(|_| Error::resource("traversal diagnostic frame allocation"))?;
        Ok(Self {
            limits,
            frames,
            activations: 0,
            witness: None,
            owner: heap.owner().clone(),
            identity,
        })
    }
    pub(super) fn begin(&mut self) {
        self.frames.clear();
        self.activations = 0;
        self.witness = None;
    }
    pub(super) fn enter(
        &mut self,
        callback: ParserCallbackId,
        work: &mut MatchBudget,
    ) -> Result<()> {
        work.charge(1)?;
        if self.frames.len() >= self.limits.max_frames {
            return Err(Error::resource("traversal diagnostic active frame bound"));
        }
        self.activations = self
            .activations
            .checked_add(1)
            .filter(|n| *n <= self.limits.max_activations)
            .ok_or_else(|| Error::resource("traversal diagnostic activation bound"))?;
        self.frames.push(TraversalActivation {
            activation: self.activations,
            parent_activation: self.frames.last().map(|f| f.activation),
            callback,
            location: None,
        });
        Ok(())
    }
    pub(super) fn exit(&mut self) {
        self.frames.pop();
    }
    pub(super) fn location(
        &mut self,
        location: Option<ParserProgramLocation>,
    ) -> Option<ParserProgramLocation> {
        self.frames
            .last_mut()
            .and_then(|frame| std::mem::replace(&mut frame.location, location))
    }
    pub(super) fn capture(
        &mut self,
        table: &V,
        control: &V,
        heap: &mut Heap,
        work: &mut MatchBudget,
    ) -> Result<()> {
        if self.witness.is_some() {
            return Ok(());
        }
        let count = self.frames.len();
        work.charge(count as u64 + 2)?;
        // Charge both retained opaque handles and the active-frame snapshot before
        // allocation/publication. Failed admission remains a visible resource error.
        heap.charge_values(count + 2)?;
        heap.charge_bytes(
            count
                .checked_mul(std::mem::size_of::<TraversalActivation>())
                .ok_or_else(|| Error::resource("traversal diagnostic witness allocation"))?,
        )?;
        let mut frames = Vec::new();
        frames
            .try_reserve_exact(count)
            .map_err(|_| Error::resource("traversal diagnostic witness allocation"))?;
        frames.extend_from_slice(&self.frames);
        let active = frames.last().copied();
        self.witness = Some(TraversalFailureWitness {
            table: SessionValue::retained(self.identity.clone(), table.clone()),
            control: SessionValue::retained(self.identity.clone(), control.clone()),
            callback: active.map(|f| f.callback),
            location: active.and_then(|f| f.location),
            frames,
            owner: self.owner.clone(),
        });
        Ok(())
    }
}
