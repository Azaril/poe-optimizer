use super::*;
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_schema::{
        DeclaredSet, DefinitionDescriptor, DefinitionEntry, DefinitionSchemaIndex, SchemaState,
        SlotDescriptor,
    },
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default)]
struct ResourceUse {
    entries: usize,
    largest_collection: usize,
    largest_candidates: usize,
    largest_string: usize,
    total_strings: usize,
}
impl ResourceUse {
    fn validate(&self, limits: OwnedMappingLimits) -> Result<()> {
        limits.validate()?;
        if self.entries > limits.max_entries
            || self.largest_collection > limits.max_collection_entries
            || self.largest_candidates > limits.max_candidates
            || self.largest_string > limits.max_string_bytes
            || self.total_strings > limits.max_total_string_bytes
        {
            return invalid("mapping.resources", OwnedMappingErrorKind::LimitExceeded);
        }
        Ok(())
    }
}
struct Budget {
    limits: OwnedMappingLimits,
    used: ResourceUse,
}
impl Budget {
    fn new(limits: OwnedMappingLimits) -> Result<Self> {
        limits.validate()?;
        Ok(Self {
            limits,
            used: ResourceUse::default(),
        })
    }
    fn collection(&mut self, path: &str, length: usize) -> Result<()> {
        if length > self.limits.max_collection_entries
            || length > self.limits.max_entries - self.used.entries
        {
            return invalid(path, OwnedMappingErrorKind::LimitExceeded);
        }
        self.used.entries += length;
        self.used.largest_collection = self.used.largest_collection.max(length);
        Ok(())
    }
    fn text(&mut self, path: &str, text: &str) -> Result<()> {
        if text.len() > self.limits.max_string_bytes
            || text.len() > self.limits.max_total_string_bytes - self.used.total_strings
        {
            return invalid(path, OwnedMappingErrorKind::LimitExceeded);
        }
        self.used.total_strings += text.len();
        self.used.largest_string = self.used.largest_string.max(text.len());
        Ok(())
    }
    fn components(&mut self, path: &str, components: &[&SourceComponent]) -> Result<()> {
        for component in components {
            if let SourceComponent::Text(value) = component {
                self.text(path, value)?;
            }
        }
        Ok(())
    }
    fn owner(&mut self, path: &str, owner: &ExternalOwnerSelector) -> Result<()> {
        match owner {
            ExternalOwnerSelector::Class { key } | ExternalOwnerSelector::Reward { key } => {
                self.components(path, &[key])
            }
            ExternalOwnerSelector::Ascendancy { class, key } => {
                self.components(path, &[class, key])
            }
            ExternalOwnerSelector::ItemTemplate {
                base,
                prototype,
                variant,
            } => self.components(path, &[base, prototype, variant]),
            ExternalOwnerSelector::Modifier {
                catalog,
                key,
                variant,
            } => self.components(path, &[catalog, key, variant]),
            ExternalOwnerSelector::Gem {
                game_id,
                variant_id,
            } => self.components(path, &[game_id, variant_id]),
            ExternalOwnerSelector::Skill { effect_id } => self.components(path, &[effect_id]),
            ExternalOwnerSelector::PassiveNode {
                tree_version,
                node_id,
                view,
            } => self.components(path, &[tree_version, node_id, view]),
            ExternalOwnerSelector::UsagePolicy { key, version } => {
                self.components(path, &[key, version])
            }
        }
    }
    fn selector(&mut self, path: &str, selector: &ExternalSelector) -> Result<()> {
        match selector {
            ExternalSelector::Definition(owner) => self.owner(path, owner),
            ExternalSelector::Catalog {
                key,
                version,
                variant,
                ..
            } => self.components(path, &[key, version, variant]),
            ExternalSelector::Socket { owner, key, .. }
            | ExternalSelector::Slot { owner, key, .. } => {
                self.owner(path, owner)?;
                self.components(path, &[key])
            }
            ExternalSelector::ActionAlternative {
                owner, output, key, ..
            } => {
                self.owner(path, owner)?;
                self.components(path, &[output, key])
            }
            ExternalSelector::Configuration { key, value, .. } => {
                self.components(path, &[key, value])
            }
        }
    }
    fn pin(&mut self, pin: &SourcePin) -> Result<()> {
        self.text("source.revision", &pin.revision)?;
        if pin.revision.trim().is_empty()
            || pin.revision.chars().any(char::is_control)
            || pin.files.is_empty()
        {
            return invalid("source", OwnedMappingErrorKind::InvalidSourcePin);
        }
        self.collection("source.files", pin.files.len())?;
        let mut paths = BTreeSet::new();
        for file in &pin.files {
            self.text("source.files.path", &file.path)?;
            self.text("source.files.sha256", &file.sha256)?;
            if file.path.is_empty()
                || file.path.contains('\\')
                || file.path.contains(':')
                || file.path.chars().any(char::is_control)
                || file
                    .path
                    .split('/')
                    .any(|part| part.is_empty() || matches!(part, "." | ".."))
                || !paths.insert(&file.path)
            {
                return invalid("source.files.path", OwnedMappingErrorKind::InvalidSourcePin);
            }
            if file.sha256.len() != 64
                || !file
                    .sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            {
                return invalid("source.files.sha256", OwnedMappingErrorKind::InvalidDigest);
            }
        }
        Ok(())
    }
}

