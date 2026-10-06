use poe_optimizer_core::{owned_definitions::*, owned_rules::*};

fn id<K: DefinitionDomain>(name: &str) -> DefId<K> {
    DefId::new(
        GameVersionNamespace::new("equipment-slot", "test").unwrap(),
        OwnedDefinitionKey::new(name).unwrap(),
    )
}

#[test]
fn player_slot_wire_has_only_explicit_occupancy_and_computed_outputs() {
    for read in [
        PlayerEquipmentSlotRead::Occupied,
        PlayerEquipmentSlotRead::Stat { stat: id("amount") },
        PlayerEquipmentSlotRead::Capability {
            capability: id("profile"),
        },
    ] {
        let source = RuleReadSource::PlayerEquipmentSlot {
            slot: id("hand"),
            read,
        };
        let json = serde_json::to_value(&source).unwrap();
        assert_eq!(json["kind"], "player_equipment_slot");
        assert_eq!(
            serde_json::from_value::<RuleReadSource>(json.clone()).unwrap(),
            source
        );
        let mut bad = json.clone();
        bad["value"]["raw_parameter"] = serde_json::json!("level");
        assert!(serde_json::from_value::<RuleReadSource>(bad).is_err());
        let mut bad = json.clone();
        bad["value"]["read"] = serde_json::Value::Null;
        assert!(serde_json::from_value::<RuleReadSource>(bad).is_err());
        let mut bad = json;
        bad["value"]["read"] = serde_json::json!({"kind":"parameter","value":{"slot":"level"}});
        assert!(serde_json::from_value::<RuleReadSource>(bad).is_err());
    }
    assert!(RuleOperationsVersion::V21.supports_player_equipment_slots());
    assert!(!RuleOperationsVersion::V20.supports_player_equipment_slots());
}
