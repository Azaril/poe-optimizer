//! Owned schema v2 root references, canonicalization and bounded storage laws.
use poe_optimizer_core::{owned_definitions::*, owned_schema::*};
use poe_optimizer_data::owned_schema::*;
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("root-schema", "v1").unwrap()
}
fn id<K: DefinitionDomain>(key: &str) -> DefId<K> {
    DefId::parse(ns(), key).unwrap()
}
fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn empty<T>() -> DeclaredSet<T> {
    DeclaredSet::complete(vec![])
}
fn declarations() -> DeclaredSlots {
    DeclaredSlots {
        parameters: empty(),
        choices: empty(),
        grants: empty(),
        actors: empty(),
        skill_grants: empty(),
        outputs: empty(),
        sockets: empty(),
    }
}
fn known<I, D>(id: I, schema: D) -> DefinitionEntry<I, D> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn gap(subject: SchemaSubject) -> SchemaGap {
    SchemaGap {
        subject,
        facet: SchemaFacet::StaticLinks,
        code: key("unconverted-roots"),
    }
}
fn input() -> SchemaPackageInput {
    SchemaPackageInput {
        schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
        namespace: ns(),
        release: key("test"),
        semantics_version: key("v1"),
        definitions: vec![
            DefinitionDescriptor::Class(known(
                id("class"),
                ClassSchema {
                    level: IntegerRange {
                        minimum: BoundedInteger::new(1).unwrap(),
                        maximum: BoundedInteger::new(100).unwrap(),
                    },
                    ascendancies: DeclaredSet::complete(vec![id("asc")]),
                    implicit_passives: DeclaredSet::complete(vec![id("shared"), id("root")]),
                    declarations: declarations(),
                },
            )),
            DefinitionDescriptor::Ascendancy(known(
                id("asc"),
                AscendancySchema {
                    classes: DeclaredSet::complete(vec![id("class")]),
                    implicit_passives: DeclaredSet::complete(vec![id("shared")]),
                    declarations: declarations(),
                },
            )),
            DefinitionDescriptor::PassiveNode(known(
                id("shared"),
                PassiveNodeSchema {
                    pools: empty(),
                    adjacent: empty(),
                    declarations: declarations(),
                },
            )),
            DefinitionDescriptor::PassiveNode(known(
                id("root"),
                PassiveNodeSchema {
                    pools: empty(),
                    adjacent: empty(),
                    declarations: declarations(),
                },
            )),
            DefinitionDescriptor::PointPool(known(
                id("pool"),
                PointPoolSchema {
                    scope: PointPoolScope::Either,
                },
            )),
        ],
        slots: vec![],
    }
}
fn class(input: &mut SchemaPackageInput) -> &mut ClassSchema {
    match input
        .definitions
        .iter_mut()
        .find(|d| d.address() == id::<ClassDefinition>("class").address())
        .unwrap()
    {
        DefinitionDescriptor::Class(DefinitionEntry {
            schema: SchemaState::Known(s),
            ..
        }) => s,
        _ => panic!("class"),
    }
}
fn passive(input: &mut SchemaPackageInput) -> &mut PassiveNodeSchema {
    match input
        .definitions
        .iter_mut()
        .find(|d| d.address() == id::<PassiveNodeDefinition>("root").address())
        .unwrap()
    {
        DefinitionDescriptor::PassiveNode(DefinitionEntry {
            schema: SchemaState::Known(s),
            ..
        }) => s,
        _ => panic!("root"),
    }
}
fn error(input: SchemaPackageInput, expected: SchemaPackageErrorKind) {
    assert!(
        matches!(OwnedDefinitionSchemaPackage::new(input,OwnedSchemaLimits::default()),Err(SchemaPackageError::Invalid{kind,..}) if kind==expected)
    );
}
fn arrays(value: &serde_json::Value) -> (usize, usize) {
    match value {
        serde_json::Value::Array(values) => {
            values
                .iter()
                .fold((values.len(), values.len()), |(sum, max), v| {
                    let (a, b) = arrays(v);
                    (sum + a, max.max(b))
                })
        }
        serde_json::Value::Object(values) => values.values().fold((0, 0), |(sum, max), v| {
            let (a, b) = arrays(v);
            (sum + a, max.max(b))
        }),
        _ => (0, 0),
    }
}
#[test]
fn roots_are_canonical_references_with_changed_membership_bound_to_identity() {
    let original = input();
    let mut reordered = original.clone();
    reordered.definitions.reverse();
    class(&mut reordered).implicit_passives.members.reverse();
    let first = OwnedDefinitionSchemaPackage::new(original.clone(), Default::default()).unwrap();
    let second = OwnedDefinitionSchemaPackage::new(reordered, Default::default()).unwrap();
    assert_eq!(first.identity(), second.identity());
    assert_eq!(first.identity().schema_version, 2);
    let bytes = encode_schema_package(&first, Default::default()).unwrap();
    let restored = decode_schema_package(&bytes, Default::default()).unwrap();
    assert_eq!(restored.input(), first.input());
    assert!(matches!(
        restored.definition(&id::<PointPoolDefinition>("pool")),
        SchemaLookup::Known(PointPoolSchema {
            scope: PointPoolScope::Either
        })
    ));
    let mut changed = original;
    class(&mut changed).implicit_passives.members.pop();
    let changed = OwnedDefinitionSchemaPackage::new(changed, Default::default()).unwrap();
    assert_ne!(first.identity(), changed.identity());
}
#[test]
fn duplicate_missing_and_foreign_root_references_reject() {
    let mut duplicate = input();
    class(&mut duplicate)
        .implicit_passives
        .members
        .push(id("root"));
    error(duplicate, SchemaPackageErrorKind::DuplicateMember);
    let mut missing = input();
    class(&mut missing)
        .implicit_passives
        .members
        .push(id("missing"));
    error(missing, SchemaPackageErrorKind::MissingDefinition);
    let mut foreign = input();
    class(&mut foreign)
        .implicit_passives
        .members
        .push(DefId::parse(GameVersionNamespace::new("other", "v1").unwrap(), "root").unwrap());
    error(foreign, SchemaPackageErrorKind::ForeignNamespace);
    let mut gapless = input();
    class(&mut gapless).implicit_passives.closure = SchemaClosure::Partial { gaps: vec![] };
    error(gapless, SchemaPackageErrorKind::EmptyGapEvidence);
}
#[test]
fn positive_root_pools_conflict_but_partial_empty_pools_and_unmapped_roots_survive() {
    for partial in [false, true] {
        let mut bad = input();
        passive(&mut bad).pools.members.push(id("pool"));
        if partial {
            passive(&mut bad).pools.closure = SchemaClosure::Partial {
                gaps: vec![gap(SchemaSubject::Definition(
                    id::<PassiveNodeDefinition>("root").address(),
                ))],
            };
        }
        error(bad, SchemaPackageErrorKind::ImplicitPassiveHasPools);
    }
    let mut partial = input();
    class(&mut partial).implicit_passives.closure = SchemaClosure::Partial {
        gaps: vec![gap(SchemaSubject::Definition(
            id::<ClassDefinition>("class").address(),
        ))],
    };
    passive(&mut partial).pools.closure = SchemaClosure::Partial {
        gaps: vec![gap(SchemaSubject::Definition(
            id::<PassiveNodeDefinition>("root").address(),
        ))],
    };
    let checked = OwnedDefinitionSchemaPackage::new(partial, Default::default()).unwrap();
    let SchemaLookup::Known(c) = checked.definition(&id::<ClassDefinition>("class")) else {
        panic!("class")
    };
    assert!(!c.implicit_passives.is_complete());
    let SchemaLookup::Known(p) = checked.definition(&id::<PassiveNodeDefinition>("root")) else {
        panic!("root")
    };
    assert!(!p.pools.is_complete());
    let mut unmapped = input();
    let row = unmapped
        .definitions
        .iter_mut()
        .find(|d| d.address() == id::<PassiveNodeDefinition>("root").address())
        .unwrap();
    *row = DefinitionDescriptor::PassiveNode(DefinitionEntry {
        id: id("root"),
        schema: SchemaState::Unmapped {
            gaps: vec![gap(SchemaSubject::Definition(
                id::<PassiveNodeDefinition>("root").address(),
            ))],
        },
    });
    assert!(OwnedDefinitionSchemaPackage::new(unmapped, Default::default()).is_ok());
}
#[test]
fn root_members_and_gap_evidence_use_the_shared_constructor_decode_and_encode_budget() {
    let mut raw = input();
    class(&mut raw).implicit_passives.closure = SchemaClosure::Partial {
        gaps: vec![gap(SchemaSubject::Definition(
            id::<ClassDefinition>("class").address(),
        ))],
    };
    passive(&mut raw).pools.closure = SchemaClosure::Partial {
        gaps: vec![gap(SchemaSubject::Definition(
            id::<PassiveNodeDefinition>("root").address(),
        ))],
    };
    let bytes = serde_json::to_vec(&raw).unwrap();
    let (total, largest) = arrays(&serde_json::to_value(&raw).unwrap());
    let limits = OwnedSchemaLimits {
        max_entries: total,
        max_collection_entries: largest,
        max_wire_bytes: bytes.len() + 1024,
    };
    let checked = OwnedDefinitionSchemaPackage::new(raw.clone(), limits).unwrap();
    assert!(decode_schema_package(&bytes, limits).is_ok());
    assert!(encode_schema_package(&checked, limits).is_ok());
    let tight = OwnedSchemaLimits {
        max_entries: total - 1,
        ..limits
    };
    assert!(OwnedDefinitionSchemaPackage::new(raw, tight).is_err());
    assert!(decode_schema_package(&bytes, tight).is_err());
    assert!(encode_schema_package(&checked, tight).is_err());
}
#[test]
fn schema_v2_requires_both_root_fields_and_rejects_old_or_duplicate_wire() {
    let raw = input();
    let bytes = serde_json::to_vec(&raw).unwrap();
    let mut old = raw;
    old.schema_version = 1;
    assert!(matches!(
        OwnedDefinitionSchemaPackage::new(old, Default::default()),
        Err(SchemaPackageError::UnsupportedVersion(1))
    ));
    for kind in ["class", "ascendancy"] {
        let mut missing: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let row = missing["definitions"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|d| d["kind"] == kind)
            .unwrap();
        row["value"]["schema"]["value"]
            .as_object_mut()
            .unwrap()
            .remove("implicit_passives");
        assert!(
            decode_schema_package(&serde_json::to_vec(&missing).unwrap(), Default::default())
                .is_err()
        );
    }
    let text = String::from_utf8(bytes).unwrap();
    let duplicate=text.replacen("\"implicit_passives\":","\"implicit_passives\":{\"members\":[],\"closure\":{\"kind\":\"complete\"}},\"implicit_passives\":",1);
    assert_ne!(duplicate, text);
    assert!(decode_schema_package(duplicate.as_bytes(), Default::default()).is_err());
}