/// Immutable exact mappings bound to one registry snapshot, definition package,
/// source manifest and normalization policy. This is not an imported build.
#[derive(Clone, Debug)]
pub struct OwnedMappingIndex {
    input: MappingPackageInput,
    positions: BTreeMap<ExternalSelector, usize>,
    identity: OwnedContentDigest,
    source_identity: OwnedContentDigest,
    canonical_bytes: Vec<u8>,
    resources: ResourceUse,
}
impl OwnedMappingIndex {
    pub fn new<I: DefinitionSchemaIndex>(
        mut input: MappingPackageInput,
        registry: &OwnedIdRegistry,
        definitions: &I,
        limits: OwnedMappingLimits,
    ) -> Result<Self> {
        let mut budget = Budget::new(limits)?;
        if input.schema_version != OWNED_MAPPING_PACKAGE_VERSION {
            return Err(OwnedMappingError::UnsupportedVersion {
                artifact: "owned external mapping",
                version: input.schema_version,
            });
        }
        check_bindings(&input, registry, definitions)?;
        budget.collection("mapping.entries", input.entries.len())?;
        budget.pin(&input.source)?;
        for entry in &input.entries {
            budget.selector("mapping.entries.source", &entry.source)?;
        }
        input.entries.sort_by(|a, b| a.source.cmp(&b.source));
        if input
            .entries
            .windows(2)
            .any(|pair| pair[0].source == pair[1].source)
        {
            return invalid("mapping.entries", OwnedMappingErrorKind::DuplicateSelector);
        }
        let mut exact_targets = BTreeSet::new();
        for (index, entry) in input.entries.iter_mut().enumerate() {
            let path = format!("mapping.entries[{index}]");
            match &mut entry.outcome {
                MappingOutcome::Mapped { target, basis } => {
                    check_target(&entry.source, target, registry, definitions)?;
                    if matches!(basis, MappingBasis::Exact)
                        && !exact_targets.insert(TargetKey::from(&*target))
                    {
                        return invalid(&path, OwnedMappingErrorKind::UnreviewedAlias);
                    }
                }
                MappingOutcome::Ambiguous { candidates, .. } => {
                    if candidates.len() < 2 {
                        return invalid(&path, OwnedMappingErrorKind::TooFewCandidates);
                    }
                    if candidates.len() > limits.max_candidates {
                        return invalid(&path, OwnedMappingErrorKind::LimitExceeded);
                    }
                    budget.collection(&path, candidates.len())?;
                    budget.used.largest_candidates =
                        budget.used.largest_candidates.max(candidates.len());
                    for candidate in candidates.iter() {
                        check_target(&entry.source, candidate, registry, definitions)?;
                    }
                    candidates.sort_by_key(|target| TargetKey::from(target));
                    if candidates.windows(2).any(|pair| pair[0] == pair[1]) {
                        return invalid(&path, OwnedMappingErrorKind::DuplicateCandidate);
                    }
                }
                MappingOutcome::Unmapped { .. } => {}
            }
        }
        let positions = input
            .entries
            .iter()
            .enumerate()
            .map(|(i, entry)| (entry.source.clone(), i))
            .collect();
        check_coherence(&input.entries, &positions, definitions, &mut budget)?;
        input.source.files.sort_by(|a, b| a.path.cmp(&b.path));
        let canonical_bytes = codec::bounded_json(&input, limits.max_wire_bytes)?;
        let identity = digest_owned(MAPPING_DOMAIN, &input, limits.max_wire_bytes)?;
        let source_identity =
            digest_owned(SOURCE_PIN_DOMAIN, &input.source, limits.max_wire_bytes)?;
        Ok(Self {
            input,
            positions,
            identity,
            source_identity,
            canonical_bytes,
            resources: budget.used,
        })
    }
    pub fn input(&self) -> &MappingPackageInput {
        &self.input
    }
    pub fn identity(&self) -> &OwnedContentDigest {
        &self.identity
    }
    pub fn source_identity(&self) -> &OwnedContentDigest {
        &self.source_identity
    }
    pub fn lookup(&self, selector: &ExternalSelector) -> Option<&MappingOutcome> {
        self.positions
            .get(selector)
            .map(|index| &self.input.entries[*index].outcome)
    }
    pub fn validate_limits(&self, limits: OwnedMappingLimits) -> Result<()> {
        self.resources.validate(limits)?;
        if self.canonical_bytes.len() > limits.max_wire_bytes {
            return Err(OwnedMappingError::TooLarge {
                maximum: limits.max_wire_bytes,
            });
        }
        Ok(())
    }
    /// Match a caller-selected source/policy and current immutable dependencies.
    /// Pin equality is evidence binding, not proof that source bytes were acquired.
    pub fn verify_bindings<I: DefinitionSchemaIndex>(
        &self,
        registry: &OwnedIdRegistry,
        definitions: &I,
        source: &SourcePin,
        policy_version: &OwnedDefinitionKey,
        limits: OwnedMappingLimits,
    ) -> Result<()> {
        self.validate_limits(limits)?;
        check_bindings(&self.input, registry, definitions)?;
        if &self.input.policy_version != policy_version {
            return invalid(
                "mapping.policy_version",
                OwnedMappingErrorKind::PolicyBindingMismatch,
            );
        }
        let mut budget = Budget::new(limits)?;
        budget.pin(source)?;
        let mut source = source.clone();
        source.files.sort_by(|a, b| a.path.cmp(&b.path));
        if digest_owned(SOURCE_PIN_DOMAIN, &source, limits.max_wire_bytes)? != self.source_identity
        {
            return invalid(
                "mapping.source",
                OwnedMappingErrorKind::SourceBindingMismatch,
            );
        }
        Ok(())
    }
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
}

