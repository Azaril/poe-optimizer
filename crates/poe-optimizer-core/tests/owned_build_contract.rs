//! Direct-author contract tests: no source format, game package or evaluator required.
use poe_optimizer_core::{build_identity::*, owned_build::*, owned_definitions::*};
use serde_json::{Value, json};
use std::fmt::{Debug, Display};

fn limits() -> OwnedInputLimits {
    OwnedInputLimits::default()
}
fn lineage() -> BuildLineage {
    BuildLineage::from_bytes([0x42; 16])
}
fn id<T: BuildInstanceId>(local: u64) -> T {
    T::from_instance_id(InstanceId::from_parts(lineage(), local).unwrap())
}
fn definition<K: DefinitionDomain>(key: &str) -> DefId<K> {
    DefId::parse(namespace(), key).unwrap()
}
fn namespace() -> GameVersionNamespace {
    GameVersionNamespace::new("test-game", "v1").unwrap()
}
fn declared<K: DefinitionDomain>(owner: SlotOwnerDefId, key: &str) -> DeclaredSlot<DefId<K>> {
    DeclaredSlot {
        declaration: owner,
        slot: definition(key),
    }
}
fn quality() -> QualitySelection {
    QualitySelection {
        kind: definition("authored-quality"),
        amount: FiniteQuantity::new(12.5, definition("quality-unit")).unwrap(),
    }
}
fn build_input() -> BuildInput {
    let support_owner = SlotOwnerDefId::Gem(definition("support-gem"));
    BuildInput {
        allocator: InstanceAllocatorState::from_parts(lineage(), 100),
        revision: BuildRevision::from_u64(7),
        game_version: namespace(),
        character: CharacterSpec {
            class: definition("class-a"),
            ascendancy: Some(definition("ascendancy-a")),
            level: 60,
            rewards: vec![RewardSelection {
                id: id(14),
                definition: definition("earned-reward"),
                parameters: vec![ParameterAssignment {
                    slot: declared(
                        SlotOwnerDefId::Reward(definition("earned-reward")),
                        "reward-choice",
                    ),
                    value: ParameterValue::Boolean(true),
                }],
            }],
        },
        weapon_loadouts: vec![id(2), id(1)],
        active_weapon_loadout: id(2),
        items: vec![
            ItemRecord {
                id: id(18),
                template: definition("template-a"),
                parameters: vec![],
                item_level: Some(60),
                quality: None,
                modifiers: vec![],
            },
            ItemRecord {
                id: id(3),
                template: definition("template-a"),
                parameters: vec![ParameterAssignment {
                    slot: declared(
                        SlotOwnerDefId::ItemTemplate(definition("template-a")),
                        "intrinsic-state",
                    ),
                    value: ParameterValue::Boolean(true),
                }],
                item_level: Some(60),
                quality: Some(quality()),
                modifiers: vec![RolledModifier {
                    id: id(15),
                    definition: definition("rolled-modifier"),
                    rolls: vec![ParameterAssignment {
                        slot: declared(
                            SlotOwnerDefId::Modifier(definition("rolled-modifier")),
                            "magnitude",
                        ),
                        value: ParameterValue::Quantity(
                            FiniteQuantity::new(-3.5, definition("resource-unit")).unwrap(),
                        ),
                    }],
                }],
            },
        ],
        gems: vec![
            GemInstance {
                id: id(17),
                definition: definition("active-gem"),
                parameters: vec![],
                level: 1,
                quality: None,
            },
            GemInstance {
                id: id(10),
                definition: definition("support-gem"),
                parameters: vec![],
                level: 1,
                quality: None,
            },
            GemInstance {
                id: id(6),
                definition: definition("active-gem"),
                parameters: vec![ParameterAssignment {
                    slot: declared(
                        SlotOwnerDefId::Gem(definition("active-gem")),
                        "intrinsic-variant",
                    ),
                    value: ParameterValue::Option(definition("alternate-variant")),
                }],
                level: 1,
                quality: Some(quality()),
            },
        ],
        equipment: vec![
            EquipmentUse {
                id: id(5),
                item: id(3),
                destination: EquipmentDestination::CharacterSlot(definition("slot-b")),
                scope: LoadoutScope::Shared,
            },
            EquipmentUse {
                id: id(4),
                item: id(3),
                destination: EquipmentDestination::CharacterSlot(definition("slot-a")),
                scope: LoadoutScope::Selected {
                    loadouts: vec![id(2), id(1)],
                },
            },
        ],
        allocations: vec![
            Allocation {
                id: id(13),
                node: definition("disconnected-special-node"),
                pool: definition("ascendancy-pool"),
                scope: LoadoutScope::Shared,
                access: AllocationAccess::Ordinary,
                choices: vec![],
            },
            Allocation {
                id: id(12),
                node: definition("ordinary-node"),
                pool: definition("ordinary-pool"),
                scope: LoadoutScope::Shared,
                access: AllocationAccess::Ordinary,
                choices: vec![ChoiceSelection {
                    slot: declared(
                        SlotOwnerDefId::PassiveNode(definition("ordinary-node")),
                        "attribute-choice",
                    ),
                    value: ParameterValue::Option(definition("attribute-option")),
                }],
            },
        ],
        skills: vec![
            SkillUse {
                id: id(19),
                source: AuthoredSkillSource::Direct(definition("direct-action")),
                enabled: false,
                scope: LoadoutScope::Shared,
            },
            SkillUse {
                id: id(9),
                source: AuthoredSkillSource::Gem(id(17)),
                enabled: true,
                scope: LoadoutScope::Shared,
            },
            SkillUse {
                id: id(8),
                source: AuthoredSkillSource::Gem(id(6)),
                enabled: true,
                scope: LoadoutScope::Shared,
            },
        ],
        supports: vec![SupportAssignment {
            id: id(11),
            support: id(10),
            target: SkillTarget::Authored(id(8)),
            enabled: true,
        }],
        payload_links: vec![PayloadLink {
            id: id(16),
            container: id(8),
            payload: id(9),
            role: definition("contained-payload"),
        }],
        choices: vec![MechanicChoice {
            owner: ChoiceOwner::Provider(ProviderKey {
                root: ProviderRoot::SupportAssignment(id(11)),
                grant_path: vec![],
            }),
            choice: ChoiceSelection {
                slot: declared(support_owner, "count-choice"),
                value: ParameterValue::Integer(BoundedInteger::new(2).unwrap()),
            },
        }],
    }
}
fn provider(root: ProviderRoot) -> ProviderKey {
    ProviderKey {
        root,
        grant_path: vec![declared(
            SlotOwnerDefId::Gem(definition("support-gem")),
            "secondary-grant",
        )],
    }
}
fn action(root: ProviderRoot) -> ActionSelection {
    let provider = provider(root);
    let actor = ActorKey::Owned(Box::new(OwnedActorKey {
        provider: provider.clone(),
        slot: declared(
            SlotOwnerDefId::Gem(definition("support-gem")),
            "owned-actor",
        ),
    }));
    ActionSelection {
        action: ActionKey {
            actor,
            provider,
            output: declared(
                SlotOwnerDefId::Gem(definition("support-gem")),
                "secondary-output",
            ),
        },
        part: definition("primary-part"),
        mode: definition("damage-per-time"),
        stat_set: definition("normal-stat-set"),
    }
}
fn query_input(root: ProviderRoot) -> QueryInput {
    QueryInput {
        game_version: namespace(),
        requests: vec![
            MetricRequest {
                id: QueryId::new("z-action").unwrap(),
                metric: definition("action-damage"),
                target: MetricTarget::Action(Box::new(action(root))),
            },
            MetricRequest {
                id: QueryId::new("a-player").unwrap(),
                metric: definition("player-life"),
                target: MetricTarget::Actor(ActorKey::Player),
            },
        ],
    }
}
fn scenario_input() -> ScenarioInput {
    ScenarioInput {
        game_version: namespace(),
        enemy: EnemySpec {
            encounter: definition("authored-encounter"),
            level: 82,
        },
        assumptions: vec![ExternalAssumption {
            input: definition("external-enemy-condition"),
            target: AssumptionTarget::Enemy,
            value: ParameterValue::Boolean(false),
        }],
        usage: vec![UsagePolicySelection {
            policy: definition("explicit-usage"),
            target: UsageTarget::Actor(ActorKey::Player),
            parameters: vec![ParameterAssignment {
                slot: declared(
                    SlotOwnerDefId::UsagePolicy(definition("explicit-usage")),
                    "usage-magnitude",
                ),
                value: ParameterValue::Quantity(
                    FiniteQuantity::new(0.5, definition("ratio-unit")).unwrap(),
                ),
            }],
        }],
    }
}
fn build() -> BuildSpec {
    BuildSpec::new(build_input(), limits()).unwrap()
}
fn scenario() -> ScenarioSpec {
    ScenarioSpec::new(scenario_input(), limits()).unwrap()
}
fn queries(root: ProviderRoot) -> QuerySpec {
    QuerySpec::new(query_input(root), limits()).unwrap()
}
fn request() -> OwnedEvaluationRequest {
    OwnedEvaluationRequest::new(
        build(),
        scenario(),
        queries(ProviderRoot::SupportAssignment(id(11))),
        limits(),
    )
    .unwrap()
}
fn fails<T: Debug, E: Display>(result: Result<T, E>, words: &[&str]) {
    let error = match result {
        Ok(value) => panic!("unexpected success: {value:?}"),
        Err(error) => error.to_string().to_lowercase(),
    };
    assert!(
        words.iter().any(|word| error.contains(word)),
        "expected {words:?} in diagnostic: {error}"
    );
}
fn encoded_build() -> Value {
    serde_json::from_slice(
        &encode_owned(&OwnedDocument::Build(Box::new(build())), limits()).unwrap(),
    )
    .unwrap()
}
fn decode_value(value: Value) -> Result<OwnedDocument, CodecError> {
    decode_owned(&serde_json::to_vec(&value).unwrap(), limits())
}

