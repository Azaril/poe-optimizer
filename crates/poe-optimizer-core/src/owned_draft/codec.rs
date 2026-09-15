//! Bounded draft persistence, separate from complete owned input documents.
use super::*;
use serde::{Deserialize, Serialize};
use std::{fmt, io};
pub const OWNED_DRAFT_SCHEMA_VERSION: u32 = 4;
#[derive(Debug)]
pub enum DraftCodecError {
    Structure(StructuralError),
    Json(serde_json::Error),
    UnsupportedVersion(u32),
    TooLarge { maximum: usize },
}
impl fmt::Display for DraftCodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Structure(e) => e.fmt(f),
            Self::Json(e) => e.fmt(f),
            Self::UnsupportedVersion(v) => write!(f, "unsupported owned draft schema version {v}"),
            Self::TooLarge { maximum } => write!(f, "owned draft exceeds {maximum} bytes"),
        }
    }
}
impl std::error::Error for DraftCodecError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Structure(e) => Some(e),
            Self::Json(e) => Some(e),
            _ => None,
        }
    }
}
impl From<StructuralError> for DraftCodecError {
    fn from(e: StructuralError) -> Self {
        Self::Structure(e)
    }
}
impl From<serde_json::Error> for DraftCodecError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireInput<T> {
    schema_version: u32,
    draft: T,
}
#[derive(Serialize)]
struct WireOutput<'a> {
    schema_version: u32,
    draft: &'a DraftSession,
}
/// Preserve exact typed pending values and authoring order. Deserialization alone
/// cannot manufacture a validated session; the same constructor is used by editors.
pub fn decode_draft(bytes: &[u8], limits: DraftLimits) -> Result<DraftSession, DraftCodecError> {
    limits.validate()?;
    if bytes.len() > limits.input.max_wire_bytes {
        return Err(DraftCodecError::TooLarge {
            maximum: limits.input.max_wire_bytes,
        });
    }
    // Read a strict envelope first so old payload shapes fail with their actual
    // version, even if required current-version fields were absent in that schema.
    let wire: WireInput<serde::de::IgnoredAny> = serde_json::from_slice(bytes)?;
    if wire.schema_version != OWNED_DRAFT_SCHEMA_VERSION {
        return Err(DraftCodecError::UnsupportedVersion(wire.schema_version));
    }
    let wire: WireInput<DraftSessionInput> = serde_json::from_slice(bytes)?;
    Ok(DraftSession::new(wire.draft, limits)?)
}
/// Deterministic persistence of the ordered draft; not semantic canonicalization.
pub fn encode_draft(draft: &DraftSession, limits: DraftLimits) -> Result<Vec<u8>, DraftCodecError> {
    draft.validate_limits(limits)?;
    let mut writer = BoundedWriter {
        bytes: Vec::new(),
        maximum: limits.input.max_wire_bytes,
        exceeded: false,
    };
    let result = serde_json::to_writer(
        &mut writer,
        &WireOutput {
            schema_version: OWNED_DRAFT_SCHEMA_VERSION,
            draft,
        },
    );
    if writer.exceeded {
        return Err(DraftCodecError::TooLarge {
            maximum: writer.maximum,
        });
    }
    result?;
    Ok(writer.bytes)
}
struct BoundedWriter {
    bytes: Vec<u8>,
    maximum: usize,
    exceeded: bool,
}
impl io::Write for BoundedWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.maximum - self.bytes.len() {
            self.exceeded = true;
            return Err(io::Error::other("owned draft byte limit exceeded"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
