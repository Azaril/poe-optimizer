use super::*;
use poe_optimizer_core::{
    owned_build::DeclaredSlot,
    owned_content::digest_owned,
    owned_schema::{SchemaDefinitionId, SchemaSlotId},
};
use std::collections::BTreeMap;

/// Persistable allocation history. Mutations take no external key or display name.
/// Identity hashing is deferred until requested, rather than repeated per allocation.
#[derive(Clone, Debug)]
pub struct OwnedIdRegistry {
    input: RegistryInput,
    positions: BTreeMap<TargetKey, usize>,
    active_dependents: BTreeMap<DefinitionAddress, usize>,
    entry_bytes: usize,
    wire_bytes: usize,
    limits: OwnedMappingLimits,
}

impl OwnedIdRegistry {
    pub fn empty(namespace: GameVersionNamespace, limits: OwnedMappingLimits) -> Result<Self> {
        Self::new(
            RegistryInput {
                schema_version: OWNED_ID_REGISTRY_VERSION,
                namespace,
                revision: integer(0, OwnedMappingErrorKind::RevisionOverflow)?,
                last_issued: integer(0, OwnedMappingErrorKind::CounterOverflow)?,
                entries: vec![],
            },
            limits,
        )
    }

    pub fn new(mut input: RegistryInput, limits: OwnedMappingLimits) -> Result<Self> {
        limits.validate()?;
        if input.schema_version != OWNED_ID_REGISTRY_VERSION {
            return Err(OwnedMappingError::UnsupportedVersion {
                artifact: "owned ID registry",
                version: input.schema_version,
            });
        }
        check_count(input.entries.len(), limits)?;
        if input.revision.get() < 0 || input.last_issued.get() < 0 {
            return invalid("registry.counter", OwnedMappingErrorKind::InvalidCounter);
        }
        input.entries.sort_by_key(|entry| entry.sequence);
        if input.last_issued.get() != input.entries.len() as i64 {
            return invalid("registry.last_issued", OwnedMappingErrorKind::HistoryGap);
        }
        let mut positions = BTreeMap::new();
        let mut entry_bytes = 0usize;
        let mut retired = 0i64;
        for (index, entry) in input.entries.iter().enumerate() {
            if entry.sequence.get() != index as i64 + 1 {
                return invalid(
                    "registry.entries.sequence",
                    OwnedMappingErrorKind::HistoryGap,
                );
            }
            check_target_namespace(&entry.target, &input.namespace)?;
            if target_symbol(&entry.target) != &allocated_key(entry.sequence)? {
                return invalid(
                    "registry.entries.target",
                    OwnedMappingErrorKind::WrongAllocatedKey,
                );
            }
            if positions
                .insert(TargetKey::from(&entry.target), index)
                .is_some()
            {
                return invalid("registry.entries", OwnedMappingErrorKind::DuplicateTarget);
            }
            if matches!(entry.state, RegistryState::Retired { .. }) {
                retired += 1;
            }
            entry_bytes = entry_bytes
                .checked_add(codec::serialized_len(entry, limits.max_wire_bytes)?)
                .ok_or(OwnedMappingError::TooLarge {
                    maximum: limits.max_wire_bytes,
                })?;
            if entry_bytes > limits.max_wire_bytes {
                return Err(OwnedMappingError::TooLarge {
                    maximum: limits.max_wire_bytes,
                });
            }
        }
        // Version 1 has exactly two edits: one allocation and at most one retirement
        // per entry. This checks snapshot shape; it does not replace successor checks.
        if input.revision.get() != input.last_issued.get() + retired {
            return invalid("registry.revision", OwnedMappingErrorKind::InvalidCounter);
        }
        let mut active_dependents = BTreeMap::new();
        for entry in &input.entries {
            if let SchemaSubject::Slot(slot) = &entry.target {
                let owner = owner_address(slot.declaration());
                let Some(index) = positions.get(&TargetKey::Definition(owner.clone())) else {
                    return invalid(
                        "registry.slot.owner",
                        OwnedMappingErrorKind::MissingRegistryOwner,
                    );
                };
                let owner_entry = &input.entries[*index];
                if owner_entry.sequence >= entry.sequence {
                    return invalid("registry.slot.owner", OwnedMappingErrorKind::HistoryGap);
                }
                if matches!(entry.state, RegistryState::Active) {
                    if !matches!(owner_entry.state, RegistryState::Active) {
                        return invalid(
                            "registry.slot.owner",
                            OwnedMappingErrorKind::RetiredTarget,
                        );
                    }
                    *active_dependents.entry(owner).or_insert(0) += 1;
                }
            }
        }
        let wire_bytes = encoded_size(&input, entry_bytes, input.entries.len(), limits)?;
        Ok(Self {
            input,
            positions,
            active_dependents,
            entry_bytes,
            wire_bytes,
            limits,
        })
    }

