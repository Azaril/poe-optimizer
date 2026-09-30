//! Final type channels remain distinct from initial inputs and ordinary writers.
#[path = "support/owned_support_output_fixture.rs"]
mod fixture;
use fixture::*;
use poe_optimizer_core::{
    owned_build::{DeclaredSlot, ParameterValue},
    owned_content::digest_owned,
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
    owned_stages::*,
    owned_support_outputs::*,
};
use poe_optimizer_data::owned_support_outputs::*;

#[test]
fn canonical_complete_type_map_roundtrips_with_exact_dependencies() {
    let f = Fixture::new();
    let package = f.build(f.input.clone()).unwrap();
    assert_eq!(package.resources().entries, 2);
    let mut reversed = f.input.clone();
    reversed.final_skill_types.reverse();
    let reordered = f.build(reversed).unwrap();
    assert_eq!(package.identity(), reordered.identity());
    package.verify_bindings(&f.dependencies()).unwrap();
    let wire = encode_support_output_bindings(&package, Default::default()).unwrap();
    let decoded =
        decode_support_output_bindings(&wire, &f.dependencies(), Default::default()).unwrap();
    assert_eq!(decoded.identity(), package.identity());
    assert_eq!(decoded.input(), package.input());
    assert_eq!(
        encode_support_output_bindings(&decoded, Default::default()).unwrap(),
        wire
    );
    let mut swapped = f.input.clone();
    let stat = swapped.final_skill_types[0].stat.clone();
    swapped.final_skill_types[0].stat = swapped.final_skill_types[1].stat.clone();
    swapped.final_skill_types[1].stat = stat;
    assert_ne!(f.build(swapped).unwrap().identity(), package.identity());
}

#[test]
fn output_channels_require_exact_type_vocabulary_and_unique_stats() {
    let f = Fixture::new();
    for case in 0..5 {
        let mut input = f.input.clone();
        match case {
            0 => {
                input.final_skill_types.pop();
            }
            1 => input
                .final_skill_types
                .push(input.final_skill_types[0].clone()),
            2 => {
                input.final_skill_types[1].support_type =
                    input.final_skill_types[0].support_type.clone()
            }
            3 => input.final_skill_types[0].support_type = key("unlisted"),
            _ => input.final_skill_types[1].stat = input.final_skill_types[0].stat.clone(),
        }
        assert!(matches!(
            f.build(input),
            Err(SupportOutputStorageError::Invalid(_))
        ));
    }
}

#[test]
fn only_known_boolean_exclusively_skill_channels_are_accepted() {
    let f = Fixture::new();
    for stat in [
        "wrong-value",
        "wrong-scope",
        "multiple-scopes",
        "unmapped",
        "missing",
    ] {
        let mut input = f.input.clone();
        input.final_skill_types[0].stat = id(stat);
        assert!(matches!(
            f.build(input),
            Err(SupportOutputStorageError::Invalid(_))
        ));
    }
    let mut input = f.input.clone();
    input.final_skill_types[0].stat = StatDefId::new(
        GameVersionNamespace::new("foreign", "v1").unwrap(),
        key("final-duration"),
    );
    assert!(matches!(
        f.build(input),
        Err(SupportOutputStorageError::Invalid(_))
    ));
}

#[test]
fn no_initial_membership_flag_presence_level_or_quality_can_be_aliased() {
    let f = Fixture::new();
    let mut initial = initial_booleans(&f.inputs.input().target);
    initial.push(f.inputs.input().effective_level.clone());
    initial.push(f.inputs.input().effective_quality.clone());
    for stat in initial {
        let mut input = f.input.clone();
        input.final_skill_types[0].stat = stat;
        assert!(matches!(
            f.build(input),
            Err(SupportOutputStorageError::Invalid(_))
        ));
    }
}

