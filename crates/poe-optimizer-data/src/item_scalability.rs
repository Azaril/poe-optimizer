//! Complete item-scaling definitions. Membership does not certify item mechanics.
use crate::game_data::GameDataError;
use crate::item_loading::ItemLoadingSource;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub const ITEM_SCALABILITY_SCHEMA_VERSION: u32 = 1;
type Result<T> = std::result::Result<T, GameDataError>;
fn error(message: &str) -> GameDataError {
    GameDataError(format!("item scalability catalog: {message}"))
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemScalabilityValue {
    pub is_scalable: bool,
    /// None and an empty source array remain distinguishable. Order is semantic.
    pub formats: Option<Vec<String>>,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemFormatAssignments {
    /// None preserves the state established by preceding format labels.
    pub precision: Option<f64>,
    pub display_precision: Option<u8>,
    pub if_required: Option<bool>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalystScalingData {
    /// Source missing-quality default; independent of default item quality.
    pub default_quality: f64,
    pub percent_offset: f64,
    pub percent_divisor: f64,
    pub neutral_scalar: f64,
    /// Added only after the original modTags array passed its nonempty guard.
    pub extra_tag_flags: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemScalabilityData {
    pub schema_version: u32,
    pub source: ItemLoadingSource,
    /// Exact case-sensitive keys. An empty vector prevents unknown-key fallback.
    pub entries: BTreeMap<String, Vec<ItemScalabilityValue>>,
    /// Missing labels are original dispatcher no-ops, retained in formats above.
    pub format_assignments: BTreeMap<String, ItemFormatAssignments>,
    /// Fallback decimal exponent; not the exact-key internal multiplier.
    pub default_high_precision: u8,
    /// Missing entry in an authored per-number range array, not affix quality.
    pub missing_range_value: f64,
    pub antonyms: BTreeMap<String, String>,
    pub catalyst_scaling: CatalystScalingData,
}
#[derive(Debug, Clone)]
pub struct ItemScalabilityCatalog(Arc<ItemScalabilityData>);
impl ItemScalabilityCatalog {
    pub fn new(data: ItemScalabilityData) -> Result<Self> {
        data.validate()?;
        Ok(Self(Arc::new(data)))
    }
    pub fn data(&self) -> &ItemScalabilityData {
        &self.0
    }
    pub fn lookup_exact(&self, key: &str) -> Option<&[ItemScalabilityValue]> {
        self.0.entries.get(key).map(Vec::as_slice)
    }
    pub fn format_assignments(&self, label: &str) -> Option<&ItemFormatAssignments> {
        self.0.format_assignments.get(label)
    }
}
fn valid_digest(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
fn valid_text(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && !value.contains('\0')
}
fn identifier(value: &str) -> bool {
    valid_text(value, 128)
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
}
impl ItemScalabilityData {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != ITEM_SCALABILITY_SCHEMA_VERSION
            || !valid_digest(&self.source.upstream_revision, 40)
            || self.source.files.is_empty()
            || self.source.files.len() > 64
            || self.source.construction_spans.is_empty()
            || self.source.construction_spans.len() > 128
            || self.source.module_order.len() > 128
            || self.entries.is_empty()
            || self.entries.len() > 50_000
            || self.format_assignments.len() > 256
        {
            return Err(error("invalid version, source or table bounds"));
        }
        for (path, digest) in &self.source.files {
            if !valid_text(path, 512)
                || !path.starts_with("src/")
                || path.contains('\\')
                || path
                    .split('/')
                    .any(|v| v.is_empty() || v == "." || v == "..")
                || !valid_digest(digest, 64)
            {
                return Err(error("invalid source file identity"));
            }
        }
        for (key, span) in &self.source.construction_spans {
            if !valid_text(key, 256)
                || !self.source.files.contains_key(&span.path)
                || span.line == 0
                || span.end_line < span.line
                || span.end_line > 1_000_000
                || !valid_digest(&span.sha256, 64)
            {
                return Err(error("invalid source span"));
            }
        }
        if self
            .source
            .module_order
            .iter()
            .any(|p| !self.source.files.contains_key(p))
        {
            return Err(error("unknown source module"));
        }
        let mut capture_count = 0usize;
        let mut label_count = 0usize;
        let mut text_bytes = 0usize;
        for (key, values) in &self.entries {
            if !valid_text(key, 4096)
                || values.len() > 64
                || key.bytes().filter(|b| *b == b'#').count() != values.len()
            {
                return Err(error("invalid exact key or capture arity"));
            }
            capture_count += values.len();
            text_bytes += key.len();
            for value in values {
                if let Some(labels) = &value.formats {
                    if labels.len() > 32 {
                        return Err(error("too many ordered format labels"));
                    }
                    for label in labels {
                        if !valid_text(label, 256) {
                            return Err(error("invalid format label"));
                        }
                        text_bytes += label.len();
                    }
                    label_count += labels.len();
                }
            }
        }
        if capture_count > 200_000 || label_count > 500_000 || text_bytes > 8 * 1024 * 1024 {
            return Err(error("aggregate entries exceed bounds"));
        }
        for (label, assignments) in &self.format_assignments {
            if !valid_text(label, 256)
                || assignments
                    .precision
                    .is_some_and(|n| !n.is_finite() || !(1.0..=10_000.0).contains(&n))
                || assignments.display_precision.is_some_and(|n| n > 2)
            {
                return Err(error("invalid format assignments"));
            }
        }
        if self.default_high_precision > 12
            || !self.missing_range_value.is_finite()
            || !(0.0..=1.0).contains(&self.missing_range_value)
            || self.antonyms.len() > 64
            || self
                .antonyms
                .iter()
                .any(|(a, b)| !identifier(a) || !identifier(b))
        {
            return Err(error("invalid fallback policy"));
        }
        let c = &self.catalyst_scaling;
        if [
            c.default_quality,
            c.percent_offset,
            c.percent_divisor,
            c.neutral_scalar,
        ]
        .iter()
        .any(|n| !n.is_finite() || n.abs() > 1_000_000.0)
            || c.percent_divisor <= 0.0
            || c.extra_tag_flags.len() > 32
            || c.extra_tag_flags.iter().any(|v| !identifier(v))
            || c.extra_tag_flags.iter().collect::<BTreeSet<_>>().len() != c.extra_tag_flags.len()
        {
            return Err(error("invalid catalyst policy"));
        }
        Ok(())
    }
}
