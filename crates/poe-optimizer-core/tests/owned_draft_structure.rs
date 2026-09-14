use poe_optimizer_core::{build_identity::*, owned_build::*, owned_definitions::*, owned_draft::*};

fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("draft-validation", "v1").unwrap()
}
fn lineage() -> BuildLineage {
    BuildLineage::from_bytes([0x63; 16])
}
fn id<T: BuildInstanceId>(local: u64) -> T {
    T::from_instance_id(InstanceId::from_parts(lineage(), local).unwrap())
}
fn def<K: DefinitionDomain>(key: &str) -> DefId<K> {
    DefId::parse(ns(), key).unwrap()
}
fn list<T>(members: Vec<T>) -> DraftList<T> {
    DraftList {
        members,
        completion: DraftListCompletion::Complete,
    }
}
fn pending<T>(local: u64, candidates: Vec<T>) -> PendingValue<T> {
    PendingValue {
        id: id(local),
        code: OwnedDefinitionKey::new("needs-mapping").unwrap(),
        candidates,
    }
}
fn parameter(owner: SlotOwnerDefId) -> ParameterAssignment {
    ParameterAssignment {
        slot: DeclaredSlot {
            declaration: owner,
            slot: def("property"),
        },
        value: ParameterValue::Boolean(true),
    }
}
fn empty() -> DraftSessionInput {
    DraftSessionInput {
        allocator: InstanceAllocatorState::from_parts(lineage(), 200),
        revision: BuildRevision::INITIAL,
        game_version: ns(),
        weapon_loadouts: list(vec![]),
        items: list(vec![]),
        gems: list(vec![]),
        rewards: list(vec![]),
        equipment: list(vec![]),
        allocations: list(vec![]),
        skills: list(vec![]),
        supports: list(vec![]),
        payload_links: list(vec![]),
        character_presets: list(vec![]),
        equipment_presets: list(vec![]),
        allocation_presets: list(vec![]),
        skill_presets: list(vec![]),
        choice_presets: list(vec![]),
        scenario_presets: list(vec![]),
        query_presets: list(vec![]),
        saved_variants: list(vec![]),
    }
}
fn bare_item(local: u64) -> ItemDraft {
    ItemRecord {
        id: id(local),
        template: def("item"),
        parameters: vec![],
        item_level: 77,
        quality: None,
        modifiers: vec![],
    }
    .into()
}
fn grant() -> DeclaredSlot<GrantSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Gem(def("gem")),
        slot: def("grant"),
    }
}
fn owned_actor(root: ProviderRoot) -> ActorKey {
    ActorKey::Owned(Box::new(OwnedActorKey {
        provider: ProviderKey {
            root,
            grant_path: vec![grant()],
        },
        slot: DeclaredSlot {
            declaration: SlotOwnerDefId::Gem(def("gem")),
            slot: def("actor"),
        },
    }))
}
fn query(root: ProviderRoot) -> MetricRequestDraft {
    MetricRequest {
        id: QueryId::new("result").unwrap(),
        metric: def("metric"),
        target: MetricTarget::Actor(owned_actor(root)),
    }
    .into()
}
fn fixture() -> DraftSessionInput {
    let mut input = empty();
    input.weapon_loadouts = list(vec![id(1)]);
    input.items = list(vec![
        ItemRecord {
            id: id(2),
            template: def("item"),
            parameters: vec![parameter(SlotOwnerDefId::ItemTemplate(def("item")))],
            item_level: 77,
            quality: None,
            modifiers: vec![RolledModifier {
                id: id(3),
                definition: def("modifier"),
                rolls: vec![parameter(SlotOwnerDefId::Modifier(def("modifier")))],
            }],
        }
        .into(),
    ]);
    input.gems = list(vec![
        GemInstance {
            id: id(4),
            definition: def("gem"),
            parameters: vec![parameter(SlotOwnerDefId::Gem(def("gem")))],
            level: 9,
            quality: None,
        }
        .into(),
    ]);
    input.rewards = list(vec![
        RewardSelection {
            id: id(5),
            definition: def("reward"),
            parameters: vec![parameter(SlotOwnerDefId::Reward(def("reward")))],
        }
        .into(),
    ]);
    input.equipment = list(vec![
        EquipmentUse {
            id: id(6),
            item: id(2),
            destination: EquipmentDestination::CharacterSlot(def("hand")),
            scope: LoadoutScope::Shared,
        }
        .into(),
    ]);
    input.allocations = list(vec![
        Allocation {
            id: id(7),
            node: def("node"),
            pool: def("ordinary"),
            scope: LoadoutScope::Shared,
            access: AllocationAccess::Ordinary,
            choices: vec![],
        }
        .into(),
    ]);
    input.skills = list(vec![
        SkillUse {
            id: id(8),
            source: AuthoredSkillSource::Gem(id(4)),
            enabled: true,
            scope: LoadoutScope::Shared,
        }
        .into(),
        SkillUse {
            id: id(11),
            source: AuthoredSkillSource::Direct(def("skill")),
            enabled: false,
            scope: LoadoutScope::Selected {
                loadouts: vec![id(1)],
            },
        }
        .into(),
    ]);
    input.supports = list(vec![
        SupportAssignment {
            id: id(9),
            support: id(4),
            target: SkillTarget::Authored(id(8)),
            enabled: true,
        }
        .into(),
    ]);
    input.payload_links = list(vec![
        PayloadLink {
            id: id(10),
            container: id(8),
            payload: id(11),
            role: def("payload"),
        }
        .into(),
    ]);
    input.character_presets = list(vec![CharacterPresetDraft {
        id: id(20),
        class: def::<ClassDefinition>("class").into(),
        ascendancy: Some(def::<AscendancyDefinition>("ascendancy")).into(),
        level: 77.into(),
        rewards: list(vec![id(5)]),
    }]);
    input.equipment_presets = list(vec![EquipmentPresetDraft {
        id: id(21),
        equipment: list(vec![id(6)]),
    }]);
    input.allocation_presets = list(vec![AllocationPresetDraft {
        id: id(22),
        allocations: list(vec![id(7)]),
        equipment: list(vec![]),
    }]);
    input.skill_presets = list(vec![SkillPresetDraft {
        id: id(23),
        skills: list(vec![id(8), id(11)]),
        supports: list(vec![id(9)]),
        payload_links: list(vec![id(10)]),
    }]);
    input.choice_presets = list(vec![ChoicePresetDraft {
        id: id(24),
        rewards: list(vec![]),
        choices: list(vec![]),
    }]);
    input.scenario_presets = list(vec![ScenarioPresetDraft {
        id: id(25),
        scenario: ScenarioInput {
            game_version: ns(),
            enemy: EnemySpec {
                encounter: def("encounter"),
                level: 80,
            },
            assumptions: vec![ExternalAssumption {
                input: def("input"),
                target: AssumptionTarget::Environment,
                value: ParameterValue::Boolean(true),
            }],
            usage: vec![UsagePolicySelection {
                policy: def("policy"),
                target: UsageTarget::Actor(ActorKey::Player),
                parameters: vec![parameter(SlotOwnerDefId::UsagePolicy(def("policy")))],
            }],
        }
        .into(),
    }]);
    input.query_presets = list(vec![QueryPresetDraft {
        id: id(26),
        queries: QueryDraft {
            game_version: ns(),
            requests: list(vec![query(ProviderRoot::SupportAssignment(id(9)))]),
        },
    }]);
    input.saved_variants = list(vec![SavedVariantDraft {
        id: id(27),
        selection: SelectionDraft {
            character: id::<CharacterPresetId>(20).into(),
            equipment: id::<EquipmentPresetId>(21).into(),
            allocations: id::<AllocationPresetId>(22).into(),
            skills: id::<SkillPresetId>(23).into(),
            choices: id::<ChoicePresetId>(24).into(),
            active_weapon_loadout: id::<WeaponLoadoutId>(1).into(),
            scenario: id::<ScenarioPresetId>(25).into(),
            queries: id::<QueryPresetId>(26).into(),
        },
    }]);
    input
}
fn check(input: &DraftSessionInput) -> Result<DraftValidation, StructuralError> {
    validate_draft(input, DraftLimits::default())
}

