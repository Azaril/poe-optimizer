//! Injected parameters for the fixed, ordered local-jewel producer.
use super::{Budget, ItemAssemblyOverridePolicy, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyJewelSpectrum {
    pub name_item_field: String,
    pub name_pattern: String,
    pub modifier_name: String,
    pub modifier_type: String,
    pub modifier_value: f64,
    pub minion_name: String,
    pub minion_type: String,
    /// The nested record is the same object added to the outer modifier list.
    pub nested_mod_field: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyJewelListField {
    pub query_name: String,
    pub output_field: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyJewelFromNothing {
    /// This List query is still called even though an empty result table is truthy.
    pub guard_query_name: String,
    pub output_field: String,
    /// A separate, repeated List call supplies the actual stores.
    pub entries: ItemAssemblyOverridePolicy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyJewelSkillCorrection {
    pub matching_skill: String,
    pub replacement_skill: String,
    pub node_count_below: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyJewelValidity {
    pub output_field: String,
    pub keystone_field: String,
    pub smalls_are_nothingness_field: String,
    pub socket_count_override_field: String,
    pub nothingness_count_field: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyJewelCluster {
    pub item_field: String,
    pub notables: ItemAssemblyJewelListField,
    pub added_mods: ItemAssemblyJewelListField,
    pub skill_field: String,
    pub node_count_field: String,
    pub skills_field: String,
    pub min_nodes_field: String,
    pub max_nodes_field: String,
    pub correction: ItemAssemblyJewelSkillCorrection,
    /// The producer retains Lua's operand values in its fixed short-circuit formula.
    pub validity: ItemAssemblyJewelValidity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAssemblyJewelPolicy {
    /// Shared by the BuildModList reset and the slot-local producer.
    pub output_field: String,
    pub grand_spectrum: ItemAssemblyJewelSpectrum,
    pub functions: ItemAssemblyJewelListField,
    pub overrides: ItemAssemblyOverridePolicy,
    pub alternate_class_start: ItemAssemblyJewelListField,
    pub from_nothing: ItemAssemblyJewelFromNothing,
    pub cluster: ItemAssemblyJewelCluster,
}

pub(super) fn validate(b: &mut Budget, p: &ItemAssemblyJewelPolicy) -> Result<()> {
    let g = &p.grand_spectrum;
    let c = &p.cluster;
    for text in [
        &p.output_field,
        &g.name_item_field,
        &g.name_pattern,
        &g.modifier_name,
        &g.modifier_type,
        &g.minion_name,
        &g.minion_type,
        &g.nested_mod_field,
        &p.from_nothing.guard_query_name,
        &p.from_nothing.output_field,
        &c.item_field,
        &c.skill_field,
        &c.node_count_field,
        &c.skills_field,
        &c.min_nodes_field,
        &c.max_nodes_field,
        &c.correction.matching_skill,
        &c.correction.replacement_skill,
        &c.validity.output_field,
        &c.validity.keystone_field,
        &c.validity.smalls_are_nothingness_field,
        &c.validity.socket_count_override_field,
        &c.validity.nothingness_count_field,
    ] {
        b.text(text)?;
    }
    for field in [
        &p.functions,
        &p.alternate_class_start,
        &c.notables,
        &c.added_mods,
    ] {
        b.text(&field.query_name)?;
        b.text(&field.output_field)?;
    }
    for stores in [&p.overrides, &p.from_nothing.entries] {
        b.text(&stores.query_name)?;
        b.text(&stores.key_field)?;
        b.text(&stores.value_field)?;
    }
    b.number(g.modifier_value)?;
    b.number(c.correction.node_count_below)
}
