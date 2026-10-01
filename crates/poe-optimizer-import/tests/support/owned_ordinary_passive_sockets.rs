//! Placement proves source relationships independently of unimplemented mechanics.
use super::*;
use poe_optimizer_import::{owned_item_lines::*, owned_item_source::*};

struct PlacementFixture {
    core: Fixture,
    items: OwnedItemLinePolicy,
    source: ItemSourceLayoutPolicy,
}

fn partial<T>(members: Vec<T>, subject: SchemaSubject) -> DeclaredSet<T> {
    DeclaredSet {
        members,
        closure: SchemaClosure::Partial {
            gaps: vec![SchemaGap {
                subject,
                facet: SchemaFacet::InputSchema,
                code: key("review-incomplete"),
            }],
        },
    }
}

fn fixture() -> PlacementFixture {
    let mut f = super::fixture();
    let mut schema = f.schema.input().clone();
    let template = f
        .registry
        .allocate_definition::<ItemTemplateDefinition>()
        .unwrap();
    let socket = f
        .registry
        .allocate_definition::<SocketSlotDefinition>()
        .unwrap();
    let owner = f.ordinary.clone();
    let mut tree_content = f.tree.input().content.clone();
    let root = match &tree_content
        .tokens
        .iter()
        .find(|row| row.token == "1")
        .unwrap()
        .role
    {
        TreeTokenRole::ImplicitRoot { node } => node.clone(),
        _ => panic!(),
    };
    let pool = match &tree_content
        .tokens
        .iter()
        .find(|row| row.token == "30")
        .unwrap()
        .role
    {
        TreeTokenRole::Allocation { pool, .. } => pool.clone(),
        _ => panic!(),
    };
    tree_content.syntax.ignored_spec_children = vec!["URL".into(), "Sockets".into()];
    tree_content.access = Some(AllocationAccessPolicy::PobIndependentSavedPathsV1 {
        pools: vec![AllocationAccessPool {
            pool,
            root: AllocationRootKind::Class,
        }],
        nodes: vec![owner.clone()],
    });
    for descriptor in &mut schema.definitions {
        if let DefinitionDescriptor::PassiveNode(entry) = descriptor
            && entry.id == owner
            && let SchemaState::Known(node) = &mut entry.schema
        {
            node.declarations.sockets = partial(vec![socket.clone()], subject(&owner));
            node.adjacent = DeclaredSet::complete(vec![root.clone()]);
        }
    }
    schema
        .definitions
        .push(DefinitionDescriptor::SocketSlot(known(
            socket.clone(),
            SocketSlotSchema {
                owner: SlotOwnerDefId::PassiveNode(owner.clone()),
                kind: SocketKind::Passive,
                scope: ScopePolicy::Shared,
            },
        )));
    let mut declared = declarations();
    declared.parameters = partial(vec![], subject(&template));
    schema
        .definitions
        .push(DefinitionDescriptor::ItemTemplate(known(
            template.clone(),
            ItemTemplateSchema {
                item_level: IntegerRange {
                    minimum: BoundedInteger::new(1).unwrap(),
                    maximum: BoundedInteger::new(100).unwrap(),
                },
                equipment_slots: DeclaredSet::complete(vec![]),
                socket_destinations: partial(vec![socket.clone()], subject(&template)),
                modifiers: partial(vec![], subject(&template)),
                quality: QualityUseSchema {
                    presence: QualityPresence::Forbidden,
                    allowed_kinds: DeclaredSet::complete(vec![]),
                },
                declarations: declared,
            },
        )));
    f.schema = OwnedDefinitionSchemaPackage::new(schema, Default::default()).unwrap();
    let mut mapping = f.mapping.input().clone();
    mapping.registry = f.registry.identity().unwrap();
    mapping.definitions = f.schema.identity().clone();
    f.mapping =
        OwnedMappingIndex::new(mapping, &f.registry, &f.schema, Default::default()).unwrap();
    let mut roles = f.roles.input().clone();
    roles.definitions = f.schema.identity().clone();
    roles.mapping = *f.mapping.identity();
    roles.compilation.staged_registry = f.registry.identity().unwrap();
    f.roles = OwnedSkillRoleIndex::new(roles, &f.mapping, &f.schema, Default::default()).unwrap();
    let mut rewards = f.rewards.input().clone();
    rewards.definitions = f.schema.identity().clone();
    rewards.mapping = *f.mapping.identity();
    f.rewards = OwnedRewardPolicy::new(rewards, &f.mapping, &f.schema, Default::default()).unwrap();
    let items = OwnedItemLinePolicy::new(
        ItemLinePolicyInput {
            schema_version: OWNED_ITEM_LINE_POLICY_VERSION,
            namespace: ns(),
            version: key("jewel-base"),
            definitions: f.schema.identity().clone(),
            whitespace: WhitespacePolicy::TrimAscii,
            rules: vec![ItemLineRule {
                id: key("reviewed-jewel"),
                pattern: vec![ItemPatternPart::Literal("Injected Jewel".into())],
                captures: vec![],
                emissions: vec![ItemEmission::Template {
                    definition: template.clone(),
                }],
            }],
        },
        &f.schema,
        Default::default(),
    )
    .unwrap();
    let mut source = empty_owned_items::empty_source_for_items(&f.schema, &items)
        .input()
        .clone();
    source.rule_layouts = vec![ItemRuleSourceLayout {
        rule: key("reviewed-jewel"),
        role: ItemRuleSourceRole::Header,
    }];
    let source =
        ItemSourceLayoutPolicy::new(source, &items, &f.schema, Default::default()).unwrap();
    f.base.equipment_membership = Some(EquipmentMembershipPolicy::PobOrdinaryItemSetsV1 {
        definitions: f.schema.identity().clone(),
        templates: vec![],
        source_base_names: vec!["Injected Jewel".into(), "Other Source Base".into()],
        loader_jewel_fallback_titles: vec!["Legacy Jewel Title".into()],
    });
    f.tree = OwnedTreeNormalizationPolicy::bind_new(
        tree_content,
        &f.registry,
        &f.schema,
        &f.mapping,
        &f.base,
        Default::default(),
    )
    .unwrap();
    f.base.passive_socket_membership = Some(
        PassiveSocketMembershipPolicy::PobOrdinarySharedSpecSocketsV2 {
            definitions: f.schema.identity().clone(),
            mapping: *f.mapping.identity(),
            mapping_source: *f.mapping.source_identity(),
            item_lines: *items.identity(),
            item_source: *source.identity(),
            equipment: equipment_membership_identity(
                f.base.equipment_membership.as_ref().unwrap(),
                Default::default(),
            )
            .unwrap(),
            tree_content: tree_content_identity(&f.tree.input().content, Default::default())
                .unwrap(),
            source_bases: vec![OrdinaryPassiveJewelBase {
                template: template.clone(),
                base_name: "Injected Jewel".into(),
                rule: key("reviewed-jewel"),
            }],
            bindings: vec![OrdinaryPassiveSocketBinding {
                node_token: "30".into(),
                node: owner,
                slot: socket,
                templates: vec![template],
            }],
        },
    );
    PlacementFixture {
        core: f,
        items,
        source,
    }
}

