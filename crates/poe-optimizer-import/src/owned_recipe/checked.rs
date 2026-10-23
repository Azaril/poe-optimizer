//! Recipe validation independent of publication manifests and artifact buffers.
use super::{
    OWNED_RECIPE_VERSION, OwnedRecipeError, OwnedRecipeInput, OwnedRecipeLimits,
    OwnedRecipeManifest, OwnedRecipeView, RecipeArtifactManifest,
};
use crate::owned_mapping::{OwnedIdRegistrySnapshot, RegistryState};
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_schema::{SchemaClosure, SchemaSubject},
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

    /// Recipe commitment selected by validation. Construction retains the supplied
    /// typed input; complete release checking explicitly commits canonical inputs.
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

    pub(crate) fn view(&self) -> OwnedRecipeView<'_> {
        OwnedRecipeView {
            schema_version: OWNED_RECIPE_VERSION,
            registry: self.registry.input(),
            schema: self.schema.input(),
            rules: self.rules.input(),
            routing: self.routing.input(),
        }
    }

    /// Release checking commits the canonical recipe without recompiling it.
    /// A failed bounded digest leaves both the prior commitment and handles intact.
    pub(crate) fn canonicalize_input_identity(
        &mut self,
        maximum: usize,
    ) -> Result<(), OwnedRecipeError> {
        self.input_identity = digest_owned("owned-recipe-input-v1", &self.view(), maximum)?;
        Ok(())
    }

    pub(crate) fn manifest(
        &self,
        artifacts: Vec<RecipeArtifactManifest>,
    ) -> Result<OwnedRecipeManifest, OwnedRecipeError> {
        Ok(OwnedRecipeManifest {
            schema_version: 1,
            document_kind: "owned_recipe_manifest",
            recipe: self.input_identity,
            namespace: self.schema.input().namespace.clone(),
            registry: self.registry.identity()?,
            definitions: self.schema.identity().clone(),
            rules: *self.rules.identity(),
            compiled_rules: self.compiled_rules.identity(),
            routing: *self.routing.identity(),
            operations_version: self.rules.input().operations_version.clone(),
            rule_semantics_version: self.rules.input().semantics_version.clone(),
            partial_rule_owners: self
                .rules
                .input()
                .owners
                .iter()
                .filter(|v| matches!(v.programs.closure, SchemaClosure::Partial { .. }))
                .count(),
            partial_route_outputs: self
                .routing
                .input()
                .outputs
                .iter()
                .filter(|v| matches!(v.routes.closure, SchemaClosure::Partial { .. }))
                .count(),
            coverage: "declarations_preserved_not_certified",
            calculation: "not_run",
            whole_build_parity: "not_established",
            artifacts,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        owned_mapping::{OwnedIdRegistry, OwnedMappingLimits},
        owned_recipe::{assemble_owned_recipe, stage_checked_recipe},
    };
    use poe_optimizer_core::{
        owned_definitions::{GameVersionNamespace, GemDefId, OwnedDefinitionKey},
        owned_routing::{ActionRoutingInput, OWNED_ACTION_ROUTING_VERSION},
        owned_rules::{OWNED_RULE_OPERATIONS_V14, OWNED_RULE_PACKAGE_VERSION, RulePackageInput},
        owned_schema::DeclaredSet,
    };
    use poe_optimizer_data::owned_schema::{
        OWNED_SCHEMA_PACKAGE_VERSION, OwnedSchemaLimits, SchemaPackageInput,
    };

    fn shuffled_recipe() -> OwnedRecipeInput {
        let key = |value| OwnedDefinitionKey::new(value).unwrap();
        let namespace = GameVersionNamespace::new("recipe-staging-test", "v1").unwrap();
        let mut registry =
            OwnedIdRegistry::empty(namespace.clone(), OwnedMappingLimits::default()).unwrap();
        let _: GemDefId = registry.allocate_definition().unwrap();
        let _: GemDefId = registry.allocate_definition().unwrap();
        let mut registry = registry.input().clone();
        registry.entries.reverse();
        let schema = OwnedDefinitionSchemaPackage::new(
            SchemaPackageInput {
                schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
                namespace: namespace.clone(),
                release: key("schema"),
                semantics_version: key("inputs"),
                definitions: vec![],
                slots: vec![],
            },
            OwnedSchemaLimits::default(),
        )
        .unwrap();
        OwnedRecipeInput {
            schema_version: OWNED_RECIPE_VERSION,
            registry,
            schema: schema.input().clone(),
            rules: RulePackageInput {
                schema_version: OWNED_RULE_PACKAGE_VERSION,
                namespace: namespace.clone(),
                release: key("rules"),
                semantics_version: key("rules-semantics"),
                operations_version: key(OWNED_RULE_OPERATIONS_V14),
                definitions: schema.identity().clone(),
                tables: vec![],
                owners: vec![],
                receivers: DeclaredSet::complete(vec![]),
                effect_applications: None,
                contribution_queries: None,
                existing_actor_rules: None,
                support_discovery: None,
            },
            routing: ActionRoutingInput {
                schema_version: OWNED_ACTION_ROUTING_VERSION,
                namespace,
                release: key("routing"),
                definitions: schema.identity().clone(),
                outputs: vec![],
            },
        }
    }

    #[test]
    fn borrowed_views_preserve_wire_bytes_and_explicit_commitment_selection() {
        let input = shuffled_recipe();
        let limits = OwnedRecipeLimits::default();
        let input_bytes = serde_json::to_vec(&input).unwrap();
        let input_view = OwnedRecipeView::from(&input);
        assert_eq!(serde_json::to_vec(&input_view).unwrap(), input_bytes);
        assert_eq!(input_view.to_input(), input);
        let supplied =
            digest_owned("owned-recipe-input-v1", &input, limits.max_wire_bytes).unwrap();
        let mut checked = CheckedOwnedRecipe::new(input.clone(), limits).unwrap();
        assert_eq!(checked.input_identity(), supplied);
        assert_eq!(
            assemble_owned_recipe(input, limits)
                .unwrap()
                .manifest()
                .recipe,
            supplied
        );

        let canonical_input = checked.view().to_input();
        let canonical_bytes = serde_json::to_vec(&canonical_input).unwrap();
        assert_eq!(
            serde_json::to_vec(&checked.view()).unwrap(),
            canonical_bytes
        );
        assert_ne!(canonical_bytes, input_bytes);
        let canonical = digest_owned(
            "owned-recipe-input-v1",
            &canonical_input,
            limits.max_wire_bytes,
        )
        .unwrap();
        assert_ne!(canonical, supplied);
        let compiled = Arc::clone(checked.compiled_rules());
        let schema = Arc::clone(checked.schema());
        let rules = Arc::clone(checked.rules());
        let routing = Arc::clone(checked.routing());
        assert!(checked.canonicalize_input_identity(1).is_err());
        assert_eq!(checked.input_identity(), supplied);
        checked
            .canonicalize_input_identity(limits.max_wire_bytes)
            .unwrap();
        assert_eq!(checked.input_identity(), canonical);

        let staged = stage_checked_recipe(checked, limits).unwrap();
        assert!(Arc::ptr_eq(staged.checked().compiled_rules(), &compiled));
        assert!(Arc::ptr_eq(staged.checked().schema(), &schema));
        assert!(Arc::ptr_eq(staged.checked().rules(), &rules));
        assert!(Arc::ptr_eq(staged.checked().routing(), &routing));
        let rebuilt = assemble_owned_recipe(canonical_input, limits).unwrap();
        assert_eq!(staged.manifest(), rebuilt.manifest());
        assert_eq!(
            staged
                .artifacts()
                .iter()
                .map(|artifact| (artifact.name(), artifact.bytes()))
                .collect::<Vec<_>>(),
            rebuilt
                .artifacts()
                .iter()
                .map(|artifact| (artifact.name(), artifact.bytes()))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn staging_checked_recipe_enforces_exact_aggregate_output_bound() {
        let checked =
            CheckedOwnedRecipe::new(shuffled_recipe(), OwnedRecipeLimits::default()).unwrap();
        let staged = stage_checked_recipe(checked.clone(), OwnedRecipeLimits::default()).unwrap();
        let output_bytes = staged
            .artifacts()
            .iter()
            .map(|artifact| artifact.bytes().len())
            .sum();
        let mut limits = OwnedRecipeLimits {
            max_output_bytes: output_bytes,
            ..OwnedRecipeLimits::default()
        };
        assert!(stage_checked_recipe(checked.clone(), limits).is_ok());
        limits.max_output_bytes -= 1;
        assert!(matches!(
            stage_checked_recipe(checked, limits),
            Err(OwnedRecipeError::Limit("output bytes"))
        ));
    }
}
