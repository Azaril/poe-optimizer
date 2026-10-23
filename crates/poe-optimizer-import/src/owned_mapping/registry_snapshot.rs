use super::*;
use poe_optimizer_core::owned_content::digest_owned;
use std::collections::BTreeMap;

/// Immutable, checked allocation history used by registry readers.
/// Construction validates existing identities and never issues new ones.
#[derive(Clone, Debug)]
pub struct OwnedIdRegistrySnapshot {
    pub(super) input: RegistryInput,
    pub(super) positions: BTreeMap<TargetKey, usize>,
    pub(super) entry_bytes: usize,
    pub(super) wire_bytes: usize,
    pub(super) limits: OwnedMappingLimits,
}

impl OwnedIdRegistrySnapshot {
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
        for entry in &input.entries {
            if let SchemaSubject::Slot(slot) = &entry.target {
                let owner = owner_address(slot.declaration());
                let Some(index) = positions.get(&TargetKey::Definition(owner)) else {
                    return invalid(
                        "registry.slot.owner",
                        OwnedMappingErrorKind::MissingRegistryOwner,
                    );
                };
                let owner_entry = &input.entries[*index];
                if owner_entry.sequence >= entry.sequence {
                    return invalid("registry.slot.owner", OwnedMappingErrorKind::HistoryGap);
                }
                if matches!(entry.state, RegistryState::Active)
                    && !matches!(owner_entry.state, RegistryState::Active)
                {
                    return invalid("registry.slot.owner", OwnedMappingErrorKind::RetiredTarget);
                }
            }
        }
        let wire_bytes = encoded_size(&input, entry_bytes, input.entries.len(), limits)?;
        Ok(Self {
            input,
            positions,
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
}

pub(super) fn check_count(count: usize, limits: OwnedMappingLimits) -> Result<()> {
    if count > limits.max_entries || count > limits.max_collection_entries {
        return invalid("registry.entries", OwnedMappingErrorKind::LimitExceeded);
    }
    Ok(())
}
pub(super) fn allocated_key(sequence: BoundedInteger) -> Result<OwnedDefinitionKey> {
    OwnedDefinitionKey::new(format!("def.{:016x}", sequence.get())).map_err(|_| {
        OwnedMappingError::Invalid {
            path: "registry.sequence".into(),
            kind: OwnedMappingErrorKind::WrongAllocatedKey,
        }
    })
}
pub(super) fn encoded_size(
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