#[test]
fn direct_authored_documents_roundtrip_without_source_or_definition_packages() {
    let request = request();
    assert_eq!(request.build().input().allocator.last_issued(), 100);
    assert_eq!(request.build().input().revision.get(), 7);
    assert_eq!(request.scenario().input().enemy.level, 82);
    assert_eq!(
        request.queries().input().requests[0].id.as_str(),
        "z-action"
    );
    for document in [
        OwnedDocument::Build(Box::new(build())),
        OwnedDocument::Scenario(scenario()),
        OwnedDocument::Query(queries(ProviderRoot::SupportAssignment(id(11)))),
        OwnedDocument::Request(Box::new(request)),
    ] {
        let bytes = encode_owned(&document, limits()).unwrap();
        let decoded = decode_owned(&bytes, limits()).unwrap();
        assert_eq!(encode_owned(&decoded, limits()).unwrap(), bytes);
        let wire: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(wire["schema_version"], OWNED_INPUT_SCHEMA_VERSION);
        let mut previous = wire.clone();
        previous["schema_version"] = json!(2);
        assert!(matches!(
            decode_value(previous),
            Err(CodecError::UnsupportedVersion(2))
        ));
        assert!(!wire.as_object().unwrap().contains_key("source"));
    }
    let original = build().into_input();
    assert_eq!(
        BuildSpec::new(original.clone(), limits()).unwrap().input(),
        &original
    );
    let original = scenario().into_input();
    assert_eq!(
        ScenarioSpec::new(original.clone(), limits())
            .unwrap()
            .input(),
        &original
    );
}

