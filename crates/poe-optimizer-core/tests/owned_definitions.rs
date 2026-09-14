use poe_optimizer_core::owned_definitions::*;
use serde_json::{Value, json};
use std::collections::HashSet;

fn namespace() -> GameVersionNamespace {
    GameVersionNamespace::new("authored-game", "0.1-test").unwrap()
}
fn unit() -> UnitDefId {
    UnitDefId::parse(namespace(), "resource.test").unwrap()
}

#[test]
fn symbols_are_bounded_and_never_normalize_external_text() {
    for text in ["a", "1", "test.output_2-a", "0.1-test"] {
        let key = OwnedDefinitionKey::new(text).unwrap();
        assert_eq!(key.as_str(), text);
        assert_eq!(key.to_string(), text);
        assert_eq!(text.parse::<OwnedDefinitionKey>().unwrap(), key);
        assert_eq!(
            serde_json::from_str::<OwnedDefinitionKey>(&serde_json::to_string(&key).unwrap())
                .unwrap(),
            key
        );
    }
    let maximum = "a".repeat(MAX_OWNED_DEFINITION_KEY_BYTES);
    assert!(OwnedDefinitionKey::new(&maximum).is_ok());
    assert!(serde_json::from_value::<OwnedDefinitionKey>(json!(maximum)).is_ok());
    for text in [
        "",
        "A",
        "has space",
        " leading",
        "trailing ",
        "line\nbreak",
        "_prefix",
        ".prefix",
        "-prefix",
        "source/id",
        "source:id",
        "unicod\u{00e9}",
        "\0",
    ] {
        assert!(
            OwnedDefinitionKey::new(text).is_err(),
            "constructed {text:?}"
        );
        assert!(
            serde_json::from_value::<OwnedDefinitionKey>(json!(text)).is_err(),
            "decoded {text:?}"
        );
    }
    let oversized = "a".repeat(MAX_OWNED_DEFINITION_KEY_BYTES + 1);
    assert!(matches!(
        OwnedDefinitionKey::new(&oversized),
        Err(OwnedDefinitionError::KeyTooLong { .. })
    ));
    assert!(serde_json::from_value::<OwnedDefinitionKey>(json!(oversized)).is_err());
    for invalid in [
        json!(null),
        json!(10),
        json!(["symbol"]),
        json!({"key":"symbol"}),
    ] {
        assert!(serde_json::from_value::<OwnedDefinitionKey>(invalid).is_err());
    }
}

