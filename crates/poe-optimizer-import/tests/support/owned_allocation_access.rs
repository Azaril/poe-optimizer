//! Small, renamed-ID graph contrasts. These assert import access only; they do
//! not claim a complete passive evaluator or spend/budget/legality calculation.
use super::*;
use std::collections::BTreeSet;

fn node(f: &Fixture, token: &str) -> PassiveNodeDefId {
    match f.tree.lookup("tree", token).unwrap() {
        TreeTokenRole::Allocation { node, .. } | TreeTokenRole::ImplicitRoot { node } => {
            node.clone()
        }
        _ => panic!("physical node token"),
    }
}
fn pool(f: &Fixture, token: &str) -> PointPoolDefId {
    match f.tree.lookup("tree", token).unwrap() {
        TreeTokenRole::Allocation { pool, .. } => pool.clone(),
        _ => panic!("paid token"),
    }
}
fn partial<T>(members: Vec<T>, owner: &PassiveNodeDefId) -> DeclaredSet<T> {
    DeclaredSet::partial(
        members,
        vec![SchemaGap {
            subject: subject(owner),
            facet: SchemaFacet::StaticLinks,
            code: key("other-links-unconverted"),
        }],
    )
}
fn rebind(f: &mut Fixture, schema: SchemaPackageInput, content: TreeNormalizationContent) {
    f.schema = OwnedDefinitionSchemaPackage::new(schema, OwnedSchemaLimits::default()).unwrap();
    let mut mapping = f.mapping.input().clone();
    mapping.definitions = f.schema.identity().clone();
    f.mapping = OwnedMappingIndex::new(
        mapping,
        &f.registry,
        &f.schema,
        OwnedMappingLimits::default(),
    )
    .unwrap();
    let mut roles = f.roles.input().clone();
    roles.definitions = f.schema.identity().clone();
    roles.mapping = *f.mapping.identity();
    f.roles = OwnedSkillRoleIndex::new(roles, &f.mapping, &f.schema, SkillCatalogLimits::default())
        .unwrap();
    let mut rewards = f.rewards.input().clone();
    rewards.definitions = f.schema.identity().clone();
    rewards.mapping = *f.mapping.identity();
    f.rewards = OwnedRewardPolicy::new(
        rewards,
        &f.mapping,
        &f.schema,
        RewardPolicyLimits::default(),
    )
    .unwrap();
    f.tree = OwnedTreeNormalizationPolicy::bind_new(
        content,
        &f.registry,
        &f.schema,
        &f.mapping,
        &f.base,
        TreePolicyLimits::default(),
    )
    .unwrap();
}
fn prepared() -> Fixture {
    let mut f = fixture();
    let root = node(&f, "1");
    let asc_root = node(&f, "2");
    let mut schema = f.schema.input().clone();
    for descriptor in &mut schema.definitions {
        if let DefinitionDescriptor::PassiveNode(row) = descriptor
            && let SchemaState::Known(s) = &mut row.schema
        {
            let links = if row.id == f.attribute {
                vec![root.clone()]
            } else if row.id == f.parent {
                vec![f.attribute.clone()]
            } else if row.id == f.ordinary {
                vec![f.parent.clone()]
            } else if row.id == f.asc_paid {
                vec![asc_root.clone()]
            } else {
                vec![]
            };
            s.adjacent = partial(links, &row.id);
            if !s.pools.members.is_empty() {
                s.pools = partial(s.pools.members.clone(), &row.id);
            }
        }
    }
    let mut content = f.tree.input().content.clone();
    content
        .syntax
        .ignored_spec_children
        .retain(|v| v != "Notes");
    content.access = Some(AllocationAccessPolicy::PobIndependentSavedPathsV1 {
        pools: vec![
            AllocationAccessPool {
                pool: pool(&f, "10"),
                root: AllocationRootKind::Class,
            },
            AllocationAccessPool {
                pool: pool(&f, "40"),
                root: AllocationRootKind::Ascendancy,
            },
        ],
        nodes: vec![
            f.attribute.clone(),
            f.parent.clone(),
            f.ordinary.clone(),
            f.asc_paid.clone(),
        ],
    });
    rebind(&mut f, schema, content);
    f
}
fn scoped_fixture() -> Fixture {
    let mut f = prepared();
    let mut input = f.tree.input().clone();
    let Some(AllocationAccessPolicy::PobIndependentSavedPathsV1 { pools, nodes }) =
        input.content.access.take()
    else {
        unreachable!()
    };
    input.content.access =
        Some(AllocationAccessPolicy::PobIndependentSavedPathsV2 { pools, nodes });
    f.tree = checked(&f, input).unwrap();
    f
}
fn evaluate(f: &Fixture, nodes: &str, children: &str) -> NormalizedImport {
    run(f, &document(&spec(nodes, children), true), true, &f.base).unwrap()
}
fn is_ordinary(result: &NormalizedImport, id: &PassiveNodeDefId) -> bool {
    matches!(
        allocation(result, id).access,
        DraftAllocationAccess::Ordinary
    )
}
fn count(result: &NormalizedImport) -> usize {
    result
        .draft()
        .input()
        .allocations
        .members
        .iter()
        .filter(|a| matches!(a.access, DraftAllocationAccess::Ordinary))
        .count()
}
fn checked(
    f: &Fixture,
    input: TreeNormalizationPackageInput,
) -> Result<OwnedTreeNormalizationPolicy, TreePolicyError> {
    OwnedTreeNormalizationPolicy::new(
        input,
        &f.registry,
        &f.schema,
        &f.mapping,
        &f.base,
        TreePolicyLimits::default(),
    )
}

