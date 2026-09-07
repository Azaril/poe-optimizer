use poe_optimizer_core::candidate::{
    Candidate, CandidateBudgets, CandidateConstraints, CandidateDomain, CandidateIssueCode,
    PassiveKind,
};
use poe_optimizer_pob::{
    tree_data::{SourceTable, SourceValue, TreeDataSnapshot, TreeNodeKind, extract_pinned_tree},
    tree_projection::{AuthenticatedTreeSnapshot, TreeProjection, TreeProjectionError},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
    process::{Command, Stdio},
    sync::OnceLock,
    thread,
    time::{Duration, Instant},
};

#[derive(Serialize, Deserialize)]
struct Extracted {
    snapshot: TreeDataSnapshot,
    digest: String,
}
fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}
fn source() -> &'static AuthenticatedTreeSnapshot {
    static SOURCE: OnceLock<AuthenticatedTreeSnapshot> = OnceLock::new();
    SOURCE.get_or_init(|| {
        let scratch = tempfile::tempdir().unwrap();
        let output = scratch.path().join("snapshot.json");
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--ignored",
                "--exact",
                "projection_extraction_child",
                "--nocapture",
            ])
            .env("POE_PROJECTION_TEST_OUTPUT", &output)
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let mut child = command.spawn().unwrap();
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "projection source child failed");
                break;
            }
            if Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("projection source child timed out");
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert!(fs::metadata(&output).unwrap().len() < 64 * 1024 * 1024);
        let extracted: Extracted = serde_json::from_slice(&fs::read(output).unwrap()).unwrap();
        // The producer is our isolated, trusted child running verified source;
        // its private result is not an externally supplied self-asserted digest.
        AuthenticatedTreeSnapshot::from_trusted_digest(extracted.snapshot, &extracted.digest)
            .unwrap()
    })
}
#[test]
#[ignore = "trusted extraction child invoked by projection tests"]
fn projection_extraction_child() {
    let snapshot =
        extract_pinned_tree(&repository().join("vendor/path-of-building-poe2"), "0_5").unwrap();
    let digest = snapshot.sha256().unwrap();
    fs::write(
        std::env::var_os("POE_PROJECTION_TEST_OUTPUT").unwrap(),
        serde_json::to_vec(&Extracted { snapshot, digest }).unwrap(),
    )
    .unwrap();
}
fn projection(ids: &[u32]) -> TreeProjection {
    TreeProjection::new(source(), ids.iter().copied().collect()).unwrap()
}
fn candidate(
    projection: &TreeProjection,
    class: &str,
    asc: Option<&str>,
    passives: &[u32],
) -> Candidate {
    Candidate {
        catalog: projection.catalog().identity.clone(),
        class_id: class.into(),
        ascendancy_id: asc.map(str::to_owned),
        passives: passives.iter().copied().collect(),
        equipment: BTreeMap::new(),
        skills: BTreeMap::new(),
    }
}
fn domain(projection: &TreeProjection, ordinary: u32, ascendancy: u32) -> CandidateDomain {
    CandidateDomain::new(
        projection.catalog().clone(),
        CandidateConstraints {
            budgets: CandidateBudgets {
                ordinary_passive_points: ordinary,
                ascendancy_passive_points: ascendancy,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap()
}
// Deliberately trusted malformed producer output for projection contract tests;
// an attacker cannot replace an independently trusted expected digest this way.
fn producer_fixture(snapshot: TreeDataSnapshot) -> AuthenticatedTreeSnapshot {
    let digest = snapshot.sha256().unwrap();
    AuthenticatedTreeSnapshot::from_trusted_digest(snapshot, &digest).unwrap()
}

#[test]
fn all_class_and_ascendancy_identities_keep_shared_physical_root_ownership() {
    let projected = projection(&[4739, 56651, 58751]);
    let catalog = projected.catalog();
    assert_eq!(catalog.classes.len(), 8);
    assert_eq!(catalog.ascendancies.len(), 23);
    assert_eq!(catalog.passive_nodes.len(), 31); // 28 implicit roots + three paid nodes
    assert!(
        matches!(&catalog.passive_nodes[&54447].kind, PassiveKind::ClassStart { class_ids } if class_ids == &BTreeSet::from(["1".into(), "7".into()]))
    );
    assert!(
        matches!(&catalog.passive_nodes[&50459].kind, PassiveKind::ClassStart { class_ids } if class_ids == &BTreeSet::from(["2".into(), "8".into()]))
    );
    for id in [23710, 58751] {
        let owners = match &catalog.passive_nodes[&id].kind {
            PassiveKind::AscendancyStart { ascendancy_ids }
            | PassiveKind::Ascendancy { ascendancy_ids } => ascendancy_ids,
            _ => panic!("wrong point category"),
        };
        assert_eq!(owners, &BTreeSet::from(["Witch3".into(), "Witch3b".into()]));
    }
    let rules = domain(&projected, 0, 0);
    for class in catalog.classes.keys() {
        assert!(
            rules
                .validate(&candidate(&projected, class, None, &[]))
                .is_searchable()
        );
    }
    for (id, ascendancy) in &catalog.ascendancies {
        assert!(
            rules
                .validate(&candidate(&projected, &ascendancy.class_id, Some(id), &[]))
                .is_searchable()
        );
    }
    assert!(
        !rules
            .validate(&candidate(&projected, "7", Some("Warrior1"), &[]))
            .is_valid_within_catalog()
    );
    assert_eq!(
        projected
            .snapshot()
            .effective_node(1, None, 4739)
            .unwrap()
            .effective_source_id,
        17306
    );
    assert_eq!(
        projected
            .snapshot()
            .effective_node(8, None, 56651)
            .unwrap()
            .effective_source_id,
        39263
    );
    assert_eq!(
        projected
            .snapshot()
            .effective_node(1, Some("Witch3b"), 58751)
            .unwrap()
            .effective_source_id,
        35941
    );
}

#[test]
fn induced_graph_keeps_both_endpoints_and_never_repairs_missing_connectors() {
    let ids = [
        3936, 38646, 4739, 44871, 13828, 56651, 59779, 59915, 10364, 52980, 13855, 50084,
    ];
    let projected = projection(&ids);
    for (id, node) in &projected.catalog().passive_nodes {
        for adjacent in &node.links {
            assert!(
                projected.catalog().passive_nodes[adjacent]
                    .links
                    .contains(id)
            );
            assert!(projected.snapshot().nodes[id].adjacent.contains(adjacent));
        }
        assert_eq!(
            node.links,
            projected.snapshot().nodes[id]
                .adjacent
                .iter()
                .filter(|target| projected.catalog().passive_nodes.contains_key(target))
                .copied()
                .collect()
        );
    }
    assert!(!projected.source_coverage().boundary_edges.is_empty());
    for (left, right) in &projected.source_coverage().boundary_edges {
        assert_ne!(
            projected.catalog().passive_nodes.contains_key(left),
            projected.catalog().passive_nodes.contains_key(right)
        );
    }
    let rules = domain(&projected, 1, 0);
    for (class_id, class) in &projected.snapshot().classes {
        for entrance in projected.snapshot().ordinary_entrances(*class_id).unwrap() {
            assert!(
                rules
                    .validate(&candidate(
                        &projected,
                        &class.integer_id.to_string(),
                        None,
                        &[entrance]
                    ))
                    .is_searchable()
            );
        }
    }
    let disconnected = projection(&[18845]); // reaches the root only through unselected4739
    let validation =
        domain(&disconnected, 1, 0).validate(&candidate(&disconnected, "7", None, &[18845]));
    assert!(
        validation
            .violations
            .iter()
            .any(|issue| issue.code == CandidateIssueCode::DisconnectedPassive)
    );
    assert!(!disconnected.catalog().passive_nodes.contains_key(&4739));
}

#[test]
fn explicit_budgets_distinguish_paid_nodes_from_implicit_starts() {
    let projected = projection(&[4739, 58751]);
    assert_eq!(projected.catalog().passive_nodes[&54447].point_cost, 0);
    assert_eq!(projected.catalog().passive_nodes[&23710].point_cost, 0);
    assert_eq!(projected.catalog().passive_nodes[&4739].point_cost, 1);
    assert_eq!(projected.catalog().passive_nodes[&58751].point_cost, 1);
    let both = candidate(&projected, "1", Some("Witch3b"), &[4739, 58751]);
    assert!(domain(&projected, 1, 1).validate(&both).is_searchable());
    for (ordinary, ascendancy) in [(0, 1), (1, 0)] {
        assert!(
            domain(&projected, ordinary, ascendancy)
                .validate(&both)
                .violations
                .iter()
                .any(|issue| issue.code == CandidateIssueCode::BudgetExceeded)
        );
    }
    let paid_root = candidate(&projected, "1", Some("Witch3b"), &[54447]);
    assert!(
        domain(&projected, 1, 1)
            .validate(&paid_root)
            .violations
            .iter()
            .any(|issue| issue.code == CandidateIssueCode::WrongPointCategory)
    );
}

#[test]
fn missing_roots_special_nodes_and_unreviewed_node_kinds_fail_closed() {
    for id in [5162, 54447, 23710] {
        assert!(
            matches!(TreeProjection::new(source(), BTreeSet::from([id])), Err(TreeProjectionError::UnsupportedNode { node_id, .. }) if node_id == id)
        );
    }
    for kind in [
        TreeNodeKind::ImageOnly,
        TreeNodeKind::Socket,
        TreeNodeKind::Notable,
        TreeNodeKind::Keystone,
    ] {
        let node = source()
            .snapshot()
            .nodes
            .values()
            .find(|node| node.kind == kind)
            .unwrap();
        assert!(TreeProjection::new(source(), BTreeSet::from([node.id])).is_err());
    }
    for mechanic in ["attribute_choice", "unlock_constraint"] {
        let node = source()
            .snapshot()
            .nodes
            .values()
            .find(|node| node.unsupported_mechanics.contains(mechanic))
            .unwrap();
        assert!(TreeProjection::new(source(), BTreeSet::from([node.id])).is_err());
    }
}

#[test]
fn unknown_effective_overlay_fields_do_not_silently_narrow_class_scope() {
    let mut changed = source().snapshot().clone();
    changed
        .nodes
        .get_mut(&4739)
        .unwrap()
        .automatic_overrides
        .get_mut("Witch")
        .unwrap()
        .named
        .insert(
            "connections".into(),
            SourceValue::Table(SourceTable::default()),
        );
    let changed = producer_fixture(changed);
    let error = TreeProjection::new(&changed, BTreeSet::from([4739]))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("unsupported effective overlay field connections"),
        "{error}"
    );
    // Selecting another node retains every class; no class disappears to hide a bad choice.
    let independent = TreeProjection::new(&changed, BTreeSet::from([56651])).unwrap();
    assert_eq!(independent.catalog().classes.len(), 8);
    let mut changed = source().snapshot().clone();
    changed
        .nodes
        .get_mut(&4739)
        .unwrap()
        .automatic_overrides
        .get_mut("Witch")
        .unwrap()
        .named
        .insert(
            "ascendancyName".into(),
            SourceValue::String("Stormweaver".into()),
        );
    assert!(
        TreeProjection::new(&producer_fixture(changed), BTreeSet::from([4739]))
            .unwrap_err()
            .to_string()
            .contains("ownership")
    );
}

#[test]
fn trusted_digest_authenticates_contents_and_identity_labels_alone_do_not() {
    let snapshot = source().snapshot().clone();
    assert!(
        AuthenticatedTreeSnapshot::from_trusted_digest(snapshot.clone(), &"0".repeat(64)).is_err()
    );
    assert!(
        AuthenticatedTreeSnapshot::from_trusted_digest(snapshot.clone(), "not a digest").is_err()
    );
    let mut changed = snapshot.clone();
    changed.nodes.get_mut(&4739).unwrap().name = "changed content with unchanged identity".into();
    changed.validate_source_identity().unwrap();
    assert!(
        AuthenticatedTreeSnapshot::from_trusted_digest(changed, source().content_sha256())
            .unwrap_err()
            .to_string()
            .contains("contents")
    );
    let mut stale = snapshot;
    stale.identity.tree_version = "0_4".into();
    let stale_digest = stale.sha256().unwrap();
    assert!(AuthenticatedTreeSnapshot::from_trusted_digest(stale, &stale_digest).is_err());
}

#[test]
fn projection_identity_and_coverage_bind_the_finite_supported_scope() {
    let projected = projection(&[4739, 58751]);
    let reversed = projection(&[58751, 4739]);
    assert_eq!(projected.catalog(), reversed.catalog());
    assert_ne!(
        projected.catalog().identity,
        projection(&[4739]).catalog().identity
    );
    assert_eq!(
        projected.source_coverage().snapshot_sha256,
        source().content_sha256()
    );
    assert_eq!(
        projected
            .source_coverage()
            .source_dangling_connections
            .len(),
        14
    );
    assert!(
        projected
            .source_coverage()
            .source_unsupported_mechanics
            .contains("live_game_topology_completeness")
    );
    assert!(
        projected
            .source_coverage()
            .source_unsupported_mechanics
            .contains("stat_and_modifier_translation")
    );
    assert!(projected.catalog().unsupported_mechanics.is_empty());
    assert!(
        domain(&projected, 1, 1)
            .validate(&candidate(&projected, "1", Some("Witch3b"), &[4739, 58751]))
            .is_searchable()
    );
    assert_eq!(
        projected.source_coverage().projected_node_count
            + projected.source_coverage().unselected_node_count,
        source().snapshot().nodes.len()
    );
    assert!(
        projected
            .source_coverage()
            .limitations
            .contains("not_a_live_game_legality_certificate")
    );
    assert_eq!(
        projected.allowed_paid_nodes(),
        &BTreeSet::from([4739, 58751])
    );
}
