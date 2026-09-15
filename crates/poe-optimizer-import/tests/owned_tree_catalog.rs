//! Offline generic catalog contracts; no source checkout, originals or source VM.
#[path = "support/owned_recipe_fixture.rs"]
mod fixture;
use poe_optimizer_core::{owned_definitions::*, owned_schema::*};
use poe_optimizer_import::{
    owned_mapping::*, owned_recipe::*, owned_tree_catalog::*, owned_tree_policy::*,
};

fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn pin(path: &str) -> SourcePin {
    SourcePin {
        system: ExternalSourceSystem::PathOfBuilding2,
        revision: "a".repeat(40),
        files: vec![SourceFilePin {
            path: path.into(),
            sha256: "b".repeat(64),
        }],
    }
}
fn seed() -> (StagedOwnedRecipe, OwnedMappingIndex) {
    let mut input = fixture::recipe(7);
    let mut registry = OwnedIdRegistry::new(input.registry, OwnedMappingLimits::default()).unwrap();
    let retired: UnitDefId = registry.allocate_definition().unwrap();
    registry
        .retire(
            &SchemaSubject::Definition(retired.address()),
            key("removed-before-tree"),
        )
        .unwrap();
    input.registry = registry.input().clone();
    let base = assemble_owned_recipe(input, OwnedRecipeLimits::default()).unwrap();
    let mapping = OwnedMappingIndex::new(
        MappingPackageInput {
            schema_version: OWNED_MAPPING_PACKAGE_VERSION,
            namespace: base.schema().namespace().clone(),
            registry: base.registry().identity().unwrap(),
            definitions: base.schema().identity().clone(),
            source: pin("fixture/base.json"),
            policy_version: key("existing-mapping-policy"),
            entries: vec![],
        },
        base.registry(),
        base.schema(),
        OwnedMappingLimits::default(),
    )
    .unwrap();
    (base, mapping)
}
fn policy() -> TreeCatalogPolicy {
    TreeCatalogPolicy {
        version: key("reviewed-tree-policy"),
        level: IntegerRange {
            minimum: BoundedInteger::new(1).unwrap(),
            maximum: BoundedInteger::new(100).unwrap(),
        },
        syntax: TreeNormalizationSyntax {
            tree_version_attribute: "treeVersion".into(),
            class_attribute: "classInternalId".into(),
            ascendancy_attribute: "ascendancyInternalId".into(),
            class_consistency_attribute: Some("classId".into()),
            ascendancy_consistency_attribute: Some("ascendClassId".into()),
            overrides_element: "Overrides".into(),
            attribute_override_element: "AttributeOverride".into(),
            weapon_overlays: vec![],
            ignored_spec_children: vec!["Sockets".into()],
        },
    }
}
fn node(key: &str, kind: TreeNodeKind) -> TreeNodeInput {
    TreeNodeInput {
        key: key.into(),
        kind,
        stats: vec!["untranslated source stat".into()],
        views: vec![],
        unlock: vec![],
    }
}
fn catalog() -> TreeCatalogInput {
    TreeCatalogInput {
        schema_version: OWNED_TREE_CATALOG_VERSION,
        tree_version: "tree-one".into(),
        source: pin("fixture/tree.json"),
        classes: vec![
            TreeClassInput {
                key: "1".into(),
                root: "r".into(),
                ascendancies: vec![TreeAscendancyInput {
                    key: "alpha".into(),
                    ordinal: 1,
                    root: "ar".into(),
                }],
            },
            TreeClassInput {
                key: "2".into(),
                root: "r".into(),
                ascendancies: vec![TreeAscendancyInput {
                    key: "beta".into(),
                    ordinal: 1,
                    root: "ar".into(),
                }],
            },
        ],
        nodes: vec![
            node("r", TreeNodeKind::ImplicitRoot),
            node("ar", TreeNodeKind::ImplicitRoot),
            node(
                "p",
                TreeNodeKind::Allocation {
                    pool: TreePoolKind::Ascendancy,
                },
            ),
            node(
                "a",
                TreeNodeKind::Attribute {
                    pool: TreePoolKind::Ordinary,
                },
            ),
            node("o1", TreeNodeKind::AttachedChoice { parent: "p".into() }),
            node("o2", TreeNodeKind::AttachedChoice { parent: "p".into() }),
            node(
                "image",
                TreeNodeKind::Unsupported {
                    code: key("nonallocation-image"),
                },
            ),
        ],
        edges: [("r", "a"), ("ar", "p"), ("o1", "p"), ("o2", "p")]
            .into_iter()
            .map(|(left, right)| TreeEdgeInput {
                left: left.into(),
                right: right.into(),
            })
            .collect(),
        unresolved_edges: vec![TreeEdgeInput {
            left: "r".into(),
            right: "unavailable-connector".into(),
        }],
        attribute_options: ["strNodes", "dexNodes", "intNodes"]
            .into_iter()
            .map(|key| TreeAttributeOptionInput {
                key: key.into(),
                stats: vec![format!("explicit {key} effect remains untranslated")],
            })
            .collect(),
    }
}
fn extend(base: &StagedOwnedRecipe, map: &OwnedMappingIndex) -> StagedTreeCatalog {
    compile_owned_tree_catalog_extension(
        base,
        map,
        &catalog(),
        &policy(),
        TreeCatalogLimits::default(),
    )
    .unwrap()
}
fn role<'a>(out: &'a StagedTreeCatalog, token: &str) -> &'a TreeTokenRole {
    &out.content
        .tokens
        .iter()
        .find(|r| r.token == token)
        .unwrap()
        .role
}
fn bind_mapping(
    prior: &OwnedMappingIndex,
    out: &StagedTreeCatalog,
    staged: &StagedOwnedRecipe,
) -> OwnedMappingIndex {
    let mut input = prior.input().clone();
    input.registry = staged.registry().identity().unwrap();
    input.definitions = staged.schema().identity().clone();
    input.entries.extend(out.new_mappings.clone());
    for file in &out.content.source.files {
        if !input.source.files.iter().any(|p| p.path == file.path) {
            input.source.files.push(file.clone());
        }
    }
    OwnedMappingIndex::new(
        input,
        staged.registry(),
        staged.schema(),
        OwnedMappingLimits::default(),
    )
    .unwrap()
}
#[test]
fn shared_roots_attached_choices_and_attribute_lanes_have_exact_owned_structure() {
    let (base, map) = seed();
    let out = extend(&base, &map);
    let staged =
        assemble_owned_recipe(out.successor.clone(), OwnedRecipeLimits::default()).unwrap();
    let TreeTokenRole::ImplicitRoot { node: root } = role(&out, "r") else {
        panic!()
    };
    for row in &out.content.classes {
        let SchemaLookup::Known(c) = staged.schema().definition(&row.class) else {
            panic!()
        };
        assert_eq!(c.implicit_passives.members, vec![root.clone()]);
        assert!(c.implicit_passives.is_complete());
        assert!(!c.declarations.grants.is_complete());
    }
    let SchemaLookup::Known(r) = staged.schema().definition(root) else {
        panic!()
    };
    assert!(r.pools.is_complete() && r.pools.members.is_empty());
    assert!(!r.adjacent.is_complete());
    assert_eq!(r.adjacent.members.len(), 1);
    let TreeTokenRole::AttachedChoice {
        parent,
        slot,
        option,
    } = role(&out, "o1")
    else {
        panic!()
    };
    assert_eq!(
        slot.declaration,
        SlotOwnerDefId::PassiveNode(parent.clone())
    );
    let SchemaLookup::Known(s) = staged.schema().slot(slot) else {
        panic!()
    };
    let ValueSchema::Option { allowed } = &s.value else {
        panic!()
    };
    assert!(allowed.is_complete());
    assert!(allowed.members.contains(option));
    assert_eq!(allowed.members.len(), 2);
    assert_eq!(s.presence, SlotPresence::RequiredOnce);
    let SchemaLookup::Known(p) = staged.schema().definition(parent) else {
        panic!()
    };
    assert_eq!(
        p.adjacent.members.len(),
        1,
        "attached options are not physical neighbors"
    );
    assert_eq!(out.content.attributes.len(), 1);
    assert_eq!(out.content.attributes[0].lanes.len(), 3);
    let TreeTokenRole::Allocation { pool, .. } = role(&out, "a") else {
        panic!()
    };
    assert!(matches!(
        staged.schema().definition(pool),
        SchemaLookup::Known(PointPoolSchema {
            scope: PointPoolScope::Either
        })
    ));
    assert!(matches!(
        role(&out, "image"),
        TreeTokenRole::Unresolved { .. }
    ));
    assert_eq!(out.receipt.counts.allocated_definitions, 15);
    assert_eq!(out.receipt.counts.allocated_slots, 2);
    assert!(!out.new_mappings.iter().any(|row|matches!(&row.source,ExternalSelector::Definition(ExternalOwnerSelector::PassiveNode{node_id:SourceComponent::Text(k),..}) if k=="o1"||k=="image"||k=="unavailable-connector")));
    assert!(
        staged
            .rules()
            .input()
            .owners
            .iter()
            .skip(base.rules().input().owners.len())
            .all(|o| !o.programs.is_complete() && o.programs.members.is_empty())
    );
}
#[test]
fn exact_rerun_reuses_every_id_and_preserves_prior_tombstones_and_programs() {
    let (base, map) = seed();
    let before = serde_json::to_vec(base.registry().input()).unwrap();
    let out = extend(&base, &map);
    assert_eq!(
        &out.successor.registry.entries[..base.registry().input().entries.len()],
        base.registry().input().entries.as_slice()
    );
    for old in &base.schema().input().definitions {
        assert_eq!(
            out.successor
                .schema
                .definitions
                .iter()
                .find(|d| d.address() == old.address()),
            Some(old)
        );
    }
    assert_eq!(
        &out.successor.rules.owners[..base.rules().input().owners.len()],
        base.rules().input().owners.as_slice()
    );
    assert_eq!(out.successor.rules.tables, base.rules().input().tables);
    let staged =
        assemble_owned_recipe(out.successor.clone(), OwnedRecipeLimits::default()).unwrap();
    let nextmap = bind_mapping(&map, &out, &staged);
    let again = extend(&staged, &nextmap);
    assert_eq!(again.receipt.counts.allocated_definitions, 0);
    assert_eq!(again.receipt.counts.allocated_slots, 0);
    assert_eq!(again.receipt.counts.reused_definitions, 15);
    assert_eq!(again.receipt.counts.reused_slots, 2);
    assert!(again.new_mappings.is_empty());
    assert_eq!(again.successor, out.successor);
    assert_eq!(again.content, out.content);
    assert_eq!(before, serde_json::to_vec(base.registry().input()).unwrap());
    let mut reordered = catalog();
    reordered.classes.reverse();
    reordered.nodes.reverse();
    reordered.edges.reverse();
    reordered.attribute_options.reverse();
    let reordered = compile_owned_tree_catalog_extension(
        &base,
        &map,
        &reordered,
        &policy(),
        TreeCatalogLimits::default(),
    )
    .unwrap();
    assert_eq!(reordered.successor.registry, out.successor.registry);
    assert_eq!(reordered.successor.schema, out.successor.schema);
}
#[test]
fn conflicting_source_and_existing_descriptors_or_unresolved_selectors_fail_atomically() {
    let (base, map) = seed();
    let out = extend(&base, &map);
    let staged =
        assemble_owned_recipe(out.successor.clone(), OwnedRecipeLimits::default()).unwrap();
    let nextmap = bind_mapping(&map, &out, &staged);
    let before = serde_json::to_vec(staged.registry().input()).unwrap();
    let mut changed = catalog();
    changed
        .nodes
        .iter_mut()
        .find(|n| n.key == "a")
        .unwrap()
        .kind = TreeNodeKind::Attribute {
        pool: TreePoolKind::Ascendancy,
    };
    assert!(matches!(
        compile_owned_tree_catalog_extension(
            &staged,
            &nextmap,
            &changed,
            &policy(),
            TreeCatalogLimits::default()
        ),
        Err(TreeCatalogError::Preservation)
    ));
    let mut changed = catalog();
    changed.source.files.push(SourceFilePin {
        path: "fixture/base.json".into(),
        sha256: "c".repeat(64),
    });
    assert!(matches!(
        compile_owned_tree_catalog_extension(
            &base,
            &map,
            &changed,
            &policy(),
            TreeCatalogLimits::default()
        ),
        Err(TreeCatalogError::Binding)
    ));
    let mut raw = nextmap.input().clone();
    let entry = raw
        .entries
        .iter_mut()
        .find(|e| {
            matches!(
                e.source,
                ExternalSelector::Definition(ExternalOwnerSelector::Class { .. })
            )
        })
        .unwrap();
    entry.outcome = MappingOutcome::Unmapped {
        issue: key("deliberately-unresolved"),
    };
    let unresolved = OwnedMappingIndex::new(
        raw,
        staged.registry(),
        staged.schema(),
        OwnedMappingLimits::default(),
    )
    .unwrap();
    assert!(matches!(
        compile_owned_tree_catalog_extension(
            &staged,
            &unresolved,
            &catalog(),
            &policy(),
            TreeCatalogLimits::default()
        ),
        Err(TreeCatalogError::Reuse)
    ));
    assert!(
        compile_owned_tree_catalog_extension(
            &staged,
            &map,
            &catalog(),
            &policy(),
            TreeCatalogLimits::default()
        )
        .is_err(),
        "old bound mapping must not be silently rebound"
    );
    assert_eq!(
        before,
        serde_json::to_vec(staged.registry().input()).unwrap()
    );
}
#[test]
fn malformed_graph_roots_choices_and_lane_duplicates_are_rejected() {
    let (base, map) = seed();
    let check = |input: &TreeCatalogInput| {
        assert!(
            compile_owned_tree_catalog_extension(
                &base,
                &map,
                input,
                &policy(),
                TreeCatalogLimits::default()
            )
            .is_err()
        )
    };
    let mut c = catalog();
    c.nodes.push(c.nodes[0].clone());
    check(&c);
    let mut c = catalog();
    c.classes[0].root = "a".into();
    check(&c);
    let mut c = catalog();
    c.classes[0].ascendancies[0].ordinal = 2;
    check(&c);
    let mut c = catalog();
    c.edges.push(TreeEdgeInput {
        left: "a".into(),
        right: "r".into(),
    });
    check(&c);
    let mut c = catalog();
    c.edges.retain(|e| e.left != "o1");
    check(&c);
    let mut c = catalog();
    c.nodes.iter_mut().find(|n| n.key == "o1").unwrap().kind =
        TreeNodeKind::AttachedChoice { parent: "a".into() };
    check(&c);
    let mut c = catalog();
    c.unresolved_edges[0].right = "p".into();
    check(&c);
    let mut c = catalog();
    c.attribute_options.push(c.attribute_options[0].clone());
    check(&c);
    let mut c = catalog();
    c.nodes[0].unlock.push("missing-node".into());
    check(&c);
}
#[test]
fn strict_wire_and_shared_work_output_and_nested_limits_apply() {
    let limits = TreeCatalogLimits::default();
    let c = catalog();
    let bytes = serde_json::to_vec(&c).unwrap();
    assert_eq!(decode_tree_catalog(&bytes, limits).unwrap(), c);
    let raw = String::from_utf8(bytes.clone()).unwrap();
    let duplicate = format!("{{\"schema_version\":1,{}", &raw[1..]);
    assert!(decode_tree_catalog(duplicate.as_bytes(), limits).is_err());
    for field in ["nodes", "unresolved_edges", "attribute_options"] {
        let mut v = serde_json::to_value(&c).unwrap();
        v.as_object_mut().unwrap().remove(field);
        assert!(decode_tree_catalog(&serde_json::to_vec(&v).unwrap(), limits).is_err());
    }
    let mut v = serde_json::to_value(&c).unwrap();
    v["nodes"][0]["kind"]["value"] = serde_json::json!({"unexpected":true});
    assert!(decode_tree_catalog(&serde_json::to_vec(&v).unwrap(), limits).is_err());
    let (base, map) = seed();
    let out = extend(&base, &map);
    for l in [
        TreeCatalogLimits {
            max_work: out.receipt.work_used - 1,
            ..limits
        },
        TreeCatalogLimits {
            max_rows: 1,
            ..limits
        },
        TreeCatalogLimits {
            max_entries: 1,
            ..limits
        },
        TreeCatalogLimits {
            max_text_bytes: 1,
            ..limits
        },
        TreeCatalogLimits {
            max_string_bytes: 1,
            ..limits
        },
        TreeCatalogLimits {
            max_output_bytes: 1,
            ..limits
        },
        TreeCatalogLimits {
            max_wire_bytes: bytes.len() - 1,
            ..limits
        },
        TreeCatalogLimits {
            recipe: OwnedRecipeLimits {
                compile: poe_optimizer_engine::owned_rules::RuleLimits {
                    max_owners: 1,
                    ..limits.recipe.compile
                },
                ..limits.recipe
            },
            ..limits
        },
    ] {
        assert!(compile_owned_tree_catalog_extension(&base, &map, &c, &policy(), l).is_err());
    }
    assert!(
        compile_owned_tree_catalog_extension(
            &base,
            &map,
            &c,
            &policy(),
            TreeCatalogLimits {
                max_work: out.receipt.work_used,
                ..limits
            }
        )
        .is_ok()
    );
}
