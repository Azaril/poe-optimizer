//! Inverse check for exact checked-release dependency rebinding.
//! Callers separately validate every schema, rule and registry change before
//! allowing these three graphs to differ. Everything else must survive exactly.
use poe_optimizer_import::{
    owned_normalize::NormalizationPolicy, owned_release::StagedOwnedRelease,
};
use serde_json::json;

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
    assert_eq!(policy, old.normalization);
    restored["normalization"] = old_value["normalization"].clone();
    assert_eq!(new.provenance.len(), old.provenance.len() + 1);
    assert_eq!(new.provenance[..old.provenance.len()], old.provenance);
    restored["provenance"].as_array_mut().unwrap().pop();
    assert!(
        restored == old_value,
        "every other prior release field is unchanged"
    );
}
