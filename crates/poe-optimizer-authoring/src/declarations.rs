//! Maintained Stat syntax stages additions through the existing allocator and
//! schema validator. It grants no producer, owner, or evaluation authority.
use crate::{
    AllocationError, AllocationErrorKind, ManifestError, ManifestErrorKind, ManifestLimits,
    NameRequest, NamedIdRegistry, SymbolError, SymbolErrorKind, SymbolLimits, SymbolRegistry,
    bindings::valid_name,
    manifest_parse::{Node, parse_document},
};
use poe_optimizer_core::{
    owned_content::{
        ContentDigestError, MAX_OWNED_CONTENT_BYTES, OwnedContentDigest, digest_owned,
    },
    owned_definitions::*,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::{
    OwnedDefinitionSchemaPackage, OwnedSchemaLimits, SchemaPackageError,
};
use poe_optimizer_import::owned_mapping::{OwnedMappingError, OwnedMappingLimits, RegistryState};
use std::collections::{BTreeMap, BTreeSet};

/// Unresolved syntax for the existing computed value types. Only quantities
/// require a name lookup; their unit must already exist in the supplied schema.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StatValueDeclaration {
    Boolean,
    Integer,
    Option,
    Quantity { unit: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatDeclaration {
    pub name: String,
    pub value: StatValueDeclaration,
    pub targets: Vec<RuleEntityKind>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatDeclarations {
    pub namespace: GameVersionNamespace,
    pub stats: Vec<StatDeclaration>,
}

#[derive(Clone, Copy, Debug)]
pub struct DeclarationLimits {
    pub max_declarations: usize,
    /// Combined target memberships across all declarations.
    pub max_targets: usize,
    pub max_registry_checks: usize,
    /// Canonical persisted names snapshot, checked without buffering it.
    pub max_names_bytes: usize,
    /// Applies to both the persisted names and the merged symbol projection.
    pub symbols: SymbolLimits,
    pub registry: OwnedMappingLimits,
    pub schema: OwnedSchemaLimits,
}
impl Default for DeclarationLimits {
    fn default() -> Self {
        Self {
            max_declarations: 4096,
            max_targets: 32768,
            max_registry_checks: 1_000_000,
            max_names_bytes: 16 * 1024 * 1024,
            symbols: SymbolLimits::default(),
            registry: OwnedMappingLimits::default(),
            schema: OwnedSchemaLimits::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeclarationErrorKind {
    InvalidLimits,
    LimitExceeded,
    NamespaceMismatch,
    InvalidName,
    DuplicateName,
    InvalidTargets,
    UnregisteredBase,
    ExistingDefinition,
    DuplicateAddress,
    UnknownUnit,
    WrongKind,
    SymbolConflict,
}

#[derive(Debug, thiserror::Error)]
pub enum DeclarationError {
    #[error("invalid Stat declaration: {kind:?} at declaration {declaration:?}")]
    Invalid {
        kind: DeclarationErrorKind,
        declaration: Option<usize>,
    },
    #[error(transparent)]
    Allocation(#[from] AllocationError),
    #[error(transparent)]
    Registry(#[from] OwnedMappingError),
    #[error(transparent)]
    Schema(#[from] SchemaPackageError),
    #[error(transparent)]
    Symbols(#[from] SymbolError),
    #[error("input symbols do not resolve against the supplied schema: {0:?}")]
    SymbolIndex(SymbolErrorKind),
    #[error(transparent)]
    Content(#[from] ContentDigestError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

fn invalid(kind: DeclarationErrorKind, declaration: Option<usize>) -> DeclarationError {
    DeclarationError::Invalid { kind, declaration }
}

#[derive(Debug)]
pub struct StagedStatDeclarations {
    names: NamedIdRegistry,
    schema: OwnedDefinitionSchemaPackage,
    symbols: SymbolRegistry,
    /// Only this declaration set is shared with packets. The declaration's
    /// own lookup aliases and other persisted reservations remain private.
    declared_names: Vec<String>,
}
impl StagedStatDeclarations {
    pub fn names(&self) -> &NamedIdRegistry {
        &self.names
    }
    pub fn schema(&self) -> &OwnedDefinitionSchemaPackage {
        &self.schema
    }
    pub fn symbols(&self) -> &SymbolRegistry {
        &self.symbols
    }

    /// Resolve each packet independently against the final schema. Persisted
    /// spellings retain their exact addresses, while unpersisted aliases stay
    /// local to this packet. Equal explicit Stat aliases count only once.
    pub(crate) fn symbols_for_packet(
        &self,
        aliases: &SymbolRegistry,
        limits: SymbolLimits,
    ) -> Result<SymbolRegistry, DeclarationError> {
        let ledger = LedgerNames::new(&self.names);
        let mut count = ledger.check_aliases(aliases, limits)?;
        aliases
            .validate_against(&self.schema)
            .map_err(DeclarationError::SymbolIndex)?;
        for name in &self.declared_names {
            check_name(name, None, limits)?;
            if aliases.definition(name).is_none() {
                count = count
                    .checked_add(1)
                    .filter(|total| *total <= limits.max_symbols)
                    .ok_or_else(|| invalid(DeclarationErrorKind::LimitExceeded, None))?;
            }
        }
        // Check the unique result size and every copied name before cloning
        // addresses or constructing the owned output symbol registry.
        let mut definitions: Vec<_> = aliases
            .definitions()
            .map(|(name, address)| (name, address.clone()))
            .collect();
        definitions.extend(
            self.declared_names
                .iter()
                .filter(|name| aliases.definition(name).is_none())
                .map(|name| {
                    (
                        name.as_str(),
                        self.symbols
                            .definition(name)
                            .expect("staged declarations have exact symbols")
                            .clone(),
                    )
                }),
        );
        let slots: Vec<_> = aliases
            .slots()
            .map(|(name, address)| (name, address.clone()))
            .collect();
        Ok(SymbolRegistry::new(
            &self.schema,
            &definitions,
            &slots,
            limits,
        )?)
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        NamedIdRegistry,
        OwnedDefinitionSchemaPackage,
        SymbolRegistry,
    ) {
        (self.names, self.schema, self.symbols)
    }
}

fn shape(node: Option<usize>, message: &'static str) -> ManifestError {
    ManifestError {
        kind: ManifestErrorKind::Shape,
        node,
        message,
    }
}

fn fields(node: &Node, id: usize, args: usize, properties: &[&str]) -> Result<(), ManifestError> {
    if node.arguments.len() != args
        || node.properties.len() != properties.len()
        || properties
            .iter()
            .any(|name| !node.properties.contains_key(*name))
    {
        return Err(shape(Some(id), "unexpected or missing declaration fields"));
    }
    Ok(())
}

/// Parse maintained KDL v2 syntax; the shared parser's documented source and
/// retained-document limits apply. There are no includes or filesystem reads.
pub fn parse_stat_declarations(
    text: &str,
    limits: ManifestLimits,
) -> Result<StatDeclarations, ManifestError> {
    let document = parse_document(text, limits)?;
    let [root_id] = document.roots.as_slice() else {
        return Err(shape(
            None,
            "declarations require exactly one definitions root",
        ));
    };
    let root = &document.nodes[*root_id];
    if root.name != "definitions" {
        return Err(shape(Some(*root_id), "expected definitions root"));
    }
    fields(root, *root_id, 0, &["game", "version"])?;
    let namespace =
        GameVersionNamespace::new(&root.properties["game"], &root.properties["version"]).map_err(
            |_| ManifestError {
                kind: ManifestErrorKind::InvalidKey,
                node: Some(*root_id),
                message: "invalid declaration namespace",
            },
        )?;
    let mut stats = Vec::new();
    let mut names = BTreeSet::new();
    for &id in &root.children {
        let node = &document.nodes[id];
        if node.name != "stat" {
            return Err(shape(Some(id), "only stat declarations are supported"));
        }
        let quantity = node
            .properties
            .get("type")
            .is_some_and(|value| value == "quantity");
        fields(
            node,
            id,
            1,
            if quantity {
                &["type", "unit"]
            } else {
                &["type"]
            },
        )?;
        let name = &node.arguments[0];
        if !valid_name(name) || !names.insert(name.as_str()) {
            return Err(shape(Some(id), "invalid or duplicate Stat name"));
        }
        let value = match node.properties["type"].as_str() {
            "boolean" => StatValueDeclaration::Boolean,
            "integer" => StatValueDeclaration::Integer,
            "option" => StatValueDeclaration::Option,
            "quantity" if valid_name(&node.properties["unit"]) => StatValueDeclaration::Quantity {
                unit: node.properties["unit"].clone(),
            },
            _ => return Err(shape(Some(id), "invalid Stat value type or unit name")),
        };
        let mut targets = BTreeSet::new();
        for &target_id in &node.children {
            let target = &document.nodes[target_id];
            fields(target, target_id, 1, &[])?;
            if target.name != "target" || !target.children.is_empty() {
                return Err(shape(Some(target_id), "expected a leaf target declaration"));
            }
            let kind = match target.arguments[0].as_str() {
                "actor" => RuleEntityKind::Actor,
                "action" => RuleEntityKind::Action,
                "equipment_use" => RuleEntityKind::EquipmentUse,
                "modifier" => RuleEntityKind::Modifier,
                "environment" => RuleEntityKind::Environment,
                "support_origin" => RuleEntityKind::SupportOrigin,
                "skill" => RuleEntityKind::Skill,
                _ => return Err(shape(Some(target_id), "unknown Stat target kind")),
            };
            if !targets.insert(kind) {
                return Err(shape(Some(target_id), "duplicate Stat target kind"));
            }
        }
        if targets.is_empty() {
            return Err(shape(Some(id), "a Stat requires at least one target"));
        }
        stats.push(StatDeclaration {
            name: name.clone(),
            value,
            targets: targets.into_iter().collect(),
        });
    }
    Ok(StatDeclarations { namespace, stats })
}

fn check_name(
    name: &str,
    index: Option<usize>,
    limits: SymbolLimits,
) -> Result<(), DeclarationError> {
    if name.len() > limits.max_name_bytes {
        return Err(invalid(DeclarationErrorKind::LimitExceeded, index));
    }
    if !valid_name(name) {
        return Err(invalid(DeclarationErrorKind::InvalidName, index));
    }
    Ok(())
}

/// Borrow the canonical ledger without copying names or addresses. Definition
/// and slot spellings are intentionally independent namespaces.
struct LedgerNames<'a> {
    definitions: BTreeMap<&'a str, &'a DefinitionAddress>,
    slots: BTreeMap<&'a str, &'a SlotAddress>,
}
impl<'a> LedgerNames<'a> {
    fn new(names: &'a NamedIdRegistry) -> Self {
        let mut result = Self {
            definitions: BTreeMap::new(),
            slots: BTreeMap::new(),
        };
        for binding in &names.input().bindings {
            match &binding.target {
                SchemaSubject::Definition(address) => {
                    result.definitions.insert(&binding.name, address);
                }
                SchemaSubject::Slot(address) => {
                    result.slots.insert(&binding.name, address);
                }
            }
        }
        result
    }

    fn check_aliases(
        &self,
        symbols: &SymbolRegistry,
        limits: SymbolLimits,
    ) -> Result<usize, DeclarationError> {
        if limits.max_symbols == 0 || limits.max_name_bytes == 0 {
            return Err(invalid(DeclarationErrorKind::InvalidLimits, None));
        }
        let mut count = 0usize;
        for (name, agrees) in symbols
            .definitions()
            .map(|(name, address)| {
                (
                    name,
                    self.definitions
                        .get(name)
                        .is_none_or(|saved| *saved == address),
                )
            })
            .chain(symbols.slots().map(|(name, address)| {
                (
                    name,
                    self.slots.get(name).is_none_or(|saved| *saved == address),
                )
            }))
        {
            check_name(name, None, limits)?;
            if !agrees {
                return Err(invalid(DeclarationErrorKind::SymbolConflict, None));
            }
            count = count
                .checked_add(1)
                .filter(|total| *total <= limits.max_symbols)
                .ok_or_else(|| invalid(DeclarationErrorKind::LimitExceeded, None))?;
        }
        Ok(count)
    }
}

/// Stage one immutable successor. Names reserve identities only; an existing
/// baseline descriptor can never be replaced, including by an equal descriptor.
/// Retrying against the original schema may reuse already persisted allocations.
pub fn stage_stat_declarations(
    base: &NamedIdRegistry,
    expected: OwnedContentDigest,
    schema: &OwnedDefinitionSchemaPackage,
    symbols: &SymbolRegistry,
    declarations: &StatDeclarations,
    limits: DeclarationLimits,
) -> Result<StagedStatDeclarations, DeclarationError> {
    if limits.max_declarations == 0
        || limits.max_targets == 0
        || limits.max_registry_checks == 0
        || limits.max_names_bytes == 0
        || limits.max_names_bytes > MAX_OWNED_CONTENT_BYTES
        || limits.symbols.max_symbols == 0
        || limits.symbols.max_name_bytes == 0
    {
        return Err(invalid(DeclarationErrorKind::InvalidLimits, None));
    }
    schema.validate_limits(limits.schema)?;
    base.registry().validate_limits(limits.registry)?;
    if declarations.namespace != schema.input().namespace
        || declarations.namespace != base.registry().input().namespace
    {
        return Err(invalid(DeclarationErrorKind::NamespaceMismatch, None));
    }
    let descriptor_count = schema
        .input()
        .definitions
        .len()
        .checked_add(schema.input().slots.len())
        .ok_or_else(|| invalid(DeclarationErrorKind::LimitExceeded, None))?;
    if descriptor_count > limits.max_registry_checks
        || declarations.stats.len() > limits.max_declarations
        || base.input().bindings.len() > limits.symbols.max_symbols
    {
        return Err(invalid(DeclarationErrorKind::LimitExceeded, None));
    }
    for binding in &base.input().bindings {
        check_name(&binding.name, None, limits.symbols)?;
    }
    digest_owned(
        "owned-stat-declaration-name-budget-v1",
        base.input(),
        limits.max_names_bytes,
    )?;
    if expected != base.identity()? {
        return Err(AllocationError::Invalid {
            kind: AllocationErrorKind::StalePredecessor,
            name: None,
        }
        .into());
    }
    for subject in schema
        .input()
        .definitions
        .iter()
        .map(|entry| SchemaSubject::Definition(entry.address()))
        .chain(
            schema
                .input()
                .slots
                .iter()
                .map(|entry| SchemaSubject::Slot(entry.address())),
        )
    {
        if !base
            .registry()
            .entry(&subject)
            .is_some_and(|entry| matches!(entry.state, RegistryState::Active))
        {
            return Err(invalid(DeclarationErrorKind::UnregisteredBase, None));
        }
    }
    let ledger = LedgerNames::new(base);
    let projected_count = ledger.check_aliases(symbols, limits.symbols)?;
    symbols
        .validate_against(schema)
        .map_err(DeclarationError::SymbolIndex)?;
    let mut names = BTreeSet::new();
    let mut targets = 0usize;
    let mut added_names = 0usize;
    for (index, stat) in declarations.stats.iter().enumerate() {
        check_name(&stat.name, Some(index), limits.symbols)?;
        if !names.insert(stat.name.as_str()) {
            return Err(invalid(DeclarationErrorKind::DuplicateName, Some(index)));
        }
        if symbols.definition(&stat.name).is_some() {
            return Err(invalid(DeclarationErrorKind::SymbolConflict, Some(index)));
        }
        targets = targets
            .checked_add(stat.targets.len())
            .filter(|total| *total <= limits.max_targets)
            .ok_or_else(|| invalid(DeclarationErrorKind::LimitExceeded, Some(index)))?;
        if stat.targets.len() > limits.schema.max_collection_entries {
            return Err(invalid(DeclarationErrorKind::LimitExceeded, Some(index)));
        }
        if stat.targets.is_empty()
            || stat.targets.iter().collect::<BTreeSet<_>>().len() != stat.targets.len()
        {
            return Err(invalid(DeclarationErrorKind::InvalidTargets, Some(index)));
        }
        if let StatValueDeclaration::Quantity { unit } = &stat.value {
            check_name(unit, Some(index), limits.symbols)?;
            match symbols.definition(unit) {
                Some(DefinitionAddress::Unit(_)) => {}
                Some(_) => return Err(invalid(DeclarationErrorKind::WrongKind, Some(index))),
                None => return Err(invalid(DeclarationErrorKind::UnknownUnit, Some(index))),
            }
        }
        if let Some(previous) = ledger.definitions.get(stat.name.as_str()) {
            let DefinitionAddress::Stat(id) = previous else {
                return Err(invalid(DeclarationErrorKind::WrongKind, Some(index)));
            };
            if schema
                .lookup_definition(&DefinitionAddress::Stat(id.clone()))
                .is_some()
            {
                return Err(invalid(
                    DeclarationErrorKind::ExistingDefinition,
                    Some(index),
                ));
            }
        } else {
            added_names += 1;
        }
    }
    for (before, added, maximum) in [
        (
            base.input().bindings.len(),
            added_names,
            limits.symbols.max_symbols,
        ),
        (
            base.registry().input().entries.len(),
            added_names,
            limits.registry.max_entries,
        ),
        (
            base.registry().input().entries.len(),
            added_names,
            limits.registry.max_collection_entries,
        ),
        (
            projected_count,
            declarations.stats.len(),
            limits.symbols.max_symbols,
        ),
        (
            schema.input().definitions.len(),
            declarations.stats.len(),
            limits.schema.max_collection_entries,
        ),
        (
            descriptor_count,
            declarations.stats.len(),
            limits.max_registry_checks,
        ),
    ] {
        if before
            .checked_add(added)
            .is_none_or(|total| total > maximum)
        {
            return Err(invalid(DeclarationErrorKind::LimitExceeded, None));
        }
    }
    let additional_entries = declarations
        .stats
        .len()
        .checked_add(targets)
        .ok_or_else(|| invalid(DeclarationErrorKind::LimitExceeded, None))?;
    schema.validate_entry_growth(additional_entries, limits.schema)?;
    // The sole allocator orders names canonically and leaves its base untouched.
    let requests: Vec<_> = declarations
        .stats
        .iter()
        .map(|stat| NameRequest::Definition {
            name: stat.name.clone(),
            kind: DefinitionKind::Stat,
        })
        .collect();
    let names = base.allocate(expected, &requests)?;
    names.registry().validate_limits(limits.registry)?;
    digest_owned(
        "owned-stat-declaration-name-budget-v1",
        names.input(),
        limits.max_names_bytes,
    )?;
    let staged_definitions: BTreeMap<_, _> = names
        .input()
        .bindings
        .iter()
        .filter_map(|binding| {
            if let SchemaSubject::Definition(address) = &binding.target {
                Some((binding.name.as_str(), address))
            } else {
                None
            }
        })
        .collect();
    let mut additions = Vec::new();
    let mut definition_symbols: Vec<_> = symbols
        .definitions()
        .map(|(name, address)| (name, address.clone()))
        .collect();
    let mut addresses = BTreeSet::new();
    for (index, stat) in declarations.stats.iter().enumerate() {
        let DefinitionAddress::Stat(id) = staged_definitions[stat.name.as_str()] else {
            unreachable!("allocator preserves requested Stat kind")
        };
        let address = DefinitionAddress::Stat(id.clone());
        if schema.lookup_definition(&address).is_some() {
            return Err(invalid(
                DeclarationErrorKind::ExistingDefinition,
                Some(index),
            ));
        }
        if !addresses.insert(address.clone()) {
            return Err(invalid(DeclarationErrorKind::DuplicateAddress, Some(index)));
        }
        let value = match &stat.value {
            StatValueDeclaration::Boolean => ComputedValueType::Boolean,
            StatValueDeclaration::Integer => ComputedValueType::Integer,
            StatValueDeclaration::Option => ComputedValueType::Option,
            StatValueDeclaration::Quantity { unit } => {
                let Some(DefinitionAddress::Unit(unit)) = symbols.definition(unit) else {
                    unreachable!("validated unit name")
                };
                ComputedValueType::Quantity { unit: unit.clone() }
            }
        };
        additions.push(DefinitionDescriptor::Stat(DefinitionEntry {
            id: id.clone(),
            schema: SchemaState::Known(StatSchema {
                value,
                targets: stat.targets.clone(),
            }),
        }));
        definition_symbols.push((stat.name.as_str(), address));
    }
    // Count the actual extended wire representation before cloning the baseline.
    schema.validate_definition_append_bytes(&additions, limits.schema)?;
    let mut input = schema.input().clone();
    input.definitions.extend(additions);
    let schema = OwnedDefinitionSchemaPackage::new(input, limits.schema)?;
    let slots: Vec<_> = symbols
        .slots()
        .map(|(name, address)| (name, address.clone()))
        .collect();
    let symbols = SymbolRegistry::new(&schema, &definition_symbols, &slots, limits.symbols)?;
    Ok(StagedStatDeclarations {
        names,
        schema,
        symbols,
        declared_names: declarations
            .stats
            .iter()
            .map(|stat| stat.name.clone())
            .collect(),
    })
}
