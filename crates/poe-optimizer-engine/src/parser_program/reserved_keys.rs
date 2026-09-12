//! Immutable, source-bound TDUP reserved-key order. No physical hash layout.
use crate::lua_pattern::{MatchBudget, PatternError};
use std::{cmp::Ordering, sync::Arc};

#[derive(Debug)]
pub(super) struct CompiledReservedKeys {
    order: Box<[Arc<[u8]>]>,
    positions: Box<[usize]>,
}
impl CompiledReservedKeys {
    /// The catalog has already bounded and validated unique template keys.
    pub(super) fn new(keys: &[Vec<u8>]) -> Self {
        let order: Box<[Arc<[u8]>]> = keys.iter().map(|key| Arc::from(key.as_slice())).collect();
        let mut positions: Vec<_> = (0..order.len()).collect();
        positions.sort_unstable_by(|a, b| order[*a].cmp(&order[*b]));
        Self {
            order,
            positions: positions.into_boxed_slice(),
        }
    }
    pub(super) fn len(&self) -> usize {
        self.order.len()
    }
    pub(super) fn key(&self, position: usize) -> &Arc<[u8]> {
        &self.order[position]
    }
    /// Search positions rather than sorting the observed order or cloning bytes.
    pub(super) fn find(
        &self,
        key: &[u8],
        work: &mut MatchBudget,
    ) -> Result<Option<usize>, PatternError> {
        work.charge(1)?;
        let mut low = 0;
        let mut high = self.positions.len();
        while low < high {
            work.charge(1)?;
            let middle = low + (high - low) / 2;
            let position = self.positions[middle];
            let candidate = self.order[position].as_ref();
            work.charge(candidate.len().min(key.len()) as u64)?;
            match candidate.cmp(key) {
                Ordering::Less => low = middle + 1,
                Ordering::Greater => high = middle,
                Ordering::Equal => return Ok(Some(position)),
            }
        }
        Ok(None)
    }
}
