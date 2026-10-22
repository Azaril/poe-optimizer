use crate::*;
use poe_optimizer_core::{
    data::DataIdentity,
    owned_definitions::*,
    owned_rules::{RuleRead, RuleReadSource},
    owned_schema::*,
};

struct Units {
    identity: DataIdentity,
    namespace: GameVersionNamespace,
    definitions: Vec<DefinitionDescriptor>,
}

impl DefinitionSchemaIndex for Units {
    fn identity(&self) -> &DataIdentity {
        &self.identity
    }
    fn namespace(&self) -> &GameVersionNamespace {
        &self.namespace
    }
    fn lookup_definition(&self, address: &DefinitionAddress) -> Option<&DefinitionDescriptor> {
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

fn index() -> Units {
    // Reuse the production loader for the actual schema identity. Unit-only
    // packages need no invented Actor/routing environment in these tests.
    use poe_optimizer_data::owned_schema::*;
    let namespace = GameVersionNamespace::new("binding-tests", "v1").unwrap();
    let definitions = [
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
    let bindings = ExpressionBindings::from_reads(
        &index,
        &[("resource.life", unit("life")), ("%", unit("percent"))],
        &[("character.level", read()), ("level", read())],
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
        ExpressionBindings::from_reads(&index, units, reads, BindingLimits::default())
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
            ExpressionBindings::from_reads(
                &index,
                &[],
                &[(name, read())],
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
    assert!(ExpressionBindings::from_reads(&index, &[], &[("a", read())], limits).is_ok());
    assert_eq!(
        ExpressionBindings::from_reads(&index, &[], &[("ab", read())], limits)
            .unwrap_err()
            .kind,
        BindingErrorKind::LimitExceeded
    );
    assert_eq!(
        ExpressionBindings::from_reads(&index, &[("a", unit("life"))], &[("b", read())], limits)
            .unwrap_err()
            .kind,
        BindingErrorKind::LimitExceeded
    );
    assert_eq!(
        ExpressionBindings::from_reads(
            &index,
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
