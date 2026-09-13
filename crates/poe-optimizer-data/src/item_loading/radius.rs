//! Injected source operands for versioned radius resolution and Item headers.
use super::{Result, error, text};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JewelRadiusPolicy {
    pub version_pattern: String,
    pub canonical_separator: String,
    pub latest_tree_version: String,
    pub distance_multiplier: f64,
    pub initial_maximum: f64,
    pub outer_field: String,
    pub inner_field: String,
    pub outer_squared_field: String,
    pub inner_squared_field: String,
    pub label_field: String,
    pub header: String,
    pub jewel_type: String,
    pub label_pattern: String,
    pub variable_pattern: String,
    pub variable_label: String,
    pub item_label_field: String,
    pub item_index_field: String,
    pub item_data_field: String,
    pub deferred_index_field: String,
    pub override_field: String,
}
impl JewelRadiusPolicy {
    pub(super) fn validate(&self) -> Result<()> {
        for value in [
            &self.version_pattern,
            &self.canonical_separator,
            &self.latest_tree_version,
            &self.outer_field,
            &self.inner_field,
            &self.outer_squared_field,
            &self.inner_squared_field,
            &self.label_field,
            &self.header,
            &self.jewel_type,
            &self.label_pattern,
            &self.variable_pattern,
            &self.variable_label,
            &self.item_label_field,
            &self.item_index_field,
            &self.item_data_field,
            &self.deferred_index_field,
            &self.override_field,
        ] {
            text(value, 4096)?;
            if value.is_empty() {
                return Err(error("empty jewel radius policy operand"));
            }
        }
        if !self.distance_multiplier.is_finite() || !self.initial_maximum.is_finite() {
            return Err(error("nonfinite jewel radius policy operand"));
        }
        Ok(())
    }
}
