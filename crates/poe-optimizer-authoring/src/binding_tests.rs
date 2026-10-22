use crate::*;
use poe_optimizer_core::{
    data::DataIdentity,
    owned_build::ParameterValue,
    owned_definitions::*,
    owned_rules::{IntegerRuleTable, RuleRead, RuleReadSource},
    owned_schema::*,
};

struct Units {
    identity: DataIdentity,
    namespace: GameVersionNamespace,
    definitions: Vec<DefinitionDescriptor>,
    lookup_override: Option<(DefinitionAddress, Option<DefinitionDescriptor>)>,
}

impl DefinitionSchemaIndex for Units {
    fn identity(&self) -> &DataIdentity {
        &self.identity
    }
    fn namespace(&self) -> &GameVersionNamespace {
        &self.namespace
    }
    fn lookup_definition(&self, address: &DefinitionAddress) -> Option<&DefinitionDescriptor> {
        if let Some((requested, result)) = &self.lookup_override
            && requested == address
        {
            return result.as_ref();
        }
        self.definitions
            .iter()
            .find(|entry| &entry.address() == address)
    }
    fn lookup_slot(&self, _: &SlotAddress) -> Option<&SlotDescriptor> {
        None
    }
}

fn unit(name: &str) -> UnitDefId {
    UnitDefId::parse(
        GameVersionNamespace::new("binding-tests", "v1").unwrap(),
        name,
    )
    .unwrap()
}

fn option(name: &str) -> OptionDefId {
    OptionDefId::parse(
        GameVersionNamespace::new("binding-tests", "v1").unwrap(),
        name,
    )
    .unwrap()
}

fn index() -> Units {
    // Reuse the production loader for the actual schema identity. Units and
    // Options need no invented Actor/routing environment in these tests.
    use poe_optimizer_data::owned_schema::*;
    let namespace = GameVersionNamespace::new("binding-tests", "v1").unwrap();
    let mut definitions = [
        ("life", UnitDimension::ResourcePoints),
        ("percent", UnitDimension::PercentagePoints),
        ("factor", UnitDimension::DimensionlessFactor),
    ]
    .into_iter()
    .map(|(name, dimension)| {
        DefinitionDescriptor::Unit(DefinitionEntry {
            id: unit(name),
            schema: SchemaState::Known(UnitSchema { dimension }),
        })
    })
    .collect::<Vec<_>>();
    definitions.push(DefinitionDescriptor::Option(DefinitionEntry {
        id: option("selected"),
        schema: SchemaState::Known(OptionSchema {}),
    }));
    let schema = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
            namespace: namespace.clone(),
            release: OwnedDefinitionKey::new("test").unwrap(),
            semantics_version: OwnedDefinitionKey::new("test").unwrap(),
            definitions: definitions.clone(),
            slots: vec![],
        },
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    Units {
        identity: schema.identity().clone(),
        namespace,
        definitions,
        lookup_override: None,
    }
}

fn read() -> RuleRead {
    RuleRead {
        id: OwnedDefinitionKey::new("level").unwrap(),
        value_type: ComputedValueType::Integer,
        source: RuleReadSource::CharacterLevel,
    }
}

#[test]
fn symbolic_bindings_keep_exact_ids_and_read_aliases_without_defaults() {
    let index = index();
    let bindings = ExpressionBindings::from_inputs(
        &index,
        &[("resource.life", unit("life")), ("%", unit("percent"))],
        &[("character.level", read()), ("level", read())],
        &[],
        &[],
        BindingLimits::default(),
    )
    .unwrap();
    assert_eq!(bindings.value("character.level").unwrap().read, read().id);
    assert_eq!(bindings.value("level").unwrap().read, read().id);
    assert_eq!(bindings.unit("resource.life").unwrap().unit, unit("life"));
    assert!(bindings.unit("life").is_none());
    assert!(bindings.value("character.missing").is_none());
    assert_eq!(
        bindings.dimension(&unit("life")),
        Some(UnitDimension::ResourcePoints)
    );
    assert!(bindings.dimension(&unit("factor")).is_none());
}

