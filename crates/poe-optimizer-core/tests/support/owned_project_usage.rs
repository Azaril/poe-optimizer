use super::*;

fn usage(target: UsageTarget, count: i64) -> UsagePolicySelection {
    UsagePolicySelection {
        policy: def("usage"),
        target,
        parameters: vec![ParameterAssignment {
            slot: DeclaredSlot {
                declaration: SlotOwnerDefId::UsagePolicy(def("usage")),
                slot: def("count"),
            },
            value: ParameterValue::Integer(BoundedInteger::new(count).unwrap()),
        }],
    }
}
fn authored(id_value: u64, count: i64) -> UsagePolicySelection {
    usage(
        UsageTarget::Skill(SkillTarget::Authored(id(id_value))),
        count,
    )
}
fn scenario(usage: Vec<UsagePolicySelection>) -> ScenarioSpec {
    ScenarioSpec::new(
        ScenarioInput {
            game_version: namespace(),
            enemy: EnemySpec {
                encounter: def("encounter"),
                level: 80,
            },
            assumptions: vec![],
            usage,
        },
        limits(),
    )
    .unwrap()
}
fn queries() -> QuerySpec {
    QuerySpec::new(
        QueryInput {
            game_version: namespace(),
            requests: vec![MetricRequest {
                id: QueryId::new("retained-query").unwrap(),
                metric: def("metric"),
                target: MetricTarget::Actor(ActorKey::Player),
            }],
        },
        limits(),
    )
    .unwrap()
}
fn with_preferences(preferences: Vec<UsagePolicySelection>) -> ProjectInput {
    let mut raw = input();
    raw.skill_presets
        .iter_mut()
        .find(|p| p.id == id(130))
        .unwrap()
        .usage_preferences = Some(preferences);
    raw
}
fn request(raw: ProjectInput, overrides: Vec<UsagePolicySelection>) -> OwnedEvaluationRequest {
    compose_request(
        &BuildProject::new(raw, limits()).unwrap(),
        &selection(),
        None,
        scenario(overrides),
        queries(),
        limits(),
    )
    .unwrap()
}
fn generated(root: ProviderRoot) -> SkillTarget {
    SkillTarget::Generated(Box::new(GeneratedSkillKey {
        provider: ProviderKey {
            root,
            grant_path: vec![slot("grant")],
        },
        slot: slot("generated-skill"),
    }))
}
fn actor(root: ProviderRoot) -> ActorKey {
    ActorKey::Owned(Box::new(OwnedActorKey {
        provider: ProviderKey {
            root,
            grant_path: vec![slot("grant")],
        },
        slot: slot("actor"),
    }))
}
fn action(root: ProviderRoot, actor: ActorKey) -> UsageTarget {
    UsageTarget::Action(Box::new(ActionSelection {
        action: ActionKey {
            actor,
            provider: provider(root),
            output: slot("output"),
        },
        part: def("part"),
        mode: def("mode"),
        stat_set: def("stat-set"),
    }))
}

#[test]
fn preferences_follow_selected_uses_not_shared_physical_gem() {
    let mut raw = with_preferences(vec![authored(60, 2)]);
    let alternate = raw
        .skill_presets
        .iter_mut()
        .find(|p| p.id == id(131))
        .unwrap();
    alternate.skills = vec![id(61)];
    alternate.usage_preferences = Some(vec![authored(61, 7)]);
    let project = BuildProject::new(raw, limits()).unwrap();
    let original = compose(&project, &selection(), None, limits()).unwrap();
    let first = compose_request(
        &project,
        &selection(),
        None,
        scenario(vec![]),
        queries(),
        limits(),
    )
    .unwrap();
    let second = compose_request(
        &project,
        &VariantSelection {
            skills: id(131),
            ..selection()
        },
        None,
        scenario(vec![]),
        queries(),
        limits(),
    )
    .unwrap();
    assert_eq!(first.scenario().input().usage, vec![authored(60, 2)]);
    assert_eq!(second.scenario().input().usage, vec![authored(61, 7)]);
    assert_eq!(first.build(), &original);
    assert_eq!(
        first.build().input().gems.iter().find(|g| g.id == id(20)),
        second.build().input().gems.iter().find(|g| g.id == id(20))
    );
    assert_eq!(first.queries(), &queries());
    assert_eq!(second.queries(), &queries());
}