fn normalize(
    f: &PlacementFixture,
    xml: &str,
    policy: &NormalizationPolicy,
    limits: NormalizationLimits,
) -> std::result::Result<NormalizedImport, NormalizationError> {
    let tree = OwnedTreeNormalizationPolicy::bind_new(
        f.core.tree.input().content.clone(),
        &f.core.registry,
        &f.core.schema,
        &f.core.mapping,
        policy,
        Default::default(),
    )?;
    let source = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([92; 16]),
        Default::default(),
    )
    .unwrap();
    let evidence = SourceProjectEvidence::collect(&source, Default::default()).unwrap();
    normalize_fresh(
        &evidence,
        *source.allocator_state(),
        NormalizationArtifacts {
            registry: &f.core.registry,
            definitions: &f.core.schema,
            mappings: &f.core.mapping,
            roles: &f.core.roles,
            rewards: &f.core.rewards,
            items: &f.items,
            item_source: &f.source,
            tree: Some(&tree),
        },
        policy,
        &[],
        limits,
    )
}

const RAW: &str = "Rarity: RARE\nUnconverted Title\nInjected Jewel\nUnique ID: abcdef\nItem Level: 60\nLevelReq: 0\nImplicits: 0\nAn unimplemented modifier";
const SOCKET: &str = r#"<Socket nodeId="30" itemId="1"/>"#;
fn xml(specs: &str, raw: &str) -> String {
    format!(
        r#"<PathOfBuilding2><Build level="60"/><Tree activeSpec="1">{specs}</Tree><Items><Item id="1">{raw}</Item><Item id="2">{raw}</Item><ItemSet id="1"/></Items><Skills/><Config/></PathOfBuilding2>"#
    )
}
fn occupied(nodes: &str, socket: &str, extra: &str) -> String {
    spec(nodes, &format!("<Sockets>{socket}</Sockets>{extra}"))
}
fn placements(result: &NormalizedImport) -> usize {
    result
        .draft()
        .input()
        .equipment
        .members
        .iter()
        .filter(|usage| {
            matches!(
                usage.destination,
                DraftEquipmentDestination::PassiveSocket { .. }
            )
        })
        .count()
}

