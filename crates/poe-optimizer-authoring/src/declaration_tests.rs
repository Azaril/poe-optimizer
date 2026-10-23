use crate::{declarations::*, *};
use poe_optimizer_core::{
    owned_build::ParameterValue, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::{
    OWNED_SCHEMA_PACKAGE_VERSION, OwnedDefinitionSchemaPackage, OwnedSchemaLimits,
    SchemaPackageInput, encode_schema_package,
};
use poe_optimizer_engine::owned_rules::{
    CompiledRulePackage, EffectDisposition, RuleFact, RuleLimits,
};
use poe_optimizer_import::owned_mapping::{OwnedIdRegistry, OwnedMappingLimits, RegistryInput};

fn namespace() -> GameVersionNamespace {
    GameVersionNamespace::new("stat-tests", "v1").unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn ports() -> DeclaredSlots {
    DeclaredSlots {
        parameters: DeclaredSet::complete(vec![]),
        choices: DeclaredSet::complete(vec![]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    }
}

struct Fixture {
    names: NamedIdRegistry,
    schema: OwnedDefinitionSchemaPackage,
    symbols: SymbolRegistry,
    unit: UnitDefId,
    actor: ActorDefId,
}
fn fixture() -> Fixture {
    let mut registry = OwnedIdRegistry::empty(namespace(), OwnedMappingLimits::default()).unwrap();
    let unit = registry.allocate_definition::<UnitDefinition>().unwrap();
    let actor = registry.allocate_definition::<ActorDefinition>().unwrap();
    let option = registry.allocate_definition::<OptionDefinition>().unwrap();
    let schema = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
            namespace: namespace(),
            release: key("baseline"),
            semantics_version: key("test"),
            definitions: vec![
                DefinitionDescriptor::Unit(DefinitionEntry {
                    id: unit.clone(),
                    schema: SchemaState::Known(UnitSchema {
                        dimension: UnitDimension::DimensionlessFactor,
                    }),
                }),
                DefinitionDescriptor::Actor(DefinitionEntry {
                    id: actor.clone(),
                    schema: SchemaState::Known(ActorSchema {
                        declarations: ports(),
                    }),
                }),
                DefinitionDescriptor::Option(DefinitionEntry {
                    id: option.clone(),
                    schema: SchemaState::Known(OptionSchema {}),
                }),
            ],
            slots: vec![],
        },
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    let names = NamedIdRegistry::empty(registry, AllocationLimits::default()).unwrap();
    let names = names
        .allocate(
            names.identity().unwrap(),
            &[
                NameRequest::Alias {
                    name: "factor".into(),
                    target: SchemaSubject::Definition(DefinitionAddress::Unit(unit.clone())),
                },
                NameRequest::Alias {
                    name: "owner".into(),
                    target: SchemaSubject::Definition(DefinitionAddress::Actor(actor.clone())),
                },
                NameRequest::Alias {
                    name: "choice".into(),
                    target: SchemaSubject::Definition(DefinitionAddress::Option(option)),
                },
            ],
        )
        .unwrap();
    let symbols = names.symbols(&schema, SymbolLimits::default()).unwrap();
    Fixture {
        names,
        schema,
        symbols,
        unit,
        actor,
    }
}

fn fixture_with_slots() -> (Fixture, SlotAddress, SlotAddress) {
    let mut f = fixture();
    let mut registry = f.names.registry().clone();
    let modifier = registry
        .allocate_definition::<ModifierDefinition>()
        .unwrap();
    let first = registry
        .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Modifier(modifier.clone()))
        .unwrap();
    let second = registry
        .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Modifier(modifier.clone()))
        .unwrap();
    let mut input = f.schema.input().clone();
    let mut declarations = ports();
    declarations.parameters = DeclaredSet::complete(vec![first.clone(), second.clone()]);
    input
        .definitions
        .push(DefinitionDescriptor::Modifier(DefinitionEntry {
            id: modifier,
            schema: SchemaState::Known(ModifierSchema { declarations }),
        }));
    for id in [first.clone(), second.clone()] {
        input.slots.push(SlotDescriptor::Parameter(DefinitionEntry {
            id,
            schema: SchemaState::Known(ParameterSlotSchema {
                value: ValueSchema::Integer(IntegerRange {
                    minimum: BoundedInteger::new(0).unwrap(),
                    maximum: BoundedInteger::new(1).unwrap(),
                }),
                presence: SlotPresence::RequiredOnce,
                sites: vec![ParameterSite::ModifierRoll],
                skill_input: None,
            }),
        }));
    }
    f.schema = OwnedDefinitionSchemaPackage::new(input, OwnedSchemaLimits::default()).unwrap();
    let mut input = f.names.input().clone();
    input.registry = registry.identity().unwrap();
    f.names = NamedIdRegistry::new(registry, input, AllocationLimits::default()).unwrap();
    let first = SlotAddress::Parameter(first);
    let second = SlotAddress::Parameter(second);
    f.names = f
        .names
        .allocate(
            f.names.identity().unwrap(),
            &[NameRequest::Alias {
                name: "flag".into(),
                target: SchemaSubject::Slot(first.clone()),
            }],
        )
        .unwrap();
    f.symbols = f.names.symbols(&f.schema, SymbolLimits::default()).unwrap();
    (f, first, second)
}

