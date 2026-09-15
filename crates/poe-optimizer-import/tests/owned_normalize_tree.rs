//! Injected tree identities and source syntax; no source runtime or numerical model.
#[path = "support/empty_owned_items.rs"]
mod empty_owned_items;
use empty_owned_items::{empty_item_source, empty_items};
use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_content::OwnedContentDigest, owned_definitions::*,
    owned_draft::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::*;
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_mapping::*,
    owned_normalize::*,
    owned_reward_policy::*,
    owned_skill_catalog::*,
    owned_source::*,
    owned_tree_policy::*,
    owned_value::*,
    owned_value_policy::*,
};

fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("tree-normalizer", "v1").unwrap()
}
fn digest() -> OwnedContentDigest {
    "a".repeat(64).parse().unwrap()
}
fn subject<I: SchemaDefinitionId>(id: &I) -> SchemaSubject {
    SchemaSubject::Definition(id.address())
}
fn known<I, S>(id: I, schema: S) -> DefinitionEntry<I, S> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn declarations() -> DeclaredSlots {
    DeclaredSlots {
        parameters: DeclaredSet::complete(vec![]),
        choices: DeclaredSet::complete(vec![]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    }
}
fn pin() -> SourcePin {
    SourcePin {
        system: ExternalSourceSystem::PathOfBuilding2,
        revision: "c".repeat(40),
        files: vec![SourceFilePin {
            path: "injected/tree.json".into(),
            sha256: "a".repeat(64),
        }],
    }
}
fn exact(source: ExternalSelector, target: SchemaSubject) -> MappingEntry {
    MappingEntry {
        source,
        outcome: MappingOutcome::Mapped {
            target,
            basis: MappingBasis::Exact,
        },
    }
}
fn scalar(id: &str, attribute: &str, boolean: bool) -> ValueRecipeInput {
    ValueRecipeInput {
        id: key(id),
        codec: ValueCodecInput {
            namespace: ns(),
            whitespace: WhitespacePolicy::Exact,
            codec: if boolean {
                ValueCodecKind::Boolean {
                    tokens: vec![
                        BooleanToken {
                            token: "true".into(),
                            value: true,
                        },
                        BooleanToken {
                            token: "false".into(),
                            value: false,
                        },
                    ],
                }
            } else {
                ValueCodecKind::Integer {
                    syntax: DecimalSyntax::Integer,
                }
            },
        },
        tiers: vec![ValueTier {
            selectors: vec![ValueSelector {
                lane: ValueLane::Attribute,
                name: attribute.into(),
            }],
            duplicates: DuplicatePolicy::Reject,
        }],
        missing: MissingValuePolicy::Pending,
    }
}

struct Fixture {
    registry: OwnedIdRegistry,
    schema: OwnedDefinitionSchemaPackage,
    mapping: OwnedMappingIndex,
    roles: OwnedSkillRoleIndex,
    rewards: OwnedRewardPolicy,
    base: NormalizationPolicy,
    tree: OwnedTreeNormalizationPolicy,
    attribute: PassiveNodeDefId,
    parent: PassiveNodeDefId,
    ordinary: PassiveNodeDefId,
    asc_paid: PassiveNodeDefId,
    dexterity: OptionDefId,
    intelligence: OptionDefId,
    option_a: OptionDefId,
}
fn fixture() -> Fixture {
    let mut registry = OwnedIdRegistry::empty(ns(), OwnedMappingLimits::default()).unwrap();
    let initial = registry.identity().unwrap();
    let class_a = registry.allocate_definition::<ClassDefinition>().unwrap();
    let class_b = registry.allocate_definition::<ClassDefinition>().unwrap();
    let asc_a = registry
        .allocate_definition::<AscendancyDefinition>()
        .unwrap();
    let asc_b = registry
        .allocate_definition::<AscendancyDefinition>()
        .unwrap();
    let root = registry
        .allocate_definition::<PassiveNodeDefinition>()
        .unwrap();
    let asc_root_a = registry
        .allocate_definition::<PassiveNodeDefinition>()
        .unwrap();
    let asc_root_b = registry
        .allocate_definition::<PassiveNodeDefinition>()
        .unwrap();
    let attribute = registry
        .allocate_definition::<PassiveNodeDefinition>()
        .unwrap();
    let parent = registry
        .allocate_definition::<PassiveNodeDefinition>()
        .unwrap();
    let ordinary = registry
        .allocate_definition::<PassiveNodeDefinition>()
        .unwrap();
    let asc_paid = registry
        .allocate_definition::<PassiveNodeDefinition>()
        .unwrap();
    let pool = registry
        .allocate_definition::<PointPoolDefinition>()
        .unwrap();
    let asc_pool = registry
        .allocate_definition::<PointPoolDefinition>()
        .unwrap();
    let strength = registry.allocate_definition::<OptionDefinition>().unwrap();
    let dexterity = registry.allocate_definition::<OptionDefinition>().unwrap();
    let intelligence = registry.allocate_definition::<OptionDefinition>().unwrap();
    let option_a = registry.allocate_definition::<OptionDefinition>().unwrap();
    let option_b = registry.allocate_definition::<OptionDefinition>().unwrap();
    let equipment = registry
        .allocate_definition::<EquipmentSlotDefinition>()
        .unwrap();
    let attribute_slot = registry
        .allocate_slot::<ChoiceSlotDefinition>(SlotOwnerDefId::PassiveNode(attribute.clone()))
        .unwrap();
    let attached_slot = registry
        .allocate_slot::<ChoiceSlotDefinition>(SlotOwnerDefId::PassiveNode(parent.clone()))
        .unwrap();
    let mut definitions = vec![
        DefinitionDescriptor::Class(known(
            class_a.clone(),
            ClassSchema {
                level: IntegerRange {
                    minimum: BoundedInteger::new(1).unwrap(),
                    maximum: BoundedInteger::new(100).unwrap(),
                },
                ascendancies: DeclaredSet::complete(vec![asc_a.clone()]),
                implicit_passives: DeclaredSet::complete(vec![root.clone()]),
                declarations: declarations(),
            },
        )),
        DefinitionDescriptor::Class(known(
            class_b.clone(),
            ClassSchema {
                level: IntegerRange {
                    minimum: BoundedInteger::new(1).unwrap(),
                    maximum: BoundedInteger::new(100).unwrap(),
                },
                ascendancies: DeclaredSet::complete(vec![asc_b.clone()]),
                implicit_passives: DeclaredSet::complete(vec![root.clone()]),
                declarations: declarations(),
            },
        )),
        DefinitionDescriptor::Ascendancy(known(
            asc_a.clone(),
            AscendancySchema {
                classes: DeclaredSet::complete(vec![class_a.clone()]),
                implicit_passives: DeclaredSet::complete(vec![asc_root_a.clone()]),
                declarations: declarations(),
            },
        )),
        DefinitionDescriptor::Ascendancy(known(
            asc_b.clone(),
            AscendancySchema {
                classes: DeclaredSet::complete(vec![class_b.clone()]),
                implicit_passives: DeclaredSet::complete(vec![asc_root_b.clone()]),
                declarations: declarations(),
            },
        )),
        DefinitionDescriptor::PointPool(known(
            pool.clone(),
            PointPoolSchema {
                scope: PointPoolScope::Either,
            },
        )),
        DefinitionDescriptor::PointPool(known(
            asc_pool.clone(),
            PointPoolSchema {
                scope: PointPoolScope::Shared,
            },
        )),
        DefinitionDescriptor::EquipmentSlot(known(
            equipment.clone(),
            EquipmentSlotSchema {
                scope: ScopePolicy::Selected,
            },
        )),
    ];
    for option in [&strength, &dexterity, &intelligence, &option_a, &option_b] {
        definitions.push(DefinitionDescriptor::Option(known(
            option.clone(),
            OptionSchema {},
        )));
    }
    for node in [
        &root,
        &asc_root_a,
        &asc_root_b,
        &attribute,
        &parent,
        &ordinary,
        &asc_paid,
    ] {
        let mut d = declarations();
        if node == &attribute {
            d.choices = DeclaredSet::complete(vec![attribute_slot.clone()]);
        }
        if node == &parent {
            d.choices = DeclaredSet::complete(vec![attached_slot.clone()]);
        }
        definitions.push(DefinitionDescriptor::PassiveNode(known(
            node.clone(),
            PassiveNodeSchema {
                pools: DeclaredSet::complete(
                    if [root.clone(), asc_root_a.clone(), asc_root_b.clone()].contains(node) {
                        vec![]
                    } else if node == &asc_paid {
                        vec![asc_pool.clone()]
                    } else {
                        vec![pool.clone()]
                    },
                ),
                adjacent: DeclaredSet::complete(vec![]),
                declarations: d,
            },
        )));
    }
    let slots = vec![
        SlotDescriptor::Choice(known(
            attribute_slot.clone(),
            ChoiceSlotSchema {
                value: ValueSchema::Option {
                    allowed: DeclaredSet::complete(vec![
                        strength.clone(),
                        dexterity.clone(),
                        intelligence.clone(),
                    ]),
                },
                presence: SlotPresence::RequiredOnce,
                owners: vec![ChoiceOwnerScope::Allocation],
            },
        )),
        SlotDescriptor::Choice(known(
            attached_slot.clone(),
            ChoiceSlotSchema {
                value: ValueSchema::Option {
                    allowed: DeclaredSet::complete(vec![option_a.clone(), option_b.clone()]),
                },
                presence: SlotPresence::RequiredOnce,
                owners: vec![ChoiceOwnerScope::Allocation],
            },
        )),
    ];
    let schema = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
            namespace: ns(),
            release: key("release"),
            semantics_version: key("v1"),
            definitions,
            slots,
        },
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    let mut entries = vec![];
    for (value, class) in [("1", &class_a), ("2", &class_b)] {
        entries.push(exact(
            ExternalSelector::Definition(ExternalOwnerSelector::Class {
                key: SourceComponent::Text(value.into()),
            }),
            subject(class),
        ));
    }
    for (class, value, asc) in [("1", "alpha", &asc_a), ("2", "beta", &asc_b)] {
        entries.push(exact(
            ExternalSelector::Definition(ExternalOwnerSelector::Ascendancy {
                class: SourceComponent::Text(class.into()),
                key: SourceComponent::Text(value.into()),
            }),
            subject(asc),
        ));
    }
    let mut tokens = vec![];
    for (token, node, root_role) in [
        ("1", &root, true),
        ("2", &asc_root_a, true),
        ("3", &asc_root_b, true),
        ("10", &attribute, false),
        ("20", &parent, false),
        ("30", &ordinary, false),
        ("40", &asc_paid, false),
    ] {
        entries.push(exact(
            ExternalSelector::Definition(ExternalOwnerSelector::PassiveNode {
                tree_version: SourceComponent::Text("tree".into()),
                node_id: SourceComponent::Text(token.into()),
                view: SourceComponent::Missing,
            }),
            subject(node),
        ));
        tokens.push(TreeTokenRow {
            token: token.into(),
            role: if root_role {
                TreeTokenRole::ImplicitRoot { node: node.clone() }
            } else {
                TreeTokenRole::Allocation {
                    node: node.clone(),
                    pool: if node == &asc_paid {
                        asc_pool.clone()
                    } else {
                        pool.clone()
                    },
                }
            },
        });
    }
    for (token, option) in [("21", &option_a), ("22", &option_b)] {
        tokens.push(TreeTokenRow {
            token: token.into(),
            role: TreeTokenRole::AttachedChoice {
                parent: parent.clone(),
                slot: attached_slot.clone(),
                option: option.clone(),
            },
        });
    }
    let mut equipment_loadouts = vec![];
    for (name, loadout, alias) in [("Weapon 1", "one", false), ("Weapon 1 Swap", "two", true)] {
        entries.push(MappingEntry {
            source: ExternalSelector::Catalog {
                kind: ExternalCatalogKind::EquipmentSlot,
                key: SourceComponent::Text(name.into()),
                version: SourceComponent::Missing,
                variant: SourceComponent::Missing,
            },
            outcome: MappingOutcome::Mapped {
                target: subject(&equipment),
                basis: if alias {
                    MappingBasis::ReviewedAlias {
                        reason: key("same-receiving-slot"),
                    }
                } else {
                    MappingBasis::Exact
                },
            },
        });
        equipment_loadouts.push(EquipmentLoadoutRule {
            source_slot: SourceComponent::Text(name.into()),
            destination: equipment.clone(),
            scope: ImportEquipmentScope::Selected {
                loadouts: vec![key(loadout)],
            },
        });
    }
    let mapping = OwnedMappingIndex::new(
        MappingPackageInput {
            schema_version: OWNED_MAPPING_PACKAGE_VERSION,
            namespace: ns(),
            registry: registry.identity().unwrap(),
            definitions: schema.identity().clone(),
            source: pin(),
            policy_version: key("mapping-v1"),
            entries,
        },
        &registry,
        &schema,
        OwnedMappingLimits::default(),
    )
    .unwrap();
    let roles = OwnedSkillRoleIndex::new(
        OwnedSkillRolePackageInput {
            schema_version: OWNED_SKILL_ROLE_VERSION,
            namespace: ns(),
            definitions: schema.identity().clone(),
            mapping: *mapping.identity(),
            compilation: SkillCatalogReceipt {
                source: pin(),
                catalog_digest: digest(),
                policy: SkillCatalogPolicy {
                    version: key("mapping-v1"),
                    absent_support: AbsentSupportPolicy::Pending,
                    absent_from_tree: AbsentFromTreePolicy::Pending,
                },
                base_registry: initial,
                staged_registry: registry.identity().unwrap(),
                gem_count: 0,
                skill_count: 0,
            },
            roles: vec![],
        },
        &mapping,
        &schema,
        SkillCatalogLimits::default(),
    )
    .unwrap();
    let rewards = OwnedRewardPolicy::new(
        RewardPolicyInput {
            schema_version: OWNED_REWARD_POLICY_VERSION,
            namespace: ns(),
            version: key("empty-rewards"),
            definitions: schema.identity().clone(),
            mapping: *mapping.identity(),
            rules: vec![],
        },
        &mapping,
        &schema,
        RewardPolicyLimits::default(),
    )
    .unwrap();
    let base = NormalizationPolicy {
        version: key("base-v1"),
        namespace: ns(),
        character_level: scalar("character", "level", false),
        gem_level: scalar("gem", "level", false),
        gem_enabled: scalar("gem-enabled", "enabled", true),
        group_enabled: scalar("group-enabled", "enabled", true),
        manual_skill_sources: vec![SourceComponent::Missing],
        empty_item_keys: vec![SourceComponent::Text("0".into())],
        generated_support_prefixes: vec![],
        allocation_attribute: "nodes".into(),
        single_active_support_target: false,
        equipment_loadouts,
        gem_quality: GemQualityPolicy::Unconverted,
    };
    let tree = OwnedTreeNormalizationPolicy::bind_new(
        TreeNormalizationContent {
            version: key("tree-v1"),
            source: pin(),
            catalog: digest(),
            policy: digest(),
            tree_version: "tree".into(),
            classes: vec![
                TreeClassRow {
                    key: "1".into(),
                    class: class_a,
                },
                TreeClassRow {
                    key: "2".into(),
                    class: class_b,
                },
            ],
            ascendancies: vec![
                TreeAscendancyRow {
                    class_key: "1".into(),
                    key: "alpha".into(),
                    ordinal: 1,
                    ascendancy: asc_a,
                },
                TreeAscendancyRow {
                    class_key: "2".into(),
                    key: "beta".into(),
                    ordinal: 1,
                    ascendancy: asc_b,
                },
            ],
            tokens,
            attributes: vec![TreeAttributeRule {
                node: attribute.clone(),
                slot: attribute_slot,
                lanes: vec![
                    TreeAttributeLane {
                        attribute: "strNodes".into(),
                        option: strength,
                    },
                    TreeAttributeLane {
                        attribute: "dexNodes".into(),
                        option: dexterity.clone(),
                    },
                    TreeAttributeLane {
                        attribute: "intNodes".into(),
                        option: intelligence.clone(),
                    },
                ],
            }],
            syntax: TreeNormalizationSyntax {
                tree_version_attribute: "treeVersion".into(),
                class_attribute: "classInternalId".into(),
                ascendancy_attribute: "ascendancyInternalId".into(),
                class_consistency_attribute: Some("classId".into()),
                ascendancy_consistency_attribute: Some("ascendClassId".into()),
                overrides_element: "Overrides".into(),
                attribute_override_element: "AttributeOverride".into(),
                weapon_overlays: vec![
                    TreeWeaponOverlay {
                        element: "WeaponSet1".into(),
                        nodes_attribute: "nodes".into(),
                        loadout: key("one"),
                    },
                    TreeWeaponOverlay {
                        element: "WeaponSet2".into(),
                        nodes_attribute: "nodes".into(),
                        loadout: key("two"),
                    },
                ],
                ignored_spec_children: vec!["Sockets".into(), "URL".into(), "Notes".into()],
            },
        },
        &registry,
        &schema,
        &mapping,
        &base,
        TreePolicyLimits::default(),
    )
    .unwrap();
    Fixture {
        registry,
        schema,
        mapping,
        roles,
        rewards,
        base,
        tree,
        attribute,
        parent,
        ordinary,
        asc_paid,
        dexterity,
        intelligence,
        option_a,
    }
}

