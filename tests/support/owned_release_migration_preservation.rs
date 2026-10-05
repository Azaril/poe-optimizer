//! Inverse check for exact checked-release dependency rebinding.
//! Callers separately validate every schema, rule and registry change before
//! allowing these three graphs to differ. Everything else must survive exactly.
use poe_optimizer_import::{
    owned_normalize::NormalizationPolicy, owned_release::StagedOwnedRelease,
};
use serde_json::{Value, json};

pub fn assert_import_rebindings_only(prior: &StagedOwnedRelease, next: &StagedOwnedRelease) {
    let old = prior.input();
    let new = next.input();
    let old_value = json!(old);
    let mut restored = json!(new);
    // Checked release assembly authenticates each dependency before these exact
    // rebind locations are restored. No recursive identity stripping is used.
    for path in [
        "/recipe/schema",
        "/recipe/rules",
        "/recipe/registry",
        "/recipe/routing/definitions",
        "/mapping/definitions",
        "/mapping/registry",
        "/roles/definitions",
        "/roles/mapping",
        "/rewards/definitions",
        "/rewards/mapping",
        "/items/definitions",
        "/item_source/item_lines",
        "/tree/definitions",
        "/tree/registry",
        "/tree/mapping",
        "/tree/normalization",
        "/normalization/gem_quality/value/definitions",
        "/normalization/gem_inputs/definitions",
        "/normalization/direct_skill_inputs/definitions",
        "/normalization/direct_skill_inputs/roles",
        "/normalization/gem_inventory/definitions",
        "/normalization/gem_inventory/roles",
        "/normalization/gem_inventory/scalar_inputs",
        "/normalization/gem_inventory/usage_inputs",
        "/normalization/usage_inputs/definitions",
        "/normalization/usage_inputs/roles",
        "/normalization/usage_inputs/scalar_inputs",
        "/normalization/support_origin_order/roles",
        "/normalization/payload_inventory/roles",
        "/normalization/skill_inventory/roles",
        "/normalization/skill_inventory/direct_inputs",
        "/normalization/configuration_reward_inventory/reward_policy",
        "/normalization/equipment_membership/definitions",
        "/normalization/equipment_membership/item_lines",
        "/normalization/equipment_membership/item_source",
        "/normalization/item_modifier_membership/definitions",
        "/normalization/item_modifier_membership/item_lines",
        "/normalization/item_modifier_membership/item_source",
        "/normalization/item_parameter_inputs/definitions",
        "/normalization/item_parameter_inputs/item_lines",
        "/normalization/item_parameter_inputs/item_source",
        "/normalization/passive_socket_membership/definitions",
        "/normalization/passive_socket_membership/mapping",
        "/normalization/passive_socket_membership/item_lines",
        "/normalization/passive_socket_membership/item_source",
        "/normalization/passive_socket_membership/equipment",
    ] {
        *restored
            .pointer_mut(path)
            .unwrap_or_else(|| panic!("new {path}")) = old_value
            .pointer(path)
            .unwrap_or_else(|| panic!("old {path}"))
            .clone();
    }
    // These optional policies postdate the older checkpoint fixtures. Preserve
    // their presence and all source/parameter/target correspondence; only the
    // exact authenticated dependency fields below may be rebound.
    for path in [
        "/normalization/generated_skill_inputs/definitions",
        "/normalization/generated_skill_inputs/roles",
        // direct_skill_inputs::rebind recomputes this inherited commitment
        // after its exact definitions/roles dependencies above are rebound.
        // The target policy kind, presence and all other fields stay exact.
        "/normalization/direct_support_targets/direct_inputs",
    ] {
        match (old_value.pointer(path), restored.pointer_mut(path)) {
            (Some(old), Some(new)) => *new = old.clone(),
            (None, None) => {}
            _ => panic!("optional dependency presence changed: {path}"),
        }
    }
    for list in [
        "/normalization/gem_inventory/primary_dispositions",
        "/normalization/direct_skill_inputs/dispositions",
    ] {
        let count = old_value.pointer(list).unwrap().as_array().unwrap().len();
        assert_eq!(
            restored.pointer(list).unwrap().as_array().unwrap().len(),
            count
        );
        for index in 0..count {
            for field in ["definitions", "roles"] {
                let path = format!("{list}/{index}/reference_action/{field}");
                *restored.pointer_mut(&path).unwrap() = old_value.pointer(&path).unwrap().clone();
            }
        }
    }
    // Preserve finite f64 values through their actual typed policy representation.
    let policy: NormalizationPolicy =
        serde_json::from_value(restored["normalization"].clone()).unwrap();
    assert!(
        policy == old.normalization,
        "normalization changed beyond exact dependency rebinding: {}",
        first_difference(&json!(old.normalization), &json!(policy), "/normalization")
            .unwrap_or_else(|| "typed policy mismatch with equal JSON".into())
    );
    restored["normalization"] = old_value["normalization"].clone();
    assert_eq!(new.provenance.len(), old.provenance.len() + 1);
    assert_eq!(new.provenance[..old.provenance.len()], old.provenance);
    restored["provenance"].as_array_mut().unwrap().pop();
    assert!(
        restored == old_value,
        "prior release changed beyond the checked delta: {}",
        first_difference(&old_value, &restored, "").unwrap_or_else(|| "unknown mismatch".into())
    );
}

// Failure diagnostics must not print a multi-megabyte policy or release. Keep
// the first exact JSON pointer and bounded scalar summaries; equality above
// still compares the complete typed policy and complete release.
fn first_difference(old: &Value, new: &Value, path: &str) -> Option<String> {
    if old == new {
        return None;
    }
    match (old, new) {
        (Value::Object(old), Value::Object(new)) => {
            for (key, value) in old {
                let child = format!("{path}/{}", key.replace('~', "~0").replace('/', "~1"));
                let Some(next) = new.get(key) else {
                    return Some(format!("{child}: missing in new input"));
                };
                if let Some(found) = first_difference(value, next, &child) {
                    return Some(found);
                }
            }
            let key = new.keys().find(|key| !old.contains_key(*key)).unwrap();
            Some(format!(
                "{path}/{}: added in new input",
                key.replace('~', "~0").replace('/', "~1")
            ))
        }
        (Value::Array(old), Value::Array(new)) => {
            if old.len() != new.len() {
                return Some(format!(
                    "{path}: array lengths {} -> {}",
                    old.len(),
                    new.len()
                ));
            }
            old.iter()
                .zip(new)
                .enumerate()
                .find_map(|(index, (old, new))| {
                    first_difference(old, new, &format!("{path}/{index}"))
                })
        }
        _ => Some(format!("{path}: {} -> {}", brief(old), brief(new))),
    }
}

fn brief(value: &Value) -> String {
    match value {
        Value::String(value) => format!("{:?}", value.chars().take(96).collect::<String>()),
        Value::Array(value) => format!("array({})", value.len()),
        Value::Object(value) => format!("object({})", value.len()),
        _ => value.to_string(),
    }
}
