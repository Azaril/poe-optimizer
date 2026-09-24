//! Exact, offline Gem schema knowledge migration; never an implicit repair.
use super::{Result, SuccessorBindings, SuccessorBundleError, SuccessorBundleLimits, charge};
use crate::{
    owned_mapping::{OwnedMappingIndex, SourcePin},
    owned_recipe::StagedOwnedRecipe,
    owned_skill_catalog::{
        OwnedGemMaterialization, OwnedGemRole, OwnedPrimarySkill, OwnedSkillRoleIndex,
    },
};
use poe_optimizer_core::{
    data::DataIdentity, owned_build::DeclaredSlot, owned_content::OwnedContentDigest,
    owned_definitions::*, owned_schema::*,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// V4 changes only explicitly listed existing Unmapped Gem descriptors to Known.
/// New parameter slots must belong to those gems. Both schemas and the exact
/// prior source/role context are committed; this does not establish game rules.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GemSchemaRefinement {
    pub schema_version: u32,
    pub before: DataIdentity,
    pub after: DataIdentity,
    pub prior_mapping: OwnedContentDigest,
    pub prior_roles: OwnedContentDigest,
    pub source: SourcePin,
    pub gems: Vec<GemDefId>,
    pub parameters: Vec<DeclaredSlot<ParameterSlotDefId>>,
}

fn invalid(message: &'static str) -> SuccessorBundleError {
    SuccessorBundleError::Refinement(message)
}
fn selected(policy: &GemSchemaRefinement) -> Result<BTreeSet<GemDefId>> {
    if policy.schema_version != 4 || policy.before == policy.after || policy.gems.is_empty() {
        return Err(invalid("invalid gem migration endpoints or version"));
    }
    if policy.gems.windows(2).any(|pair| pair[0] >= pair[1])
        || policy
            .parameters
            .windows(2)
            .any(|pair| pair[0].slot.key() >= pair[1].slot.key())
    {
        return Err(invalid(
            "gem migration subjects need unique canonical order",
        ));
    }
    Ok(policy.gems.iter().cloned().collect())
}
fn incomplete_ports(schema: &GemSchema) -> bool {
    !schema.skills.is_complete()
        && !schema.declarations.choices.is_complete()
        && !schema.declarations.grants.is_complete()
        && !schema.declarations.actors.is_complete()
        && !schema.declarations.skill_grants.is_complete()
        && !schema.declarations.outputs.is_complete()
        && !schema.declarations.sockets.is_complete()
}
fn index_parameters(
    members: &[DeclaredSlot<ParameterSlotDefId>],
    left: &mut usize,
) -> Result<BTreeSet<DeclaredSlot<ParameterSlotDefId>>> {
    // Charge the original list before allocating the index. A shared counter
    // bounds aggregate declarations across all promoted owners.
    charge(left, members.len(), "gem migration parameter memberships")?;
    Ok(members.iter().cloned().collect())
}
pub(super) fn validate_current<I: DefinitionSchemaIndex>(
    policy: &GemSchemaRefinement,
    index: &I,
) -> Result<()> {
    let mut membership_left = SuccessorBundleLimits::default().max_validation_entries;
    charge(
        &mut membership_left,
        policy.gems.len().saturating_add(policy.parameters.len()),
        "gem migration entries",
    )?;
    let gems = selected(policy)?;
    if &policy.after != index.identity() {
        return Err(invalid("current gem migration endpoint binding"));
    }
    let mut parameters = BTreeMap::new();
    for gem in &policy.gems {
        if gem.namespace() != index.namespace() {
            return Err(invalid("foreign gem migration owner"));
        }
        let SchemaLookup::Known(schema) = index.definition(gem) else {
            return Err(invalid("gem migration requires Known current gem"));
        };
        if !incomplete_ports(schema) {
            return Err(invalid("gem migration closes unreviewed provider ports"));
        }
        parameters.insert(
            gem.clone(),
            index_parameters(
                &schema.declarations.parameters.members,
                &mut membership_left,
            )?,
        );
    }
    for slot in &policy.parameters {
        let SlotOwnerDefId::Gem(gem) = &slot.declaration else {
            return Err(invalid("new parameter is not owned by a gem"));
        };
        if !gems.contains(gem) || slot.slot.namespace() != index.namespace() {
            return Err(invalid("new parameter belongs to an unlisted gem"));
        }
        let SchemaLookup::Known(schema) = index.slot(slot) else {
            return Err(invalid("new gem parameter must have Known schema"));
        };
        if schema.sites != [ParameterSite::GemParameter] {
            return Err(invalid("new gem parameter has foreign use sites"));
        }
        if !parameters
            .get(gem)
            .is_some_and(|members| members.contains(slot))
        {
            return Err(invalid("new gem parameter is not declared by its owner"));
        }
    }
    Ok(())
}
fn physical<I: DefinitionSchemaIndex>(
    policy: &GemSchemaRefinement,
    roles: &OwnedSkillRoleIndex,
    index: &I,
) -> Result<()> {
    for gem in &policy.gems {
        let Some(row) = roles.role(gem) else {
            return Err(invalid("gem migration lacks prior role evidence"));
        };
        let (
            OwnedGemMaterialization::Physical,
            OwnedGemRole::Known(role),
            OwnedPrimarySkill::Known(primary),
        ) = (&row.materialization, &row.role, &row.primary)
        else {
            return Err(invalid(
                "gem migration requires an explicitly physical known role",
            ));
        };
        let SchemaLookup::Known(schema) = index.definition(gem) else {
            return Err(invalid("gem migration requires Known current gem"));
        };
        if schema.roles.as_slice() != [*role] || !schema.skills.members.contains(primary) {
            return Err(invalid(
                "gem migration contradicts the prior primary skill or role",
            ));
        }
    }
    Ok(())
}
impl GemSchemaRefinement {
    pub(crate) fn validate_prior_catalog<I: DefinitionSchemaIndex>(
        &self,
        mapping: &OwnedMappingIndex,
        roles: &OwnedSkillRoleIndex,
        after: &I,
    ) -> Result<()> {
        if self.prior_mapping != *mapping.identity()
            || self.prior_roles != *roles.identity()
            || self.source != mapping.input().source
            || self.before != mapping.input().definitions
            || self.before != roles.input().definitions
            || roles.input().mapping != *mapping.identity()
        {
            return Err(invalid("gem migration prior source or role binding"));
        }
        validate_current(self, after)?;
        physical(self, roles, after)
    }

