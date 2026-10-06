use poe_optimizer_core::{owned_definitions::*, owned_rules::*};

#[test]
fn player_application_has_no_build_occurrence_or_supply_identity() {
    let application = ExistingActorRuleApplication {
        id: OwnedDefinitionKey::new("player-intrinsics").unwrap(),
        owner: ActorDefId::new(
            GameVersionNamespace::new("game", "test").unwrap(),
            OwnedDefinitionKey::new("intrinsics").unwrap(),
        ),
        targets: vec![ExistingActorRuleTarget::Player],
    };
    let json = serde_json::to_value(&application).unwrap();
    assert_eq!(json["targets"], serde_json::json!(["player"]));
    assert_eq!(json.as_object().unwrap().len(), 3);
    assert_eq!(
        serde_json::from_value::<ExistingActorRuleApplication>(json.clone()).unwrap(),
        application
    );
    for target in ["owned_slot", "all_actors", "enemy"] {
        let mut unsupported = json.clone();
        unsupported["targets"][0] = serde_json::json!(target);
        assert!(serde_json::from_value::<ExistingActorRuleApplication>(unsupported).is_err());
    }
    let mut fabricated = json;
    fabricated["provider"] = serde_json::json!({"root":"character", "grant_path":[]});
    assert!(serde_json::from_value::<ExistingActorRuleApplication>(fabricated).is_err());
}