fn spec(nodes: &str, children: &str) -> String {
    format!(
        r#"<Spec treeVersion="tree" classInternalId="1" classId="1" ascendancyInternalId="alpha" ascendClassId="1" nodes="{nodes}">{children}</Spec>"#
    )
}
fn overrides(dexterity: &str, intelligence: &str) -> String {
    format!(
        r#"<Overrides><AttributeOverride strNodes="" dexNodes="{dexterity}" intNodes="{intelligence}"/></Overrides>"#
    )
}
fn document(specs: &str, equipment: bool) -> String {
    let slots = if equipment {
        r#"<Slot name="Weapon 1" itemId="0"/><Slot name="Weapon 1 Swap" itemId="0"/>"#
    } else {
        ""
    };
    format!(
        r#"<PathOfBuilding2><Build level="60"/><Tree activeSpec="1">{specs}</Tree><Items><ItemSet id="1">{slots}</ItemSet></Items><Skills/><Config/></PathOfBuilding2>"#
    )
}
fn run(
    f: &Fixture,
    xml: &str,
    enabled: bool,
    base: &NormalizationPolicy,
) -> Result<NormalizedImport, NormalizationError> {
    let source = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([91; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let queries: Vec<_> = (0..22)
        .map(|i| ImportQueryTemplate {
            id: QueryId::new(format!("ordered-{i:02}")).unwrap(),
            metric: ExternalSelector::Catalog {
                kind: ExternalCatalogKind::Metric,
                key: SourceComponent::Text(format!("metric-{i}")),
                version: SourceComponent::Missing,
                variant: SourceComponent::Missing,
            },
            target: ImportQueryTarget::Player,
        })
        .collect();
    normalize_fresh(
        &evidence,
        *source.allocator_state(),
        NormalizationArtifacts {
            registry: &f.registry,
            definitions: &f.schema,
            mappings: &f.mapping,
            roles: &f.roles,
            rewards: &f.rewards,
            items: &empty_items(&f.schema),
            item_source: &empty_item_source(&f.schema),
            tree: enabled.then_some(&f.tree),
        },
        base,
        &queries,
        NormalizationLimits::default(),
    )
}
fn allocation<'a>(result: &'a NormalizedImport, node: &PassiveNodeDefId) -> &'a AllocationDraft {
    result
        .draft()
        .input()
        .allocations
        .members
        .iter()
        .find(|a| a.node.to_resolved().as_ref() == Some(node))
        .unwrap()
}
fn chosen(row: &AllocationDraft) -> Option<ParameterValue> {
    row.choices
        .members
        .first()
        .and_then(|c| c.value.to_resolved())
}
fn pending_list<T>(list: &DraftList<T>) -> bool {
    matches!(list.completion, DraftListCompletion::Pending { .. })
}

