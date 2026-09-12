use poe_optimizer_data::{class_tree::*, game_data::*, tree_data::TreeNodeKind};
use std::collections::BTreeSet;
fn selection(
    class_id: u32,
    ids: &[u32],
    attrs: &[(u32, AttributeOption)],
) -> PassiveAllocationSelection {
    PassiveAllocationSelection {
        class_id,
        ordinary_nodes: ids.iter().copied().collect(),
        attribute_options: attrs.iter().copied().collect(),
        ..Default::default()
    }
}
fn custom(mut p: GameDataPackage) -> Result<GameDataSnapshot, GameDataError> {
    p.refresh_section_digests()?;
    GameDataLoader::from_bytes(
        &p.canonical_bytes()?,
        &TrustPolicy::AllowCustom,
        &LoadLimits::default(),
    )
}
#[test]
fn whole_source_partition_keeps_exclusions_and_all_attribute_choices() {
    let s = bundled_snapshot().unwrap();
    let t = s.tree();
    assert_eq!(t.allocation_nodes.len(), 4109);
    assert_eq!(t.allocation_views.len(), 4758);
    assert_eq!(s.package().passive_effects.len(), 1282);
    assert_eq!(s.package().passive_exclusions.len(), 3476);
    let admitted: BTreeSet<_> = s
        .package()
        .passive_effects
        .iter()
        .map(|v| v.key.clone())
        .collect();
    let excluded: BTreeSet<_> = s
        .package()
        .passive_exclusions
        .iter()
        .map(|v| v.key.clone())
        .collect();
    assert!(admitted.is_disjoint(&excluded));
    assert_eq!(
        admitted.union(&excluded).cloned().collect::<BTreeSet<_>>(),
        t.allocation_views.iter().map(|v| v.key.clone()).collect()
    );
    let mut attribute_count = 0;
    for node in t
        .allocation_nodes
        .values()
        .filter(|n| !n.attribute_options.is_empty())
    {
        attribute_count += 1;
        for option in AttributeOption::ALL {
            let k = PassiveViewKey {
                physical_node_id: node.id,
                selector: PassiveViewSelector::Attribute { option },
            };
            let e = s.package().passive_view_effects(&k).unwrap();
            assert_eq!(e.effective_node_id, node.attribute_options[&option]);
            assert_eq!(e.actor_modifiers.len(), 1);
            assert!(e.effects.is_empty());
        }
    }
    assert_eq!(attribute_count, 293);
    assert!(s.package().passive_effects.iter().any(|e| {
        t.allocation_nodes
            .get(&e.key.physical_node_id)
            .is_some_and(|n| n.kind == TreeNodeKind::Notable)
    }));
    let graph = allocation_candidate_catalog(&s).unwrap();
    assert_eq!(
        graph.passive_nodes.len(),
        admitted
            .iter()
            .map(|k| k.physical_node_id)
            .collect::<BTreeSet<_>>()
            .len()
            + t.roots.len()
    );
}
#[test]
fn explicit_physical_attribute_options_and_class_overrides_resolve_connected_paths() {
    let s = bundled_snapshot().unwrap();
    for option in AttributeOption::ALL {
        let resolved = selection(1, &[4739, 22419], &[(22419, option)])
            .resolve(&s)
            .unwrap();
        assert!(resolved.allocated_nodes.contains(&22419));
        assert!(!resolved.allocated_nodes.contains(&26297));
        let attr = resolved
            .views
            .iter()
            .find(|v| v.source.key.physical_node_id == 22419)
            .unwrap();
        assert_eq!(
            attr.source.key.selector,
            PassiveViewSelector::Attribute { option }
        );
        assert_eq!(
            attr.source.effective_node_id,
            match option {
                AttributeOption::Strength => 26297,
                AttributeOption::Dexterity => 14927,
                AttributeOption::Intelligence => 57022,
            }
        );
    }
    let witch = selection(1, &[4739, 18845, 1755, 41965, 51184], &[])
        .resolve(&s)
        .unwrap();
    let sorceress = selection(7, &[4739, 18845, 1755, 41965, 51184], &[])
        .resolve(&s)
        .unwrap();
    assert_eq!(witch.views.last().unwrap().source.name, "Raw Destruction");
    assert_eq!(sorceress.views.last().unwrap().source.name, "Raw Power");
    assert_ne!(
        witch.views.last().unwrap().effects,
        sorceress.views.last().unwrap().effects
    );
    let warrior = selection(6, &[3936, 13397], &[(13397, AttributeOption::Strength)]);
    assert!(warrior.resolve(&s).is_ok());
}
#[test]
fn allocation_rejects_missing_options_dormant_choices_disconnection_foreign_roots_and_whole_unsupported_nodes()
 {
    let s = bundled_snapshot().unwrap();
    let valid = selection(1, &[4739, 22419], &[(22419, AttributeOption::Strength)]);
    assert!(valid.resolve(&s).is_ok());
    let mut cases = Vec::new();
    let mut p = valid.clone();
    p.attribute_options.clear();
    cases.push(p);
    let mut p = valid.clone();
    p.attribute_options.insert(4739, AttributeOption::Strength);
    cases.push(p);
    let mut p = valid.clone();
    p.attribute_options.insert(9999, AttributeOption::Strength);
    cases.push(p);
    let mut p = valid.clone();
    p.ordinary_nodes.remove(&4739);
    cases.push(p);
    let mut p = valid.clone();
    p.ordinary_nodes.insert(54447);
    cases.push(p);
    let mut p = valid.clone();
    p.class_id = 6;
    cases.push(p);
    let mut p = valid.clone();
    p.ascendancy_id = Some("Warrior1".into());
    cases.push(p);
    let mut p = valid.clone();
    p.ascendancy_nodes.insert(14960);
    cases.push(p);
    let mut p = valid.clone();
    p.ordinary_nodes.insert(1_000_000);
    cases.push(p);
    for p in cases {
        assert!(p.resolve(&s).is_err(), "{p:?}");
    }
    let excluded = s
        .package()
        .passive_exclusions
        .iter()
        .find(|v| v.key.selector == PassiveViewSelector::Base)
        .unwrap();
    assert!(
        selection(1, &[excluded.key.physical_node_id], &[])
            .resolve(&s)
            .unwrap_err()
            .to_string()
            .contains("unsupported whole passive")
    );
}
#[test]
fn passive_custom_values_bind_identity_but_unknown_capabilities_and_order_sensitive_operations_reject()
 {
    let original = bundled_snapshot().unwrap();
    let mut p = original.package().clone();
    let key = PassiveViewKey {
        physical_node_id: 22419,
        selector: PassiveViewSelector::Attribute {
            option: AttributeOption::Strength,
        },
    };
    p.passive_effects
        .iter_mut()
        .find(|e| e.key == key)
        .unwrap()
        .actor_modifiers[0]
        .effect = ActorModifierEffect::Numeric {
        operation: ActorNumericOperation::Base,
        value: 17.0,
    };
    let changed = custom(p).unwrap();
    assert_ne!(changed.identity(), original.identity());
    let r = selection(1, &[4739, 22419], &[(22419, AttributeOption::Strength)])
        .resolve(&changed)
        .unwrap();
    assert_eq!(
        r.views.last().unwrap().actor_modifiers[0].effect,
        ActorModifierEffect::Numeric {
            operation: ActorNumericOperation::Base,
            value: 17.0
        }
    );
    let mut p = original.package().clone();
    // Keep the source partition and ordinary structural eligibility valid so
    // this attempt reaches the reviewed capability-set guard itself.
    let excluded_index = p
        .passive_exclusions
        .iter()
        .position(|entry| {
            p.tree
                .allocation_nodes
                .get(&entry.key.physical_node_id)
                .is_some_and(|node| {
                    matches!(node.kind, TreeNodeKind::Normal | TreeNodeKind::Notable)
                        && node.source_default_point_cost == Some(1)
                        && node
                            .unsupported_mechanics
                            .iter()
                            .all(|name| name == "attribute_choice")
                        && (!matches!(entry.key.selector, PassiveViewSelector::Attribute { .. })
                            || !node.attribute_options.is_empty())
                })
        })
        .expect("an excluded ordinary source view with eligible structure");
    let exclusion = p.passive_exclusions.remove(excluded_index);
    let source = p
        .tree
        .allocation_views
        .iter()
        .find(|v| v.key == exclusion.key)
        .unwrap();
    p.passive_effects.push(PassiveEffects {
        key: exclusion.key,
        effective_node_id: source.effective_node_id,
        effects: Vec::new(),
        actor_modifiers: original
            .package()
            .passive_view_effects(&key)
            .unwrap()
            .actor_modifiers
            .clone(),
    });
    assert!(custom(p).unwrap_err().to_string().contains(
        "custom package cannot expand or replace the source-reviewed passive capability set"
    ));
    for op in [ActorNumericOperation::More, ActorNumericOperation::Override] {
        let mut p = original.package().clone();
        p.passive_effects
            .iter_mut()
            .find(|e| e.key == key)
            .unwrap()
            .actor_modifiers[0]
            .effect = ActorModifierEffect::Numeric {
            operation: op,
            value: 10.0,
        };
        assert!(custom(p).is_err());
    }
    let mut p = original.package().clone();
    p.passive_effects
        .iter_mut()
        .find(|e| e.key == key)
        .unwrap()
        .actor_modifiers[0]
        .flags = 1;
    assert!(custom(p).is_err());
}
#[test]
fn jewellery_catalog_is_source_injected_and_rejects_broken_implicit_contracts() {
    let s = bundled_snapshot().unwrap();
    assert_eq!(
        s.package()
            .jewellery_bases
            .iter()
            .map(|b| b.name.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            "Amber Amulet",
            "Bloodstone Amulet",
            "Jade Amulet",
            "Lapis Amulet",
            "Lunar Amulet",
            "Pearlescent Amulet",
            "Solar Amulet"
        ])
    );
    assert!(
        s.package()
            .jewellery_base_by_name("Stellar Amulet")
            .is_none()
    );
    let amber = s.package().jewellery_base("amber_amulet").unwrap();
    assert_eq!(amber.requirements.level, 8);
    assert_eq!(
        (amber.implicit.minimum, amber.implicit.maximum),
        (10.0, 15.0)
    );
    assert!(!amber.source.named.is_empty());
    let mut p = s.package().clone();
    p.jewellery_bases[0].name = "Custom Amber".into();
    p.jewellery_bases[0].implicit.minimum = 11.0;
    let changed = custom(p).unwrap();
    assert_ne!(changed.identity(), s.identity());
    assert!(
        changed
            .package()
            .jewellery_base_by_name("Custom Amber")
            .is_some()
    );
    let edits: [fn(&mut GameDataPackage); 5] = [
        |p| p.jewellery_bases[0].implicit.actor_rule_id = "missing".into(),
        |p| p.jewellery_bases[0].implicit.minimum = 16.0,
        |p| p.jewellery_bases[0].implicit.maximum = f64::NAN,
        |p| p.jewellery_bases[0].requirements.level = 1001,
        |p| p.jewellery_bases.push(p.jewellery_bases[0].clone()),
    ];
    for edit in edits {
        let mut p = s.package().clone();
        edit(&mut p);
        assert!(custom(p).is_err());
    }
}
