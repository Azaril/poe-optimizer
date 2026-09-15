use poe_optimizer_core::{
    owned_build::ParameterValue, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::{owned_rules::*, owned_schema::*};
fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("owned-rules-test", "v1").unwrap()
}
fn owner() -> SchemaSubject {
    SchemaSubject::Definition(DefinitionAddress::Stat(DefId::new(ns(), key("stat"))))
}
fn schema() -> OwnedDefinitionSchemaPackage {
    let SchemaSubject::Definition(DefinitionAddress::Stat(id)) = owner() else {
        unreachable!()
    };
    OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: poe_optimizer_data::owned_schema::OWNED_SCHEMA_PACKAGE_VERSION,
            namespace: ns(),
            release: key("schema"),
            semantics_version: key("schema-v1"),
            definitions: vec![DefinitionDescriptor::Stat(DefinitionEntry {
                id,
                schema: SchemaState::Known(StatSchema {
                    value: ComputedValueType::Integer,
                    targets: vec![RuleEntityKind::Actor],
                }),
            })],
            slots: vec![],
        },
        OwnedSchemaLimits::default(),
    )
    .unwrap()
}
fn input(schema: &OwnedDefinitionSchemaPackage) -> RulePackageInput {
    let SchemaSubject::Definition(DefinitionAddress::Stat(stat)) = owner() else {
        unreachable!()
    };
    RulePackageInput {
        receivers: DeclaredSet::complete(vec![]),
        tables: vec![],
        schema_version: OWNED_RULE_PACKAGE_VERSION,
        namespace: ns(),
        release: key("rules"),
        semantics_version: key("integer-stat-v1"),
        operations_version: key(OWNED_RULE_OPERATIONS_VERSION),
        definitions: schema.identity().clone(),
        owners: vec![DefinitionRules {
            owner: owner(),
            programs: DeclaredSet::complete(vec![RuleProgram {
                id: key("constant"),
                context: RuleEntityKind::Actor,
                reads: vec![],
                nodes: vec![RuleNode {
                    id: key("value"),
                    expression: RuleExpression::Literal {
                        value: ParameterValue::Integer(BoundedInteger::new(3).unwrap()),
                    },
                }],
                effects: vec![RuleEffect {
                    id: key("derive"),
                    when: None,
                    effect: RuleEffectKind::Derive {
                        entity: RuleEntity::Current,
                        stat,
                        value: key("value"),
                    },
                }],
            }]),
        }],
    }
}
#[test]
fn strict_bound_package_roundtrip_and_data_only_identity_change() {
    let s = schema();
    let l = RuleStorageLimits::default();
    let i = input(&s);
    let p = OwnedRulePackage::new(i.clone(), &s, l).unwrap();
    let bytes = encode_rule_package(&p, l).unwrap();
    let restored = decode_rule_package(&bytes, &s, l).unwrap();
    assert_eq!(p.identity(), restored.identity());
    assert_eq!(p.input(), restored.input());
    assert_eq!(p.resources().programs, 1);
    assert_eq!(p.resources().edges, 1);
    let mut changed = i;
    changed.owners[0].programs.members[0].nodes[0].expression = RuleExpression::Literal {
        value: ParameterValue::Integer(BoundedInteger::new(5).unwrap()),
    };
    let changed = OwnedRulePackage::new(changed, &s, l).unwrap();
    assert_ne!(p.identity(), changed.identity());
    assert_eq!(p.definitions(), changed.definitions());
    // Serialization keys/order are owned; no source code, XML, callbacks or profiles.
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(value.get("source").is_none());
}
#[test]
fn unknown_duplicate_missing_fields_and_foreign_bindings_reject() {
    let s = schema();
    let l = RuleStorageLimits::default();
    let i = input(&s);
    let raw = serde_json::to_string(&i).unwrap();
    let duplicate = format!(
        "{{\"schema_version\":{},{}",
        OWNED_RULE_PACKAGE_VERSION,
        &raw[1..]
    );
    assert!(decode_rule_package(duplicate.as_bytes(), &s, l).is_err());
    let mut value = serde_json::to_value(&i).unwrap();
    value["unknown"] = true.into();
    assert!(decode_rule_package(&serde_json::to_vec(&value).unwrap(), &s, l).is_err());
    let mut value = serde_json::to_value(&i).unwrap();
    value.as_object_mut().unwrap().remove("operations_version");
    assert!(decode_rule_package(&serde_json::to_vec(&value).unwrap(), &s, l).is_err());
    let mut i = i.clone();
    i.definitions.content_sha256 = "ab".repeat(32);
    assert!(matches!(
        OwnedRulePackage::new(i, &s, l),
        Err(RuleStorageError::Binding)
    ));
    let mut i = input(&s);
    i.schema_version += 1;
    assert!(matches!(
        OwnedRulePackage::new(i, &s, l),
        Err(RuleStorageError::Version(_))
    ));
}
#[test]
fn local_duplicates_dangling_edges_and_owner_gaps_remain_errors() {
    let s = schema();
    let l = RuleStorageLimits::default();
    let mut i = input(&s);
    i.owners.push(i.owners[0].clone());
    assert!(OwnedRulePackage::new(i, &s, l).is_err());
    let mut i = input(&s);
    let program = i.owners[0].programs.members[0].clone();
    i.owners[0].programs.members.push(program);
    assert!(OwnedRulePackage::new(i, &s, l).is_err());
    let mut i = input(&s);
    let p = &mut i.owners[0].programs.members[0];
    p.nodes.push(p.nodes[0].clone());
    assert!(OwnedRulePackage::new(i, &s, l).is_err());
    let mut i = input(&s);
    i.owners[0].programs.members[0].effects[0].when = Some(key("missing"));
    assert!(OwnedRulePackage::new(i, &s, l).is_err());
    let mut i = input(&s);
    i.owners[0].programs.closure = SchemaClosure::Partial { gaps: vec![] };
    assert!(OwnedRulePackage::new(i, &s, l).is_err());
    let mut i = input(&s);
    i.owners[0].programs.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: owner(),
            facet: SchemaFacet::GameRules,
            code: key("unconverted"),
        }],
    };
    let p = OwnedRulePackage::new(i, &s, l).unwrap();
    assert!(!p.input().owners[0].programs.is_complete());
}
#[test]
fn tightened_aggregate_limits_are_checked_on_construction_decode_and_encode() {
    let s = schema();
    let l = RuleStorageLimits::default();
    let p = OwnedRulePackage::new(input(&s), &s, l).unwrap();
    let tiny = RuleStorageLimits {
        max_wire_bytes: 1,
        ..l
    };
    assert!(encode_rule_package(&p, tiny).is_err());
    assert!(decode_rule_package(&encode_rule_package(&p, l).unwrap(), &s, tiny).is_err());
    assert!(OwnedRulePackage::new(input(&s), &s, tiny).is_err());
    assert!(OwnedRulePackage::new(input(&s), &s, RuleStorageLimits { max_nodes: 0, ..l }).is_err());
    let mut i = input(&s);
    i.owners[0].programs.members[0].nodes.push(RuleNode {
        id: key("extra"),
        expression: RuleExpression::Literal {
            value: ParameterValue::Boolean(false),
        },
    });
    assert!(OwnedRulePackage::new(i, &s, RuleStorageLimits { max_nodes: 1, ..l }).is_err());
}

