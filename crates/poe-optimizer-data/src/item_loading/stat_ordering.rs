//! Injected operands for the source's unique-line lookup and crafted-line order.
//! Raw modifier rows remain in ItemLoadingData::modifier_tables. Their reached
//! type/errors and traversal ambiguity belong to the consumer, not this schema.
use super::{Result, error, text};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemStatOrderingSubstitution {
    pub pattern: String,
    pub replacement: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemStatOrderingGroups {
    pub crafted_custom: f64,
    pub fractured: f64,
    pub ordinary: f64,
    pub compare_order_below: f64,
}

/// Fixed domain algorithm: numeric/range substitutions precede lowercasing and
/// newline substitution; exact keys use only lowercasing/newline substitution.
/// The source comparator's original-position tie break and nil-as-infinity
/// semantics are operations, not a claim that Lua table.sort itself is stable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemStatOrderingPolicy {
    pub modifier_table: String,
    pub stat_order_field: String,
    pub unique_rarity: String,
    pub relic_rarity: String,
    pub normalize_numbers: ItemStatOrderingSubstitution,
    pub normalize_ranges: ItemStatOrderingSubstitution,
    pub flatten_newlines: ItemStatOrderingSubstitution,
    pub groups: ItemStatOrderingGroups,
}

impl ItemStatOrderingPolicy {
    /// Validate direct caller construction before cloning or compiling patterns.
    /// This does not authenticate source, grant an execution capability, resolve
    /// the named modifier family, or eagerly validate its finite metadata rows.
    pub fn validate(&self) -> Result<()> {
        let mut bytes = 0usize;
        let mut check = |value: &str, nonempty: bool| -> Result<()> {
            text(value, 4096)?;
            if nonempty && value.is_empty() {
                return Err(error("empty stat ordering policy identity"));
            }
            bytes = bytes
                .checked_add(value.len())
                .ok_or_else(|| error("stat ordering policy text overflow"))?;
            if bytes > 32 * 1024 {
                return Err(error("stat ordering policy aggregate text bound"));
            }
            Ok(())
        };
        for value in [
            &self.modifier_table,
            &self.stat_order_field,
            &self.unique_rarity,
            &self.relic_rarity,
        ] {
            check(value, true)?;
        }
        for rule in [
            &self.normalize_numbers,
            &self.normalize_ranges,
            &self.flatten_newlines,
        ] {
            // Empty patterns/replacements have defined Lua string semantics.
            check(&rule.pattern, false)?;
            check(&rule.replacement, false)?;
        }
        for value in [
            self.groups.crafted_custom,
            self.groups.fractured,
            self.groups.ordinary,
            self.groups.compare_order_below,
        ] {
            if !value.is_finite() {
                return Err(error("nonfinite stat ordering group operand"));
            }
        }
        Ok(())
    }
}