#[test]
fn equal_definitions_preserve_distinct_occurrences_uses_payloads_and_loadouts() {
    let build = build();
    let input = build.input();
    assert_eq!(input.items.len(), 2);
    assert_eq!(input.items[0].template, input.items[1].template);
    assert_ne!(input.items[0].id, input.items[1].id);
    assert_eq!(input.equipment.len(), 2);
    assert_eq!(input.equipment[0].item, input.equipment[1].item);
    assert_ne!(input.equipment[0].id, input.equipment[1].id);
    let active_gems: Vec<_> = input
        .gems
        .iter()
        .filter(|gem| gem.definition.key().as_str() == "active-gem")
        .collect();
    assert_eq!(active_gems.len(), 2);
    assert_ne!(active_gems[0].id, active_gems[1].id);
    assert_eq!(input.skills.len(), 3);
    assert_eq!(input.payload_links[0].container, id::<SkillUseId>(8));
    assert_eq!(input.payload_links[0].payload, id::<SkillUseId>(9));
    assert_ne!(
        input.payload_links[0].container,
        input.payload_links[0].payload
    );
    assert_eq!(input.weapon_loadouts, vec![id::<WeaponLoadoutId>(1), id(2)]);
    assert_eq!(input.active_weapon_loadout, id::<WeaponLoadoutId>(2));
    assert!(
        !input
            .skills
            .iter()
            .find(|skill| skill.id == id::<SkillUseId>(19))
            .unwrap()
            .enabled
    );
    assert_eq!(
        input.equipment[0].scope,
        LoadoutScope::Selected {
            loadouts: vec![id(1), id(2)]
        }
    );
}

