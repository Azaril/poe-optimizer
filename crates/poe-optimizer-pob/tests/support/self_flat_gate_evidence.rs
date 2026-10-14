//! Offline comparison of source observations, never a native coverage certificate.
use serde_json::Value;

/// Compare only the three reviewed cross-stat inventories by stat-name buckets.
/// Stable sorting preserves every record and the order within each stat. No raw
/// evidence is rewritten, and no query, occurrence, tag or numeric output sorts.
pub fn comparison_view(state: &Value) -> Value {
    fn bucket(value: &mut Value) {
        if let Some(rows) = value.as_array_mut() {
            for row in rows.iter() {
                assert!(row["name"].is_string(), "inventory stat name is required");
            }
            rows.sort_by(|left, right| {
                left["name"]
                    .as_str()
                    .unwrap()
                    .cmp(right["name"].as_str().unwrap())
            });
        } else {
            assert_eq!(value, &serde_json::json!({}));
        }
    }
    let mut view = state.clone();
    for row in view["group_deliveries"].as_array_mut().unwrap() {
        bucket(&mut row["modifiers"]);
    }
    for row in view["player_buff_visits"].as_array_mut().unwrap() {
        bucket(&mut row["buff"]["modifiers"]);
    }
    for row in view["player_visits"].as_array_mut().unwrap() {
        if let Some(buffs) = row["buffs"].as_array_mut() {
            for buff in buffs {
                bucket(&mut buff["modifiers"]);
            }
        } else {
            assert_eq!(row["buffs"], serde_json::json!({}));
        }
    }
    view
}

pub fn reviewed_reordering_count(left: &Value, right: &Value) -> usize {
    let mut count = 0;
    for (left, right) in left["group_deliveries"]
        .as_array()
        .unwrap()
        .iter()
        .zip(right["group_deliveries"].as_array().unwrap())
    {
        count += usize::from(left["modifiers"] != right["modifiers"]);
    }
    for (left, right) in left["player_buff_visits"]
        .as_array()
        .unwrap()
        .iter()
        .zip(right["player_buff_visits"].as_array().unwrap())
    {
        count += usize::from(left["buff"]["modifiers"] != right["buff"]["modifiers"]);
    }
    for (left, right) in left["player_visits"]
        .as_array()
        .unwrap()
        .iter()
        .zip(right["player_visits"].as_array().unwrap())
    {
        if let (Some(left), Some(right)) = (left["buffs"].as_array(), right["buffs"].as_array()) {
            for (left, right) in left.iter().zip(right) {
                count += usize::from(left["modifiers"] != right["modifiers"]);
            }
        }
    }
    count
}