#[test]
fn same_spec_join_preserves_items_ids_and_partial_mechanics() {
    let f = fixture();
    let xml = xml(
        &format!(
            "{}{}",
            occupied("1,2,30", SOCKET, ""),
            occupied(
                "1,2,30",
                &SOCKET.replace("itemId=\"1\"", "itemId=\"2\""),
                ""
            )
        ),
        RAW,
    );
    let mut old = f.core.base.clone();
    old.passive_socket_membership =
        Some(PassiveSocketMembershipPolicy::PobExplicitEmptySpecSocketsV1 {});
    let before = normalize(&f, &xml, &old, Default::default()).unwrap();
    let after = normalize(&f, &xml, &f.core.base, Default::default()).unwrap();
    assert_eq!(placements(&before), 0);
    assert_eq!(placements(&after), 2);
    assert_eq!(before.allocator_after(), after.allocator_after());
    assert_eq!(before.draft().input().items, after.draft().input().items);
    assert_eq!(
        before.draft().input().allocations,
        after.draft().input().allocations
    );
    assert_eq!(
        serde_json::to_value(&before.sidecar().item_texts).unwrap(),
        serde_json::to_value(&after.sidecar().item_texts).unwrap()
    );
    let after = after.draft().input();
    assert_ne!(
        after.equipment.members[0].item,
        after.equipment.members[1].item
    );
    let mut joined = std::collections::BTreeSet::new();
    for preset in &after.allocation_presets.members {
        assert!(matches!(
            preset.equipment.completion,
            DraftListCompletion::Complete
        ));
        let usage = after
            .equipment
            .members
            .iter()
            .find(|u| u.id == preset.equipment.members[0])
            .unwrap();
        let DraftEquipmentDestination::PassiveSocket {
            allocation,
            slot: _,
        } = &usage.destination
        else {
            panic!()
        };
        let allocation = allocation.to_resolved().unwrap();
        assert!(preset.allocations.members.contains(&allocation));
        assert!(joined.insert(allocation));
        assert_eq!(usage.scope.to_resolved(), Some(LoadoutScope::Shared));
    }
    for item in &after.items.members {
        assert!(matches!(
            item.parameters.completion,
            DraftListCompletion::Pending { .. }
        ));
    }
}

#[test]
fn unresolved_modifier_text_and_reviewed_flags_do_not_claim_mechanical_completeness() {
    let f = fixture();
    for raw in [
        RAW.to_owned(),
        RAW.replace("RARE", "UNIQUE"),
        format!("{RAW}\nCorrupted"),
        RAW.replace(
            "An unimplemented modifier",
            "{crafted}An unimplemented radius effect",
        ),
        RAW.replace("Implicits: 0", "Radius: Small\nImplicits: 0"),
    ] {
        let result = normalize(
            &f,
            &xml(&occupied("1,2,30", SOCKET, ""), &raw),
            &f.core.base,
            Default::default(),
        )
        .unwrap();
        assert_eq!(placements(&result), 1, "{raw}");
        assert!(matches!(
            result.draft().input().items.members[0].modifiers.completion,
            DraftListCompletion::Pending { .. }
        ));
    }
}