#[test]
fn namespace_fields_validate_without_package_loading_or_fallbacks() {
    let original = namespace();
    assert_eq!(original.game().as_str(), "authored-game");
    assert_eq!(original.version().as_str(), "0.1-test");
    assert_eq!(
        original,
        GameVersionNamespace::from_keys(
            OwnedDefinitionKey::new("authored-game").unwrap(),
            OwnedDefinitionKey::new("0.1-test").unwrap(),
        )
    );
    assert_ne!(
        original,
        GameVersionNamespace::new("other-game", "0.1-test").unwrap()
    );
    assert_ne!(
        original,
        GameVersionNamespace::new("authored-game", "0.2-test").unwrap()
    );
    assert!(GameVersionNamespace::new("", "v1").is_err());
    assert!(GameVersionNamespace::new("game", "Latest Version").is_err());
    for invalid in [
        json!({"game":"test","version":"v1","source_hash":"deadbeef"}),
        json!({"game":"test"}),
        json!({"game":"test","version":""}),
        json!({"game":"test","version":"a".repeat(MAX_OWNED_DEFINITION_KEY_BYTES + 1)}),
    ] {
        assert!(serde_json::from_value::<GameVersionNamespace>(invalid).is_err());
    }
    assert!(
        serde_json::from_str::<GameVersionNamespace>(r#"{"game":"a","game":"b","version":"v1"}"#)
            .is_err()
    );
}

#[test]
fn every_definition_domain_retains_its_wire_kind() {
    macro_rules! check {
        ($($id:ty => $kind:literal),+ $(,)?) => { $(
            let id = <$id>::parse(namespace(), "same.symbol").unwrap();
            let wire = serde_json::to_value(&id).unwrap();
            assert_eq!(id.kind().as_str(), $kind);
            assert_eq!(wire["kind"], $kind);
            assert_eq!(wire["key"], "same.symbol");
            assert_eq!(wire["namespace"], json!({"game":"authored-game","version":"0.1-test"}));
            assert_eq!(serde_json::from_value::<$id>(wire.clone()).unwrap(), id);
            let mut other = wire;
            other["kind"] = json!(if $kind == "class" { "skill" } else { "class" });
            assert!(serde_json::from_value::<$id>(other).is_err(), "cross-kind {}", $kind);
        )+ };
    }
    check! {
        ClassDefId => "class", AscendancyDefId => "ascendancy", RewardDefId => "reward",
        ItemTemplateDefId => "item_template", ModifierDefId => "modifier", GemDefId => "gem",
        SkillDefId => "skill", PassiveNodeDefId => "passive_node", PointPoolDefId => "point_pool",
        EquipmentSlotDefId => "equipment_slot", EncounterDefId => "encounter", MetricDefId => "metric",
        ParameterSlotDefId => "parameter_slot", ChoiceSlotDefId => "choice_slot", OptionDefId => "option",
        GrantSlotDefId => "grant_slot", ActorSlotDefId => "actor_slot", SkillGrantSlotDefId => "skill_grant_slot",
        ActionOutputDefId => "action_output", ActionPartDefId => "action_part", ActionModeDefId => "action_mode",
        ActionStatSetDefId => "action_stat_set", UsagePolicyDefId => "usage_policy", SkillLinkRoleDefId => "skill_link_role",
        SocketSlotDefId => "socket_slot", UnitDefId => "unit", QualityDefId => "quality", ExternalInputDefId => "external_input",
    }
}

#[test]
fn references_require_explicit_typed_fields_and_preserve_namespace_identity() {
    let id = SkillDefId::new(
        namespace(),
        OwnedDefinitionKey::new("uninstalled.output").unwrap(),
    );
    assert_eq!(id.key().as_str(), "uninstalled.output");
    assert_eq!(id.namespace(), &namespace());
    let alternate = SkillDefId::parse(
        GameVersionNamespace::new("authored-game", "0.2-test").unwrap(),
        "uninstalled.output",
    )
    .unwrap();
    assert_ne!(id, alternate);
    let wire = serde_json::to_value(&id).unwrap();
    for field in ["kind", "namespace", "key"] {
        let mut missing = wire.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<SkillDefId>(missing).is_err());
    }
    for field in ["source_hash", "display_name", "dense_index"] {
        let mut extra = wire.clone();
        extra[field] = json!("not-authority");
        assert!(serde_json::from_value::<SkillDefId>(extra).is_err());
    }
    for key in [
        json!(12),
        json!(""),
        json!("external/path"),
        json!("a".repeat(MAX_OWNED_DEFINITION_KEY_BYTES + 1)),
    ] {
        let mut invalid = wire.clone();
        invalid["key"] = key;
        assert!(serde_json::from_value::<SkillDefId>(invalid).is_err());
    }
    let duplicate =
        r#"{"kind":"skill","kind":"skill","namespace":{"game":"a","version":"v1"},"key":"b"}"#;
    assert!(serde_json::from_str::<SkillDefId>(duplicate).is_err());
}

#[test]
fn declaring_owner_is_closed_and_cannot_relabel_another_definition_kind() {
    let owners = [
        SlotOwnerDefId::Class(ClassDefId::parse(namespace(), "declaring").unwrap()),
        SlotOwnerDefId::Ascendancy(AscendancyDefId::parse(namespace(), "declaring").unwrap()),
        SlotOwnerDefId::Reward(RewardDefId::parse(namespace(), "declaring").unwrap()),
        SlotOwnerDefId::ItemTemplate(ItemTemplateDefId::parse(namespace(), "declaring").unwrap()),
        SlotOwnerDefId::Modifier(ModifierDefId::parse(namespace(), "declaring").unwrap()),
        SlotOwnerDefId::Gem(GemDefId::parse(namespace(), "declaring").unwrap()),
        SlotOwnerDefId::Skill(SkillDefId::parse(namespace(), "declaring").unwrap()),
        SlotOwnerDefId::PassiveNode(PassiveNodeDefId::parse(namespace(), "declaring").unwrap()),
        SlotOwnerDefId::UsagePolicy(UsagePolicyDefId::parse(namespace(), "declaring").unwrap()),
    ];
    for owner in owners {
        assert_eq!(owner.namespace(), &namespace());
        assert_eq!(owner.key().as_str(), "declaring");
        let wire = serde_json::to_value(&owner).unwrap();
        assert_eq!(wire["kind"], owner.kind().as_str());
        assert_eq!(
            serde_json::from_value::<SlotOwnerDefId>(wire.clone()).unwrap(),
            owner
        );
        let mut extra = wire.clone();
        extra["unrecognized"] = json!(true);
        assert!(serde_json::from_value::<SlotOwnerDefId>(extra).is_err());
        let mut wrong = wire;
        wrong["definition"]["kind"] = json!("metric");
        assert!(serde_json::from_value::<SlotOwnerDefId>(wrong).is_err());
    }
    let metric = MetricDefId::parse(namespace(), "declaring").unwrap();
    assert!(
        serde_json::from_value::<SlotOwnerDefId>(json!({"kind":"metric","definition":metric}))
            .is_err()
    );
}

