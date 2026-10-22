use crate::*;
use poe_optimizer_core::{
    data::DataIdentity, owned_build::DeclaredSlot, owned_definitions::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::{
    OWNED_SCHEMA_PACKAGE_VERSION, OwnedDefinitionSchemaPackage, OwnedSchemaLimits,
    SchemaPackageInput,
};

struct Index {
    schema: OwnedDefinitionSchemaPackage,
    definition_override: Option<(DefinitionAddress, Option<DefinitionDescriptor>)>,
    slot_override: Option<(SlotAddress, Option<SlotDescriptor>)>,
}

impl DefinitionSchemaIndex for Index {
    fn identity(&self) -> &DataIdentity {
        self.schema.identity()
    }

    fn namespace(&self) -> &GameVersionNamespace {
        self.schema.namespace()
    }

    fn lookup_definition(&self, address: &DefinitionAddress) -> Option<&DefinitionDescriptor> {
        if let Some((requested, result)) = &self.definition_override
            && requested == address
        {
            return result.as_ref();
        }
        self.schema.lookup_definition(address)
    }

    fn lookup_slot(&self, address: &SlotAddress) -> Option<&SlotDescriptor> {
        if let Some((requested, result)) = &self.slot_override
            && requested == address
        {
            return result.as_ref();
        }
        self.schema.lookup_slot(address)
    }
}

fn namespace() -> GameVersionNamespace {
    GameVersionNamespace::new("symbol-tests", "v1").unwrap()
}

fn factor() -> UnitDefId {
    UnitDefId::parse(namespace(), "factor").unwrap()
}

fn modifier() -> ModifierDefId {
    ModifierDefId::parse(namespace(), "modifier").unwrap()
}

fn roll() -> DeclaredSlot<ParameterSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Modifier(modifier()),
        slot: ParameterSlotDefId::parse(namespace(), "roll").unwrap(),
    }
}

