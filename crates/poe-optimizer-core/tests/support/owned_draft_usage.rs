use super::*;

fn usage(target: UsageTarget, count: i64) -> UsagePolicySelection {
    UsagePolicySelection {
        policy: def("per-use"),
        target,
        parameters: vec![ParameterAssignment {
            slot: slot(SlotOwnerDefId::UsagePolicy(def("per-use")), "count"),
            value: ParameterValue::Integer(BoundedInteger::new(count).unwrap()),
        }],
    }
}
fn authored(local: u64, count: i64) -> UsagePolicySelection {
    usage(UsageTarget::Skill(SkillTarget::Authored(id(local))), count)
}
fn preferences(input: &mut DraftSessionInput, local: u64, values: Vec<UsagePolicySelection>) {
    input
        .skill_presets
        .members
        .iter_mut()
        .find(|p| p.id == id(local))
        .unwrap()
        .usage_preferences = Some(values.into());
}
fn generated_target(local: u64) -> UsageTarget {
    UsageTarget::Skill(SkillTarget::Generated(Box::new(GeneratedSkillKey {
        provider: ProviderKey {
            root: ProviderRoot::SkillUse(id(local)),
            grant_path: vec![slot(SlotOwnerDefId::Gem(def("gem")), "grant")],
        },
        slot: slot(SlotOwnerDefId::Gem(def("gem")), "skill"),
    })))
}

#[test]
fn finalization_uses_the_shared_composer_and_preserves_queries_and_physical_inputs() {
    let mut raw = input();
    preferences(&mut raw, 130, vec![authored(60, 2), authored(61, 4)]);
    let scenario = raw
        .scenario_presets
        .members
        .iter_mut()
        .find(|p| p.id == id(150))
        .unwrap();
    scenario.scenario.usage.members.push(authored(60, 8).into());
    let session = DraftSession::new(raw, limits()).unwrap();
    let finalized = ready(&session, selection());
    let mut project = project_input();
    project.skill_presets[0].usage_preferences = Some(vec![authored(60, 2), authored(61, 4)]);
    let project = BuildProject::new(project, limits().input).unwrap();
    let scenario = session
        .input()
        .scenario_presets
        .members
        .iter()
        .find(|p| p.id == id(150))
        .unwrap()
        .scenario
        .to_resolved()
        .unwrap();
    let expected = compose_request(
        &project,
        &selection().build,
        None,
        ScenarioSpec::new(scenario, limits().input).unwrap(),
        QuerySpec::new(queries(0), limits().input).unwrap(),
        limits().input,
    )
    .unwrap();
    assert_eq!(finalized.request(), &expected);
    assert_eq!(expected.queries().input(), &queries(0));
    assert_eq!(
        expected.build().input().gems,
        compose(&project, &selection().build, None, limits().input)
            .unwrap()
            .input()
            .gems
    );
    assert!(expected.scenario().input().usage.contains(&authored(60, 8)));
    assert!(expected.scenario().input().usage.contains(&authored(61, 4)));
}

#[test]
fn pending_preferences_belong_to_their_preset_and_overrides_do_not_erase_obligations() {
    let mut raw = input();
    preferences(&mut raw, 131, vec![authored(62, 3)]);
    raw.skill_presets.members[1]
        .usage_preferences
        .as_mut()
        .unwrap()
        .completion = open(990);
    let session = DraftSession::new(raw.clone(), limits()).unwrap();
    assert!(
        session
            .validate_limits(limits())
            .unwrap()
            .issues
            .iter()
            .any(|issue| issue.owner == Some(id::<SkillPresetId>(131).instance_id()))
    );
    ready(&session, selection());
    preferences(&mut raw, 130, vec![authored(60, 2)]);
    raw.skill_presets.members[0]
        .usage_preferences
        .as_mut()
        .unwrap()
        .members[0]
        .parameters
        .members[0]
        .value = DraftField::Pending(pending(
        991,
        vec![ParameterValue::Integer(BoundedInteger::new(2).unwrap())],
    ));
    raw.scenario_presets
        .members
        .iter_mut()
        .find(|p| p.id == id(150))
        .unwrap()
        .scenario
        .usage
        .members
        .push(authored(60, 8).into());
    let session = DraftSession::new(raw, limits()).unwrap();
    let (issues, retained_queries) = pending_result(&session);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].id, id(991));
    assert_eq!(
        issues[0].owner,
        Some(id::<SkillPresetId>(130).instance_id())
    );
    assert!(issues[0].path.contains("usage_preferences"));
    assert_eq!(retained_queries.to_resolved().unwrap(), queries(0));
}