#[test]
fn closure_gaps_and_read_edges_share_aggregate_storage_limits() {
    let s = schema();
    let l = RuleStorageLimits::default();
    let mut i = input(&s);
    i.owners[0].programs.closure = SchemaClosure::Partial {
        gaps: ["first", "second"]
            .into_iter()
            .map(|code| SchemaGap {
                subject: owner(),
                facet: SchemaFacet::GameRules,
                code: key(code),
            })
            .collect(),
    };
    let p = OwnedRulePackage::new(i.clone(), &s, l).unwrap();
    assert_eq!(p.resources().gaps, 2);
    let tight = RuleStorageLimits { max_gaps: 1, ..l };
    assert!(matches!(
        OwnedRulePackage::new(i, &s, tight),
        Err(RuleStorageError::Limit("gaps"))
    ));
    assert!(matches!(
        decode_rule_package(&encode_rule_package(&p, l).unwrap(), &s, tight),
        Err(RuleStorageError::Limit("gaps"))
    ));
    assert!(matches!(
        encode_rule_package(&p, tight),
        Err(RuleStorageError::Limit("gaps"))
    ));

    let mut i = input(&s);
    let program = &mut i.owners[0].programs.members[0];
    program.reads.push(RuleRead {
        id: key("level"),
        value_type: ComputedValueType::Integer,
        source: RuleReadSource::CharacterLevel,
    });
    program.nodes[0].expression = RuleExpression::Read {
        input: key("level"),
    };
    let p = OwnedRulePackage::new(i.clone(), &s, l).unwrap();
    assert_eq!(p.resources().edges, 2);
    let tight = RuleStorageLimits { max_edges: 1, ..l };
    assert!(matches!(
        OwnedRulePackage::new(i, &s, tight),
        Err(RuleStorageError::Limit("edges"))
    ));
    assert!(matches!(
        decode_rule_package(&encode_rule_package(&p, l).unwrap(), &s, tight),
        Err(RuleStorageError::Limit("edges"))
    ));
    assert!(matches!(
        encode_rule_package(&p, tight),
        Err(RuleStorageError::Limit("edges"))
    ));
}

#[test]
fn unit_read_variants_reject_unknown_and_duplicate_wire_fields() {
    for source in [
        RuleReadSource::CharacterLevel,
        RuleReadSource::GemLevel,
        RuleReadSource::ItemLevel,
    ] {
        let raw = serde_json::to_string(&source).unwrap();
        assert_eq!(
            serde_json::from_str::<RuleReadSource>(&raw).unwrap(),
            source
        );
        let mut value = serde_json::to_value(&source).unwrap();
        value["surprise"] = true.into();
        assert!(serde_json::from_value::<RuleReadSource>(value).is_err());
        let kind = serde_json::to_value(&source).unwrap()["kind"].clone();
        let duplicate = format!("{{\"kind\":{kind},{}", &raw[1..]);
        assert!(serde_json::from_str::<RuleReadSource>(&duplicate).is_err());
    }
}

#[test]
fn operation_version_changes_identity_without_changing_storage_envelope() {
    let s = schema();
    let limits = RuleStorageLimits::default();
    let current = OwnedRulePackage::new(input(&s), &s, limits).unwrap();
    let mut previous = input(&s);
    previous.operations_version = key("owned-domain-operations-v4");
    // Data storage preserves an explicit operation contract; Engine decides
    // which version it can execute. Neither path silently upgrades the package.
    let previous = OwnedRulePackage::new(previous, &s, limits).unwrap();
    assert_ne!(current.identity(), previous.identity());
    assert_eq!(
        current.input().schema_version,
        previous.input().schema_version
    );
    let restored =
        decode_rule_package(&encode_rule_package(&previous, limits).unwrap(), &s, limits).unwrap();
    assert_eq!(
        restored.input().operations_version.as_str(),
        "owned-domain-operations-v4"
    );
}