fn declarations() -> DeclaredSlots {
    DeclaredSlots {
        parameters: DeclaredSet::complete(vec![roll()]),
        choices: DeclaredSet::complete(vec![]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    }
}

fn index() -> Index {
    let schema = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
            namespace: namespace(),
            release: OwnedDefinitionKey::new("symbol-tests").unwrap(),
            semantics_version: OwnedDefinitionKey::new("test").unwrap(),
            definitions: vec![
                DefinitionDescriptor::Unit(DefinitionEntry {
                    id: factor(),
                    schema: SchemaState::Known(UnitSchema {
                        dimension: UnitDimension::DimensionlessFactor,
                    }),
                }),
                DefinitionDescriptor::Modifier(DefinitionEntry {
                    id: modifier(),
                    schema: SchemaState::Known(ModifierSchema {
                        declarations: declarations(),
                    }),
                }),
            ],
            slots: vec![SlotDescriptor::Parameter(DefinitionEntry {
                id: roll(),
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
        },
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    Index {
        schema,
        definition_override: None,
        slot_override: None,
    }
}

fn definition_error(index: &Index, address: DefinitionAddress) -> SymbolError {
    SymbolRegistry::new(
        index,
        &[("requested", address)],
        &[],
        SymbolLimits::default(),
    )
    .unwrap_err()
}

fn slot_error(index: &Index, address: SlotAddress) -> SymbolError {
    SymbolRegistry::new(
        index,
        &[],
        &[("requested", address)],
        SymbolLimits::default(),
    )
    .unwrap_err()
}

fn assert_error(error: SymbolError, kind: SymbolErrorKind, slot: bool, index: Option<usize>) {
    assert_eq!(error, SymbolError { kind, slot, index });
}

#[test]
fn symbols_keep_exact_core_addresses_aliases_and_separate_name_spaces() {
    let index = index();
    let definition = DefinitionAddress::Unit(factor());
    let slot = SlotAddress::Parameter(roll());
    let registry = SymbolRegistry::new(
        &index,
        &[
            ("unit.factor", definition.clone()),
            ("same", definition.clone()),
            ("modifier", DefinitionAddress::Modifier(modifier())),
        ],
        &[("same", slot.clone()), ("roll.alias", slot.clone())],
        SymbolLimits::default(),
    )
    .unwrap();
    assert_eq!(registry.definition("unit.factor"), Some(&definition));
    assert_eq!(registry.definition("same"), Some(&definition));
    assert_eq!(registry.slot("same"), Some(&slot));
    assert_eq!(registry.slot("roll.alias"), Some(&slot));
    assert!(registry.definition("factor").is_none());
    assert!(registry.definition("%").is_none());
    assert!(registry.slot("modifier").is_none());
    assert_eq!(
        registry
            .definitions()
            .map(|(name, _)| name)
            .collect::<Vec<_>>(),
        ["modifier", "same", "unit.factor"]
    );
    assert_eq!(
        registry.slots().map(|(name, _)| name).collect::<Vec<_>>(),
        ["roll.alias", "same"]
    );
    // The owner need not itself have an authored alias, but is still checked.
    assert!(SymbolRegistry::new(&index, &[], &[("roll", slot)], SymbolLimits::default()).is_ok());
}

#[test]
fn symbols_reject_duplicate_names_in_each_slice_with_exact_input_positions() {
    let index = index();
    let definition = DefinitionAddress::Unit(factor());
    let slot = SlotAddress::Parameter(roll());
    assert_error(
        SymbolRegistry::new(
            &index,
            &[("a", definition.clone()), ("a", definition)],
            &[],
            SymbolLimits::default(),
        )
        .unwrap_err(),
        SymbolErrorKind::DuplicateName,
        false,
        Some(1),
    );
    assert_error(
        SymbolRegistry::new(
            &index,
            &[],
            &[("a", slot.clone()), ("a", slot)],
            SymbolLimits::default(),
        )
        .unwrap_err(),
        SymbolErrorKind::DuplicateName,
        true,
        Some(1),
    );
}

#[test]
fn symbols_validate_names_and_whole_registry_limits_without_defaults() {
    let index = index();
    for name in [
        "", "a..b", "a.", ".a", "1name", "true", "a.let", "let", "a-b", "é", "%",
    ] {
        for slot in [false, true] {
            let result = if slot {
                SymbolRegistry::new(
                    &index,
                    &[],
                    &[(name, SlotAddress::Parameter(roll()))],
                    SymbolLimits::default(),
                )
            } else {
                SymbolRegistry::new(
                    &index,
                    &[(name, DefinitionAddress::Unit(factor()))],
                    &[],
                    SymbolLimits::default(),
                )
            };
            assert_error(
                result.unwrap_err(),
                SymbolErrorKind::InvalidName,
                slot,
                Some(0),
            );
        }
    }
    // Function names and the non-keyword "output" are valid symbol names.
    for name in ["min", "output", "unit.e2", "unit._factor"] {
        SymbolRegistry::new(
            &index,
            &[(name, DefinitionAddress::Unit(factor()))],
            &[],
            SymbolLimits::default(),
        )
        .unwrap();
    }
    let limits = SymbolLimits {
        max_symbols: 2,
        max_name_bytes: 1,
    };
    SymbolRegistry::new(
        &index,
        &[("a", DefinitionAddress::Unit(factor()))],
        &[("a", SlotAddress::Parameter(roll()))],
        limits,
    )
    .unwrap();
    assert_error(
        SymbolRegistry::new(
            &index,
            &[("a", DefinitionAddress::Unit(factor()))],
            &[("a", SlotAddress::Parameter(roll()))],
            SymbolLimits {
                max_symbols: 1,
                ..limits
            },
        )
        .unwrap_err(),
        SymbolErrorKind::LimitExceeded,
        false,
        None,
    );
    assert_error(
        SymbolRegistry::new(
            &index,
            &[],
            &[("aa", SlotAddress::Parameter(roll()))],
            limits,
        )
        .unwrap_err(),
        SymbolErrorKind::LimitExceeded,
        true,
        Some(0),
    );
    for invalid in [
        SymbolLimits {
            max_symbols: 0,
            ..limits
        },
        SymbolLimits {
            max_name_bytes: 0,
            ..limits
        },
    ] {
        assert_error(
            SymbolRegistry::new(&index, &[], &[], invalid).unwrap_err(),
            SymbolErrorKind::InvalidLimits,
            false,
            None,
        );
    }
    let empty = SymbolRegistry::new(&index, &[], &[], limits).unwrap();
    assert_eq!(empty.definitions().count(), 0);
    assert_eq!(empty.slots().count(), 0);
}

#[test]
fn definition_symbols_require_known_exact_kind_key_and_namespace() {
    let mut index = index();
    let wanted = DefinitionAddress::Unit(factor());
    let other = UnitDefId::parse(namespace(), "other").unwrap();
    assert_error(
        definition_error(&index, DefinitionAddress::Unit(other.clone())),
        SymbolErrorKind::Missing,
        false,
        Some(0),
    );
    let foreign = UnitDefId::parse(
        GameVersionNamespace::new("foreign-symbols", "v1").unwrap(),
        "factor",
    )
    .unwrap();
    assert_error(
        definition_error(&index, DefinitionAddress::Unit(foreign)),
        SymbolErrorKind::NamespaceMismatch,
        false,
        Some(0),
    );
    let unmapped = DefinitionDescriptor::Unit(DefinitionEntry {
        id: factor(),
        schema: SchemaState::Unmapped {
            gaps: vec![SchemaGap {
                subject: SchemaSubject::Definition(wanted.clone()),
                facet: SchemaFacet::InputSchema,
                code: OwnedDefinitionKey::new("not-converted").unwrap(),
            }],
        },
    });
    index.definition_override = Some((wanted.clone(), Some(unmapped)));
    assert_error(
        definition_error(&index, wanted.clone()),
        SymbolErrorKind::Unmapped,
        false,
        Some(0),
    );
    for wrong in [
        DefinitionDescriptor::Unit(DefinitionEntry {
            id: other,
            schema: SchemaState::Known(UnitSchema {
                dimension: UnitDimension::DimensionlessFactor,
            }),
        }),
        DefinitionDescriptor::Modifier(DefinitionEntry {
            id: modifier(),
            schema: SchemaState::Known(ModifierSchema {
                declarations: declarations(),
            }),
        }),
    ] {
        index.definition_override = Some((wanted.clone(), Some(wrong)));
        assert_error(
            definition_error(&index, wanted.clone()),
            SymbolErrorKind::InconsistentIndex,
            false,
            Some(0),
        );
    }
}

#[test]
fn slot_symbols_require_exact_slot_kind_declaration_and_known_schema() {
    let mut index = index();
    let wanted = SlotAddress::Parameter(roll());
    let original = index.lookup_slot(&wanted).unwrap().clone();
    let mut missing = roll();
    missing.slot = ParameterSlotDefId::parse(namespace(), "missing").unwrap();
    assert_error(
        slot_error(&index, SlotAddress::Parameter(missing)),
        SymbolErrorKind::Missing,
        true,
        Some(0),
    );
    let foreign = GameVersionNamespace::new("foreign-symbols", "v1").unwrap();
    for declaration in [false, true] {
        let mut cross_namespace = roll();
        if declaration {
            cross_namespace.declaration = SlotOwnerDefId::Modifier(
                ModifierDefId::parse(foreign.clone(), "modifier").unwrap(),
            );
        } else {
            cross_namespace.slot = ParameterSlotDefId::parse(foreign.clone(), "roll").unwrap();
        }
        assert_error(
            slot_error(&index, SlotAddress::Parameter(cross_namespace)),
            SymbolErrorKind::NamespaceMismatch,
            true,
            Some(0),
        );
    }
    let SlotDescriptor::Parameter(mut parameter) = original else {
        panic!("parameter fixture")
    };
    parameter.schema = SchemaState::Unmapped {
        gaps: vec![SchemaGap {
            subject: SchemaSubject::Slot(wanted.clone()),
            facet: SchemaFacet::InputSchema,
            code: OwnedDefinitionKey::new("not-converted").unwrap(),
        }],
    };
    index.slot_override = Some((wanted.clone(), Some(SlotDescriptor::Parameter(parameter))));
    assert_error(
        slot_error(&index, wanted.clone()),
        SymbolErrorKind::Unmapped,
        true,
        Some(0),
    );
    index.slot_override = None;
    let SlotDescriptor::Parameter(mut parameter) = index.lookup_slot(&wanted).unwrap().clone()
    else {
        panic!("parameter fixture")
    };
    parameter.id.declaration =
        SlotOwnerDefId::Modifier(ModifierDefId::parse(namespace(), "other-modifier").unwrap());
    let wrong_kind = SlotDescriptor::Choice(DefinitionEntry {
        id: DeclaredSlot {
            declaration: roll().declaration,
            slot: ChoiceSlotDefId::parse(namespace(), "roll").unwrap(),
        },
        schema: SchemaState::Known(ChoiceSlotSchema {
            value: ValueSchema::Boolean,
            presence: SlotPresence::OptionalOnce,
            owners: vec![],
        }),
    });
    for wrong in [SlotDescriptor::Parameter(parameter), wrong_kind] {
        index.slot_override = Some((wanted.clone(), Some(wrong)));
        assert_error(
            slot_error(&index, wanted.clone()),
            SymbolErrorKind::InconsistentIndex,
            true,
            Some(0),
        );
    }
}

#[test]
fn a_known_slot_never_substitutes_for_its_missing_unmapped_or_inconsistent_owner() {
    let mut index = index();
    let wanted = SlotAddress::Parameter(roll());
    let owner = DefinitionAddress::Modifier(modifier());
    index.definition_override = Some((owner.clone(), None));
    assert_error(
        slot_error(&index, wanted.clone()),
        SymbolErrorKind::Missing,
        true,
        Some(0),
    );
    index.definition_override = Some((
        owner.clone(),
        Some(DefinitionDescriptor::Modifier(DefinitionEntry {
            id: modifier(),
            schema: SchemaState::Unmapped {
                gaps: vec![SchemaGap {
                    subject: SchemaSubject::Definition(owner.clone()),
                    facet: SchemaFacet::InputSchema,
                    code: OwnedDefinitionKey::new("owner-not-converted").unwrap(),
                }],
            },
        })),
    ));
    assert_error(
        slot_error(&index, wanted.clone()),
        SymbolErrorKind::Unmapped,
        true,
        Some(0),
    );
    let wrong = index
        .lookup_definition(&DefinitionAddress::Unit(factor()))
        .unwrap()
        .clone();
    index.definition_override = Some((owner, Some(wrong)));
    assert_error(
        slot_error(&index, wanted),
        SymbolErrorKind::InconsistentIndex,
        true,
        Some(0),
    );
}

#[test]
fn existing_registry_revalidates_addresses_and_slot_owners_against_changed_indexes() {
    let mut index = index();
    let unit = DefinitionAddress::Unit(factor());
    let owner = DefinitionAddress::Modifier(modifier());
    let slot = SlotAddress::Parameter(roll());
    let registry = SymbolRegistry::new(
        &index,
        &[("factor", unit.clone()), ("modifier", owner.clone())],
        &[("roll", slot.clone())],
        SymbolLimits::default(),
    )
    .unwrap();
    assert_eq!(registry.validate_against(&index), Ok(()));

    // No descriptor semantics are cached: a known same-address schema update
    // can be resolved by the next compiler using that new schema's semantics.
    index.definition_override = Some((
        unit.clone(),
        Some(DefinitionDescriptor::Unit(DefinitionEntry {
            id: factor(),
            schema: SchemaState::Known(UnitSchema {
                dimension: UnitDimension::PercentagePoints,
            }),
        })),
    ));
    assert_eq!(registry.validate_against(&index), Ok(()));

    // The alias registry remains unchanged; missing non-unit definitions in
    // the new index are rejected even though unit lookup still succeeds.
    index.definition_override = Some((owner.clone(), None));
    assert_eq!(
        registry.validate_against(&index),
        Err(SymbolErrorKind::Missing)
    );
    index.definition_override = None;
    index.slot_override = Some((slot.clone(), None));
    assert_eq!(
        registry.validate_against(&index),
        Err(SymbolErrorKind::Missing)
    );
    index.slot_override = Some((
        slot.clone(),
        Some(SlotDescriptor::Parameter(DefinitionEntry {
            id: roll(),
            schema: SchemaState::Unmapped {
                gaps: vec![SchemaGap {
                    subject: SchemaSubject::Slot(slot.clone()),
                    facet: SchemaFacet::InputSchema,
                    code: OwnedDefinitionKey::new("now-unmapped").unwrap(),
                }],
            },
        })),
    ));
    assert_eq!(
        registry.validate_against(&index),
        Err(SymbolErrorKind::Unmapped)
    );
    index.slot_override = None;

    // Owner validation is also mandatory when no definition alias names it.
    let slot_only =
        SymbolRegistry::new(&index, &[], &[("roll", slot)], SymbolLimits::default()).unwrap();
    let wrong = index.lookup_definition(&unit).unwrap().clone();
    index.definition_override = Some((owner, Some(wrong)));
    assert_eq!(
        slot_only.validate_against(&index),
        Err(SymbolErrorKind::InconsistentIndex)
    );
}
