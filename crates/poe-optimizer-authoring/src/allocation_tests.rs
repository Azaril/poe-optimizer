use crate::*;
use poe_optimizer_core::{owned_definitions::*, owned_schema::*};
use poe_optimizer_data::owned_schema::{
    OWNED_SCHEMA_PACKAGE_VERSION, OwnedDefinitionSchemaPackage, OwnedSchemaLimits,
    SchemaPackageInput,
};
use poe_optimizer_import::owned_mapping::{OwnedIdRegistry, OwnedMappingLimits};

fn namespace() -> GameVersionNamespace {
    GameVersionNamespace::new("named-allocation-tests", "v1").unwrap()
}

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}

fn registry() -> OwnedIdRegistry {
    OwnedIdRegistry::empty(namespace(), OwnedMappingLimits::default()).unwrap()
}

fn empty() -> NamedIdRegistry {
    NamedIdRegistry::empty(registry(), AllocationLimits::default()).unwrap()
}

fn definition(name: &str, kind: DefinitionKind) -> NameRequest {
    NameRequest::Definition {
        name: name.into(),
        kind,
    }
}

fn slot(name: &str, kind: DefinitionKind, owner: &str) -> NameRequest {
    NameRequest::Slot {
        name: name.into(),
        kind,
        owner: owner.into(),
    }
}

fn alias(name: &str, target: SchemaSubject) -> NameRequest {
    NameRequest::Alias {
        name: name.into(),
        target,
    }
}

fn allocate(base: &NamedIdRegistry, requests: &[NameRequest]) -> NamedIdRegistry {
    base.allocate(base.identity().unwrap(), requests).unwrap()
}

fn target(base: &NamedIdRegistry, name: &str, is_slot: bool) -> SchemaSubject {
    base.input()
        .bindings
        .iter()
        .find(|row| row.name == name && matches!(row.target, SchemaSubject::Slot(_)) == is_slot)
        .unwrap()
        .target
        .clone()
}

fn snapshot(base: &NamedIdRegistry) -> (Vec<u8>, Vec<u8>) {
    (
        serde_json::to_vec(base.input()).unwrap(),
        serde_json::to_vec(base.registry().input()).unwrap(),
    )
}

fn unit_schema(units: &[UnitDefId]) -> OwnedDefinitionSchemaPackage {
    OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
            namespace: namespace(),
            release: key("allocation-tests"),
            semantics_version: key("test"),
            definitions: units
                .iter()
                .map(|id| {
                    DefinitionDescriptor::Unit(DefinitionEntry {
                        id: id.clone(),
                        schema: SchemaState::Known(UnitSchema {
                            dimension: UnitDimension::DimensionlessFactor,
                        }),
                    })
                })
                .collect(),
            slots: vec![],
        },
        OwnedSchemaLimits::default(),
    )
    .unwrap()
}

#[test]
fn named_allocation_is_order_independent_forward_resolving_and_replayable() {
    let base = empty();
    let original = snapshot(&base);
    // The same name is legal in the definition and declared-slot spaces.
    // The slot precedes its owner's declaration in the authored request list.
    let requests = vec![
        slot("shared", DefinitionKind::ParameterSlot, "owner"),
        definition("z_unit", DefinitionKind::Unit),
        definition("shared", DefinitionKind::Unit),
        definition("owner", DefinitionKind::Modifier),
    ];
    let staged = allocate(&base, &requests);
    let mut reversed = requests.clone();
    reversed.reverse();
    let reordered = allocate(&base, &reversed);
    assert_eq!(snapshot(&staged), snapshot(&reordered));
    assert_eq!(staged.identity().unwrap(), reordered.identity().unwrap());
    assert_eq!(snapshot(&base), original);
    assert_eq!(staged.registry().input().last_issued.get(), 4);
    assert_eq!(
        staged
            .input()
            .bindings
            .iter()
            .map(|row| (
                matches!(row.target, SchemaSubject::Slot(_)),
                row.name.as_str()
            ))
            .collect::<Vec<_>>(),
        [
            (false, "owner"),
            (false, "shared"),
            (false, "z_unit"),
            (true, "shared")
        ]
    );
    let SchemaSubject::Definition(DefinitionAddress::Modifier(owner)) =
        target(&staged, "owner", false)
    else {
        panic!("modifier owner lost its exact kind");
    };
    let SchemaSubject::Slot(SlotAddress::Parameter(parameter)) = target(&staged, "shared", true)
    else {
        panic!("parameter lost its declared-slot identity");
    };
    assert_eq!(parameter.declaration, SlotOwnerDefId::Modifier(owner));
    assert_eq!(snapshot(&allocate(&staged, &requests)), snapshot(&staged));
    assert_eq!(snapshot(&allocate(&staged, &[])), snapshot(&staged));
    let decoded: NamesInput = serde_json::from_slice(&snapshot(&staged).0).unwrap();
    let restored = NamedIdRegistry::new(
        staged.registry().clone(),
        decoded,
        AllocationLimits::default(),
    )
    .unwrap();
    assert_eq!(restored.identity().unwrap(), staged.identity().unwrap());
}