#[test]
fn output_stage_strictly_follows_preparation_and_matches_every_freeze() {
    let f = Fixture::new();
    for stage in ["prepare", "independent", "missing", "consume"] {
        let mut input = f.input.clone();
        input.output_stage = key(stage);
        assert!(matches!(
            f.build(input),
            Err(SupportOutputStorageError::Invalid(_))
        ));
    }
    for frozen in [None, Some("prepare"), Some("consume")] {
        let f = Fixture::with(
            |rules| {
                rules.owners.clear();
            },
            |stages| {
                let at = stages.frozen_channels.iter().position(|r| matches!(&r.channel, StageChannel::Stat { stat, .. } if stat == &id("final-duration"))).unwrap();
                if let Some(stage) = frozen {
                    stages.frozen_channels[at].stage = key(stage);
                } else {
                    stages.frozen_channels.remove(at);
                }
            },
        );
        assert!(matches!(
            f.build(f.input.clone()),
            Err(SupportOutputStorageError::Invalid(
                "output stat must freeze exactly at output stage"
            ))
        ));
    }
}

#[test]
fn known_consumers_require_an_explicit_strictly_later_stage() {
    let same_stage = Fixture::with(|_| {}, |s| s.programs.members[0].stage = key("output"));
    assert!(matches!(
        same_stage.build(same_stage.input.clone()),
        Err(SupportOutputStorageError::Invalid(
            "output consumer must follow output stage"
        ))
    ));
    let unclassified = Fixture::with(
        |_| {},
        |s| {
            s.programs.members.clear();
            s.programs.closure = partial();
        },
    );
    assert!(matches!(
        unclassified.build(unclassified.input.clone()),
        Err(SupportOutputStorageError::Invalid(
            "output consumer has no declared stage"
        ))
    ));
}

#[test]
fn partial_whole_owner_does_not_hide_known_consumers_or_gain_closure() {
    let f = Fixture::with(|r| r.owners[0].programs.closure = partial(), |_| {});
    f.build(f.input.clone()).unwrap();
    assert!(!f.rules.input().owners[0].programs.is_complete());
    let bad = Fixture::with(
        |r| r.owners[0].programs.closure = partial(),
        |s| s.programs.members[0].stage = key("output"),
    );
    assert!(matches!(
        bad.build(bad.input.clone()),
        Err(SupportOutputStorageError::Invalid(_))
    ));
}

#[test]
fn output_reads_cannot_be_reinterpreted_as_actor_stats_or_numeric_collections() {
    for case in 0..5 {
        let f = Fixture::with(
            |r| {
                let p = &mut r.owners[0].programs.members[0];
                match case {
                    0 => {
                        p.reads[0].source = RuleReadSource::Stat {
                            entity: RuleEntity::Player,
                            stat: id("final-duration"),
                        }
                    }
                    1 => p.reads[0].value_type = ComputedValueType::Integer,
                    2 => {
                        p.reads[0].source = RuleReadSource::Contributions {
                            entity: RuleEntity::Skill,
                            stat: id("final-duration"),
                            contribution: ContributionKind::Add,
                            reduction: ContributionReduction::Sum,
                            empty: ParameterValue::Boolean(false),
                        }
                    }
                    3 => {
                        p.reads[0].source = RuleReadSource::ModifierTransforms {
                            stat: id("final-duration"),
                            initial: id("wrong-value"),
                        }
                    }
                    _ => {
                        p.reads[0].source = RuleReadSource::ModifierTransforms {
                            stat: id("wrong-value"),
                            initial: id("final-duration"),
                        }
                    }
                }
            },
            |_| {},
        );
        assert!(matches!(
            f.build(f.input.clone()),
            Err(SupportOutputStorageError::Invalid(_))
        ));
    }
}

#[test]
fn ordinary_potential_writers_cannot_claim_native_output_even_when_false_guarded() {
    for case in 0..4 {
        let f = Fixture::with(
            |r| {
                r.owners[0].programs.closure = partial();
                let p = &mut r.owners[0].programs.members[0];
                p.reads.clear();
                p.nodes[0].expression = RuleExpression::Literal {
                    value: ParameterValue::Boolean(true),
                };
                p.effects[0].when = Some(key("false"));
                p.effects[0].effect = match case {
                    0 => RuleEffectKind::Derive {
                        entity: RuleEntity::Current,
                        stat: id("final-duration"),
                        value: key("value"),
                    },
                    1 => RuleEffectKind::ProjectActorStat {
                        actor: DeclaredSlot {
                            declaration: SlotOwnerDefId::Skill(id("skill")),
                            slot: id("actor"),
                        },
                        stat: id("final-duration"),
                        value: key("value"),
                    },
                    2 => RuleEffectKind::Contribute {
                        entity: RuleEntity::Current,
                        stat: id("final-duration"),
                        contribution: ContributionKind::Add,
                        value: key("value"),
                    },
                    _ => RuleEffectKind::ProjectModifierTransform {
                        stat: id("final-duration"),
                        targets: vec![ModifierTransformTarget {
                            definition: id("modifier"),
                            when: None,
                        }],
                        order: BoundedInteger::new(0).unwrap(),
                        operation: ModifierTransformOperation::Add,
                        value: key("value"),
                    },
                };
            },
            |s| s.programs.members[0].stage = key("output"),
        );
        assert!(matches!(
            f.build(f.input.clone()),
            Err(SupportOutputStorageError::Invalid(
                "output stat has an ordinary potential writer"
            ))
        ));
    }
}

