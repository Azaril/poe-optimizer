//! Check the original source dependency space before authoring changes it.
use super::{
    OwnedReleaseInput, OwnedReleaseLimits, ReleaseView, Result,
    checked::input_domain,
    constituents::{CheckedImportConstituents, CheckedNormalizationPolicy},
    preflight_view, source_union,
};
use crate::{
    owned_item_lines::OwnedItemLinePolicy,
    owned_item_source::ItemSourceLayoutPolicy,
    owned_mapping::{OwnedIdRegistrySnapshot, OwnedMappingIndex},
    owned_recipe::{OwnedRecipeError, check_recipe_input, validate_definition_space},
    owned_reward_policy::OwnedRewardPolicy,
    owned_skill_roles::OwnedSkillRoleIndex,
    owned_tree_policy::OwnedTreeNormalizationPolicy,
};
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_rules::{OWNED_RULE_PACKAGE_VERSION, RulePackageInput},
};
use poe_optimizer_data::{
    owned_routing::OwnedActionRouting, owned_rules::RuleStorageError,
    owned_schema::OwnedDefinitionSchemaPackage,
};

/// Original source dependencies checked independently of executable programs.
/// Private storage cannot be forged from a raw runtime policy or deserialized.
pub struct CheckedImportSource {
    registry: OwnedIdRegistrySnapshot,
    schema: OwnedDefinitionSchemaPackage,
    rules: RulePackageInput,
    routing: OwnedActionRouting,
    imports: CheckedImportConstituents,
}

/// Owned constituents from the authenticated source. Only the rule envelope was
/// checked; its original payload and seals remain unchanged. These parts alone
/// are neither an executable package nor a certificate for a reassembled source.
pub struct CheckedImportSourceParts {
    pub registry: OwnedIdRegistrySnapshot,
    pub schema: OwnedDefinitionSchemaPackage,
    pub rules: RulePackageInput,
    pub routing: OwnedActionRouting,
    pub mapping: OwnedMappingIndex,
    pub roles: OwnedSkillRoleIndex,
    pub normalization: CheckedNormalizationPolicy,
    pub rewards: OwnedRewardPolicy,
    pub items: OwnedItemLinePolicy,
    pub item_source: ItemSourceLayoutPolicy,
    pub tree: Option<OwnedTreeNormalizationPolicy>,
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
    pub fn into_parts(self) -> CheckedImportSourceParts {
        CheckedImportSourceParts {
            registry: self.registry,
            schema: self.schema,
            rules: self.rules,
            routing: self.routing,
            mapping: self.imports.mapping,
            roles: self.imports.roles,
            normalization: self.imports.normalization,
            rewards: self.imports.rewards,
            items: self.imports.items,
            item_source: self.imports.item_source,
            tree: self.imports.tree,
        }
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
        rules: rules.clone(),
        routing,
        imports: imported,
    })
}
