#[path = "support/owned_rule_receiver_fixture.rs"]
mod fixture;
use fixture::*;
use poe_optimizer_core::{owned_rules::*, owned_schema::*};
use poe_optimizer_data::{owned_rules::*, owned_schema::*};
#[test]
fn receiver_wire_v2_is_required_strict_and_canonical() {
    let s = schema();
    let l = RuleStorageLimits::default();
    let raw = input(&s);
    let package = OwnedRulePackage::new(raw.clone(), &s, l).unwrap();
    assert_eq!(package.resources().receivers, 2);
    assert_eq!(package.resources().receiver_targets, 3);
    let bytes = encode_rule_package(&package, l).unwrap();
    assert_eq!(
        decode_rule_package(&bytes, &s, l).unwrap().identity(),
        package.identity()
    );
    let mut reordered = raw.clone();
    reordered.receivers.members.reverse();
    reordered.receivers.members[0].targets.reverse();
    assert_eq!(
        OwnedRulePackage::new(reordered, &s, l).unwrap().identity(),
        package.identity()
    );
    let mut stale = raw.clone();
    stale.schema_version = 1;
    assert!(matches!(
        OwnedRulePackage::new(stale, &s, l),
        Err(RuleStorageError::Version(1))
    ));
    let mut json = serde_json::to_value(&raw).unwrap();
    json.as_object_mut().unwrap().remove("receivers");
    assert!(decode_rule_package(&serde_json::to_vec(&json).unwrap(), &s, l).is_err());
    let text = serde_json::to_string(&raw).unwrap();
    let duplicate = format!(
        "{{\"receivers\":{},{}",
        serde_json::to_string(&raw.receivers).unwrap(),
        &text[1..]
    );
    assert!(decode_rule_package(duplicate.as_bytes(), &s, l).is_err());
    for bad in [
        r#"{"kind":"player","extra":1}"#,
        r#"{"kind":"player","value":{}}"#,
        r#"{"kind":"player","kind":"player"}"#,
    ] {
        assert!(
            serde_json::from_str::<ActorReceiverTarget>(bad).is_err(),
            "{bad}"
        );
    }
    let mut changed = raw;
    changed.receivers.members[1].targets.pop();
    assert_ne!(
        OwnedRulePackage::new(changed, &s, l).unwrap().identity(),
        package.identity()
    );
}
#[test]
fn duplicate_keys_and_targets_are_rejected_even_when_identical() {
    let s = schema();
    let l = RuleStorageLimits::default();
    for case in 0..4 {
        let mut i = input(&s);
        match case {
            0 => i.receivers.members.push(i.receivers.members[0].clone()),
            1 => {
                i.receivers.members[1].stat = i.receivers.members[0].stat.clone();
                i.receivers.members[1].program = i.receivers.members[0].program.clone();
            }
            2 => {
                let t = i.receivers.members[0].targets[0].clone();
                i.receivers.members[0].targets.push(t);
            }
            _ => i.receivers.members[0].targets.clear(),
        }
        assert!(OwnedRulePackage::new(i, &s, l).is_err(), "case {case}");
    }
}
#[test]
fn receiver_program_topology_and_exact_schema_refs_are_required() {
    let s = schema();
    let l = RuleStorageLimits::default();
    for case in 0..8 {
        let mut i = input(&s);
        match case {
            0 => i.receivers.members[0].program = key("missing"),
            1 => i.receivers.members[0].stat = id("missing"),
            2 => {
                i.receivers.members[0].targets = vec![ActorReceiverTarget::OwnedSlot {
                    slot: slot("missing"),
                }]
            }
            3 => i.owners[0].programs.members[0].context = RuleEntityKind::Action,
            4 => {
                let e = i.owners[0].programs.members[0].effects[0].clone();
                i.owners[0].programs.members[0].effects.push(RuleEffect {
                    id: key("other"),
                    ..e
                });
            }
            5 => {
                if let RuleEffectKind::Derive { entity, .. } =
                    &mut i.owners[0].programs.members[0].effects[0].effect
                {
                    *entity = RuleEntity::Player;
                }
            }
            6 => {
                if let RuleEffectKind::Derive { stat, .. } =
                    &mut i.owners[0].programs.members[0].effects[0].effect
                {
                    *stat = id("other");
                }
            }
            _ => i.definitions.content_sha256 = "00".repeat(32),
        }
        assert!(OwnedRulePackage::new(i, &s, l).is_err(), "case {case}");
    }
    let mut raw = schema_input();
    if let DefinitionDescriptor::Stat(entry) = &mut raw.definitions[2] {
        entry.schema = SchemaState::Unmapped {
            gaps: vec![gap("stat-unmapped")],
        };
    }
    let unknown = OwnedDefinitionSchemaPackage::new(raw, OwnedSchemaLimits::default()).unwrap();
    assert!(OwnedRulePackage::new(input(&unknown), &unknown, l).is_err());
}
#[test]
fn empty_and_partial_registries_keep_distinct_coverage_and_gap_validation() {
    let s = schema();
    let l = RuleStorageLimits::default();
    let mut i = input(&s);
    i.receivers.members.clear();
    let empty = OwnedRulePackage::new(i.clone(), &s, l).unwrap();
    i.receivers.closure = SchemaClosure::Partial {
        gaps: vec![gap("not-converted")],
    };
    let partial = OwnedRulePackage::new(i.clone(), &s, l).unwrap();
    assert!(!partial.input().receivers.is_complete());
    assert_eq!(partial.resources().gaps, 1);
    assert_ne!(empty.identity(), partial.identity());
    for gaps in [
        vec![],
        vec![gap("same"), gap("same")],
        vec![SchemaGap {
            facet: SchemaFacet::StaticLinks,
            ..gap("wrong-facet")
        }],
        vec![SchemaGap {
            subject: SchemaSubject::Definition(DefinitionAddress::Stat(id("missing"))),
            ..gap("missing")
        }],
    ] {
        i.receivers.closure = SchemaClosure::Partial { gaps };
        assert!(OwnedRulePackage::new(i.clone(), &s, l).is_err());
    }
}
#[test]
fn aggregate_receiver_and_gap_limits_hold_during_new_decode_and_encode() {
    let s = schema();
    let l = RuleStorageLimits::default();
    let mut i = input(&s);
    i.receivers.closure = SchemaClosure::Partial {
        gaps: vec![gap("first"), gap("second")],
    };
    let p = OwnedRulePackage::new(i.clone(), &s, l).unwrap();
    let bytes = encode_rule_package(&p, l).unwrap();
    for tight in [
        RuleStorageLimits {
            max_receivers: 1,
            ..l
        },
        RuleStorageLimits {
            max_receiver_targets: 2,
            ..l
        },
        RuleStorageLimits { max_gaps: 1, ..l },
        RuleStorageLimits {
            max_receiver_work: 1,
            ..l
        },
        RuleStorageLimits {
            max_wire_bytes: 1,
            ..l
        },
    ] {
        assert!(OwnedRulePackage::new(i.clone(), &s, tight).is_err());
        assert!(decode_rule_package(&bytes, &s, tight).is_err());
        assert!(encode_rule_package(&p, tight).is_err());
    }
    for invalid in [
        RuleStorageLimits {
            max_receivers: 0,
            ..l
        },
        RuleStorageLimits {
            max_receiver_targets: 0,
            ..l
        },
        RuleStorageLimits {
            max_receiver_work: 0,
            ..l
        },
    ] {
        assert!(OwnedRulePackage::new(i.clone(), &s, invalid).is_err());
    }
}