#[test]
fn fully_authored_records_validate_without_source_and_keep_query_order_and_independent_pools() {
    let mut input = fixture();
    let mut ascendancy = input.allocations.members[0].clone();
    ascendancy.id = id(12);
    ascendancy.pool = def::<PointPoolDefinition>("ascendancy").into();
    input.allocations.members.push(ascendancy);
    input.allocation_presets.members[0]
        .allocations
        .members
        .push(id(12));
    let mut second = query(ProviderRoot::ItemModifier {
        equipment_use: id(6),
        modifier: id(3),
    });
    second.id = QueryId::new("first-alphabetically").unwrap();
    input.query_presets.members[0]
        .queries
        .requests
        .members
        .push(second);
    let before = input.clone();
    assert!(check(&input).unwrap().issues.is_empty());
    assert_eq!(input, before);
}

#[test]
fn issues_report_exact_paths_and_top_level_ownership_without_losing_known_siblings() {
    let mut input = fixture();
    input.items.members[0].modifiers.members[0].rolls.members[0].value =
        DraftField::Pending(pending(101, vec![ParameterValue::Boolean(false)]));
    input.items.completion = DraftListCompletion::Pending {
        id: id(102),
        code: OwnedDefinitionKey::new("more-items").unwrap(),
    };
    input.scenario_presets.members[0].scenario.enemy.level =
        DraftField::Pending(pending(103, vec![81]));
    input.query_presets.members[0].queries.requests.members[0].metric =
        DraftField::Pending(pending(104, vec![]));
    input.skill_presets.members[0].skills.completion = DraftListCompletion::Pending {
        id: id(105),
        code: OwnedDefinitionKey::new("more-skills").unwrap(),
    };
    let before = input.clone();
    let issues = check(&input).unwrap().issues;
    assert_eq!(issues.len(), 5);
    let expected = [
        (
            101,
            Some(id::<ItemRecordId>(2).instance_id()),
            "items.members[0].modifiers.members[0].rolls.members[0].value",
        ),
        (102, None, "items.completion"),
        (
            103,
            Some(id::<ScenarioPresetId>(25).instance_id()),
            "scenario_presets.members[0].scenario.enemy.level",
        ),
        (
            104,
            Some(id::<QueryPresetId>(26).instance_id()),
            "query_presets.members[0].queries.requests.members[0].metric",
        ),
        (
            105,
            Some(id::<SkillPresetId>(23).instance_id()),
            "skill_presets.members[0].skills.completion",
        ),
    ];
    for (local, owner, path) in expected {
        let issue = issues.iter().find(|issue| issue.id == id(local)).unwrap();
        assert_eq!(issue.owner, owner);
        assert_eq!(issue.path, path);
    }
    assert_eq!(input, before);
    assert!(input.items.members[0].to_resolved().is_none());
}