#[test]
fn override_replaces_whole_record_and_only_the_exact_key() {
    let mut preference = authored(60, 2);
    preference.parameters.push(ParameterAssignment {
        slot: DeclaredSlot {
            declaration: SlotOwnerDefId::UsagePolicy(def("usage")),
            slot: def("other"),
        },
        value: ParameterValue::Boolean(true),
    });
    let mut other_policy = authored(60, 9);
    other_policy.policy = def("other-policy");
    other_policy.parameters[0].slot.declaration = SlotOwnerDefId::UsagePolicy(def("other-policy"));
    let original = with_preferences(vec![preference, authored(61, 4), other_policy.clone()]);
    let result = request(original.clone(), vec![authored(60, 8)]);
    assert!(result.scenario().input().usage.contains(&authored(60, 8)));
    assert!(result.scenario().input().usage.contains(&authored(61, 4)));
    assert!(result.scenario().input().usage.contains(&other_policy));
    assert_eq!(result.scenario().input().usage.len(), 3);
    assert_eq!(
        request(original, vec![])
            .scenario()
            .input()
            .usage
            .iter()
            .find(|v| v.policy == def("usage") && v.target == authored(60, 1).target)
            .unwrap()
            .parameters
            .len(),
        2
    );
    let mut empty_override = authored(60, 8);
    empty_override.parameters.clear();
    assert!(
        request(
            with_preferences(vec![authored(60, 1)]),
            vec![empty_override]
        )
        .scenario()
        .input()
        .usage[0]
            .parameters
            .is_empty()
    );
}

#[test]
fn all_skill_supplied_symbolic_topologies_are_retained_without_resolution() {
    let skill_root = ProviderRoot::SkillUse(id(60));
    let support_root = ProviderRoot::SupportAssignment(id(70));
    let preferences = vec![
        usage(UsageTarget::Skill(generated(skill_root.clone())), 2),
        usage(UsageTarget::Skill(generated(support_root.clone())), 3),
        usage(UsageTarget::Actor(actor(skill_root.clone())), 4),
        usage(UsageTarget::Actor(actor(support_root.clone())), 5),
        usage(action(skill_root.clone(), ActorKey::Player), 6),
        usage(action(support_root, actor(skill_root)), 7),
    ];
    let result = request(with_preferences(preferences.clone()), vec![]);
    assert_eq!(result.scenario().input().usage.len(), preferences.len());
    for preference in preferences {
        assert!(result.scenario().input().usage.contains(&preference));
    }
    // Definition binding, not composition, establishes that those slots really exist.
    assert_eq!(result.build().input().skills.len(), 2);
}

#[test]
fn global_occurrence_existence_cannot_supply_another_presets_preferences() {
    let foreign = ProviderRoot::SkillUse(id(62));
    let targets = vec![
        UsageTarget::Skill(SkillTarget::Authored(id(62))),
        UsageTarget::Skill(generated(foreign.clone())),
        UsageTarget::Skill(generated(ProviderRoot::SupportAssignment(id(71)))),
        UsageTarget::Actor(actor(foreign.clone())),
        action(foreign.clone(), ActorKey::Player),
        action(ProviderRoot::SkillUse(id(60)), actor(foreign)),
        UsageTarget::Skill(generated(ProviderRoot::EquipmentUse(id(40)))),
        UsageTarget::Actor(ActorKey::Player),
    ];
    for target in targets {
        let error =
            BuildProject::new(with_preferences(vec![usage(target, 1)]), limits()).unwrap_err();
        assert!(matches!(
            error,
            ProjectError::Structure(StructuralError {
                kind: StructuralErrorKind::WrongProviderOwner,
                ..
            })
        ));
    }
}