#[test]
fn unordered_record_permutations_encode_identically_but_query_row_order_is_preserved() {
    let first = build_input();
    let mut second = first.clone();
    second.items.reverse();
    second.gems.reverse();
    second.skills.reverse();
    second.equipment.reverse();
    second.allocations.reverse();
    second.weapon_loadouts.reverse();
    let first = OwnedDocument::Build(Box::new(BuildSpec::new(first, limits()).unwrap()));
    let second = OwnedDocument::Build(Box::new(BuildSpec::new(second, limits()).unwrap()));
    assert_eq!(
        encode_owned(&first, limits()).unwrap(),
        encode_owned(&second, limits()).unwrap()
    );
    let input = query_input(ProviderRoot::SupportAssignment(id(11)));
    let query = QuerySpec::new(input.clone(), limits()).unwrap();
    assert_eq!(query.input().requests, input.requests);
    let bytes = encode_owned(&OwnedDocument::Query(query), limits()).unwrap();
    let OwnedDocument::Query(decoded) = decode_owned(&bytes, limits()).unwrap() else {
        panic!("wrong document kind")
    };
    assert_eq!(decoded.into_input(), input);
}

#[test]
fn duplicate_ids_cross_domain_collisions_and_wrong_domain_references_are_rejected() {
    let mut duplicate = build_input();
    duplicate.gems.push(duplicate.gems[0].clone());
    fails(
        BuildSpec::new(duplicate, limits()),
        &["duplicate", "collision"],
    );
    let mut collision = build_input();
    collision.gems[0].id = id(3);
    fails(
        BuildSpec::new(collision, limits()),
        &["duplicate", "collision"],
    );
    let mut wrong_domain = build_input();
    wrong_domain.skills[0].source = AuthoredSkillSource::Gem(id(18));
    fails(
        BuildSpec::new(wrong_domain, limits()),
        &["reference", "missing", "gem", "domain"],
    );
    let mut dangling = build_input();
    dangling.supports[0].target = SkillTarget::Authored(id(88));
    fails(
        BuildSpec::new(dangling, limits()),
        &["reference", "missing", "skill", "dangling"],
    );
    let mut foreign = build_input();
    foreign.equipment[0].item = ItemRecordId::from_instance_id(
        InstanceId::from_parts(BuildLineage::from_bytes([0x99; 16]), 3).unwrap(),
    );
    fails(BuildSpec::new(foreign, limits()), &["lineage", "foreign"]);
    let mut watermark = build_input();
    watermark.allocator = InstanceAllocatorState::from_parts(lineage(), 10);
    fails(
        BuildSpec::new(watermark, limits()),
        &["watermark", "allocator", "issued"],
    );
}

#[test]
fn containment_cycles_and_empty_or_foreign_loadout_scopes_are_structural_errors() {
    let mut cycle = build_input();
    cycle.equipment[0].destination = EquipmentDestination::ItemSocket {
        container: id(4),
        slot: definition("socket-a"),
    };
    cycle.equipment[1].destination = EquipmentDestination::ItemSocket {
        container: id(5),
        slot: definition("socket-b"),
    };
    fails(BuildSpec::new(cycle, limits()), &["cycle", "containment"]);
    let mut empty = build_input();
    empty.skills[0].scope = LoadoutScope::Selected { loadouts: vec![] };
    fails(BuildSpec::new(empty, limits()), &["empty", "loadout"]);
    let mut missing = build_input();
    missing.skills[0].scope = LoadoutScope::Selected {
        loadouts: vec![id(88)],
    };
    fails(
        BuildSpec::new(missing, limits()),
        &["loadout", "reference", "missing"],
    );
}

#[test]
fn allocation_pools_and_provider_grants_are_preserved_without_invented_graph_legality() {
    let mut input = build_input();
    input.allocations.push(Allocation {
        id: id(20),
        node: definition("granted-node"),
        pool: definition("weapon-pool"),
        scope: LoadoutScope::Selected {
            loadouts: vec![id(2)],
        },
        access: AllocationAccess::Granted(provider(ProviderRoot::EquipmentUse(id(4)))),
        choices: vec![],
    });
    let build = BuildSpec::new(input, limits()).unwrap();
    let pools: Vec<_> = build
        .input()
        .allocations
        .iter()
        .map(|a| a.pool.key().as_str())
        .collect();
    assert_eq!(pools, ["ordinary-pool", "ascendancy-pool", "weapon-pool"]);
    assert_eq!(
        build.input().allocations[1].node.key().as_str(),
        "disconnected-special-node"
    );
    assert!(matches!(
        build.input().allocations[2].access,
        AllocationAccess::Granted(_)
    ));
    let bytes = encode_owned(&OwnedDocument::Build(Box::new(build.clone())), limits()).unwrap();
    let OwnedDocument::Build(decoded) = decode_owned(&bytes, limits()).unwrap() else {
        panic!("wrong document kind")
    };
    assert_eq!(decoded.input().allocations, build.input().allocations);
}

