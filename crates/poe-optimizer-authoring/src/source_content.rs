//! Authored content obtained by consuming the exact checked source dependencies.
//! Neither source-envelope validation nor extraction admits executable rules.
use poe_optimizer_data::{
    owned_routing::ActionRoutingContent, owned_schema::OwnedDefinitionSchemaPackage,
};
use poe_optimizer_import::{
    owned_mapping::OwnedIdRegistrySnapshot,
    owned_release::{CheckedImportSource, OwnedReleaseError},
    owned_tree_policy::OwnedTreeNormalizationPolicy,
};

mod import_content;
mod recipe_source_content;
pub use import_content::{ImportSourceContent, ImportSourceInput};
pub use recipe_source_content::RuleSourceContent;

/// Source declarations and their original definition space. Rule semantics
/// require ordinary final validation after lowering against the completed schema.
pub struct ReleaseSourceParts {
    pub registry: OwnedIdRegistrySnapshot,
    pub schema: OwnedDefinitionSchemaPackage,
    pub rules: RuleSourceContent,
    pub routing: ActionRoutingContent,
    pub imports: ImportSourceContent,
}

impl ReleaseSourceParts {
    /// Consume the complete checked source, not raw policies or previously
    /// separated parts. Evaluation is absent because this owner never checked it.
    pub fn from_checked(source: CheckedImportSource) -> Result<Self, OwnedReleaseError> {
        let parts = source.into_parts();
        Ok(Self {
            registry: parts.registry,
            schema: parts.schema,
            rules: RuleSourceContent::from_checked_header(parts.rules),
            routing: parts.routing.into_authored_content(),
            imports: ImportSourceContent {
                mapping: parts.mapping.into_authored_content(),
                roles: parts.roles.into_authored_content(),
                normalization: parts.normalization.into_authored_content(),
                rewards: parts.rewards.into_authored_content(),
                items: parts.items.into_authored_content()?,
                item_source: parts.item_source.into_authored_content()?,
                tree: parts
                    .tree
                    .map(OwnedTreeNormalizationPolicy::into_authored_content),
            },
        })
    }
}
