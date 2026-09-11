//! Private native function instances. Compiled owners never retain mutable state.
use super::{Error, Heap, Result, V};
use crate::{lua_pattern::MatchBudget, parser_program::runtime::intrinsics::Gmatch};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::parser_program::runtime) struct IntrinsicClosureRef(u32);

pub(super) enum IntrinsicClosure {
    Gmatch(Gmatch),
}

impl Heap<'_> {
    pub(in crate::parser_program::runtime) fn new_gmatch(&mut self, state: Gmatch) -> Result<V> {
        let id = u32::try_from(self.intrinsic_closures.len())
            .map_err(|_| Error::resource("intrinsic closure identity"))?;
        // Account for the function and its three captured values independently
        // of the already charged subject conversion and compiled pattern bytes.
        self.charge_values(4)?;
        self.charge_bytes(std::mem::size_of::<Option<IntrinsicClosure>>())?;
        self.intrinsic_closures
            .push(Some(IntrinsicClosure::Gmatch(state)));
        Ok(V::IntrinsicClosure(IntrinsicClosureRef(id)))
    }

    pub(in crate::parser_program::runtime) fn invoke_intrinsic_closure(
        &mut self,
        reference: IntrinsicClosureRef,
        patterns: &mut MatchBudget,
    ) -> Result<Vec<V>> {
        let index = reference.0 as usize;
        let mut closure = self
            .intrinsic_closures
            .get_mut(index)
            .and_then(Option::take)
            .ok_or_else(|| Error::input("intrinsic closure identity is unavailable"))?;
        // Matching has no callbacks and cannot reenter this heap. Moving out the
        // state avoids cloning its buffers while result allocation borrows Heap.
        // Restore it before propagating every source or resource error.
        let result = match &mut closure {
            IntrinsicClosure::Gmatch(state) => {
                state.next(self, patterns).map(Option::unwrap_or_default)
            }
        };
        self.intrinsic_closures[index] = Some(closure);
        result
    }
}