#[test]
fn binding_errors_reject_ambiguous_authority_and_wrong_schema_units() {
    let index = index();
    let mut conflict = read();
    conflict.source = RuleReadSource::EnemyLevel;
    let bind = |units: &[(&str, UnitDefId)], reads: &[(&str, RuleRead)]| {
        ExpressionBindings::from_inputs(&index, units, reads, &[], &[], BindingLimits::default())
            .unwrap_err()
            .kind
    };
    assert_eq!(
        bind(&[], &[("a", read()), ("a", read())]),
        BindingErrorKind::DuplicateName
    );
    assert_eq!(
        bind(&[], &[("a", read()), ("b", conflict)]),
        BindingErrorKind::ConflictingRead
    );
    assert_eq!(
        bind(&[("a", unit("life")), ("a", unit("factor"))], &[]),
        BindingErrorKind::DuplicateName
    );
    assert_eq!(
        bind(&[("missing", unit("missing"))], &[]),
        BindingErrorKind::UnknownUnit
    );
    assert_eq!(
        bind(&[("%", unit("factor"))], &[]),
        BindingErrorKind::InvalidPercentUnit
    );
    let foreign =
        UnitDefId::parse(GameVersionNamespace::new("other", "v1").unwrap(), "life").unwrap();
    assert_eq!(
        bind(&[("life", foreign)], &[]),
        BindingErrorKind::UnknownUnit
    );
    let mut unknown = read();
    unknown.value_type = ComputedValueType::Quantity {
        unit: unit("missing"),
    };
    assert_eq!(bind(&[], &[("a", unknown)]), BindingErrorKind::UnknownUnit);
}

#[test]
fn binding_names_and_limits_match_parsable_paths() {
    let index = index();
    for name in ["", "1name", "a..b", "a.", ".a", "true", "a.if", "a-b", "é"] {
        assert_eq!(
            ExpressionBindings::from_inputs(
                &index,
                &[],
                &[(name, read())],
                &[],
                &[],
                BindingLimits::default()
            )
            .unwrap_err()
            .kind,
            BindingErrorKind::InvalidName,
            "{name}"
        );
    }
    let limits = BindingLimits {
        max_bindings: 1,
        max_name_bytes: 1,
    };
    assert!(
        ExpressionBindings::from_inputs(&index, &[], &[("a", read())], &[], &[], limits).is_ok()
    );
    assert_eq!(
        ExpressionBindings::from_inputs(&index, &[], &[("ab", read())], &[], &[], limits)
            .unwrap_err()
            .kind,
        BindingErrorKind::LimitExceeded
    );
    assert_eq!(
        ExpressionBindings::from_inputs(
            &index,
            &[("a", unit("life"))],
            &[("b", read())],
            &[],
            &[],
            limits
        )
        .unwrap_err()
        .kind,
        BindingErrorKind::LimitExceeded
    );
    assert_eq!(
        ExpressionBindings::from_inputs(
            &index,
            &[],
            &[],
            &[],
            &[],
            BindingLimits {
                max_bindings: 0,
                ..limits
            }
        )
        .unwrap_err()
        .kind,
        BindingErrorKind::InvalidLimits
    );
}

fn table(name: &str, value_type: ComputedValueType) -> IntegerRuleTable {
    let value = match &value_type {
        ComputedValueType::Boolean => ParameterValue::Boolean(false),
        ComputedValueType::Integer => ParameterValue::Integer(BoundedInteger::new(0).unwrap()),
        ComputedValueType::Quantity { unit } => {
            ParameterValue::Quantity(FiniteQuantity::new(0.0, unit.clone()).unwrap())
        }
        ComputedValueType::Option => ParameterValue::Option(option("selected")),
    };
    IntegerRuleTable {
        id: OwnedDefinitionKey::new(name).unwrap(),
        minimum: BoundedInteger::new(0).unwrap(),
        maximum: BoundedInteger::new(0).unwrap(),
        value_type,
        rows: vec![value],
    }
}

