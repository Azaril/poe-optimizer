//! Portable consumers authenticate complete bytes; the PoB adapter separately
//! proves that the committed artifact is reproducible from the pinned source.
use poe_optimizer_data::{
    bundled::{authenticate_bundle, class_tree, content_sha256},
    tree_data::{
        OverrideProvenance, SourceTable, SourceValue, TreeDataSnapshot, expected_identity,
    },
    tree_projection::AuthenticatedTreeSnapshot,
};
use std::collections::BTreeSet;

#[test]
fn bundled_scope_preserves_all_identities_without_claiming_full_tree_coverage() {
    let tree = class_tree().unwrap();
    assert!(std::ptr::eq(tree, class_tree().unwrap()));
    assert_eq!(tree.sha256().unwrap(), content_sha256());
    assert_eq!(tree.source.schema_version, 2);
    assert_eq!(
        tree.classes.keys().copied().collect::<Vec<_>>(),
        [1, 2, 6, 7, 8, 9, 10, 11]
    );
    assert_eq!(
        tree.classes
            .values()
            .map(|class| class.source_index)
            .collect::<BTreeSet<_>>(),
        (1..=8).collect()
    );
    assert_eq!(tree.ascendancies.len(), 23);
    assert_eq!(tree.roots.len(), 28);
    assert_eq!(tree.ordinary_nodes.len(), 12);
    assert_eq!(tree.coverage.class_entrance_view_count, 16);
    assert_eq!(tree.coverage.no_effect_implicit_root_selections, 31);
    assert_eq!(tree.coverage.source_node_count, 4914);
    assert_eq!(tree.coverage.retained_node_count, 4141);
    assert_eq!(tree.coverage.excluded_node_count, 773);
    assert_eq!(tree.coverage.source_dangling_connections.len(), 14);
    assert!(!tree.coverage.boundary_edges.is_empty());
    assert!(
        tree.coverage
            .limitations
            .contains("strict_subset_not_full_tree")
    );
    let warrior = tree.class(6).unwrap();
    assert_eq!((warrior.source_index, warrior.integer_id), (3, 6));
    assert_eq!(
        (
            warrior.base_strength,
            warrior.base_dexterity,
            warrior.base_intelligence
        ),
        (15, 7, 7)
    );
    for class in tree.classes.values() {
        assert_eq!(tree.entrances(class.integer_id).unwrap().len(), 2);
        for id in &class.ascendancy_ids {
            assert_eq!(
                tree.ascendancy(class.integer_id, id).unwrap().internal_id,
                *id
            );
        }
    }
}

#[test]
fn shared_physical_roots_and_class_switch_evidence_remain_distinct() {
    let tree = class_tree().unwrap();
    assert_eq!(tree.roots[&54447].class_ids, BTreeSet::from([1, 7]));
    assert_eq!(tree.roots[&50459].class_ids, BTreeSet::from([2, 8]));
    let witch = tree.entrance(1, 4739).unwrap();
    let sorceress = tree.entrance(7, 4739).unwrap();
    assert_eq!(
        (witch.physical_node_id, witch.effective_source_id),
        (4739, 17306)
    );
    assert_eq!(sorceress.effective_source_id, 4739);
    assert_eq!(
        witch.stats,
        [
            "10% increased Spell Damage",
            "Minions deal 10% increased Damage"
        ]
    );
    assert_eq!(sorceress.stats, ["10% increased Spell Damage"]);
    assert!(matches!(
        witch.provenance,
        OverrideProvenance::Class { class_id: 1, .. }
    ));
    assert_eq!(tree.ordinary_nodes[&4739].stats, sorceress.stats);
    assert!(!tree.ordinary_nodes[&4739].automatic_overrides.is_empty());
    assert!(!witch.override_fields.is_empty());
    let huntress = tree.entrance(8, 56651).unwrap();
    assert_eq!(huntress.effective_source_id, 39263);
    assert_eq!(huntress.stats, ["10% increased Attack Damage"]);
    assert_eq!(
        tree.entrance(2, 56651).unwrap().stats,
        ["10% increased Projectile Damage"]
    );
    let lich = tree
        .ascendancies
        .values()
        .find(|value| value.name == "Lich")
        .unwrap();
    let abyssal = tree
        .ascendancies
        .values()
        .find(|value| value.name == "Abyssal Lich")
        .unwrap();
    assert_eq!(lich.start_node_id, abyssal.start_node_id);
    assert!(
        tree.roots[&lich.start_node_id]
            .ascendancy_ids
            .contains(&abyssal.internal_id)
    );
}

#[test]
fn borrowed_lookup_rejects_wrong_class_owner_and_unadmitted_nodes() {
    let tree = class_tree().unwrap();
    let witch_asc = tree.class(1).unwrap().ascendancy_ids.first().unwrap();
    assert!(tree.ascendancy(6, witch_asc).is_err());
    assert!(tree.ascendancy(1, "missing").is_err());
    assert!(tree.class(3).is_err());
    assert!(tree.entrance(6, 4739).is_err());
    assert!(tree.entrance(6, 47175).is_err());
    assert!(tree.entrances(999).is_err());
}

