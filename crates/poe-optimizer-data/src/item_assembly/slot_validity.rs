//! Injected operands for the fixed source-ordered slot-validity predicate.
use super::{Budget, ItemAssemblyTextRewrite, Result, error};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemJewelSlotPolicy {
    pub slot_type: String,
    pub item_type: String,
    pub unique_rarities: [String; 2],
    pub sinister_field: String,
    pub contained_socket_field: String,
    pub charm_socket_field: String,
    pub expansion_field: String,
    pub expansion_size_field: String,
    pub cluster_field: String,
    pub cluster_size_field: String,
    pub charm_subtype: String,
    pub outer_size: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemFlaskSlotRoute {
    pub base_name_pattern: String,
    pub slot_name_pattern: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemFlaskSlotPolicy {
    pub item_type: String,
    pub slot_type: String,
    pub routes: [ItemFlaskSlotRoute; 2],
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemSubtypeSlotRule {
    pub base_subtype: String,
    pub slot_type: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemEmbeddedJewelSlotPolicy {
    pub item_type: String,
    pub slot_pattern: String,
    pub excluded_rarity: String,
    pub parent_rewrite: ItemAssemblyTextRewrite,
    pub restriction_field: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemOffhandSlotLink {
    pub offhand: String,
    pub primary: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemSlotValidityFlag {
    pub state_field: String,
    pub query_name: String,
    pub default: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemWeaponSlotValidityPolicy {
    pub primary_slots: Vec<String>,
    pub offhand_slots: [ItemOffhandSlotLink; 2],
    pub empty_selection: f64,
    pub unarmed_sentinel: String,
    pub primary_tags: [String; 2],
    pub onehand_tag: String,
    pub dual_wield_tag: String,
    pub giants_blood: ItemSlotValidityFlag,
    pub instruments_of_power: ItemSlotValidityFlag,
    pub lord_of_the_wilds: ItemSlotValidityFlag,
    pub bow_type: String,
    pub quiver_type: String,
    pub talisman_type: String,
    pub sceptre_type: String,
    pub staff_type: String,
    pub focus_type: String,
    pub talisman_excluded_rarities: [String; 2],
    pub ordinary_offhand_types: Vec<String>,
    pub excluded_primary_types: Vec<String>,
    pub excluded_offhand_type: String,
    pub giant_tags: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemSlotValidityPolicy {
    pub slot_pattern: String,
    pub jewel: ItemJewelSlotPolicy,
    pub flask: ItemFlaskSlotPolicy,
    pub subtypes: [ItemSubtypeSlotRule; 2],
    pub embedded: ItemEmbeddedJewelSlotPolicy,
    pub weapon: ItemWeaponSlotValidityPolicy,
}

impl ItemSlotValidityPolicy {
    /// Check the same shape and storage limits used by the containing catalog.
    /// This permits safe preparation of a directly constructed caller policy
    /// without pinning its game operands or evaluating its Lua patterns.
    pub fn validate(&self) -> Result<()> {
        validate(&mut Budget { bytes: 0 }, self)
    }
}

pub(super) fn validate(b: &mut Budget, p: &ItemSlotValidityPolicy) -> Result<()> {
    let j = &p.jewel;
    let e = &p.embedded;
    let w = &p.weapon;
    for text in [
        &p.slot_pattern,
        &j.slot_type,
        &j.item_type,
        &j.sinister_field,
        &j.contained_socket_field,
        &j.charm_socket_field,
        &j.expansion_field,
        &j.expansion_size_field,
        &j.cluster_field,
        &j.cluster_size_field,
        &j.charm_subtype,
        &p.flask.item_type,
        &p.flask.slot_type,
        &e.item_type,
        &e.slot_pattern,
        &e.excluded_rarity,
        &e.restriction_field,
        &w.unarmed_sentinel,
        &w.onehand_tag,
        &w.dual_wield_tag,
        &w.bow_type,
        &w.quiver_type,
        &w.talisman_type,
        &w.sceptre_type,
        &w.staff_type,
        &w.focus_type,
        &w.excluded_offhand_type,
    ] {
        b.text(text)?;
    }
    b.number(j.outer_size)?;
    b.number(w.empty_selection)?;
    b.rewrite(&e.parent_rewrite)?;
    for list in [
        j.unique_rarities.as_slice(),
        w.primary_tags.as_slice(),
        w.talisman_excluded_rarities.as_slice(),
        w.primary_slots.as_slice(),
        w.ordinary_offhand_types.as_slice(),
        w.excluded_primary_types.as_slice(),
        w.giant_tags.as_slice(),
    ] {
        b.count(list.len(), 64)?;
        if list.is_empty() {
            return Err(error("empty slot-validity operand list"));
        }
        for text in list {
            b.text(text)?;
        }
    }
    for route in &p.flask.routes {
        b.text(&route.base_name_pattern)?;
        b.text(&route.slot_name_pattern)?;
    }
    for rule in &p.subtypes {
        b.text(&rule.base_subtype)?;
        b.text(&rule.slot_type)?;
    }
    for link in &w.offhand_slots {
        b.text(&link.offhand)?;
        b.text(&link.primary)?;
    }
    if w.offhand_slots[0].offhand == w.offhand_slots[1].offhand {
        return Err(error("ambiguous offhand slot relation"));
    }
    for flag in [
        &w.giants_blood,
        &w.instruments_of_power,
        &w.lord_of_the_wilds,
    ] {
        b.text(&flag.state_field)?;
        b.text(&flag.query_name)?;
    }
    Ok(())
}
