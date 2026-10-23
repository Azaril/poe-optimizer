//! Source-free storage, compatibility and resource laws for injected preparation data.
use poe_optimizer_core::{
    owned_definitions::*, owned_rules::*, owned_schema::*, owned_supports::*,
};
use poe_optimizer_data::{owned_rules::*, owned_schema::*, owned_supports::*};

fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("support-data", "v1").unwrap()
}
fn id<K: DefinitionDomain>(s: &str) -> DefId<K> {
    DefId::new(ns(), key(s))
}
fn empty<T>() -> DeclaredSet<T> {
    DeclaredSet::complete(vec![])
}
fn fixture() -> (
    OwnedDefinitionSchemaPackage,
    OwnedRulePackage,
    SupportPreparationInput,
) {
    let gem = GemSchema {
        level: IntegerRange {
            minimum: BoundedInteger::new(1).unwrap(),
            maximum: BoundedInteger::new(100).unwrap(),
        },
        roles: vec![AuthoredGemRole::SupportAssignment],
        skills: empty(),
        quality: QualityUseSchema {
            allowed_kinds: empty(),
            presence: QualityPresence::Optional,
        },
        declarations: DeclaredSlots {
            parameters: empty(),
            choices: empty(),
            grants: empty(),
            actors: empty(),
            skill_grants: empty(),
            outputs: empty(),
            sockets: empty(),
        },
    };
    let schema = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
            namespace: ns(),
            release: key("schema"),
            semantics_version: key("v1"),
            definitions: vec![
                DefinitionDescriptor::Gem(DefinitionEntry {
                    id: id("gem-a"),
                    schema: SchemaState::Known(gem.clone()),
                }),
                DefinitionDescriptor::Gem(DefinitionEntry {
                    id: id("gem-b"),
                    schema: SchemaState::Known(gem),
                }),
                DefinitionDescriptor::Unit(DefinitionEntry {
                    id: id("quality"),
                    schema: SchemaState::Known(UnitSchema {
                        dimension: UnitDimension::PercentagePoints,
                    }),
                }),
            ],
            slots: vec![],
        },
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    let rules = OwnedRulePackage::new(
        RulePackageInput {
            support_discovery: None,
            existing_actor_rules: None,
            contribution_queries: None,
            effect_applications: None,
            schema_version: OWNED_RULE_PACKAGE_VERSION,
            namespace: ns(),
            release: key("rules"),
            semantics_version: key("v1"),
            operations_version: key(OWNED_RULE_OPERATIONS_V14),
            definitions: schema.identity().clone(),
            tables: vec![],
            owners: vec![],
            receivers: empty(),
        },
        &schema,
        RuleStorageLimits::default(),
    )
    .unwrap();
    let input = SupportPreparationInput {
        schema_version: OWNED_SUPPORT_PREPARATION_VERSION,
        namespace: ns(),
        release: key("preparation"),
        definitions: schema.identity().clone(),
        rules: *rules.identity(),
        policy: SupportPreparationPolicy::OrderedReplacementRetryFrontierV1,
        quality_unit: id("quality"),
        types: vec![key("spell"), key("duration")],
        effects: vec![key("effect-b"), key("effect-a")],
        families: vec![key("family-b"), key("family-a")],
        supports: ["gem-b", "gem-a"]
            .map(|name| SupportPreparationEntry {
                gem: id(name),
                preparation: SchemaState::Known(SupportPreparationDefinition {
                    effect: key(if name == "gem-a" {
                        "effect-a"
                    } else {
                        "effect-b"
                    }),
                    families: Some(vec![key("family-b"), key("family-a")]),
                    plus_version_of: None,
                    requires: Some(SupportTypePredicate::All(vec![
                        SupportTypePredicate::Type(key("spell")),
                        SupportTypePredicate::Not(Box::new(SupportTypePredicate::Type(key(
                            "duration",
                        )))),
                    ])),
                    excludes: None,
                    added_types: vec![key("spell"), key("duration")],
                    gems_only: true,
                    from_item: false,
                    is_support: true,
                    is_trigger: false,
                    ignore_minion_types: false,
                }),
            })
            .to_vec(),
    };
    (schema, rules, input)
}
fn first(input: &mut SupportPreparationInput) -> &mut SupportPreparationDefinition {
    let SchemaState::Known(v) = &mut input.supports[0].preparation else {
        panic!()
    };
    v
}

