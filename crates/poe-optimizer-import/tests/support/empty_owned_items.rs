//! Explicit empty fixture policy; production callers must supply an artifact.
use poe_optimizer_core::{
    owned_definitions::OwnedDefinitionKey, owned_schema::DefinitionSchemaIndex,
};
use poe_optimizer_import::{owned_item_lines::*, owned_value::WhitespacePolicy};
pub fn empty_items<I: DefinitionSchemaIndex>(schema: &I) -> OwnedItemLinePolicy {
    OwnedItemLinePolicy::new(
        ItemLinePolicyInput {
            schema_version: OWNED_ITEM_LINE_POLICY_VERSION,
            namespace: schema.namespace().clone(),
            version: OwnedDefinitionKey::new("explicit-empty-item-fixture").unwrap(),
            definitions: schema.identity().clone(),
            whitespace: WhitespacePolicy::Exact,
            rules: vec![],
        },
        schema,
        ItemLineLimits::default(),
    )
    .unwrap()
}

pub fn empty_item_source<I: DefinitionSchemaIndex>(
    schema: &I,
) -> poe_optimizer_import::owned_item_source::ItemSourceLayoutPolicy {
    empty_source_for_items(schema, &empty_items(schema))
}
pub fn empty_source_for_items<I: DefinitionSchemaIndex>(
    schema: &I,
    items: &OwnedItemLinePolicy,
) -> poe_optimizer_import::owned_item_source::ItemSourceLayoutPolicy {
    use poe_optimizer_import::{owned_item_source::*, owned_mapping::*};
    ItemSourceLayoutPolicy::new(
        ItemSourceLayoutPolicyInput {
            schema_version: OWNED_ITEM_SOURCE_POLICY_VERSION,
            namespace: schema.namespace().clone(),
            version: OwnedDefinitionKey::new("explicit-empty-source-fixture").unwrap(),
            source: SourcePin {
                system: ExternalSourceSystem::PathOfBuilding2,
                revision: "c".repeat(40),
                files: vec![SourceFilePin {
                    path: "injected/source-layout.json".into(),
                    sha256: "d".repeat(64),
                }],
            },
            item_lines: *items.identity(),
            dialect: ItemSourceDialect::PobExportedSingleTextV1,
            property_bindings: vec![],
            template_defaults: vec![],
            rule_layouts: vec![],
            template_layouts: vec![],
        },
        items,
        schema,
        ItemSourceLimits::default(),
    )
    .unwrap()
}
