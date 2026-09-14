//! Direct authored projection/persistence laws. These tests claim structural
//! reconstruction only, with no source adapter, schema, legality or evaluation.
use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_content::digest_owned, owned_definitions::*,
    owned_draft::*, owned_project::*,
};
use serde_json::json;

fn limits() -> DraftLimits {
    DraftLimits::default()
}
fn lineage() -> BuildLineage {
    BuildLineage::from_bytes([0x53; 16])
}
fn namespace() -> GameVersionNamespace {
    GameVersionNamespace::new("authored-draft-game", "v1").unwrap()
}
fn id<T: BuildInstanceId>(local: u64) -> T {
    T::from_instance_id(InstanceId::from_parts(lineage(), local).unwrap())
}
fn def<K: DefinitionDomain>(key: &str) -> DefId<K> {
    DefId::parse(namespace(), key).unwrap()
}
fn slot<K: DefinitionDomain>(owner: SlotOwnerDefId, key: &str) -> DeclaredSlot<DefId<K>> {
    DeclaredSlot {
        declaration: owner,
        slot: def(key),
    }
}
fn provider(root: ProviderRoot) -> ProviderKey {
    ProviderKey {
        root,
        grant_path: vec![],
    }
}
fn pending<T>(local: u64, candidates: Vec<T>) -> PendingValue<T> {
    PendingValue {
        id: id(local),
        code: OwnedDefinitionKey::new("needs-mapping").unwrap(),
        candidates,
    }
}
fn open(local: u64) -> DraftListCompletion {
    DraftListCompletion::Pending {
        id: id(local),
        code: OwnedDefinitionKey::new("unknown-members").unwrap(),
    }
}
fn item(local: u64) -> ItemRecord {
    ItemRecord {
        id: id(local),
        template: def("item"),
        item_level: 50,
        quality: None,
        parameters: vec![ParameterAssignment {
            slot: slot(SlotOwnerDefId::ItemTemplate(def("item")), "rarity"),
            value: ParameterValue::Integer(BoundedInteger::new(2).unwrap()),
        }],
        modifiers: vec![RolledModifier {
            id: id(local + 1),
            definition: def("modifier"),
            rolls: vec![ParameterAssignment {
                slot: slot(SlotOwnerDefId::Modifier(def("modifier")), "roll"),
                value: ParameterValue::Quantity(
                    FiniteQuantity::new(7.0, def("flat-unit")).unwrap(),
                ),
            }],
        }],
    }
}
fn gem(local: u64) -> GemInstance {
    GemInstance {
        id: id(local),
        definition: def("gem"),
        parameters: vec![],
        level: 12,
        quality: None,
    }
}
fn selection() -> EvaluationSelection {
    EvaluationSelection {
        build: VariantSelection {
            character: id(100),
            equipment: id(110),
            allocations: id(120),
            skills: id(130),
            choices: id(140),
            active_weapon_loadout: id(1),
        },
        scenario: id(150),
        queries: id(160),
    }
}
fn generated() -> SkillTarget {
    SkillTarget::Generated(Box::new(GeneratedSkillKey {
        provider: provider(ProviderRoot::EquipmentUse(id(40))),
        slot: slot(SlotOwnerDefId::ItemTemplate(def("item")), "granted-skill"),
    }))
}
fn action(root: ProviderRoot) -> ActionSelection {
    ActionSelection {
        action: ActionKey {
            actor: ActorKey::Player,
            provider: provider(root),
            output: slot(SlotOwnerDefId::Gem(def("gem")), "output"),
        },
        part: def("part"),
        mode: def("mode"),
        stat_set: def("stat-set"),
    }
}
fn project_input() -> ProjectInput {
    ProjectInput {
        allocator: InstanceAllocatorState::from_parts(lineage(), 1000),
        revision: BuildRevision::from_u64(5),
        game_version: namespace(),
        weapon_loadouts: vec![id(2), id(1)],
        items: vec![item(10), item(12)],
        gems: vec![gem(20), gem(21), gem(22)],
        rewards: vec![RewardSelection {
            id: id(30),
            definition: def("reward"),
            parameters: vec![],
        }],
        equipment: vec![
            EquipmentUse {
                id: id(40),
                item: id(10),
                destination: EquipmentDestination::CharacterSlot(def("ring-a")),
                scope: LoadoutScope::Shared,
            },
            EquipmentUse {
                id: id(41),
                item: id(10),
                destination: EquipmentDestination::CharacterSlot(def("ring-b")),
                scope: LoadoutScope::Shared,
            },
        ],
        allocations: vec![Allocation {
            id: id(50),
            node: def("node"),
            pool: def("pool"),
            scope: LoadoutScope::Shared,
            access: AllocationAccess::Ordinary,
            choices: vec![],
        }],
        skills: vec![
            SkillUse {
                id: id(60),
                source: AuthoredSkillSource::Gem(id(20)),
                enabled: true,
                scope: LoadoutScope::Shared,
            },
            SkillUse {
                id: id(61),
                source: AuthoredSkillSource::Direct(def("manual-skill")),
                enabled: true,
                scope: LoadoutScope::Shared,
            },
            SkillUse {
                id: id(62),
                source: AuthoredSkillSource::Gem(id(22)),
                enabled: false,
                scope: LoadoutScope::Selected {
                    loadouts: vec![id(2)],
                },
            },
        ],
        supports: vec![SupportAssignment {
            id: id(70),
            support: id(21),
            target: generated(),
            enabled: true,
        }],
        payload_links: vec![PayloadLink {
            id: id(80),
            container: id(60),
            payload: id(61),
            role: def("payload-role"),
        }],
        character_presets: vec![CharacterPreset {
            id: id(100),
            class: def("class"),
            ascendancy: None,
            level: 80,
            rewards: vec![id(30)],
        }],
        equipment_presets: vec![EquipmentPreset {
            id: id(110),
            equipment: vec![id(41), id(40)],
        }],
        allocation_presets: vec![AllocationPreset {
            id: id(120),
            allocations: vec![id(50)],
            equipment: vec![],
        }],
        skill_presets: vec![
            SkillPreset {
                id: id(130),
                skills: vec![id(61), id(60)],
                supports: vec![id(70)],
                payload_links: vec![id(80)],
            },
            SkillPreset {
                id: id(131),
                skills: vec![id(62)],
                supports: vec![],
                payload_links: vec![],
            },
        ],
        choice_presets: vec![ChoicePreset {
            id: id(140),
            rewards: vec![],
            choices: vec![MechanicChoice {
                owner: ChoiceOwner::Character,
                choice: ChoiceSelection {
                    slot: slot(SlotOwnerDefId::Class(def("class")), "choice"),
                    value: ParameterValue::Boolean(false),
                },
            }],
        }],
        saved_variants: vec![],
    }
}
fn scenario(which: u8) -> ScenarioInput {
    ScenarioInput {
        game_version: namespace(),
        enemy: EnemySpec {
            encounter: def(if which == 0 { "boss" } else { "mapping" }),
            level: if which == 0 { 82 } else { 75 },
        },
        assumptions: vec![ExternalAssumption {
            input: def("external-condition"),
            target: AssumptionTarget::Enemy,
            value: ParameterValue::Boolean(which == 0),
        }],
        usage: vec![UsagePolicySelection {
            policy: def("usage"),
            target: UsageTarget::Actor(ActorKey::Player),
            parameters: vec![],
        }],
    }
}
fn queries(which: u8) -> QueryInput {
    let mut requests = vec![
        MetricRequest {
            id: QueryId::new("row-z").unwrap(),
            metric: def("average-hit"),
            target: MetricTarget::Action(Box::new(action(ProviderRoot::SkillUse(id(60))))),
        },
        MetricRequest {
            id: QueryId::new("row-a").unwrap(),
            metric: def("life"),
            target: MetricTarget::Actor(ActorKey::Player),
        },
    ];
    if which != 0 {
        requests.reverse();
    }
    QueryInput {
        game_version: namespace(),
        requests,
    }
}
fn input() -> DraftSessionInput {
    let p = project_input();
    DraftSessionInput {
        allocator: p.allocator,
        revision: p.revision,
        game_version: p.game_version,
        weapon_loadouts: p.weapon_loadouts.into(),
        items: p.items.into(),
        gems: p.gems.into(),
        rewards: p.rewards.into(),
        equipment: p.equipment.into(),
        allocations: p.allocations.into(),
        skills: p.skills.into(),
        supports: p.supports.into(),
        payload_links: p.payload_links.into(),
        character_presets: p.character_presets.into(),
        equipment_presets: p.equipment_presets.into(),
        allocation_presets: p.allocation_presets.into(),
        skill_presets: p.skill_presets.into(),
        choice_presets: p.choice_presets.into(),
        scenario_presets: DraftList {
            members: vec![
                ScenarioPresetDraft {
                    id: id(151),
                    scenario: scenario(1).into(),
                },
                ScenarioPresetDraft {
                    id: id(150),
                    scenario: scenario(0).into(),
                },
            ],
            completion: DraftListCompletion::Complete,
        },
        query_presets: DraftList {
            members: vec![
                QueryPresetDraft {
                    id: id(160),
                    queries: queries(0).into(),
                },
                QueryPresetDraft {
                    id: id(161),
                    queries: queries(1).into(),
                },
            ],
            completion: DraftListCompletion::Complete,
        },
        saved_variants: DraftList {
            members: vec![SavedVariantDraft {
                id: id(170),
                selection: selection().into(),
            }],
            completion: DraftListCompletion::Complete,
        },
    }
}
fn ready(session: &DraftSession, selected: EvaluationSelection) -> FinalizedDraft {
    match session.finalize_selection(selected, limits()).unwrap() {
        DraftFinalization::Ready(value) => *value,
        other => panic!("expected complete structural request: {other:?}"),
    }
}
fn pending_result(session: &DraftSession) -> (Vec<DraftIssue>, QueryDraft) {
    match session.finalize_selection(selection(), limits()).unwrap() {
        DraftFinalization::Pending {
            draft_digest,
            selection: selected,
            issues,
            queries,
        } => {
            assert_eq!(
                draft_digest,
                session.digest(limits().input.max_wire_bytes).unwrap()
            );
            assert_eq!(*selected, selection());
            (issues, queries)
        }
        other => panic!("expected selected pending input: {other:?}"),
    }
}