#[test]
fn roots_are_character_owned_and_options_are_exact_parent_choices() {
    let f = fixture();
    let result = run(
        &f,
        &document(&spec("1,2,10,20,21,30", &overrides("10", "")), true),
        true,
        &f.base,
    )
    .unwrap();
    assert_eq!(result.draft().input().allocations.members.len(), 3);
    assert_eq!(result.sidecar().schema_version, 9);
    assert_eq!(result.sidecar().tree_policy, Some(*f.tree.identity()));
    assert_eq!(
        result
            .sidecar()
            .origins
            .iter()
            .flat_map(|o| &o.links)
            .filter(|o| matches!(o, OwnedOriginTarget::ImplicitPassive { .. }))
            .count(),
        2
    );
    assert_eq!(
        chosen(allocation(&result, &f.attribute)),
        Some(ParameterValue::Option(f.dexterity.clone()))
    );
    assert_eq!(
        chosen(allocation(&result, &f.parent)),
        Some(ParameterValue::Option(f.option_a.clone()))
    );
    assert!(!pending_list(
        &result.draft().input().allocation_presets.members[0].allocations
    ));
    assert!(
        result
            .draft()
            .input()
            .allocations
            .members
            .iter()
            .all(|a| matches!(a.access, DraftAllocationAccess::Pending(_)))
    );
    let queries = &result.draft().input().query_presets.members[0]
        .queries
        .requests
        .members;
    assert_eq!(queries.len(), 22);
    assert_eq!(queries[0].id.as_str(), "ordered-00");
    assert_eq!(queries[21].id.as_str(), "ordered-21");
    let actual_issues: std::collections::BTreeSet<_> =
        validate_draft(result.draft().input(), DraftLimits::default())
            .unwrap()
            .issues
            .into_iter()
            .map(|issue| issue.id)
            .collect();
    let linked_issues: std::collections::BTreeSet<_> = result
        .sidecar()
        .origins
        .iter()
        .flat_map(|origin| &origin.links)
        .filter_map(|link| match link {
            OwnedOriginTarget::Issue(id) => Some(*id),
            _ => None,
        })
        .collect();
    assert_eq!(
        linked_issues, actual_issues,
        "every tree issue must point to a live draft obligation"
    );
}