#[test]
fn both_layers_reject_duplicate_keys_and_structurally_invalid_preferences_before_override() {
    let duplicate = authored(60, 1);
    assert!(
        BuildProject::new(
            with_preferences(vec![duplicate.clone(), duplicate.clone()]),
            limits()
        )
        .is_err()
    );
    assert!(
        ScenarioSpec::new(
            ScenarioInput {
                usage: vec![duplicate.clone(), duplicate.clone()],
                ..scenario(vec![]).into_input()
            },
            limits()
        )
        .is_err()
    );
    let mut wrong = duplicate.clone();
    wrong.parameters[0].slot.declaration = SlotOwnerDefId::Gem(def("gem"));
    assert!(BuildProject::new(with_preferences(vec![wrong]), limits()).is_err());
    let mut wrong = duplicate;
    wrong.policy =
        DefId::parse(GameVersionNamespace::new("foreign", "v1").unwrap(), "usage").unwrap();
    assert!(BuildProject::new(with_preferences(vec![wrong]), limits()).is_err());
}

#[test]
fn both_raw_parameter_inventories_are_bounded_before_whole_record_replacement() {
    let mut preference = authored(60, 1);
    for index in 0..300 {
        preference.parameters.push(ParameterAssignment {
            slot: DeclaredSlot {
                declaration: SlotOwnerDefId::UsagePolicy(def("usage")),
                slot: def(&format!("parameter-{index}")),
            },
            value: ParameterValue::Boolean(true),
        });
    }
    let project = BuildProject::new(with_preferences(vec![preference.clone()]), limits()).unwrap();
    let explicit = scenario(vec![preference]);
    let tight = (1..1_000)
        .find_map(|max_entries| {
            let candidate = OwnedInputLimits {
                max_entries,
                ..limits()
            };
            let build = compose(&project, &selection(), None, candidate).ok()?;
            OwnedEvaluationRequest::new(build, explicit.clone(), queries(), candidate).ok()?;
            Some(candidate)
        })
        .expect("both independently authored layers fit a bounded request");
    // The final result would contain just one parameter record. Both raw records
    // still count against the pre-composition allowance before that replacement.
    assert!(matches!(
        compose_request(
            &project,
            &selection(),
            None,
            explicit.clone(),
            queries(),
            tight
        ),
        Err(ProjectError::Structure(StructuralError {
            kind: StructuralErrorKind::LimitExceeded,
            ..
        }))
    ));
    assert!(compose_request(&project, &selection(), None, explicit, queries(), limits()).is_ok());
}

#[test]
fn preference_roundtrip_is_canonical_and_absent_v4_layer_is_byte_compatible() {
    let old = OwnedDocument::Project(Box::new(project()));
    let bytes = encode_owned(&old, limits()).unwrap();
    assert!(
        !String::from_utf8(bytes.clone())
            .unwrap()
            .contains("usage_preferences")
    );
    assert_eq!(
        encode_owned(&decode_owned(&bytes, limits()).unwrap(), limits()).unwrap(),
        bytes
    );
    let absent_request = compose_request(
        &project(),
        &selection(),
        None,
        scenario(vec![]),
        queries(),
        limits(),
    )
    .unwrap();
    let historical_request = OwnedEvaluationRequest::new(
        compose(&project(), &selection(), None, limits()).unwrap(),
        scenario(vec![]),
        queries(),
        limits(),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_vec(&absent_request).unwrap(),
        serde_json::to_vec(&historical_request).unwrap()
    );
    let mut raw = with_preferences(vec![authored(61, 2), authored(60, 1)]);
    let canonical = BuildProject::new(raw.clone(), limits()).unwrap();
    raw.skill_presets
        .iter_mut()
        .find(|p| p.id == id(130))
        .unwrap()
        .usage_preferences
        .as_mut()
        .unwrap()
        .reverse();
    assert_eq!(canonical, BuildProject::new(raw, limits()).unwrap());
    let document = OwnedDocument::Project(Box::new(canonical));
    let bytes = encode_owned(&document, limits()).unwrap();
    assert_eq!(decode_owned(&bytes, limits()).unwrap(), document);
    let mut wire: Value = serde_json::from_slice(&bytes).unwrap();
    wire["document"]["value"]["skill_presets"][0]["usage_preferences"] = Value::Null;
    assert!(decode_owned(&serde_json::to_vec(&wire).unwrap(), limits()).is_err());
}

