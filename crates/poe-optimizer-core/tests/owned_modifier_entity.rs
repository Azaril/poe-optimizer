//! Core is a strict wire contract; owner/context checks belong to the semantic compiler.
use poe_optimizer_core::{owned_definitions::*, owned_rules::*};
use serde_json::json;
fn stat() -> StatDefId {
    StatDefId::parse(
        GameVersionNamespace::new("entity-test", "v1").unwrap(),
        "scalar",
    )
    .unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}

#[test]
fn modifier_relative_target_roundtrips_without_reinterpreting_current() {
    for (entity, wire) in [
        (RuleEntity::Current, "current"),
        (RuleEntity::Modifier, "modifier"),
        (RuleEntity::Actor, "actor"),
        (RuleEntity::Player, "player"),
        (RuleEntity::Enemy, "enemy"),
        (RuleEntity::Environment, "environment"),
    ] {
        assert_eq!(serde_json::to_value(entity).unwrap(), json!(wire));
        assert_eq!(
            serde_json::from_value::<RuleEntity>(json!(wire)).unwrap(),
            entity
        );
    }
    assert_ne!(RuleEntity::Modifier, RuleEntity::Current);
    for value in [
        json!("item_modifier"),
        json!("provider"),
        json!({"kind":"modifier"}),
        json!(null),
    ] {
        assert!(serde_json::from_value::<RuleEntity>(value).is_err());
    }
}

#[test]
fn normal_stat_read_and_derive_carry_modifier_scope_and_remain_strict() {
    let source = RuleReadSource::Stat {
        entity: RuleEntity::Modifier,
        stat: stat(),
    };
    let wire = serde_json::to_value(&source).unwrap();
    assert_eq!(wire["value"]["entity"], "modifier");
    assert_eq!(
        serde_json::from_value::<RuleReadSource>(wire.clone()).unwrap(),
        source
    );
    let mut unknown = wire.clone();
    unknown["value"]["provider"] = json!("some-source-identity");
    assert!(serde_json::from_value::<RuleReadSource>(unknown).is_err());
    let mut wrong_id = wire;
    wrong_id["value"]["stat"]["kind"] = json!("item_template");
    assert!(serde_json::from_value::<RuleReadSource>(wrong_id).is_err());
    let effect = RuleEffectKind::Derive {
        entity: RuleEntity::Modifier,
        stat: stat(),
        value: key("computed"),
    };
    let wire = serde_json::to_value(&effect).unwrap();
    assert_eq!(wire["entity"], "modifier");
    assert_eq!(
        serde_json::from_value::<RuleEffectKind>(wire).unwrap(),
        effect
    );
    let stat = serde_json::to_string(&stat()).unwrap();
    let duplicate = format!(
        r#"{{"kind":"derive","entity":"modifier","entity":"current","stat":{stat},"value":"computed"}}"#
    );
    assert!(serde_json::from_str::<RuleEffectKind>(&duplicate).is_err());
}
