//! Explicit offline catalog extension. Runtime loaders still reject stale bindings.
use crate::{owned_mapping::*, owned_recipe::*, owned_skill_catalog::*};
use poe_optimizer_core::{
    data::DataIdentity,
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
};
use poe_optimizer_data::{
    owned_schema::{OwnedDefinitionSchemaPackage, SchemaPackageError},
    skill_identities::SkillIdentityCatalog,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default)]
pub struct CatalogRecipeLimits {
    pub recipe: OwnedRecipeLimits,
    pub catalog: SkillCatalogLimits,
}
#[derive(Debug, thiserror::Error)]
pub enum CatalogRecipeError {
    #[error(transparent)]
    Recipe(#[from] OwnedRecipeError),
    #[error(transparent)]
    Catalog(#[from] SkillCatalogError),
    #[error(transparent)]
    Mapping(#[from] OwnedMappingError),
    #[error(transparent)]
    Schema(#[from] SchemaPackageError),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("catalog extension changed an existing schema descriptor")]
    ChangedDescriptor,
    #[error("catalog recipe exceeds aggregate output bytes")]
    OutputLimit,
}
#[derive(Debug, Serialize)]
pub struct CatalogRecipeTransition {
    pub schema_version: u32,
    pub document_kind: &'static str,
    pub input: OwnedContentDigest,
    pub before_registry: OwnedContentDigest,
    pub after_registry: OwnedContentDigest,
    pub before_definitions: DataIdentity,
    pub after_definitions: DataIdentity,
    pub before_mapping: OwnedContentDigest,
    pub after_mapping: OwnedContentDigest,
    pub roles: OwnedContentDigest,
    pub preserved_definitions: usize,
    pub preserved_slots: usize,
    pub counts: SkillCatalogExtensionCounts,
    pub compilation: SkillCatalogReceipt,
    pub calculation: &'static str,
    pub whole_build_parity: &'static str,
    pub artifacts: Vec<RecipeArtifactManifest>,
}
struct ExtraArtifact {
    name: &'static str,
    bytes: Vec<u8>,
}
pub struct StagedCatalogRecipe {
    recipe: OwnedRecipeInput,
    assembled: StagedOwnedRecipe,
    mapping: OwnedMappingIndex,
    roles: OwnedSkillRoleIndex,
    transition: CatalogRecipeTransition,
    extra: Vec<ExtraArtifact>,
}
impl StagedCatalogRecipe {
    pub fn recipe(&self) -> &OwnedRecipeInput {
        &self.recipe
    }
    pub fn assembled(&self) -> &StagedOwnedRecipe {
        &self.assembled
    }
    pub fn mapping(&self) -> &OwnedMappingIndex {
        &self.mapping
    }
    pub fn roles(&self) -> &OwnedSkillRoleIndex {
        &self.roles
    }
    pub fn transition(&self) -> &CatalogRecipeTransition {
        &self.transition
    }
    /// Nine fixed safe basenames. transition.json identifies all other artifacts.
    pub fn artifacts(&self) -> impl Iterator<Item = (&str, &[u8])> {
        self.assembled
            .artifacts()
            .iter()
            .map(|a| (a.name(), a.bytes()))
            .chain(self.extra.iter().map(|a| (a.name, a.bytes.as_slice())))
    }
}

/// Validate the complete prior bundle, extend catalog identities, prove descriptor
/// preservation, and explicitly construct new bindings. This is an authoring
/// operation, never implicit recovery of a stale runtime artifact. The prior
/// mapping binds the policy version; the receipt records the current full policy.
/// It does not attest historical settings absent from the seed mapping.
pub fn extend_owned_catalog_recipe(
    input: OwnedRecipeInput,
    prior_mapping: MappingPackageInput,
    catalog: &SkillIdentityCatalog,
    source: &SourcePin,
    policy: &SkillCatalogPolicy,
    limits: CatalogRecipeLimits,
) -> Result<StagedCatalogRecipe, CatalogRecipeError> {
    let input_digest = digest_owned(
        "owned-catalog-recipe-input-v1",
        &(&input, &prior_mapping, catalog.data(), source, policy),
        limits.recipe.max_wire_bytes,
    )?;
    let before = assemble_owned_recipe(input, limits.recipe)?;
    let old_mapping = OwnedMappingIndex::new(
        prior_mapping,
        before.registry(),
        before.schema(),
        limits.catalog.mapping,
    )?;
    let extended = compile_owned_skill_catalog_extension(
        catalog,
        before.registry(),
        before.schema(),
        &old_mapping,
        source,
        policy,
        limits.catalog,
    )?;
    // Independent boundary check: all old descriptors, not just rule-referenced
    // ones, survive unchanged. New addresses cannot overwrite a reviewed schema.
    let old_schema = before.schema().input();
    let mut next_schema = old_schema.clone();
    next_schema.definitions = extended.definitions;
    next_schema.slots = extended.slots;
    let schema = OwnedDefinitionSchemaPackage::new(next_schema, limits.recipe.schema)?;
    let defs: BTreeMap<_, _> = schema
        .input()
        .definitions
        .iter()
        .map(|d| (d.address(), d))
        .collect();
    let slots: BTreeMap<_, _> = schema
        .input()
        .slots
        .iter()
        .map(|d| (d.address(), d))
        .collect();
    if old_schema
        .definitions
        .iter()
        .any(|d| defs.get(&d.address()).copied() != Some(d))
        || old_schema
            .slots
            .iter()
            .any(|d| slots.get(&d.address()).copied() != Some(d))
    {
        return Err(CatalogRecipeError::ChangedDescriptor);
    }
    before.registry().validate_successor(&extended.registry)?;
    let mut mapping_input = old_mapping.input().clone();
    mapping_input.registry = extended.registry.identity()?;
    mapping_input.definitions = schema.identity().clone();
    mapping_input.entries = extended.mappings;
    let mapping = OwnedMappingIndex::new(
        mapping_input,
        &extended.registry,
        &schema,
        limits.catalog.mapping,
    )?;
    let roles = OwnedSkillRoleIndex::new(
        OwnedSkillRolePackageInput {
            schema_version: OWNED_SKILL_ROLE_VERSION,
            namespace: schema.input().namespace.clone(),
            definitions: schema.identity().clone(),
            mapping: *mapping.identity(),
            compilation: extended.receipt.clone(),
            roles: extended.roles,
        },
        &mapping,
        &schema,
        limits.catalog,
    )?;
    let mut rules = before.rules().input().clone();
    let mut routing = before.routing().input().clone();
    rules.definitions = schema.identity().clone();
    routing.definitions = schema.identity().clone();
    let recipe = OwnedRecipeInput {
        schema_version: OWNED_RECIPE_VERSION,
        registry: extended.registry.input().clone(),
        schema: schema.input().clone(),
        rules,
        routing,
    };
    let assembled = assemble_owned_recipe(recipe.clone(), limits.recipe)?;
    let mut extra = vec![];
    let mut used = assembled
        .artifacts()
        .iter()
        .map(|a| a.bytes().len())
        .sum::<usize>();
    let maximum = limits.recipe.max_output_bytes;
    add(&mut extra, &mut used, maximum, "recipe.json", &recipe)?;
    add(
        &mut extra,
        &mut used,
        maximum,
        "mapping.json",
        mapping.input(),
    )?;
    add(&mut extra, &mut used, maximum, "roles.json", roles.input())?;
    let artifacts = assembled
        .artifacts()
        .iter()
        .map(|a| metadata(a.name(), a.bytes()))
        .chain(extra.iter().map(|a| metadata(a.name, &a.bytes)))
        .collect();
    let transition = CatalogRecipeTransition {
        schema_version: 1,
        document_kind: "owned_catalog_recipe_transition",
        input: input_digest,
        before_registry: before.registry().identity()?,
        after_registry: assembled.registry().identity()?,
        before_definitions: before.schema().identity().clone(),
        after_definitions: assembled.schema().identity().clone(),
        before_mapping: *old_mapping.identity(),
        after_mapping: *mapping.identity(),
        roles: *roles.identity(),
        preserved_definitions: old_schema.definitions.len(),
        preserved_slots: old_schema.slots.len(),
        counts: extended.counts,
        compilation: extended.receipt,
        calculation: "not_run",
        whole_build_parity: "not_established",
        artifacts,
    };
    add(
        &mut extra,
        &mut used,
        maximum,
        "transition.json",
        &transition,
    )?;
    Ok(StagedCatalogRecipe {
        recipe,
        assembled,
        mapping,
        roles,
        transition,
        extra,
    })
}
fn metadata(name: &'static str, bytes: &[u8]) -> RecipeArtifactManifest {
    RecipeArtifactManifest {
        file: name,
        bytes: bytes.len(),
        sha256: format!("{:x}", Sha256::digest(bytes)),
    }
}
fn add<T: Serialize>(
    out: &mut Vec<ExtraArtifact>,
    used: &mut usize,
    maximum: usize,
    name: &'static str,
    value: &T,
) -> Result<(), CatalogRecipeError> {
    let remaining = maximum
        .checked_sub(*used)
        .ok_or(CatalogRecipeError::OutputLimit)?;
    // Streaming check precedes allocation; these concrete serde DTOs have stable
    // serialization. Aggregate space is charged before retaining each buffer.
    digest_owned("owned-catalog-artifact-v1", value, remaining)?;
    let bytes = serde_json::to_vec(value)?;
    *used = used
        .checked_add(bytes.len())
        .filter(|n| *n <= maximum)
        .ok_or(CatalogRecipeError::OutputLimit)?;
    out.push(ExtraArtifact { name, bytes });
    Ok(())
}