fn parse(body: &str) -> StatDeclarations {
    parse_stat_declarations(
        &format!("definitions game=\"stat-tests\" version=\"v1\" {{ {body} }}"),
        ManifestLimits::default(),
    )
    .unwrap()
}
fn stage(f: &Fixture, declarations: &StatDeclarations) -> StagedStatDeclarations {
    stage_stat_declarations(
        &f.names,
        f.names.identity().unwrap(),
        &f.schema,
        &f.symbols,
        declarations,
        DeclarationLimits::default(),
    )
    .unwrap()
}
fn bytes(
    names: &NamedIdRegistry,
    schema: &OwnedDefinitionSchemaPackage,
) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    (
        serde_json::to_vec(names.input()).unwrap(),
        serde_json::to_vec(names.registry().input()).unwrap(),
        encode_schema_package(schema, OwnedSchemaLimits::default()).unwrap(),
    )
}
fn failure_kind(error: DeclarationError) -> DeclarationErrorKind {
    let DeclarationError::Invalid { kind, .. } = error else {
        panic!("{error}")
    };
    kind
}

#[test]
fn stat_syntax_lowers_all_value_and_target_kinds_without_inferred_units() {
    let f = fixture();
    let declarations = parse(
        r#"
        stat "boolean_fact" type="boolean" { target "skill"; }
        stat "integer_fact" type="integer" {
            target "actor"; target "action"; target "equipment_use";
            target "modifier"; target "environment"; target "support_origin"; target "skill";
        }
        stat "option_fact" type="option" { target "actor"; }
        stat "quantity_fact" type="quantity" unit="factor" { target "action"; }
    "#,
    );
    let result = stage(&f, &declarations);
    for (name, expected) in [
        ("boolean_fact", ComputedValueType::Boolean),
        ("integer_fact", ComputedValueType::Integer),
        ("option_fact", ComputedValueType::Option),
        (
            "quantity_fact",
            ComputedValueType::Quantity {
                unit: f.unit.clone(),
            },
        ),
    ] {
        let Some(DefinitionAddress::Stat(id)) = result.symbols().definition(name) else {
            panic!("typed Stat")
        };
        let SchemaLookup::Known(stat) = result.schema().definition(id) else {
            panic!("known Stat")
        };
        assert_eq!(stat.value, expected);
        let expected_targets = declarations
            .stats
            .iter()
            .find(|stat| stat.name == name)
            .unwrap();
        assert_eq!(stat.targets, expected_targets.targets);
    }
    assert_eq!(declarations.stats[1].targets.len(), 7);
    assert_eq!(
        result.schema().input().namespace,
        f.schema.input().namespace
    );
    assert_eq!(result.schema().input().release, f.schema.input().release);
    assert_eq!(
        result.schema().input().semantics_version,
        f.schema.input().semantics_version
    );
    for descriptor in &f.schema.input().definitions {
        assert_eq!(
            result.schema().lookup_definition(&descriptor.address()),
            Some(descriptor)
        );
    }
    assert_eq!(
        result.symbols().definition("factor"),
        f.symbols.definition("factor")
    );
}

