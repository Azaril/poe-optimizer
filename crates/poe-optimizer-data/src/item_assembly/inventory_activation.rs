//! Operands for the fixed activation and rune-choice construction algorithms.
//! Raw rune rows and effects remain in the existing item-loading catalog.
use super::{Budget, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemInventoryEmptyRuneChoice {
    pub name: String,
    pub label: String,
    pub line: String,
    pub slot_type: String,
    pub required_level: f64,
    pub order: f64,
    pub group: f64,
    pub is_socket_bound: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemInventoryRuneChoicePolicy {
    pub empty: ItemInventoryEmptyRuneChoice,
    pub order_default: f64,
    pub broad_slot_type: String,
    pub bonded_display_prefix: String,
    pub modifier_source_prefix: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemInventoryActivationPolicy {
    /// Exact injected lookup used by Populate; absence remains a reached error.
    pub rarity_colors: BTreeMap<String, String>,
    pub rune_choices: ItemInventoryRuneChoicePolicy,
}
impl ItemInventoryActivationPolicy {
    pub fn validate(&self) -> Result<()> {
        validate(&mut Budget { bytes: 0 }, self)
    }
}
pub(super) fn validate(b: &mut Budget, p: &ItemInventoryActivationPolicy) -> Result<()> {
    b.count(p.rarity_colors.len(), 256)?;
    for (key, value) in &p.rarity_colors {
        b.text(key)?;
        b.text(value)?;
    }
    let r = &p.rune_choices;
    for value in [
        &r.empty.name,
        &r.empty.label,
        &r.empty.line,
        &r.empty.slot_type,
        &r.broad_slot_type,
        &r.bonded_display_prefix,
        &r.modifier_source_prefix,
    ] {
        b.text(value)?;
    }
    for value in [
        r.empty.required_level,
        r.empty.order,
        r.empty.group,
        r.order_default,
    ] {
        b.number(value)?;
    }
    Ok(())
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    pub(crate) fn caller() -> ItemInventoryActivationPolicy {
        ItemInventoryActivationPolicy {
            rarity_colors: [("Caller rarity".into(), "prefix:".into())].into(),
            rune_choices: ItemInventoryRuneChoicePolicy {
                empty: ItemInventoryEmptyRuneChoice {
                    name: "Empty caller rune".into(),
                    label: "Caller empty".into(),
                    line: "Caller line".into(),
                    slot_type: "no slot".into(),
                    required_level: -2.0,
                    order: -7.0,
                    group: 0.5,
                    is_socket_bound: true,
                },
                order_default: 3.5,
                broad_slot_type: "caller armour".into(),
                bonded_display_prefix: "linked: ".into(),
                modifier_source_prefix: "caller:".into(),
            },
        }
    }
    #[test]
    fn activation_policy_preserves_custom_operands_without_prejudging_rows_or_sort_order() {
        let p = caller();
        p.validate().unwrap();
        assert_eq!(
            serde_json::from_slice::<ItemInventoryActivationPolicy>(
                &serde_json::to_vec(&p).unwrap()
            )
            .unwrap(),
            p
        );
        let mut p = p;
        p.rarity_colors.clear();
        p.rune_choices.empty.name.clear();
        p.rune_choices.modifier_source_prefix = "\0".into();
        p.validate().unwrap();
    }
    #[test]
    fn activation_policy_bounds_direct_construction_and_requires_fields() {
        let mut p = caller();
        p.rune_choices.order_default = f64::NAN;
        assert!(p.validate().is_err());
        let mut p = caller();
        p.rune_choices.empty.name = "x".repeat(4097);
        assert!(p.validate().is_err());
        let mut p = caller();
        p.rarity_colors = (0..257).map(|i| (i.to_string(), String::new())).collect();
        assert!(p.validate().is_err());
        let mut p = caller();
        p.rarity_colors = (0..100)
            .map(|i| (i.to_string(), "x".repeat(4096)))
            .collect();
        assert!(p.validate().is_err());
        let mut value = serde_json::to_value(caller()).unwrap();
        value["rune_choices"]
            .as_object_mut()
            .unwrap()
            .remove("order_default");
        assert!(serde_json::from_value::<ItemInventoryActivationPolicy>(value).is_err());
    }
}