    pub fn input(&self) -> &RegistryInput {
        &self.input
    }
    pub fn identity(&self) -> Result<OwnedContentDigest> {
        Ok(digest_owned(
            REGISTRY_DOMAIN,
            &self.input,
            self.limits.max_wire_bytes,
        )?)
    }
    pub fn entry(&self, target: &SchemaSubject) -> Option<&RegistryEntry> {
        self.positions
            .get(&TargetKey::from(target))
            .map(|index| &self.input.entries[*index])
    }
    pub fn validate_limits(&self, limits: OwnedMappingLimits) -> Result<()> {
        limits.validate()?;
        check_count(self.input.entries.len(), limits)?;
        if self.wire_bytes > limits.max_wire_bytes {
            return Err(OwnedMappingError::TooLarge {
                maximum: limits.max_wire_bytes,
            });
        }
        Ok(())
    }

    pub fn allocate_definition<K: DefinitionDomain>(&mut self) -> Result<DefId<K>>
    where
        DefId<K>: SchemaDefinitionId,
    {
        let (sequence, revision) = self.next_counters()?;
        let id = DefId::<K>::new(self.input.namespace.clone(), allocated_key(sequence)?);
        self.append(sequence, revision, SchemaSubject::Definition(id.address()))?;
        Ok(id)
    }

    pub fn allocate_slot<K: DefinitionDomain>(
        &mut self,
        owner: SlotOwnerDefId,
    ) -> Result<DeclaredSlot<DefId<K>>>
    where
        DefId<K>: SchemaSlotId,
    {
        let owner_target = SchemaSubject::Definition(owner_address(&owner));
        self.require_active(&owner_target)?;
        let (sequence, revision) = self.next_counters()?;
        let key = DeclaredSlot {
            declaration: owner,
            slot: DefId::<K>::new(self.input.namespace.clone(), allocated_key(sequence)?),
        };
        self.append(
            sequence,
            revision,
            SchemaSubject::Slot(<DefId<K> as SchemaSlotId>::address(&key)),
        )?;
        Ok(key)
    }

    pub fn retire(&mut self, target: &SchemaSubject, reason: OwnedDefinitionKey) -> Result<()> {
        check_target_namespace(target, &self.input.namespace)?;
        let Some(index) = self.positions.get(&TargetKey::from(target)).copied() else {
            return invalid(
                "registry.retire",
                OwnedMappingErrorKind::UnknownRegistryTarget,
            );
        };
        let previous = &self.input.entries[index];
        if !matches!(previous.state, RegistryState::Active) {
            return invalid("registry.retire", OwnedMappingErrorKind::AlreadyRetired);
        }
        if let SchemaSubject::Definition(id) = target
            && self.active_dependents.get(id).copied().unwrap_or(0) > 0
        {
            return invalid(
                "registry.retire",
                OwnedMappingErrorKind::ActiveDependentSlots,
            );
        }
        let revision = increment(self.input.revision, OwnedMappingErrorKind::RevisionOverflow)?;
        let mut replacement = previous.clone();
        replacement.state = RegistryState::Retired { reason };
        let old_bytes = codec::serialized_len(previous, self.limits.max_wire_bytes)?;
        let new_bytes = codec::serialized_len(&replacement, self.limits.max_wire_bytes)?;
        let entry_bytes = self.entry_bytes - old_bytes + new_bytes;
        let metadata = self.metadata(revision, self.input.last_issued);
        let wire_bytes = encoded_size(
            &metadata,
            entry_bytes,
            self.input.entries.len(),
            self.limits,
        )?;

        // All fallible checks finish before changing the stored snapshot.
        if let SchemaSubject::Slot(slot) = target
            && let Some(count) = self
                .active_dependents
                .get_mut(&owner_address(slot.declaration()))
        {
            *count -= 1;
        }
        self.input.entries[index] = replacement;
        self.input.revision = revision;
        self.entry_bytes = entry_bytes;
        self.wire_bytes = wire_bytes;
        Ok(())
    }

    /// Compare two validated snapshots before a host attempts a persisted update.
    /// The host must still compare-and-swap the actual stored predecessor digest.
    pub fn validate_successor(&self, next: &Self) -> Result<()> {
        if self.input.namespace != next.input.namespace
            || self.input.schema_version != next.input.schema_version
            || next.input.entries.len() < self.input.entries.len()
            || next.input.revision < self.input.revision
        {
            return invalid(
                "registry.successor",
                OwnedMappingErrorKind::SuccessorConflict,
            );
        }
        for (previous, replacement) in self.input.entries.iter().zip(&next.input.entries) {
            if previous.sequence != replacement.sequence || previous.target != replacement.target {
                return invalid(
                    "registry.successor",
                    OwnedMappingErrorKind::SuccessorConflict,
                );
            }
            if matches!(previous.state, RegistryState::Retired { .. })
                && previous.state != replacement.state
            {
                return invalid(
                    "registry.successor",
                    OwnedMappingErrorKind::SuccessorConflict,
                );
            }
        }
        Ok(())
    }

