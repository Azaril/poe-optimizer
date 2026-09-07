//! Actual pinned tree extraction runs in a child process, with a deadline and a
//! bounded JSON result. The parent tests only consume owned serialized data.
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::OnceLock,
    thread,
    time::{Duration, Instant},
};

use poe_optimizer_pob::tree_data::{
    DanglingConnection, OverrideProvenance, SourceValue, TreeDataSnapshot, TreeNodeKind,
    TreePointCategory, extract_pinned_tree,
};

fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}
fn source_root() -> PathBuf {
    repository().join("vendor/path-of-building-poe2")
}

fn extract_child(root: &Path) -> Result<TreeDataSnapshot, String> {
    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("tree.json");
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "tree_extraction_child",
            "--ignored",
            "--nocapture",
        ])
        .env("POE_TREE_TEST_SOURCE", root)
        .env("POE_TREE_TEST_OUTPUT", &output)
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
            assert!(status.success(), "tree extraction child failed");
            break;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("tree extraction child exceeded deadline");
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert!(fs::metadata(&output).unwrap().len() < 64 * 1024 * 1024);
    serde_json::from_slice(&fs::read(output).unwrap()).unwrap()
}
fn snapshot() -> &'static TreeDataSnapshot {
    static SNAPSHOT: OnceLock<TreeDataSnapshot> = OnceLock::new();
    SNAPSHOT.get_or_init(|| extract_child(&source_root()).unwrap())
}

#[test]
#[ignore = "worker process invoked by extraction integration tests"]
fn tree_extraction_child() {
    let root = PathBuf::from(std::env::var_os("POE_TREE_TEST_SOURCE").unwrap());
    let output = PathBuf::from(std::env::var_os("POE_TREE_TEST_OUTPUT").unwrap());
    let result = extract_pinned_tree(&root, "0_5").map_err(|error| error.to_string());
    fs::write(output, serde_json::to_vec(&result).unwrap()).unwrap();
}

#[test]
fn all_catalog_classes_keep_both_ordinary_entrances_and_physical_root_owners() {
    let tree = snapshot();
    let expected = [
        (1, "Witch", 54447, [4739, 44871]),
        (2, "Ranger", 50459, [13828, 56651]),
        (6, "Warrior", 47175, [3936, 38646]),
        (7, "Sorceress", 54447, [4739, 44871]),
        (8, "Huntress", 50459, [13828, 56651]),
        (9, "Mercenary", 50986, [59779, 59915]),
        (10, "Monk", 44683, [10364, 52980]),
        (11, "Druid", 61525, [13855, 50084]),
    ];
    assert_eq!(tree.classes.len(), expected.len());
    for (id, name, root, entrances) in expected {
        let class = &tree.classes[&id];
        assert_eq!(class.name, name);
        assert_eq!(class.start_node_id, root);
        assert!(tree.nodes[&root].class_ids.contains(&id));
        assert_eq!(
            tree.ordinary_entrances(id).unwrap(),
            BTreeSet::from(entrances)
        );
        for asc in &class.ascendancy_ids {
            assert_eq!(tree.ascendancies[asc].class_id, id);
        }
    }
    assert_eq!(tree.nodes[&54447].class_ids, BTreeSet::from([1, 7]));
    assert_eq!(tree.nodes[&50459].class_ids, BTreeSet::from([2, 8]));
    assert!(
        tree.nodes
            .values()
            .any(|node| node.class_start_labels.contains("Shadow"))
    );
    assert!(tree.classes.values().all(|class| class.name != "Shadow"));
    assert_eq!(tree.classes[&7].base_intelligence, 15);
    assert_eq!(tree.classes[&6].base_strength, 15);
}

