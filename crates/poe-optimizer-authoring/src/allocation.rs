//! Stable authoring names over the single existing allocation registry.
//!
//! Transactions return a staged successor without changing their base. They do
//! not publish files: the host publishes registry and names in one immutable
//! checkpoint. A future mutable head requires compare-and-swap against its actual
//! stored predecessor; choosing an explicit immutable predecessor is not head CAS.
//! Neither allocation nor a
//! name binding grants schema membership or permission to execute a rule.
use crate::{
    SymbolError, SymbolLimits, SymbolRegistry, bindings::valid_name, manifest::slot_owner,
};
use poe_optimizer_core::{
    owned_content::{
        ContentDigestError, MAX_OWNED_CONTENT_BYTES, OwnedContentDigest, digest_owned,
    },
    owned_definitions::*,
    owned_schema::{DefinitionAddress, DefinitionSchemaIndex, SchemaSubject, SlotAddress},
};
use poe_optimizer_import::owned_mapping::{OwnedIdRegistry, OwnedMappingError, RegistryState};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const NAME_BINDINGS_VERSION: u32 = 1;
const NAMES_DOMAIN: &str = "owned-authoring-name-bindings-v1";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NameBinding {
    pub name: String,
    pub target: SchemaSubject,
}

/// Persist alongside the allocation registry. The digest binds that exact
/// registry, including its revision and retired entries. Names are canonicalized
/// by definition/slot namespace and then spelling, independently of input order.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamesInput {
    pub schema_version: u32,
    pub registry: OwnedContentDigest,
    pub bindings: Vec<NameBinding>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NameRequest {
    Definition {
        name: String,
        kind: DefinitionKind,
    },
    Slot {
        name: String,
        kind: DefinitionKind,
        owner: String,
    },
    /// Another name for an exact target in the immutable base registry.
    Alias {
        name: String,
        target: SchemaSubject,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct AllocationLimits {
    /// Includes retired bindings so retirement cannot evade the history budget.
    pub symbols: SymbolLimits,
    pub max_requests: usize,
    /// Names snapshot only; the allocation registry retains its own limits.
    pub max_snapshot_bytes: usize,
}
impl Default for AllocationLimits {
    fn default() -> Self {
        Self {
            symbols: SymbolLimits::default(),
            max_requests: 4096,
            max_snapshot_bytes: 16 * 1024 * 1024,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AllocationErrorKind {
    InvalidLimits,
    UnsupportedVersion,
    LimitExceeded,
    InvalidName,
    DuplicateName,
    RegistryMismatch,
    StalePredecessor,
    UnknownTarget,
    RetiredTarget,
    WrongKind,
    UnknownOwner,
    UnsupportedOwner,
    BindingConflict,
}

#[derive(Debug, thiserror::Error)]
pub enum AllocationError {
    #[error("invalid authoring allocation: {kind:?} at {name:?}")]
    Invalid {
        kind: AllocationErrorKind,
        name: Option<String>,
    },
    #[error(transparent)]
    Registry(#[from] OwnedMappingError),
    #[error(transparent)]
    Content(#[from] ContentDigestError),
    #[error(transparent)]
    Symbols(#[from] SymbolError),
}
type Result<T> = std::result::Result<T, AllocationError>;

fn invalid(kind: AllocationErrorKind, name: Option<&str>) -> AllocationError {
    AllocationError::Invalid {
        kind,
        name: name.map(str::to_owned),
    }
}

#[derive(Clone, Debug)]
pub struct NamedIdRegistry {
    registry: OwnedIdRegistry,
    input: NamesInput,
    limits: AllocationLimits,
}

impl NamedIdRegistry {
    pub fn empty(registry: OwnedIdRegistry, limits: AllocationLimits) -> Result<Self> {
        let input = NamesInput {
            schema_version: NAME_BINDINGS_VERSION,
            registry: registry.identity()?,
            bindings: vec![],
        };
        Self::new(registry, input, limits)
    }

    pub fn new(
        registry: OwnedIdRegistry,
        mut input: NamesInput,
        limits: AllocationLimits,
    ) -> Result<Self> {
        if limits.max_requests == 0
            || limits.symbols.max_symbols == 0
            || limits.symbols.max_name_bytes == 0
            || limits.max_snapshot_bytes == 0
            || limits.max_snapshot_bytes > MAX_OWNED_CONTENT_BYTES
        {
            return Err(invalid(AllocationErrorKind::InvalidLimits, None));
        }
        if input.schema_version != NAME_BINDINGS_VERSION {
            return Err(invalid(AllocationErrorKind::UnsupportedVersion, None));
        }
        if input.bindings.len() > limits.symbols.max_symbols {
            return Err(invalid(AllocationErrorKind::LimitExceeded, None));
        }
        if input.registry != registry.identity()? {
            return Err(invalid(AllocationErrorKind::RegistryMismatch, None));
        }
        // Check lengths before sorting/comparing arbitrary source names.
        for binding in &input.bindings {
            check_name(&binding.name, limits)?;
            if registry.entry(&binding.target).is_none() {
                return Err(invalid(
                    AllocationErrorKind::UnknownTarget,
                    Some(&binding.name),
                ));
            }
        }
        input
            .bindings
            .sort_by(|a, b| binding_key(a).cmp(&binding_key(b)));
        if let Some(pair) = input
            .bindings
            .windows(2)
            .find(|pair| binding_key(&pair[0]) == binding_key(&pair[1]))
        {
            return Err(invalid(
                AllocationErrorKind::DuplicateName,
                Some(&pair[0].name),
            ));
        }
        let output = Self {
            registry,
            input,
            limits,
        };
        output.identity()?;
        Ok(output)
    }

    pub fn registry(&self) -> &OwnedIdRegistry {
        &self.registry
    }
    pub fn input(&self) -> &NamesInput {
        &self.input
    }
    pub fn identity(&self) -> Result<OwnedContentDigest> {
        Ok(digest_owned(
            NAMES_DOMAIN,
            &self.input,
            self.limits.max_snapshot_bytes,
        )?)
    }

    fn binding(&self, slot: bool, name: &str) -> Option<&NameBinding> {
        let position = self
            .input
            .bindings
            .binary_search_by(|binding| binding_key(binding).cmp(&(slot, name)))
            .ok()?;
        Some(&self.input.bindings[position])
    }

    /// Allocate in canonical definition-name order followed by slot-name order.
    /// Existing active names are reused only with the same kind/exact owner.
    /// Aliases never allocate. Request permutation has no effect on the result.
    /// All failures leave this base untouched, including late byte-limit errors.
    pub fn allocate(&self, expected: OwnedContentDigest, requests: &[NameRequest]) -> Result<Self> {
        if expected != self.identity()? {
            return Err(invalid(AllocationErrorKind::StalePredecessor, None));
        }
        if requests.len() > self.limits.max_requests {
            return Err(invalid(AllocationErrorKind::LimitExceeded, None));
        }
        let mut ordered = BTreeMap::new();
        let mut added = 0usize;
        for request in requests {
            let (slot, name) = request_key(request);
            check_name(name, self.limits)?;
            match request {
                NameRequest::Definition { kind, .. } if slot_kind(*kind) => {
                    return Err(invalid(AllocationErrorKind::WrongKind, Some(name)));
                }
                NameRequest::Slot { kind, owner, .. } => {
                    check_name(owner, self.limits)?;
                    if !slot_kind(*kind) {
                        return Err(invalid(AllocationErrorKind::WrongKind, Some(name)));
                    }
                }
                // This check deliberately uses the base, not the staged allocator.
                NameRequest::Alias { target, .. } => require_active(&self.registry, target, name)?,
                _ => {}
            }
            if ordered.insert((slot, name), request).is_some() {
                return Err(invalid(AllocationErrorKind::DuplicateName, Some(name)));
            }
            added += usize::from(self.binding(slot, name).is_none());
        }
        if added > self.limits.symbols.max_symbols - self.input.bindings.len() {
            return Err(invalid(AllocationErrorKind::LimitExceeded, None));
        }
        let mut registry = self.registry.clone();
        let mut bindings: BTreeMap<_, _> = self
            .input
            .bindings
            .iter()
            .map(|binding| {
                (
                    (is_slot(&binding.target), binding.name.clone()),
                    binding.target.clone(),
                )
            })
            .collect();
        for ((slot, name), request) in ordered {
            let binding_key = (slot, name.to_owned());
            let previous = bindings.get(&binding_key);
            if let Some(target) = previous {
                require_active(&registry, target, name)?;
            }
            let target = match request {
                NameRequest::Definition { kind, .. } => {
                    if let Some(SchemaSubject::Definition(target)) = previous {
                        if target.kind() != *kind {
                            return Err(invalid(AllocationErrorKind::BindingConflict, Some(name)));
                        }
                        continue;
                    }
                    SchemaSubject::Definition(allocate_definition(&mut registry, *kind)?)
                }
                NameRequest::Slot { kind, owner, .. } => {
                    let Some(SchemaSubject::Definition(owner)) =
                        bindings.get(&(false, owner.clone()))
                    else {
                        return Err(invalid(AllocationErrorKind::UnknownOwner, Some(name)));
                    };
                    require_active(&registry, &SchemaSubject::Definition(owner.clone()), name)?;
                    let owner = slot_owner(owner).ok_or_else(|| {
                        invalid(AllocationErrorKind::UnsupportedOwner, Some(name))
                    })?;
                    if let Some(SchemaSubject::Slot(target)) = previous {
                        if target.kind() != *kind || target.declaration() != &owner {
                            return Err(invalid(AllocationErrorKind::BindingConflict, Some(name)));
                        }
                        continue;
                    }
                    SchemaSubject::Slot(allocate_slot(&mut registry, *kind, owner)?)
                }
                NameRequest::Alias { target, .. } => {
                    if let Some(previous) = previous {
                        if previous != target {
                            return Err(invalid(AllocationErrorKind::BindingConflict, Some(name)));
                        }
                        continue;
                    }
                    target.clone()
                }
            };
            bindings.insert(binding_key, target);
        }
        let input = NamesInput {
            schema_version: NAME_BINDINGS_VERSION,
            registry: registry.identity()?,
            bindings: bindings
                .into_iter()
                .map(|((_, name), target)| NameBinding { name, target })
                .collect(),
        };
        let next = Self::new(registry, input, self.limits)?;
        self.validate_successor(&next)?;
        Ok(next)
    }

    /// Check a loaded successor's allocation history and all prior bindings.
    /// Retired names cannot disappear or acquire a different target. The host
    /// still has to authenticate which predecessor is currently persisted.
    pub fn validate_successor(&self, next: &Self) -> Result<()> {
        self.registry.validate_successor(&next.registry)?;
        for binding in &self.input.bindings {
            if next.binding(is_slot(&binding.target), &binding.name) != Some(binding) {
                return Err(invalid(
                    AllocationErrorKind::BindingConflict,
                    Some(&binding.name),
                ));
            }
        }
        Ok(())
    }

    /// Resolve active bindings against the actual final schema. Allocation alone
    /// is insufficient: missing/unmapped descriptors remain errors. Retired
    /// bindings remain reserved in history but never become executable symbols.
    pub fn symbols<I: DefinitionSchemaIndex>(
        &self,
        index: &I,
        limits: SymbolLimits,
    ) -> Result<SymbolRegistry> {
        if limits.max_symbols == 0 || limits.max_name_bytes == 0 {
            return Err(invalid(AllocationErrorKind::InvalidLimits, None));
        }
        if index.namespace() != &self.registry.input().namespace {
            return Err(invalid(AllocationErrorKind::RegistryMismatch, None));
        }
        // Enforce the caller's smaller projection budget before cloning any
        // addresses, even when this persisted registry used larger limits.
        let mut count = 0usize;
        for binding in &self.input.bindings {
            let entry = self
                .registry
                .entry(&binding.target)
                .expect("validated named target");
            if matches!(entry.state, RegistryState::Active) {
                count += 1;
                if count > limits.max_symbols || binding.name.len() > limits.max_name_bytes {
                    return Err(invalid(AllocationErrorKind::LimitExceeded, None));
                }
            }
        }
        let mut definitions = Vec::new();
        let mut slots = Vec::new();
        for binding in &self.input.bindings {
            let entry = self
                .registry
                .entry(&binding.target)
                .expect("validated named target");
            if !matches!(entry.state, RegistryState::Active) {
                continue;
            }
            match &binding.target {
                SchemaSubject::Definition(target) => {
                    definitions.push((binding.name.as_str(), target.clone()))
                }
                SchemaSubject::Slot(target) => slots.push((binding.name.as_str(), target.clone())),
            }
        }
        Ok(SymbolRegistry::new(index, &definitions, &slots, limits)?)
    }
}

fn check_name(name: &str, limits: AllocationLimits) -> Result<()> {
    if name.len() > limits.symbols.max_name_bytes {
        return Err(invalid(AllocationErrorKind::LimitExceeded, None));
    }
    if !valid_name(name) {
        return Err(invalid(AllocationErrorKind::InvalidName, Some(name)));
    }
    Ok(())
}
fn is_slot(target: &SchemaSubject) -> bool {
    matches!(target, SchemaSubject::Slot(_))
}
fn binding_key(binding: &NameBinding) -> (bool, &str) {
    (is_slot(&binding.target), &binding.name)
}
fn request_key(request: &NameRequest) -> (bool, &str) {
    match request {
        NameRequest::Definition { name, .. } => (false, name),
        NameRequest::Slot { name, .. } => (true, name),
        NameRequest::Alias { name, target } => (is_slot(target), name),
    }
}
fn slot_kind(kind: DefinitionKind) -> bool {
    matches!(
        kind,
        DefinitionKind::ParameterSlot
            | DefinitionKind::ChoiceSlot
            | DefinitionKind::GrantSlot
            | DefinitionKind::ActorSlot
            | DefinitionKind::SkillGrantSlot
            | DefinitionKind::ActionOutput
    )
}
fn require_active(registry: &OwnedIdRegistry, target: &SchemaSubject, name: &str) -> Result<()> {
    match registry.entry(target) {
        None => Err(invalid(AllocationErrorKind::UnknownTarget, Some(name))),
        Some(entry) if !matches!(entry.state, RegistryState::Active) => {
            Err(invalid(AllocationErrorKind::RetiredTarget, Some(name)))
        }
        Some(_) => Ok(()),
    }
}

fn allocate_definition(
    registry: &mut OwnedIdRegistry,
    kind: DefinitionKind,
) -> Result<DefinitionAddress> {
    macro_rules! definitions {
        ($($kind:ident),+ $(,)?) => { match kind {
            $(DefinitionKind::$kind => Ok(DefinitionAddress::$kind(registry.allocate_definition()?)),)+
            _ => Err(invalid(AllocationErrorKind::WrongKind, None)),
        } };
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
        Capability
    )
}
fn allocate_slot(
    registry: &mut OwnedIdRegistry,
    kind: DefinitionKind,
    owner: SlotOwnerDefId,
) -> Result<SlotAddress> {
    macro_rules! slots {
        ($($kind:ident => $variant:ident),+ $(,)?) => { match kind {
            $(DefinitionKind::$kind => Ok(SlotAddress::$variant(registry.allocate_slot(owner)?)),)+
            _ => Err(invalid(AllocationErrorKind::WrongKind, None)),
        } };
    }
    slots!(ParameterSlot => Parameter, ChoiceSlot => Choice, GrantSlot => Grant, ActorSlot => Actor,
        SkillGrantSlot => SkillGrant, ActionOutput => ActionOutput)
}
