//! Names resolve to explicit IR reads, exact schema units/Options and declared
//! table signatures. These bindings do not establish read or table authority:
//! the complete package still needs the Engine's existing semantic compiler.
//! No spelling creates a default, table row, interpolation or conversion.

use poe_optimizer_core::{
    owned_definitions::{OptionDefId, OwnedDefinitionKey, UnitDefId},
    owned_rules::{IntegerRuleTable, RuleRead},
    owned_schema::{ComputedValueType, DefinitionSchemaIndex, SchemaLookup, UnitDimension},
};
use std::{collections::BTreeMap, fmt};

#[derive(Clone, Copy, Debug)]
pub struct BindingLimits {
    pub max_bindings: usize,
    pub max_name_bytes: usize,
}

impl Default for BindingLimits {
    fn default() -> Self {
        Self {
            max_bindings: 4096,
            max_name_bytes: 128,
        }
    }
}

/// Index into the caller's explicitly declared reads, not a new read source.
#[derive(Clone, Debug)]
pub struct ValueBinding {
    pub read: OwnedDefinitionKey,
    pub value_type: ComputedValueType,
}

#[derive(Clone, Debug)]
pub struct UnitBinding {
    pub unit: UnitDefId,
    pub dimension: UnitDimension,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingErrorKind {
    InvalidLimits,
    LimitExceeded,
    InvalidName,
    DuplicateName,
    ConflictingRead,
    UnknownUnit,
    InvalidPercentUnit,
    UnknownOption,
    DuplicateTable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingKind {
    Unit,
    Read,
    Option,
    Table,
}

/// `index` addresses the supplied slice selected by `binding`, without copying
/// source names into an error. Both are `None` for global limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BindingError {
    pub kind: BindingErrorKind,
    pub binding: Option<BindingKind>,
    pub index: Option<usize>,
}

impl fmt::Display for BindingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} in {:?} binding {:?}",
            self.kind, self.binding, self.index
        )
    }
}
impl std::error::Error for BindingError {}

/// Immutable expression-local names. The definition index is inspected once at
/// construction; lowering only uses these owned, bounded descriptors.
#[derive(Clone, Debug)]
pub struct ExpressionBindings {
    values: BTreeMap<String, ValueBinding>,
    units: BTreeMap<String, UnitBinding>,
    options: BTreeMap<String, OptionDefId>,
    tables: BTreeMap<OwnedDefinitionKey, ComputedValueType>,
    dimensions: BTreeMap<UnitDefId, UnitDimension>,
}

