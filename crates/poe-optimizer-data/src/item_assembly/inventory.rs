//! Injected definitions for fixed inventory construction and item-set loading.
//! This catalog carries identities and operands; it does not execute rune effects,
//! power-stat transforms, dropdown callbacks, or an arbitrary source program.
use super::{Budget, Result, error};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemInventorySwap {
    pub slot_pattern: String,
    pub suffix: String,
    pub primary_weapon_set: u16,
    pub alternate_weapon_set: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemInventoryEmbedded {
    pub parent_slots: Vec<String>,
    pub count: u16,
    pub name_infix: String,
    pub label_prefix: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemInventoryPassive {
    pub node_type: String,
    pub contained_socket_field: String,
    pub slot_prefix: String,
    pub label: String,
    pub nodes: ItemInventoryPassiveNodes,
}

/// Complete constructor predicate result over the authenticated full tree,
/// before projection to the partial bundled class tree. This grants no passive
/// allocation, cluster or numerical capability.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemInventoryPassiveNodes {
    pub tree_version: String,
    pub full_snapshot_sha256: String,
    pub ids: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemInventoryRuneSlot {
    pub name: String,
    pub slot_type: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemInventoryLayout {
    /// Constructor order, before adding swaps, embedded children and tree sockets.
    pub base_slots: Vec<String>,
    pub swap: ItemInventorySwap,
    pub embedded: ItemInventoryEmbedded,
    /// Applied to the injected latest-tree nodes, never a frozen observed ID list.
    pub passive: ItemInventoryPassive,
    /// Control identities only. Choice order, parsed effects and selection remain
    /// separate consumers of the existing rune catalog and runtime state.
    pub rune_slots: Vec<ItemInventoryRuneSlot>,
    pub slot_number_patterns: [String; 2],
    pub activation_patterns: [String; 2],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemInventoryDefaults {
    pub first_set_id: f64,
    pub set_id_increment: f64,
    pub reset_active_set_id: f64,
    pub empty_item_id: f64,
    pub default_set_title: String,
    pub empty_rune_name: String,
    pub empty_slot_name: String,
    pub empty_url: String,
    pub true_token: String,
    pub empty_item_label: String,
    pub show_stat_differences: bool,
}

/// A typed description, not executable behavior. Vector position in the owning
/// ItemInventoryPowerStats is its identity. Equal descriptions at different
/// positions remain different source functions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ItemInventoryPowerTransform {
    Negate,
    Gsub {
        pattern: String,
        replacement: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemInventoryPowerStat {
    pub stat: Option<String>,
    pub label: Option<String>,
    /// Zero-based reference into this exact catalog's transform vector. Absence
    /// means nil, not an identity transform. Copied rows can share a reference.
    pub transform: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemInventoryPowerStats {
    /// Source lookup order; duplicate and absent stat keys are meaningful.
    pub rows: Vec<ItemInventoryPowerStat>,
    pub transforms: Vec<ItemInventoryPowerTransform>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemInventoryPolicy {
    pub layout: ItemInventoryLayout,
    pub defaults: ItemInventoryDefaults,
    pub power_stats: ItemInventoryPowerStats,
}

impl ItemInventoryPolicy {
    /// Same bounds as catalog validation, for directly injected caller policies.
    /// This does not authenticate source text or compile/evaluate Lua patterns.
    pub fn validate(&self) -> Result<()> {
        validate(&mut Budget { bytes: 0 }, self)
    }
}

pub(super) fn validate(b: &mut Budget, p: &ItemInventoryPolicy) -> Result<()> {
    let l = &p.layout;
    b.count(l.base_slots.len(), 128)?;
    b.count(l.embedded.parent_slots.len(), 128)?;
    b.count(l.rune_slots.len(), 128)?;
    b.count(usize::from(l.embedded.count), 256)?;
    if l.base_slots.is_empty() {
        return Err(error("inventory constructor has no base slots"));
    }
    for text in l
        .base_slots
        .iter()
        .chain(l.embedded.parent_slots.iter())
        .chain(l.slot_number_patterns.iter())
        .chain(l.activation_patterns.iter())
        .chain([
            &l.swap.slot_pattern,
            &l.swap.suffix,
            &l.embedded.name_infix,
            &l.embedded.label_prefix,
            &l.passive.node_type,
            &l.passive.contained_socket_field,
            &l.passive.slot_prefix,
            &l.passive.label,
        ])
    {
        b.text(text)?;
    }
    b.text(&l.passive.nodes.tree_version)?;
    b.text(&l.passive.nodes.full_snapshot_sha256)?;
    if l.passive.nodes.tree_version.is_empty()
        || l.passive.nodes.full_snapshot_sha256.len() != 64
        || !l
            .passive
            .nodes
            .full_snapshot_sha256
            .bytes()
            .all(|v| v.is_ascii_digit() || (b'a'..=b'f').contains(&v))
    {
        return Err(error("inventory passive tree provenance shape"));
    }
    b.count(l.passive.nodes.ids.len(), 65_536)?;
    if l.passive
        .nodes
        .ids
        .windows(2)
        .any(|pair| pair[0] >= pair[1])
    {
        return Err(error(
            "inventory passive node IDs must be sorted and unique",
        ));
    }
    for slot in &l.rune_slots {
        for text in [&slot.name, &slot.slot_type, &slot.label] {
            b.text(text)?;
        }
    }
    let d = &p.defaults;
    for value in [
        d.first_set_id,
        d.set_id_increment,
        d.reset_active_set_id,
        d.empty_item_id,
    ] {
        b.number(value)?;
        if value.fract() != 0.0 || value.abs() > 9_007_199_254_740_991.0 {
            return Err(error("inventory default ID must be an exact integer"));
        }
    }
    if d.set_id_increment <= 0.0 {
        return Err(error(
            "inventory generated set ID increment must be positive",
        ));
    }
    for text in [
        &d.default_set_title,
        &d.empty_rune_name,
        &d.empty_slot_name,
        &d.empty_url,
        &d.true_token,
        &d.empty_item_label,
    ] {
        b.text(text)?;
    }
    b.count(p.power_stats.rows.len(), 1024)?;
    b.count(p.power_stats.transforms.len(), 128)?;
    for row in &p.power_stats.rows {
        for text in row.stat.iter().chain(row.label.iter()) {
            b.text(text)?;
        }
        if row
            .transform
            .is_some_and(|id| usize::from(id) >= p.power_stats.transforms.len())
        {
            return Err(error(
                "inventory power-stat transform reference is outside its catalog",
            ));
        }
    }
    for transform in &p.power_stats.transforms {
        if let ItemInventoryPowerTransform::Gsub {
            pattern,
            replacement,
        } = transform
        {
            b.text(pattern)?;
            b.text(replacement)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> ItemInventoryPolicy {
        ItemInventoryPolicy {
            layout: ItemInventoryLayout {
                base_slots: vec!["Caller slot".into()],
                swap: ItemInventorySwap {
                    slot_pattern: "Caller".into(),
                    suffix: " other".into(),
                    primary_weapon_set: 4,
                    alternate_weapon_set: 7,
                },
                embedded: ItemInventoryEmbedded {
                    parent_slots: vec!["Caller slot".into()],
                    count: 2,
                    name_infix: " socket ".into(),
                    label_prefix: "Socket ".into(),
                },
                passive: ItemInventoryPassive {
                    node_type: "Caller socket".into(),
                    contained_socket_field: "callerSocket".into(),
                    slot_prefix: "Node ".into(),
                    label: "Tree slot".into(),
                    nodes: ItemInventoryPassiveNodes {
                        tree_version: "caller".into(),
                        full_snapshot_sha256: "a".repeat(64),
                        ids: vec![7, 31],
                    },
                },
                rune_slots: vec![ItemInventoryRuneSlot {
                    name: "Caller rune".into(),
                    slot_type: "caller".into(),
                    label: "Rune".into(),
                }],
                slot_number_patterns: ["(%d+)$".into(), "(%d+)".into()],
                activation_patterns: ["Caller".into(), "Other".into()],
            },
            defaults: ItemInventoryDefaults {
                first_set_id: 5.0,
                set_id_increment: 2.0,
                reset_active_set_id: -1.0,
                empty_item_id: 0.0,
                default_set_title: "Caller".into(),
                empty_rune_name: "Empty".into(),
                empty_slot_name: "".into(),
                empty_url: "".into(),
                true_token: "yes".into(),
                empty_item_label: "No item".into(),
                show_stat_differences: false,
            },
            power_stats: ItemInventoryPowerStats {
                rows: vec![
                    ItemInventoryPowerStat {
                        stat: None,
                        label: None,
                        transform: None,
                    },
                    ItemInventoryPowerStat {
                        stat: Some("Caller".into()),
                        label: Some("First".into()),
                        transform: Some(0),
                    },
                    ItemInventoryPowerStat {
                        stat: Some("Caller".into()),
                        label: Some("Second".into()),
                        transform: Some(1),
                    },
                    ItemInventoryPowerStat {
                        stat: Some("Copy".into()),
                        label: None,
                        transform: Some(0),
                    },
                ],
                transforms: vec![
                    ItemInventoryPowerTransform::Negate,
                    ItemInventoryPowerTransform::Negate,
                ],
            },
        }
    }

    #[test]
    fn caller_inventory_preserves_order_nil_and_distinct_or_shared_transform_ids() {
        let p = policy();
        p.validate().unwrap();
        let encoded = serde_json::to_vec(&p).unwrap();
        let decoded: ItemInventoryPolicy = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, p);
        assert_eq!(
            decoded.power_stats.rows[1].transform,
            decoded.power_stats.rows[3].transform
        );
        assert_ne!(
            decoded.power_stats.rows[1].transform,
            decoded.power_stats.rows[2].transform
        );
    }

    #[test]
    fn inventory_rejects_bad_transform_references_and_unbounded_direct_policies() {
        let mut p = policy();
        p.power_stats.rows[1].transform = Some(2);
        assert!(p.validate().unwrap_err().0.contains("transform reference"));
        let mut p = policy();
        p.layout.base_slots.resize(129, "slot".into());
        assert!(p.validate().is_err());
        let mut p = policy();
        p.power_stats
            .transforms
            .push(ItemInventoryPowerTransform::Gsub {
                pattern: "x".repeat(4097),
                replacement: String::new(),
            });
        assert!(p.validate().is_err());
        let mut p = policy();
        p.defaults.set_id_increment = 0.0;
        assert!(p.validate().is_err());
        let mut p = policy();
        p.layout.passive.nodes.ids = vec![31, 7];
        assert!(p.validate().is_err());
        let mut p = policy();
        p.layout.passive.nodes.ids = vec![7, 7];
        assert!(p.validate().is_err());
    }
}
