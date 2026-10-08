//! Definition-side support-source coverage. Missing rows remain unknown at binding.
use super::*;
use poe_optimizer_core::owned_schema::SchemaState;

pub(super) fn validate<I: DefinitionSchemaIndex>(
    input: &RulePackageInput,
    index: &I,
    limits: RuleStorageLimits,
    usage: &mut RuleStorageUse,
) -> Result<(), RuleStorageError> {
    let Some(discovery) = &input.support_discovery else {
        return Ok(());
    };
    add(&mut usage.support_source_domains, discovery.providers.len())?;
    add(&mut usage.support_discovery_work, discovery.providers.len())?;
    usage.check(limits)?;
    // Bound both rows and gap payloads before constructing secondary indexes.
    for row in &discovery.providers {
        if let SchemaState::Unmapped { gaps } = &row.domain {
            add(&mut usage.gaps, gaps.len())?;
            add(&mut usage.support_discovery_work, gaps.len())?;
        }
    }
    usage.check(limits)?;
    let mut owners = BTreeSet::new();
    for row in &discovery.providers {
        let valid = match &row.owner {
            SchemaSubject::Definition(id) => {
                id.namespace() == &input.namespace
                    && index
                        .lookup_definition(id)
                        .is_some_and(|d| d.address() == *id)
            }
            SchemaSubject::Slot(id) => {
                id.namespace() == &input.namespace
                    && id.declaration().namespace() == &input.namespace
                    && index.lookup_slot(id).is_some_and(|s| s.address() == *id)
            }
        };
        if !valid || !owners.insert(owner_key(&row.owner)) {
            return Err(RuleStorageError::Structure(
                "unknown, foreign or duplicate support source owner",
            ));
        }
        if let SchemaState::Unmapped { gaps } = &row.domain {
            let mut seen = BTreeSet::new();
            if gaps.is_empty()
                || gaps.iter().any(|gap| {
                    gap.subject != row.owner
                        || gap.facet != SchemaFacet::GameRules
                        || !seen.insert(&gap.code)
                })
            {
                return Err(RuleStorageError::Structure(
                    "unmapped support source domain requires exact unique game-rule gaps",
                ));
            }
        }
    }
    Ok(())
}