    /// Validate a persisted current bundle and its metadata. Historical digests
    /// are checked against its manifest; absent predecessor bytes are not proved.
    pub fn validate_current_catalog<I: DefinitionSchemaIndex>(
        &self,
        before: &SuccessorBindings,
        after: &SuccessorBindings,
        mapping: &OwnedMappingIndex,
        roles: &OwnedSkillRoleIndex,
        index: &I,
    ) -> Result<()> {
        if self.before != before.definitions
            || self.after != after.definitions
            || self.prior_mapping != before.mapping
            || self.prior_roles != before.roles
            || self.source != mapping.input().source
            || mapping.input().registry != after.registry
            || *mapping.identity() != after.mapping
            || *roles.identity() != after.roles
            || roles.input().mapping != after.mapping
            || mapping.input().definitions != self.after
            || roles.input().definitions != self.after
        {
            return Err(invalid("current gem migration catalog metadata binding"));
        }
        validate_current(self, index)?;
        physical(self, roles, index)
    }
}

pub(super) fn preserve(
    policy: &GemSchemaRefinement,
    before: &StagedOwnedRecipe,
    after: &StagedOwnedRecipe,
    left: &mut usize,
) -> Result<(usize, usize)> {
    if &policy.before != before.schema().identity() || &policy.after != after.schema().identity() {
        return Err(invalid("gem migration endpoint binding"));
    }
    validate_current(policy, after.schema())?;
    let gems = selected(policy)?;
    charge(
        left,
        policy.gems.len() + policy.parameters.len(),
        "gem migration entries",
    )?;
    let previous = before.schema().input();
    let current = after.schema().input();
    charge(
        left,
        previous.definitions.len()
            + current.definitions.len()
            + previous.slots.len()
            + current.slots.len(),
        "gem migration descriptors",
    )?;
    let mut restored = current.clone();
    let old: BTreeMap<_, _> = previous
        .definitions
        .iter()
        .map(|row| (row.address(), row))
        .collect();
    let mut changed = 0;
    for row in &mut restored.definitions {
        let address = row.address();
        let Some(prior) = old.get(&address) else {
            return Err(invalid("gem migration cannot add definitions"));
        };
        if let DefinitionAddress::Gem(gem) = &address
            && gems.contains(gem)
        {
            if !matches!(prior, DefinitionDescriptor::Gem(entry) if matches!(entry.schema, SchemaState::Unmapped { .. }))
            {
                return Err(invalid("gem migration requires Unmapped predecessor"));
            }
            *row = (*prior).clone();
            changed += 1;
        }
    }
    if changed != gems.len() {
        return Err(invalid("gem migration owner absent from predecessor"));
    }
    let requested: BTreeSet<_> = policy
        .parameters
        .iter()
        .cloned()
        .map(SlotAddress::Parameter)
        .collect();
    let old_slots: BTreeSet<_> = previous.slots.iter().map(SlotDescriptor::address).collect();
    if requested.iter().any(|slot| old_slots.contains(slot)) {
        return Err(invalid("gem migration parameter already exists"));
    }
    let actual: BTreeSet<_> = current
        .slots
        .iter()
        .map(SlotDescriptor::address)
        .filter(|slot| !old_slots.contains(slot))
        .collect();
    if actual != requested {
        return Err(invalid("gem migration contains unlisted new slots"));
    }
    restored
        .slots
        .retain(|slot| !requested.contains(&slot.address()));
    if &restored != previous {
        return Err(invalid(
            "gem migration changed unrelated schema or existing slots",
        ));
    }
    let mut registry = before.registry().clone();
    for slot in &policy.parameters {
        if registry.allocate_slot::<ParameterSlotDefinition>(slot.declaration.clone())? != *slot {
            return Err(invalid("gem migration parameter registry order"));
        }
    }
    if registry.input() != after.registry().input() {
        return Err(invalid("gem migration registry contains other edits"));
    }
    let mut rules = after.rules().input().clone();
    rules.definitions = before.rules().input().definitions.clone();
    let mut routing = after.routing().input().clone();
    routing.definitions = before.routing().input().definitions.clone();
    if &rules != before.rules().input() || &routing != before.routing().input() {
        return Err(invalid("gem migration changed existing rules or routing"));
    }
    Ok((changed, 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameter_membership_index_charges_aggregate_original_lists_before_collection() {
        let namespace = GameVersionNamespace::new("test-game", "test-version").unwrap();
        let owner = SlotOwnerDefId::Gem(GemDefId::new(
            namespace.clone(),
            OwnedDefinitionKey::new("gem").unwrap(),
        ));
        let slot = |name: &str| DeclaredSlot {
            declaration: owner.clone(),
            slot: ParameterSlotDefId::new(
                namespace.clone(),
                OwnedDefinitionKey::new(name).unwrap(),
            ),
        };
        let first = vec![slot("one"), slot("two")];
        let second = vec![slot("three"), slot("four")];
        let mut remaining = 3;
        let indexed = index_parameters(&first, &mut remaining).unwrap();
        assert_eq!(remaining, 1);
        assert_eq!(indexed.len(), 2);
        assert!(indexed.contains(&first[0]));
        assert!(matches!(
            index_parameters(&second, &mut remaining),
            Err(SuccessorBundleError::Limit(
                "gem migration parameter memberships"
            ))
        ));
        let mut exact = 4;
        index_parameters(&first, &mut exact).unwrap();
        index_parameters(&second, &mut exact).unwrap();
        assert_eq!(exact, 0);
        assert!(index_parameters(&first, &mut exact).is_err());
        // Deduplication must not erase the amount of supplied validation work.
        let mut tight = 1;
        assert!(index_parameters(&[first[0].clone(), first[0].clone()], &mut tight).is_err());
    }
}