#[test]
fn declaration_parser_rejects_ambiguous_or_unsupported_shapes_and_exact_source_limit() {
    for body in [
        r#"stat "a" type="float" { target "skill"; }"#,
        r#"stat "a" type="quantity" { target "skill"; }"#,
        r#"stat "a" type="boolean" unit="factor" { target "skill"; }"#,
        r#"stat "a" type="boolean" { target "player"; }"#,
        r#"stat "a" type="boolean" { target "skill"; target "skill"; }"#,
        r#"stat "a" type="boolean""#,
        r#"stat "bad-name" type="boolean" { target "skill"; }"#,
        r#"stat "a" type="boolean" { target "skill"; }; stat "a" type="boolean" { target "skill"; }"#,
        r#"stat "a" type="boolean" type="integer" { target "skill"; }"#,
        r#"stat "a" type="boolean" { target "skill" extra="x"; }"#,
        r#"stat "a" type="boolean" { target "skill" { target "actor"; }; }"#,
        r#"include "other.kdl""#,
    ] {
        let text = format!("definitions game=\"stat-tests\" version=\"v1\" {{ {body}; }}");
        assert!(
            parse_stat_declarations(&text, ManifestLimits::default()).is_err(),
            "{body}"
        );
    }
    for text in [
        r#"definitions game="bad game" version="v1" {}"#,
        r#"definitions game="stat-tests" version="v1" extra="x" {}"#,
        r#"definitions game="stat-tests" version="v1" {}; definitions game="stat-tests" version="v1" {}"#,
    ] {
        assert!(parse_stat_declarations(text, ManifestLimits::default()).is_err());
    }
    let text = r#"definitions game="stat-tests" version="v1" { stat "flag" type="boolean" { target "skill"; }; }"#;
    let limits = ManifestLimits {
        max_source_bytes: text.len(),
        ..Default::default()
    };
    assert!(parse_stat_declarations(text, limits).is_ok());
    assert_eq!(
        parse_stat_declarations(
            text,
            ManifestLimits {
                max_source_bytes: text.len() - 1,
                ..limits
            }
        )
        .unwrap_err()
        .kind,
        ManifestErrorKind::SourceBytes
    );
}

#[test]
fn allocations_are_order_stable_persisted_replayable_and_monotone() {
    let f = fixture();
    let before = bytes(&f.names, &f.schema);
    let declarations = parse(
        r#"
        stat "z_flag" type="boolean" { target "skill"; }
        stat "a_count" type="integer" { target "actor"; target "action"; }
    "#,
    );
    let first = stage(&f, &declarations);
    let mut reversed = declarations.clone();
    reversed.stats.reverse();
    for stat in &mut reversed.stats {
        stat.targets.reverse();
    }
    let permuted = stage(&f, &reversed);
    assert_eq!(
        bytes(first.names(), first.schema()),
        bytes(permuted.names(), permuted.schema())
    );
    assert_eq!(bytes(&f.names, &f.schema), before);
    assert_eq!(
        first.names().registry().input().last_issued.get(),
        f.names.registry().input().last_issued.get() + 2
    );
    f.names.validate_successor(first.names()).unwrap();

    let (saved_names, saved_registry, _) = bytes(first.names(), first.schema());
    let restored = NamedIdRegistry::new(
        OwnedIdRegistry::new(
            serde_json::from_slice::<RegistryInput>(&saved_registry).unwrap(),
            OwnedMappingLimits::default(),
        )
        .unwrap(),
        serde_json::from_slice::<NamesInput>(&saved_names).unwrap(),
        AllocationLimits::default(),
    )
    .unwrap();
    let retry = stage_stat_declarations(
        &restored,
        restored.identity().unwrap(),
        &f.schema,
        &f.symbols,
        &declarations,
        DeclarationLimits::default(),
    )
    .unwrap();
    assert_eq!(
        bytes(retry.names(), retry.schema()),
        bytes(first.names(), first.schema())
    );
    let mut extended = declarations.clone();
    extended.stats.push(StatDeclaration {
        name: "earlier_name".into(),
        value: StatValueDeclaration::Option,
        targets: vec![RuleEntityKind::Actor],
    });
    let successor = stage_stat_declarations(
        &restored,
        restored.identity().unwrap(),
        &f.schema,
        &f.symbols,
        &extended,
        DeclarationLimits::default(),
    )
    .unwrap();
    restored.validate_successor(successor.names()).unwrap();
    for name in ["z_flag", "a_count"] {
        assert_eq!(
            successor.symbols().definition(name),
            first.symbols().definition(name)
        );
    }
    assert_eq!(
        successor.names().registry().input().last_issued.get(),
        restored.registry().input().last_issued.get() + 1
    );
    let error = stage_stat_declarations(
        &restored,
        f.names.identity().unwrap(),
        &f.schema,
        &f.symbols,
        &extended,
        DeclarationLimits::default(),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        DeclarationError::Allocation(AllocationError::Invalid {
            kind: AllocationErrorKind::StalePredecessor,
            ..
        })
    ));
    assert_eq!(serde_json::to_vec(restored.input()).unwrap(), saved_names);
    assert_eq!(
        serde_json::to_vec(restored.registry().input()).unwrap(),
        saved_registry
    );
}

