use poe_optimizer_core::{
    owned_definitions::*, owned_readiness::*, owned_rules::*, owned_schema::*,
};

#[test]
fn participation_is_an_optional_typed_requirement_not_an_input_or_default() {
    let namespace = GameVersionNamespace::new("participation", "test").unwrap();
    let key = |v| OwnedDefinitionKey::new(v).unwrap();
    let mut row = SkillReadiness {
        skill: SkillDefId::new(namespace.clone(), key("skill")),
        parameters: DeclaredSet::complete(vec![]),
        participation: None,
    };
    let absent = serde_json::to_value(&row).unwrap();
    assert!(absent.get("participation").is_none());
    assert_eq!(
        serde_json::from_value::<SkillReadiness>(absent.clone()).unwrap(),
        row
    );
    row.participation = Some(StatDefId::new(namespace, key("requested")));
    let explicit = serde_json::to_value(&row).unwrap();
    assert_eq!(explicit.as_object().unwrap().len(), 3);
    assert_eq!(
        serde_json::from_value::<SkillReadiness>(explicit.clone()).unwrap(),
        row
    );
    for value in [
        serde_json::Value::Null,
        serde_json::json!(true),
        serde_json::json!("requested"),
    ] {
        let mut bad = absent.clone();
        bad["participation"] = value;
        assert!(serde_json::from_value::<SkillReadiness>(bad).is_err());
    }
    let mut fabricated = explicit;
    fabricated["activation"] = serde_json::json!(true);
    assert!(serde_json::from_value::<SkillReadiness>(fabricated).is_err());
    assert!(RuleOperationsVersion::V21.supports_skill_participation());
    assert!(!RuleOperationsVersion::V20.supports_skill_participation());
}
