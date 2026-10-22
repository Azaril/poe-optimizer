//! Bounded authoring names for exact, known Core schema addresses.
//!
//! This registry resolves names only. It does not establish rule ownership,
//! contextual applicability, read authority or complete definition coverage.
use crate::bindings::valid_name;
use poe_optimizer_core::{
    owned_definitions::SlotOwnerDefId,
    owned_schema::{DefinitionAddress, DefinitionSchemaIndex, SchemaLookup, SlotAddress},
};
use std::{collections::BTreeMap, fmt};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SymbolLimits {
    /// The combined number of explicit definition and slot names, including aliases.
    pub max_symbols: usize,
    pub max_name_bytes: usize,
}

impl Default for SymbolLimits {
    fn default() -> Self {
        Self {
            max_symbols: 4096,
            max_name_bytes: 128,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SymbolErrorKind {
    InvalidLimits,
    LimitExceeded,
    InvalidName,
    DuplicateName,
    Missing,
    Unmapped,
    NamespaceMismatch,
    InconsistentIndex,
}

/// `index` addresses the supplied definitions or slots slice; `slot` selects
/// that slice. `None` denotes a global limit. Errors never copy source names.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SymbolError {
    pub kind: SymbolErrorKind,
    pub slot: bool,
    pub index: Option<usize>,
}

impl fmt::Display for SymbolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} in {} symbol {:?}",
            self.kind,
            if self.slot { "slot" } else { "definition" },
            self.index
        )
    }
}

impl std::error::Error for SymbolError {}

/// Immutable aliases to the existing typed address model. Definitions and slots
/// have separate name spaces. Different names may denote the same exact address.
#[derive(Clone, Debug)]
pub struct SymbolRegistry {
    definitions: BTreeMap<String, DefinitionAddress>,
    slots: BTreeMap<String, SlotAddress>,
}

impl SymbolRegistry {
    /// Every address must resolve to its exact known descriptor. A slot also
    /// requires its exact declaration owner to resolve to a known descriptor.
    /// Complete slot membership and rule permissions remain schema/Engine checks.
    pub fn new<I: DefinitionSchemaIndex>(
        index: &I,
        definitions: &[(&str, DefinitionAddress)],
        slots: &[(&str, SlotAddress)],
        limits: SymbolLimits,
    ) -> Result<Self, SymbolError> {
        let global = |kind| SymbolError {
            kind,
            slot: false,
            index: None,
        };
        if limits.max_symbols == 0 || limits.max_name_bytes == 0 {
            return Err(global(SymbolErrorKind::InvalidLimits));
        }
        if definitions
            .len()
            .checked_add(slots.len())
            .is_none_or(|count| count > limits.max_symbols)
        {
            return Err(global(SymbolErrorKind::LimitExceeded));
        }
        let mut output = Self {
            definitions: BTreeMap::new(),
            slots: BTreeMap::new(),
        };
        for (position, (name, address)) in definitions.iter().enumerate() {
            let error = |kind| SymbolError {
                kind,
                slot: false,
                index: Some(position),
            };
            check_name(name, limits).map_err(error)?;
            if output.definitions.contains_key(*name) {
                return Err(error(SymbolErrorKind::DuplicateName));
            }
            check_definition(index, address).map_err(error)?;
            output
                .definitions
                .insert((*name).to_owned(), address.clone());
        }
        for (position, (name, address)) in slots.iter().enumerate() {
            let error = |kind| SymbolError {
                kind,
                slot: true,
                index: Some(position),
            };
            check_name(name, limits).map_err(error)?;
            if output.slots.contains_key(*name) {
                return Err(error(SymbolErrorKind::DuplicateName));
            }
            check_slot(index, address).map_err(error)?;
            check_owner(index, address.declaration()).map_err(error)?;
            output.slots.insert((*name).to_owned(), address.clone());
        }
        Ok(output)
    }

    pub fn definition(&self, name: &str) -> Option<&DefinitionAddress> {
        self.definitions.get(name)
    }

    pub fn slot(&self, name: &str) -> Option<&SlotAddress> {
        self.slots.get(name)
    }