#[test]
fn symmetric_adjacency_retains_reverse_edges_and_exact_dangling_evidence() {
    let tree = snapshot();
    let missing = BTreeMap::from([
        (44683, vec![5162, 45406, 50198]),
        (47175, vec![16732, 51916, 54579]),
        (50459, vec![24665]),
        (50986, vec![39383, 10889, 62386]),
        (61525, vec![35715, 26353, 950, 28429]),
    ])
    .into_iter()
    .flat_map(|(from, ids)| {
        ids.into_iter()
            .map(move |missing| DanglingConnection { from, missing })
    })
    .collect();
    assert_eq!(tree.dangling_connections, missing);
    assert_eq!(tree.dangling_connections.len(), 14);
    for (root, reverse_entrance) in [
        (44683, 10364),
        (50986, 59779),
        (54447, 44871),
        (61525, 50084),
    ] {
        assert!(
            !tree.nodes[&root]
                .raw_connections
                .contains(&reverse_entrance)
        );
        assert!(
            tree.nodes[&reverse_entrance]
                .raw_connections
                .contains(&root)
        );
        assert!(tree.nodes[&root].adjacent.contains(&reverse_entrance));
    }
    for (id, node) in &tree.nodes {
        assert!(!node.adjacent.contains(id));
        if node.kind == TreeNodeKind::ImageOnly {
            assert!(node.adjacent.is_empty());
        }
        for adjacent in &node.adjacent {
            assert!(tree.nodes[adjacent].adjacent.contains(id));
            assert_ne!(tree.nodes[adjacent].kind, TreeNodeKind::ImageOnly);
        }
    }
    for dangling in &tree.dangling_connections {
        assert!(!tree.nodes.contains_key(&dangling.missing));
        assert!(
            !tree.nodes[&dangling.from]
                .adjacent
                .contains(&dangling.missing)
        );
    }
}

#[test]
fn shared_ascendancy_roots_paid_nodes_and_overrides_keep_provenance() {
    let tree = snapshot();
    let owners = BTreeSet::from(["Witch3".into(), "Witch3b".into()]);
    assert_eq!(tree.ascendancies.len(), 23);
    assert_eq!(tree.nodes[&23710].ascendancy_ids, owners);
    assert_eq!(tree.nodes[&58751].ascendancy_ids, owners);
    assert_eq!(tree.ascendancies["Witch3"].start_node_id, 23710);
    assert_eq!(tree.ascendancies["Witch3b"].start_node_id, 23710);
    assert_eq!(
        tree.ascendancies["Witch3"].replaced_by.as_deref(),
        Some("Abyssal Lich")
    );
    assert_eq!(
        tree.ascendancies["Witch3b"].replaces.as_deref(),
        Some("Lich")
    );
    let lich = tree.effective_node(1, Some("Witch3"), 58751).unwrap();
    let abyssal = tree.effective_node(1, Some("Witch3b"), 58751).unwrap();
    assert_eq!(lich.effective_source_id, 58751);
    assert_eq!(abyssal.physical_node_id, 58751);
    assert_eq!(abyssal.effective_source_id, 35941);
    assert_eq!(abyssal.stats, ["20% increased maximum Energy Shield"]);
    assert!(
        matches!(abyssal.provenance, OverrideProvenance::Ascendancy { ref internal_id, .. } if internal_id == "Witch3b")
    );
    let root = tree.effective_node(1, Some("Witch3b"), 23710).unwrap();
    assert_eq!(root.effective_source_id, 23710); // inherited, absent in option
    assert_eq!(root.name, "Lich"); // upstream option inherits the display name
    assert!(!root.override_fields.contains("name"));
    assert_eq!(
        root.source.named["ascendancyName"],
        SourceValue::String("Abyssal Lich".into())
    );
}

#[test]
fn class_specific_payloads_and_selector_precedence_match_source_contract() {
    let tree = snapshot();
    let witch = tree.effective_node(1, None, 4739).unwrap();
    let sorceress = tree.effective_node(7, None, 4739).unwrap();
    assert_eq!(witch.effective_source_id, 17306);
    assert_eq!(witch.name, "Spell and Minion Damage");
    assert_eq!(
        witch.stats,
        [
            "10% increased Spell Damage",
            "Minions deal 10% increased Damage"
        ]
    );
    assert_eq!(witch.physical_node_id, sorceress.physical_node_id);
    assert_eq!(sorceress.effective_source_id, 4739);
    assert_eq!(sorceress.provenance, OverrideProvenance::Base);
    assert_eq!(
        tree.effective_node(8, None, 56651)
            .unwrap()
            .effective_source_id,
        39263
    );
    assert_eq!(
        tree.effective_node(2, None, 56651)
            .unwrap()
            .effective_source_id,
        56651
    );
    // Independent precedence fixture: both names match, as permitted by the
    // loader's class-first/ascendancy-second branch; no source is modified.
    let mut both = tree.clone();
    let paid = both.nodes.get_mut(&4739).unwrap();
    let mut asc_option = paid.automatic_overrides["Witch"].clone();
    asc_option
        .named
        .insert("id".into(), SourceValue::Integer(999));
    paid.automatic_overrides.insert("Lich".into(), asc_option);
    assert_eq!(
        both.effective_node(1, Some("Witch3"), 4739)
            .unwrap()
            .effective_source_id,
        17306
    );
    assert!(tree.effective_node(7, Some("Warrior1"), 4739).is_err());
    assert!(tree.effective_node(3, None, 4739).is_err());
    assert!(tree.effective_node(7, Some("missing"), 4739).is_err());
    assert!(tree.effective_node(7, None, 5162).is_err());
}