#[test]
fn unchanged_source_labels_do_not_authenticate_modified_bundle_bytes() {
    let tree = class_tree().unwrap();
    let mut tampered = tree.clone();
    tampered
        .class_entrances
        .get_mut(&1)
        .unwrap()
        .get_mut(&4739)
        .unwrap()
        .stats[0] = "999% increased Spell Damage".into();
    assert_eq!(tampered.source, tree.source);
    assert!(authenticate_bundle(&tampered.canonical_bytes().unwrap()).is_err());
    let mut whitespace = tree.canonical_bytes().unwrap();
    whitespace.push(b'\n');
    assert!(authenticate_bundle(&whitespace).is_err());
    assert!(authenticate_bundle(&vec![b' '; 1024 * 1024 + 1]).is_err());
}

#[test]
fn trusted_subset_validation_rejects_new_root_mechanics_or_effect_changes() {
    let tree = class_tree().unwrap();
    let mut changed = tree.clone();
    changed
        .roots
        .get_mut(&54447)
        .unwrap()
        .stats
        .push("10% increased Spell Damage".into());
    assert!(changed.validate_scope().is_err());
    let mut changed = tree.clone();
    changed
        .roots
        .get_mut(&54447)
        .unwrap()
        .source
        .named
        .insert("grantsPassivePoints".into(), SourceValue::Integer(1));
    assert!(changed.validate_scope().is_err());
    let mut changed = tree.clone();
    changed.roots.get_mut(&54447).unwrap().source.named.insert(
        "stats".into(),
        SourceValue::Table(SourceTable {
            named: Default::default(),
            indexed: [(1, SourceValue::String("10% increased Spell Damage".into()))].into(),
        }),
    );
    assert!(changed.validate_scope().is_err());
    let mut changed = tree.clone();
    changed
        .class_entrances
        .get_mut(&1)
        .unwrap()
        .get_mut(&4739)
        .unwrap()
        .stats
        .clear();
    assert!(changed.validate_scope().is_err());
    let mut changed = tree.clone();
    changed
        .ordinary_nodes
        .get_mut(&4739)
        .unwrap()
        .adjacent
        .remove(&54447);
    assert!(changed.validate_scope().is_err());
}

#[test]
fn source_identity_check_and_external_content_authentication_are_separate() {
    // A deliberately empty test fixture can copy legitimate source labels. It is
    // not a source extraction, so it must never be accepted by labels alone.
    let mut snapshot = TreeDataSnapshot {
        identity: expected_identity().unwrap(),
        classes: Default::default(),
        ascendancies: Default::default(),
        nodes: Default::default(),
        dangling_connections: Default::default(),
        ignored_image_connections: Default::default(),
        ignored_self_connections: Default::default(),
        unsupported_mechanics: Default::default(),
    };
    snapshot.validate_source_identity().unwrap();
    assert!(
        AuthenticatedTreeSnapshot::from_trusted_digest(
            snapshot.clone(),
            &class_tree().unwrap().full_snapshot_sha256
        )
        .is_err()
    );
    snapshot.identity.schema_version = 1;
    assert!(snapshot.validate_source_identity().is_err());
    assert_eq!(poe_optimizer_data::implementation_fingerprint().len(), 64);
}

#[test]
fn ascendancy_subset_preserves_full_source_ownership_and_rejects_extra_semantics() {
    let tree = class_tree().unwrap();
    assert_eq!(tree.ascendancy_nodes.len(), 4);
    assert_eq!(tree.coverage.ascendancy_passive_view_count, 4);
    for (class_id, ascendancy, node, root, stat) in [
        (6, "Warrior3", 14960, 5852, "+8% to Fire Resistance"),
        (
            11,
            "Druid2",
            61722,
            35535,
            "+3% to all Elemental Resistances",
        ),
        (10, "Monk3", 24475, 74, "+7% to Chaos Resistance"),
        (
            8,
            "Huntress3",
            17058,
            36365,
            "-20% to all Elemental Resistances",
        ),
    ] {
        let view = tree.ascendancy_passive(class_id, ascendancy, node).unwrap();
        assert_eq!(view.physical_node_id, node);
        assert_eq!(view.effective_source_id, node);
        assert_eq!(view.stats, [stat]);
        assert!(tree.ascendancy_nodes[&node].adjacent.contains(&root));
        assert!(tree.roots[&root].adjacent.contains(&node));
    }
    assert!(tree.ascendancy_passive(6, "Warrior1", 14960).is_err());
    for edit in 0..6 {
        let mut changed = tree.clone();
        let node = changed.ascendancy_nodes.get_mut(&14960).unwrap();
        match edit {
            0 => node.ascendancy_ids = BTreeSet::from(["Warrior1".into()]),
            1 => {
                node.adjacent.remove(&5852);
            }
            2 => node.source_default_point_cost = Some(0),
            3 => {
                node.source
                    .named
                    .insert("grantsPassivePoints".into(), SourceValue::Integer(1));
            }
            4 => {
                changed
                    .ascendancy_passives
                    .get_mut("Warrior3")
                    .unwrap()
                    .get_mut(&14960)
                    .unwrap()
                    .stats
                    .clear();
            }
            _ => {
                changed.ascendancy_passives.remove("Warrior3");
            }
        }
        assert!(changed.validate_scope().is_err(), "edit {edit}");
    }
}
