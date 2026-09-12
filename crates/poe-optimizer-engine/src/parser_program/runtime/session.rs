//! One build's persistent state on the shared source VM.
use super::{
    ProgramAllocationUsage, ProgramLimits, ProgramRuntimeError as Error, ProgramTableCoverage,
    ProgramValueGraph, RuntimeResult as Result, SourceProgramOutput,
    execute::Run,
    value::{Heap, V},
};
use crate::{lua_pattern::MatchBudget, parser_program::CompiledSourcePrograms};
use poe_optimizer_data::{
    modifier_parser::ParserCallbackId,
    source_program::{
        SourceClassHandle, SourceProgramOwner, SourceProgramRootHandle, SourceSessionInput,
    },
};
use std::sync::Arc;

/// An opaque value in exactly one session. Cloning preserves Lua table aliases.
/// A handle cannot be constructed from raw table IDs or passed to another session.
#[derive(Debug, Clone)]
pub struct SessionValue {
    identity: Arc<()>,
    value: V,
}

/// Persistent writable state for one build, with cumulative work and allocation
/// limits across every callback, borrowed import and snapshot (including errors).
/// Definitions and borrowed argument tables remain immutable. Source errors can
/// leave prior writes in place, as in Lua; the session does not imply rollback.
/// No mutable state is stored in the shareable compiled library.
pub struct ProgramSession {
    library: CompiledSourcePrograms,
    heap: Heap<'static>,
    patterns: MatchBudget,
    limits: ProgramLimits,
    steps: u64,
    identity: Arc<()>,
}
impl CompiledSourcePrograms {
    /// Instantiate a coherent owner-bound graph of state, class associations,
    /// closures and shared capture cells. Only the importing domain authenticates its source origin.
    pub fn session_from_input(
        &self,
        input: &SourceSessionInput,
        limits: ProgramLimits,
    ) -> Result<(ProgramSession, Vec<SessionValue>)> {
        let (mut session, _) = self.session(&ProgramValueGraph::default(), limits)?;
        let roots = session.import_session_input(input)?;
        Ok((session, roots))
    }
    /// Explicitly import a writable initial graph. This is separate from the
    /// standalone/parser executor, whose input tables are always borrowed.
    pub fn session(
        &self,
        state: &ProgramValueGraph,
        limits: ProgramLimits,
    ) -> Result<(ProgramSession, Vec<SessionValue>)> {
        self.session_with_coverage(state, &ProgramTableCoverage::new(), limits)
    }
    /// Import writable state with explicit table coverage. Unavailable fields
    /// remain dependency errors; the plain graph must never imply their absence.
    pub fn session_with_coverage(
        &self,
        state: &ProgramValueGraph,
        coverage: &ProgramTableCoverage,
        limits: ProgramLimits,
    ) -> Result<(ProgramSession, Vec<SessionValue>)> {
        let mut session = ProgramSession {
            library: self.clone(),
            heap: Heap::owned(self.catalog().owner(), limits),
            patterns: MatchBudget::new(limits.pattern),
            limits,
            steps: 0,
            identity: Arc::new(()),
        };
        session.check_pack(state.values.len())?;
        let values = session.heap.import_with_coverage(state, coverage, true)?;
        let handles = session.handles(values)?;
        Ok((session, handles))
    }
}
impl ProgramSession {
    pub fn owner(&self) -> &SourceProgramOwner {
        self.library.catalog().owner()
    }
    pub fn is_bound_to(&self, owner: &SourceProgramOwner) -> bool {
        self.owner().is_same_owner(owner)
    }
    pub fn steps(&self) -> u64 {
        self.steps
    }
    pub fn pattern_steps(&self) -> u64 {
        self.patterns.steps_used()
    }
    pub fn allocations(&self) -> ProgramAllocationUsage {
        let used = self.heap.stats();
        ProgramAllocationUsage {
            values: used.values,
            bytes: used.bytes,
            tables: used.tables,
        }
    }
    /// Import additional immutable argument tables. Existing session aliases and
    /// identities are retained; numerical graph IDs are local to this import.
    pub fn borrow(&mut self, input: &ProgramValueGraph) -> Result<Vec<SessionValue>> {
        self.borrow_with_coverage(input, &ProgramTableCoverage::new())
    }
    /// Import an immutable graph with coverage scoped to this import's table IDs.
    /// This never widens previously imported tables or changes their identities.
    pub fn borrow_with_coverage(
        &mut self,
        input: &ProgramValueGraph,
        coverage: &ProgramTableCoverage,
    ) -> Result<Vec<SessionValue>> {
        self.check_pack(input.values.len())?;
        let values = self.heap.import_with_coverage(input, coverage, false)?;
        self.handles(values)
    }
    /// Import additional writable state produced at an explicit lifecycle step.
    /// All table IDs are local to the supplied graph. Reuse returned handles for
    /// subsequent calls to retain aliases with the existing live session.
    pub fn import_with_coverage(
        &mut self,
        input: &ProgramValueGraph,
        coverage: &ProgramTableCoverage,
    ) -> Result<Vec<SessionValue>> {
        self.check_pack(input.values.len())?;
        let values = self.heap.import_with_coverage(input, coverage, true)?;
        self.handles(values)
    }
    /// Import a whole additional class/closure/state observation. All graph, closure
    /// and capture-cell IDs are local to this artifact; returned opaque handles
    /// preserve its identities across subsequent calls in this session.
    pub fn import_session_input(
        &mut self,
        input: &SourceSessionInput,
    ) -> Result<Vec<SessionValue>> {
        self.check_pack(input.state.values.len())?;
        let values = self.heap.import_session_input(input)?;
        self.handles(values)
    }
    /// Return an immutable definition root from this session's retained owner.
    pub fn definition(&mut self, root: &SourceProgramRootHandle) -> Result<SessionValue> {
        self.owner()
            .resolve_root(root)
            .map_err(|error| Error::input(error.to_string()))?;
        let id = root.table_id();
        self.heap.charge_values(1)?;
        Ok(SessionValue {
            identity: self.identity.clone(),
            value: self.heap.definition(id)?,
        })
    }
    /// Allocate source-bound class state, including Object alias and callable
    /// parent proxies. This does not execute or imply completion of its constructor.
    pub fn allocate_instance(&mut self, class: &SourceClassHandle) -> Result<SessionValue> {
        self.owner()
            .resolve_class(class)
            .map_err(|error| Error::input(error.to_string()))?;
        let value = self
            .heap
            .allocate_instance(class.id(), &mut self.patterns)?;
        self.heap.charge_values(1)?;
        Ok(SessionValue {
            identity: self.identity.clone(),
            value,
        })
    }
    /// Execute another source callback on the same heap. Callback IDs resolve
    /// only in this session's compiled owner, never in a caller-supplied catalog.
    pub fn invoke(
        &mut self,
        callback: ParserCallbackId,
        input: &[SessionValue],
    ) -> Result<Vec<SessionValue>> {
        let index = *self
            .library
            .0
            .callbacks
            .get(&callback)
            .ok_or_else(|| Error::input("callback has no compiled program"))?;
        let arguments = self.values(input)?;
        let mut run = Run {
            library: &self.library,
            heap: &mut self.heap,
            patterns: &mut self.patterns,
            limits: self.limits,
            steps: &mut self.steps,
            call_depth: 0,
        };
        let values = run.invoke(index, arguments, 0)?;
        self.handles(values)
    }
    /// Invoke an already resolved source value without an implicit self argument.
    /// Callable handles and arguments must belong to this live session.
    pub fn invoke_callable(
        &mut self,
        callable: &SessionValue,
        input: &[SessionValue],
    ) -> Result<Vec<SessionValue>> {
        let target = self.values(std::slice::from_ref(callable))?.remove(0);
        let arguments = self.values(input)?;
        let mut run = Run {
            library: &self.library,
            heap: &mut self.heap,
            patterns: &mut self.patterns,
            limits: self.limits,
            steps: &mut self.steps,
            call_depth: 0,
        };
        let values = run.invoke_value(target, arguments, 0)?;
        self.handles(values)
    }
    /// Invoke a source method with implicit self on an owner/session-bound value.
    /// Lookup occurs before invocation; raw fields may override injected methods.
    pub fn invoke_method(
        &mut self,
        receiver: &SessionValue,
        name: &str,
        input: &[SessionValue],
    ) -> Result<Vec<SessionValue>> {
        let receiver = self.values(std::slice::from_ref(receiver))?.remove(0);
        let mut arguments = self.values(input)?;
        let mut run = Run {
            library: &self.library,
            heap: &mut self.heap,
            patterns: &mut self.patterns,
            limits: self.limits,
            steps: &mut self.steps,
            call_depth: 0,
        };
        let target = run.lookup_method(&receiver, name.as_bytes())?;
        run.prepend(&mut arguments, receiver)?;
        let values = run.invoke_method_target(target, arguments, 0)?;
        self.handles(values)
    }
    /// Export selected reachable values without ending the session. The returned
    /// graph is a snapshot; it retains the real definition owner and cumulative
    /// counters, and cannot act as a writable handle into the live session.
    pub fn snapshot(&mut self, values: &[SessionValue]) -> Result<SourceProgramOutput> {
        let values = self.values(values)?;
        let graph = self.heap.freeze(&values)?;
        Ok(SourceProgramOutput {
            graph,
            owner: self.owner().clone(),
            steps: self.steps(),
            pattern_steps: self.pattern_steps(),
            allocations: self.allocations(),
        })
    }
    fn check_pack(&self, count: usize) -> Result<()> {
        if count > self.limits.max_results {
            return Err(Error::resource("session value pack"));
        }
        Ok(())
    }
    fn values(&mut self, values: &[SessionValue]) -> Result<Vec<V>> {
        self.check_pack(values.len())?;
        for value in values {
            if !Arc::ptr_eq(&self.identity, &value.identity) {
                return Err(Error::input("value handle belongs to another session"));
            }
        }
        self.heap.charge_values(values.len())?;
        Ok(values.iter().map(|v| v.value.clone()).collect())
    }
    fn handles(&mut self, values: Vec<V>) -> Result<Vec<SessionValue>> {
        self.check_pack(values.len())?;
        self.heap.charge_values(values.len())?;
        Ok(values
            .into_iter()
            .map(|value| SessionValue {
                identity: self.identity.clone(),
                value,
            })
            .collect())
    }
}
