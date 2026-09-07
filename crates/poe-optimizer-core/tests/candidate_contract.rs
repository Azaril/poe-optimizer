use poe_optimizer_core::candidate::*;
use std::collections::{BTreeMap, BTreeSet};

fn strings(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn payload(content: &str) -> ExactPayload {
    ExactPayload {
        format: "fixture-text".into(),
        content: content.into(),
        sha256: "a".repeat(64),
    }
}

// These are finite contract fixtures, not a claim that their invented mechanics describe PoE.
fn fixture() -> (CandidateCatalog, CandidateConstraints, Candidate) {
    let identity = CatalogIdentity {
        schema_version: CANDIDATE_SCHEMA_VERSION,
        game: "poe2-contract-fixture".into(),
        rules_revision: "fixture-v1".into(),
        content_fingerprint: "b".repeat(64),
    };
    let mut passive_nodes = BTreeMap::new();
    for (class, root, ordinary, asc, asc_root, asc_node) in [
        ("class-a", 10, 11, "asc-a", 100, 101),
        ("class-b", 20, 21, "asc-b", 200, 201),
    ] {
        passive_nodes.insert(
            root,
            PassiveNode {
                kind: PassiveKind::ClassStart {
                    class_ids: strings(&[class]),
                },
                ..PassiveNode::default()
            },
        );
        // Only the child names the root: reverse-edge reachability is intentional.
        passive_nodes.insert(
            ordinary,
            PassiveNode {
                point_cost: 1,
                links: BTreeSet::from([root]),
                ..PassiveNode::default()
            },
        );
        passive_nodes.insert(
            asc_root,
            PassiveNode {
                kind: PassiveKind::AscendancyStart {
                    ascendancy_ids: strings(&[asc]),
                },
                ..PassiveNode::default()
            },
        );
        passive_nodes.insert(
            asc_node,
            PassiveNode {
                kind: PassiveKind::Ascendancy {
                    ascendancy_ids: strings(&[asc]),
                },
                point_cost: 1,
                links: BTreeSet::from([asc_root]),
                ..PassiveNode::default()
            },
        );
    }
    let mut items = BTreeMap::new();
    items.insert(
        "weapon-copy".into(),
        ItemInstance {
            definition_id: "mace".into(),
            payload: payload("Exact mace: physical 10-20, attack speed 1.4"),
            compatible_slots: strings(&["weapon"]),
            ..ItemInstance::default()
        },
    );
    for id in ["ring-copy-a", "ring-copy-b", "ring-copy-c"] {
        items.insert(
            id.into(),
            ItemInstance {
                definition_id: "ring".into(),
                payload: payload("Identical exact ring: fire resistance 20"),
                compatible_slots: strings(&["ring-1", "ring-2"]),
                ..ItemInstance::default()
            },
        );
    }
    let active_skills = ["attack", "aura", "buff-a", "buff-b"]
        .into_iter()
        .map(|id| {
            (
                format!("{id}-copy"),
                ActiveSkillInstance {
                    definition_id: id.into(),
                    payload: payload(id),
                    resource_costs: BTreeMap::from([("spirit".into(), 1)]),
                    ..ActiveSkillInstance::default()
                },
            )
        })
        .collect();
    let supports = ["support-a", "support-b", "support-copy-a"]
        .into_iter()
        .map(|id| {
            (
                id.into(),
                SupportInstance {
                    definition_id: if id == "support-copy-a" {
                        "support-a".into()
                    } else {
                        id.into()
                    },
                    payload: payload(id),
                    compatible_active_skill_ids: strings(&["attack", "aura", "buff-a", "buff-b"]),
                    resource_costs: BTreeMap::from([("strength-support-capacity".into(), 1)]),
                    ..SupportInstance::default()
                },
            )
        })
        .collect();
    let catalog = CandidateCatalog {
        identity: identity.clone(),
        classes: BTreeMap::from([
            ("class-a".into(), ClassDefinition { start_node_id: 10 }),
            ("class-b".into(), ClassDefinition { start_node_id: 20 }),
        ]),
        ascendancies: BTreeMap::from([
            (
                "asc-a".into(),
                AscendancyDefinition {
                    class_id: "class-a".into(),
                    start_node_id: 100,
                },
            ),
            (
                "asc-b".into(),
                AscendancyDefinition {
                    class_id: "class-b".into(),
                    start_node_id: 200,
                },
            ),
        ]),
        passive_nodes,
        equipment_slots: strings(&["weapon", "ring-1", "ring-2"]),
        skill_slots: strings(&["main", "aura", "buff", "empty"]),
        items,
        active_skills,
        supports,
        support_definition_limits: BTreeMap::from([("support-a".into(), 1)]),
        unsupported_mechanics: BTreeSet::new(),
    };
    let constraints = CandidateConstraints {
        required_skill_ids: strings(&["attack", "aura"]),
        required_item_instance_ids: strings(&["weapon-copy", "ring-copy-a"]),
        budgets: CandidateBudgets {
            ordinary_passive_points: 1,
            ascendancy_passive_points: 1,
            active_skill_count: 3,
            supports_per_skill: 2,
            resource_limits: BTreeMap::from([
                ("spirit".into(), 3),
                ("strength-support-capacity".into(), 2),
            ]),
        },
        ..CandidateConstraints::default()
    };
    let candidate = Candidate {
        catalog: identity,
        class_id: "class-a".into(),
        ascendancy_id: Some("asc-a".into()),
        passives: BTreeSet::from([11, 101]),
        equipment: BTreeMap::from([
            ("weapon".into(), "weapon-copy".into()),
            ("ring-1".into(), "ring-copy-a".into()),
            ("ring-2".into(), "ring-copy-b".into()),
        ]),
        skills: BTreeMap::from([
            (
                "main".into(),
                SkillAssignment {
                    active_instance_id: "attack-copy".into(),
                    support_instance_ids: strings(&["support-a"]),
                },
            ),
            (
                "aura".into(),
                SkillAssignment {
                    active_instance_id: "aura-copy".into(),
                    ..SkillAssignment::default()
                },
            ),
            (
                "buff".into(),
                SkillAssignment {
                    active_instance_id: "buff-a-copy".into(),
                    ..SkillAssignment::default()
                },
            ),
        ]),
    };
    (catalog, constraints, candidate)
}

fn has(report: &CandidateValidation, code: CandidateIssueCode) -> bool {
    report.violations.iter().any(|issue| issue.code == code)
}

#[test]
fn all_six_dimensions_change_together_while_two_skills_and_two_items_remain_required() {
    let (catalog, constraints, candidate) = fixture();
    let domain = CandidateDomain::new(catalog, constraints).unwrap();
    assert!(domain.validate(&candidate).is_searchable());
    let mut next = candidate.clone();
    next.class_id = "class-b".into();
    next.ascendancy_id = Some("asc-b".into());
    next.passives = BTreeSet::from([21, 201]);
    next.equipment.insert("ring-2".into(), "ring-copy-c".into());
    next.skills.get_mut("main").unwrap().support_instance_ids = strings(&["support-b"]);
    next.skills.get_mut("buff").unwrap().active_instance_id = "buff-b-copy".into();
    assert!(
        domain.validate(&next).is_searchable(),
        "{:?}",
        domain.validate(&next)
    );
    assert_ne!(candidate, next);
    next.skills.remove("aura");
    next.equipment.remove("ring-1");
    let report = domain.validate(&next);
    assert_eq!(
        report
            .violations
            .iter()
            .filter(|issue| issue.code == CandidateIssueCode::MissingRequirement)
            .count(),
        2
    );
}

#[test]
fn canonical_choices_deduplicate_regardless_of_insertion_order() {
    let (_, _, candidate) = fixture();
    let mut reordered = candidate.clone();
    reordered.passives = candidate.passives.iter().rev().copied().collect();
    reordered.equipment = candidate
        .equipment
        .iter()
        .rev()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    reordered.skills = candidate
        .skills
        .iter()
        .rev()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    assert_eq!(candidate, reordered);
    assert_eq!(
        serde_json::to_string(&candidate).unwrap(),
        serde_json::to_string(&reordered).unwrap()
    );
    assert_eq!(BTreeSet::from([candidate, reordered]).len(), 1);
}

#[test]
fn cross_class_changes_cannot_repair_away_locked_tree_ascendancy_or_items() {
    let (catalog, mut constraints, candidate) = fixture();
    constraints.locks.ascendancy = Some(AscendancyLock::Id("asc-a".into()));
    constraints.locks.allocated_passives.insert(11);
    constraints
        .locks
        .equipment
        .insert("ring-2".into(), Some("ring-copy-b".into()));
    let domain = CandidateDomain::new(catalog, constraints).unwrap();
    assert!(domain.validate(&candidate).is_searchable());
    let mut next = candidate.clone();
    next.class_id = "class-b".into();
    assert!(has(
        &domain.validate(&next),
        CandidateIssueCode::IncompatibleClass
    ));
    assert!(has(
        &domain.validate(&next),
        CandidateIssueCode::DisconnectedPassive
    ));
    next.ascendancy_id = Some("asc-b".into());
    next.passives = BTreeSet::from([21, 201]);
    next.equipment.insert("ring-2".into(), "ring-copy-c".into());
    let before = next.clone();
    let report = domain.validate(&next);
    assert_eq!(
        report
            .violations
            .iter()
            .filter(|issue| issue.code == CandidateIssueCode::LockViolation)
            .count(),
        3
    );
    assert_eq!(next, before, "Validation must not mutate or relax locks");
}

#[test]
fn support_and_active_locks_are_independent_and_can_keep_slots_empty() {
    let (catalog, mut constraints, mut candidate) = fixture();
    constraints.locks.skill_groups.insert(
        "main".into(),
        SkillGroupLocks {
            required_support_instance_ids: strings(&["support-a"]),
            forbidden_support_instance_ids: strings(&["support-b"]),
            ..SkillGroupLocks::default()
        },
    );
    constraints.locks.skill_groups.insert(
        "buff".into(),
        SkillGroupLocks {
            active_instance_id: Some("buff-a-copy".into()),
            exact_support_instance_ids: Some(BTreeSet::new()),
            ..SkillGroupLocks::default()
        },
    );
    constraints.locks.empty_skill_groups.insert("empty".into());
    let domain = CandidateDomain::new(catalog, constraints).unwrap();
    assert!(domain.validate(&candidate).is_searchable());
    candidate
        .skills
        .get_mut("main")
        .unwrap()
        .support_instance_ids = strings(&["support-b"]);
    candidate.skills.get_mut("buff").unwrap().active_instance_id = "buff-b-copy".into();
    candidate.skills.insert(
        "empty".into(),
        SkillAssignment {
            active_instance_id: "buff-a-copy".into(),
            ..SkillAssignment::default()
        },
    );
    assert_eq!(
        domain
            .validate(&candidate)
            .violations
            .iter()
            .filter(|issue| issue.code == CandidateIssueCode::LockViolation)
            .count(),
        3
    );
}

#[test]
fn reverse_edges_connect_but_point_categories_and_ownership_remain_separate() {
    let (catalog, mut constraints, mut candidate) = fixture();
    constraints.budgets.ascendancy_passive_points = 0;
    constraints.budgets.ordinary_passive_points = 100;
    let domain = CandidateDomain::new(catalog, constraints).unwrap();
    let report = domain.validate(&candidate);
    assert_eq!(report.violations.len(), 1);
    assert_eq!(report.violations[0].path, "ascendancy_passive_points");
    candidate.passives = BTreeSet::from([10, 201]);
    let report = domain.validate(&candidate);
    assert!(has(&report, CandidateIssueCode::WrongPointCategory));
    assert!(has(&report, CandidateIssueCode::IncompatibleClass));
}

#[test]
fn identical_payloads_are_distinct_instances_but_one_copy_cannot_fill_two_slots() {
    let (catalog, constraints, mut candidate) = fixture();
    assert_eq!(
        catalog.items["ring-copy-a"].payload,
        catalog.items["ring-copy-b"].payload
    );
    let domain = CandidateDomain::new(catalog, constraints).unwrap();
    assert!(domain.validate(&candidate).is_searchable());
    candidate
        .equipment
        .insert("ring-2".into(), "ring-copy-a".into());
    assert!(has(
        &domain.validate(&candidate),
        CandidateIssueCode::DuplicateInstance
    ));
    candidate
        .equipment
        .insert("weapon".into(), "ring-copy-c".into());
    assert!(has(
        &domain.validate(&candidate),
        CandidateIssueCode::IncompatibleSlot
    ));
}

#[test]
fn support_compatibility_multiplicity_and_resource_limits_are_explicit() {
    let (mut catalog, mut constraints, mut candidate) = fixture();
    catalog
        .supports
        .get_mut("support-b")
        .unwrap()
        .compatible_active_skill_ids = strings(&["aura"]);
    constraints.budgets.resource_limits.remove("spirit");
    candidate
        .skills
        .get_mut("main")
        .unwrap()
        .support_instance_ids = strings(&["support-a", "support-b"]);
    candidate
        .skills
        .get_mut("aura")
        .unwrap()
        .support_instance_ids = strings(&["support-copy-a"]);
    let domain = CandidateDomain::new(catalog, constraints).unwrap();
    let report = domain.validate(&candidate);
    assert!(has(&report, CandidateIssueCode::IncompatibleSupport));
    assert!(has(&report, CandidateIssueCode::MissingResourceBudget));
    assert!(
        report
            .violations
            .iter()
            .any(|issue| issue.path == "support_definition_limits.support-a")
    );
    assert!(
        report
            .violations
            .iter()
            .any(|issue| issue.path == "resources.strength-support-capacity")
    );
}

#[test]
fn global_and_selected_unknown_mechanics_block_search_without_claiming_bad_numeric_rules() {
    let (mut catalog, constraints, candidate) = fixture();
    catalog
        .unsupported_mechanics
        .insert("unverified-weapon-set-topology".into());
    catalog
        .items
        .get_mut("ring-copy-a")
        .unwrap()
        .unsupported_mechanics
        .insert("conditional-reservation".into());
    catalog
        .items
        .get_mut("ring-copy-c")
        .unwrap()
        .unsupported_mechanics
        .insert("unused-item-mechanic".into());
    let domain = CandidateDomain::new(catalog, constraints).unwrap();
    let report = domain.validate(&candidate);
    assert!(report.is_valid_within_catalog());
    assert!(!report.is_searchable());
    assert_eq!(
        report.unsupported_mechanics,
        strings(&["unverified-weapon-set-topology", "conditional-reservation"])
    );
}

#[test]
fn granted_skill_requires_its_exact_item_and_passive_owner() {
    let (mut catalog, constraints, mut candidate) = fixture();
    catalog
        .active_skills
        .get_mut("buff-a-copy")
        .unwrap()
        .availability = Availability {
        class_ids: strings(&["class-a"]),
        ascendancy_ids: strings(&["asc-a"]),
        required_item_instance_ids: strings(&["ring-copy-b"]),
        required_passives: BTreeSet::from([101]),
    };
    let domain = CandidateDomain::new(catalog, constraints).unwrap();
    assert!(domain.validate(&candidate).is_searchable());
    candidate.equipment.remove("ring-2");
    candidate.passives.remove(&101);
    let report = domain.validate(&candidate);
    assert_eq!(
        report
            .violations
            .iter()
            .filter(|issue| issue.code == CandidateIssueCode::MissingRequirement)
            .count(),
        2
    );
}

#[test]
fn unknown_catalog_edges_payloads_owners_and_constraints_are_rejected_before_use() {
    let (mut catalog, mut constraints, _) = fixture();
    catalog
        .passive_nodes
        .get_mut(&11)
        .unwrap()
        .links
        .insert(999);
    catalog.items.get_mut("weapon-copy").unwrap().payload.sha256 = "invalid-digest".into();
    catalog.ascendancies.get_mut("asc-a").unwrap().class_id = "missing-class".into();
    constraints
        .required_skill_ids
        .insert("missing-skill".into());
    constraints.locks.allocated_passives.insert(11);
    constraints.locks.unallocated_passives.insert(11);
    constraints.locks.skill_groups.insert(
        "main".into(),
        SkillGroupLocks {
            required_support_instance_ids: strings(&["support-a"]),
            exact_support_instance_ids: Some(BTreeSet::new()),
            ..SkillGroupLocks::default()
        },
    );
    let error = CandidateDomain::new(catalog, constraints).unwrap_err();
    assert!(
        error
            .issues
            .iter()
            .filter(|issue| issue.code == CandidateIssueCode::InvalidCatalog)
            .count()
            >= 3
    );
    assert!(
        error
            .issues
            .iter()
            .filter(|issue| issue.code == CandidateIssueCode::InvalidConstraints)
            .count()
            >= 3
    );
}

#[test]
fn candidate_version_and_unknown_references_fail_closed() {
    let (catalog, constraints, mut candidate) = fixture();
    let domain = CandidateDomain::new(catalog, constraints).unwrap();
    candidate.catalog.rules_revision = "another-ruleset".into();
    assert_eq!(
        domain.validate(&candidate).violations[0].code,
        CandidateIssueCode::CatalogMismatch
    );
    candidate.catalog = domain.catalog().identity.clone();
    candidate.passives.insert(999);
    candidate.skills.get_mut("main").unwrap().active_instance_id = "unknown".into();
    assert!(has(
        &domain.validate(&candidate),
        CandidateIssueCode::UnknownReference
    ));
}

#[test]
fn no_ascendancy_lock_round_trips_distinctly_from_an_unlocked_dimension() {
    let unlocked = CandidateLocks::default();
    let locked = CandidateLocks {
        ascendancy: Some(AscendancyLock::None),
        ..CandidateLocks::default()
    };
    let unlocked_json = serde_json::to_string(&unlocked).unwrap();
    let locked_json = serde_json::to_string(&locked).unwrap();
    assert_ne!(unlocked_json, locked_json);
    assert_eq!(
        serde_json::from_str::<CandidateLocks>(&locked_json).unwrap(),
        locked
    );
    let (catalog, mut constraints, mut candidate) = fixture();
    constraints.locks = locked;
    let domain = CandidateDomain::new(catalog, constraints).unwrap();
    assert!(has(
        &domain.validate(&candidate),
        CandidateIssueCode::LockViolation
    ));
    candidate.ascendancy_id = None;
    candidate.passives.remove(&101);
    assert!(domain.validate(&candidate).is_searchable());
}

// A structural subset of pinned 0_5: shared physical starts and a switched Lich node.
// Calculated stats and the class/ascendancy-specific override payloads remain unmodeled.
fn shared_roots_fixture() -> (CandidateCatalog, CandidateConstraints, Candidate) {
    let (mut catalog, constraints, mut candidate) = fixture();
    catalog.classes = BTreeMap::from([
        (
            "1".into(),
            ClassDefinition {
                start_node_id: 54447,
            },
        ),
        (
            "7".into(),
            ClassDefinition {
                start_node_id: 54447,
            },
        ),
    ]);
    catalog.ascendancies = BTreeMap::from([
        (
            "Witch3".into(),
            AscendancyDefinition {
                class_id: "1".into(),
                start_node_id: 23710,
            },
        ),
        (
            "Witch3b".into(),
            AscendancyDefinition {
                class_id: "1".into(),
                start_node_id: 23710,
            },
        ),
    ]);
    catalog.passive_nodes = BTreeMap::from([
        (
            54447,
            PassiveNode {
                kind: PassiveKind::ClassStart {
                    class_ids: strings(&["1", "7"]),
                },
                links: BTreeSet::from([4739]),
                ..PassiveNode::default()
            },
        ),
        (
            4739,
            PassiveNode {
                point_cost: 1,
                unsupported_mechanics: strings(&["class-override-payload-unresolved"]),
                ..PassiveNode::default()
            },
        ),
        (
            23710,
            PassiveNode {
                kind: PassiveKind::AscendancyStart {
                    ascendancy_ids: strings(&["Witch3", "Witch3b"]),
                },
                links: BTreeSet::from([58751]),
                ..PassiveNode::default()
            },
        ),
        (
            58751,
            PassiveNode {
                kind: PassiveKind::Ascendancy {
                    ascendancy_ids: strings(&["Witch3", "Witch3b"]),
                },
                point_cost: 1,
                unsupported_mechanics: strings(&["ascendancy-override-payload-unresolved"]),
                ..PassiveNode::default()
            },
        ),
    ]);
    candidate.class_id = "1".into();
    candidate.ascendancy_id = Some("Witch3".into());
    candidate.passives = BTreeSet::from([4739, 58751]);
    (catalog, constraints, candidate)
}

#[test]
fn shared_physical_starts_and_paid_nodes_preserve_distinct_class_and_ascendancy_identity() {
    let (catalog, constraints, candidate) = shared_roots_fixture();
    let domain = CandidateDomain::new(catalog, constraints).unwrap();
    let report = domain.validate(&candidate);
    assert!(report.is_valid_within_catalog(), "{report:?}");
    assert!(
        !report.is_searchable(),
        "Override coverage is still explicitly unknown"
    );
    let mut variant = candidate.clone();
    variant.ascendancy_id = Some("Witch3b".into());
    assert!(domain.validate(&variant).is_valid_within_catalog());
    assert_ne!(
        candidate, variant,
        "A shared graph location does not merge ascendancy choices"
    );
    assert_eq!(
        candidate.passives, variant.passives,
        "The physical allocated node IDs remain stable"
    );
    assert_eq!(
        domain.catalog().ascendancies["Witch3"].start_node_id,
        domain.catalog().ascendancies["Witch3b"].start_node_id
    );
    variant.class_id = "7".into();
    assert!(
        has(
            &domain.validate(&variant),
            CandidateIssueCode::IncompatibleClass
        ),
        "Sharing Witch's physical root cannot grant Sorceress a Witch ascendancy"
    );
    variant.ascendancy_id = None;
    variant.passives.remove(&58751);
    let report = domain.validate(&variant);
    assert!(report.is_valid_within_catalog(), "{report:?}");
    assert_eq!(
        report.unsupported_mechanics,
        strings(&["class-override-payload-unresolved"])
    );
    assert_eq!(
        domain.catalog().classes["1"].start_node_id,
        domain.catalog().classes["7"].start_node_id
    );
    assert_eq!(
        domain.catalog().passive_nodes.len(),
        4,
        "Shared locations are not synthetic duplicate nodes"
    );
}

#[test]
fn shared_owner_sets_must_be_nonempty_known_and_match_root_references_both_ways() {
    let cases: [fn(&mut CandidateCatalog); 9] = [
        |catalog| {
            catalog.passive_nodes.get_mut(&54447).unwrap().kind = PassiveKind::ClassStart {
                class_ids: BTreeSet::new(),
            }
        },
        |catalog| {
            catalog.passive_nodes.get_mut(&23710).unwrap().kind = PassiveKind::AscendancyStart {
                ascendancy_ids: BTreeSet::new(),
            }
        },
        |catalog| {
            catalog.passive_nodes.get_mut(&58751).unwrap().kind = PassiveKind::Ascendancy {
                ascendancy_ids: BTreeSet::new(),
            }
        },
        |catalog| {
            catalog.passive_nodes.get_mut(&54447).unwrap().kind = PassiveKind::ClassStart {
                class_ids: strings(&["1", "7", "unknown-class"]),
            }
        },
        |catalog| {
            catalog.passive_nodes.get_mut(&23710).unwrap().kind = PassiveKind::AscendancyStart {
                ascendancy_ids: strings(&["Witch3", "Witch3b", "unknown-ascendancy"]),
            }
        },
        |catalog| {
            catalog.passive_nodes.get_mut(&58751).unwrap().kind = PassiveKind::Ascendancy {
                ascendancy_ids: strings(&["unknown-ascendancy"]),
            }
        },
        // Forward membership: a definition points at a root that omits it.
        |catalog| {
            catalog.passive_nodes.get_mut(&54447).unwrap().kind = PassiveKind::ClassStart {
                class_ids: strings(&["1"]),
            }
        },
        |catalog| {
            catalog.passive_nodes.get_mut(&23710).unwrap().kind = PassiveKind::AscendancyStart {
                ascendancy_ids: strings(&["Witch3"]),
            }
        },
        // Reverse membership: all definitions point to valid roots, but an old root
        // falsely retains a class that moved to another physical start.
        |catalog| {
            catalog.classes.get_mut("7").unwrap().start_node_id = 999;
            catalog.passive_nodes.insert(
                999,
                PassiveNode {
                    kind: PassiveKind::ClassStart {
                        class_ids: strings(&["7"]),
                    },
                    ..PassiveNode::default()
                },
            );
        },
    ];
    for (index, mutate) in cases.into_iter().enumerate() {
        let (mut catalog, constraints, _) = shared_roots_fixture();
        mutate(&mut catalog);
        let error = CandidateDomain::new(catalog, constraints).unwrap_err();
        assert!(
            error
                .issues
                .iter()
                .any(|issue| issue.code == CandidateIssueCode::InvalidCatalog),
            "case {index}: {error:?}"
        );
    }
    let (mut catalog, constraints, _) = shared_roots_fixture();
    catalog
        .ascendancies
        .get_mut("Witch3b")
        .unwrap()
        .start_node_id = 999;
    catalog.passive_nodes.insert(
        999,
        PassiveNode {
            kind: PassiveKind::AscendancyStart {
                ascendancy_ids: strings(&["Witch3b"]),
            },
            ..PassiveNode::default()
        },
    );
    let error = CandidateDomain::new(catalog, constraints).unwrap_err();
    assert!(
        error
            .issues
            .iter()
            .any(|issue| issue.path == "passive_nodes.23710"),
        "The old shared root must not retain a moved ascendancy: {error:?}"
    );
}
