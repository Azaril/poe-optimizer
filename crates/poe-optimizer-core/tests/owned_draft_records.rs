//! Raw DTO/conversion laws; structurally valid draft authority is tested elsewhere.
use poe_optimizer_core::{build_identity::*, owned_build::*, owned_definitions::*, owned_draft::*};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::json;
use std::fmt::Debug;

fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("draft-game", "v1").unwrap()
}
fn id<T: BuildInstanceId>(local: u64) -> T {
    T::from_instance_id(
        InstanceId::from_parts(BuildLineage::from_bytes([0x47; 16]), local).unwrap(),
    )
}
fn def<K: DefinitionDomain>(key: &str) -> DefId<K> {
    DefId::parse(ns(), key).unwrap()
}
fn slot<K: DefinitionDomain>(key: &str) -> DeclaredSlot<DefId<K>> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Gem(def("gem")),
        slot: def(key),
    }
}
fn pending<T>(local: u64, candidates: Vec<T>) -> PendingValue<T> {
    PendingValue {
        id: id(local),
        code: OwnedDefinitionKey::new("unmapped-input").unwrap(),
        candidates,
    }
}
fn provider() -> ProviderKey {
    ProviderKey {
        root: ProviderRoot::SupportAssignment(id(5)),
        grant_path: vec![slot("grant-b"), slot("grant-a")],
    }
}
fn actor() -> ActorKey {
    ActorKey::Owned(Box::new(OwnedActorKey {
        provider: provider(),
        slot: slot("actor"),
    }))
}
fn skill() -> SkillTarget {
    SkillTarget::Generated(Box::new(GeneratedSkillKey {
        provider: provider(),
        slot: slot("skill-grant"),
    }))
}
fn action() -> ActionSelection {
    ActionSelection {
        action: ActionKey {
            actor: actor(),
            provider: provider(),
            output: slot("output"),
        },
        part: def("part"),
        mode: def("mode"),
        stat_set: def("stat-set"),
    }
}
fn parameter() -> ParameterAssignment {
    ParameterAssignment {
        slot: slot("parameter"),
        value: ParameterValue::Integer(BoundedInteger::new(7).unwrap()),
    }
}
fn choice() -> ChoiceSelection {
    ChoiceSelection {
        slot: slot("choice"),
        value: ParameterValue::Boolean(true),
    }
}
fn quality() -> QualitySelection {
    QualitySelection {
        kind: def("quality"),
        amount: FiniteQuantity::new(12.0, def("quality-unit")).unwrap(),
    }
}
fn item() -> ItemRecord {
    ItemRecord {
        id: id(1),
        template: def("template"),
        parameters: vec![parameter()],
        item_level: Some(72),
        quality: Some(quality()),
        modifiers: vec![RolledModifier {
            id: id(2),
            definition: def("modifier"),
            rolls: vec![parameter()],
        }],
    }
}
fn roundtrip<R, D>(resolved: R)
where
    R: Clone + Debug + PartialEq,
    D: From<R> + ResolveDraft<Resolved = R> + Serialize + DeserializeOwned + PartialEq + Debug,
{
    let draft = D::from(resolved.clone());
    assert_eq!(draft.to_resolved(), Some(resolved));
    let decoded: D = serde_json::from_slice(&serde_json::to_vec(&draft).unwrap()).unwrap();
    assert_eq!(decoded, draft);
    assert_eq!(decoded.to_resolved(), draft.to_resolved());
}