#[test]
fn all_occurrences_and_pending_issues_share_one_lineage_watermark_and_kind_registry() {
    let mut input = fixture();
    input.gems.members[0].id = id(2);
    assert!(matches!(
        check(&input).unwrap_err().kind,
        StructuralErrorKind::DuplicateIdentity { .. }
    ));
    let mut input = fixture();
    input.items.members[0].template = DraftField::Pending(pending(26, vec![]));
    assert!(matches!(
        check(&input).unwrap_err().kind,
        StructuralErrorKind::DuplicateIdentity { .. }
    ));
    let mut input = fixture();
    input.items.members[0].template = DraftField::Pending(pending(201, vec![]));
    assert_eq!(
        check(&input).unwrap_err().kind,
        StructuralErrorKind::BeyondWatermark
    );
    let mut input = fixture();
    let mut value = pending(101, vec![]);
    value.id = DraftIssueId::from_instance_id(
        InstanceId::from_parts(BuildLineage::from_bytes([0x64; 16]), 101).unwrap(),
    );
    input.items.members[0].template = DraftField::Pending(value);
    assert_eq!(
        check(&input).unwrap_err().kind,
        StructuralErrorKind::ForeignLineage
    );
}

#[test]
fn historical_references_allow_absence_but_never_wrong_live_domains_or_future_ids() {
    let mut input = fixture();
    input.query_presets.members[0].queries.requests.members[0] =
        query(ProviderRoot::SupportAssignment(id(88)));
    input.scenario_presets.members[0]
        .scenario
        .assumptions
        .members[0]
        .target =
        AssumptionTarget::Actor(owned_actor(ProviderRoot::SupportAssignment(id(88)))).into();
    assert!(check(&input).is_ok());
    input.query_presets.members[0].queries.requests.members[0] =
        query(ProviderRoot::SupportAssignment(id(2)));
    assert!(matches!(
        check(&input).unwrap_err().kind,
        StructuralErrorKind::MissingReference {
            expected: OccurrenceKind::SupportAssignment,
            ..
        }
    ));
    input.query_presets.members[0].queries.requests.members[0] =
        query(ProviderRoot::SupportAssignment(id(101)));
    input.items.members[0].template = DraftField::Pending(pending(101, vec![]));
    assert!(matches!(
        check(&input).unwrap_err().kind,
        StructuralErrorKind::MissingReference {
            expected: OccurrenceKind::SupportAssignment,
            ..
        }
    ));
    input.query_presets.members[0].queries.requests.members[0] =
        query(ProviderRoot::SupportAssignment(id(201)));
    assert_eq!(
        check(&input).unwrap_err().kind,
        StructuralErrorKind::BeyondWatermark
    );
    let mut input = fixture();
    input.supports.members[0].target = SkillTarget::Authored(id(88)).into();
    assert!(matches!(
        check(&input).unwrap_err().kind,
        StructuralErrorKind::MissingReference {
            expected: OccurrenceKind::SkillUse,
            ..
        }
    ));
}

