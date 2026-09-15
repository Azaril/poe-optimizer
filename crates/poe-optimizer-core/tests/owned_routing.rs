use poe_optimizer_core::{owned_definitions::*, owned_routing::*};
use serde_json::json;
fn id<K: DefinitionDomain>(s: &str) -> DefId<K> {
    DefId::parse(GameVersionNamespace::new("test", "v1").unwrap(), s).unwrap()
}
#[test]
fn selectors_are_explicit_and_exact_preserves_all_dimensions() {
    let exact = ActionRouteSelection::Exact(Box::new(ActionRouteSelector {
        part: id("part"),
        mode: id("mode"),
        stat_set: id("set"),
    }));
    let value = serde_json::to_value(&exact).unwrap();
    assert_eq!(
        serde_json::from_value::<ActionRouteSelection>(value.clone()).unwrap(),
        exact
    );
    assert_eq!(
        value,
        json!({
            "kind": "exact",
            "value": {
                "part": id::<ActionPartDefinition>("part"),
                "mode": id::<ActionModeDefinition>("mode"),
                "stat_set": id::<ActionStatSetDefinition>("set")
            }
        })
    );
    let raw = serde_json::to_string(&value).unwrap();
    for field in ["part", "mode", "stat_set"] {
        let needle = format!("\"{field}\":{}", value["value"][field]);
        let duplicate = raw.replacen(&needle, &format!("{needle},{needle}"), 1);
        assert_ne!(duplicate, raw);
        assert!(serde_json::from_str::<ActionRouteSelection>(&duplicate).is_err());
    }
    let mut missing = value.clone();
    missing["value"].as_object_mut().unwrap().remove("mode");
    assert!(serde_json::from_value::<ActionRouteSelection>(missing).is_err());
    let mut unknown = value;
    unknown["value"]["fallback"] = json!(true);
    assert!(serde_json::from_value::<ActionRouteSelection>(unknown).is_err());
    assert_eq!(
        serde_json::from_value::<ActionRouteSelection>(json!({"kind":"all"})).unwrap(),
        ActionRouteSelection::All
    );
    assert!(
        serde_json::from_value::<ActionRouteSelection>(json!({"kind":"all","fallback":true}))
            .is_err()
    );
    assert!(serde_json::from_value::<ActionRouteSelection>(json!(null)).is_err());
}
#[test]
fn transport_sources_cannot_be_unknown_or_cross_kind() {
    let source = ActionStatRouteSource::PlayerEquipment {
        slot: id("weapon"),
        stat: id("stat"),
    };
    let mut value = serde_json::to_value(&source).unwrap();
    assert_eq!(
        serde_json::from_value::<ActionStatRouteSource>(value.clone()).unwrap(),
        source
    );
    value["value"]["slot"]["kind"] = json!("stat");
    assert!(serde_json::from_value::<ActionStatRouteSource>(value).is_err());
    assert!(
        serde_json::from_value::<ActionStatRouteSource>(json!({"kind":"selected_weapon"})).is_err()
    );
}