#[test]
fn historical_scenario_usage_is_preserved_with_or_without_preset_preferences() {
    for has_preferences in [false, true] {
        let raw = if has_preferences {
            with_preferences(vec![authored(60, 2)])
        } else {
            input()
        };
        let project = BuildProject::new(raw, limits()).unwrap();
        let build = compose(&project, &selection(), None, limits()).unwrap();
        // 62 exists only in an unselected preset; 199 is an absent historical
        // occurrence within this project's lineage and allocator watermark.
        for historical in [62, 199] {
            let usage = authored(historical, 9);
            let explicit = scenario(vec![usage.clone()]);
            let legacy =
                OwnedEvaluationRequest::new(build.clone(), explicit.clone(), queries(), limits())
                    .unwrap();
            let composed =
                compose_request(&project, &selection(), None, explicit, queries(), limits())
                    .unwrap();
            assert!(composed.scenario().input().usage.contains(&usage));
            assert_eq!(composed.build(), legacy.build());
            assert_eq!(composed.queries(), legacy.queries());
            if has_preferences {
                assert!(composed.scenario().input().usage.contains(&authored(60, 2)));
                assert_eq!(composed.scenario().input().usage.len(), 2);
            } else {
                assert_eq!(composed, legacy);
            }
            // This checks structural retention only. Normal definition binding
            // rejects a missing authored usage provider; it does not inherit
            // the Unavailable classification used for historical queries.
        }
        let foreign = SkillUseId::from_instance_id(
            InstanceId::from_parts(BuildLineage::from_bytes([0x23; 16]), 60).unwrap(),
        );
        for (target, kind) in [
            (
                id(20),
                StructuralErrorKind::MissingReference {
                    expected: OccurrenceKind::SkillUse,
                    id: id::<GemInstanceId>(20).instance_id(),
                },
            ),
            (foreign, StructuralErrorKind::ForeignLineage),
            (id(201), StructuralErrorKind::BeyondWatermark),
        ] {
            let error = compose_request(
                &project,
                &selection(),
                None,
                scenario(vec![usage(
                    UsageTarget::Skill(SkillTarget::Authored(target)),
                    9,
                )]),
                queries(),
                limits(),
            )
            .unwrap_err();
            assert!(
                matches!(error, ProjectError::Structure(StructuralError { kind: actual, .. }) if actual == kind)
            );
        }
    }
}

#[test]
fn composition_revalidates_tighter_limits_before_override_compaction() {
    let mut raw = with_preferences(vec![authored(60, 1)]);
    let preference = &mut raw
        .skill_presets
        .iter_mut()
        .find(|p| p.id == id(130))
        .unwrap()
        .usage_preferences
        .as_mut()
        .unwrap()[0];
    for index in 0..90 {
        preference.parameters.push(ParameterAssignment {
            slot: DeclaredSlot {
                declaration: SlotOwnerDefId::UsagePolicy(def("usage")),
                slot: def(&format!("parameter-{index}")),
            },
            value: ParameterValue::Boolean(true),
        });
    }
    let project = BuildProject::new(raw, limits()).unwrap();
    let tight = OwnedInputLimits {
        max_collection_entries: 80,
        ..limits()
    };
    assert!(
        compose_request(
            &project,
            &selection(),
            None,
            scenario(vec![authored(60, 2)]),
            queries(),
            tight
        )
        .is_err()
    );
    assert!(
        compose_request(
            &project,
            &selection(),
            None,
            scenario(vec![authored(60, 2)]),
            queries(),
            limits()
        )
        .is_ok()
    );
    let mut too_deep = generated(ProviderRoot::SkillUse(id(60)));
    if let SkillTarget::Generated(key) = &mut too_deep {
        key.provider.grant_path = vec![slot("step"); limits().max_provider_steps + 1];
    }
    assert!(
        BuildProject::new(
            with_preferences(vec![usage(UsageTarget::Skill(too_deep), 1)]),
            limits()
        )
        .is_err()
    );
}
