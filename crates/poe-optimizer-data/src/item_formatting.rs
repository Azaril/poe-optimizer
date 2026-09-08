//! Source-keyed item preprocessing. Configuration modifiers do not use this data.
use crate::game_data::GameDataError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemNumberFormat {
    /// Internal multiplier/divisor passed by ItemTools.applyRange to formatValue.
    pub precision: f64,
    pub display_precision: Option<u32>,
    pub trim_trailing_zeroes: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemFormattingRule {
    /// Exact, case-sensitive ModScalability key. `#` marks formatted values;
    /// literal numbers remain significant for source most-specific matching.
    pub template: String,
    pub captures: Vec<ItemNumberFormat>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemFormattingData {
    /// Missing exact keys preserve literal values in the admitted unit-scalar,
    /// rangeless item pipeline. This is independent of ModParser's case folding.
    pub rules: Vec<ItemFormattingRule>,
}
fn invalid(message: &str) -> GameDataError {
    GameDataError(message.into())
}
impl ItemFormattingData {
    pub(crate) fn validate(&self) -> Result<(), GameDataError> {
        if self.rules.is_empty() || self.rules.len() > 1024 {
            return Err(invalid(
                "item formatting requires one to 1024 source-keyed rules",
            ));
        }
        let mut keys = BTreeSet::new();
        for rule in &self.rules {
            if rule.template.is_empty()
                || rule.template.len() > 256
                || rule.template.trim() != rule.template
                || !rule
                    .template
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b" #%+-,.'".contains(&b))
                || !keys.insert(&rule.template)
            {
                return Err(invalid(
                    "item formatting keys must be unique bounded exact templates",
                ));
            }
            if rule.captures.len() > 2
                || rule.template.bytes().filter(|b| *b == b'#').count() != rule.captures.len()
            {
                return Err(invalid(
                    "item formatting capture count differs from source key",
                ));
            }
            for capture in &rule.captures {
                if !capture.precision.is_finite()
                    || !(1.0..=10000.0).contains(&capture.precision)
                    || capture.display_precision.is_some_and(|v| v > 2)
                    || (capture.trim_trailing_zeroes && capture.display_precision.is_none())
                {
                    return Err(invalid(
                        "item numeric formatting precision is outside admitted source semantics",
                    ));
                }
            }
        }
        Ok(())
    }
}
