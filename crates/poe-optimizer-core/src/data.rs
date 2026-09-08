//! Lightweight data-content identity shared across evaluator contracts.
use serde::{Deserialize, Serialize};

/// Verified content identity; provenance and trust are separate evidence.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DataIdentity {
    pub game: String,
    pub release: String,
    pub schema_version: u32,
    pub content_sha256: String,
    pub semantics_version: String,
}
impl DataIdentity {
    /// Validate a recorded identity's shape without claiming provenance or trust.
    pub fn validate(&self) -> Result<(), String> {
        for (name, value) in [
            ("game", &self.game),
            ("release", &self.release),
            ("semantics_version", &self.semantics_version),
        ] {
            if value.trim().is_empty() || value.len() > 4096 || value.chars().any(char::is_control)
            {
                return Err(format!(
                    "Data identity {name} must be a nonempty bounded label without control characters"
                ));
            }
        }
        if self.schema_version == 0 {
            return Err("Data identity schema_version must be positive".into());
        }
        if self.content_sha256.len() != 64
            || !self
                .content_sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(
                "Data identity content_sha256 must be a lowercase SHA-256 hex digest".into(),
            );
        }
        Ok(())
    }
}