#[test]
fn option_and_table_bindings_keep_separate_explicit_names_and_exact_signatures() {
    let index = index();
    let quantity = ComputedValueType::Quantity { unit: unit("life") };
    let input_table = table("same", quantity.clone());
    let bindings = ExpressionBindings::from_inputs(
        &index,
        &[("same", unit("factor"))],
        &[("same", read())],
        &[("same", option("selected")), ("alias", option("selected"))],
        &[input_table],
        BindingLimits::default(),
    )
    .unwrap();
    assert_eq!(bindings.unit("same").unwrap().unit, unit("factor"));
    assert_eq!(bindings.value("same").unwrap().read, read().id);
    assert_eq!(bindings.option("same"), Some(&option("selected")));
    assert_eq!(bindings.option("alias"), Some(&option("selected")));
    assert_eq!(
        bindings.table(&OwnedDefinitionKey::new("same").unwrap()),
        Some(&quantity)
    );
    assert!(bindings.option("selected").is_none());
    assert!(
        bindings
            .table(&OwnedDefinitionKey::new("missing").unwrap())
            .is_none()
    );
    assert!(bindings.unit("life").is_none());
    assert_eq!(
        bindings.dimension(&unit("life")),
        Some(UnitDimension::ResourcePoints)
    );
}

#[test]
fn option_bindings_require_known_exact_definition_and_indexed_names() {
    let mut index = index();
    let expected_error = |kind, position| BindingError {
        kind,
        binding: Some(BindingKind::Option),
        index: Some(position),
    };
    for requested in [
        option("missing"),
        OptionDefId::parse(
            GameVersionNamespace::new("foreign", "v1").unwrap(),
            "selected",
        )
        .unwrap(),
    ] {
        assert_eq!(
            ExpressionBindings::from_inputs(
                &index,
                &[],
                &[],
                &[("value", requested)],
                &[],
                BindingLimits::default()
            )
            .unwrap_err(),
            expected_error(BindingErrorKind::UnknownOption, 0),
        );
    }
    let address = DefinitionAddress::Option(option("selected"));
    for returned in [
        DefinitionDescriptor::Option(DefinitionEntry {
            id: option("other"),
            schema: SchemaState::Known(OptionSchema {}),
        }),
        DefinitionDescriptor::Unit(DefinitionEntry {
            id: unit("life"),
            schema: SchemaState::Known(UnitSchema {
                dimension: UnitDimension::ResourcePoints,
            }),
        }),
        DefinitionDescriptor::Option(DefinitionEntry {
            id: option("selected"),
            schema: SchemaState::Unmapped {
                gaps: vec![SchemaGap {
                    subject: SchemaSubject::Definition(address.clone()),
                    facet: SchemaFacet::InputSchema,
                    code: OwnedDefinitionKey::new("unmapped-option").unwrap(),
                }],
            },
        }),
    ] {
        index.lookup_override = Some((address.clone(), Some(returned)));
        assert_eq!(
            ExpressionBindings::from_inputs(
                &index,
                &[],
                &[],
                &[("value", option("selected"))],
                &[],
                BindingLimits::default()
            )
            .unwrap_err(),
            expected_error(BindingErrorKind::UnknownOption, 0),
        );
    }
    index.lookup_override = None;
    assert_eq!(
        ExpressionBindings::from_inputs(
            &index,
            &[],
            &[],
            &[("a", option("selected")), ("a", option("selected"))],
            &[],
            BindingLimits::default(),
        )
        .unwrap_err(),
        expected_error(BindingErrorKind::DuplicateName, 1),
    );
    for name in ["%", "a..b", "a.let", "1option", ""] {
        assert_eq!(
            ExpressionBindings::from_inputs(
                &index,
                &[],
                &[],
                &[(name, option("selected"))],
                &[],
                BindingLimits::default()
            )
            .unwrap_err(),
            expected_error(BindingErrorKind::InvalidName, 0),
        );
    }
}