#[test]
fn unsupported_source_shapes_stay_pending_without_changing_other_specs() {
    let f = fixture();
    let valid = occupied("1,2,30", SOCKET, "");
    for bad in [
        occupied("1,2", SOCKET, ""),
        occupied("1,2,30", SOCKET, r#"<WeaponSet1 nodes="30"/>"#),
        occupied("1,2,30", &format!("{SOCKET}{SOCKET}"), ""),
        occupied(
            "1,2,30",
            &SOCKET.replace("itemId=\"1\"", "itemId=\"9\""),
            "",
        ),
        occupied(
            "1,2,30",
            &SOCKET.replace("nodeId=\"30\"", "nodeId=\"030\""),
            "",
        ),
        occupied(
            "1,2,30",
            &SOCKET.replace("itemId=\"1\"", "itemId=\"0\""),
            "",
        ),
        occupied("1,2,30", SOCKET, "<Unknown/>"),
        occupied("1,2,30", &SOCKET.replace("/>", " unknown=\"true\"/>"), ""),
        valid.replace(" classInternalId=\"1\"", ""),
        valid.replace("classInternalId=\"1\"", "classInternalId=\"99\""),
        valid.replace(" ascendancyInternalId=\"alpha\"", ""),
        valid.replace(
            "ascendancyInternalId=\"alpha\"",
            "ascendancyInternalId=\"invalid\"",
        ),
    ] {
        let result = normalize(
            &f,
            &xml(&format!("{bad}{valid}"), RAW),
            &f.core.base,
            Default::default(),
        )
        .unwrap();
        assert_eq!(placements(&result), 1, "{bad}");
        assert!(
            matches!(
                result.draft().input().allocation_presets.members[0]
                    .equipment
                    .completion,
                DraftListCompletion::Pending { .. }
            ),
            "{bad}"
        );
        assert!(matches!(
            result.draft().input().allocation_presets.members[1]
                .equipment
                .completion,
            DraftListCompletion::Complete
        ));
    }
}

#[test]
fn unreached_tree_versions_and_unproved_access_never_create_placements() {
    let mut f = fixture();
    let valid = occupied("1,2,30", SOCKET, "");
    let unsupported = valid.replace("treeVersion=\"tree\"", "treeVersion=\"future\"");
    for specs in [
        format!("{unsupported}{valid}"),
        format!("{valid}{unsupported}"),
    ] {
        assert_eq!(
            placements(
                &normalize(&f, &xml(&specs, RAW), &f.core.base, Default::default()).unwrap()
            ),
            0
        );
    }
    let mut content = f.core.tree.input().content.clone();
    content.access = None;
    let Some(PassiveSocketMembershipPolicy::PobOrdinarySharedSpecSocketsV2 {
        tree_content, ..
    }) = &mut f.core.base.passive_socket_membership
    else {
        panic!()
    };
    *tree_content = tree_content_identity(&content, Default::default()).unwrap();
    f.core.tree = OwnedTreeNormalizationPolicy::bind_new(
        content,
        &f.core.registry,
        &f.core.schema,
        &f.core.mapping,
        &f.core.base,
        Default::default(),
    )
    .unwrap();
    assert_eq!(
        placements(&normalize(&f, &xml(&valid, RAW), &f.core.base, Default::default()).unwrap()),
        0
    );
}

#[test]
fn known_template_alone_never_proves_ambiguous_source_base() {
    let f = fixture();
    for raw in [
        format!("{RAW}\nOther Source Base"),
        format!("{RAW}\nSuperior Other Source Base"),
        format!("{RAW}\n{{crafted}}Other Source Base"),
        format!("{RAW}\nInjected Jewel"),
        RAW.replace("Unique ID: abcdef", "Unique ID: abcdef\nUnique ID: abcdef"),
        RAW.replace("LevelReq: 0", "Crafted: false\nLevelReq: 0"),
        RAW.replace("Unconverted Title", "Other Source Base"),
        RAW.replace(
            "An unimplemented modifier",
            "{range:0.5}An unimplemented modifier",
        ),
        RAW.replace("Item Level: 60", "Item Level: sixty"),
        RAW.replace("An unimplemented modifier", "(unproved reminder block)"),
        format!("{RAW}\nCorrupted\nAnother modifier"),
    ] {
        let result = normalize(
            &f,
            &xml(&occupied("1,2,30", SOCKET, ""), &raw),
            &f.core.base,
            Default::default(),
        )
        .unwrap();
        assert_eq!(placements(&result), 0, "{raw}");
        assert!(matches!(
            result.draft().input().allocation_presets.members[0]
                .equipment
                .completion,
            DraftListCompletion::Pending { .. }
        ));
    }
}

#[test]
fn explicit_policy_dependencies_and_declared_destinations_are_checked_even_when_empty() {
    let f = fixture();
    let xml = xml(&spec("1,2,30", "<Sockets/>"), RAW);
    for field in [
        "mapping",
        "mapping_source",
        "item_lines",
        "item_source",
        "equipment",
        "tree_content",
    ] {
        let mut policy = serde_json::to_value(&f.core.base).unwrap();
        policy["passive_socket_membership"]["source_bases"] = serde_json::json!([]);
        policy["passive_socket_membership"]["bindings"] = serde_json::json!([]);
        policy["passive_socket_membership"][field] = serde_json::json!("e".repeat(64));
        let policy = serde_json::from_value(policy).unwrap();
        assert!(
            normalize(&f, &xml, &policy, Default::default()).is_err(),
            "{field}"
        );
    }
    for mutation in 0..5 {
        let mut policy = f.core.base.clone();
        let Some(PassiveSocketMembershipPolicy::PobOrdinarySharedSpecSocketsV2 {
            source_bases,
            bindings,
            ..
        }) = &mut policy.passive_socket_membership
        else {
            panic!()
        };
        match mutation {
            0 => bindings[0].node = f.core.parent.clone(),
            1 => bindings[0].node_token = "030".into(),
            2 => bindings.push(bindings[0].clone()),
            3 => source_bases[0].base_name = "Other Source Base".into(),
            _ => source_bases[0].rule = key("unreviewed-rule"),
        }
        assert!(
            normalize(&f, &xml, &policy, Default::default()).is_err(),
            "mutation {mutation}"
        );
    }
    assert!(
        normalize(
            &f,
            &xml,
            &f.core.base,
            NormalizationLimits {
                max_work: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    let result = normalize(&f, &xml, &f.core.base, Default::default()).unwrap();
    assert!(matches!(
        result.draft().input().allocation_presets.members[0]
            .equipment
            .completion,
        DraftListCompletion::Complete
    ));
}

#[test]
fn shared_inventory_and_repeated_item_proofs_fit_a_single_bounded_work_budget() {
    let mut f = fixture();
    let Some(EquipmentMembershipPolicy::PobOrdinaryItemSetsV1 {
        source_base_names, ..
    }) = &mut f.core.base.equipment_membership
    else {
        panic!()
    };
    // The finite recognition catalogue is shared with equipment normalization.
    // Compiling and charging it twice alone would exceed this execution budget.
    source_base_names.extend(
        (0..1000).map(|index| format!("Unadmitted source base {index:04} {}", "x".repeat(80))),
    );
    let equipment = equipment_membership_identity(
        f.core.base.equipment_membership.as_ref().unwrap(),
        Default::default(),
    )
    .unwrap();
    let Some(PassiveSocketMembershipPolicy::PobOrdinarySharedSpecSocketsV2 {
        equipment: binding,
        ..
    }) = &mut f.core.base.passive_socket_membership
    else {
        panic!()
    };
    *binding = equipment;
    let specs = occupied("1,2,30", SOCKET, "").repeat(8);
    let result = normalize(
        &f,
        &xml(&specs, RAW),
        &f.core.base,
        NormalizationLimits {
            max_work: 150_000,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(placements(&result), 8);
    let item = result.draft().input().equipment.members[0]
        .item
        .to_resolved()
        .unwrap();
    assert!(
        result
            .draft()
            .input()
            .equipment
            .members
            .iter()
            .all(|usage| usage.item.to_resolved() == Some(item))
    );
    assert!(
        normalize(
            &f,
            &xml(&specs, RAW),
            &f.core.base,
            NormalizationLimits {
                max_work: 50_000,
                ..Default::default()
            }
        )
        .is_err()
    );
}