#[test]
fn all_record_families_convert_and_roundtrip_without_source_or_schema_lookup() {
    roundtrip::<_, QualityDraft>(quality());
    roundtrip::<_, ParameterDraft>(parameter());
    roundtrip::<_, ChoiceSelectionDraft>(choice());
    roundtrip::<_, ModifierDraft>(item().modifiers.remove(0));
    roundtrip::<_, ItemDraft>(item());
    roundtrip::<_, GemDraft>(GemInstance {
        id: id(3),
        definition: def("gem"),
        parameters: vec![parameter()],
        level: 12,
        quality: None,
    });
    let reward = RewardSelection {
        id: id(4),
        definition: def("reward"),
        parameters: vec![parameter()],
    };
    roundtrip::<_, RewardDraft>(reward.clone());
    roundtrip::<_, CharacterDraft>(CharacterSpec {
        class: def("class"),
        ascendancy: None,
        level: 72,
        rewards: vec![reward],
    });
    roundtrip::<_, EquipmentDraft>(EquipmentUse {
        id: id(6),
        item: id(1),
        destination: EquipmentDestination::ItemSocket {
            container: id(7),
            slot: def("socket"),
        },
        scope: LoadoutScope::Selected {
            loadouts: vec![id(12), id(11)],
        },
    });
    roundtrip::<_, AllocationDraft>(Allocation {
        id: id(8),
        node: def("node"),
        pool: def("pool"),
        scope: LoadoutScope::Shared,
        access: AllocationAccess::Granted(provider()),
        choices: vec![choice()],
    });
    roundtrip::<_, SkillDraft>(SkillUse {
        id: id(9),
        source: AuthoredSkillSource::Gem(id(3)),
        enabled: false,
        scope: LoadoutScope::Shared,
    });
    roundtrip::<_, SupportDraft>(SupportAssignment {
        id: id(5),
        support: id(3),
        target: skill(),
        enabled: true,
    });
    roundtrip::<_, PayloadDraft>(PayloadLink {
        id: id(10),
        container: id(9),
        payload: id(13),
        role: def("role"),
    });
    roundtrip::<_, ChoiceDraft>(MechanicChoice {
        owner: ChoiceOwner::Action(Box::new(action())),
        choice: choice(),
    });
    roundtrip::<_, ProviderKeyDraft>(provider());
    roundtrip::<_, GeneratedSkillKeyDraft>(GeneratedSkillKey {
        provider: provider(),
        slot: slot("skill-grant"),
    });
    roundtrip::<_, OwnedActorKeyDraft>(OwnedActorKey {
        provider: provider(),
        slot: slot("actor"),
    });
    roundtrip::<_, ActionKeyDraft>(action().action);
    roundtrip::<_, ActionSelectionDraft>(action());
    let enemy = EnemySpec {
        encounter: def("encounter"),
        level: 82,
    };
    roundtrip::<_, EnemyDraft>(enemy.clone());
    let assumption = ExternalAssumption {
        input: def("external"),
        target: AssumptionTarget::Actor(actor()),
        value: ParameterValue::Boolean(true),
    };
    roundtrip::<_, ExternalAssumptionDraft>(assumption.clone());
    let usage = UsagePolicySelection {
        policy: def("usage"),
        target: UsageTarget::Action(Box::new(action())),
        parameters: vec![parameter()],
    };
    roundtrip::<_, UsagePolicyDraft>(usage.clone());
    roundtrip::<_, ScenarioDraft>(ScenarioInput {
        game_version: ns(),
        enemy,
        assumptions: vec![assumption],
        usage: vec![usage],
    });
    let query = MetricRequest {
        id: QueryId::new("row-b").unwrap(),
        metric: def("metric"),
        target: MetricTarget::Action(Box::new(action())),
    };
    roundtrip::<_, MetricRequestDraft>(query.clone());
    roundtrip::<_, QueryDraft>(QueryInput {
        game_version: ns(),
        requests: vec![query],
    });
}

#[test]
fn every_composite_variant_has_a_lossless_complete_conversion() {
    for value in [
        EquipmentDestination::CharacterSlot(def("slot")),
        EquipmentDestination::ItemSocket {
            container: id(7),
            slot: def("socket"),
        },
        EquipmentDestination::PassiveSocket {
            allocation: id(8),
            slot: def("socket"),
        },
    ] {
        roundtrip::<_, DraftEquipmentDestination>(value);
    }
    for value in [
        AuthoredSkillSource::Gem(id(3)),
        AuthoredSkillSource::Direct(def("skill")),
    ] {
        roundtrip::<_, DraftAuthoredSkillSource>(value);
    }
    for value in [
        AllocationAccess::Ordinary,
        AllocationAccess::Granted(provider()),
    ] {
        roundtrip::<_, DraftAllocationAccess>(value);
    }
    for value in [
        ProviderRoot::Character,
        ProviderRoot::ItemModifier {
            equipment_use: id(6),
            modifier: id(2),
        },
        ProviderRoot::SkillUse(id(9)),
        ProviderRoot::SupportAssignment(id(5)),
        ProviderRoot::EquipmentUse(id(6)),
        ProviderRoot::Allocation(id(8)),
        ProviderRoot::Reward(id(4)),
    ] {
        roundtrip::<_, DraftProviderRoot>(value);
    }
    for value in [SkillTarget::Authored(id(9)), skill()] {
        roundtrip::<_, DraftSkillTarget>(value);
    }
    for value in [ActorKey::Player, actor()] {
        roundtrip::<_, DraftActorKey>(value);
    }
    for value in [
        ChoiceOwner::Character,
        ChoiceOwner::EquipmentUse(id(6)),
        ChoiceOwner::Allocation(id(8)),
        ChoiceOwner::Skill(skill()),
        ChoiceOwner::Action(Box::new(action())),
        ChoiceOwner::Provider(provider()),
    ] {
        roundtrip::<_, DraftChoiceOwner>(value);
    }
    for value in [
        AssumptionTarget::Environment,
        AssumptionTarget::Enemy,
        AssumptionTarget::Actor(actor()),
        AssumptionTarget::Skill(skill()),
    ] {
        roundtrip::<_, DraftAssumptionTarget>(value);
    }
    for value in [
        UsageTarget::Actor(actor()),
        UsageTarget::Action(Box::new(action())),
        UsageTarget::Skill(skill()),
    ] {
        roundtrip::<_, DraftUsageTarget>(value);
    }
    for value in [
        MetricTarget::Actor(actor()),
        MetricTarget::Action(Box::new(action())),
    ] {
        roundtrip::<_, DraftMetricTarget>(value);
    }
}