fn check_bindings<I: DefinitionSchemaIndex>(
    input: &MappingPackageInput,
    registry: &OwnedIdRegistry,
    definitions: &I,
) -> Result<()> {
    if &input.namespace != definitions.namespace() || input.namespace != registry.input().namespace
    {
        return invalid("mapping.namespace", OwnedMappingErrorKind::ForeignNamespace);
    }
    if input.registry != registry.identity()? {
        return invalid(
            "mapping.registry",
            OwnedMappingErrorKind::RegistryBindingMismatch,
        );
    }
    if &input.definitions != definitions.identity()
        || input.definitions.validate().is_err()
        || input.definitions.game != input.namespace.game().as_str()
    {
        return invalid(
            "mapping.definitions",
            OwnedMappingErrorKind::SchemaBindingMismatch,
        );
    }
    Ok(())
}
fn check_target<I: DefinitionSchemaIndex>(
    source: &ExternalSelector,
    target: &SchemaSubject,
    registry: &OwnedIdRegistry,
    definitions: &I,
) -> Result<()> {
    if expected_kind(source) != target_kind(target) {
        return invalid("mapping.target", OwnedMappingErrorKind::WrongTargetKind);
    }
    if let (ExternalSelector::Slot { owner, .. }, SchemaSubject::Slot(address)) = (source, target)
        && source_owner_kind(owner) != address.declaration().kind()
    {
        return invalid(
            "mapping.target.owner",
            OwnedMappingErrorKind::WrongTargetOwner,
        );
    }
    registry.require_active(target)?;
    // Unmapped schema entries still establish identity; their coverage is not
    // promoted to Known by the mapping. A bad index implementation never falls back.
    match target {
        SchemaSubject::Definition(address) => {
            let Some(descriptor) = definitions.lookup_definition(address) else {
                return invalid("mapping.target", OwnedMappingErrorKind::MissingSchemaTarget);
            };
            if descriptor.address() != *address {
                return invalid(
                    "mapping.target",
                    OwnedMappingErrorKind::InconsistentSchemaIndex,
                );
            }
            if let (
                ExternalSelector::Socket { owner, kind, .. },
                DefinitionDescriptor::SocketSlot(DefinitionEntry {
                    schema: SchemaState::Known(schema),
                    ..
                }),
            ) = (source, descriptor)
            {
                if source_owner_kind(owner) != schema.owner.kind() {
                    return invalid(
                        "mapping.target.owner",
                        OwnedMappingErrorKind::WrongTargetOwner,
                    );
                }
                if kind != &schema.kind {
                    return invalid(
                        "mapping.target.socket_kind",
                        OwnedMappingErrorKind::WrongSocketKind,
                    );
                }
            }
        }
        SchemaSubject::Slot(address) => {
            let Some(descriptor) = definitions.lookup_slot(address) else {
                return invalid("mapping.target", OwnedMappingErrorKind::MissingSchemaTarget);
            };
            if descriptor.address() != *address {
                return invalid(
                    "mapping.target",
                    OwnedMappingErrorKind::InconsistentSchemaIndex,
                );
            }
        }
    }
    Ok(())
}
fn expected_kind(source: &ExternalSelector) -> DefinitionKind {
    match source {
        ExternalSelector::Definition(owner) => match owner {
            ExternalOwnerSelector::Class { .. } => DefinitionKind::Class,
            ExternalOwnerSelector::Ascendancy { .. } => DefinitionKind::Ascendancy,
            ExternalOwnerSelector::Reward { .. } => DefinitionKind::Reward,
            ExternalOwnerSelector::ItemTemplate { .. } => DefinitionKind::ItemTemplate,
            ExternalOwnerSelector::Modifier { .. } => DefinitionKind::Modifier,
            ExternalOwnerSelector::Gem { .. } => DefinitionKind::Gem,
            ExternalOwnerSelector::Skill { .. } => DefinitionKind::Skill,
            ExternalOwnerSelector::PassiveNode { .. } => DefinitionKind::PassiveNode,
            ExternalOwnerSelector::UsagePolicy { .. } => DefinitionKind::UsagePolicy,
        },
        ExternalSelector::Catalog { kind, .. } => match kind {
            ExternalCatalogKind::PointPool => DefinitionKind::PointPool,
            ExternalCatalogKind::EquipmentSlot => DefinitionKind::EquipmentSlot,
            ExternalCatalogKind::Encounter => DefinitionKind::Encounter,
            ExternalCatalogKind::Metric => DefinitionKind::Metric,
            ExternalCatalogKind::Option => DefinitionKind::Option,
            ExternalCatalogKind::SkillLinkRole => DefinitionKind::SkillLinkRole,
            ExternalCatalogKind::Unit => DefinitionKind::Unit,
            ExternalCatalogKind::Quality => DefinitionKind::Quality,
            ExternalCatalogKind::ExternalInput => DefinitionKind::ExternalInput,
            ExternalCatalogKind::Stat => DefinitionKind::Stat,
            ExternalCatalogKind::Capability => DefinitionKind::Capability,
        },
        ExternalSelector::Socket { .. } => DefinitionKind::SocketSlot,
        ExternalSelector::Slot { kind, .. } => match kind {
            ExternalSlotKind::Parameter => DefinitionKind::ParameterSlot,
            ExternalSlotKind::Choice => DefinitionKind::ChoiceSlot,
            ExternalSlotKind::Grant => DefinitionKind::GrantSlot,
            ExternalSlotKind::Actor => DefinitionKind::ActorSlot,
            ExternalSlotKind::SkillGrant => DefinitionKind::SkillGrantSlot,
            ExternalSlotKind::ActionOutput => DefinitionKind::ActionOutput,
        },
        ExternalSelector::ActionAlternative { kind, .. } => match kind {
            ActionAlternativeKind::Part => DefinitionKind::ActionPart,
            ActionAlternativeKind::Mode => DefinitionKind::ActionMode,
            ActionAlternativeKind::StatSet => DefinitionKind::ActionStatSet,
        },
        ExternalSelector::Configuration { role, .. } => match role {
            ConfigMappingRole::Parameter => DefinitionKind::ParameterSlot,
            ConfigMappingRole::Choice => DefinitionKind::ChoiceSlot,
            ConfigMappingRole::Reward => DefinitionKind::Reward,
            ConfigMappingRole::Option => DefinitionKind::Option,
            ConfigMappingRole::ExternalInput => DefinitionKind::ExternalInput,
            ConfigMappingRole::UsagePolicy => DefinitionKind::UsagePolicy,
            ConfigMappingRole::ActionOutput => DefinitionKind::ActionOutput,
            ConfigMappingRole::ActionPart => DefinitionKind::ActionPart,
            ConfigMappingRole::ActionMode => DefinitionKind::ActionMode,
            ConfigMappingRole::ActionStatSet => DefinitionKind::ActionStatSet,
        },
    }
}