#[test]
fn support_owned_actors_and_missing_same_lineage_providers_keep_their_exact_selectors() {
    let live = request();
    let MetricTarget::Action(selection) = &live.queries().input().requests[0].target else {
        panic!("missing action target")
    };
    let ActorKey::Owned(actor) = &selection.action.actor else {
        panic!("support-owned actor became player")
    };
    assert_eq!(actor.provider.root, ProviderRoot::SupportAssignment(id(11)));
    assert_eq!(
        selection.action.provider.root,
        ProviderRoot::SupportAssignment(id(11))
    );
    assert_eq!(selection.part.key().as_str(), "primary-part");
    assert_eq!(selection.mode.key().as_str(), "damage-per-time");
    assert_eq!(selection.stat_set.key().as_str(), "normal-stat-set");
    let missing = queries(ProviderRoot::SupportAssignment(id(88)));
    let expected = missing.input().clone();
    let request = OwnedEvaluationRequest::new(build(), scenario(), missing, limits()).unwrap();
    assert_eq!(request.queries().input(), &expected);
    let mut disabled = build_input();
    disabled.supports[0].enabled = false;
    let disabled = OwnedEvaluationRequest::new(
        BuildSpec::new(disabled, limits()).unwrap(),
        scenario(),
        queries(ProviderRoot::SupportAssignment(id(11))),
        limits(),
    )
    .unwrap();
    assert!(!disabled.build().input().supports[0].enabled);
    let foreign = SupportAssignmentId::from_instance_id(
        InstanceId::from_parts(BuildLineage::from_bytes([0x99; 16]), 11).unwrap(),
    );
    fails(
        OwnedEvaluationRequest::new(
            build(),
            scenario(),
            queries(ProviderRoot::SupportAssignment(foreign)),
            limits(),
        ),
        &["lineage", "foreign"],
    );
    fails(
        OwnedEvaluationRequest::new(
            build(),
            scenario(),
            queries(ProviderRoot::SupportAssignment(id(101))),
            limits(),
        ),
        &["watermark", "allocator", "issued"],
    );
}

#[test]
fn query_ids_and_namespaces_cannot_implicitly_merge_or_retarget_requests() {
    let mut duplicate = query_input(ProviderRoot::SupportAssignment(id(11)));
    duplicate.requests[1].id = duplicate.requests[0].id.clone();
    fails(QuerySpec::new(duplicate, limits()), &["duplicate", "query"]);
    let other_namespace = GameVersionNamespace::new("other-game", "v2").unwrap();
    let other = ScenarioSpec::new(
        ScenarioInput {
            game_version: other_namespace.clone(),
            enemy: EnemySpec {
                encounter: EncounterDefId::parse(other_namespace, "encounter").unwrap(),
                level: 1,
            },
            assumptions: vec![],
            usage: vec![],
        },
        limits(),
    )
    .unwrap();
    fails(
        OwnedEvaluationRequest::new(
            build(),
            other,
            queries(ProviderRoot::SupportAssignment(id(11))),
            limits(),
        ),
        &["namespace", "version"],
    );
    let mut nested = build_input();
    nested.items[1].modifiers[0].rolls[0].value = ParameterValue::Quantity(
        FiniteQuantity::new(
            3.5,
            UnitDefId::parse(
                GameVersionNamespace::new("other-game", "v2").unwrap(),
                "resource-unit",
            )
            .unwrap(),
        )
        .unwrap(),
    );
    fails(BuildSpec::new(nested, limits()), &["namespace", "version"]);
}