    pub(crate) fn require_active(&self, target: &SchemaSubject) -> Result<()> {
        check_target_namespace(target, &self.input.namespace)?;
        match self.entry(target) {
            None => invalid(
                "registry.target",
                OwnedMappingErrorKind::UnknownRegistryTarget,
            ),
            Some(RegistryEntry {
                state: RegistryState::Retired { .. },
                ..
            }) => invalid("registry.target", OwnedMappingErrorKind::RetiredTarget),
            Some(_) => Ok(()),
        }
    }
    fn next_counters(&self) -> Result<(BoundedInteger, BoundedInteger)> {
        Ok((
            increment(
                self.input.last_issued,
                OwnedMappingErrorKind::CounterOverflow,
            )?,
            increment(self.input.revision, OwnedMappingErrorKind::RevisionOverflow)?,
        ))
    }
    fn metadata(&self, revision: BoundedInteger, last_issued: BoundedInteger) -> RegistryInput {
        RegistryInput {
            schema_version: self.input.schema_version,
            namespace: self.input.namespace.clone(),
            revision,
            last_issued,
            entries: vec![],
        }
    }
    fn append(
        &mut self,
        sequence: BoundedInteger,
        revision: BoundedInteger,
        target: SchemaSubject,
    ) -> Result<()> {
        let count = self.input.entries.len() + 1;
        check_count(count, self.limits)?;
        let key = TargetKey::from(&target);
        if self.positions.contains_key(&key) {
            return invalid("registry.allocate", OwnedMappingErrorKind::DuplicateTarget);
        }
        let entry = RegistryEntry {
            sequence,
            target,
            state: RegistryState::Active,
        };
        let entry_bytes =
            self.entry_bytes + codec::serialized_len(&entry, self.limits.max_wire_bytes)?;
        let metadata = self.metadata(revision, sequence);
        let wire_bytes = encoded_size(&metadata, entry_bytes, count, self.limits)?;

        if let SchemaSubject::Slot(slot) = &entry.target {
            *self
                .active_dependents
                .entry(owner_address(slot.declaration()))
                .or_insert(0) += 1;
        }
        self.positions.insert(key, self.input.entries.len());
        self.input.entries.push(entry);
        self.input.last_issued = sequence;
        self.input.revision = revision;
        self.entry_bytes = entry_bytes;
        self.wire_bytes = wire_bytes;
        Ok(())
    }
}

fn check_count(count: usize, limits: OwnedMappingLimits) -> Result<()> {
    if count > limits.max_entries || count > limits.max_collection_entries {
        return invalid("registry.entries", OwnedMappingErrorKind::LimitExceeded);
    }
    Ok(())
}
fn integer(value: i64, kind: OwnedMappingErrorKind) -> Result<BoundedInteger> {
    BoundedInteger::new(value).map_err(|_| OwnedMappingError::Invalid {
        path: "registry.counter".into(),
        kind,
    })
}
fn increment(value: BoundedInteger, kind: OwnedMappingErrorKind) -> Result<BoundedInteger> {
    let next = value
        .get()
        .checked_add(1)
        .ok_or_else(|| OwnedMappingError::Invalid {
            path: "registry.counter".into(),
            kind: kind.clone(),
        })?;
    integer(next, kind)
}
fn allocated_key(sequence: BoundedInteger) -> Result<OwnedDefinitionKey> {
    OwnedDefinitionKey::new(format!("def.{:016x}", sequence.get())).map_err(|_| {
        OwnedMappingError::Invalid {
            path: "registry.sequence".into(),
            kind: OwnedMappingErrorKind::WrongAllocatedKey,
        }
    })
}
fn encoded_size(
    input: &RegistryInput,
    entry_bytes: usize,
    count: usize,
    limits: OwnedMappingLimits,
) -> Result<usize> {
    let metadata = RegistryInput {
        schema_version: input.schema_version,
        namespace: input.namespace.clone(),
        revision: input.revision,
        last_issued: input.last_issued,
        entries: vec![],
    };
    let bytes = codec::serialized_len(&metadata, limits.max_wire_bytes)?
        .checked_add(entry_bytes)
        .and_then(|n| n.checked_add(count.saturating_sub(1)))
        .ok_or(OwnedMappingError::TooLarge {
            maximum: limits.max_wire_bytes,
        })?;
    if bytes > limits.max_wire_bytes {
        return Err(OwnedMappingError::TooLarge {
            maximum: limits.max_wire_bytes,
        });
    }
    Ok(bytes)
}