#[test]
fn aliases_reuse_exact_addresses_without_allocating_and_bind_stale_checks() {
    let base = allocate(&empty(), &[definition("unit", DefinitionKind::Unit)]);
    let address = target(&base, "unit", false);
    let prior_identity = base.identity().unwrap();
    let staged = allocate(&base, &[alias("unit_alias", address.clone())]);
    assert_eq!(staged.registry().input(), base.registry().input());
    assert_eq!(target(&staged, "unit_alias", false), address);
    assert_ne!(staged.identity().unwrap(), prior_identity);
    assert!(
        staged
            .allocate(prior_identity, &[definition("later", DefinitionKind::Unit)])
            .is_err()
    );
    assert_eq!(
        snapshot(&allocate(&staged, &[alias("unit_alias", address)])),
        snapshot(&staged)
    );
}

#[test]
fn names_cannot_change_kind_owner_or_exact_alias_target() {
    let base = allocate(
        &empty(),
        &[
            definition("owner", DefinitionKind::Modifier),
            definition("other_owner", DefinitionKind::Modifier),
            definition("unit", DefinitionKind::Unit),
            definition("other_unit", DefinitionKind::Unit),
            slot("roll", DefinitionKind::ParameterSlot, "owner"),
        ],
    );
    let original = snapshot(&base);
    for request in [
        definition("unit", DefinitionKind::Stat),
        slot("roll", DefinitionKind::ChoiceSlot, "owner"),
        slot("roll", DefinitionKind::ParameterSlot, "other_owner"),
        alias("unit", target(&base, "other_unit", false)),
        definition("invalid_definition", DefinitionKind::ParameterSlot),
        slot("invalid_slot", DefinitionKind::Unit, "owner"),
        slot("invalid_owner", DefinitionKind::ParameterSlot, "unit"),
    ] {
        assert!(base.allocate(base.identity().unwrap(), &[request]).is_err());
        assert_eq!(snapshot(&base), original);
    }
}

#[test]
fn duplicate_names_in_a_request_batch_fail_even_when_targets_agree() {
    let base = allocate(
        &empty(),
        &[
            definition("owner", DefinitionKind::Modifier),
            definition("unit", DefinitionKind::Unit),
        ],
    );
    for requests in [
        vec![
            definition("unit", DefinitionKind::Unit),
            definition("unit", DefinitionKind::Unit),
        ],
        vec![
            definition("unit", DefinitionKind::Unit),
            alias("unit", target(&base, "unit", false)),
        ],
        vec![
            slot("roll", DefinitionKind::ParameterSlot, "owner"),
            slot("roll", DefinitionKind::ParameterSlot, "owner"),
        ],
    ] {
        assert!(base.allocate(base.identity().unwrap(), &requests).is_err());
    }
}

#[test]
fn staged_allocations_do_not_authorize_predicted_alias_targets_or_mutate_on_failure() {
    let base = empty();
    let original = snapshot(&base);
    // Derive the future address with the sole allocator, rather than duplicate
    // its key-generation algorithm in this test. It is absent from the base.
    let mut independent = base.registry().clone();
    let predicted = independent.allocate_definition::<UnitDefinition>().unwrap();
    let requests = [
        definition("a_new_unit", DefinitionKind::Unit),
        alias(
            "z_alias",
            SchemaSubject::Definition(DefinitionAddress::Unit(predicted)),
        ),
    ];
    assert!(base.allocate(base.identity().unwrap(), &requests).is_err());
    assert_eq!(snapshot(&base), original);
    assert!(
        base.allocate(
            base.identity().unwrap(),
            &[
                definition("a_new_owner", DefinitionKind::Modifier),
                slot("z_roll", DefinitionKind::ParameterSlot, "missing_owner"),
            ],
        )
        .is_err()
    );
    assert_eq!(snapshot(&base), original);
}

