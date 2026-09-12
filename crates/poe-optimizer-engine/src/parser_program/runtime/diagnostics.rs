//! Bounded, opt-in failure witnesses. These records never grant execution capability.
use super::{
    ProgramRuntimeError as Error, RuntimeResult as Result,
    session::SessionValue,
    value::{Heap, V},
};
use crate::lua_pattern::MatchBudget;
use poe_optimizer_data::{
    modifier_parser::{ParserCallbackId, ParserProgramLocation},
    source_program::SourceProgramOwner,
};
use std::sync::Arc;

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
