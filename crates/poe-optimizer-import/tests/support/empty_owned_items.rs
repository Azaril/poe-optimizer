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
