//! Offline, source-authenticated typed-program extraction for parity/development.
//!
//! This optional PoB adapter executes pinned Lua only while extracting data. The
//! resulting catalog executes through the independent native engine. As with the
//! game-data extractor, callers supervise offline source construction when a hard
//! wall-clock deadline is needed; VM instruction/memory caps are not that deadline.
//! A lowered program is not a claim of proved source parity or public admission.
use crate::game_data::{GameDataExtractionError, error, hash};
use poe_optimizer_data::modifier_parser::{
    ModifierParserCatalog, ParserCallbackId, ParserProgramCatalog,
};
use std::{collections::BTreeMap, path::Path};

type Result<T> = std::result::Result<T, GameDataExtractionError>;
const MAX_SOURCE_BYTES: usize = 256 * 1024 * 1024;
const MAX_SOURCE_FILES: usize = 4096;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
pub struct ParserProgramExtraction {
    catalog: ParserProgramCatalog,
    unsupported: BTreeMap<ParserCallbackId, String>,
    implementation_sha256: String,
}
impl ParserProgramExtraction {
    pub(crate) fn new(
        catalog: ParserProgramCatalog,
        unsupported: BTreeMap<ParserCallbackId, String>,
    ) -> Self {
        Self {
            catalog,
            unsupported,
            implementation_sha256: crate::game_data::extractor_sha256(),
        }
    }
    /// Structurally valid programs retaining the exact caller-owned parser catalog.
    /// Whole-source parity and package/public-dispatch admission are separate gates.
    pub fn catalog(&self) -> &ParserProgramCatalog {
        &self.catalog
    }
    /// Non-program, non-legacy-Pure callbacks and their complete-lowering failures.
    pub fn unsupported(&self) -> &BTreeMap<ParserCallbackId, String> {
        &self.unsupported
    }
    /// Host-normalized implementation identity, including all lowering/authentication code.
    pub fn implementation_sha256(&self) -> &str {
        &self.implementation_sha256
    }
}

/// Read exactly the owner's consumed source inventory after verifying the pinned
/// installation. No item/build selection or runtime program template is supplied here.
pub fn extract_pinned(
    root: &Path,
    owner: &ModifierParserCatalog,
) -> Result<ParserProgramExtraction> {
    crate::source::verify(root)?;
    let mut sources = BTreeMap::new();
    let mut total = 0usize;
    for path in owner.data().source.files.keys() {
        let text = crate::source::read_verified_text(root, path)?;
        total = total
            .checked_add(text.len())
            .ok_or_else(|| error("program source size overflow"))?;
        if total > MAX_SOURCE_BYTES {
            return Err(error("program source byte bound"));
        }
        sources.insert(path.clone(), text);
    }
    extract_from_sources(&sources, owner)
}

/// Extract from already LF-normalized pinned sources. Every supplied file and every
/// declared dependency is checked before Lua runs. Fresh complete parser extraction
/// must then match the owner byte-for-byte, including captures, lookup definitions,
/// legacy recipes and source descriptors. Caller-authored program/data experiments
/// can instead construct a separate validated catalog after this trusted extraction.
pub fn extract_from_sources(
    sources: &BTreeMap<String, String>,
    owner: &ModifierParserCatalog,
) -> Result<ParserProgramExtraction> {
    validate_sources(sources, owner)?;
    crate::modifier_parser_extract::extract_programs(sources, owner)
}
fn validate_sources(
    sources: &BTreeMap<String, String>,
    owner: &ModifierParserCatalog,
) -> Result<()> {
    if sources.len() > MAX_SOURCE_FILES {
        return Err(error("program source file bound"));
    }
    if owner.data().source.upstream_revision != crate::source::UPSTREAM_REVISION {
        return Err(error("program owner source revision mismatch"));
    }
    let mut total = 0usize;
    for (path, text) in sources {
        total = total
            .checked_add(text.len())
            .ok_or_else(|| error("program source size overflow"))?;
        if total > MAX_SOURCE_BYTES {
            return Err(error("program source byte bound"));
        }
        if hash(text.as_bytes()) != crate::source::expected_file_sha256(path)? {
            return Err(error(format!(
                "program source is not the authenticated normalized file: {path}"
            )));
        }
    }
    for (path, expected) in &owner.data().source.files {
        let text = sources
            .get(path)
            .ok_or_else(|| error(format!("missing program source dependency: {path}")))?;
        if hash(text.as_bytes()) != *expected {
            return Err(error(format!("program owner source hash mismatch: {path}")));
        }
    }
    Ok(())
}