    /// Stable name order; addresses and names remain borrowed from this registry.
    pub fn definitions(&self) -> impl Iterator<Item = (&str, &DefinitionAddress)> {
        self.definitions
            .iter()
            .map(|(name, address)| (name.as_str(), address))
    }

    /// Stable name order, independent of definition aliases with the same name.
    pub fn slots(&self) -> impl Iterator<Item = (&str, &SlotAddress)> {
        self.slots
            .iter()
            .map(|(name, address)| (name.as_str(), address))
    }

    /// Reuse against another schema requires fresh exact-address checks. The
    /// registry caches no descriptor semantics, so known same-address updates
    /// are permitted; the complete program still needs the Engine's checks.
    pub(crate) fn validate_against<I: DefinitionSchemaIndex>(
        &self,
        index: &I,
    ) -> Result<(), SymbolErrorKind> {
        for address in self.definitions.values() {
            check_definition(index, address)?;
        }
        for address in self.slots.values() {
            check_slot(index, address)?;
            check_owner(index, address.declaration())?;
        }
        Ok(())
    }
}

fn check_name(name: &str, limits: SymbolLimits) -> Result<(), SymbolErrorKind> {
    if name.len() > limits.max_name_bytes {
        Err(SymbolErrorKind::LimitExceeded)
    } else if !valid_name(name) {
        Err(SymbolErrorKind::InvalidName)
    } else {
        Ok(())
    }
}

fn known<T>(lookup: SchemaLookup<'_, T>) -> Result<(), SymbolErrorKind> {
    match lookup {
        SchemaLookup::Known(_) => Ok(()),
        SchemaLookup::Missing => Err(SymbolErrorKind::Missing),
        SchemaLookup::Unmapped(_) => Err(SymbolErrorKind::Unmapped),
        SchemaLookup::NamespaceMismatch => Err(SymbolErrorKind::NamespaceMismatch),
        SchemaLookup::InconsistentIndex => Err(SymbolErrorKind::InconsistentIndex),
    }
}

fn check_definition<I: DefinitionSchemaIndex>(
    index: &I,
    address: &DefinitionAddress,
) -> Result<(), SymbolErrorKind> {
    // Dispatch to Core's sealed typed lookup so namespace, returned address and
    // descriptor kind checks stay in the existing schema access contract.
    macro_rules! definitions {
        ($($kind:ident),+ $(,)?) => {
            match address {
                $(DefinitionAddress::$kind(id) => known(index.definition(id))),+
            }
        };
    }
    definitions!(
        Class,
        Ascendancy,
        Reward,
        ItemTemplate,
        Modifier,
        Gem,
        Skill,
        Actor,
        PassiveNode,
        PointPool,
        EquipmentSlot,
        Encounter,
        Metric,
        Option,
        ActionPart,
        ActionMode,
        ActionStatSet,
        UsagePolicy,
        SkillLinkRole,
        SocketSlot,
        Unit,
        Quality,
        ExternalInput,
        Stat,
        Capability,
    )
}

fn check_slot<I: DefinitionSchemaIndex>(
    index: &I,
    address: &SlotAddress,
) -> Result<(), SymbolErrorKind> {
    match address {
        SlotAddress::Parameter(slot) => known(index.slot(slot)),
        SlotAddress::Choice(slot) => known(index.slot(slot)),
        SlotAddress::Grant(slot) => known(index.slot(slot)),
        SlotAddress::Actor(slot) => known(index.slot(slot)),
        SlotAddress::SkillGrant(slot) => known(index.slot(slot)),
        SlotAddress::ActionOutput(slot) => known(index.slot(slot)),
    }
}

fn check_owner<I: DefinitionSchemaIndex>(
    index: &I,
    owner: &SlotOwnerDefId,
) -> Result<(), SymbolErrorKind> {
    macro_rules! owners {
        ($($kind:ident),+ $(,)?) => {
            match owner {
                $(SlotOwnerDefId::$kind(id) => known(index.definition(id))),+
            }
        };
    }
    owners!(
        Class,
        Ascendancy,
        Reward,
        ItemTemplate,
        Modifier,
        Gem,
        Skill,
        Actor,
        PassiveNode,
        UsagePolicy,
    )
}