#[test]
fn snapshots_require_current_registry_exact_targets_and_unique_names() {
    let base = allocate(&empty(), &[definition("unit", DefinitionKind::Unit)]);
    let valid = base.input().clone();
    let mut wrong_version = valid.clone();
    wrong_version.schema_version = 2;
    let mut stale = valid.clone();
    stale.registry = registry().identity().unwrap();
    let mut duplicate = valid.clone();
    duplicate.bindings.push(duplicate.bindings[0].clone());
    let mut unknown = valid.clone();
    unknown.bindings[0].target = SchemaSubject::Definition(DefinitionAddress::Unit(
        UnitDefId::parse(namespace(), "unknown").unwrap(),
    ));
    let mut foreign = valid.clone();
    let SchemaSubject::Definition(DefinitionAddress::Unit(unit)) = target(&base, "unit", false)
    else {
        unreachable!();
    };
    foreign.bindings[0].target =
        SchemaSubject::Definition(DefinitionAddress::Unit(UnitDefId::new(
            GameVersionNamespace::new("another-game", "v1").unwrap(),
            unit.key().clone(),
        )));
    for input in [wrong_version, stale, duplicate, unknown, foreign] {
        assert!(
            NamedIdRegistry::new(base.registry().clone(), input, AllocationLimits::default())
                .is_err()
        );
    }
    let mut shuffled = allocate(&base, &[alias("second", target(&base, "unit", false))])
        .input()
        .clone();
    shuffled.bindings.reverse();
    let normalized = NamedIdRegistry::new(
        base.registry().clone(),
        shuffled,
        AllocationLimits::default(),
    )
    .unwrap();
    assert_eq!(normalized.input().bindings[0].name, "second");
    assert_eq!(normalized.input().bindings[1].name, "unit");
}

#[test]
fn malformed_names_are_rejected_in_snapshots_and_every_request_position() {
    let base = allocate(&empty(), &[definition("owner", DefinitionKind::Modifier)]);
    for name in ["", "1name", "a..b", "a.", ".a", "let", "a.true", "a-b", "é"] {
        for request in [
            definition(name, DefinitionKind::Unit),
            slot(name, DefinitionKind::ParameterSlot, "owner"),
            alias(name, target(&base, "owner", false)),
            slot("roll", DefinitionKind::ParameterSlot, name),
        ] {
            assert!(
                base.allocate(base.identity().unwrap(), &[request]).is_err(),
                "accepted malformed name {name:?}"
            );
        }
        let mut input = base.input().clone();
        input.bindings[0].name = name.into();
        assert!(
            NamedIdRegistry::new(base.registry().clone(), input, AllocationLimits::default())
                .is_err()
        );
    }
}