#[test]
fn owned_decode_requires_explicit_options_and_rejects_unknown_or_duplicate_fields() {
    let valid = encoded_build();
    for path in [
        "/document/value/character/ascendancy",
        "/document/value/items/0/quality",
        "/document/value/items/0/item_level",
        "/document/value/gems/0/quality",
    ] {
        let mut missing = valid.clone();
        let (parent, field) = path.rsplit_once('/').unwrap();
        missing
            .pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove(field);
        fails(decode_value(missing), &["missing", "required"]);
    }
    for path in [
        "",
        "/document",
        "/document/value",
        "/document/value/character",
    ] {
        let mut extra = valid.clone();
        extra
            .pointer_mut(path)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unknown_semantic_field".into(), json!(true));
        fails(decode_value(extra), &["unknown", "field"]);
    }
    let document = serde_json::to_string(&valid["document"]).unwrap();
    let duplicate = format!(r#"{{"schema_version":3,"schema_version":3,"document":{document}}}"#);
    fails(decode_owned(duplicate.as_bytes(), limits()), &["duplicate"]);
    let encoded = serde_json::to_string(&valid).unwrap();
    let duplicate = encoded.replacen("\"level\":60", "\"level\":60,\"level\":60", 1);
    assert_ne!(encoded, duplicate);
    fails(decode_owned(duplicate.as_bytes(), limits()), &["duplicate"]);
    let mut unsupported = valid;
    unsupported["schema_version"] = json!(OWNED_INPUT_SCHEMA_VERSION + 1);
    fails(decode_value(unsupported), &["schema", "version"]);
}

#[test]
fn owned_limits_cover_bytes_collection_counts_total_entries_and_provider_paths() {
    let document = OwnedDocument::Build(Box::new(build()));
    let bytes = encode_owned(&document, limits()).unwrap();
    let mut exact = limits();
    exact.max_wire_bytes = bytes.len();
    assert!(decode_owned(&bytes, exact).is_ok());
    let mut too_small = exact;
    too_small.max_wire_bytes -= 1;
    fails(decode_owned(&bytes, too_small), &["byte", "limit", "size"]);
    fails(
        encode_owned(&document, too_small),
        &["byte", "limit", "size"],
    );
    let mut collection = limits();
    collection.max_collection_entries = 2;
    fails(
        BuildSpec::new(build_input(), collection),
        &["collection", "limit", "count"],
    );
    let mut total = limits();
    total.max_entries = 1;
    fails(
        BuildSpec::new(build_input(), total),
        &["entry", "entries", "limit", "count"],
    );
    let mut input = query_input(ProviderRoot::SupportAssignment(id(11)));
    let MetricTarget::Action(selection) = &mut input.requests[0].target else {
        panic!("missing action target")
    };
    selection.action.provider.grant_path.push(declared(
        SlotOwnerDefId::Gem(definition("support-gem")),
        "third-grant",
    ));
    let mut path = limits();
    path.max_provider_steps = 1;
    fails(
        QuerySpec::new(input.clone(), path),
        &["provider", "path", "limit"],
    );
    path.max_provider_steps = 2;
    assert!(QuerySpec::new(input, path).is_ok());
    let mut invalid = limits();
    invalid.max_provider_steps = 0;
    fails(
        BuildSpec::new(build_input(), invalid),
        &["limit", "positive", "zero"],
    );
}

#[test]
fn intrinsic_supply_properties_survive_shared_uses_and_same_occurrence_level_edits() {
    let original = build();
    let item = original
        .input()
        .items
        .iter()
        .find(|item| item.id == id::<ItemRecordId>(3))
        .unwrap();
    assert_eq!(item.parameters.len(), 1);
    assert_eq!(item.parameters[0].value, ParameterValue::Boolean(true));
    assert_eq!(
        item.parameters[0].slot.declaration,
        SlotOwnerDefId::ItemTemplate(item.template.clone())
    );
    assert_eq!(
        original
            .input()
            .equipment
            .iter()
            .filter(|usage| usage.item == item.id)
            .count(),
        2
    );
    let bytes = encode_owned(&OwnedDocument::Build(Box::new(original.clone())), limits()).unwrap();
    let OwnedDocument::Build(decoded) = decode_owned(&bytes, limits()).unwrap() else {
        panic!("wrong document kind")
    };
    let decoded_item = decoded
        .input()
        .items
        .iter()
        .find(|record| record.id == item.id)
        .unwrap();
    assert_eq!(decoded_item.parameters, item.parameters);

    let before = original
        .input()
        .gems
        .iter()
        .find(|gem| gem.id == id::<GemInstanceId>(6))
        .unwrap();
    assert_eq!(before.parameters.len(), 1);
    let mut edited = original.clone().into_input();
    edited.revision = edited.revision.checked_next().unwrap();
    let changed = edited
        .gems
        .iter_mut()
        .find(|gem| gem.id == before.id)
        .unwrap();
    changed.level += 1;
    let edited = BuildSpec::new(edited, limits()).unwrap();
    let after = edited
        .input()
        .gems
        .iter()
        .find(|gem| gem.id == before.id)
        .unwrap();
    assert_eq!(after.id, before.id);
    assert_eq!(after.definition, before.definition);
    assert_eq!(after.parameters, before.parameters);
    assert_eq!(after.level, before.level + 1);
    assert_eq!(
        original
            .input()
            .gems
            .iter()
            .find(|gem| gem.id == before.id)
            .unwrap()
            .level,
        before.level
    );
    assert_ne!(edited.input().revision, original.input().revision);
}

#[test]
fn owned_parameters_reject_duplicate_slots_wrong_declarations_and_foreign_namespaces() {
    let mut duplicate = build_input();
    let repeated = duplicate.items[1].parameters[0].clone();
    duplicate.items[1].parameters.push(repeated);
    fails(BuildSpec::new(duplicate, limits()), &["duplicate"]);
    let mut duplicate = build_input();
    let repeated = duplicate.gems[2].parameters[0].clone();
    duplicate.gems[2].parameters.push(repeated);
    fails(BuildSpec::new(duplicate, limits()), &["duplicate"]);

    for case in 0..4 {
        let mut input = build_input();
        match case {
            0 => {
                input.items[1].parameters[0].slot.declaration =
                    SlotOwnerDefId::ItemTemplate(definition("other-template"))
            }
            1 => {
                input.gems[2].parameters[0].slot.declaration =
                    SlotOwnerDefId::Gem(definition("other-gem"))
            }
            2 => {
                input.items[1].modifiers[0].rolls[0].slot.declaration =
                    SlotOwnerDefId::Modifier(definition("other-modifier"))
            }
            3 => {
                input.character.rewards[0].parameters[0].slot.declaration =
                    SlotOwnerDefId::Reward(definition("other-reward"))
            }
            _ => unreachable!(),
        }
        fails(
            BuildSpec::new(input, limits()),
            &["wrongdeclaration", "wrong declaration"],
        );
    }
    let mut wrong_kind = build_input();
    wrong_kind.items[1].parameters[0].slot.declaration =
        SlotOwnerDefId::Gem(definition("active-gem"));
    fails(
        BuildSpec::new(wrong_kind, limits()),
        &["wrongdeclaration", "wrong declaration"],
    );
    let mut usage = scenario_input();
    usage.usage[0].parameters[0].slot.declaration =
        SlotOwnerDefId::UsagePolicy(definition("other-policy"));
    fails(
        ScenarioSpec::new(usage, limits()),
        &["wrongdeclaration", "wrong declaration"],
    );
    let mut foreign = build_input();
    foreign.items[1].parameters[0].slot.slot = ParameterSlotDefId::parse(
        GameVersionNamespace::new("other-game", "v1").unwrap(),
        "intrinsic-state",
    )
    .unwrap();
    fails(BuildSpec::new(foreign, limits()), &["namespace"]);
}

#[test]
fn request_selectors_reject_present_wrong_domain_ids_but_keep_missing_historical_ids() {
    let wrong_domain = queries(ProviderRoot::SupportAssignment(id(3)));
    fails(
        OwnedEvaluationRequest::new(build(), scenario(), wrong_domain, limits()),
        &["reference", "domain"],
    );
    for (local, allowed) in [(3, false), (88, true)] {
        let mut input = scenario_input();
        input.assumptions[0].target = AssumptionTarget::Actor(
            action(ProviderRoot::SupportAssignment(id(local)))
                .action
                .actor,
        );
        let targeted = ScenarioSpec::new(input.clone(), limits()).unwrap();
        let result = OwnedEvaluationRequest::new(
            build(),
            targeted,
            queries(ProviderRoot::SupportAssignment(id(11))),
            limits(),
        );
        if allowed {
            assert_eq!(result.unwrap().scenario().input(), &input);
        } else {
            fails(result, &["reference", "domain"]);
        }
    }
}

fn modifier_query(roots: Vec<ProviderRoot>) -> QuerySpec {
    QuerySpec::new(
        QueryInput {
            game_version: namespace(),
            requests: roots
                .into_iter()
                .enumerate()
                .map(|(index, root)| MetricRequest {
                    id: QueryId::new(format!("modifier-{index}")).unwrap(),
                    metric: definition("owned-actor-resource"),
                    target: MetricTarget::Actor(ActorKey::Owned(Box::new(OwnedActorKey {
                        provider: ProviderKey {
                            root,
                            grant_path: vec![],
                        },
                        slot: declared(
                            SlotOwnerDefId::Modifier(definition("rolled-modifier")),
                            "granted-actor",
                        ),
                    }))),
                })
                .collect(),
        },
        limits(),
    )
    .unwrap()
}

#[test]
fn modifier_providers_distinguish_repeated_modifiers_and_shared_item_uses() {
    let mut input = build_input();
    let mut repeated = input.items[1].modifiers[0].clone();
    repeated.id = id(21);
    input.items[1].modifiers.push(repeated);
    let build = BuildSpec::new(input, limits()).unwrap();
    let providers = vec![
        ProviderRoot::ItemModifier {
            equipment_use: id(4),
            modifier: id(15),
        },
        ProviderRoot::ItemModifier {
            equipment_use: id(5),
            modifier: id(15),
        },
        ProviderRoot::ItemModifier {
            equipment_use: id(4),
            modifier: id(21),
        },
    ];
    assert_eq!(
        providers
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        3
    );
    let request = OwnedEvaluationRequest::new(
        build,
        scenario(),
        modifier_query(providers.clone()),
        limits(),
    )
    .unwrap();
    let bytes = encode_owned(&OwnedDocument::Request(Box::new(request)), limits()).unwrap();
    let OwnedDocument::Request(decoded) = decode_owned(&bytes, limits()).unwrap() else {
        panic!("wrong document kind")
    };
    let actual: Vec<_> = decoded
        .queries()
        .input()
        .requests
        .iter()
        .map(|row| {
            let MetricTarget::Actor(ActorKey::Owned(actor)) = &row.target else {
                panic!("modifier-owned actor changed")
            };
            actor.provider.root.clone()
        })
        .collect();
    assert_eq!(actual, providers);
    let shared_item = decoded
        .build()
        .input()
        .items
        .iter()
        .find(|item| item.id == id::<ItemRecordId>(3))
        .unwrap();
    assert_eq!(shared_item.modifiers.len(), 2);
    assert_eq!(
        shared_item.modifiers[0].definition,
        shared_item.modifiers[1].definition
    );
    assert_ne!(shared_item.modifiers[0].id, shared_item.modifiers[1].id);
}

#[test]
fn modifier_provider_requires_the_modifier_to_belong_to_its_receiving_use() {
    let mut input = build_input();
    let mut elsewhere = input.items[1].modifiers[0].clone();
    elsewhere.id = id(22);
    input.items[0].modifiers.push(elsewhere);
    let wrong_parent = ProviderRoot::ItemModifier {
        equipment_use: id(4),
        modifier: id(22),
    };
    let valid = BuildSpec::new(input.clone(), limits()).unwrap();
    let mut authored = input;
    authored.choices[0].owner = ChoiceOwner::Provider(ProviderKey {
        root: wrong_parent.clone(),
        grant_path: vec![],
    });
    fails(
        BuildSpec::new(authored, limits()),
        &["wrongproviderowner", "wrong provider owner"],
    );
    fails(
        OwnedEvaluationRequest::new(
            valid.clone(),
            scenario(),
            modifier_query(vec![wrong_parent]),
            limits(),
        ),
        &["wrongproviderowner", "wrong provider owner"],
    );
    for wrong_domain in [
        ProviderRoot::ItemModifier {
            equipment_use: id(3),
            modifier: id(15),
        },
        ProviderRoot::ItemModifier {
            equipment_use: id(4),
            modifier: id(6),
        },
    ] {
        fails(
            OwnedEvaluationRequest::new(
                valid.clone(),
                scenario(),
                modifier_query(vec![wrong_domain]),
                limits(),
            ),
            &["reference", "domain"],
        );
    }
    let historical = ProviderRoot::ItemModifier {
        equipment_use: id(88),
        modifier: id(15),
    };
    assert!(
        OwnedEvaluationRequest::new(
            valid,
            scenario(),
            modifier_query(vec![historical]),
            limits()
        )
        .is_ok()
    );
}

#[test]
fn item_level_null_is_explicit_and_roundtrips_in_build_and_request_envelopes() {
    for level in [None, Some(0), Some(u16::MAX)] {
        let mut raw = build_input();
        raw.items[0].item_level = level;
        let build = BuildSpec::new(raw, limits()).unwrap();
        let request = OwnedEvaluationRequest::new(
            build.clone(),
            scenario(),
            queries(ProviderRoot::SupportAssignment(id(11))),
            limits(),
        )
        .unwrap();
        for document in [
            OwnedDocument::Build(Box::new(build)),
            OwnedDocument::Request(Box::new(request)),
        ] {
            let bytes = encode_owned(&document, limits()).unwrap();
            assert_eq!(decode_owned(&bytes, limits()).unwrap(), document);
        }
    }
    let mut wire = encoded_build();
    wire["document"]["value"]["items"][0]["item_level"] = Value::Null;
    let text = serde_json::to_string(&wire).unwrap();
    let duplicate = text.replacen(
        "\"item_level\":null",
        "\"item_level\":null,\"item_level\":null",
        1,
    );
    assert_ne!(text, duplicate);
    fails(decode_owned(duplicate.as_bytes(), limits()), &["duplicate"]);
    for invalid in [json!(-1), json!(65536), json!(1.5), json!("81")] {
        let mut malformed = wire.clone();
        malformed["document"]["value"]["items"][0]["item_level"] = invalid;
        assert!(matches!(decode_value(malformed), Err(CodecError::Json(_))));
    }
}