#[test]
fn canonical_membership_roundtrip_and_semantic_identity() {
    let (schema, rules, input) = fixture();
    let limits = SupportStorageLimits::default();
    let package = OwnedSupportPreparation::new(input.clone(), &schema, &rules, limits).unwrap();
    let bytes = encode_support_preparation(&package, limits).unwrap();
    let restored = decode_support_preparation(&bytes, &schema, &rules, limits).unwrap();
    assert_eq!(package.identity(), restored.identity());
    let mut reordered = input.clone();
    reordered.types.reverse();
    reordered.effects.reverse();
    reordered.families.reverse();
    reordered.supports.reverse();
    for entry in &mut reordered.supports {
        if let SchemaState::Known(v) = &mut entry.preparation {
            v.added_types.reverse();
            v.families.as_mut().unwrap().reverse();
        }
    }
    assert_eq!(
        encode_support_preparation(
            &OwnedSupportPreparation::new(reordered, &schema, &rules, limits).unwrap(),
            limits
        )
        .unwrap(),
        bytes
    );
    let mut absent = input.clone();
    first(&mut absent).families = None;
    let mut present_empty = absent.clone();
    first(&mut present_empty).families = Some(vec![]);
    assert_ne!(
        OwnedSupportPreparation::new(absent, &schema, &rules, limits)
            .unwrap()
            .identity(),
        OwnedSupportPreparation::new(present_empty, &schema, &rules, limits)
            .unwrap()
            .identity()
    );
    let mut changed = input;
    first(&mut changed).gems_only = false;
    assert_ne!(
        package.identity(),
        OwnedSupportPreparation::new(changed, &schema, &rules, limits)
            .unwrap()
            .identity()
    );
}

