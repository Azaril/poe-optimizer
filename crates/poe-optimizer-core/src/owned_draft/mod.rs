//! Partial authored semantic inputs, independent of source formats and UI state.
//!
//! Draft validation checks known structure without treating uncertainty as empty
//! effects or defaults. Only explicit finalization can produce a complete request;
//! neither operation establishes definition coverage, legality or numerical parity.
mod codec;
mod finalize;
mod records;
mod session;
mod structure;

use crate::owned_build::StructuralError;
use crate::owned_content::{ContentDigestError, OwnedContentDigest, digest_owned};
pub use codec::{DraftCodecError, OWNED_DRAFT_SCHEMA_VERSION, decode_draft, encode_draft};
pub use finalize::{DraftFinalization, FinalizationError, FinalizedDraft};
pub use records::*;
use serde::Serialize;
pub use session::*;
pub use structure::{DraftIssue, DraftLimits, DraftValidation, validate_draft};

/// Immutable validated draft. Order and unresolved alternatives are preserved.
/// No Deserialize implementation can bypass the bounded constructor.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct DraftSession(DraftSessionInput);
impl DraftSession {
    pub fn new(input: DraftSessionInput, limits: DraftLimits) -> Result<Self, StructuralError> {
        validate_draft(&input, limits)?;
        Ok(Self(input))
    }
    pub fn input(&self) -> &DraftSessionInput {
        &self.0
    }
    pub fn into_input(self) -> DraftSessionInput {
        self.0
    }
    pub fn validate_limits(&self, limits: DraftLimits) -> Result<DraftValidation, StructuralError> {
        validate_draft(&self.0, limits)
    }
    /// Exact ordered authoring snapshot identity, not a numerical-plan cache key.
    /// Includes allocator watermark/revision and inactive/unresolved alternatives.
    pub fn digest(&self, max_bytes: usize) -> Result<OwnedContentDigest, ContentDigestError> {
        digest_owned("owned-draft-v4", self, max_bytes)
    }
}