#[test]
fn point_categories_and_opaque_data_do_not_claim_unimplemented_allocation_rules() {
    let tree = snapshot();
    assert_eq!(
        tree.nodes[&54447].point_category,
        TreePointCategory::ImplicitRoot
    );
    assert_eq!(tree.nodes[&54447].source_default_point_cost, Some(0));
    assert_eq!(tree.nodes[&23710].source_default_point_cost, Some(0));
    assert_eq!(
        tree.nodes[&4739].point_category,
        TreePointCategory::Ordinary
    );
    assert_eq!(tree.nodes[&4739].source_default_point_cost, Some(1));
    assert_eq!(
        tree.nodes[&58751].point_category,
        TreePointCategory::Ascendancy
    );
    assert_eq!(tree.nodes[&58751].source_default_point_cost, Some(1));
    assert!(
        tree.nodes
            .values()
            .any(|node| node.unsupported_mechanics.contains("unlock_constraint"))
    );
    let attribute = tree
        .nodes
        .values()
        .find(|node| node.unsupported_mechanics.contains("attribute_choice"))
        .unwrap();
    let SourceValue::Table(options) = &attribute.source.named["options"] else {
        panic!("attribute options not retained");
    };
    assert!(!options.indexed.is_empty());
    assert!(attribute.automatic_overrides.is_empty());
    assert!(
        tree.unsupported_mechanics
            .contains("live_game_topology_completeness")
    );
    assert!(
        tree.unsupported_mechanics
            .contains("stat_and_modifier_translation")
    );
    assert!(
        tree.unsupported_mechanics
            .contains("weapon_set_allocations")
    );
}

#[test]
fn fresh_extractions_have_identical_serialization_and_versioned_source_identity() {
    let tree = snapshot();
    let second = extract_child(&source_root()).unwrap();
    tree.validate_source_identity().unwrap();
    assert_eq!(
        tree.identity.upstream_revision,
        "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4"
    );
    assert_eq!(
        tree.identity.source_manifest_sha256,
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    assert_eq!(tree.identity.source_files_sha256.len(), 3);
    let bytes = serde_json::to_vec(tree).unwrap();
    assert_eq!(bytes, serde_json::to_vec(&second).unwrap());
    let decoded: TreeDataSnapshot = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(*tree, decoded);
    assert_eq!(tree.sha256().unwrap(), decoded.sha256().unwrap());
    assert!(
        !String::from_utf8(bytes)
            .unwrap()
            .contains(repository().to_str().unwrap())
    );
    let mut stale = tree.clone();
    stale.identity.tree_version = "0_4".into();
    assert!(stale.validate_source_identity().is_err());
    stale.identity = tree.identity.clone();
    stale.identity.extractor_sha256 = "0".repeat(64);
    assert!(stale.validate_source_identity().is_err());
    assert!(
        extract_pinned_tree(Path::new("does-not-exist"), "0_4")
            .unwrap_err()
            .to_string()
            .contains("unsupported tree version")
    );
}

#[test]
fn modified_local_tree_is_rejected_before_evaluating_its_lua() {
    let directory = tempfile::tempdir().unwrap();
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../data/pob-source-manifest.json")).unwrap();
    for entry in manifest["files"].as_array().unwrap() {
        let relative = entry["path"].as_str().unwrap();
        let destination = directory.path().join(relative);
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::copy(source_root().join(relative), destination).unwrap();
    }
    let path = directory.path().join("src/TreeData/0_5/tree.lua");
    let text = fs::read_to_string(&path).unwrap();
    fs::write(path, format!("-- changed extraction input\n{text}")).unwrap();
    let error = extract_child(directory.path()).unwrap_err();
    assert!(error.contains("src/TreeData/0_5/tree.lua"), "{error}");
    assert!(error.contains("mismatch"), "{error}");
}