#[test]
fn table_ids_are_unique_and_quantity_signatures_require_known_units() {
    let mut index = index();
    let first = table("local-table", ComputedValueType::Integer);
    let second = table("local-table", ComputedValueType::Boolean);
    assert_eq!(
        ExpressionBindings::from_inputs(
            &index,
            &[],
            &[],
            &[],
            &[first, second],
            BindingLimits::default()
        )
        .unwrap_err(),
        BindingError {
            kind: BindingErrorKind::DuplicateTable,
            binding: Some(BindingKind::Table),
            index: Some(1),
        },
    );
    let check = |index: &Units, result_unit: UnitDefId| {
        ExpressionBindings::from_inputs(
            index,
            &[],
            &[],
            &[],
            &[table(
                "result",
                ComputedValueType::Quantity { unit: result_unit },
            )],
            BindingLimits::default(),
        )
        .unwrap_err()
    };
    let expected = BindingError {
        kind: BindingErrorKind::UnknownUnit,
        binding: Some(BindingKind::Table),
        index: Some(0),
    };
    assert_eq!(check(&index, unit("missing")), expected);
    let foreign =
        UnitDefId::parse(GameVersionNamespace::new("foreign", "v1").unwrap(), "life").unwrap();
    assert_eq!(check(&index, foreign), expected);
    let address = DefinitionAddress::Unit(unit("life"));
    index.lookup_override = Some((
        address.clone(),
        Some(DefinitionDescriptor::Unit(DefinitionEntry {
            id: unit("life"),
            schema: SchemaState::Unmapped {
                gaps: vec![SchemaGap {
                    subject: SchemaSubject::Definition(address),
                    facet: SchemaFacet::InputSchema,
                    code: OwnedDefinitionKey::new("unmapped-unit").unwrap(),
                }],
            },
        })),
    ));
    assert_eq!(check(&index, unit("life")), expected);
}

#[test]
fn combined_binding_budget_counts_options_and_tables_and_bounds_option_names() {
    let index = index();
    let units = [("a", unit("life"))];
    let reads = [("a", read())];
    let options = [("a", option("selected"))];
    let tables = [table("package-table", ComputedValueType::Integer)];
    let limits = BindingLimits {
        max_bindings: 4,
        max_name_bytes: 1,
    };
    // A table key is already bounded by Core; the symbolic-name bound does not
    // replace that separate package-key grammar or impose an alias on it.
    ExpressionBindings::from_inputs(&index, &units, &reads, &options, &tables, limits).unwrap();
    assert_eq!(
        ExpressionBindings::from_inputs(
            &index,
            &units,
            &reads,
            &options,
            &tables,
            BindingLimits {
                max_bindings: 3,
                ..limits
            },
        )
        .unwrap_err(),
        BindingError {
            kind: BindingErrorKind::LimitExceeded,
            binding: None,
            index: None
        },
    );
    assert_eq!(
        ExpressionBindings::from_inputs(
            &index,
            &[],
            &[],
            &[("long", option("selected"))],
            &[],
            limits,
        )
        .unwrap_err(),
        BindingError {
            kind: BindingErrorKind::LimitExceeded,
            binding: Some(BindingKind::Option),
            index: Some(0),
        },
    );
}

#[test]
fn table_signatures_do_not_validate_or_copy_rows_or_domains() {
    let index = index();
    let mut input = table("unchecked-table", ComputedValueType::Integer);
    input.minimum = BoundedInteger::new(10).unwrap();
    input.maximum = BoundedInteger::new(0).unwrap();
    input.rows = vec![ParameterValue::Boolean(true)];
    // Deliberately malformed package data: this frontend records a declaration
    // signature only. The Engine must reject its domain and row/type mismatch.
    let bindings = ExpressionBindings::from_inputs(
        &index,
        &[],
        &[],
        &[],
        std::slice::from_ref(&input),
        BindingLimits::default(),
    )
    .unwrap();
    assert_eq!(bindings.table(&input.id), Some(&ComputedValueType::Integer));
    input.rows.clear();
    input.value_type = ComputedValueType::Boolean;
    assert_eq!(bindings.table(&input.id), Some(&ComputedValueType::Integer));
}