#[test]
fn complete_selection_matches_explicit_composition_and_shared_record_identity() {
    let session = DraftSession::new(input(), limits()).unwrap();
    let finalized = ready(&session, selection());
    let p = project_input();
    let project = BuildProject::new(p.clone(), limits().input).unwrap();
    let composed = compose(&project, &selection().build, None, limits().input).unwrap();
    // An independently specified concrete build also checks the exact projection.
    let direct = BuildSpec::new(
        BuildInput {
            allocator: p.allocator,
            revision: p.revision,
            game_version: p.game_version,
            character: CharacterSpec {
                class: def("class"),
                ascendancy: None,
                level: 80,
                rewards: p.rewards,
            },
            weapon_loadouts: p.weapon_loadouts,
            active_weapon_loadout: id(1),
            items: vec![item(10)],
            gems: vec![gem(20), gem(21)],
            equipment: p.equipment,
            allocations: p.allocations,
            skills: vec![p.skills[1].clone(), p.skills[0].clone()],
            supports: p.supports,
            payload_links: p.payload_links,
            choices: p.choice_presets[0].choices.clone(),
        },
        limits().input,
    )
    .unwrap();
    assert_eq!(composed, direct);
    let expected = OwnedEvaluationRequest::new(
        composed,
        ScenarioSpec::new(scenario(0), limits().input).unwrap(),
        QuerySpec::new(queries(0), limits().input).unwrap(),
        limits().input,
    )
    .unwrap();
    assert_eq!(finalized.request(), &expected);
    assert_eq!(finalized.selection(), selection());
    assert_eq!(
        finalized.draft_digest(),
        session.digest(limits().input.max_wire_bytes).unwrap()
    );
    assert_eq!(
        finalized.request_digest(),
        digest_owned("owned-request-v1", &expected, limits().input.max_wire_bytes).unwrap()
    );
    let build = finalized.request().build().input();
    assert_eq!(build.items.len(), 1);
    assert_eq!(build.items[0].id, id(10));
    assert_eq!(build.equipment.len(), 2);
    assert_ne!(build.equipment[0].id, build.equipment[1].id);
    assert!(build.equipment.iter().all(|row| row.item == id(10)));
    assert_eq!(build.allocator, session.input().allocator);
    assert_eq!(session.input().items.members.len(), 2);
}

