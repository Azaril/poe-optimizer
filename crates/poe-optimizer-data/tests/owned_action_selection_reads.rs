use poe_optimizer_core::{
    owned_build::ParameterValue, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::{owned_rules::*, owned_schema::*};

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("selection-storage", "v1").unwrap()
}
fn id<T: DefinitionDomain>(value: &str) -> DefId<T> {
    DefId::new(ns(), key(value))
}
fn known<T: DefinitionDomain, S>(id: DefId<T>, schema: S) -> DefinitionEntry<DefId<T>, S> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn fixture() -> (OwnedDefinitionSchemaPackage, RulePackageInput) {
    let schema = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
            namespace: ns(),
            release: key("schema"),
            semantics_version: key("selection-storage-v1"),
            definitions: vec![
                DefinitionDescriptor::Stat(known(
                    id("result"),
                    StatSchema {
                        value: ComputedValueType::Boolean,
                        targets: vec![RuleEntityKind::Action],
                    },
                )),
                DefinitionDescriptor::ActionPart(known(id("part"), ActionPartSchema {})),
                DefinitionDescriptor::ActionMode(known(id("mode"), ActionModeSchema {})),
                DefinitionDescriptor::ActionStatSet(known(id("set"), ActionStatSetSchema {})),
                DefinitionDescriptor::ActionStatSet(known(id("other-set"), ActionStatSetSchema {})),
            ],
            slots: vec![],
        },
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    let reads = vec![
        RuleRead {
            id: key("part"),
            value_type: ComputedValueType::Boolean,
            source: RuleReadSource::ActionPartIs { part: id("part") },
        },
        RuleRead {
            id: key("mode"),
            value_type: ComputedValueType::Boolean,
            source: RuleReadSource::ActionModeIs { mode: id("mode") },
        },
        RuleRead {
            id: key("set"),
            value_type: ComputedValueType::Boolean,
            source: RuleReadSource::ActionStatSetIs {
                stat_set: id("set"),
            },
        },
    ];
    let mut nodes: Vec<_> = reads
        .iter()
        .map(|r| RuleNode {
            id: r.id.clone(),
            expression: RuleExpression::Read {
                input: r.id.clone(),
            },
        })
        .collect();
    nodes.push(RuleNode {
        id: key("all"),
        expression: RuleExpression::All {
            values: reads.iter().map(|r| r.id.clone()).collect(),
        },
    });
    let input = RulePackageInput {
        existing_actor_rules: None,
        ordered_contributions: None,
        schema_version: OWNED_RULE_PACKAGE_VERSION,
        namespace: ns(),
        release: key("rules"),
        semantics_version: key("selection-storage-v1"),
        operations_version: key(OWNED_RULE_OPERATIONS_V20),
        definitions: schema.identity().clone(),
        receivers: DeclaredSet::complete(vec![]),
        tables: vec![],
        effect_applications: Some(DeclaredSet::complete(vec![])),
        owners: vec![DefinitionRules {
            owner: SchemaSubject::Definition(DefinitionAddress::Stat(id("result"))),
            programs: DeclaredSet::complete(vec![RuleProgram {
                id: key("selection"),
                context: RuleEntityKind::Action,
                reads,
                nodes,
                effects: vec![RuleEffect {
                    id: key("result"),
                    when: None,
                    effect: RuleEffectKind::Derive {
                        entity: RuleEntity::Current,
                        stat: id("result"),
                        value: key("all"),
                    },
                }],
            }]),
        }],
    };
    (schema, input)
}

#[test]
fn v20_selection_reads_roundtrip_and_commit_exact_selector_identity() {
    let (schema, input) = fixture();
    let limits = RuleStorageLimits::default();
    let package = OwnedRulePackage::new(input.clone(), &schema, limits).unwrap();
    let bytes = encode_rule_package(&package, limits).unwrap();
    let decoded = decode_rule_package(&bytes, &schema, limits).unwrap();
    assert_eq!(package.input(), decoded.input());
    assert_eq!(package.identity(), decoded.identity());
    assert_eq!(package.resources().reads, 3);
    let mut changed = input;
    changed.owners[0].programs.members[0].reads[2].source = RuleReadSource::ActionStatSetIs {
        stat_set: id("other-set"),
    };
    let changed = OwnedRulePackage::new(changed, &schema, limits).unwrap();
    assert_ne!(package.identity(), changed.identity());
}

#[test]
fn selection_authority_is_not_available_in_old_or_unknown_operations() {
    let (schema, input) = fixture();
    for version in (1..=19).chain([21, 999]) {
        let mut old = input.clone();
        old.operations_version = key(&format!("owned-domain-operations-v{version}"));
        assert!(matches!(
            OwnedRulePackage::new(old, &schema, RuleStorageLimits::default()),
            Err(RuleStorageError::Structure(
                "action selection reads require owned-domain-operations-v20"
            ))
        ));
    }
    assert_eq!(OWNED_RULE_OPERATIONS_VERSION, OWNED_RULE_OPERATIONS_V14);
    let mut legacy = input;
    legacy.operations_version = key(OWNED_RULE_OPERATIONS_VERSION);
    legacy.effect_applications = None;
    let program = &mut legacy.owners[0].programs.members[0];
    program.reads.clear();
    program.nodes = vec![RuleNode {
        id: key("all"),
        expression: RuleExpression::Literal {
            value: ParameterValue::Boolean(true),
        },
    }];
    let package = OwnedRulePackage::new(legacy, &schema, RuleStorageLimits::default()).unwrap();
    let bytes = encode_rule_package(&package, RuleStorageLimits::default()).unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["operations_version"], OWNED_RULE_OPERATIONS_V14);
    assert_eq!(
        json["owners"][0]["programs"]["members"][0]["reads"],
        serde_json::json!([])
    );
}

#[test]
fn selection_storage_rejects_wrong_context_type_wire_and_read_budget() {
    let (schema, input) = fixture();
    for context in [
        RuleEntityKind::Actor,
        RuleEntityKind::Skill,
        RuleEntityKind::SupportOrigin,
        RuleEntityKind::EquipmentUse,
    ] {
        let mut wrong = input.clone();
        wrong.owners[0].programs.members[0].context = context;
        assert!(matches!(
            OwnedRulePackage::new(wrong, &schema, RuleStorageLimits::default()),
            Err(RuleStorageError::Structure(
                "action selection reads require Action context and Boolean type"
            ))
        ));
    }
    for index in 0..3 {
        let mut wrong = input.clone();
        wrong.owners[0].programs.members[0].reads[index].value_type = ComputedValueType::Integer;
        assert!(OwnedRulePackage::new(wrong, &schema, RuleStorageLimits::default()).is_err());
    }
    let limits = RuleStorageLimits {
        max_reads: 2,
        ..RuleStorageLimits::default()
    };
    assert!(OwnedRulePackage::new(input.clone(), &schema, limits).is_err());
    let bytes = serde_json::to_vec(&input).unwrap();
    assert!(decode_rule_package(&bytes, &schema, limits).is_err());
    let exact = RuleStorageLimits {
        max_reads: 3,
        ..RuleStorageLimits::default()
    };
    assert!(decode_rule_package(&bytes, &schema, exact).is_ok());
    let mut wire = serde_json::to_value(input).unwrap();
    wire["owners"][0]["programs"]["members"][0]["reads"][0]["source"]["value"]["unexpected"] =
        true.into();
    assert!(decode_rule_package(&serde_json::to_vec(&wire).unwrap(), &schema, exact).is_err());
}
