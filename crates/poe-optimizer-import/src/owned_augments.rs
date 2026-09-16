//! Finite offline socketed-augment acquisition, not executable game semantics.
//!
//! Source descriptions and selection metadata remain outside the evaluator.
//! Decoding establishes bounded structure and exact byte provenance only; it
//! does not authenticate source files, select sockets, or claim effect coverage.
use crate::owned_mapping::SourcePin;
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, io::Write};

pub const OWNED_AUGMENT_CATALOG_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AugmentCatalog {
    pub schema_version: u32,
    pub source: SourcePin,
    pub augments: Vec<AugmentDefinition>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AugmentDefinition {
    pub source_name: String,
    pub selectors: Vec<AugmentSelector>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AugmentSelector {
    pub source_selector: String,
    pub kind: AugmentKind,
    pub normal: Vec<AugmentLine>,
    #[serde(deserialize_with = "required_option")]
    pub bonded: Option<Vec<AugmentLine>>,
    pub metadata: AugmentMetadata,
    pub semantics: AugmentSemantics,
}
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AugmentKind {
    Rune,
    SoulCore,
    Idol,
    AbyssalEye,
    CongealedMist,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AugmentSemantics {
    /// Acquisition is complete for the reviewed source shape. Effects,
    /// eligibility, magnitude and Bonded activation have not been converted.
    Unconverted,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AugmentLine {
    pub text: String,
    /// Source ordering is fractional for real definitions. Absence is retained;
    /// any default belongs to a separately reviewed reconstruction policy.
    #[serde(deserialize_with = "required_option")]
    pub stat_order: Option<f64>,
}
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AugmentMetadata {
    #[serde(deserialize_with = "required_option")]
    pub local_mod: Option<bool>,
    #[serde(deserialize_with = "required_option")]
    pub level_requirement: Option<u32>,
    #[serde(deserialize_with = "required_option")]
    pub limit: Option<u32>,
    #[serde(deserialize_with = "required_option")]
    pub limit_id: Option<String>,
    #[serde(deserialize_with = "required_option")]
    pub socket_bound: Option<bool>,
    #[serde(deserialize_with = "required_option")]
    pub can_socket_in_chakra_slots: Option<bool>,
    #[serde(deserialize_with = "required_option")]
    pub can_socket_in_unique_items: Option<bool>,
    #[serde(deserialize_with = "required_option")]
    pub can_socket_in_jewellery: Option<bool>,
    #[serde(deserialize_with = "required_option")]
    pub can_socket_in_corrupted_sanctified: Option<bool>,
    #[serde(deserialize_with = "required_option")]
    pub trade_hashes: Option<Vec<AugmentTradeHash>>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AugmentTradeHash {
    pub hash: u64,
    pub lines: Vec<String>,
}
fn required_option<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    Option::<T>::deserialize(deserializer)
}

#[derive(Clone, Copy, Debug)]
pub struct AugmentCatalogLimits {
    pub max_catalog_bytes: usize,
    pub max_augments: usize,
    pub max_selectors: usize,
    pub max_lines: usize,
    pub max_trade_hashes: usize,
    pub max_text_bytes: usize,
    pub max_total_text_bytes: usize,
    pub max_work: usize,
}
impl Default for AugmentCatalogLimits {
    fn default() -> Self {
        Self {
            max_catalog_bytes: 4 * 1024 * 1024,
            max_augments: 4096,
            max_selectors: 16_384,
            max_lines: 65_536,
            max_trade_hashes: 65_536,
            max_text_bytes: 16 * 1024,
            max_total_text_bytes: 2 * 1024 * 1024,
            max_work: 4 * 1024 * 1024,
        }
    }
}
impl AugmentCatalogLimits {
    pub fn validate(self) -> Result<(), AugmentCatalogError> {
        let max = Self::default();
        for (name, value, ceiling) in [
            (
                "catalog bytes",
                self.max_catalog_bytes,
                max.max_catalog_bytes,
            ),
            ("augments", self.max_augments, max.max_augments),
            ("selectors", self.max_selectors, max.max_selectors),
            ("lines", self.max_lines, max.max_lines),
            ("trade hashes", self.max_trade_hashes, max.max_trade_hashes),
            ("text bytes", self.max_text_bytes, max.max_text_bytes),
            (
                "total text bytes",
                self.max_total_text_bytes,
                max.max_total_text_bytes,
            ),
            ("work", self.max_work, max.max_work),
        ] {
            if value == 0 || value > ceiling {
                return Err(AugmentCatalogError::Limit(name));
            }
        }
        Ok(())
    }
}
#[derive(Debug, thiserror::Error)]
pub enum AugmentCatalogError {
    #[error("augment catalog limit: {0}")]
    Limit(&'static str),
    #[error("invalid augment catalog: {0}")]
    Invalid(&'static str),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct AugmentCatalogCounts {
    pub augments: usize,
    pub selectors: usize,
    pub normal_lines: usize,
    pub bonded_lines: usize,
    pub trade_hashes: usize,
    pub trade_lines: usize,
}

/// Exact received bytes are private and cannot be replaced without decoding
/// again. This is acquisition provenance, never source authentication authority.
#[derive(Debug)]
pub struct AcquiredAugmentCatalog {
    catalog: AugmentCatalog,
    bytes: Vec<u8>,
    sha256: String,
    counts: AugmentCatalogCounts,
}
impl AcquiredAugmentCatalog {
    pub fn catalog(&self) -> &AugmentCatalog {
        &self.catalog
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
    pub fn counts(&self) -> AugmentCatalogCounts {
        self.counts
    }
}
pub fn decode_owned_augments(
    bytes: &[u8],
    limits: AugmentCatalogLimits,
) -> Result<AcquiredAugmentCatalog, AugmentCatalogError> {
    limits.validate()?;
    if bytes.len() > limits.max_catalog_bytes {
        return Err(AugmentCatalogError::Limit("catalog bytes"));
    }
    let catalog: AugmentCatalog = serde_json::from_slice(bytes)?;
    let counts = validate_owned_augments(&catalog, limits)?;
    Ok(AcquiredAugmentCatalog {
        catalog,
        bytes: bytes.to_vec(),
        sha256: format!("{:x}", Sha256::digest(bytes)),
        counts,
    })
}
pub fn validate_owned_augments(
    catalog: &AugmentCatalog,
    limits: AugmentCatalogLimits,
) -> Result<AugmentCatalogCounts, AugmentCatalogError> {
    limits.validate()?;
    if catalog.schema_version != OWNED_AUGMENT_CATALOG_VERSION {
        return Err(AugmentCatalogError::Invalid("version"));
    }
    let mut work = limits.max_work;
    let mut text_left = limits.max_total_text_bytes;
    let mut text = |s: &str, empty: bool| -> Result<(), AugmentCatalogError> {
        if (!empty && s.is_empty()) || s.contains('\0') {
            return Err(AugmentCatalogError::Invalid("empty or NUL text"));
        }
        if s.len() > limits.max_text_bytes {
            return Err(AugmentCatalogError::Limit("text bytes"));
        }
        text_left = text_left
            .checked_sub(s.len())
            .ok_or(AugmentCatalogError::Limit("total text bytes"))?;
        // Conservative comparison allowance for indexes bounded below.
        work = work
            .checked_sub(s.len().saturating_add(1).saturating_mul(16))
            .ok_or(AugmentCatalogError::Limit("work"))?;
        Ok(())
    };
    text(&catalog.source.revision, false)?;
    if catalog.source.files.is_empty() || catalog.source.files.len() > 64 {
        return Err(AugmentCatalogError::Invalid("source file membership"));
    }
    let mut paths = BTreeSet::new();
    for file in &catalog.source.files {
        text(&file.path, false)?;
        text(&file.sha256, false)?;
        if !paths.insert(&file.path)
            || file.path.starts_with('/')
            || file.path.contains('\\')
            || file
                .path
                .split('/')
                .any(|p| p.is_empty() || p == "." || p == ".." || p.contains(':'))
            || file.sha256.len() != 64
            || !file
                .sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(AugmentCatalogError::Invalid("source file pin"));
        }
    }
    if catalog.augments.is_empty() || catalog.augments.len() > limits.max_augments {
        return Err(AugmentCatalogError::Limit("augments"));
    }
    let mut counts = AugmentCatalogCounts {
        augments: catalog.augments.len(),
        selectors: 0,
        normal_lines: 0,
        bonded_lines: 0,
        trade_hashes: 0,
        trade_lines: 0,
    };
    let mut names = BTreeSet::new();
    for augment in &catalog.augments {
        text(&augment.source_name, false)?;
        if !names.insert(&augment.source_name) || augment.selectors.is_empty() {
            return Err(AugmentCatalogError::Invalid(
                "duplicate name or empty selectors",
            ));
        }
        counts.selectors = counts
            .selectors
            .checked_add(augment.selectors.len())
            .filter(|n| *n <= limits.max_selectors)
            .ok_or(AugmentCatalogError::Limit("selectors"))?;
        let mut selectors = BTreeSet::new();
        for row in &augment.selectors {
            text(&row.source_selector, false)?;
            if !selectors.insert(&row.source_selector) {
                return Err(AugmentCatalogError::Invalid("duplicate selector"));
            }
            counts.normal_lines += row.normal.len();
            counts.bonded_lines += row.bonded.as_ref().map_or(0, Vec::len);
            if counts.normal_lines.saturating_add(counts.bonded_lines) > limits.max_lines {
                return Err(AugmentCatalogError::Limit("lines"));
            }
            for line in row.normal.iter().chain(row.bonded.iter().flatten()) {
                text(&line.text, true)?;
                if line.stat_order.is_some_and(|n| !n.is_finite()) {
                    return Err(AugmentCatalogError::Invalid("nonfinite stat order"));
                }
            }
            if let Some(id) = &row.metadata.limit_id {
                text(id, false)?;
            }
            let mut hashes = BTreeSet::new();
            if let Some(groups) = &row.metadata.trade_hashes {
                counts.trade_hashes = counts
                    .trade_hashes
                    .checked_add(groups.len())
                    .filter(|n| *n <= limits.max_trade_hashes)
                    .ok_or(AugmentCatalogError::Limit("trade hashes"))?;
                for group in groups {
                    if group.hash > 9_007_199_254_740_991 || !hashes.insert(group.hash) {
                        return Err(AugmentCatalogError::Invalid("trade hash"));
                    }
                    counts.trade_lines = counts
                        .trade_lines
                        .checked_add(group.lines.len())
                        .filter(|n| *n <= limits.max_lines)
                        .ok_or(AugmentCatalogError::Limit("trade lines"))?;
                    for line in &group.lines {
                        text(line, true)?;
                    }
                }
            }
        }
    }
    Ok(counts)
}

/// Canonical storage order affects only lookup collections. Ordered source line
/// arrays are retained exactly; equal-order entries are never rearranged.
pub fn encode_owned_augments(
    catalog: &AugmentCatalog,
    limits: AugmentCatalogLimits,
) -> Result<Vec<u8>, AugmentCatalogError> {
    validate_owned_augments(catalog, limits)?;
    let mut value = catalog.clone();
    value.source.files.sort_by(|a, b| a.path.cmp(&b.path));
    value
        .augments
        .sort_by(|a, b| a.source_name.cmp(&b.source_name));
    for augment in &mut value.augments {
        augment
            .selectors
            .sort_by(|a, b| a.source_selector.cmp(&b.source_selector));
        for row in &mut augment.selectors {
            if let Some(groups) = &mut row.metadata.trade_hashes {
                groups.sort_by_key(|g| g.hash);
            }
        }
    }
    let mut out = BoundedBytes {
        bytes: Vec::new(),
        limit: limits.max_catalog_bytes,
    };
    serde_json::to_writer_pretty(&mut out, &value)?;
    out.write_all(b"\n")
        .map_err(|_| AugmentCatalogError::Limit("catalog bytes"))?;
    Ok(out.bytes)
}
struct BoundedBytes {
    bytes: Vec<u8>,
    limit: usize,
}
impl Write for BoundedBytes {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.limit - self.bytes.len() {
            return Err(std::io::Error::other("augment catalog byte limit"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