#[test]
fn scenario_and_ordered_query_presets_select_independently_of_build_and_each_other() {
    let session = DraftSession::new(input(), limits()).unwrap();
    let base = ready(&session, selection());
    for scenario_id in [150, 151] {
        for query_id in [160, 161] {
            let mut selected = selection();
            selected.scenario = id(scenario_id);
            selected.queries = id(query_id);
            let actual = ready(&session, selected);
            assert_eq!(actual.request().build(), base.request().build());
            assert_eq!(
                actual.request().scenario(),
                &ScenarioSpec::new(scenario(u8::from(scenario_id == 151)), limits().input).unwrap()
            );
            assert_eq!(
                actual.request().queries().input(),
                &queries(u8::from(query_id == 161))
            );
        }
    }
}

#[test]
fn inactive_partial_rows_and_global_completion_remain_saved_without_blocking_selection() {
    let mut raw = input();
    raw.items.members[1].template = DraftField::Pending(pending(200, vec![]));
    raw.skill_presets.members[1].skills.completion = open(201);
    raw.items.completion = open(202);
    raw.skill_presets.completion = open(203);
    raw.query_presets.completion = open(204);
    let session = DraftSession::new(raw.clone(), limits()).unwrap();
    let actual = ready(&session, selection());
    assert_eq!(
        actual.request(),
        ready(&DraftSession::new(input(), limits()).unwrap(), selection()).request()
    );
    assert_eq!(session.input(), &raw);
    assert_eq!(session.validate_limits(limits()).unwrap().issues.len(), 5);
}

