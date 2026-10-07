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
        ("owned-domain-operations-v14", 14, "owned-effect-plan-v11"),
        ("owned-domain-operations-v15", 15, "owned-effect-plan-v12"),
        ("owned-domain-operations-v16", 16, "owned-effect-plan-v13"),
        ("owned-domain-operations-v17", 17, "owned-effect-plan-v14"),
        ("owned-domain-operations-v18", 18, "owned-effect-plan-v15"),
        ("owned-domain-operations-v19", 19, "owned-effect-plan-v16"),
        ("owned-domain-operations-v20", 20, "owned-effect-plan-v17"),
        ("owned-domain-operations-v21", 21, "owned-effect-plan-v18"),
        ("owned-domain-operations-v22", 22, "owned-effect-plan-v19"),
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
        assert_eq!(version.supports_enemy_level(), revision >= 14);
        assert_eq!(version.supports_effect_applications(), revision >= 15);
        assert_eq!(version.supports_readiness(), revision >= 16);
        assert_eq!(version.supports_skill_inputs(), revision >= 17);
        assert_eq!(version.supports_source_properties(), revision >= 18);
        assert_eq!(version.supports_preset_skill_inputs(), revision >= 19);
        assert_eq!(version.supports_action_selection(), revision >= 20);
        assert_eq!(version.supports_contribution_queries(), revision >= 21);
        assert_eq!(version.supports_boolean_contributions(), revision >= 22);
    }
    assert_eq!(OWNED_RULE_OPERATIONS_V11, "owned-domain-operations-v11");
    assert_eq!(OWNED_RULE_OPERATIONS_V12, "owned-domain-operations-v12");
    assert_eq!(OWNED_RULE_OPERATIONS_V13, "owned-domain-operations-v13");
    assert_eq!(OWNED_RULE_OPERATIONS_V14, "owned-domain-operations-v14");
    assert_eq!(OWNED_RULE_OPERATIONS_V15, "owned-domain-operations-v15");
    assert_eq!(OWNED_RULE_OPERATIONS_V16, "owned-domain-operations-v16");
    assert_eq!(OWNED_RULE_OPERATIONS_V17, "owned-domain-operations-v17");
    assert_eq!(OWNED_RULE_OPERATIONS_V18, "owned-domain-operations-v18");
    assert_eq!(OWNED_RULE_OPERATIONS_V19, "owned-domain-operations-v19");
    assert_eq!(OWNED_RULE_OPERATIONS_V20, "owned-domain-operations-v20");
    assert_eq!(OWNED_RULE_OPERATIONS_V21, "owned-domain-operations-v21");
    assert_eq!(OWNED_RULE_OPERATIONS_V22, "owned-domain-operations-v22");
    assert_eq!(OWNED_RULE_PACKAGE_VERSION, 3);
    assert_eq!(OWNED_RULE_OPERATIONS_VERSION, OWNED_RULE_OPERATIONS_V14);
    assert!(RuleOperationsVersion::parse(OWNED_RULE_OPERATIONS_VERSION).is_some());
}

#[test]
fn native_operation_versions_are_closed_not_parsed_from_numeric_suffixes() {
    for unknown in [
        "owned-domain-operations-v5",
        "owned-domain-operations-v999",
        "owned-domain-operations-v011",
        "owned-domain-operations-v014",
        "owned-domain-operations-v017",
        "owned-domain-operations-v018",
        "owned-domain-operations-v019",
        "owned-domain-operations-v020",
        "owned-domain-operations-v021",
        "owned-domain-operations-v022",
        "different-operations",
    ] {
        assert_eq!(RuleOperationsVersion::parse(unknown), None);
    }
}

#[test]
fn enemy_level_is_an_explicit_unit_read_without_request_fields() {
    assert_eq!(
        serde_json::to_string(&RuleReadSource::EnemyLevel).unwrap(),
        r#"{"kind":"enemy_level"}"#
    );
    // Adding a unit variant does not change the historical level-read wire form.
    for (source, wire) in [
        (
            RuleReadSource::CharacterLevel,
            r#"{"kind":"character_level"}"#,
        ),
        (RuleReadSource::GemLevel, r#"{"kind":"gem_level"}"#),
        (RuleReadSource::ItemLevel, r#"{"kind":"item_level"}"#),
    ] {
        assert_eq!(serde_json::to_string(&source).unwrap(), wire);
        assert_eq!(
            serde_json::from_str::<RuleReadSource>(wire).unwrap(),
            source
        );
    }
}