#[test]
fn positive_directed_paths_use_partial_memberships_without_closing_schema_or_queries() {
    let f = prepared();
    let before = f.schema.input().clone();
    let result = evaluate(&f, "1,2,10,20,21,30,40", &overrides("10", ""));
    assert_eq!(count(&result), 4);
    assert_eq!(f.schema.input(), &before);
    for row in &before.definitions {
        if let DefinitionDescriptor::PassiveNode(row) = row
            && let SchemaState::Known(schema) = &row.schema
        {
            assert!(!schema.adjacent.is_complete());
        }
    }
    assert_eq!(
        result.draft().input().query_presets.members[0]
            .queries
            .requests
            .members
            .len(),
        22
    );
    assert!(
        validate_draft(result.draft().input(), DraftLimits::default())
            .unwrap()
            .issues
            .len()
            > 20
    );
    let ids: BTreeSet<_> = validate_draft(result.draft().input(), DraftLimits::default())
        .unwrap()
        .issues
        .into_iter()
        .map(|v| v.id)
        .collect();
    let linked: BTreeSet<_> = result
        .sidecar()
        .origins
        .iter()
        .flat_map(|o| &o.links)
        .filter_map(|v| match v {
            OwnedOriginTarget::Issue(id) => Some(*id),
            _ => None,
        })
        .collect();
    assert_eq!(ids, linked, "no retired access issue or ghost link");
}