#[test]
fn selected_membership_completion_blocks_and_returns_every_ordered_query_row() {
    let mut raw = input();
    raw.skill_presets.members[0].skills.completion = open(205);
    let expected_queries = raw.query_presets.members[0].queries.clone();
    let session = DraftSession::new(raw, limits()).unwrap();
    let (issues, retained_queries) = pending_result(&session);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].id, id(205));
    assert_eq!(
        issues[0].owner,
        Some(id::<SkillPresetId>(130).instance_id())
    );
    assert_eq!(retained_queries, expected_queries);
    assert_eq!(
        retained_queries
            .requests
            .members
            .iter()
            .map(|r| r.id.as_str())
            .collect::<Vec<_>>(),
        vec!["row-z", "row-a"]
    );
}

#[test]
fn selected_nested_modifier_and_scenario_uncertainty_keep_containing_owner() {
    let mut raw = input();
    raw.items.members[0].modifiers.members[0].rolls.members[0].value =
        DraftField::Pending(pending(206, vec![]));
    raw.scenario_presets.members[1].scenario.usage.members[0].target =
        DraftUsageTarget::Action(Box::new(ActionSelectionDraft {
            mode: DraftField::Pending(pending(207, vec![])),
            ..action(ProviderRoot::SkillUse(id(60))).into()
        }));
    let session = DraftSession::new(raw, limits()).unwrap();
    let (issues, _) = pending_result(&session);
    assert_eq!(issues.len(), 2);
    assert!(
        issues.iter().any(|issue| issue.id == id(206)
            && issue.owner == Some(id::<ItemRecordId>(10).instance_id()))
    );
    assert!(issues.iter().any(|issue| issue.id == id(207)
        && issue.owner == Some(id::<ScenarioPresetId>(150).instance_id())));
}

#[test]
fn disabled_and_off_loadout_selected_records_still_require_complete_authored_values() {
    for (enabled, scope) in [
        (false, LoadoutScope::Shared),
        (
            true,
            LoadoutScope::Selected {
                loadouts: vec![id(2)],
            },
        ),
    ] {
        let mut raw = input();
        raw.skills.members[0].enabled = enabled.into();
        raw.skills.members[0].scope = scope.into();
        raw.skills.members[0].source = DraftAuthoredSkillSource::Pending(pending(208, vec![]));
        let session = DraftSession::new(raw, limits()).unwrap();
        let (issues, _) = pending_result(&session);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].id, id(208));
        assert_eq!(issues[0].owner, Some(id::<SkillUseId>(60).instance_id()));
    }
}

#[test]
fn missing_historical_query_provider_is_retained_without_retargeting() {
    let mut raw = input();
    let historical = MetricTarget::Action(Box::new(action(ProviderRoot::SkillUse(id(900)))));
    raw.query_presets.members[0].queries.requests.members[0].target = historical.clone().into();
    let session = DraftSession::new(raw, limits()).unwrap();
    let finalized = ready(&session, selection());
    let requests = &finalized.request().queries().input().requests;
    assert_eq!(requests[0].target, historical);
    assert_eq!(requests[0].id.as_str(), "row-z");
    assert_eq!(requests[1], queries(0).requests[1]);
    assert!(
        finalized
            .request()
            .build()
            .input()
            .skills
            .iter()
            .all(|row| row.id != id(900))
    );
}

#[test]
fn selected_build_cannot_pull_omitted_supplying_provider_from_another_preset() {
    let mut raw = input();
    raw.equipment_presets.members[0].equipment = vec![id::<ItemSlotUseId>(41)].into();
    raw.equipment_presets.members.push(
        EquipmentPreset {
            id: id(111),
            equipment: vec![id(40)],
        }
        .into(),
    );
    let session = DraftSession::new(raw, limits()).unwrap();
    let error = session
        .finalize_selection(selection(), limits())
        .unwrap_err();
    let structural = match error {
        FinalizationError::Structure(error)
        | FinalizationError::Project(ProjectError::Structure(error)) => error,
        other => panic!("expected missing selected provider: {other:?}"),
    };
    assert_eq!(
        structural.kind,
        StructuralErrorKind::MissingReference {
            expected: OccurrenceKind::EquipmentUse,
            id: id::<ItemSlotUseId>(40).instance_id(),
        }
    );
    assert_eq!(session.input().equipment.members.len(), 2);
}

