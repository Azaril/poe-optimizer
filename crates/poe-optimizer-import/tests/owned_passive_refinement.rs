//! Endpoint-bound schema refinement; numerical coverage is deliberately unchanged.
use poe_optimizer_core::{
    owned_definitions::{OwnedDefinitionKey, PassiveNodeDefId},
    owned_schema::{DefinitionDescriptor, SchemaClosure, SchemaState},
};
use poe_optimizer_data::owned_schema::{OwnedDefinitionSchemaPackage, OwnedSchemaLimits};
use poe_optimizer_import::{owned_successor::*, owned_tree_policy::TreeNormalizationPackageInput};
use serde::de::DeserializeOwned;
use std::{fs, path::PathBuf};

fn load<T: DeserializeOwned>(name: &str) -> T {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/owned/poe2/3887ae68/current")
        .join(name);
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn input() -> SuccessorBundleInput {
    SuccessorBundleInput {
        schema_version: OWNED_SUCCESSOR_VERSION,
        prior: load("recipe.json"),
        successor: load("recipe.json"),
        mapping: load("mapping.json"),
        roles: load("roles.json"),
        normalization: load("normalization.json"),
        rewards: load("rewards.json"),
        items: load("items.json"),
        item_source: load("item-source.json"),
        query_sets: (1..=5)
            .map(|i| NamedQuerySet {
                name: OwnedDefinitionKey::new(format!("original-{i:02}")).unwrap(),
                queries: load(&format!("queries-original-{i:02}.json")),
            })
            .collect(),
    }
}
fn append(input: &SuccessorBundleInput) -> CatalogAppend {
    CatalogAppend {
        mappings: vec![],
        source: input.mapping.source.clone(),
        item_policies: CatalogItemPolicyMode::RebindPrior,
    }
}
fn tree() -> TreePolicyTransitionInput {
    TreePolicyTransitionInput::RebindPrior {
        prior: Box::new(load("tree-normalization.json")),
    }
}
fn bind(input: &mut SuccessorBundleInput) {
    let schema = OwnedDefinitionSchemaPackage::new(
        input.successor.schema.clone(),
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    input.successor.rules.definitions = schema.identity().clone();
    input.successor.routing.definitions = schema.identity().clone();
}
fn changed() -> (SuccessorBundleInput, PassiveDeclarationRefinement) {
    let mut input = input();
    let node = input
        .successor
        .schema
        .definitions
        .iter_mut()
        .find_map(|row| {
            let DefinitionDescriptor::PassiveNode(entry) = row else {
                return None;
            };
            let SchemaState::Known(schema) = &mut entry.schema else {
                return None;
            };
            if schema.declarations.choices.members.len() != 1 {
                return None;
            }
            let d = &mut schema.declarations;
            for closure in [
                &mut d.parameters.closure,
                &mut d.choices.closure,
                &mut d.grants.closure,
                &mut d.actors.closure,
                &mut d.skill_grants.closure,
                &mut d.outputs.closure,
                &mut d.sockets.closure,
            ] {
                *closure = SchemaClosure::Complete;
            }
            Some(entry.id.clone())
        })
        .unwrap();
    bind(&mut input);
    let policy = PassiveDeclarationRefinement {
        schema_version: 1,
        before: input.prior.rules.definitions.clone(),
        after: input.successor.rules.definitions.clone(),
        nodes: vec![node],
    };
    (input, policy)
}
fn apply(
    input: SuccessorBundleInput,
    policy: PassiveDeclarationRefinement,
) -> Result<StagedSuccessorBundle, SuccessorBundleError> {
    let append = append(&input);
    transition_owned_catalog_with_tree_refinement(
        input,
        append,
        tree(),
        policy,
        SuccessorBundleLimits::default(),
    )
}
fn passive<'a>(
    input: &'a mut SuccessorBundleInput,
    node: &PassiveNodeDefId,
) -> &'a mut poe_optimizer_core::owned_schema::PassiveNodeSchema {
    input
        .successor
        .schema
        .definitions
        .iter_mut()
        .find_map(|d| match d {
            DefinitionDescriptor::PassiveNode(entry) if &entry.id == node => {
                match &mut entry.schema {
                    SchemaState::Known(schema) => Some(schema),
                    _ => None,
                }
            }
            _ => None,
        })
        .unwrap()
}