#[test]
fn preference_layer_absence_pending_and_explicit_empty_roundtrip_distinctly() {
    let original = DraftSession::new(input(), limits()).unwrap();
    let old_bytes = encode_draft(&original, limits()).unwrap();
    assert!(
        !String::from_utf8(old_bytes.clone())
            .unwrap()
            .contains("usage_preferences")
    );
    assert_eq!(
        encode_draft(&decode_draft(&old_bytes, limits()).unwrap(), limits()).unwrap(),
        old_bytes
    );
    let mut raw = input();
    preferences(&mut raw, 130, vec![]);
    let complete = DraftSession::new(raw.clone(), limits()).unwrap();
    let complete_bytes = encode_draft(&complete, limits()).unwrap();
    assert_ne!(old_bytes, complete_bytes);
    assert_eq!(
        ready(&original, selection()).request(),
        ready(&complete, selection()).request()
    );
    raw.skill_presets.members[0]
        .usage_preferences
        .as_mut()
        .unwrap()
        .completion = open(990);
    let pending_session = DraftSession::new(raw, limits()).unwrap();
    let bytes = encode_draft(&pending_session, limits()).unwrap();
    let restored = decode_draft(&bytes, limits()).unwrap();
    assert_eq!(restored, pending_session);
    assert_eq!(pending_result(&restored).0[0].id, id(990));
    let mut wire: serde_json::Value = serde_json::from_slice(&old_bytes).unwrap();
    wire["draft"]["skill_presets"]["members"][0]["usage_preferences"] = serde_json::Value::Null;
    assert!(decode_draft(&serde_json::to_vec(&wire).unwrap(), limits()).is_err());
}

#[test]
fn partial_generated_targets_and_pending_candidates_cannot_cross_preset_ownership() {
    for foreign in [false, true] {
        let mut raw = input();
        preferences(
            &mut raw,
            130,
            vec![usage(generated_target(if foreign { 62 } else { 60 }), 1)],
        );
        let row = &mut raw.skill_presets.members[0]
            .usage_preferences
            .as_mut()
            .unwrap()
            .members[0];
        let DraftUsageTarget::Skill(DraftSkillTarget::Generated(key)) = &mut row.target else {
            panic!()
        };
        key.slot = DraftField::Pending(pending(990, vec![]));
        let result = DraftSession::new(raw, limits());
        if foreign {
            assert!(matches!(
                result,
                Err(StructuralError {
                    kind: StructuralErrorKind::WrongProviderOwner,
                    ..
                })
            ));
        } else {
            assert_eq!(pending_result(&result.unwrap()).0[0].id, id(990));
        }
    }
    let mut raw = input();
    preferences(&mut raw, 130, vec![authored(60, 1)]);
    raw.skill_presets.members[0]
        .usage_preferences
        .as_mut()
        .unwrap()
        .members[0]
        .target = DraftUsageTarget::Pending(pending(
        990,
        vec![authored(60, 1).target, generated_target(62)],
    ));
    assert!(DraftSession::new(raw, limits()).is_err());
}

#[test]
fn complete_generated_actor_action_and_skill_preferences_survive_draft_conversion() {
    let root = ProviderRoot::SkillUse(id(60));
    let owned_actor = ActorKey::Owned(Box::new(OwnedActorKey {
        provider: provider(root.clone()),
        slot: slot(SlotOwnerDefId::Gem(def("gem")), "actor"),
    }));
    let mut owned_action = action(root);
    owned_action.action.actor = owned_actor.clone();
    let rows = vec![
        usage(generated_target(60), 2),
        usage(UsageTarget::Actor(owned_actor), 3),
        usage(UsageTarget::Action(Box::new(owned_action)), 4),
    ];
    let mut raw = input();
    preferences(&mut raw, 130, rows.clone());
    let draft = DraftSession::new(raw, limits()).unwrap();
    let bytes = encode_draft(&draft, limits()).unwrap();
    let restored = decode_draft(&bytes, limits()).unwrap();
    assert_eq!(restored, draft);
    let request = ready(&restored, selection());
    for row in rows {
        assert!(request.request().scenario().input().usage.contains(&row));
    }
}

#[test]
fn draft_layers_reject_duplicate_keys_slots_foreign_namespaces_and_tight_bounds() {
    let mut raw = input();
    preferences(&mut raw, 130, vec![authored(60, 1), authored(60, 2)]);
    assert!(DraftSession::new(raw, limits()).is_err());
    let mut raw = input();
    preferences(&mut raw, 130, vec![authored(60, 1)]);
    let row = &mut raw.skill_presets.members[0]
        .usage_preferences
        .as_mut()
        .unwrap()
        .members[0];
    row.parameters
        .members
        .push(row.parameters.members[0].clone());
    assert!(DraftSession::new(raw, limits()).is_err());
    let mut raw = input();
    preferences(&mut raw, 130, vec![authored(60, 1)]);
    let policy =
        DefId::parse(GameVersionNamespace::new("other", "v1").unwrap(), "per-use").unwrap();
    raw.skill_presets.members[0]
        .usage_preferences
        .as_mut()
        .unwrap()
        .members[0]
        .policy = DraftField::Known { value: policy };
    assert!(DraftSession::new(raw, limits()).is_err());
    let mut raw = input();
    preferences(&mut raw, 130, vec![authored(60, 1)]);
    let row = &mut raw.skill_presets.members[0]
        .usage_preferences
        .as_mut()
        .unwrap()
        .members[0];
    row.target = DraftUsageTarget::Pending(pending(
        990,
        vec![authored(60, 1).target, authored(61, 2).target],
    ));
    let session = DraftSession::new(raw, limits()).unwrap();
    let tight = DraftLimits {
        max_candidates_per_field: 1,
        ..limits()
    };
    assert!(session.validate_limits(tight).is_err());
    assert!(session.finalize_selection(selection(), tight).is_err());
    let bytes = encode_draft(&session, limits()).unwrap();
    let mut tight = limits();
    tight.input.max_wire_bytes = bytes.len() - 1;
    assert!(encode_draft(&session, tight).is_err());
    assert!(decode_draft(&bytes, tight).is_err());
}