#[test]
fn absent_pending_query_target_keeps_full_query_draft_in_order() {
    let mut raw = input();
    raw.query_presets.members[0].queries.requests.members[0].target =
        DraftMetricTarget::Pending(pending(209, vec![]));
    let expected = raw.query_presets.members[0].queries.clone();
    let session = DraftSession::new(raw, limits()).unwrap();
    let (issues, actual) = pending_result(&session);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].id, id(209));
    assert_eq!(
        issues[0].owner,
        Some(id::<QueryPresetId>(160).instance_id())
    );
    assert_eq!(actual, expected);
    assert_eq!(actual.requests.members.len(), 2);
    assert!(actual.requests.members[0].target.to_resolved().is_none());
    assert_eq!(
        actual.requests.members[1].to_resolved(),
        Some(queries(0).requests[1].clone())
    );
}

#[test]
fn exact_digests_distinguish_inactive_draft_changes_and_revision_without_retargeting() {
    let mut pending_input = input();
    pending_input.items.members[1].template = DraftField::Pending(pending(210, vec![def("item")]));
    let before = DraftSession::new(pending_input.clone(), limits()).unwrap();
    let before_request = ready(&before, selection());
    // Raw snapshots are test data, not a repair/identity transition API.
    let mut resolved_input = pending_input;
    resolved_input.items.members[1].template = def::<ItemTemplateDefinition>("item").into();
    let same_revision = DraftSession::new(resolved_input.clone(), limits()).unwrap();
    let same_request = ready(&same_revision, selection());
    assert_ne!(before_request.draft_digest(), same_request.draft_digest());
    assert_eq!(
        before_request.request_digest(),
        same_request.request_digest()
    );
    assert_eq!(before_request.request(), same_request.request());
    resolved_input.revision = resolved_input.revision.checked_next().unwrap();
    let revised = DraftSession::new(resolved_input, limits()).unwrap();
    let revised_request = ready(&revised, selection());
    assert_ne!(same_request.draft_digest(), revised_request.draft_digest());
    assert_ne!(
        same_request.request_digest(),
        revised_request.request_digest()
    );
    assert_eq!(
        before_request.request().queries(),
        revised_request.request().queries()
    );
    assert_eq!(
        before_request.request().build().input().equipment,
        revised_request.request().build().input().equipment
    );
    assert_eq!(
        before_request.request().build().input().allocator,
        revised_request.request().build().input().allocator
    );
    assert_eq!(revised_request.request().build().input().revision.get(), 6);
}

#[test]
fn draft_codec_preserves_order_pending_candidates_issue_ids_and_snapshot_digest() {
    let mut raw = input();
    raw.items.members[1].template =
        DraftField::Pending(pending(211, vec![def("z-template"), def("a-template")]));
    raw.items.completion = open(212);
    let session = DraftSession::new(raw.clone(), limits()).unwrap();
    let bytes = encode_draft(&session, limits()).unwrap();
    let decoded = decode_draft(&bytes, limits()).unwrap();
    assert_eq!(decoded.input(), &raw);
    assert_eq!(decoded, session);
    assert_eq!(encode_draft(&decoded, limits()).unwrap(), bytes);
    assert_eq!(
        decoded.digest(limits().input.max_wire_bytes).unwrap(),
        session.digest(limits().input.max_wire_bytes).unwrap()
    );
    assert_eq!(
        ready(&decoded, selection()).request(),
        ready(&session, selection()).request()
    );
}

