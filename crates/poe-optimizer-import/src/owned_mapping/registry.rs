use super::registry_snapshot::{allocated_key, check_count, encoded_size};
use super::*;
use poe_optimizer_core::{
    owned_build::DeclaredSlot,
    owned_schema::{SchemaDefinitionId, SchemaSlotId},
};
use std::{collections::BTreeMap, ops::Deref};

/// Offline allocator over one checked registry snapshot.
/// Mutations take no external key or display name and finish checks before editing.
#[derive(Clone, Debug)]
pub struct OwnedIdRegistry {
    snapshot: OwnedIdRegistrySnapshot,
    active_dependents: BTreeMap<DefinitionAddress, usize>,
}

impl Deref for OwnedIdRegistry {
    type Target = OwnedIdRegistrySnapshot;

    fn deref(&self) -> &Self::Target {
        &self.snapshot
    }
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

    pub fn new(input: RegistryInput, limits: OwnedMappingLimits) -> Result<Self> {
        Ok(Self::from_snapshot(OwnedIdRegistrySnapshot::new(
            input, limits,
        )?))
    }

    /// Resume offline allocation from a checked snapshot without issuing identities.
    pub fn from_snapshot(snapshot: OwnedIdRegistrySnapshot) -> Self {
        let mut active_dependents = BTreeMap::new();
        for entry in &snapshot.input.entries {
            if matches!(entry.state, RegistryState::Active)
                && let SchemaSubject::Slot(slot) = &entry.target
            {
                *active_dependents
                    .entry(owner_address(slot.declaration()))
                    .or_insert(0) += 1;
            }
        }
        Self {
            snapshot,
            active_dependents,
        }
    }

    pub fn snapshot(&self) -> &OwnedIdRegistrySnapshot {
        &self.snapshot
    }

    pub fn into_snapshot(self) -> OwnedIdRegistrySnapshot {
        self.snapshot
    }

    pub fn allocate_definition<K: DefinitionDomain>(&mut self) -> Result<DefId<K>>
    where
        DefId<K>: SchemaDefinitionId,
    {
        let (sequence, revision) = self.next_counters()?;
        let id = DefId::<K>::new(
            self.snapshot.input.namespace.clone(),
            allocated_key(sequence)?,
        );
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
            slot: DefId::<K>::new(
                self.snapshot.input.namespace.clone(),
                allocated_key(sequence)?,
            ),
        };
        self.append(
            sequence,
            revision,
            SchemaSubject::Slot(<DefId<K> as SchemaSlotId>::address(&key)),
        )?;
        Ok(key)
    }

    pub fn retire(&mut self, target: &SchemaSubject, reason: OwnedDefinitionKey) -> Result<()> {
        check_target_namespace(target, &self.snapshot.input.namespace)?;
        let Some(index) = self
            .snapshot
            .positions
            .get(&TargetKey::from(target))
            .copied()
        else {
            return invalid(
                "registry.retire",
                OwnedMappingErrorKind::UnknownRegistryTarget,
            );
        };
        let previous = &self.snapshot.input.entries[index];
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
        let revision = increment(
            self.snapshot.input.revision,
            OwnedMappingErrorKind::RevisionOverflow,
        )?;
        let mut replacement = previous.clone();
        replacement.state = RegistryState::Retired { reason };
        let old_bytes = codec::serialized_len(previous, self.snapshot.limits.max_wire_bytes)?;
        let new_bytes = codec::serialized_len(&replacement, self.snapshot.limits.max_wire_bytes)?;
        let entry_bytes = self.snapshot.entry_bytes - old_bytes + new_bytes;
        let metadata = self.metadata(revision, self.snapshot.input.last_issued);
        let wire_bytes = encoded_size(
            &metadata,
            entry_bytes,
            self.snapshot.input.entries.len(),
            self.snapshot.limits,
        )?;

        // All fallible checks finish before changing the stored snapshot.
        if let SchemaSubject::Slot(slot) = target
            && let Some(count) = self
                .active_dependents
                .get_mut(&owner_address(slot.declaration()))
        {
            *count -= 1;
        }
        self.snapshot.input.entries[index] = replacement;
        self.snapshot.input.revision = revision;
        self.snapshot.entry_bytes = entry_bytes;
        self.snapshot.wire_bytes = wire_bytes;
        Ok(())
    }

    fn next_counters(&self) -> Result<(BoundedInteger, BoundedInteger)> {
        Ok((
            increment(
                self.snapshot.input.last_issued,
                OwnedMappingErrorKind::CounterOverflow,
            )?,
            increment(
                self.snapshot.input.revision,
                OwnedMappingErrorKind::RevisionOverflow,
            )?,
        ))
    }
    fn metadata(&self, revision: BoundedInteger, last_issued: BoundedInteger) -> RegistryInput {
        RegistryInput {
            schema_version: self.snapshot.input.schema_version,
            namespace: self.snapshot.input.namespace.clone(),
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
        let count = self.snapshot.input.entries.len() + 1;
        check_count(count, self.snapshot.limits)?;
        let key = TargetKey::from(&target);
        if self.snapshot.positions.contains_key(&key) {
            return invalid("registry.allocate", OwnedMappingErrorKind::DuplicateTarget);
        }
        let entry = RegistryEntry {
            sequence,
            target,
            state: RegistryState::Active,
        };
        let entry_bytes = self.snapshot.entry_bytes
            + codec::serialized_len(&entry, self.snapshot.limits.max_wire_bytes)?;
        let metadata = self.metadata(revision, sequence);
        let wire_bytes = encoded_size(&metadata, entry_bytes, count, self.snapshot.limits)?;

        if let SchemaSubject::Slot(slot) = &entry.target {
            *self
                .active_dependents
                .entry(owner_address(slot.declaration()))
                .or_insert(0) += 1;
        }
        self.snapshot
            .positions
            .insert(key, self.snapshot.input.entries.len());
        self.snapshot.input.entries.push(entry);
        self.snapshot.input.last_issued = sequence;
        self.snapshot.input.revision = revision;
        self.snapshot.entry_bytes = entry_bytes;
        self.snapshot.wire_bytes = wire_bytes;
        Ok(())
    }
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