fn source_owner_kind(owner: &ExternalOwnerSelector) -> DefinitionKind {
    match owner {
        ExternalOwnerSelector::Class { .. } => DefinitionKind::Class,
        ExternalOwnerSelector::Ascendancy { .. } => DefinitionKind::Ascendancy,
        ExternalOwnerSelector::Reward { .. } => DefinitionKind::Reward,
        ExternalOwnerSelector::ItemTemplate { .. } => DefinitionKind::ItemTemplate,
        ExternalOwnerSelector::Modifier { .. } => DefinitionKind::Modifier,
        ExternalOwnerSelector::Gem { .. } => DefinitionKind::Gem,
        ExternalOwnerSelector::Skill { .. } => DefinitionKind::Skill,
        ExternalOwnerSelector::PassiveNode { .. } => DefinitionKind::PassiveNode,
        ExternalOwnerSelector::UsagePolicy { .. } => DefinitionKind::UsagePolicy,
    }
}
fn mapped_target<'a>(
    entries: &'a [MappingEntry],
    positions: &BTreeMap<ExternalSelector, usize>,
    source: &ExternalSelector,
) -> Option<&'a SchemaSubject> {
    match &entries[*positions.get(source)?].outcome {
        MappingOutcome::Mapped { target, .. } => Some(target),
        MappingOutcome::Ambiguous { .. } | MappingOutcome::Unmapped { .. } => None,
    }
}
/// Only positively mapped rows establish parent identity. No row is synthesized,
/// and potential output membership is never treated as activation evidence.
fn check_coherence<I: DefinitionSchemaIndex>(
    entries: &[MappingEntry],
    positions: &BTreeMap<ExternalSelector, usize>,
    definitions: &I,
    budget: &mut Budget,
) -> Result<()> {
    let mut topology: BTreeMap<
        (SlotAddress, ActionAlternativeKind),
        Option<BTreeSet<DefinitionAddress>>,
    > = BTreeMap::new();
    for entry in entries {
        let MappingOutcome::Mapped { target, .. } = &entry.outcome else {
            continue;
        };
        let (source_owner, owned_owner) = match (&entry.source, target) {
            (ExternalSelector::Slot { owner, .. }, SchemaSubject::Slot(address)) => {
                (Some(owner), Some(address.declaration()))
            }
            (ExternalSelector::Socket { owner, .. }, SchemaSubject::Definition(address)) => {
                let Some(descriptor) = definitions.lookup_definition(address) else {
                    return invalid("mapping.parent", OwnedMappingErrorKind::MissingSchemaTarget);
                };
                if descriptor.address() != *address {
                    return invalid(
                        "mapping.parent",
                        OwnedMappingErrorKind::InconsistentSchemaIndex,
                    );
                }
                match descriptor {
                    DefinitionDescriptor::SocketSlot(DefinitionEntry {
                        schema: SchemaState::Known(schema),
                        ..
                    }) => (Some(owner), Some(&schema.owner)),
                    _ => (None, None),
                }
            }
            _ => (None, None),
        };
        if let (Some(source_owner), Some(owned_owner)) = (source_owner, owned_owner) {
            let parent = ExternalSelector::Definition(source_owner.clone());
            if let Some(mapped) = mapped_target(entries, positions, &parent)
                && mapped != &SchemaSubject::Definition(owner_address(owned_owner))
            {
                return invalid("mapping.parent", OwnedMappingErrorKind::WrongTargetOwner);
            }
        }
        let ExternalSelector::ActionAlternative {
            owner,
            output,
            kind,
            ..
        } = &entry.source
        else {
            continue;
        };
        let parent = ExternalSelector::Slot {
            owner: owner.clone(),
            kind: ExternalSlotKind::ActionOutput,
            key: output.clone(),
        };
        let Some(SchemaSubject::Slot(address)) = mapped_target(entries, positions, &parent) else {
            continue;
        };
        let cache_key = (address.clone(), *kind);
        if let std::collections::btree_map::Entry::Vacant(entry) = topology.entry(cache_key.clone())
        {
            let Some(descriptor) = definitions.lookup_slot(address) else {
                return invalid(
                    "mapping.parent_output",
                    OwnedMappingErrorKind::MissingSchemaTarget,
                );
            };
            if descriptor.address() != *address {
                return invalid(
                    "mapping.parent_output",
                    OwnedMappingErrorKind::InconsistentSchemaIndex,
                );
            }
            let members = match descriptor {
                SlotDescriptor::ActionOutput(DefinitionEntry {
                    schema: SchemaState::Known(schema),
                    ..
                }) => match kind {
                    ActionAlternativeKind::Part => {
                        complete_addresses(&schema.parts, budget, DefinitionAddress::ActionPart)?
                    }
                    ActionAlternativeKind::Mode => {
                        complete_addresses(&schema.modes, budget, DefinitionAddress::ActionMode)?
                    }
                    ActionAlternativeKind::StatSet => complete_addresses(
                        &schema.stat_sets,
                        budget,
                        DefinitionAddress::ActionStatSet,
                    )?,
                },
                _ => None,
            };
            entry.insert(members);
        }
        if let (Some(Some(members)), SchemaSubject::Definition(address)) =
            (topology.get(&cache_key), target)
            && !members.contains(address)
        {
            return invalid(
                "mapping.parent_output",
                OwnedMappingErrorKind::WrongOutputTopology,
            );
        }
    }
    Ok(())
}
fn complete_addresses<T: Clone>(
    set: &DeclaredSet<T>,
    budget: &mut Budget,
    address: impl Fn(T) -> DefinitionAddress,
) -> Result<Option<BTreeSet<DefinitionAddress>>> {
    if !set.is_complete() {
        return Ok(None);
    }
    budget.collection("mapping.output_topology", set.members.len())?;
    Ok(Some(set.members.iter().cloned().map(address).collect()))
}