impl ExpressionBindings {
    /// Unit, value, Option and table names have separate namespaces, as do
    /// function names. Tables use package-owned keys, not symbolic path aliases.
    /// Aliases to one read are allowed only for identical read declarations.
    /// The `%` suffix must be explicitly bound to a percentage-points unit.
    /// Table signatures are unchecked declarations: row/domain/type consistency
    /// and package membership must still pass the existing Engine checker.
    /// Only table keys and return types are copied; rows are never copied here.
    pub fn from_inputs<I: DefinitionSchemaIndex>(
        index: &I,
        units: &[(&str, UnitDefId)],
        reads: &[(&str, RuleRead)],
        options: &[(&str, OptionDefId)],
        tables: &[IntegerRuleTable],
        limits: BindingLimits,
    ) -> Result<Self, BindingError> {
        let global = |kind| BindingError {
            kind,
            binding: None,
            index: None,
        };
        if limits.max_bindings == 0 || limits.max_name_bytes == 0 {
            return Err(global(BindingErrorKind::InvalidLimits));
        }
        if [units.len(), reads.len(), options.len(), tables.len()]
            .into_iter()
            .try_fold(0usize, usize::checked_add)
            .is_none_or(|n| n > limits.max_bindings)
        {
            return Err(global(BindingErrorKind::LimitExceeded));
        }
        let mut output = Self {
            values: BTreeMap::new(),
            units: BTreeMap::new(),
            options: BTreeMap::new(),
            tables: BTreeMap::new(),
            dimensions: BTreeMap::new(),
        };
        for (position, (name, unit)) in units.iter().enumerate() {
            let error = |kind| BindingError {
                kind,
                binding: Some(BindingKind::Unit),
                index: Some(position),
            };
            if name.len() > limits.max_name_bytes {
                return Err(error(BindingErrorKind::LimitExceeded));
            }
            if *name != "%" && !valid_name(name) {
                return Err(error(BindingErrorKind::InvalidName));
            }
            if output.units.contains_key(*name) {
                return Err(error(BindingErrorKind::DuplicateName));
            }
            let dimension =
                known_dimension(index, unit).ok_or_else(|| error(BindingErrorKind::UnknownUnit))?;
            if *name == "%" && dimension != UnitDimension::PercentagePoints {
                return Err(error(BindingErrorKind::InvalidPercentUnit));
            }
            output.dimensions.insert(unit.clone(), dimension);
            output.units.insert(
                (*name).into(),
                UnitBinding {
                    unit: unit.clone(),
                    dimension,
                },
            );
        }
        let mut declarations = BTreeMap::new();
        for (position, (name, read)) in reads.iter().enumerate() {
            let error = |kind| BindingError {
                kind,
                binding: Some(BindingKind::Read),
                index: Some(position),
            };
            if name.len() > limits.max_name_bytes {
                return Err(error(BindingErrorKind::LimitExceeded));
            }
            if !valid_name(name) {
                return Err(error(BindingErrorKind::InvalidName));
            }
            if output.values.contains_key(*name) {
                return Err(error(BindingErrorKind::DuplicateName));
            }
            if let Some(previous) = declarations.insert(&read.id, read)
                && previous != read
            {
                return Err(error(BindingErrorKind::ConflictingRead));
            }
            if let ComputedValueType::Quantity { unit } = &read.value_type {
                let dimension = known_dimension(index, unit)
                    .ok_or_else(|| error(BindingErrorKind::UnknownUnit))?;
                output.dimensions.insert(unit.clone(), dimension);
            }
            output.values.insert(
                (*name).into(),
                ValueBinding {
                    read: read.id.clone(),
                    value_type: read.value_type.clone(),
                },
            );
        }
        for (position, (name, option)) in options.iter().enumerate() {
            let error = |kind| BindingError {
                kind,
                binding: Some(BindingKind::Option),
                index: Some(position),
            };
            if name.len() > limits.max_name_bytes {
                return Err(error(BindingErrorKind::LimitExceeded));
            }
            if !valid_name(name) {
                return Err(error(BindingErrorKind::InvalidName));
            }
            if output.options.contains_key(*name) {
                return Err(error(BindingErrorKind::DuplicateName));
            }
            if !matches!(index.definition(option), SchemaLookup::Known(_)) {
                return Err(error(BindingErrorKind::UnknownOption));
            }
            output.options.insert((*name).to_owned(), option.clone());
        }
        for (position, table) in tables.iter().enumerate() {
            let error = |kind| BindingError {
                kind,
                binding: Some(BindingKind::Table),
                index: Some(position),
            };
            if output.tables.contains_key(&table.id) {
                return Err(error(BindingErrorKind::DuplicateTable));
            }
            if let ComputedValueType::Quantity { unit } = &table.value_type {
                let dimension = known_dimension(index, unit)
                    .ok_or_else(|| error(BindingErrorKind::UnknownUnit))?;
                output.dimensions.insert(unit.clone(), dimension);
            }
            output
                .tables
                .insert(table.id.clone(), table.value_type.clone());
        }
        Ok(output)
    }

    pub fn value(&self, name: &str) -> Option<&ValueBinding> {
        self.values.get(name)
    }
    pub fn unit(&self, name: &str) -> Option<&UnitBinding> {
        self.units.get(name)
    }
    pub fn option(&self, name: &str) -> Option<&OptionDefId> {
        self.options.get(name)
    }
    pub fn table(&self, id: &OwnedDefinitionKey) -> Option<&ComputedValueType> {
        self.tables.get(id)
    }
    pub fn dimension(&self, unit: &UnitDefId) -> Option<UnitDimension> {
        self.dimensions.get(unit).copied()
    }
}

fn known_dimension<I: DefinitionSchemaIndex>(index: &I, unit: &UnitDefId) -> Option<UnitDimension> {
    match index.definition(unit) {
        SchemaLookup::Known(schema) => Some(schema.dimension),
        _ => None,
    }
}

pub(crate) fn valid_name(name: &str) -> bool {
    name.split('.').all(|part| {
        let mut bytes = part.bytes();
        bytes
            .next()
            .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
            && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
            && !matches!(
                part,
                "true" | "false" | "if" | "else" | "not" | "and" | "or" | "let"
            )
    })
}