#[test]
fn invalid_declarations_and_late_failures_leave_both_inputs_unchanged() {
    let f = fixture();
    let before = bytes(&f.names, &f.schema);
    let valid = parse(r#"stat "flag" type="boolean" { target "actor"; }"#);
    let mut cases = Vec::new();
    let mut duplicate = valid.clone();
    duplicate.stats.push(duplicate.stats[0].clone());
    cases.push((duplicate, DeclarationErrorKind::DuplicateName));
    let mut foreign = valid.clone();
    foreign.namespace = GameVersionNamespace::new("other", "v1").unwrap();
    cases.push((foreign, DeclarationErrorKind::NamespaceMismatch));
    let mut no_targets = valid.clone();
    no_targets.stats[0].targets.clear();
    cases.push((no_targets, DeclarationErrorKind::InvalidTargets));
    let mut duplicate_target = valid.clone();
    duplicate_target.stats[0]
        .targets
        .push(RuleEntityKind::Actor);
    cases.push((duplicate_target, DeclarationErrorKind::InvalidTargets));
    let mut missing_unit = valid.clone();
    missing_unit.stats[0].value = StatValueDeclaration::Quantity {
        unit: "absent".into(),
    };
    cases.push((missing_unit, DeclarationErrorKind::UnknownUnit));
    let mut wrong_unit = valid.clone();
    wrong_unit.stats[0].value = StatValueDeclaration::Quantity {
        unit: "owner".into(),
    };
    cases.push((wrong_unit, DeclarationErrorKind::WrongKind));
    let mut collision = valid.clone();
    collision.stats[0].name = "factor".into();
    cases.push((collision.clone(), DeclarationErrorKind::SymbolConflict));
    for (declarations, expected) in cases {
        assert_eq!(
            failure_kind(
                stage_stat_declarations(
                    &f.names,
                    f.names.identity().unwrap(),
                    &f.schema,
                    &f.symbols,
                    &declarations,
                    DeclarationLimits::default()
                )
                .unwrap_err()
            ),
            expected
        );
        assert_eq!(bytes(&f.names, &f.schema), before);
    }
    let no_symbols = SymbolRegistry::new(&f.schema, &[], &[], SymbolLimits::default()).unwrap();
    assert_eq!(
        failure_kind(
            stage_stat_declarations(
                &f.names,
                f.names.identity().unwrap(),
                &f.schema,
                &no_symbols,
                &collision,
                DeclarationLimits::default()
            )
            .unwrap_err()
        ),
        DeclarationErrorKind::WrongKind
    );

    let staged = stage(&f, &valid);
    for value in [StatValueDeclaration::Boolean, StatValueDeclaration::Integer] {
        let mut redeclare = valid.clone();
        redeclare.stats[0].value = value;
        assert_eq!(
            failure_kind(
                stage_stat_declarations(
                    staged.names(),
                    staged.names().identity().unwrap(),
                    staged.schema(),
                    &f.symbols,
                    &redeclare,
                    DeclarationLimits::default()
                )
                .unwrap_err()
            ),
            DeclarationErrorKind::ExistingDefinition
        );
    }
    // Distinct spellings that already reserve one address cannot emit it twice.
    let target = SchemaSubject::Definition(staged.symbols().definition("flag").unwrap().clone());
    let aliased = staged
        .names()
        .allocate(
            staged.names().identity().unwrap(),
            &[NameRequest::Alias {
                name: "flag_alias".into(),
                target,
            }],
        )
        .unwrap();
    let mut duplicate_address = valid.clone();
    duplicate_address.stats.push(StatDeclaration {
        name: "flag_alias".into(),
        ..valid.stats[0].clone()
    });
    assert_eq!(
        failure_kind(
            stage_stat_declarations(
                &aliased,
                aliased.identity().unwrap(),
                &f.schema,
                &f.symbols,
                &duplicate_address,
                DeclarationLimits::default()
            )
            .unwrap_err()
        ),
        DeclarationErrorKind::DuplicateAddress
    );
    assert_eq!(bytes(&f.names, &f.schema), before);
}

#[test]
fn baseline_addresses_must_be_active_registry_members_and_symbols_current() {
    let f = fixture();
    let declarations = parse(r#"stat "flag" type="boolean" { target "actor"; }"#);
    let empty = NamedIdRegistry::empty(
        OwnedIdRegistry::empty(namespace(), OwnedMappingLimits::default()).unwrap(),
        AllocationLimits::default(),
    )
    .unwrap();
    assert_eq!(
        failure_kind(
            stage_stat_declarations(
                &empty,
                empty.identity().unwrap(),
                &f.schema,
                &f.symbols,
                &declarations,
                DeclarationLimits::default()
            )
            .unwrap_err()
        ),
        DeclarationErrorKind::UnregisteredBase
    );
    let mut registry = f.names.registry().clone();
    registry
        .retire(
            &SchemaSubject::Definition(DefinitionAddress::Unit(f.unit.clone())),
            key("retired"),
        )
        .unwrap();
    let mut input = f.names.input().clone();
    input.registry = registry.identity().unwrap();
    let retired = NamedIdRegistry::new(registry, input, AllocationLimits::default()).unwrap();
    assert_eq!(
        failure_kind(
            stage_stat_declarations(
                &retired,
                retired.identity().unwrap(),
                &f.schema,
                &f.symbols,
                &declarations,
                DeclarationLimits::default()
            )
            .unwrap_err()
        ),
        DeclarationErrorKind::UnregisteredBase
    );
    let staged = stage(&f, &declarations);
    assert!(matches!(
        stage_stat_declarations(
            staged.names(),
            staged.names().identity().unwrap(),
            &f.schema,
            staged.symbols(),
            &declarations,
            DeclarationLimits::default()
        ),
        Err(DeclarationError::SymbolIndex(SymbolErrorKind::Missing))
    ));
}

#[test]
fn supplied_aliases_must_agree_with_persisted_names_but_need_not_all_be_persisted() {
    let f = fixture();
    let declarations = parse(r#"stat "flag" type="boolean" { target "actor"; }"#);
    let conflicting = SymbolRegistry::new(
        &f.schema,
        &[("factor", DefinitionAddress::Actor(f.actor.clone()))],
        &[],
        SymbolLimits::default(),
    )
    .unwrap();
    assert_eq!(
        failure_kind(
            stage_stat_declarations(
                &f.names,
                f.names.identity().unwrap(),
                &f.schema,
                &conflicting,
                &declarations,
                DeclarationLimits::default(),
            )
            .unwrap_err()
        ),
        DeclarationErrorKind::SymbolConflict
    );
    let extra = SymbolRegistry::new(
        &f.schema,
        &[("factor_alias", DefinitionAddress::Unit(f.unit.clone()))],
        &[],
        SymbolLimits::default(),
    )
    .unwrap();
    let quantity =
        parse(r#"stat "amount" type="quantity" unit="factor_alias" { target "actor"; }"#);
    let staged = stage_stat_declarations(
        &f.names,
        f.names.identity().unwrap(),
        &f.schema,
        &extra,
        &quantity,
        DeclarationLimits::default(),
    )
    .unwrap();
    assert_eq!(
        staged.symbols().definition("factor_alias"),
        extra.definition("factor_alias")
    );
    assert!(
        !staged
            .names()
            .input()
            .bindings
            .iter()
            .any(|binding| binding.name == "factor_alias")
    );
}

#[test]
fn packet_symbols_share_only_declared_stats_and_preserve_local_alias_scopes() {
    let f = fixture();
    let declaration_symbols = SymbolRegistry::new(
        &f.schema,
        &[("declaration_unit", DefinitionAddress::Unit(f.unit.clone()))],
        &[],
        SymbolLimits::default(),
    )
    .unwrap();
    let declarations =
        parse(r#"stat "amount" type="quantity" unit="declaration_unit" { target "actor"; }"#);
    let staged = stage_stat_declarations(
        &f.names,
        f.names.identity().unwrap(),
        &f.schema,
        &declaration_symbols,
        &declarations,
        DeclarationLimits::default(),
    )
    .unwrap();
    let before = bytes(staged.names(), staged.schema());
    let amount = staged.symbols().definition("amount").unwrap().clone();
    for local in [
        DefinitionAddress::Actor(f.actor.clone()),
        DefinitionAddress::Unit(f.unit.clone()),
    ] {
        let aliases = SymbolRegistry::new(
            &f.schema,
            &[("local", local.clone())],
            &[],
            SymbolLimits::default(),
        )
        .unwrap();
        let projected = staged
            .symbols_for_packet(&aliases, SymbolLimits::default())
            .unwrap();
        assert_eq!(projected.definition("local"), Some(&local));
        assert_eq!(projected.definition("amount"), Some(&amount));
        for hidden in ["declaration_unit", "factor", "owner", "choice"] {
            assert!(projected.definition(hidden).is_none(), "{hidden}");
        }
        assert_eq!(projected.definitions().count(), 2);
        assert!(aliases.definition("amount").is_none());
    }
    let aliases = SymbolRegistry::new(
        staged.schema(),
        &[("amount", amount.clone())],
        &[],
        SymbolLimits::default(),
    )
    .unwrap();
    let projected = staged
        .symbols_for_packet(
            &aliases,
            SymbolLimits {
                max_symbols: 1,
                max_name_bytes: "amount".len(),
            },
        )
        .unwrap();
    assert_eq!(projected.definitions().count(), 1);
    assert_eq!(projected.definition("amount"), Some(&amount));
    let (names, schema, symbols) = staged.into_parts();
    assert_eq!(bytes(&names, &schema), before);
    assert_eq!(symbols.definition("amount"), Some(&amount));
    assert!(symbols.definition("declaration_unit").is_some());
}

#[test]
fn packet_alias_collisions_require_exact_addresses_with_independent_slot_names() {
    let (f, first, second) = fixture_with_slots();
    let staged = stage(
        &f,
        &parse(r#"stat "flag" type="boolean" { target "actor"; }"#),
    );
    let flag = staged.symbols().definition("flag").unwrap().clone();
    let aliases = SymbolRegistry::new(
        staged.schema(),
        &[
            ("flag", flag.clone()),
            ("owner", DefinitionAddress::Actor(f.actor.clone())),
        ],
        &[("flag", first.clone()), ("owner", second.clone())],
        SymbolLimits::default(),
    )
    .unwrap();
    let projected = staged
        .symbols_for_packet(
            &aliases,
            SymbolLimits {
                max_symbols: 4,
                max_name_bytes: "owner".len(),
            },
        )
        .unwrap();
    assert_eq!(projected.definition("flag"), Some(&flag));
    assert_eq!(projected.slot("flag"), Some(&first));
    assert_eq!(
        projected.definition("owner"),
        Some(&DefinitionAddress::Actor(f.actor.clone()))
    );
    assert_eq!(projected.slot("owner"), Some(&second));
    let before = bytes(staged.names(), staged.schema());
    for wrong_name in ["flag", "owner"] {
        let wrong = SymbolRegistry::new(
            staged.schema(),
            &[(wrong_name, DefinitionAddress::Unit(f.unit.clone()))],
            &[],
            SymbolLimits::default(),
        )
        .unwrap();
        assert_eq!(
            failure_kind(
                staged
                    .symbols_for_packet(&wrong, SymbolLimits::default())
                    .unwrap_err()
            ),
            DeclarationErrorKind::SymbolConflict
        );
    }
    let wrong_slot = SymbolRegistry::new(
        staged.schema(),
        &[],
        &[("flag", second)],
        SymbolLimits::default(),
    )
    .unwrap();
    assert_eq!(
        failure_kind(
            staged
                .symbols_for_packet(&wrong_slot, SymbolLimits::default())
                .unwrap_err()
        ),
        DeclarationErrorKind::SymbolConflict
    );
    assert_eq!(bytes(staged.names(), staged.schema()), before);
    assert_eq!(aliases.slot("flag"), Some(&first));
}

#[test]
fn reserved_names_remain_reserved_without_becoming_packet_symbols() {
    let f = fixture();
    let reserved = f
        .names
        .allocate(
            f.names.identity().unwrap(),
            &[NameRequest::Definition {
                name: "reserved".into(),
                kind: DefinitionKind::Stat,
            }],
        )
        .unwrap();
    let reserved_binding = reserved
        .input()
        .bindings
        .iter()
        .find(|binding| binding.name == "reserved")
        .unwrap()
        .clone();
    let staged = stage_stat_declarations(
        &reserved,
        reserved.identity().unwrap(),
        &f.schema,
        &f.symbols,
        &parse(r#"stat "flag" type="boolean" { target "actor"; }"#),
        DeclarationLimits::default(),
    )
    .unwrap();
    let empty = SymbolRegistry::new(&f.schema, &[], &[], SymbolLimits::default()).unwrap();
    let projected = staged
        .symbols_for_packet(&empty, SymbolLimits::default())
        .unwrap();
    assert_eq!(projected.definitions().count(), 1);
    assert!(projected.definition("flag").is_some());
    assert!(projected.definition("reserved").is_none());
    assert!(staged.names().input().bindings.contains(&reserved_binding));
    let SchemaSubject::Definition(reserved_address) = &reserved_binding.target else {
        panic!("reserved definition")
    };
    assert!(
        staged
            .schema()
            .lookup_definition(reserved_address)
            .is_none()
    );
    let conflict = SymbolRegistry::new(
        staged.schema(),
        &[("reserved", DefinitionAddress::Actor(f.actor.clone()))],
        &[],
        SymbolLimits::default(),
    )
    .unwrap();
    assert_eq!(
        failure_kind(
            staged
                .symbols_for_packet(&conflict, SymbolLimits::default())
                .unwrap_err()
        ),
        DeclarationErrorKind::SymbolConflict
    );
}

#[test]
fn packet_symbol_limits_and_schema_are_rechecked_without_mutating_staged_inputs() {
    let f = fixture();
    let staged = stage(
        &f,
        &parse(
            r#"
            stat "long_flag" type="boolean" { target "actor"; }
            stat "other" type="integer" { target "skill"; }
        "#,
        ),
    );
    let before = bytes(staged.names(), staged.schema());
    let aliases = SymbolRegistry::new(
        staged.schema(),
        &[(
            "long_flag",
            staged.symbols().definition("long_flag").unwrap().clone(),
        )],
        &[],
        SymbolLimits::default(),
    )
    .unwrap();
    let exact = SymbolLimits {
        max_symbols: 2,
        max_name_bytes: "long_flag".len(),
    };
    assert_eq!(
        staged
            .symbols_for_packet(&aliases, exact)
            .unwrap()
            .definitions()
            .count(),
        2
    );
    for limits in [
        SymbolLimits {
            max_symbols: 1,
            ..exact
        },
        SymbolLimits {
            max_name_bytes: exact.max_name_bytes - 1,
            ..exact
        },
        SymbolLimits {
            max_symbols: 0,
            ..exact
        },
        SymbolLimits {
            max_name_bytes: 0,
            ..exact
        },
    ] {
        assert!(staged.symbols_for_packet(&aliases, limits).is_err());
        assert_eq!(bytes(staged.names(), staged.schema()), before);
    }
    // A symbol registry checked against a different staged successor cannot
    // smuggle that successor's extra definitions into this packet.
    let successor = stage_stat_declarations(
        staged.names(),
        staged.names().identity().unwrap(),
        staged.schema(),
        &f.symbols,
        &parse(r#"stat "later" type="boolean" { target "actor"; }"#),
        DeclarationLimits::default(),
    )
    .unwrap();
    let foreign = SymbolRegistry::new(
        successor.schema(),
        &[(
            "local",
            successor.symbols().definition("later").unwrap().clone(),
        )],
        &[],
        SymbolLimits::default(),
    )
    .unwrap();
    assert!(matches!(
        staged.symbols_for_packet(&foreign, SymbolLimits::default()),
        Err(DeclarationError::SymbolIndex(SymbolErrorKind::Missing))
    ));
    assert_eq!(bytes(staged.names(), staged.schema()), before);
    assert_eq!(aliases.definitions().count(), 1);
    assert!(aliases.definition("other").is_none());
}

#[test]
fn declaration_limits_accept_exact_counts_and_bytes_and_reject_one_less() {
    let f = fixture();
    let declarations =
        parse(r#"stat "long_flag" type="boolean" { target "actor"; target "skill"; }"#);
    let staged = stage(&f, &declarations);
    let mut exact = DeclarationLimits {
        max_declarations: 1,
        max_targets: 2,
        max_registry_checks: 4,
        max_names_bytes: serde_json::to_vec(staged.names().input()).unwrap().len(),
        symbols: SymbolLimits {
            max_symbols: 4,
            max_name_bytes: "long_flag".len(),
        },
        ..Default::default()
    };
    exact.registry.max_entries = 4;
    exact.registry.max_collection_entries = 4;
    exact.registry.max_wire_bytes = serde_json::to_vec(staged.names().registry().input())
        .unwrap()
        .len();
    exact.schema.max_entries = 6; // Four descriptors plus two target memberships.
    exact.schema.max_collection_entries = 4;
    exact.schema.max_wire_bytes =
        encode_schema_package(staged.schema(), OwnedSchemaLimits::default())
            .unwrap()
            .len();
    let result = stage_stat_declarations(
        &f.names,
        f.names.identity().unwrap(),
        &f.schema,
        &f.symbols,
        &declarations,
        exact,
    )
    .unwrap();
    assert_eq!(
        bytes(result.names(), result.schema()),
        bytes(staged.names(), staged.schema())
    );
    let before = bytes(&f.names, &f.schema);
    for reduce in 0..10 {
        let mut limit = exact;
        match reduce {
            0 => limit.max_targets -= 1,
            1 => limit.max_registry_checks -= 1,
            2 => limit.max_names_bytes -= 1,
            3 => limit.symbols.max_symbols -= 1,
            4 => limit.symbols.max_name_bytes -= 1,
            5 => limit.registry.max_entries -= 1,
            6 => limit.registry.max_collection_entries -= 1,
            7 => limit.registry.max_wire_bytes -= 1,
            8 => limit.schema.max_entries -= 1,
            9 => limit.schema.max_wire_bytes -= 1,
            _ => unreachable!(),
        }
        assert!(
            stage_stat_declarations(
                &f.names,
                f.names.identity().unwrap(),
                &f.schema,
                &f.symbols,
                &declarations,
                limit
            )
            .is_err(),
            "budget {reduce}"
        );
        assert_eq!(bytes(&f.names, &f.schema), before);
    }
    let mut two = declarations.clone();
    two.stats.push(StatDeclaration {
        name: "other".into(),
        ..two.stats[0].clone()
    });
    assert_eq!(
        failure_kind(
            stage_stat_declarations(
                &f.names,
                f.names.identity().unwrap(),
                &f.schema,
                &f.symbols,
                &two,
                exact
            )
            .unwrap_err()
        ),
        DeclarationErrorKind::LimitExceeded
    );
}

#[test]
fn declared_stats_lower_and_execute_through_engine_without_closing_partial_owner_coverage() {
    let f = fixture();
    let staged = stage(
        &f,
        &parse(
            r#"
        stat "stat.initial" type="boolean" { target "actor"; }
        stat "stat.output" type="boolean" { target "actor"; }
    "#,
        ),
    );
    let source = r#"program "produce" context actor {
        read "initial" as initial: boolean = stat(current, stat.initial);
        let result = initial;
        effect "output" = derive(current, stat.output, result);
    }"#;
    let program = lower_program(
        &parse_program(source, ProgramParseLimits::default()).unwrap(),
        staged.schema(),
        staged.symbols(),
        &[],
        ProgramLowerLimits::default(),
    )
    .unwrap()
    .program;
    let owner = SchemaSubject::Definition(DefinitionAddress::Actor(f.actor.clone()));
    let closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: owner.clone(),
            facet: SchemaFacet::GameRules,
            code: key("other-rules"),
        }],
    };
    let rules = RulePackageInput {
        schema_version: OWNED_RULE_PACKAGE_VERSION,
        namespace: namespace(),
        release: key("test-rules"),
        semantics_version: key("test"),
        operations_version: key(OWNED_RULE_OPERATIONS_V14),
        definitions: staged.schema().identity().clone(),
        tables: vec![],
        owners: vec![DefinitionRules {
            owner: owner.clone(),
            programs: DeclaredSet {
                members: vec![program],
                closure: closure.clone(),
            },
        }],
        receivers: DeclaredSet::complete(vec![]),
        effect_applications: None,
        contribution_queries: None,
        existing_actor_rules: None,
        support_discovery: None,
    };
    let compiled =
        CompiledRulePackage::compile(&rules, staged.schema(), RuleLimits::default()).unwrap();
    let mut scratch = compiled.new_scratch();
    for input in [Some(true), None, Some(false), Some(true)] {
        let facts: Vec<_> = input
            .into_iter()
            .map(|value| RuleFact {
                read: key("initial"),
                value: ParameterValue::Boolean(value),
            })
            .collect();
        let report = compiled
            .evaluate(
                &owner,
                &key("produce"),
                &facts,
                staged.schema(),
                &mut scratch,
            )
            .unwrap();
        assert_eq!(report.owner_programs_closure, closure);
        assert_eq!(report.effects.len(), 1);
        assert_eq!(
            report.effects[0].disposition,
            match input {
                Some(value) => EffectDisposition::Applied {
                    value: ParameterValue::Boolean(value)
                },
                None => EffectDisposition::Unresolved {
                    input: key("initial")
                },
            }
        );
        let Some(DefinitionAddress::Stat(output)) = staged.symbols().definition("stat.output")
        else {
            panic!("Stat output")
        };
        assert!(
            matches!(&report.effects[0].effect, RuleEffectKind::Derive { stat, .. } if stat == output)
        );
    }
    assert_eq!(compiled.input().owners[0].programs.closure, closure);
}