#[test]
fn pending_fields_keep_known_record_and_selector_siblings_without_selecting_candidates() {
    let original = item();
    let mut draft = ItemDraft::from(original.clone());
    draft.template = DraftField::Pending(pending(101, vec![def("alternative-template")]));
    assert!(draft.to_resolved().is_none());
    assert_eq!(draft.item_level.to_resolved(), Some(original.item_level));
    assert_eq!(draft.modifiers.to_resolved(), Some(original.modifiers));
    assert_eq!(draft.quality.to_resolved(), Some(original.quality));
    let destination = DraftEquipmentDestination::ItemSocket {
        container: id::<ItemSlotUseId>(6).into(),
        slot: DraftField::Pending(pending(102, vec![def("possible-socket")])),
    };
    assert!(destination.to_resolved().is_none());
    let DraftEquipmentDestination::ItemSocket { container, .. } = &destination else {
        panic!("shape changed")
    };
    assert_eq!(container.to_resolved(), Some(id(6)));
    let mut action = ActionSelectionDraft::from(action());
    action.action.output = DraftField::Pending(pending(103, vec![]));
    assert!(action.to_resolved().is_none());
    assert_eq!(action.action.provider.to_resolved(), Some(provider()));
    assert_eq!(
        action.part.to_resolved(),
        Some(def::<ActionPartDefinition>("part"))
    );
    let pending = DraftChoiceOwner::Pending(pending(104, vec![ChoiceOwner::Character]));
    assert!(pending.to_resolved().is_none());
    let decoded: ItemDraft = serde_json::from_slice(&serde_json::to_vec(&draft).unwrap()).unwrap();
    assert_eq!(decoded, draft);
}

#[test]
fn collection_completion_and_each_pending_member_block_resolution_independently() {
    let first = parameter();
    let mut second = parameter();
    second.slot.slot = def("second-parameter");
    let mut list: DraftList<ParameterDraft> = vec![first.clone(), second.clone()].into();
    assert_eq!(
        list.to_resolved(),
        Some(vec![first.clone(), second.clone()])
    );
    list.completion = DraftListCompletion::Pending {
        id: id(110),
        code: OwnedDefinitionKey::new("unconverted-members").unwrap(),
    };
    assert!(list.to_resolved().is_none());
    assert_eq!(list.members[0].to_resolved(), Some(first));
    list.completion = DraftListCompletion::Complete;
    list.members[1].value = DraftField::Pending(pending(111, vec![second.value]));
    assert!(list.to_resolved().is_none());
    let empty: DraftList<ParameterDraft> = Vec::<ParameterAssignment>::new().into();
    assert_eq!(empty.to_resolved(), Some(vec![]));
}

#[test]
fn generic_and_quality_known_null_are_explicit_and_not_pending_or_omitted() {
    let known: DraftField<Option<AscendancyDefId>> = None.into();
    assert_eq!(
        serde_json::to_value(&known).unwrap(),
        json!({"kind":"known","value":null})
    );
    assert_eq!(known.to_resolved(), Some(None));
    assert!(
        serde_json::from_value::<DraftField<Option<AscendancyDefId>>>(json!({"kind":"known"}))
            .is_err()
    );
    let quality = DraftQuality::from(None);
    assert_eq!(
        serde_json::to_value(&quality).unwrap(),
        json!({"kind":"known","value":null})
    );
    assert_eq!(quality.to_resolved(), Some(None));
    assert!(serde_json::from_value::<DraftQuality>(json!({"kind":"known"})).is_err());
    let unknown = DraftQuality::Pending(pending(112, vec![None, Some(self::quality())]));
    assert!(unknown.to_resolved().is_none());
    let mut partial = QualityDraft::from(self::quality());
    partial.kind = DraftField::Pending(pending(113, vec![]));
    assert!(
        DraftQuality::Known {
            value: Some(partial)
        }
        .to_resolved()
        .is_none()
    );
    let mut wire = serde_json::to_value(ItemDraft::from(item())).unwrap();
    wire.as_object_mut().unwrap().remove("quality");
    assert!(serde_json::from_value::<ItemDraft>(wire).is_err());
}

