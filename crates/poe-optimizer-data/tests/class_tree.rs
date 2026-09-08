use poe_optimizer_core::candidate::{
    AscendancyLock, Candidate, CandidateBudgets, CandidateCatalog, CandidateConstraints,
    CandidateDomain, CandidateIssueCode, CandidateLocks, PassiveKind,
};
use poe_optimizer_data::{
    class_tree::{ClassTreeSelection, candidate_catalog, selections},
    game_data::{GameDataLoader, LoadLimits, TrustPolicy, bundled_snapshot},
};
use std::collections::{BTreeMap, BTreeSet};

fn candidate(catalog: &CandidateCatalog, selection: &ClassTreeSelection) -> Candidate {
    Candidate {
        catalog: catalog.identity.clone(),
        class_id: selection.class_id.to_string(),
        ascendancy_id: selection.ascendancy_id.clone(),
        passives: selection.entrance_node_id.into_iter().collect(),
        equipment: BTreeMap::new(),
        skills: BTreeMap::new(),
    }
}

#[test]
fn all_93_selections_resolve_deterministically_with_implicit_roots_and_selected_attributes() {
    let snapshot = bundled_snapshot().unwrap();
    let tree = snapshot.tree();
    let choices = selections(tree).unwrap();
    assert_eq!(choices, selections(tree).unwrap());
    assert_eq!(choices.len(), 93);
    assert!(choices.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(
        choices
            .iter()
            .filter(|choice| choice.entrance_node_id.is_none())
            .count(),
        31
    );
    for choice in choices {
        let resolved = choice.resolve(tree).unwrap();
        assert_eq!(resolved.selection, choice);
        assert_eq!(resolved.class.integer_id, choice.class_id);
        assert_eq!(
            resolved.base_attributes.strength,
            resolved.class.base_strength
        );
        assert_eq!(
            resolved.base_attributes.dexterity,
            resolved.class.base_dexterity
        );
        assert_eq!(
            resolved.base_attributes.intelligence,
            resolved.class.base_intelligence
        );
        assert!(
            resolved
                .implicit_roots
                .contains(&resolved.class.start_node_id)
        );
        assert_eq!(
            resolved.implicit_roots.len(),
            1 + usize::from(choice.ascendancy_id.is_some())
        );
        if let Some(ascendancy) = &resolved.ascendancy {
            assert_eq!(ascendancy.class_id, choice.class_id);
            assert_eq!(Some(&ascendancy.internal_id), choice.ascendancy_id.as_ref());
            assert!(resolved.implicit_roots.contains(&ascendancy.start_node_id));
        }
        assert_eq!(
            resolved.allocated_nodes.len(),
            resolved.implicit_roots.len() + usize::from(choice.entrance_node_id.is_some())
        );
        assert_eq!(
            resolved
                .allocated_nodes
                .difference(&resolved.implicit_roots)
                .copied()
                .collect::<Vec<_>>(),
            choice.entrance_node_id.into_iter().collect::<Vec<_>>()
        );
        assert_eq!(
            resolved
                .paid_node
                .as_ref()
                .map(|node| node.physical_node_id),
            choice.entrance_node_id
        );
    }
}

#[test]
fn shared_root_ownership_does_not_replace_class_specific_effects_or_physical_allocations() {
    let snapshot = bundled_snapshot().unwrap();
    let resolve = |class_id, ascendancy_id: Option<&str>, entrance_node_id| {
        ClassTreeSelection {
            class_id,
            ascendancy_id: ascendancy_id.map(str::to_owned),
            entrance_node_id,
        }
        .resolve(snapshot.tree())
        .unwrap()
    };
    let witch = resolve(1, None, Some(4739));
    let sorceress = resolve(7, None, Some(4739));
    assert_eq!(witch.implicit_roots, sorceress.implicit_roots);
    assert_eq!(witch.paid_node.as_ref().unwrap().effective_source_id, 17306);
    assert_eq!(
        sorceress.paid_node.as_ref().unwrap().effective_source_id,
        4739
    );
    assert_eq!(witch.allocated_nodes, BTreeSet::from([4739, 54447]));
    assert!(!witch.allocated_nodes.contains(&17306));
    let huntress = resolve(8, None, Some(56651));
    let ranger = resolve(2, None, Some(56651));
    assert_eq!(huntress.implicit_roots, ranger.implicit_roots);
    assert_eq!(
        huntress.paid_node.as_ref().unwrap().effective_source_id,
        39263
    );
    assert_eq!(
        ranger.paid_node.as_ref().unwrap().effective_source_id,
        56651
    );
    assert!(!huntress.allocated_nodes.contains(&39263));
    let lich = resolve(1, Some("Witch3"), None);
    let abyssal = resolve(1, Some("Witch3b"), None);
    assert_eq!(lich.implicit_roots, abyssal.implicit_roots);
    assert_ne!(
        lich.ascendancy.as_ref().unwrap().internal_id,
        abyssal.ascendancy.as_ref().unwrap().internal_id
    );
}

#[test]
fn selections_reject_cross_class_ascendancies_unowned_roots_and_effect_source_ids() {
    let snapshot = bundled_snapshot().unwrap();
    for choice in [
        ClassTreeSelection {
            class_id: 3,
            ascendancy_id: None,
            entrance_node_id: None,
        },
        ClassTreeSelection {
            class_id: 6,
            ascendancy_id: Some("Witch3".into()),
            entrance_node_id: None,
        },
        ClassTreeSelection {
            class_id: 1,
            ascendancy_id: Some(String::new()),
            entrance_node_id: None,
        },
        ClassTreeSelection {
            class_id: 6,
            ascendancy_id: None,
            entrance_node_id: Some(4739),
        },
        ClassTreeSelection {
            class_id: 1,
            ascendancy_id: None,
            entrance_node_id: Some(17306),
        },
        ClassTreeSelection {
            class_id: 8,
            ascendancy_id: None,
            entrance_node_id: Some(39263),
        },
        ClassTreeSelection {
            class_id: 6,
            ascendancy_id: None,
            entrance_node_id: Some(47175),
        },
        ClassTreeSelection {
            class_id: 1,
            ascendancy_id: Some("Witch3".into()),
            entrance_node_id: Some(23710),
        },
    ] {
        assert!(choice.resolve(snapshot.tree()).is_err(), "{choice:?}");
    }
}

#[test]
fn partial_graph_enforces_caller_point_budgets_connectivity_categories_and_independent_locks() {
    let snapshot = bundled_snapshot().unwrap();
    let catalog = candidate_catalog(&snapshot).unwrap();
    assert_eq!(catalog.classes.len(), 8);
    assert_eq!(catalog.ascendancies.len(), 23);
    assert_eq!(catalog.passive_nodes.len(), 40);
    assert!(catalog.unsupported_mechanics.is_empty());
    assert!(snapshot.tree().coverage.excluded_node_count > catalog.passive_nodes.len());
    assert!(
        !snapshot
            .tree()
            .coverage
            .source_dangling_connections
            .is_empty()
    );
    assert_eq!(
        catalog.passive_nodes[&54447].kind,
        PassiveKind::ClassStart {
            class_ids: BTreeSet::from(["1".into(), "7".into()])
        }
    );
    assert_eq!(
        catalog.passive_nodes[&23710].kind,
        PassiveKind::AscendancyStart {
            ascendancy_ids: BTreeSet::from(["Witch3".into(), "Witch3b".into()])
        }
    );
    assert!(catalog.passive_nodes.values().all(|node| {
        node.links
            .iter()
            .all(|target| catalog.passive_nodes.contains_key(target))
    }));
    for budget in [0, 1] {
        let domain = CandidateDomain::new(
            catalog.clone(),
            CandidateConstraints {
                budgets: CandidateBudgets {
                    ordinary_passive_points: budget,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();
        for choice in selections(snapshot.tree()).unwrap() {
            let report = domain.validate(&candidate(&catalog, &choice));
            assert_eq!(
                report.is_searchable(),
                budget == 1 || choice.entrance_node_id.is_none(),
                "{choice:?}: {report:?}"
            );
        }
    }
    let locked_choice = ClassTreeSelection {
        class_id: 1,
        ascendancy_id: Some("Witch3".into()),
        entrance_node_id: Some(4739),
    };
    let domain = CandidateDomain::new(
        catalog.clone(),
        CandidateConstraints {
            budgets: CandidateBudgets {
                ordinary_passive_points: 1,
                ..Default::default()
            },
            locks: CandidateLocks {
                class_id: Some("1".into()),
                ascendancy: Some(AscendancyLock::Id("Witch3".into())),
                allocated_passives: BTreeSet::from([4739]),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    let original = candidate(&catalog, &locked_choice);
    assert!(domain.validate(&original).is_searchable());
    for dimension in 0..3 {
        let mut changed = original.clone();
        match dimension {
            0 => changed.class_id = "7".into(),
            1 => changed.ascendancy_id = Some("Witch3b".into()),
            _ => {
                changed.passives.clear();
            }
        }
        assert!(
            domain
                .validate(&changed)
                .violations
                .iter()
                .any(|issue| issue.code == CandidateIssueCode::LockViolation)
        );
    }
    let mut other_class_entrance = original.clone();
    other_class_entrance.passives = BTreeSet::from([3936]);
    assert!(
        domain
            .validate(&other_class_entrance)
            .violations
            .iter()
            .any(|issue| issue.code == CandidateIssueCode::DisconnectedPassive)
    );
    let mut root_as_paid = original;
    root_as_paid.passives = BTreeSet::from([54447]);
    assert!(
        domain
            .validate(&root_as_paid)
            .violations
            .iter()
            .any(|issue| issue.code == CandidateIssueCode::WrongPointCategory)
    );
}

#[test]
fn graph_identity_binds_all_selected_data_even_when_tree_bytes_are_unchanged() {
    let reviewed = bundled_snapshot().unwrap();
    let same_content = GameDataLoader::from_bytes(
        &reviewed.package().canonical_bytes().unwrap(),
        &TrustPolicy::AllowCustom,
        &LoadLimits::default(),
    )
    .unwrap();
    let catalog = candidate_catalog(&reviewed).unwrap();
    assert_eq!(catalog, candidate_catalog(&same_content).unwrap());
    let mut package = reviewed.package().clone();
    package.spark.lightning_maximum += 1.0;
    package.refresh_section_digests().unwrap();
    let altered = GameDataLoader::from_bytes(
        &package.canonical_bytes().unwrap(),
        &TrustPolicy::AllowCustom,
        &LoadLimits::default(),
    )
    .unwrap();
    let altered_catalog = candidate_catalog(&altered).unwrap();
    assert_eq!(reviewed.tree(), altered.tree());
    assert_eq!(catalog.passive_nodes, altered_catalog.passive_nodes);
    assert_ne!(catalog.identity, altered_catalog.identity);
    let choice = ClassTreeSelection {
        class_id: 6,
        ascendancy_id: None,
        entrance_node_id: None,
    };
    let domain = CandidateDomain::new(altered_catalog, CandidateConstraints::default()).unwrap();
    assert!(
        domain
            .validate(&candidate(&catalog, &choice))
            .violations
            .iter()
            .any(|issue| issue.code == CandidateIssueCode::CatalogMismatch)
    );
}
