use poe_optimizer_import::owned_actor_baselines::{
    ActorBaselineProfile, ActorDamageLevelTable, ActorScalarFact,
};
fn profile() -> serde_json::Value {
    serde_json::json!({
        "key":"actor", "name":"Actor", "source_module":"offline",
        "attack_time":0.0,"damage_scale":null,"damage_spread":null,
        "critical_chance":null,"attack_range":null,"weapon_family":null,
        "hostile":false,"base_damage_ignores_attack_speed":null,
        "child_skills":[],"extra_facts":{},"unconverted_fields":[],
        "unconverted_modifiers":[],"unconverted_flags":{}
    })
}
#[test]
fn typed_actor_facts_preserve_absence_zero_false_and_empty_collections() {
    let p: ActorBaselineProfile = serde_json::from_value(profile()).unwrap();
    assert_eq!(p.attack_time, Some(0.0));
    assert_eq!(p.damage_scale, None);
    assert_eq!(p.hostile, Some(false));
    assert_eq!(p.unconverted_modifiers, Some(vec![]));
    assert!(p.unconverted_flags.as_ref().unwrap().is_empty());
    let mut value = profile();
    value["unconverted_modifiers"] = serde_json::Value::Null;
    value["unconverted_flags"] = serde_json::Value::Null;
    let absent: ActorBaselineProfile = serde_json::from_value(value).unwrap();
    assert_eq!(absent.unconverted_modifiers, None);
    assert_eq!(absent.unconverted_flags, None);
    assert_ne!(p, absent);
}
#[test]
fn duplicate_scalar_facts_flags_and_unknown_fields_are_rejected() {
    let encoded = serde_json::to_string(&profile()).unwrap();
    let duplicates = encoded.replace("\"extra_facts\":{}", "\"extra_facts\":{\"life\":{\"kind\":\"number\",\"value\":1},\"life\":{\"kind\":\"number\",\"value\":2}}");
    assert!(serde_json::from_str::<ActorBaselineProfile>(&duplicates).is_err());
    let duplicates = encoded.replace(
        "\"unconverted_flags\":{}",
        "\"unconverted_flags\":{\"f\":true,\"f\":false}",
    );
    assert!(serde_json::from_str::<ActorBaselineProfile>(&duplicates).is_err());
    let mut value = profile();
    value["source_program"] = "return anything".into();
    assert!(serde_json::from_value::<ActorBaselineProfile>(value).is_err());
}
#[test]
fn numeric_payloads_reject_nonfinite_and_wrong_types() {
    for json in [
        r#"{"kind":"number","value":1e400}"#,
        r#"{"kind":"number","value":null}"#,
        r#"{"kind":"number","value":"NaN"}"#,
    ] {
        assert!(serde_json::from_str::<ActorScalarFact>(json).is_err());
    }
    assert!(
        serde_json::from_str::<ActorDamageLevelTable>(r#"{"first_level":1,"rows":[1,1e400]}"#)
            .is_err()
    );
    let mut value = profile();
    value["attack_time"] = "zero".into();
    assert!(serde_json::from_value::<ActorBaselineProfile>(value).is_err());
}