#[test]
fn missing_root_tokens_are_implicit_but_missing_and_scoped_connectors_are_not() {
    let f = prepared();
    assert_eq!(
        count(&evaluate(&f, "10,20,21,30,40", &overrides("10", ""))),
        4
    );
    let missing = evaluate(&f, "1,2,20,21,30,40", &overrides("", ""));
    assert!(!is_ordinary(&missing, &f.parent));
    assert!(!is_ordinary(&missing, &f.ordinary));
    assert!(is_ordinary(&missing, &f.asc_paid));
    let scoped = evaluate(
        &f,
        "1,2,10,20,21,30,40",
        &(overrides("10", "") + r#"<WeaponSet1 nodes="10"/>"#),
    );
    assert_eq!(count(&scoped), 1);
    assert!(is_ordinary(&scoped, &f.asc_paid));
}

#[test]
fn scoped_implicit_roots_do_not_seed_shared_paths_or_borrow_other_roots() {
    let f = prepared();
    for (scoped, ordinary, ascendancy) in [
        ("1", false, true),
        ("2", true, false),
        ("1,2", false, false),
        ("3", true, true),
    ] {
        let result = evaluate(
            &f,
            "1,2,3,10,20,21,30,40",
            &(overrides("10", "") + &format!(r#"<WeaponSet1 nodes="{scoped}"/>"#)),
        );
        for node in [&f.attribute, &f.parent, &f.ordinary] {
            assert_eq!(
                is_ordinary(&result, node),
                ordinary,
                "root overlay {scoped}"
            );
        }
        assert_eq!(
            is_ordinary(&result, &f.asc_paid),
            ascendancy,
            "root overlay {scoped}"
        );
    }
}

#[test]
fn numeric_aliases_cannot_hide_scoped_roots_or_connectors() {
    let f = prepared();
    for alias in ["01", "010", "9007199254740992"] {
        let result = evaluate(
            &f,
            &format!("1,2,10,20,21,30,40,{alias}"),
            &(overrides("10", "") + &format!(r#"<WeaponSet1 nodes="{alias}"/>"#)),
        );
        assert_eq!(count(&result), 0, "numeric alias {alias}");
    }
}

#[test]
fn unknown_foreign_and_duplicate_nodes_never_supply_a_route() {
    let f = prepared();
    let unknown = evaluate(&f, "1,2,3,10,20,21,30,40,999", &overrides("10", ""));
    assert_eq!(count(&unknown), 4, "independent positive subset survives");
    assert!(pending_list(
        &unknown.draft().input().allocation_presets.members[0].allocations
    ));
    let duplicate = evaluate(&f, "1,2,10,10,20,21,30,40", &overrides("10", ""));
    assert_eq!(count(&duplicate), 1);
    let mut g = prepared();
    let foreign_root = node(&g, "3");
    let mut schema = g.schema.input().clone();
    for row in &mut schema.definitions {
        if let DefinitionDescriptor::PassiveNode(row) = row
            && row.id == g.ordinary
            && let SchemaState::Known(s) = &mut row.schema
        {
            s.adjacent.members = vec![foreign_root.clone()];
        }
    }
    let content = g.tree.input().content.clone();
    rebind(&mut g, schema, content);
    let result = evaluate(&g, "1,2,3,10,20,21,30,40", &overrides("10", ""));
    assert!(
        !is_ordinary(&result, &g.ordinary),
        "foreign asc root cannot be a class endpoint"
    );
    assert!(is_ordinary(&result, &g.asc_paid));
}

#[test]
fn excluded_nodes_reverse_only_edges_and_cross_pool_edges_never_transit() {
    for mode in 0..3 {
        let mut f = prepared();
        let mut content = f.tree.input().content.clone();
        let mut schema = f.schema.input().clone();
        if mode == 0 {
            let Some(AllocationAccessPolicy::PobIndependentSavedPathsV1 { nodes, .. }) =
                &mut content.access
            else {
                unreachable!()
            };
            nodes.retain(|v| v != &f.parent);
        } else {
            let root = node(&f, "1");
            for row in &mut schema.definitions {
                if let DefinitionDescriptor::PassiveNode(row) = row
                    && let SchemaState::Known(s) = &mut row.schema
                {
                    if mode == 1 && row.id == root {
                        s.adjacent.members = vec![f.attribute.clone()];
                    }
                    if mode == 1 && row.id == f.attribute {
                        s.adjacent.members.clear();
                    }
                    if mode == 2 && row.id == f.parent {
                        s.adjacent.members = vec![f.asc_paid.clone()];
                    }
                }
            }
        }
        rebind(&mut f, schema, content);
        let result = evaluate(&f, "1,2,10,20,21,30,40", &overrides("10", ""));
        assert!(!is_ordinary(&result, &f.parent), "case {mode}");
        assert!(!is_ordinary(&result, &f.ordinary), "case {mode}");
        assert!(is_ordinary(&result, &f.asc_paid));
    }
}

#[test]
fn unresolved_topology_invariant_choice_can_transit_but_cannot_gain_target_access() {
    let f = prepared();
    let result = evaluate(&f, "1,2,10,20,21,30,40", &overrides("", ""));
    assert!(!is_ordinary(&result, &f.attribute));
    assert!(is_ordinary(&result, &f.parent));
    assert!(is_ordinary(&result, &f.ordinary));
    assert!(is_ordinary(&result, &f.asc_paid));
}

#[test]
fn ordinary_socket_contents_do_not_supply_or_invalidate_independent_paths() {
    let f = prepared();
    for sockets in [
        "<Sockets/>",
        r#"<Sockets><Socket nodeId="30" itemId="7"/></Sockets>"#,
        r#"<Sockets><Socket nodeId="30" itemId="9007199254740991"/></Sockets>"#,
    ] {
        assert_eq!(
            count(&evaluate(
                &f,
                "1,2,10,20,21,30,40",
                &(overrides("10", "") + sockets)
            )),
            4
        );
    }
}

#[test]
fn full_source_census_rejects_extra_namespaces_aliases_and_loader_shapes() {
    let f = prepared();
    let base = document(&spec("1,2,10,20,21,30,40", &overrides("10", "")), true);
    let probes = vec![
        base.replace("<Spec ", "<Spec unknown=\"1\" "),
        base.replace("<Spec ", "<Spec xmlns=\"urn:other\" "),
        base.replace("nodes=\"1,2,10,20,21,30,40\"", "nodes=\"1,2,10;20,21,30,40\""),
        base.replace("</Spec>", "<Unknown/></Spec>"),
        base.replace("</Spec>", "<URL/></Spec>"),
        base.replace("</Spec>", "<Sockets/><Sockets/></Spec>"),
        base.replace("</Spec>", "<Sockets><Socket nodeId=\"30\" itemId=\"7\"/><Socket nodeId=\"030\" itemId=\"8\"/></Sockets></Spec>"),
        base.replace("</Spec>", "<Sockets><Socket nodeId=\"30\" itemId=\"7\" unknown=\"x\"/></Sockets></Spec>"),
        base.replace("</Spec>", "<Sockets><Socket nodeId=\"9007199254740992\" itemId=\"7\"/></Sockets></Spec>"),
        base.replace("</Spec>", "<Sockets><Socket nodeId=\"30\" itemId=\"9007199254740993\"/></Sockets></Spec>"),
        base.replace("activeSpec=\"1\"", "activeSpec=\"9007199254740992\""),
        base.replace("</Spec>", "<WeaponSet1 nodes=\"\" extra=\"x\"/></Spec>"),
        base.replace("</Spec>", "<URL><Spec/></URL></Spec>"),
        base.replace("</Spec>", "<Sockets xmlns=\"urn:other\"/></Spec>"),
        base.replace("<Spec ", "<Spec masteryEffects=\"{10,11}\" "),
        base.replace("<Spec ", "<Spec secondaryAscendClassId=\"2\" "),
        base.replace("</PathOfBuilding2>", "<Tree activeSpec=\"1\"/></PathOfBuilding2>"),
        base.replace("</PathOfBuilding2>", "<Tree xmlns=\"urn:other\"/></PathOfBuilding2>"),
        base.replace("</PathOfBuilding2>", "<Spec/></PathOfBuilding2>"),
    ];
    for (i, xml) in probes.iter().enumerate() {
        let result = run(&f, xml, true, &f.base).unwrap();
        assert_eq!(count(&result), 0, "source census case {i}");
    }
}

#[test]
fn proof_is_source_bound_and_does_not_cross_saved_presets() {
    let f = prepared();
    let first = spec("1,2,10,20,21,30,40", &overrides("10", ""));
    let second = spec("1,2,20,21,30,40", &overrides("", ""));
    let result = run(&f, &document(&(first + &second), true), true, &f.base).unwrap();
    assert_eq!(count(&result), 5);
    let ordinary: Vec<_> = result
        .draft()
        .input()
        .allocations
        .members
        .iter()
        .filter(|a| a.node.to_resolved().as_ref() == Some(&f.ordinary))
        .collect();
    assert!(matches!(
        ordinary[0].access,
        DraftAllocationAccess::Ordinary
    ));
    assert!(matches!(
        ordinary[1].access,
        DraftAllocationAccess::Pending(_)
    ));
}

#[test]
fn constructor_rejects_wrong_source_syntax_roles_bindings_and_duplicate_claims() {
    let f = prepared();
    let current = f.tree.input();
    for mode in 0..6 {
        let mut input = current.clone();
        match mode {
            0 => input.content.syntax.class_attribute = "alternateClass".into(),
            1 => input
                .content
                .syntax
                .ignored_spec_children
                .push("Unknown".into()),
            2 => input.content.syntax.weapon_overlays[0].nodes_attribute = "other".into(),
            3 => {
                let Some(AllocationAccessPolicy::PobIndependentSavedPathsV1 { nodes, .. }) =
                    &mut input.content.access
                else {
                    unreachable!()
                };
                nodes.push(node(&f, "1"));
            }
            4 => {
                let Some(AllocationAccessPolicy::PobIndependentSavedPathsV1 { nodes, .. }) =
                    &mut input.content.access
                else {
                    unreachable!()
                };
                nodes.push(nodes[0].clone());
            }
            _ => input.definitions.content_sha256 = "b".repeat(64),
        }
        assert!(checked(&f, input).is_err(), "invalid proof case {mode}");
    }
    assert!(
        f.tree
            .validate_limits(TreePolicyLimits {
                max_schema_work: 1,
                ..Default::default()
            })
            .is_err()
    );
    let bounded = TreePolicyLimits {
        max_collection_entries: 1,
        ..Default::default()
    };
    assert!(
        OwnedTreeNormalizationPolicy::new(
            current.clone(),
            &f.registry,
            &f.schema,
            &f.mapping,
            &f.base,
            bounded
        )
        .is_err()
    );
}

#[test]
fn omitted_and_empty_domains_keep_pending_access_and_omitted_wire_shape() {
    let mut f = prepared();
    let mut input = f.tree.input().clone();
    input.content.access = None;
    f.tree = checked(&f, input.clone()).unwrap();
    let wire = serde_json::to_value(&input).unwrap();
    assert!(wire["content"].get("access").is_none());
    assert_eq!(
        count(&evaluate(&f, "1,2,10,20,21,30,40", &overrides("10", ""))),
        0
    );
    input.content.access = Some(AllocationAccessPolicy::PobIndependentSavedPathsV1 {
        pools: vec![],
        nodes: vec![],
    });
    f.tree = checked(&f, input).unwrap();
    assert_eq!(
        count(&evaluate(&f, "1,2,10,20,21,30,40", &overrides("10", ""))),
        0
    );
}

#[test]
fn v2_same_mode_targets_use_shared_prefixes_but_never_the_other_mode() {
    let f = scoped_fixture();
    for mode in [1, 2] {
        let result = evaluate(
            &f,
            "1,2,10,20,21,30,40",
            &(overrides("10", "") + &format!(r#"<WeaponSet{mode} nodes="20,30"/>"#)),
        );
        assert_eq!(count(&result), 4);
        assert_eq!(
            allocation(&result, &f.parent).scope,
            allocation(&result, &f.ordinary).scope
        );
        assert!(matches!(allocation(&result, &f.parent).scope,
            DraftField::Known { value: LoadoutScope::Selected { ref loadouts } } if loadouts.len() == 1));
    }
    let result = evaluate(
        &f,
        "1,2,10,20,21,30,40",
        &(overrides("10", "") + r#"<WeaponSet1 nodes="30"/><WeaponSet2 nodes="20"/>"#),
    );
    assert!(is_ordinary(&result, &f.parent));
    assert!(!is_ordinary(&result, &f.ordinary));
    assert_ne!(
        allocation(&result, &f.parent).scope,
        allocation(&result, &f.ordinary).scope
    );
}

#[test]
fn v2_scoped_paths_require_retained_shared_intermediates_not_raw_mixed_reachability() {
    let f = scoped_fixture();
    let result = evaluate(
        &f,
        "1,2,10,20,21,30,40",
        &(overrides("10", "") + r#"<WeaponSet1 nodes="10,30"/>"#),
    );
    assert!(is_ordinary(&result, &f.attribute));
    assert!(
        !is_ordinary(&result, &f.parent),
        "Shared node cannot depend on scoped prefix"
    );
    assert!(
        !is_ordinary(&result, &f.ordinary),
        "scoped target cannot use the pruned Shared bridge"
    );
    assert!(is_ordinary(&result, &f.asc_paid));
    let unresolved_choice = evaluate(
        &f,
        "1,2,10,20,21,30,40",
        &(overrides("", "") + r#"<WeaponSet1 nodes="20,30"/>"#),
    );
    assert!(!is_ordinary(&unresolved_choice, &f.attribute));
    assert!(
        is_ordinary(&unresolved_choice, &f.parent),
        "topology-invariant choice does not remove a positive Shared path"
    );
    assert!(is_ordinary(&unresolved_choice, &f.ordinary));
}

#[test]
fn v2_ambiguous_overlay_members_never_supply_scoped_transit() {
    let f = scoped_fixture();
    for overlays in [
        r#"<WeaponSet1 nodes="20,20,30"/>"#,
        r#"<WeaponSet1 nodes="20,30"/><WeaponSet2 nodes="20"/>"#,
    ] {
        let result = evaluate(&f, "1,2,10,20,21,30,40", &(overrides("10", "") + overlays));
        assert!(matches!(
            allocation(&result, &f.parent).scope,
            DraftField::Pending(_)
        ));
        assert!(!is_ordinary(&result, &f.parent));
        assert!(!is_ordinary(&result, &f.ordinary));
        assert!(is_ordinary(&result, &f.attribute));
        assert!(is_ordinary(&result, &f.asc_paid));
    }
}

#[test]
fn v2_scoped_root_activation_is_explicitly_unresolved_even_without_paid_descendants() {
    let f = scoped_fixture();
    for root in ["1", "2"] {
        let root_only = evaluate(&f, "1,2", &format!(r#"<WeaponSet1 nodes="{root}"/>"#));
        assert!(root_only.draft().input().allocations.members.is_empty());
        assert!(pending_list(
            &root_only.draft().input().allocation_presets.members[0].allocations
        ));
    }
    let result = evaluate(
        &f,
        "1,2,10,20,21,30,40",
        &(overrides("10", "") + r#"<WeaponSet1 nodes="1,10,20,30"/>"#),
    );
    assert_eq!(
        count(&result),
        1,
        "unrepresented implicit-root scope cannot authorize even its own mode"
    );
    assert!(is_ordinary(&result, &f.asc_paid));
    assert!(pending_list(
        &result.draft().input().allocation_presets.members[0].allocations
    ));
    let ascendancy = evaluate(
        &f,
        "1,2,10,20,21,30,40",
        &(overrides("10", "") + r#"<WeaponSet2 nodes="2,40"/>"#),
    );
    assert_eq!(count(&ascendancy), 3);
    assert!(is_ordinary(&ascendancy, &f.ordinary));
    assert!(!is_ordinary(&ascendancy, &f.asc_paid));
    assert!(pending_list(
        &ascendancy.draft().input().allocation_presets.members[0].allocations
    ));
    let old = prepared();
    let old_root = evaluate(&old, "1,2", r#"<WeaponSet1 nodes="1"/>"#);
    assert!(
        !pending_list(&old_root.draft().input().allocation_presets.members[0].allocations),
        "historical V1 behavior is frozen"
    );
}

#[test]
fn v2_pool_scope_is_a_transit_requirement_and_v1_per_loadout_rejection_is_frozen() {
    for scope in [PointPoolScope::PerLoadout, PointPoolScope::Shared] {
        let mut f = scoped_fixture();
        let target_pool = pool(&f, "10");
        let mut schema = f.schema.input().clone();
        for row in &mut schema.definitions {
            if let DefinitionDescriptor::PointPool(row) = row
                && row.id == target_pool
                && let SchemaState::Known(s) = &mut row.schema
            {
                s.scope = scope;
            }
        }
        let content = f.tree.input().content.clone();
        rebind(&mut f, schema, content);
        let mixed = evaluate(
            &f,
            "1,2,10,20,21,30,40",
            &(overrides("10", "") + r#"<WeaponSet1 nodes="20,30"/>"#),
        );
        assert!(!is_ordinary(&mixed, &f.parent));
        assert!(
            !is_ordinary(&mixed, &f.ordinary),
            "invalid Shared/Selected intermediates cannot supply a route"
        );
        assert!(is_ordinary(&mixed, &f.asc_paid));
        if scope == PointPoolScope::PerLoadout {
            assert!(matches!(
                allocation(&mixed, &f.attribute).scope,
                DraftField::Pending(_)
            ));
            let all_scoped = evaluate(
                &f,
                "1,2,10,20,21,30,40",
                &(overrides("10", "") + r#"<WeaponSet1 nodes="10,20,30"/>"#),
            );
            assert_eq!(count(&all_scoped), 4);
            let mut old = f.tree.input().clone();
            let Some(AllocationAccessPolicy::PobIndependentSavedPathsV2 { pools, nodes }) =
                old.content.access.take()
            else {
                unreachable!()
            };
            old.content.access =
                Some(AllocationAccessPolicy::PobIndependentSavedPathsV1 { pools, nodes });
            assert!(checked(&f, old).is_err());
        } else {
            assert!(is_ordinary(&mixed, &f.attribute));
            assert!(matches!(
                allocation(&mixed, &f.parent).scope,
                DraftField::Pending(_)
            ));
        }
    }
}

#[test]
fn v2_missing_equipment_bindings_preserve_shared_results_and_withhold_actual_overlays() {
    let a = prepared();
    let b = scoped_fixture();
    let plain = document(&spec("1,2,10,20,21,30,40", &overrides("10", "")), false);
    let before = run(&a, &plain, true, &a.base).unwrap();
    let after = run(&b, &plain, true, &b.base).unwrap();
    assert_eq!(count(&before), 4);
    assert_eq!(count(&after), 4);
    assert_eq!(before.draft().input(), after.draft().input());
    let mut sidecar = serde_json::to_value(before.sidecar()).unwrap();
    let next = serde_json::to_value(after.sidecar()).unwrap();
    sidecar["tree_policy"] = next["tree_policy"].clone();
    assert_eq!(sidecar, next);
    let xml = document(
        &spec(
            "1,2,10,20,21,30,40",
            &(overrides("10", "") + r#"<WeaponSet1 nodes="20,30"/>"#),
        ),
        false,
    );
    assert_eq!(count(&run(&b, &xml, true, &b.base).unwrap()), 0);
}

#[test]
fn v2_follows_injected_overlay_keys_and_keeps_saved_spec_proofs_separate() {
    let mut f = scoped_fixture();
    let mut input = f.tree.input().clone();
    input.content.syntax.weapon_overlays[0].loadout = key("two");
    input.content.syntax.weapon_overlays[1].loadout = key("one");
    f.tree = checked(&f, input).unwrap();
    let one = spec(
        "1,2,10,20,21,30,40",
        &(overrides("10", "") + r#"<WeaponSet1 nodes="20,30"/>"#),
    );
    let two = spec(
        "1,2,10,20,21,30,40",
        &(overrides("10", "") + r#"<WeaponSet1 nodes="30"/><WeaponSet2 nodes="20"/>"#),
    );
    let result = run(&f, &document(&(one + &two), true), true, &f.base).unwrap();
    let rows: Vec<_> = result
        .draft()
        .input()
        .allocations
        .members
        .iter()
        .filter(|r| r.node.to_resolved().as_ref() == Some(&f.ordinary))
        .collect();
    assert!(matches!(rows[0].access, DraftAllocationAccess::Ordinary));
    assert!(matches!(rows[1].access, DraftAllocationAccess::Pending(_)));
    assert_eq!(
        rows[0].scope, rows[1].scope,
        "scope identity alone cannot reuse another Spec's path"
    );
}

#[test]
fn v2_canonical_members_reject_duplicates_and_respect_compile_and_import_work_limits() {
    let f = scoped_fixture();
    let mut reversed = f.tree.input().clone();
    let Some(AllocationAccessPolicy::PobIndependentSavedPathsV2 { pools, nodes }) =
        &mut reversed.content.access
    else {
        unreachable!()
    };
    pools.reverse();
    nodes.reverse();
    assert_eq!(checked(&f, reversed).unwrap().identity(), f.tree.identity());
    for duplicate_pool in [false, true] {
        let mut duplicate = f.tree.input().clone();
        let Some(AllocationAccessPolicy::PobIndependentSavedPathsV2 { pools, nodes }) =
            &mut duplicate.content.access
        else {
            unreachable!()
        };
        if duplicate_pool {
            pools.push(pools[0].clone());
        } else {
            nodes.push(nodes[0].clone());
        }
        assert!(checked(&f, duplicate).is_err());
    }
    for limits in [
        TreePolicyLimits {
            max_schema_work: 1,
            ..Default::default()
        },
        TreePolicyLimits {
            max_collection_entries: 1,
            ..Default::default()
        },
    ] {
        assert!(f.tree.validate_limits(limits).is_err());
        assert!(
            OwnedTreeNormalizationPolicy::new(
                f.tree.input().clone(),
                &f.registry,
                &f.schema,
                &f.mapping,
                &f.base,
                limits
            )
            .is_err()
        );
    }
    let xml = document(
        &spec(
            "1,2,10,20,21,30,40",
            &(overrides("10", "") + r#"<WeaponSet1 nodes="20,30"/><WeaponSet2 nodes="40"/>"#),
        ),
        true,
    );
    let source = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([91; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let normalize = |max_work| {
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
                tree: Some(&f.tree),
            },
            &f.base,
            &[],
            NormalizationLimits {
                max_work,
                ..Default::default()
            },
        )
    };
    assert!(matches!(
        normalize(1),
        Err(NormalizationError::Limit("work"))
    ));
    let result = normalize(NormalizationLimits::default().max_work).unwrap();
    assert_eq!(
        count(&result),
        3,
        "the valid scoped path fits the bounded import"
    );
    assert!(
        matches!(
            allocation(&result, &f.asc_paid).scope,
            DraftField::Pending(_)
        ),
        "the Shared ascendancy pool still rejects the second-mode occurrence"
    );
    assert!(!is_ordinary(&result, &f.asc_paid));
}
