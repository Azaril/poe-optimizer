//! Shared finite inputs for successor packaging tests; never runtime build defaults.
use poe_optimizer_core::owned_definitions::OwnedDefinitionKey;
use poe_optimizer_import::{owned_successor::*, owned_tree_policy::*};
use serde::de::DeserializeOwned;
use std::{fs, path::Path};
fn load<T: DeserializeOwned>(root: &Path, name: &str) -> T {
    serde_json::from_slice(&fs::read(root.join("data/owned/poe2/3887ae68").join(name)).unwrap())
        .unwrap()
}
pub fn input(root: &Path) -> SuccessorBundleInput {
    SuccessorBundleInput {
        schema_version: OWNED_SUCCESSOR_VERSION,
        prior: load(root, "import/compiled/recipe.json"),
        successor: load(root, "resistance/recipe.json"),
        mapping: load(root, "import/compiled/mapping.json"),
        roles: load(root, "import/compiled/roles.json"),
        normalization: load(root, "import/policies/normalization.json"),
        rewards: load(root, "import/policies/rewards.json"),
        query_sets: (1..=5)
            .map(|i| NamedQuerySet {
                name: OwnedDefinitionKey::new(format!("original-{i:02}")).unwrap(),
                queries: load(root, &format!("import/queries/original-{i:02}.json")),
            })
            .collect(),
        items: load(root, "resistance/items.json"),
        item_source: load(root, "resistance/item-source.json"),
    }
}
pub fn next(prior: &StagedSuccessorBundle) -> SuccessorBundleInput {
    SuccessorBundleInput {
        schema_version: OWNED_SUCCESSOR_VERSION,
        prior: prior.recipe().clone(),
        successor: prior.recipe().clone(),
        mapping: prior.mapping().input().clone(),
        roles: prior.roles().input().clone(),
        normalization: prior.normalization().clone(),
        rewards: prior.rewards().input().clone(),
        query_sets: prior.query_sets().to_vec(),
        items: prior.items().input().clone(),
        item_source: prior.item_source().input().clone(),
    }
}
pub fn append(input: &SuccessorBundleInput) -> CatalogAppend {
    CatalogAppend {
        mappings: vec![],
        source: input.mapping.source.clone(),
        item_policies: CatalogItemPolicyMode::RebindPrior,
    }
}
pub fn tree(input: &SuccessorBundleInput) -> TreePolicyTransitionInput {
    use poe_optimizer_core::owned_content::digest_owned;
    TreePolicyTransitionInput::Install {
        content: Box::new(TreeNormalizationContent {
            version: OwnedDefinitionKey::new("compact-test-tree").unwrap(),
            source: input.mapping.source.clone(),
            catalog: digest_owned("test-catalog", &1, 100).unwrap(),
            policy: digest_owned("test-policy", &2, 100).unwrap(),
            tree_version: "test-tree".into(),
            classes: vec![],
            ascendancies: vec![],
            tokens: vec![TreeTokenRow {
                token: "unknown-node".into(),
                role: TreeTokenRole::Unresolved {
                    code: OwnedDefinitionKey::new("not-converted").unwrap(),
                },
            }],
            attributes: vec![],
            syntax: TreeNormalizationSyntax {
                tree_version_attribute: "treeVersion".into(),
                class_attribute: "classInternalId".into(),
                ascendancy_attribute: "ascendancyInternalId".into(),
                class_consistency_attribute: Some("classId".into()),
                ascendancy_consistency_attribute: Some("ascendClassId".into()),
                overrides_element: "Overrides".into(),
                attribute_override_element: "AttributeOverride".into(),
                weapon_overlays: vec![],
                ignored_spec_children: vec!["URL".into()],
            },
        }),
    }
}