#[test]
fn candidate_references_and_namespaces_are_validated_without_becoming_active_facts() {
    let mut input = fixture();
    input.supports.members[0].target =
        DraftSkillTarget::Pending(pending(101, vec![SkillTarget::Authored(id(2))]));
    let error = check(&input).unwrap_err();
    assert!(error.path.contains("candidates[0]"));
    assert!(matches!(
        error.kind,
        StructuralErrorKind::MissingReference { .. }
    ));
    let mut input = fixture();
    input.items.members[0].template = DraftField::Pending(pending(
        101,
        vec![DefId::parse(GameVersionNamespace::new("other", "v1").unwrap(), "item").unwrap()],
    ));
    assert_eq!(
        check(&input).unwrap_err().kind,
        StructuralErrorKind::ForeignNamespace
    );
    let mut input = fixture();
    input.equipment.members[0].scope = DraftField::Pending(pending(
        101,
        vec![LoadoutScope::Selected { loadouts: vec![] }],
    ));
    assert_eq!(
        check(&input).unwrap_err().kind,
        StructuralErrorKind::EmptyLoadoutScope
    );
}

#[test]
fn known_containment_cycles_reject_even_when_socket_pending_but_candidate_edges_are_not_selected() {
    let mut input = fixture();
    let mut second = input.equipment.members[0].clone();
    second.id = id(12);
    second.destination = EquipmentDestination::ItemSocket {
        container: id(6),
        slot: def("socket"),
    }
    .into();
    input.equipment.members.push(second);
    input.equipment.members[0].destination = DraftEquipmentDestination::ItemSocket {
        container: id::<ItemSlotUseId>(12).into(),
        slot: DraftField::Pending(pending(101, vec![])),
    };
    assert_eq!(
        check(&input).unwrap_err().kind,
        StructuralErrorKind::ContainmentCycle
    );
    input.equipment.members[0].destination = DraftEquipmentDestination::Pending(pending(
        101,
        vec![EquipmentDestination::ItemSocket {
            container: id(12),
            slot: def("socket"),
        }],
    ));
    assert_eq!(check(&input).unwrap().issues.len(), 1);
}

#[test]
fn modifier_provider_ownership_uses_known_item_edges_and_preserves_unknown_item_identity() {
    let mut input = fixture();
    input.items.members.push(bare_item(12));
    input.equipment.members[0].item = id::<ItemRecordId>(12).into();
    input.query_presets.members[0].queries.requests.members[0] =
        query(ProviderRoot::ItemModifier {
            equipment_use: id(6),
            modifier: id(3),
        });
    assert_eq!(
        check(&input).unwrap_err().kind,
        StructuralErrorKind::WrongProviderOwner
    );
    input.equipment.members[0].item = DraftField::Pending(pending(101, vec![id(12), id(2)]));
    assert!(check(&input).is_ok());
    input.equipment.members[0].item = id::<ItemRecordId>(12).into();
    input.query_presets.members[0].queries.requests.members[0].target =
        DraftMetricTarget::Pending(pending(
            102,
            vec![MetricTarget::Actor(owned_actor(
                ProviderRoot::ItemModifier {
                    equipment_use: id(6),
                    modifier: id(3),
                },
            ))],
        ));
    assert_eq!(
        check(&input).unwrap_err().kind,
        StructuralErrorKind::WrongProviderOwner
    );
}