#[test]
fn retired_names_remain_reserved_but_are_absent_from_execution_symbols() {
    let base = allocate(
        &empty(),
        &[
            definition("active", DefinitionKind::Unit),
            definition("retired", DefinitionKind::Unit),
        ],
    );
    let retired = target(&base, "retired", false);
    let mut registry = base.registry().clone();
    registry
        .retire(&retired, key("removed-definition"))
        .unwrap();
    let mut input = base.input().clone();
    input.registry = registry.identity().unwrap();
    let names = NamedIdRegistry::new(registry, input, AllocationLimits::default()).unwrap();
    assert_eq!(names.input().bindings.len(), 2);
    assert_eq!(target(&names, "retired", false), retired);
    for request in [
        definition("retired", DefinitionKind::Unit),
        alias("new_alias", retired.clone()),
        alias("retired", target(&names, "active", false)),
    ] {
        assert!(
            names
                .allocate(names.identity().unwrap(), &[request])
                .is_err()
        );
    }
    let SchemaSubject::Definition(DefinitionAddress::Unit(active)) =
        target(&names, "active", false)
    else {
        unreachable!();
    };
    let schema = unit_schema(&[active]);
    let symbols = names.symbols(&schema, SymbolLimits::default()).unwrap();
    assert!(symbols.definition("active").is_some());
    assert!(symbols.definition("retired").is_none());
    assert_eq!(symbols.definitions().count(), 1);
    assert!(
        NamedIdRegistry::new(
            names.registry().clone(),
            names.input().clone(),
            AllocationLimits {
                symbols: SymbolLimits {
                    max_symbols: 1,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .is_err()
    );
}

#[test]
fn loaded_successors_preserve_bindings_and_retired_name_reservations() {
    let base = allocate(
        &empty(),
        &[
            definition("first", DefinitionKind::Unit),
            definition("second", DefinitionKind::Unit),
        ],
    );
    let mut missing = base.input().clone();
    missing.bindings.remove(0);
    let missing = NamedIdRegistry::new(
        base.registry().clone(),
        missing,
        AllocationLimits::default(),
    )
    .unwrap();
    assert!(base.validate_successor(&missing).is_err());

    let mut rebound = base.input().clone();
    rebound.bindings[0].target = target(&base, "second", false);
    let rebound = NamedIdRegistry::new(
        base.registry().clone(),
        rebound,
        AllocationLimits::default(),
    )
    .unwrap();
    assert!(base.validate_successor(&rebound).is_err());

    let mut retired_registry = base.registry().clone();
    retired_registry
        .retire(&target(&base, "first", false), key("retired-definition"))
        .unwrap();
    let mut retired_input = base.input().clone();
    retired_input.registry = retired_registry.identity().unwrap();
    let retained = NamedIdRegistry::new(
        retired_registry.clone(),
        retired_input.clone(),
        AllocationLimits::default(),
    )
    .unwrap();
    base.validate_successor(&retained).unwrap();
    assert!(retained.validate_successor(&base).is_err());

    retired_input.bindings.remove(0);
    let forgotten =
        NamedIdRegistry::new(retired_registry, retired_input, AllocationLimits::default()).unwrap();
    assert!(base.validate_successor(&forgotten).is_err());
    assert!(retained.validate_successor(&forgotten).is_err());
}

#[test]
fn empty_and_all_retired_names_still_require_the_exact_schema_namespace() {
    let empty = empty();
    let matching = unit_schema(&[]);
    let mut foreign = matching.input().clone();
    foreign.namespace = GameVersionNamespace::new("another-game", "v1").unwrap();
    let foreign = OwnedDefinitionSchemaPackage::new(foreign, OwnedSchemaLimits::default()).unwrap();
    assert!(empty.symbols(&matching, SymbolLimits::default()).is_ok());
    assert!(empty.symbols(&foreign, SymbolLimits::default()).is_err());

    let populated = allocate(&empty, &[definition("unit", DefinitionKind::Unit)]);
    let mut registry = populated.registry().clone();
    registry
        .retire(
            &target(&populated, "unit", false),
            key("retired-definition"),
        )
        .unwrap();
    let mut input = populated.input().clone();
    input.registry = registry.identity().unwrap();
    let retired = NamedIdRegistry::new(registry, input, AllocationLimits::default()).unwrap();
    assert!(retired.symbols(&matching, SymbolLimits::default()).is_ok());
    assert!(retired.symbols(&foreign, SymbolLimits::default()).is_err());
}

#[test]
fn symbols_require_known_exact_schema_including_the_declared_slot_owner() {
    let names = allocate(
        &empty(),
        &[
            definition("owner", DefinitionKind::Modifier),
            slot("roll", DefinitionKind::ParameterSlot, "owner"),
        ],
    );
    assert!(
        names
            .symbols(&unit_schema(&[]), SymbolLimits::default())
            .is_err()
    );
    let SchemaSubject::Definition(DefinitionAddress::Modifier(owner)) =
        target(&names, "owner", false)
    else {
        unreachable!();
    };
    let SchemaSubject::Slot(SlotAddress::Parameter(roll)) = target(&names, "roll", true) else {
        unreachable!();
    };
    let input = SchemaPackageInput {
        schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
        namespace: namespace(),
        release: key("allocation-tests"),
        semantics_version: key("test"),
        definitions: vec![DefinitionDescriptor::Modifier(DefinitionEntry {
            id: owner,
            schema: SchemaState::Known(ModifierSchema {
                declarations: DeclaredSlots {
                    parameters: DeclaredSet::complete(vec![roll.clone()]),
                    choices: DeclaredSet::complete(vec![]),
                    grants: DeclaredSet::complete(vec![]),
                    actors: DeclaredSet::complete(vec![]),
                    skill_grants: DeclaredSet::complete(vec![]),
                    outputs: DeclaredSet::complete(vec![]),
                    sockets: DeclaredSet::complete(vec![]),
                },
            }),
        })],
        slots: vec![SlotDescriptor::Parameter(DefinitionEntry {
            id: roll.clone(),
            schema: SchemaState::Known(ParameterSlotSchema {
                value: ValueSchema::Integer(IntegerRange {
                    minimum: BoundedInteger::new(0).unwrap(),
                    maximum: BoundedInteger::new(100).unwrap(),
                }),
                presence: SlotPresence::RequiredOnce,
                sites: vec![ParameterSite::ModifierRoll],
                skill_input: None,
            }),
        })],
    };
    let schema = OwnedDefinitionSchemaPackage::new(input, OwnedSchemaLimits::default()).unwrap();
    let symbols = names.symbols(&schema, SymbolLimits::default()).unwrap();
    assert_eq!(symbols.slot("roll"), Some(&SlotAddress::Parameter(roll)));
    assert!(symbols.definition("owner").is_some());
    let mut unresolved = schema.input().clone();
    let DefinitionDescriptor::Modifier(owner) = &mut unresolved.definitions[0] else {
        unreachable!();
    };
    owner.schema = SchemaState::Unmapped {
        gaps: vec![SchemaGap {
            subject: SchemaSubject::Definition(DefinitionAddress::Modifier(owner.id.clone())),
            facet: SchemaFacet::InputSchema,
            code: key("not-defined"),
        }],
    };
    let unresolved =
        OwnedDefinitionSchemaPackage::new(unresolved, OwnedSchemaLimits::default()).unwrap();
    assert!(names.symbols(&unresolved, SymbolLimits::default()).is_err());
}

#[test]
fn symbol_projection_checks_its_own_count_name_and_zero_limits() {
    let base = allocate(&empty(), &[definition("unit", DefinitionKind::Unit)]);
    let names = allocate(&base, &[alias("alias", target(&base, "unit", false))]);
    let SchemaSubject::Definition(DefinitionAddress::Unit(unit)) = target(&names, "unit", false)
    else {
        unreachable!();
    };
    let schema = unit_schema(&[unit]);
    let exact = SymbolLimits {
        max_symbols: 2,
        max_name_bytes: 5,
    };
    assert_eq!(
        names.symbols(&schema, exact).unwrap().definitions().count(),
        2
    );
    for limits in [
        SymbolLimits {
            max_symbols: 1,
            ..exact
        },
        SymbolLimits {
            max_name_bytes: 4,
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
        assert!(names.symbols(&schema, limits).is_err());
    }
}

#[test]
fn request_and_name_limits_are_inclusive_and_replay_at_capacity_is_valid() {
    let limits = AllocationLimits {
        symbols: SymbolLimits {
            max_symbols: 1,
            max_name_bytes: 4,
        },
        max_requests: 1,
        ..Default::default()
    };
    let base = NamedIdRegistry::empty(registry(), limits).unwrap();
    let request = definition("unit", DefinitionKind::Unit);
    let full = allocate(&base, std::slice::from_ref(&request));
    assert_eq!(snapshot(&allocate(&full, &[request])), snapshot(&full));
    assert!(
        base.allocate(
            base.identity().unwrap(),
            &[definition("units", DefinitionKind::Unit)],
        )
        .is_err()
    );
    assert!(
        base.allocate(
            base.identity().unwrap(),
            &[
                definition("unit", DefinitionKind::Unit),
                definition("next", DefinitionKind::Unit),
            ],
        )
        .is_err()
    );
    assert!(
        full.allocate(
            full.identity().unwrap(),
            &[alias("more", target(&full, "unit", false))],
        )
        .is_err()
    );
}

#[test]
fn snapshot_byte_limit_is_exact_and_failed_growth_preserves_the_base() {
    let full = allocate(&empty(), &[definition("unit", DefinitionKind::Unit)]);
    let size = serde_json::to_vec(full.input()).unwrap().len();
    let limits = AllocationLimits {
        max_snapshot_bytes: size,
        ..Default::default()
    };
    let exact =
        NamedIdRegistry::new(full.registry().clone(), full.input().clone(), limits).unwrap();
    assert_eq!(snapshot(&exact), snapshot(&full));
    assert!(
        NamedIdRegistry::new(
            full.registry().clone(),
            full.input().clone(),
            AllocationLimits {
                max_snapshot_bytes: size - 1,
                ..limits
            },
        )
        .is_err()
    );
    let before = snapshot(&exact);
    assert!(
        exact
            .allocate(
                exact.identity().unwrap(),
                &[alias("another_alias", target(&exact, "unit", false))],
            )
            .is_err()
    );
    assert_eq!(snapshot(&exact), before);
    assert_eq!(
        snapshot(&allocate(
            &exact,
            &[definition("unit", DefinitionKind::Unit)]
        )),
        before
    );
}

#[test]
fn zero_allocation_limits_are_invalid_even_for_an_empty_registry() {
    let defaults = AllocationLimits::default();
    for limits in [
        AllocationLimits {
            max_requests: 0,
            ..defaults
        },
        AllocationLimits {
            max_snapshot_bytes: 0,
            ..defaults
        },
        AllocationLimits {
            symbols: SymbolLimits {
                max_symbols: 0,
                ..defaults.symbols
            },
            ..defaults
        },
        AllocationLimits {
            symbols: SymbolLimits {
                max_name_bytes: 0,
                ..defaults.symbols
            },
            ..defaults
        },
    ] {
        assert!(NamedIdRegistry::empty(registry(), limits).is_err());
    }
}