#[test]
fn closes_only_reviewed_ports_and_retains_tree_policies_and_query_meaning() {
    let (input, policy) = changed();
    let source = input.clone();
    let staged = apply(input, policy.clone()).unwrap();
    assert_eq!(staged.recipe(), &source.successor);
    assert_eq!(staged.transition().schema_refinement, Some(policy));
    assert_eq!(
        staged.transition().preserved_definitions,
        source.prior.schema.definitions.len() - 1
    );
    assert_eq!(staged.recipe().registry, source.prior.registry);
    assert_eq!(staged.recipe().schema.slots, source.prior.schema.slots);
    assert_eq!(
        staged.recipe().rules.owners,
        source.prior.rules.owners,
        "port closure is not numerical coverage"
    );
    assert_eq!(staged.mapping().input().entries, source.mapping.entries);
    assert_eq!(staged.mapping().input().source, source.mapping.source);
    let mut expected_source = source.item_source.clone();
    expected_source.item_lines = *staged.items().identity();
    assert_eq!(staged.item_source().input(), &expected_source);
    assert_eq!(staged.query_sets(), source.query_sets);
    assert_eq!(staged.transition().query_rows, 110);
    let prior_tree: TreeNormalizationPackageInput = load("tree-normalization.json");
    assert_eq!(staged.tree().unwrap().input().content, prior_tree.content);
    assert_ne!(
        staged.tree().unwrap().input().definitions,
        prior_tree.definitions
    );
}

#[test]
fn ordinary_successor_cannot_perform_implicit_refinement() {
    let (input, _) = changed();
    let append = append(&input);
    assert!(matches!(
        transition_owned_catalog_with_tree(input, append, tree(), SuccessorBundleLimits::default()),
        Err(SuccessorBundleError::ChangedDeclaration)
    ));
}

#[test]
fn policy_binds_exact_endpoints_and_unique_existing_nodes() {
    let (input, policy) = changed();
    let mut cases = vec![];
    let mut bad = policy.clone();
    bad.before = policy.after.clone();
    cases.push(bad);
    let mut bad = policy.clone();
    bad.after = policy.before.clone();
    cases.push(bad);
    let mut bad = policy.clone();
    bad.schema_version = 2;
    cases.push(bad);
    let mut bad = policy.clone();
    bad.nodes.clear();
    cases.push(bad);
    let mut bad = policy.clone();
    bad.nodes.push(bad.nodes[0].clone());
    cases.push(bad);
    let mut bad = policy.clone();
    bad.nodes[0] = PassiveNodeDefId::new(
        bad.nodes[0].namespace().clone(),
        OwnedDefinitionKey::new("absent").unwrap(),
    );
    cases.push(bad);
    for bad in cases {
        assert!(apply(input.clone(), bad).is_err());
    }
}

#[test]
fn known_topology_cannot_be_rewritten_under_a_closure_policy() {
    let (mut input, mut policy) = changed();
    let schema = passive(&mut input, &policy.nodes[0]);
    assert!(!schema.adjacent.members.is_empty());
    schema.adjacent.members.clear();
    bind(&mut input);
    policy.after = input.successor.rules.definitions.clone();
    assert!(matches!(
        apply(input, policy),
        Err(SuccessorBundleError::Refinement(
            "not an exact closure refinement"
        ))
    ));
}

#[test]
fn incomplete_closure_is_not_a_refinement() {
    let (mut input, mut policy) = changed();
    let prior = input
        .prior
        .schema
        .definitions
        .iter()
        .find_map(|row| match row {
            DefinitionDescriptor::PassiveNode(entry) if entry.id == policy.nodes[0] => match &entry
                .schema
            {
                SchemaState::Known(schema) => Some(schema.declarations.parameters.closure.clone()),
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    passive(&mut input, &policy.nodes[0])
        .declarations
        .parameters
        .closure = prior;
    bind(&mut input);
    policy.after = input.successor.rules.definitions.clone();
    assert!(matches!(
        apply(input, policy),
        Err(SuccessorBundleError::Refinement(_))
    ));
}

#[test]
fn unused_or_replayed_refinement_is_rejected() {
    let (mut input, mut policy) = changed();
    input.successor = input.prior.clone();
    policy.after = policy.before.clone();
    assert!(matches!(
        apply(input, policy),
        Err(SuccessorBundleError::Refinement(_))
    ));
}

#[test]
fn refined_package_can_be_carried_without_reasserting_old_refinement() {
    let (input, policy) = changed();
    let first = apply(input, policy).unwrap();
    let next = SuccessorBundleInput {
        schema_version: OWNED_SUCCESSOR_VERSION,
        prior: first.recipe().clone(),
        successor: first.recipe().clone(),
        mapping: first.mapping().input().clone(),
        roles: first.roles().input().clone(),
        normalization: first.normalization().clone(),
        rewards: first.rewards().input().clone(),
        items: first.items().input().clone(),
        item_source: first.item_source().input().clone(),
        query_sets: first.query_sets().to_vec(),
    };
    let append = append(&next);
    let tree = TreePolicyTransitionInput::RebindPrior {
        prior: Box::new(first.tree().unwrap().input().clone()),
    };
    let second =
        transition_owned_catalog_with_tree(next, append, tree, SuccessorBundleLimits::default())
            .unwrap();
    assert_eq!(second.recipe(), first.recipe());
    assert_eq!(second.transition().before, second.transition().after);
    assert!(second.transition().schema_refinement.is_none());
}