#[test]
fn known_parameter_slots_reject_duplicates_and_wrong_declarations_even_when_values_pending() {
    let mut input = fixture();
    let mut duplicate = input.items.members[0].parameters.members[0].clone();
    duplicate.value = DraftField::Pending(pending(101, vec![]));
    input.items.members[0].parameters.members.push(duplicate);
    assert_eq!(
        check(&input).unwrap_err().kind,
        StructuralErrorKind::DuplicateAssignment
    );
    let mut input = fixture();
    input.items.members[0].parameters.members[0] =
        parameter(SlotOwnerDefId::Gem(def("gem"))).into();
    assert_eq!(
        check(&input).unwrap_err().kind,
        StructuralErrorKind::WrongDeclaration
    );
    input.items.members[0].template = DraftField::Pending(pending(101, vec![]));
    assert_eq!(
        check(&input).unwrap_err().kind,
        StructuralErrorKind::WrongDeclaration
    );
    input.items.members[0].parameters.members[0] =
        parameter(SlotOwnerDefId::ItemTemplate(def("another-item"))).into();
    assert!(check(&input).is_ok());
    let mut input = fixture();
    input.items.members[0].parameters.members[0].slot = DraftField::Pending(pending(
        101,
        vec![parameter(SlotOwnerDefId::Gem(def("gem"))).slot],
    ));
    assert_eq!(
        check(&input).unwrap_err().kind,
        StructuralErrorKind::WrongDeclaration
    );
}

#[test]
fn preset_memberships_reject_duplicates_and_wrong_domains_without_inventing_defaults() {
    assert!(check(&empty()).is_ok());
    let mut input = fixture();
    input.skill_presets.members[0].skills.members.push(id(8));
    assert_eq!(
        check(&input).unwrap_err().kind,
        StructuralErrorKind::DuplicateAssignment
    );
    let mut input = fixture();
    input.saved_variants.members[0].selection.scenario = id::<ScenarioPresetId>(26).into();
    assert!(matches!(
        check(&input).unwrap_err().kind,
        StructuralErrorKind::MissingReference {
            expected: OccurrenceKind::ScenarioPreset,
            ..
        }
    ));
}

#[test]
fn equal_and_conflicting_alias_choice_assignments_both_reject_before_resolution() {
    let choice = ChoiceSelection {
        slot: DeclaredSlot {
            declaration: SlotOwnerDefId::Gem(def("gem")),
            slot: def("choice"),
        },
        value: ParameterValue::Boolean(true),
    };
    for second_value in [
        ParameterValue::Boolean(true),
        ParameterValue::Boolean(false),
    ] {
        let mut input = fixture();
        let mut other = choice.clone();
        other.value = second_value;
        input.choice_presets.members[0].choices = list(vec![
            MechanicChoice {
                owner: ChoiceOwner::Character,
                choice: choice.clone(),
            }
            .into(),
            MechanicChoice {
                owner: ChoiceOwner::Provider(ProviderKey {
                    root: ProviderRoot::Character,
                    grant_path: vec![],
                }),
                choice: other,
            }
            .into(),
        ]);
        assert_eq!(
            check(&input).unwrap_err().kind,
            StructuralErrorKind::DuplicateAssignment
        );
    }
}