#[test]
fn draft_codec_rejects_unknown_duplicate_missing_and_unsupported_wire_values() {
    let session = DraftSession::new(input(), limits()).unwrap();
    let bytes = encode_draft(&session, limits()).unwrap();
    let original: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    for mutation in [0, 1, 2] {
        let mut wire = original.clone();
        match mutation {
            0 => wire["source_xml"] = json!("not-core-data"),
            1 => wire["draft"]["items"]["members"][0]["extra"] = json!(true),
            _ => {
                wire["draft"]["items"]["members"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("quality");
            }
        }
        assert!(matches!(
            decode_draft(&serde_json::to_vec(&wire).unwrap(), limits()),
            Err(DraftCodecError::Json(_))
        ));
    }
    let duplicate = format!(
        "{{\"schema_version\":2,\"schema_version\":2,\"draft\":{}}}",
        serde_json::to_string(session.input()).unwrap()
    );
    assert!(matches!(
        decode_draft(duplicate.as_bytes(), limits()),
        Err(DraftCodecError::Json(_))
    ));
    let mut future = original;
    future["schema_version"] = json!(OWNED_DRAFT_SCHEMA_VERSION + 1);
    assert!(matches!(
        decode_draft(&serde_json::to_vec(&future).unwrap(), limits()),
        Err(DraftCodecError::UnsupportedVersion(3))
    ));
    // A well-formed wire ID still needs the constructor's ownership checks.
    let mut foreign = input();
    foreign.query_presets.members[0].queries.game_version =
        GameVersionNamespace::new("other-game", "v1").unwrap();
    let wire = json!({"schema_version": OWNED_DRAFT_SCHEMA_VERSION, "draft": foreign});
    assert!(matches!(
        decode_draft(&serde_json::to_vec(&wire).unwrap(), limits()),
        Err(DraftCodecError::Structure(StructuralError {
            kind: StructuralErrorKind::ForeignNamespace,
            ..
        }))
    ));
}

#[test]
fn draft_codec_enforces_exact_byte_boundary_in_both_directions() {
    let session = DraftSession::new(input(), limits()).unwrap();
    let bytes = encode_draft(&session, limits()).unwrap();
    let mut exact = limits();
    exact.input.max_wire_bytes = bytes.len();
    assert_eq!(encode_draft(&session, exact).unwrap(), bytes);
    assert_eq!(decode_draft(&bytes, exact).unwrap(), session);
    let mut tight = exact;
    tight.input.max_wire_bytes -= 1;
    assert!(
        matches!(encode_draft(&session, tight), Err(DraftCodecError::TooLarge { maximum }) if maximum == bytes.len() - 1)
    );
    assert!(
        matches!(decode_draft(&bytes, tight), Err(DraftCodecError::TooLarge { maximum }) if maximum == bytes.len() - 1)
    );
}

#[test]
fn moving_or_sharing_a_use_between_contributors_preserves_concrete_request_identity() {
    let original = DraftSession::new(input(), limits()).unwrap();
    let expected = ready(&original, selection());
    for ordinary in [vec![id::<ItemSlotUseId>(40)], vec![id(40), id(41)]] {
        let mut raw = input();
        raw.equipment_presets.members[0].equipment = ordinary.into();
        raw.allocation_presets.members[0].equipment = vec![id::<ItemSlotUseId>(41)].into();
        let session = DraftSession::new(raw, limits()).unwrap();
        let actual = ready(&session, selection());
        assert_eq!(actual.request(), expected.request());
        assert_eq!(actual.request_digest(), expected.request_digest());
        assert_ne!(actual.draft_digest(), expected.draft_digest());
        assert_eq!(actual.request().build().input().items.len(), 1);
        assert_eq!(actual.request().build().input().equipment.len(), 2);
        assert_eq!(
            session.digest(limits().input.max_wire_bytes).unwrap(),
            digest_owned("owned-draft-v2", &session, limits().input.max_wire_bytes).unwrap()
        );
        assert_ne!(
            session.digest(limits().input.max_wire_bytes).unwrap(),
            digest_owned("owned-draft-v1", &session, limits().input.max_wire_bytes).unwrap()
        );
    }
}

#[test]
fn selected_pending_contribution_blocks_with_no_members_and_with_a_duplicate_contributor() {
    for members in [vec![], vec![id::<ItemSlotUseId>(40)]] {
        let mut raw = input();
        raw.allocation_presets.members[0].equipment = DraftList {
            members,
            completion: open(220),
        };
        let session = DraftSession::new(raw, limits()).unwrap();
        let (issues, queries) = pending_result(&session);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].id, id(220));
        assert_eq!(
            issues[0].owner,
            Some(id::<AllocationPresetId>(120).instance_id())
        );
        assert!(issues[0].path.ends_with(".equipment.completion"));
        assert_eq!(
            queries
                .requests
                .members
                .iter()
                .map(|row| row.id.as_str())
                .collect::<Vec<_>>(),
            vec!["row-z", "row-a"]
        );
    }
}

fn with_contribution_backing() -> DraftSessionInput {
    let mut raw = input();
    raw.equipment.members.push(
        EquipmentUse {
            id: id(42),
            item: id(12),
            destination: EquipmentDestination::PassiveSocket {
                allocation: id(50),
                slot: def("socket"),
            },
            scope: LoadoutScope::Shared,
        }
        .into(),
    );
    raw.allocation_presets.members[0].equipment = vec![id::<ItemSlotUseId>(42)].into();
    raw
}