#[test]
fn missing_null_unknown_and_duplicate_wire_fields_are_distinct() {
    let (schema, rules, input) = fixture();
    let limits = SupportStorageLimits::default();
    for field in ["families", "plus_version_of", "requires", "excludes"] {
        let mut value = serde_json::to_value(&input).unwrap();
        value["supports"][0]["preparation"]["value"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(
            decode_support_preparation(
                &serde_json::to_vec(&value).unwrap(),
                &schema,
                &rules,
                limits
            )
            .is_err(),
            "missing {field}"
        );
    }
    let mut value = serde_json::to_value(&input).unwrap();
    value["policy"] = "future-policy".into();
    assert!(
        decode_support_preparation(
            &serde_json::to_vec(&value).unwrap(),
            &schema,
            &rules,
            limits
        )
        .is_err()
    );
    let mut value = serde_json::to_value(&input).unwrap();
    value["unknown"] = true.into();
    assert!(
        decode_support_preparation(
            &serde_json::to_vec(&value).unwrap(),
            &schema,
            &rules,
            limits
        )
        .is_err()
    );
    let raw = serde_json::to_string(&input).unwrap();
    let duplicate = format!("{{\"schema_version\":1,{}", &raw[1..]);
    assert!(decode_support_preparation(duplicate.as_bytes(), &schema, &rules, limits).is_err());
}

#[test]
fn references_bind_exact_schema_rules_and_declared_vocabulary() {
    let (schema, rules, input) = fixture();
    let limits = SupportStorageLimits::default();
    let mut bad = input.clone();
    bad.definitions.content_sha256 = "ab".repeat(32);
    assert!(matches!(
        OwnedSupportPreparation::new(bad, &schema, &rules, limits),
        Err(SupportStorageError::Binding)
    ));
    let mut other_rules = rules.input().clone();
    other_rules.release = key("changed");
    let other_rules =
        OwnedRulePackage::new(other_rules, &schema, RuleStorageLimits::default()).unwrap();
    assert!(matches!(
        OwnedSupportPreparation::new(input.clone(), &schema, &other_rules, limits),
        Err(SupportStorageError::Binding)
    ));
    let package = OwnedSupportPreparation::new(input.clone(), &schema, &rules, limits).unwrap();
    assert!(package.verify_bindings(&schema, &other_rules).is_err());
    let mut bad = input.clone();
    bad.types.push(bad.types[0].clone());
    assert!(OwnedSupportPreparation::new(bad, &schema, &rules, limits).is_err());
    let mut bad = input.clone();
    first(&mut bad).plus_version_of = Some(key("missing"));
    assert!(OwnedSupportPreparation::new(bad, &schema, &rules, limits).is_err());
    let mut bad = input.clone();
    first(&mut bad).requires = Some(SupportTypePredicate::Type(key("missing")));
    assert!(OwnedSupportPreparation::new(bad, &schema, &rules, limits).is_err());
    let mut bad = input;
    bad.supports[0].gem = DefId::new(
        GameVersionNamespace::new("foreign", "v1").unwrap(),
        key("gem-b"),
    );
    assert!(OwnedSupportPreparation::new(bad, &schema, &rules, limits).is_err());
}

#[test]
fn unmapped_and_missing_definitions_do_not_become_empty_facts() {
    let (schema, rules, mut input) = fixture();
    let limits = SupportStorageLimits::default();
    let gem = input.supports[0].gem.clone();
    input.supports[0].preparation = SchemaState::Unmapped {
        gaps: vec![SchemaGap {
            subject: SchemaSubject::Definition(gem.address()),
            facet: SchemaFacet::StaticLinks,
            code: key("unconverted"),
        }],
    };
    let package = OwnedSupportPreparation::new(input.clone(), &schema, &rules, limits).unwrap();
    assert!(matches!(
        package.preparation_for(&gem),
        Some(SchemaState::Unmapped { .. })
    ));
    assert!(package.preparation_for(&id("absent")).is_none());
    input.supports[0].preparation = SchemaState::Unmapped { gaps: vec![] };
    assert!(OwnedSupportPreparation::new(input, &schema, &rules, limits).is_err());
}

#[test]
fn same_effect_facts_must_agree_after_membership_canonicalization() {
    let (schema, rules, mut input) = fixture();
    let limits = SupportStorageLimits::default();
    let SchemaState::Known(v) = &mut input.supports[1].preparation else {
        panic!()
    };
    v.effect = key("effect-b");
    v.families.as_mut().unwrap().reverse();
    v.added_types.reverse();
    OwnedSupportPreparation::new(input.clone(), &schema, &rules, limits).unwrap();
    first(&mut input).is_trigger = true;
    assert!(matches!(
        OwnedSupportPreparation::new(input, &schema, &rules, limits),
        Err(SupportStorageError::Invalid(
            "conflicting facts for the same support effect"
        ))
    ));
}

#[test]
fn construction_decoding_and_encoding_enforce_the_same_tighter_limits() {
    let (schema, rules, input) = fixture();
    let limits = SupportStorageLimits::default();
    let package = OwnedSupportPreparation::new(input.clone(), &schema, &rules, limits).unwrap();
    let bytes = encode_support_preparation(&package, limits).unwrap();
    let used = package.resources();
    let exact = SupportStorageLimits {
        max_entries: used.entries,
        max_work: used.work,
        max_predicate_depth: used.predicate_depth,
        max_wire_bytes: bytes.len(),
    };
    OwnedSupportPreparation::new(input.clone(), &schema, &rules, exact).unwrap();
    decode_support_preparation(&bytes, &schema, &rules, exact).unwrap();
    assert_eq!(encode_support_preparation(&package, exact).unwrap(), bytes);
    for smaller in [
        SupportStorageLimits {
            max_entries: used.entries - 1,
            ..exact
        },
        SupportStorageLimits {
            max_work: used.work - 1,
            ..exact
        },
        SupportStorageLimits {
            max_predicate_depth: used.predicate_depth - 1,
            ..exact
        },
        SupportStorageLimits {
            max_wire_bytes: bytes.len() - 1,
            ..exact
        },
    ] {
        assert!(OwnedSupportPreparation::new(input.clone(), &schema, &rules, smaller).is_err());
        assert!(decode_support_preparation(&bytes, &schema, &rules, smaller).is_err());
        assert!(encode_support_preparation(&package, smaller).is_err());
    }
}

#[test]
fn authored_preparation_preserves_absence_predicates_and_unmapped_rows() {
    let (schema, rules, mut input) = fixture();
    first(&mut input).families = None;
    let other = input.supports[1].gem.clone();
    input.supports[1].preparation = SchemaState::Unmapped {
        gaps: vec![SchemaGap {
            subject: SchemaSubject::Definition(other.address()),
            facet: SchemaFacet::StaticLinks,
            code: key("unconverted"),
        }],
    };
    let original =
        OwnedSupportPreparation::new(input, &schema, &rules, Default::default()).unwrap();
    let content = original.clone().into_authored_content();
    assert_eq!(
        serde_json::from_value::<SupportPreparationContent>(
            serde_json::to_value(&content).unwrap()
        )
        .unwrap(),
        content
    );
    let rebound =
        OwnedSupportPreparation::bind_new(content.clone(), &schema, &rules, Default::default())
            .unwrap();
    assert_eq!(rebound.input(), original.input());
    assert_eq!(rebound.identity(), original.identity());
    assert!(matches!(
        rebound.preparation_for(&other),
        Some(SchemaState::Unmapped { .. })
    ));
    let mut present_empty = content.clone();
    let row = present_empty
        .supports
        .iter_mut()
        .find_map(|row| match &mut row.preparation {
            SchemaState::Known(value) => Some(value),
            _ => None,
        })
        .unwrap();
    row.families = Some(vec![]);
    assert_ne!(
        OwnedSupportPreparation::bind_new(present_empty, &schema, &rules, Default::default())
            .unwrap()
            .identity(),
        original.identity()
    );
    assert_eq!(original.into_authored_content(), content);
}

#[test]
fn authored_preparation_binds_actual_dependencies_without_accepting_missing_facts() {
    let (schema, rules, input) = fixture();
    let original =
        OwnedSupportPreparation::new(input, &schema, &rules, Default::default()).unwrap();
    let content = original.clone().into_authored_content();
    let mut extended = schema.input().clone();
    extended
        .definitions
        .push(DefinitionDescriptor::Stat(DefinitionEntry {
            id: id("appended"),
            schema: SchemaState::Known(StatSchema {
                value: ComputedValueType::Boolean,
                targets: vec![RuleEntityKind::Skill],
            }),
        }));
    let extended = OwnedDefinitionSchemaPackage::new(extended, Default::default()).unwrap();
    assert!(matches!(
        OwnedSupportPreparation::bind_new(content.clone(), &extended, &rules, Default::default()),
        Err(SupportStorageError::Binding)
    ));
    let mut rule_input = rules.input().clone();
    rule_input.definitions = extended.identity().clone();
    let extended_rules = OwnedRulePackage::new(rule_input, &extended, Default::default()).unwrap();
    let rebound = OwnedSupportPreparation::bind_new(
        content.clone(),
        &extended,
        &extended_rules,
        Default::default(),
    )
    .unwrap();
    assert_eq!(rebound.input().definitions, *extended.identity());
    assert_eq!(rebound.input().rules, *extended_rules.identity());
    assert_eq!(rebound.into_authored_content(), content);
    let mut invalid = content.clone();
    let SchemaState::Known(row) = &mut invalid.supports[0].preparation else {
        panic!()
    };
    row.requires = Some(SupportTypePredicate::Type(key("undeclared")));
    assert!(matches!(
        OwnedSupportPreparation::bind_new(invalid, &extended, &extended_rules, Default::default()),
        Err(SupportStorageError::Invalid(
            "undeclared preparation symbol"
        ))
    ));
    assert_eq!(original.into_authored_content(), content);
}

#[test]
fn authored_preparation_keeps_strict_fields_and_recursive_limits() {
    let (schema, rules, input) = fixture();
    let original =
        OwnedSupportPreparation::new(input, &schema, &rules, Default::default()).unwrap();
    let content = original.clone().into_authored_content();
    assert!(
        serde_json::from_value::<SupportPreparationContent>(
            serde_json::to_value(original.input()).unwrap()
        )
        .is_err()
    );
    for field in ["families", "plus_version_of", "requires", "excludes"] {
        let mut value = serde_json::to_value(&content).unwrap();
        value["supports"][0]["preparation"]["value"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(
            serde_json::from_value::<SupportPreparationContent>(value).is_err(),
            "{field}"
        );
    }
    let used = original.resources();
    let exact = SupportStorageLimits {
        max_entries: used.entries,
        max_work: used.work,
        max_predicate_depth: used.predicate_depth,
        max_wire_bytes: encode_support_preparation(&original, Default::default())
            .unwrap()
            .len(),
    };
    assert_eq!(
        OwnedSupportPreparation::bind_new(content.clone(), &schema, &rules, exact)
            .unwrap()
            .identity(),
        original.identity()
    );
    for limits in [
        SupportStorageLimits {
            max_entries: exact.max_entries - 1,
            ..exact
        },
        SupportStorageLimits {
            max_work: exact.max_work - 1,
            ..exact
        },
        SupportStorageLimits {
            max_predicate_depth: exact.max_predicate_depth - 1,
            ..exact
        },
        SupportStorageLimits {
            max_wire_bytes: exact.max_wire_bytes - 1,
            ..exact
        },
    ] {
        assert!(
            OwnedSupportPreparation::bind_new(content.clone(), &schema, &rules, limits).is_err()
        );
    }
    assert_eq!(original.into_authored_content(), content);
}