#[test]
fn saved_specs_keep_shared_root_identity_and_independent_attribute_choices() {
    let f = fixture();
    let a = spec("1,2,10", &overrides("10", ""));
    let b = spec("1,3,10", &overrides("", "10"))
        .replace("classInternalId=\"1\"", "classInternalId=\"2\"")
        .replace("classId=\"1\"", "classId=\"2\"")
        .replace("alpha", "beta");
    let result = run(&f, &document(&(a + &b), true), true, &f.base).unwrap();
    let rows = &result.draft().input().allocations.members;
    assert_eq!(rows.len(), 2);
    assert_ne!(rows[0].id, rows[1].id);
    assert_eq!(rows[0].node, rows[1].node);
    assert_eq!(
        chosen(&rows[0]),
        Some(ParameterValue::Option(f.dexterity.clone()))
    );
    assert_eq!(
        chosen(&rows[1]),
        Some(ParameterValue::Option(f.intelligence.clone()))
    );
    assert_eq!(result.draft().input().character_presets.members.len(), 2);
}

#[test]
fn overlay_scope_requires_real_loadout_occurrences_and_never_relabels_ascendancy() {
    let f = fixture();
    let tree = spec("1,2,30,40", r#"<WeaponSet1 nodes="30,40"/>"#);
    let result = run(&f, &document(&tree, true), true, &f.base).unwrap();
    assert!(matches!(
        allocation(&result, &f.ordinary).scope,
        DraftField::Known {
            value: LoadoutScope::Selected { .. }
        }
    ));
    assert!(matches!(
        allocation(&result, &f.asc_paid).scope,
        DraftField::Pending(_)
    ));
    let missing = run(&f, &document(&tree, false), true, &f.base).unwrap();
    assert!(missing.draft().input().weapon_loadouts.members.is_empty());
    assert!(matches!(
        allocation(&missing, &f.ordinary).scope,
        DraftField::Pending(_)
    ));
}

#[test]
fn duplicate_unknown_foreign_root_and_unparented_option_keep_real_pending_census() {
    let f = fixture();
    for nodes in [
        "1,2,10,10",
        "1,2,999",
        "1,2,3",
        "1,2,21",
        "1,2,,30",
        "1,2,+30",
    ] {
        let result = run(&f, &document(&spec(nodes, ""), true), true, &f.base).unwrap();
        assert!(
            pending_list(&result.draft().input().allocation_presets.members[0].allocations),
            "{nodes}"
        );
        assert!(
            !result.draft().input().allocations.members.iter().any(|a| a
                .node
                .to_resolved()
                .as_ref()
                == Some(&f.parent))
        );
        assert!(
            result
                .sidecar()
                .origins
                .iter()
                .flat_map(|o| &o.links)
                .any(|o| matches!(o, OwnedOriginTarget::Issue(_)))
        );
    }
}

#[test]
fn missing_or_conflicting_choice_and_overlay_data_never_selects_a_winner() {
    let f = fixture();
    for (nodes, children) in [("1,2,20", String::new()), ("1,2,20,21,22", String::new()), ("1,2,10", overrides("10", "10")), ("1,2,10", "<Overrides xmlns=\"urn:other\"><AttributeOverride strNodes=\"10\" dexNodes=\"\" intNodes=\"\"/></Overrides>".into())] {
        let result = run(&f, &document(&spec(nodes, &children), true), true, &f.base).unwrap();
        let row = &result.draft().input().allocations.members[0];
        assert_eq!(chosen(row), None, "{nodes} {children}");
    }
    let result = run(
        &f,
        &document(
            &spec(
                "1,2,30",
                r#"<WeaponSet1 nodes="30"/><WeaponSet2 nodes="30"/>"#,
            ),
            true,
        ),
        true,
        &f.base,
    )
    .unwrap();
    assert!(matches!(
        allocation(&result, &f.ordinary).scope,
        DraftField::Pending(_)
    ));
}

#[test]
fn class_consistency_unknown_children_and_namespace_do_not_gain_absence_defaults() {
    let f = fixture();
    let mismatch = spec("1,2,30", "").replace("classId=\"1\"", "classId=\"2\"");
    let result = run(&f, &document(&mismatch, true), true, &f.base).unwrap();
    assert!(matches!(
        result.draft().input().character_presets.members[0].class,
        DraftField::Pending(_)
    ));
    let mismatch = spec("1,2,30", "").replace("ascendClassId=\"1\"", "ascendClassId=\"2\"");
    let result = run(&f, &document(&mismatch, true), true, &f.base).unwrap();
    assert!(matches!(
        result.draft().input().character_presets.members[0].ascendancy,
        DraftField::Pending(_)
    ));
    let result = run(
        &f,
        &document(&spec("1,2,30", "<UnknownTreeEdit/>"), true),
        true,
        &f.base,
    )
    .unwrap();
    assert!(matches!(
        allocation(&result, &f.ordinary).scope,
        DraftField::Pending(_)
    ));
    let namespaced = spec("1,2,30", "").replace("<Spec ", "<Spec xmlns=\"urn:other\" ");
    let result = run(&f, &document(&namespaced, true), true, &f.base).unwrap();
    assert!(
        result
            .draft()
            .input()
            .allocations
            .members
            .iter()
            .all(|a| matches!(a.node, DraftField::Pending(_)))
    );
}

#[test]
fn absent_policy_preserves_conservative_path_and_stale_policy_fails_without_specs() {
    let f = fixture();
    let result = run(&f, &document(&spec("1,2,30", ""), true), false, &f.base).unwrap();
    assert_eq!(result.sidecar().tree_policy, None);
    assert_eq!(result.draft().input().allocations.members.len(), 3);
    assert!(
        result
            .draft()
            .input()
            .allocations
            .members
            .iter()
            .all(|a| matches!(a.pool, DraftField::Pending(_)))
    );
    let mut changed = f.base.clone();
    changed.version = key("different-base-policy");
    assert!(matches!(
        run(&f, &document("", false), true, &changed),
        Err(NormalizationError::Tree(_))
    ));
}