#[test]
fn finite_quantities_normalize_zero_and_preserve_exact_finite_binary_values() {
    let positive = FiniteQuantity::new(0.0, unit()).unwrap();
    let negative = FiniteQuantity::new(-0.0, unit()).unwrap();
    assert_eq!(positive, negative);
    assert_eq!(negative.value().to_bits(), 0);
    assert_eq!(HashSet::from([positive.clone(), negative.clone()]).len(), 1);
    assert_eq!(
        serde_json::to_string(&positive).unwrap(),
        serde_json::to_string(&negative).unwrap()
    );
    let mut wire = serde_json::to_value(&positive).unwrap();
    wire["value"] = json!(-0.0);
    assert_eq!(
        serde_json::from_value::<FiniteQuantity>(wire)
            .unwrap()
            .value()
            .to_bits(),
        0
    );
    for value in [
        0.0,
        -0.0,
        0.1,
        -17.25,
        f64::MAX,
        -f64::MAX,
        f64::MIN_POSITIVE,
        f64::from_bits(1),
        1e-300,
        1e300,
        f64::from_bits(1.0_f64.to_bits() + 1),
    ] {
        let quantity = FiniteQuantity::new(value, unit()).unwrap();
        let decoded: FiniteQuantity =
            serde_json::from_str(&serde_json::to_string(&quantity).unwrap()).unwrap();
        assert_eq!(decoded.value().to_bits(), quantity.value().to_bits());
        assert_eq!(decoded.unit(), &unit());
    }
    let other_unit = UnitDefId::parse(namespace(), "other.resource").unwrap();
    assert_ne!(positive, FiniteQuantity::new(0.0, other_unit).unwrap());
}

#[test]
fn finite_quantity_decode_cannot_bypass_value_or_unit_validation() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            FiniteQuantity::new(value, unit()),
            Err(OwnedDefinitionError::NonFiniteQuantity)
        );
    }
    let wire = serde_json::to_value(FiniteQuantity::new(1.0, unit()).unwrap()).unwrap();
    for value in [
        Value::Null,
        json!("NaN"),
        json!("Infinity"),
        json!({"expression":"anything"}),
    ] {
        let mut invalid = wire.clone();
        invalid["value"] = value;
        assert!(serde_json::from_value::<FiniteQuantity>(invalid).is_err());
    }
    let mut invalid = wire.clone();
    invalid["unit"]["kind"] = json!("metric");
    assert!(serde_json::from_value::<FiniteQuantity>(invalid).is_err());
    let mut invalid = wire.clone();
    invalid["extra"] = json!(0);
    assert!(serde_json::from_value::<FiniteQuantity>(invalid).is_err());
    let encoded_unit = serde_json::to_string(&unit()).unwrap();
    assert!(
        serde_json::from_str::<FiniteQuantity>(&format!(
            r#"{{"value":1e999,"unit":{encoded_unit}}}"#
        ))
        .is_err()
    );
    assert!(
        serde_json::from_str::<FiniteQuantity>(&format!(
            r#"{{"value":1,"value":2,"unit":{encoded_unit}}}"#
        ))
        .is_err()
    );
}

#[test]
fn integral_values_keep_exact_browser_roundtrips_and_reject_fractional_or_oversized_input() {
    for value in [BoundedInteger::MIN, -1, 0, 1, BoundedInteger::MAX] {
        let integer = BoundedInteger::new(value).unwrap();
        assert_eq!(integer.get(), value);
        let encoded = serde_json::to_string(&integer).unwrap();
        assert_eq!(
            serde_json::from_str::<BoundedInteger>(&encoded).unwrap(),
            integer
        );
        assert_eq!(encoded.parse::<f64>().unwrap() as i64, value);
    }
    for value in [
        i64::MIN,
        BoundedInteger::MIN - 1,
        BoundedInteger::MAX + 1,
        i64::MAX,
    ] {
        assert!(BoundedInteger::new(value).is_err());
        assert!(serde_json::from_value::<BoundedInteger>(json!(value)).is_err());
    }
    for invalid in [
        "1.5",
        "1.0",
        "null",
        "true",
        "\"1\"",
        "18446744073709551615",
    ] {
        assert!(
            serde_json::from_str::<BoundedInteger>(invalid).is_err(),
            "decoded {invalid}"
        );
    }
}