#[test]
fn exact_package_joins_and_wire_version_are_required() {
    let f = Fixture::new();
    let different = digest_owned("different-output-test", &1, 100).unwrap();
    for case in 0..8 {
        let mut input = f.input.clone();
        match case {
            0 => input.rules = different,
            1 => input.preparation = different,
            2 => input.inputs = different,
            3 => input.receiving = different,
            4 => input.stages = different,
            5 => input.definitions.release = "other".into(),
            6 => input.namespace = GameVersionNamespace::new("foreign", "v1").unwrap(),
            _ => {
                input.definitions.content_sha256 = "other".into();
            }
        }
        assert!(matches!(
            f.build(input),
            Err(SupportOutputStorageError::Binding)
        ));
    }
    let package = f.build(f.input.clone()).unwrap();
    let changed = Fixture::with(|r| r.release = key("different-rules"), |_| {});
    assert!(matches!(
        package.verify_bindings(&changed.dependencies()),
        Err(SupportOutputStorageError::Binding)
    ));
    let mut input = f.input.clone();
    input.schema_version += 1;
    assert!(matches!(
        f.build(input),
        Err(SupportOutputStorageError::Version(2))
    ));
    let mut wire = serde_json::to_value(&f.input).unwrap();
    wire.as_object_mut().unwrap().remove("receiving");
    assert!(serde_json::from_value::<SupportOutputBindingsInput>(wire).is_err());
    let mut wire = serde_json::to_value(&f.input).unwrap();
    wire["values"] = serde_json::json!([true, false]);
    assert!(serde_json::from_value::<SupportOutputBindingsInput>(wire).is_err());
}

#[test]
fn storage_limits_apply_before_growth_and_on_decode_encode_and_reuse() {
    let f = Fixture::new();
    let package = f.build(f.input.clone()).unwrap();
    let wire = encode_support_output_bindings(&package, Default::default()).unwrap();
    for limits in [
        SupportOutputStorageLimits {
            max_entries: package.resources().entries - 1,
            ..Default::default()
        },
        SupportOutputStorageLimits {
            max_work: package.resources().work - 1,
            ..Default::default()
        },
        SupportOutputStorageLimits {
            max_wire_bytes: wire.len() - 1,
            ..Default::default()
        },
    ] {
        assert!(
            OwnedSupportOutputBindings::new(f.input.clone(), &f.dependencies(), limits).is_err()
        );
        assert!(decode_support_output_bindings(&wire, &f.dependencies(), limits).is_err());
        assert!(encode_support_output_bindings(&package, limits).is_err());
        assert!(package.validate_limits(limits).is_err());
    }
    let mut input = f.input.clone();
    input
        .final_skill_types
        .resize(100, input.final_skill_types[0].clone());
    assert!(matches!(
        OwnedSupportOutputBindings::new(
            input,
            &f.dependencies(),
            SupportOutputStorageLimits {
                max_entries: 10,
                ..Default::default()
            }
        ),
        Err(SupportOutputStorageError::Limit("entries"))
    ));
    assert!(matches!(
        decode_support_output_bindings(
            b"not-json",
            &f.dependencies(),
            SupportOutputStorageLimits {
                max_wire_bytes: 1,
                ..Default::default()
            }
        ),
        Err(SupportOutputStorageError::Limit("bytes"))
    ));
    for limits in [
        SupportOutputStorageLimits {
            max_entries: 0,
            ..Default::default()
        },
        SupportOutputStorageLimits {
            max_work: 2_000_001,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            package.validate_limits(limits),
            Err(SupportOutputStorageError::InvalidLimit(_))
        ));
    }
}
