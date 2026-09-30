//! Frozen native operation capabilities must not follow the moving latest alias.
use poe_optimizer_core::owned_rules::*;

#[test]
fn explicit_operation_versions_preserve_capabilities_and_plan_domains() {
    for (text, revision, domain) in [
        ("owned-domain-operations-v6", 6, "owned-effect-plan-v6"),
        ("owned-domain-operations-v7", 7, "owned-effect-plan-v6"),
        ("owned-domain-operations-v8", 8, "owned-effect-plan-v6"),
        ("owned-domain-operations-v9", 9, "owned-effect-plan-v6"),
        ("owned-domain-operations-v10", 10, "owned-effect-plan-v7"),
        ("owned-domain-operations-v11", 11, "owned-effect-plan-v8"),
        ("owned-domain-operations-v12", 12, "owned-effect-plan-v9"),
        ("owned-domain-operations-v13", 13, "owned-effect-plan-v10"),
    ] {
        let version = RuleOperationsVersion::parse(text).unwrap();
        assert_eq!(version.revision(), revision);
        assert_eq!(version.effect_plan_domain(), domain);
        assert_eq!(version.supports_character_identity(), revision >= 7);
        assert_eq!(version.supports_quantize_integer(), revision >= 8);
        assert_eq!(version.supports_equipment_receivers(), revision >= 9);
        assert_eq!(version.supports_modifier_transforms(), revision >= 10);
        assert_eq!(version.supports_actor_supply(), revision >= 11);
        assert_eq!(version.supports_preparation_scopes(), revision >= 12);
        assert_eq!(
            version.supports_actor_support_applicability(),
            revision >= 13
        );
    }
    assert_eq!(OWNED_RULE_OPERATIONS_V11, "owned-domain-operations-v11");
    assert_eq!(OWNED_RULE_OPERATIONS_V12, "owned-domain-operations-v12");
    assert_eq!(OWNED_RULE_OPERATIONS_V13, "owned-domain-operations-v13");
    assert_eq!(OWNED_RULE_OPERATIONS_VERSION, OWNED_RULE_OPERATIONS_V13);
    assert!(RuleOperationsVersion::parse(OWNED_RULE_OPERATIONS_VERSION).is_some());
}

#[test]
fn native_operation_versions_are_closed_not_parsed_from_numeric_suffixes() {
    for unknown in [
        "owned-domain-operations-v5",
        "owned-domain-operations-v999",
        "owned-domain-operations-v011",
        "different-operations",
    ] {
        assert_eq!(RuleOperationsVersion::parse(unknown), None);
    }
}