#[test]
fn pending_selected_receiving_and_backing_rows_block_but_candidates_are_not_followed() {
    for known_backing in [false, true] {
        let mut raw = with_contribution_backing();
        raw.items.members[1].template = DraftField::Pending(pending(221, vec![def("item")]));
        if !known_backing {
            raw.equipment.members[2].item = DraftField::Pending(pending(222, vec![id(12)]));
        }
        let session = DraftSession::new(raw, limits()).unwrap();
        let (issues, _) = pending_result(&session);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].id, id(if known_backing { 221 } else { 222 }));
        assert_eq!(
            issues[0].owner,
            Some(if known_backing {
                id::<ItemRecordId>(12).instance_id()
            } else {
                id::<ItemSlotUseId>(42).instance_id()
            })
        );
    }
    let session = DraftSession::new(with_contribution_backing(), limits()).unwrap();
    let result = ready(&session, selection());
    assert_eq!(
        result
            .request()
            .build()
            .input()
            .items
            .iter()
            .map(|row| row.id)
            .collect::<Vec<_>>(),
        vec![id(10), id(12)]
    );
    assert_eq!(
        result
            .request()
            .build()
            .input()
            .equipment
            .iter()
            .map(|row| row.id)
            .collect::<Vec<_>>(),
        vec![id(40), id(41), id(42)]
    );
}

#[test]
fn inactive_pending_allocation_contribution_stays_saved_without_affecting_selected_projection() {
    let mut raw = with_contribution_backing();
    raw.allocation_presets.members[0].equipment = Vec::<ItemSlotUseId>::new().into();
    raw.allocation_presets.members.push(AllocationPresetDraft {
        id: id(121),
        allocations: vec![id::<AllocationId>(50)].into(),
        equipment: DraftList {
            members: vec![id(42)],
            completion: open(223),
        },
    });
    raw.items.members[1].template = DraftField::Pending(pending(224, vec![def("item")]));
    let session = DraftSession::new(raw.clone(), limits()).unwrap();
    assert_eq!(
        ready(&session, selection()).request(),
        ready(&DraftSession::new(input(), limits()).unwrap(), selection()).request()
    );
    let selected = EvaluationSelection {
        build: VariantSelection {
            allocations: id(121),
            ..selection().build
        },
        ..selection()
    };
    let DraftFinalization::Pending { issues, .. } =
        session.finalize_selection(selected, limits()).unwrap()
    else {
        panic!("selected pending contribution was accepted")
    };
    assert_eq!(
        issues
            .iter()
            .map(|issue| issue.id)
            .collect::<std::collections::BTreeSet<_>>(),
        [id::<DraftIssueId>(223), id(224)].into_iter().collect()
    );
    assert_eq!(session.input(), &raw);
}

#[test]
fn draft_contribution_members_are_domain_checked_and_charged_before_deduplication() {
    for members in [
        vec![id::<ItemSlotUseId>(40), id(40)],
        vec![id(50)],
        vec![id(999)],
    ] {
        let mut raw = input();
        raw.allocation_presets.members[0].equipment = members.into();
        assert!(DraftSession::new(raw, limits()).is_err());
    }
    let mut raw = input();
    raw.allocation_presets.members[0].equipment = vec![id::<ItemSlotUseId>(41), id(40)].into();
    let session = DraftSession::new(raw.clone(), limits()).unwrap();
    // Find the exact public validation boundary with at most 17 bounded checks.
    let (mut lower, mut upper) = (1, limits().input.max_entries);
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        let mut bounded = limits();
        bounded.input.max_entries = middle;
        if session.validate_limits(bounded).is_ok() {
            upper = middle
        } else {
            lower = middle + 1
        }
    }
    let minimum = lower;
    let mut exact = limits();
    exact.input.max_entries = minimum;
    assert!(session.finalize_selection(selection(), exact).is_ok());
    let mut tight = exact;
    tight.input.max_entries -= 1;
    assert!(session.finalize_selection(selection(), tight).is_err());
    raw.allocation_presets.members[0].equipment.members.clear();
    let without = DraftSession::new(raw, limits()).unwrap();
    let mut two_less = exact;
    two_less.input.max_entries -= 2;
    assert!(without.validate_limits(two_less).is_ok());
}

