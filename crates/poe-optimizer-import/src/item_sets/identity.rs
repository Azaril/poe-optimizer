//! Persistent identity of one published item-set row in its exact producer.
//! Values are native ownership tokens, never serialized graph or authored IDs.
use super::{Id, ItemSetState};
use std::{
    fmt,
    hash::{Hash, Hasher},
    sync::Arc,
};

#[derive(Clone)]
pub struct ItemSetIdentity {
    owner: Arc<()>,
    row: Id,
}
impl ItemSetIdentity {
    pub(super) fn for_row(state: &ItemSetState, row: Id) -> Self {
        Self {
            owner: Arc::clone(&state.identity_owner),
            row,
        }
    }
    /// Identity survives moving or restarting this state, but cannot authorize
    /// another producer with equal data or equal numeric set keys.
    pub fn belongs_to(&self, state: &ItemSetState) -> bool {
        Arc::ptr_eq(&self.owner, &state.identity_owner)
    }
    pub fn same_identity(&self, other: &Self) -> bool {
        self == other
    }
}
impl PartialEq for ItemSetIdentity {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.row == other.row
    }
}
impl Eq for ItemSetIdentity {}
impl Hash for ItemSetIdentity {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(Arc::as_ptr(&self.owner), state);
        self.row.0.hash(state);
    }
}
impl fmt::Debug for ItemSetIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ItemSetIdentity").finish_non_exhaustive()
    }
}
impl ItemSetState {
    /// Number of successful row publications, including constructor/default rows.
    /// Restarts retain this monotonic count and all earlier row identities.
    pub fn creation_count(&self) -> u64 {
        self.creation_count
    }
    /// The latest published row, even when its enclosing source operation fails
    /// later. Clone the token before advancing or moving the producer.
    pub fn last_created_set(&self) -> Option<&ItemSetIdentity> {
        self.last_created_set.as_ref()
    }
}
