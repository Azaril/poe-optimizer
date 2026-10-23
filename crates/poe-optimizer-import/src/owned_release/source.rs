//! Check the original source dependency space before authoring changes it.
use super::{
    ImportSourceContent, OwnedReleaseInput, OwnedReleaseLimits, ReleaseView, Result,
    RuleSourceContent, checked::input_domain, constituents::CheckedImportConstituents,
    preflight_view, source_union,
};
use crate::{
    owned_mapping::OwnedIdRegistrySnapshot,
    owned_recipe::{OwnedRecipeError, check_recipe_input, validate_definition_space},
};
use poe_optimizer_core::{owned_content::digest_owned, owned_rules::OWNED_RULE_PACKAGE_VERSION};
use poe_optimizer_data::{
    owned_routing::{ActionRoutingContent, OwnedActionRouting},
    owned_rules::RuleStorageError,
    owned_schema::OwnedDefinitionSchemaPackage,
};

/// Original source dependencies checked independently of executable programs.
/// Private storage cannot be forged from a raw runtime policy or deserialized.
pub struct CheckedImportSource {
    registry: OwnedIdRegistrySnapshot,
    schema: OwnedDefinitionSchemaPackage,
    rules: RuleSourceContent,
    routing: OwnedActionRouting,
    imports: CheckedImportConstituents,
}

/// Source parts obtained from the exact authenticated input. Routing and Import
/// content come from checked owners; rule semantics still require final ordinary
/// validation after source lowering against the completed definition schema.
pub struct ReleaseSourceParts {
    pub registry: OwnedIdRegistrySnapshot,
    pub schema: OwnedDefinitionSchemaPackage,
    pub rules: RuleSourceContent,
    pub routing: ActionRoutingContent,
    pub imports: ImportSourceContent,
}

impl CheckedImportSource {
    pub fn registry(&self) -> &OwnedIdRegistrySnapshot {
        &self.registry
    }
    pub fn schema(&self) -> &OwnedDefinitionSchemaPackage {
        &self.schema
    }

    /// Consume the actual checked owners before changing the dependency space.
    /// Evaluation inputs are deliberately absent: this checker never certified them.
    pub fn into_parts(self) -> Result<ReleaseSourceParts> {
        Ok(ReleaseSourceParts {
            registry: self.registry,
            schema: self.schema,
            rules: self.rules,
            routing: self.routing.into_authored_content(),
            imports: ImportSourceContent::from_checked(self.imports)?,
        })
    }
}

/// Validate source definitions, routing and Import dependencies against the
/// registry and schema supplied by this exact input, without changing bindings.
///
/// The original complete input is bounded before constituent cloning. Programs
/// may still be supplied by source lowering, so only the rule header's version,
/// namespace and definition binding are checked here. Full rule semantics and
/// evaluation-artifact dependencies remain mandatory final-release checks.
///
/// The result owns checked Import dependencies, never a checked release or
/// executable package. Content extraction consumes these exact owners; it grants
/// no authority to discard seals from another supplied artifact.
pub fn check_release_source_dependencies(
    input: &OwnedReleaseInput,
    limits: OwnedReleaseLimits,
) -> Result<CheckedImportSource> {
    limits.validate()?;
    let view = ReleaseView::from(input);
    let domain = input_domain(view.schema_version, view.evaluation.is_some())?;
    preflight_view(&view, limits)?;
    digest_owned(domain, &view, limits.max_input_bytes)?;
    check_recipe_input(view.recipe, limits.recipe)?;

    let registry =
        OwnedIdRegistrySnapshot::new(input.recipe.registry.clone(), limits.recipe.registry)
            .map_err(OwnedRecipeError::from)?;
    let schema =
        OwnedDefinitionSchemaPackage::new(input.recipe.schema.clone(), limits.recipe.schema)
            .map_err(OwnedRecipeError::from)?;
    validate_definition_space(&registry, &schema, limits.recipe.max_registry_checks)?;

    // A program-free shell cannot pass whole-package storage validation when
    // contribution queries refer to programs added later by DSL lowering. Check
    // its original binding now; final assembly still runs the complete checker.
    let rules = &input.recipe.rules;
    if rules.schema_version != OWNED_RULE_PACKAGE_VERSION {
        return Err(
            OwnedRecipeError::Rules(RuleStorageError::Version(rules.schema_version)).into(),
        );
    }
    if rules.namespace != schema.input().namespace || rules.definitions != *schema.identity() {
        return Err(OwnedRecipeError::Rules(RuleStorageError::Binding).into());
    }
    let routing =
        OwnedActionRouting::new(input.recipe.routing.clone(), &schema, limits.recipe.routing)
            .map_err(OwnedRecipeError::from)?;

    let imported = CheckedImportConstituents::check(&view, &registry, &schema, limits)?;
    source_union(
        &imported.mapping,
        &imported.roles,
        &imported.item_source,
        imported.tree.as_ref(),
    )?;
    Ok(CheckedImportSource {
        registry,
        schema,
        // Aggregate preflight and the complete release/recipe byte limits ran
        // before any constituent clone, including this unvalidated rule payload.
        rules: RuleSourceContent::from_checked_header(rules.clone()),
        routing,
        imports: imported,
    })
}