#[test]
fn draft_v2_contribution_codec_preserves_order_and_rejects_v1_before_old_payload_shape() {
    let mut raw = input();
    raw.allocation_presets.members[0].equipment = DraftList {
        members: vec![id(41), id(40)],
        completion: open(225),
    };
    let session = DraftSession::new(raw, limits()).unwrap();
    let bytes = encode_draft(&session, limits()).unwrap();
    let mut wire: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(wire["schema_version"], 2);
    assert_eq!(decode_draft(&bytes, limits()).unwrap(), session);
    wire["draft"]["allocation_presets"]["members"][0]
        .as_object_mut()
        .unwrap()
        .remove("equipment");
    assert!(matches!(
        decode_draft(&serde_json::to_vec(&wire).unwrap(), limits()),
        Err(DraftCodecError::Json(_))
    ));
    wire["schema_version"] = json!(1);
    assert!(matches!(
        decode_draft(&serde_json::to_vec(&wire).unwrap(), limits()),
        Err(DraftCodecError::UnsupportedVersion(1))
    ));
    let duplicate=String::from_utf8(bytes).unwrap().replacen("\"equipment\":{\"members\":[", "\"equipment\":{\"members\":[],\"completion\":{\"kind\":\"complete\"}},\"equipment\":{\"members\":[",1);
    assert!(matches!(
        decode_draft(duplicate.as_bytes(), limits()),
        Err(DraftCodecError::Json(_))
    ));
}

#[test]
fn choice_reward_pending_closure_blocks_empty_and_overlap_and_known_overlap_preserves_request() {
    let baseline = ready(&DraftSession::new(input(), limits()).unwrap(), selection());
    for members in [vec![], vec![id::<RewardSelectionId>(30)]] {
        let mut raw = input();
        raw.choice_presets.members[0].rewards = DraftList {
            members,
            completion: open(226),
        };
        let session = DraftSession::new(raw.clone(), limits()).unwrap();
        let (issues, _) = pending_result(&session);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].id, id(226));
        assert_eq!(
            issues[0].owner,
            Some(id::<ChoicePresetId>(140).instance_id())
        );
        raw.choice_presets.members[0].rewards.completion = DraftListCompletion::Complete;
        let known = ready(&DraftSession::new(raw, limits()).unwrap(), selection());
        assert_eq!(known.request(), baseline.request());
        assert_eq!(known.request_digest(), baseline.request_digest());
    }
}

#[test]
fn contributed_reward_fields_block_only_the_selected_choice_alternative() {
    let mut raw = input();
    let reward = RewardSelection {
        id: id(31),
        definition: def("reward"),
        parameters: vec![ParameterAssignment {
            slot: slot(SlotOwnerDefId::Reward(def("reward")), "reward-parameter"),
            value: ParameterValue::Boolean(true),
        }],
    };
    let mut reward: RewardDraft = reward.into();
    reward.parameters.members[0].value = DraftField::Pending(pending(227, vec![]));
    raw.rewards.members.push(reward);
    raw.choice_presets.members.push(ChoicePresetDraft {
        id: id(141),
        choices: Vec::<MechanicChoice>::new().into(),
        rewards: vec![id::<RewardSelectionId>(31)].into(),
    });
    let session = DraftSession::new(raw.clone(), limits()).unwrap();
    assert_eq!(
        ready(&session, selection()).request(),
        ready(&DraftSession::new(input(), limits()).unwrap(), selection()).request()
    );
    let selected = EvaluationSelection {
        build: VariantSelection {
            choices: id(141),
            ..selection().build
        },
        ..selection()
    };
    let DraftFinalization::Pending { issues, .. } =
        session.finalize_selection(selected, limits()).unwrap()
    else {
        panic!("selected reward input not required")
    };
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].id, id(227));
    assert_eq!(
        issues[0].owner,
        Some(id::<RewardSelectionId>(31).instance_id())
    );
    raw.rewards.members[1].parameters.members[0].value = ParameterValue::Boolean(true).into();
    let resolved = ready(&DraftSession::new(raw, limits()).unwrap(), selected);
    assert_eq!(
        resolved
            .request()
            .build()
            .input()
            .character
            .rewards
            .iter()
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        vec![id(30), id(31)]
    );
}

#[test]
fn draft_choice_reward_membership_and_required_wire_field_have_no_implicit_defaults() {
    for refs in [
        vec![id::<RewardSelectionId>(30), id(30)],
        vec![id(40)],
        vec![id(999)],
    ] {
        let mut raw = input();
        raw.choice_presets.members[0].rewards = refs.into();
        assert!(DraftSession::new(raw, limits()).is_err());
    }
    let session = DraftSession::new(input(), limits()).unwrap();
    let mut value: serde_json::Value =
        serde_json::from_slice(&encode_draft(&session, limits()).unwrap()).unwrap();
    value["draft"]["choice_presets"]["members"][0]
        .as_object_mut()
        .unwrap()
        .remove("rewards");
    assert!(matches!(
        decode_draft(&serde_json::to_vec(&value).unwrap(), limits()),
        Err(DraftCodecError::Json(_))
    ));
    value["schema_version"] = json!(1);
    assert!(matches!(
        decode_draft(&serde_json::to_vec(&value).unwrap(), limits()),
        Err(DraftCodecError::UnsupportedVersion(1))
    ));
}
