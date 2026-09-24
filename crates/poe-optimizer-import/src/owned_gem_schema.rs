//! Bounded, explicit authoring of physical Gem input schemas. No source execution.
use crate::{
    owned_mapping::{OwnedMappingIndex, SourcePin},
    owned_recipe::{
        OwnedRecipeError, OwnedRecipeInput, OwnedRecipeLimits, StagedOwnedRecipe,
        assemble_owned_recipe,
    },
    owned_skill_catalog::OwnedSkillRoleIndex,
    owned_successor::{GemSchemaRefinement, SuccessorBundleError, validate_gem_schema_refinement},
};
use poe_optimizer_core::{
    data::DataIdentity,
    owned_build::DeclaredSlot,
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GemSchemaMigrationInput {
    pub schema_version: u32,
    pub before: DataIdentity,
    pub mapping: OwnedContentDigest,
    pub roles: OwnedContentDigest,
    pub source: SourcePin,
    /// Existing Gem IDs, with explicit Known schemas in canonical ID order.
    pub gems: Vec<DefinitionEntry<GemDefId, GemSchema>>,
    /// Only new, explicitly declared Gem parameter slots, in allocation order.
    pub parameters: Vec<DefinitionEntry<DeclaredSlot<ParameterSlotDefId>, ParameterSlotSchema>>,
}
#[derive(Clone, Copy, Debug)]
pub struct GemSchemaMigrationLimits {
    pub max_wire_bytes: usize,
    pub max_entries: usize,
    pub recipe: OwnedRecipeLimits,
}
impl Default for GemSchemaMigrationLimits {
    fn default() -> Self {
        Self {
            max_wire_bytes: 16 * 1024 * 1024,
            max_entries: 100_000,
            recipe: Default::default(),
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GemSchemaMigrationReceipt {
    pub input: OwnedContentDigest,
    pub before: DataIdentity,
    pub after: DataIdentity,
    pub promoted_gems: usize,
    pub allocated_parameters: usize,
}
pub struct StagedGemSchemaMigration {
    pub successor: OwnedRecipeInput,
    pub refinement: GemSchemaRefinement,
    pub receipt: GemSchemaMigrationReceipt,
}
fn invalid(message: &'static str) -> SuccessorBundleError {
    SuccessorBundleError::Refinement(message)
}

pub fn stage_owned_gem_schema(
    base: &StagedOwnedRecipe,
    mapping: &OwnedMappingIndex,
    roles: &OwnedSkillRoleIndex,
    input: &GemSchemaMigrationInput,
    limits: GemSchemaMigrationLimits,
) -> Result<StagedGemSchemaMigration, SuccessorBundleError> {
    let hard = GemSchemaMigrationLimits::default();
    if limits.max_wire_bytes == 0
        || limits.max_wire_bytes > hard.max_wire_bytes
        || limits.max_entries == 0
        || limits.max_entries > hard.max_entries
    {
        return Err(invalid("invalid gem migration limits"));
    }
    let count = input
        .gems
        .len()
        .checked_add(input.parameters.len())
        .ok_or(SuccessorBundleError::Limit("gem migration entries"))?;
    if count > limits.max_entries {
        return Err(SuccessorBundleError::Limit("gem migration entries"));
    }
    let commitment = digest_owned(
        "owned-gem-schema-migration-input-v1",
        input,
        limits.max_wire_bytes,
    )?;
    if input.schema_version != 1
        || input.before != *base.schema().identity()
        || input.mapping != *mapping.identity()
        || input.roles != *roles.identity()
        || input.source != mapping.input().source
        || input.gems.is_empty()
    {
        return Err(invalid("gem migration input binding"));
    }
    if input.gems.windows(2).any(|pair| pair[0].id >= pair[1].id)
        || input
            .parameters
            .windows(2)
            .any(|pair| pair[0].id.slot.key() >= pair[1].id.slot.key())
    {
        return Err(invalid("gem migration inputs need unique canonical order"));
    }
    let mut schema = base.schema().input().clone();
    let positions: BTreeMap<_, _> = schema
        .definitions
        .iter()
        .enumerate()
        .map(|(i, row)| (row.address(), i))
        .collect();
    for gem in &input.gems {
        if gem.id.namespace() != base.schema().namespace()
            || !matches!(gem.schema, SchemaState::Known(_))
        {
            return Err(invalid(
                "gem migration requires same-namespace Known inputs",
            ));
        }
        let Some(position) = positions.get(&gem.id.address()) else {
            return Err(invalid("gem migration cannot allocate gem identities"));
        };
        if !matches!(&schema.definitions[*position], DefinitionDescriptor::Gem(old) if matches!(old.schema, SchemaState::Unmapped { .. }))
        {
            return Err(invalid("gem migration cannot rewrite Known descriptors"));
        }
        schema.definitions[*position] = DefinitionDescriptor::Gem(gem.clone());
    }
    let mut registry = base.registry().clone();
    for parameter in &input.parameters {
        if !matches!(parameter.schema, SchemaState::Known(_)) {
            return Err(invalid("new gem parameter must be Known"));
        }
        let SlotOwnerDefId::Gem(gem) = &parameter.id.declaration else {
            return Err(invalid("new parameter is not owned by a gem"));
        };
        if input.gems.binary_search_by(|row| row.id.cmp(gem)).is_err() {
            return Err(invalid("new parameter belongs to an unlisted gem"));
        }
        if registry.allocate_slot::<ParameterSlotDefinition>(parameter.id.declaration.clone())?
            != parameter.id
        {
            return Err(invalid("gem migration parameter registry order"));
        }
        schema
            .slots
            .push(SlotDescriptor::Parameter(parameter.clone()));
    }
    let schema = OwnedDefinitionSchemaPackage::new(schema, limits.recipe.schema)
        .map_err(OwnedRecipeError::from)?;
    let mut rules = base.rules().input().clone();
    let mut routing = base.routing().input().clone();
    rules.definitions = schema.identity().clone();
    routing.definitions = schema.identity().clone();
    let successor = OwnedRecipeInput {
        schema_version: 1,
        registry: registry.input().clone(),
        schema: schema.input().clone(),
        rules,
        routing,
    };
    let after = assemble_owned_recipe(successor.clone(), limits.recipe)?;
    let refinement = GemSchemaRefinement {
        schema_version: 4,
        before: input.before.clone(),
        after: after.schema().identity().clone(),
        prior_mapping: input.mapping,
        prior_roles: input.roles,
        source: input.source.clone(),
        gems: input.gems.iter().map(|gem| gem.id.clone()).collect(),
        parameters: input
            .parameters
            .iter()
            .map(|slot| slot.id.clone())
            .collect(),
    };
    validate_gem_schema_refinement(&refinement, base, &after, mapping, roles)?;
    Ok(StagedGemSchemaMigration {
        receipt: GemSchemaMigrationReceipt {
            input: commitment,
            before: refinement.before.clone(),
            after: refinement.after.clone(),
            promoted_gems: refinement.gems.len(),
            allocated_parameters: refinement.parameters.len(),
        },
        successor,
        refinement,
    })
}
