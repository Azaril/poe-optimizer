//! Closed, configurable modifier-line grammar and source-derived local mappings.
use crate::game_data::GameDataError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemCaptureKind {
    UnsignedInteger,
    UnsignedDecimal,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalWeaponStat {
    PhysicalMinimum,
    PhysicalMaximum,
    FireMinimum,
    FireMaximum,
    PhysicalDamage,
    Speed,
    CriticalChance,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalWeaponOperation {
    Base,
    Increased,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemModifierMapping {
    pub stat: LocalWeaponStat,
    pub operation: LocalWeaponOperation,
    /// Zero-based index into the concrete roll's values.
    pub capture: u32,
    pub flags: u64,
    pub keyword_flags: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemModifierRule {
    pub id: String,
    /// Exact source wording with ordered, unique {0}/{1} numeric placeholders.
    pub template: String,
    pub captures: Vec<ItemCaptureKind>,
    pub modifiers: Vec<ItemModifierMapping>,
}
/// Authored numerical rolls are untrusted until the selected dataset and item
/// admission/engine validate their rule IDs, numeric syntax, count and bounds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemModifierRoll {
    pub rule_id: String,
    pub values: Vec<f64>,
}
fn invalid(message: &str) -> GameDataError {
    GameDataError(message.into())
}
impl ItemModifierRule {
    /// Return literal segments surrounding the ordered captures. Restricted literal
    /// syntax prevents numeric boundaries or two separately declared templates from
    /// accepting the same line. Matching concrete numeric syntax belongs to import.
    pub fn template_literals(&self) -> Result<Vec<&str>, GameDataError> {
        if self.template.trim() != self.template
            || self.template.is_empty()
            || self.template.len() > 256
            || !(1..=2).contains(&self.captures.len())
        {
            return Err(invalid(
                "item modifier template or capture count exceeds supported bounds",
            ));
        }
        let mut literals = Vec::with_capacity(self.captures.len() + 1);
        let mut remaining = self.template.as_str();
        for index in 0..self.captures.len() {
            let marker = format!("{{{index}}}");
            let Some((literal, rest)) = remaining.split_once(&marker) else {
                return Err(invalid(
                    "item modifier placeholders must be ordered unique contiguous indexes",
                ));
            };
            if index > 0 && literal.is_empty() {
                return Err(invalid("adjacent numeric captures are ambiguous"));
            }
            literals.push(literal);
            remaining = rest;
        }
        literals.push(remaining);
        if literals.iter().any(|literal| {
            literal.chars().any(|c| {
                c.is_control() || c.is_ascii_digit() || matches!(c, '{' | '}' | '.' | '+' | '-')
            })
        }) || literals.iter().all(|literal| literal.is_empty())
        {
            return Err(invalid(
                "item modifier literals contain unsupported or ambiguous numeric syntax",
            ));
        }
        Ok(literals)
    }
}
pub(crate) fn validate_rules(rules: &[ItemModifierRule]) -> Result<(), GameDataError> {
    if rules.len() != 5 {
        return Err(invalid(
            "item modifier rules must cover exactly the five local weapon families",
        ));
    }
    let mut ids = BTreeSet::new();
    let mut templates = BTreeSet::new();
    let mut families = BTreeSet::new();
    for rule in rules {
        if rule.id.is_empty()
            || rule.id.len() > 64
            || !rule
                .id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
            || !ids.insert(&rule.id)
        {
            return Err(invalid(
                "item modifier rule IDs must be unique bounded lower-case data keys",
            ));
        }
        rule.template_literals()?;
        if !templates.insert(&rule.template) {
            return Err(invalid("duplicate or ambiguous item modifier template"));
        }
        let family = match rule.modifiers.as_slice() {
            [minimum, maximum] if rule.captures.len() == 2 => {
                use LocalWeaponStat::*;
                let family = match (minimum.stat, maximum.stat) {
                    (PhysicalMinimum, PhysicalMaximum) => 0,
                    (FireMinimum, FireMaximum) => 1,
                    _ => return Err(invalid("unsupported local flat-damage mapping order")),
                };
                for (capture, modifier) in rule.modifiers.iter().enumerate() {
                    if modifier.operation != LocalWeaponOperation::Base
                        || modifier.capture != capture as u32
                        || modifier.flags != 0
                        || modifier.keyword_flags != 0
                    {
                        return Err(invalid(
                            "local flat damage requires exact unscoped BASE captures",
                        ));
                    }
                }
                family
            }
            [modifier] if rule.captures.len() == 1 => {
                use LocalWeaponStat::*;
                // ModFlag.Attack is bit 0 in this pinned numeric flag vocabulary.
                // This is a versioned operation identity, not configurable balance.
                let (family, flags) = match modifier.stat {
                    PhysicalDamage => (2, 0),
                    Speed => (3, 1),
                    CriticalChance => (4, 0),
                    _ => return Err(invalid("unsupported local increased modifier target")),
                };
                if modifier.operation != LocalWeaponOperation::Increased
                    || modifier.capture != 0
                    || modifier.flags != flags
                    || modifier.keyword_flags != 0
                {
                    return Err(invalid(
                        "local increased modifier has unsupported operation, capture or exact flags",
                    ));
                }
                family
            }
            _ => {
                return Err(invalid(
                    "unsupported local modifier capture/mapping cardinality",
                ));
            }
        };
        if !families.insert(family) {
            return Err(invalid("duplicate local weapon modifier family"));
        }
    }
    Ok(())
}