#[test]
fn candidate_issue_and_collection_counts_share_one_aggregate_budget() {
    let mut input = empty();
    let mut item = bare_item(2);
    item.template = DraftField::Pending(pending(101, vec![def("a"), def("b")]));
    input.items = list(vec![item]);
    let limits = DraftLimits {
        input: OwnedInputLimits {
            max_entries: 4,
            ..OwnedInputLimits::default()
        },
        ..DraftLimits::default()
    };
    assert!(validate_draft(&input, limits).is_ok());
    let tighter = DraftLimits {
        input: OwnedInputLimits {
            max_entries: 3,
            ..limits.input
        },
        ..limits
    };
    assert_eq!(
        validate_draft(&input, tighter).unwrap_err().kind,
        StructuralErrorKind::LimitExceeded
    );
    let tighter = DraftLimits {
        max_candidates_per_field: 1,
        ..DraftLimits::default()
    };
    assert_eq!(
        validate_draft(&input, tighter).unwrap_err().kind,
        StructuralErrorKind::LimitExceeded
    );
    let tighter = DraftLimits {
        input: OwnedInputLimits {
            max_collection_entries: 1,
            ..OwnedInputLimits::default()
        },
        ..DraftLimits::default()
    };
    assert_eq!(
        validate_draft(&input, tighter).unwrap_err().kind,
        StructuralErrorKind::LimitExceeded
    );
    input.items.members[0].item_level = DraftField::Pending(pending(102, vec![]));
    assert_eq!(
        validate_draft(
            &input,
            DraftLimits {
                max_issues: 1,
                ..DraftLimits::default()
            }
        )
        .unwrap_err()
        .kind,
        StructuralErrorKind::LimitExceeded
    );
}

#[test]
fn known_and_candidate_provider_paths_obey_the_same_bounds() {
    let mut input = fixture();
    let mut actor = owned_actor(ProviderRoot::SupportAssignment(id(9)));
    let ActorKey::Owned(value) = &mut actor else {
        unreachable!()
    };
    value.provider.grant_path.push(grant());
    let limits = DraftLimits {
        input: OwnedInputLimits {
            max_provider_steps: 1,
            ..OwnedInputLimits::default()
        },
        ..DraftLimits::default()
    };
    input.query_presets.members[0].queries.requests.members[0].target =
        MetricTarget::Actor(actor.clone()).into();
    assert_eq!(
        validate_draft(&input, limits).unwrap_err().kind,
        StructuralErrorKind::LimitExceeded
    );
    input.query_presets.members[0].queries.requests.members[0].target =
        DraftMetricTarget::Pending(pending(101, vec![MetricTarget::Actor(actor)]));
    assert_eq!(
        validate_draft(&input, limits).unwrap_err().kind,
        StructuralErrorKind::LimitExceeded
    );
    for limits in [
        DraftLimits {
            max_issues: 0,
            ..DraftLimits::default()
        },
        DraftLimits {
            max_candidates_per_field: 65,
            ..DraftLimits::default()
        },
    ] {
        assert_eq!(
            validate_draft(&empty(), limits).unwrap_err().kind,
            StructuralErrorKind::InvalidLimit
        );
    }
}

#[test]
fn pending_parent_definitions_still_require_the_static_parameter_owner_family() {
    let wrong = parameter(SlotOwnerDefId::ItemTemplate(def("item")));
    let mut input = fixture();
    input.gems.members[0].definition = DraftField::Pending(pending(101, vec![]));
    input.gems.members[0].parameters.members[0] = wrong.clone().into();
    assert_eq!(
        check(&input).unwrap_err().kind,
        StructuralErrorKind::WrongDeclaration
    );
    let mut input = fixture();
    input.items.members[0].modifiers.members[0].definition =
        DraftField::Pending(pending(101, vec![]));
    input.items.members[0].modifiers.members[0].rolls.members[0] = wrong.clone().into();
    assert_eq!(
        check(&input).unwrap_err().kind,
        StructuralErrorKind::WrongDeclaration
    );
    let mut input = fixture();
    input.rewards.members[0].definition = DraftField::Pending(pending(101, vec![]));
    input.rewards.members[0].parameters.members[0] = wrong.clone().into();
    assert_eq!(
        check(&input).unwrap_err().kind,
        StructuralErrorKind::WrongDeclaration
    );
    let mut input = fixture();
    input.scenario_presets.members[0].scenario.usage.members[0].policy =
        DraftField::Pending(pending(101, vec![]));
    input.scenario_presets.members[0].scenario.usage.members[0]
        .parameters
        .members[0] = wrong.into();
    assert_eq!(
        check(&input).unwrap_err().kind,
        StructuralErrorKind::WrongDeclaration
    );
    let mut input = fixture();
    input.items.members[0].template = DraftField::Pending(pending(101, vec![]));
    input.items.members[0].parameters.members[0].slot = DraftField::Pending(pending(
        102,
        vec![parameter(SlotOwnerDefId::Gem(def("gem"))).slot],
    ));
    assert_eq!(
        check(&input).unwrap_err().kind,
        StructuralErrorKind::WrongDeclaration
    );
}
