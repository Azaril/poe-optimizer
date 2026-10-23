//! Recipe validation independent of publication manifests and artifact buffers.
use super::{OWNED_RECIPE_VERSION, OwnedRecipeError, OwnedRecipeInput, OwnedRecipeLimits};
use crate::owned_mapping::{OwnedIdRegistrySnapshot, RegistryState};
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_schema::SchemaSubject,
};
use poe_optimizer_data::{
    owned_routing::OwnedActionRouting, owned_rules::OwnedRulePackage,
    owned_schema::OwnedDefinitionSchemaPackage,
};
use poe_optimizer_engine::owned_rules::CompiledRulePackage;
use std::sync::Arc;

/// Immutable checked storage and executable packages. Validation preserves all
/// declared coverage; it neither allocates IDs nor establishes build completeness.
#[derive(Clone, Debug)]
pub struct CheckedOwnedRecipe {
    input_identity: OwnedContentDigest,
    registry: OwnedIdRegistrySnapshot,
    schema: Arc<OwnedDefinitionSchemaPackage>,
    rules: Arc<OwnedRulePackage>,
    routing: Arc<OwnedActionRouting>,
    compiled_rules: Arc<CompiledRulePackage>,
}

impl CheckedOwnedRecipe {
    pub fn new(
        input: OwnedRecipeInput,
        limits: OwnedRecipeLimits,
    ) -> Result<Self, OwnedRecipeError> {
        limits.validate()?;
        if input.schema_version != OWNED_RECIPE_VERSION {
            return Err(OwnedRecipeError::Version(input.schema_version));
        }
        // Preserve the supplied recipe commitment before canonicalization and
        // bound it before constructors allocate indexes or clone constituents.
        let input_identity = digest_owned("owned-recipe-input-v1", &input, limits.max_wire_bytes)?;
        let registry = OwnedIdRegistrySnapshot::new(input.registry, limits.registry)?;
        let schema = OwnedDefinitionSchemaPackage::new(input.schema, limits.schema)?;
        if registry.input().namespace != schema.input().namespace {
            return Err(OwnedRecipeError::Namespace);
        }
        let count = schema
            .input()
            .definitions
            .len()
            .checked_add(schema.input().slots.len())
            .ok_or(OwnedRecipeError::Limit("registry checks"))?;
        if count > limits.max_registry_checks {
            return Err(OwnedRecipeError::Limit("registry checks"));
        }
        for subject in schema
            .input()
            .definitions
            .iter()
            .map(|value| SchemaSubject::Definition(value.address()))
            .chain(
                schema
                    .input()
                    .slots
                    .iter()
                    .map(|value| SchemaSubject::Slot(value.address())),
            )
        {
            if !registry
                .entry(&subject)
                .is_some_and(|entry| matches!(entry.state, RegistryState::Active))
            {
                return Err(OwnedRecipeError::Unregistered(Box::new(subject)));
            }
        }
        // Incoming bindings must already refer to the final canonical schema.
        let rules = OwnedRulePackage::new(input.rules, &schema, limits.rules)?;
        let routing = OwnedActionRouting::new(input.routing, &schema, limits.routing)?;
        let compiled_rules = CompiledRulePackage::compile_stored(&rules, &schema, limits.compile)?;
        Ok(Self {
            input_identity,
            registry,
            schema: Arc::new(schema),
            rules: Arc::new(rules),
            routing: Arc::new(routing),
            compiled_rules: Arc::new(compiled_rules),
        })
    }

    /// Digest of the supplied typed recipe, before constituent canonicalization.
    pub fn input_identity(&self) -> OwnedContentDigest {
        self.input_identity
    }
    pub fn registry(&self) -> &OwnedIdRegistrySnapshot {
        &self.registry
    }
    pub fn schema(&self) -> &Arc<OwnedDefinitionSchemaPackage> {
        &self.schema
    }
    pub fn rules(&self) -> &Arc<OwnedRulePackage> {
        &self.rules
    }
    pub fn routing(&self) -> &Arc<OwnedActionRouting> {
        &self.routing
    }
    pub fn compiled_rules(&self) -> &Arc<CompiledRulePackage> {
        &self.compiled_rules
    }
}