#[test]
fn ordered_queries_and_grant_paths_survive_and_absent_targets_do_not_become_a_ready_subset() {
    let requests = vec![
        MetricRequest {
            id: QueryId::new("row-z").unwrap(),
            metric: def("first-metric"),
            target: MetricTarget::Actor(actor()),
        },
        MetricRequest {
            id: QueryId::new("row-a").unwrap(),
            metric: def("second-metric"),
            target: MetricTarget::Actor(ActorKey::Player),
        },
    ];
    let query = QueryInput {
        game_version: ns(),
        requests,
    };
    let mut draft = QueryDraft::from(query.clone());
    assert_eq!(draft.to_resolved(), Some(query.clone()));
    assert_eq!(
        ProviderKeyDraft::from(provider())
            .to_resolved()
            .unwrap()
            .grant_path,
        vec![slot("grant-b"), slot("grant-a")]
    );
    draft.requests.members[0].target = DraftMetricTarget::Pending(pending(114, vec![]));
    assert!(draft.to_resolved().is_none());
    assert_eq!(draft.requests.members.len(), 2);
    assert_eq!(draft.requests.members[0].id, query.requests[0].id);
    assert_eq!(
        draft.requests.members[1].to_resolved(),
        Some(query.requests[1].clone())
    );
}

#[test]
fn strict_wire_rejects_unknown_fields_duplicate_known_value_and_wrong_candidate_kind() {
    let known = serde_json::to_string(&DraftField::from(true)).unwrap();
    let duplicate = known.replace("\"value\":true", "\"value\":true,\"value\":false");
    assert_ne!(duplicate, known);
    assert!(serde_json::from_str::<DraftField<bool>>(&duplicate).is_err());
    assert!(
        serde_json::from_value::<DraftField<bool>>(
            json!({"kind":"known","value":true,"source":"unexpected"})
        )
        .is_err()
    );
    assert!(serde_json::from_value::<DraftList<ParameterDraft>>(json!({"members":[]})).is_err());
    let mut value = serde_json::to_value(DraftField::<GemDefId>::Pending(pending(
        115,
        vec![def("gem")],
    )))
    .unwrap();
    value["candidates"][0]["kind"] = json!("item_template");
    assert!(serde_json::from_value::<DraftField<GemDefId>>(value).is_err());
    let mut item = serde_json::to_value(ItemDraft::from(item())).unwrap();
    item["source_xml"] = json!("<Item/>");
    assert!(serde_json::from_value::<ItemDraft>(item).is_err());
}

#[test]
fn complete_conversion_does_not_claim_structural_or_definition_validity() {
    // Duplicate assignments remain visible to the owning validator; conversion
    // cannot silently deduplicate or manufacture valid project authority.
    let parameters = vec![parameter(), parameter()];
    let draft: DraftList<ParameterDraft> = parameters.clone().into();
    assert_eq!(draft.to_resolved(), Some(parameters));
    let unresolved = DraftField::<ItemTemplateDefId>::Pending(pending(116, vec![]));
    assert!(unresolved.to_resolved().is_none());
}

#[test]
fn item_level_known_null_is_resolved_while_pending_candidates_are_not_selected() {
    for level in [None, Some(0), Some(u16::MAX)] {
        let mut original = item();
        original.item_level = level;
        let draft = ItemDraft::from(original.clone());
        assert_eq!(draft.item_level.to_resolved(), Some(level));
        assert_eq!(draft.to_resolved(), Some(original));
        let decoded: ItemDraft =
            serde_json::from_slice(&serde_json::to_vec(&draft).unwrap()).unwrap();
        assert_eq!(decoded, draft);
    }
    let mut draft = ItemDraft::from(item());
    draft.item_level = DraftField::Pending(pending(120, vec![None, Some(81)]));
    assert!(draft.to_resolved().is_none());
    assert!(draft.item_level.to_resolved().is_none());
    let decoded: ItemDraft = serde_json::from_slice(&serde_json::to_vec(&draft).unwrap()).unwrap();
    assert_eq!(decoded, draft);
    draft.item_level = None.into();
    let wire = serde_json::to_value(&draft).unwrap();
    assert_eq!(wire["item_level"], json!({"kind":"known","value":null}));
    let mut missing_field = wire.clone();
    missing_field.as_object_mut().unwrap().remove("item_level");
    assert!(serde_json::from_value::<ItemDraft>(missing_field).is_err());
    let mut missing_value = wire;
    missing_value["item_level"]
        .as_object_mut()
        .unwrap()
        .remove("value");
    assert!(serde_json::from_value::<ItemDraft>(missing_value).is_err());
    assert!(
        serde_json::from_str::<DraftField<Option<u16>>>(
            r#"{"kind":"known","value":null,"value":81}"#
        )
        .is_err()
    );
}
