//! Compatibility facade for portable tree projection plus supervised PoB extraction.
use crate::tree_data::TreeDataSnapshot;
pub use poe_optimizer_data::tree_projection::{
    AuthenticationProvenance, ELIGIBILITY_POLICY, TREE_PROJECTION_SCHEMA, TreeProjection,
    TreeProjectionCoverage, TreeProjectionError,
};
use std::{ops::Deref, path::Path, time::Duration};
/// Keeps the former extraction constructor while borrowing the portable authenticated model.
#[derive(Clone, Debug)]
pub struct AuthenticatedTreeSnapshot(
    poe_optimizer_data::tree_projection::AuthenticatedTreeSnapshot,
);
impl AuthenticatedTreeSnapshot {
    /// The executable must be our trusted CLI. Source verification, artifact bounds,
    /// private paths and deadline supervision happen before this authentication boundary.
    pub fn extract(
        executable: &Path,
        pob: &Path,
        timeout: Duration,
    ) -> Result<Self, TreeProjectionError> {
        let snapshot = crate::tree_worker::extract_tree(
            executable,
            pob,
            crate::tree_data::SUPPORTED_TREE_VERSION,
            timeout,
        )
        .map_err(|error| TreeProjectionError::Extraction(error.to_string()))?;
        let digest = snapshot
            .sha256()
            .map_err(|error| TreeProjectionError::Authentication(error.to_string()))?;
        poe_optimizer_data::tree_projection::AuthenticatedTreeSnapshot::from_trusted_extraction(
            snapshot, &digest,
        )
        .map(Self)
    }
    pub fn from_trusted_digest(
        snapshot: TreeDataSnapshot,
        expected_sha256: &str,
    ) -> Result<Self, TreeProjectionError> {
        poe_optimizer_data::tree_projection::AuthenticatedTreeSnapshot::from_trusted_digest(
            snapshot,
            expected_sha256,
        )
        .map(Self)
    }
    pub fn as_portable(&self) -> &poe_optimizer_data::tree_projection::AuthenticatedTreeSnapshot {
        &self.0
    }
}
impl Deref for AuthenticatedTreeSnapshot {
    type Target = poe_optimizer_data::tree_projection::AuthenticatedTreeSnapshot;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
