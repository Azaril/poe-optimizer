//! Action selection stays an exact typed request input across the wire boundary.
use poe_optimizer_core::{owned_definitions::*, owned_rules::*};
use serde_json::{Value, json};

fn definition<K: DefinitionDomain>(name: &str) -> DefId<K> {
    DefId::new(
        GameVersionNamespace::new("test-game", "selection-v1").unwrap(),
        OwnedDefinitionKey::new(name).unwrap(),
    )
}

#[test]
fn action_predicates_round_trip_typed_identifiers_without_source_ordinals() {
    for (read, kind, field, id) in [
        (
            RuleReadSource::ActionPartIs {
                part: definition("part-a"),
            },
            "action_part_is",
            "part",
            json!(definition::<ActionPartDefinition>("part-a")),
        ),
        (
            RuleReadSource::ActionModeIs {
                mode: definition("mode-a"),
            },
            "action_mode_is",
            "mode",
            json!(definition::<ActionModeDefinition>("mode-a")),
        ),
        (
            RuleReadSource::ActionStatSetIs {
                stat_set: definition("set-a"),
            },
            "action_stat_set_is",
            "stat_set",
            json!(definition::<ActionStatSetDefinition>("set-a")),
        ),
    ] {
        let wire = json!({"kind":kind,"value":{field:id}});
        assert_eq!(serde_json::to_value(&read).unwrap(), wire);
        assert_eq!(
            serde_json::from_value::<RuleReadSource>(wire).unwrap(),
            read
        );
    }
}

#[test]
fn action_predicates_reject_missing_null_cross_domain_and_untyped_selectors() {
    let cases = [
        (
            "action_part_is",
            "part",
            json!(definition::<ActionModeDefinition>("other")),
        ),
        (
            "action_mode_is",
            "mode",
            json!(definition::<ActionStatSetDefinition>("other")),
        ),
        (
            "action_stat_set_is",
            "stat_set",
            json!(definition::<ActionPartDefinition>("other")),
        ),
    ];
    for (kind, field, wrong_domain) in cases {
        for body in [
            json!({}),
            json!({field:Value::Null}),
            json!({field:1}),
            json!({field:"source-index-1"}),
            json!({field:wrong_domain}),
        ] {
            assert!(
                serde_json::from_value::<RuleReadSource>(json!({"kind":kind,"value":body}))
                    .is_err()
            );
        }
    }
    let valid = json!(RuleReadSource::ActionStatSetIs {
        stat_set: definition("set-a")
    });
    for field in ["source_index", "default", "provider"] {
        let mut extra = valid.clone();
        extra["value"][field] = json!(1);
        assert!(serde_json::from_value::<RuleReadSource>(extra).is_err());
    }
}
