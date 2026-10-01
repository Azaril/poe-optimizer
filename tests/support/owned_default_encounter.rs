//! A checked catalog addition and optional import projection into EnemySpec.
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::OwnedDefinitionKey};
use poe_optimizer_import::{
    owned_mapping::MappingEntry,
    owned_normalize::EncounterPolicy,
    owned_recipe_extension::{OwnedRecipeExtension, extend_owned_recipe},
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, SuccessorBundleInput, TreePolicyTransitionInput,
        transition_owned_catalog_with_tree_compact,
    },
    owned_tree_policy::OwnedTreeNormalizationPolicy,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{fs, path::PathBuf};

pub fn directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/owned/poe2/3887ae68/default-encounter")
}
fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(directory().join(name)).unwrap()).unwrap()
}
fn key(text: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(text).unwrap()
}
pub fn policy() -> EncounterPolicy {
    read("policy.json")
}
pub fn authoring() -> Value {
    read("authoring.json")
}
pub fn extension() -> OwnedRecipeExtension {
    read("extension.json")
}
pub fn mapping() -> MappingEntry {
    read("mapping.json")
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    let a = authoring();
    let receipt = serde_json::to_value(prior.receipt()).unwrap();
    assert_eq!(a["before"], receipt["input"]);
    assert_eq!(a["definitions"], receipt["definitions"]);
    assert_eq!(a["registry"], receipt["registry"]);
    assert_eq!(a["source_validation"]["status"], "passed");
    let b = prior.input();
    assert!(b.normalization.encounter.is_none());
    assert!(b.evaluation.is_none());
    let e = extension();
    let extended = extend_owned_recipe(prior.assembled(), &e, Default::default()).unwrap();
    assert_eq!(extended.receipt.allocated_entries, 1);
    assert_eq!(extended.receipt.refined_subjects, 0);
    assert_eq!(extended.receipt.appended_programs, 0);
    assert_eq!(extended.receipt.appended_receivers, 0);
    assert!(extended.refinement.is_none());
    let carried = transition_owned_catalog_with_tree_compact(
        SuccessorBundleInput {
            schema_version: 1,
            prior: b.recipe.clone(),
            successor: extended.successor,
            mapping: b.mapping.clone(),
            roles: b.roles.clone(),
            normalization: b.normalization.clone(),
            rewards: b.rewards.clone(),
            query_sets: b.query_sets.clone(),
            items: b.items.clone(),
            item_source: b.item_source.clone(),
        },
        CatalogAppend {
            mappings: vec![mapping()],
            source: b.mapping.source.clone(),
            item_policies: CatalogItemPolicyMode::RebindPrior,
        },
        TreePolicyTransitionInput::RebindPrior {
            prior: Box::new(b.tree.clone().unwrap()),
        },
        Default::default(),
    )
    .unwrap();
    assert_eq!(
        carried.mapping().source_identity(),
        prior.mapping().source_identity()
    );
    let mut full = b.clone();
    full.recipe = carried.recipe().clone();
    full.mapping = carried.mapping().input().clone();
    full.roles = carried.roles().input().clone();
    full.normalization = carried.normalization().clone();
    full.rewards = carried.rewards().input().clone();
    full.items = carried.items().input().clone();
    full.item_source = carried.item_source().input().clone();
    full.normalization.encounter = Some(policy());
    full.normalization.version = key("selected-default-encounter-v1");
    full.tree = Some(
        OwnedTreeNormalizationPolicy::bind_new(
            b.tree.as_ref().unwrap().content.clone(),
            carried.assembled().registry(),
            carried.assembled().schema(),
            carried.mapping(),
            &full.normalization,
            Default::default(),
        )
        .unwrap()
        .input()
        .clone(),
    );
    full.provenance.push(OwnedReleaseProvenance {
        kind: key("explicit-default-encounter"),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-default-encounter-authoring-v1",
            &(a, e, mapping(), policy()),
            2 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    preservation(prior, &next);
    next
}

pub fn preservation(prior: &StagedOwnedRelease, next: &StagedOwnedRelease) {
    let b = prior.input();
    let mut r = next.input().clone();
    let e = extension();
    assert_eq!(
        r.recipe.registry.entries.len(),
        b.recipe.registry.entries.len() + 1
    );
    assert_eq!(
        &r.recipe.registry.entries[..b.recipe.registry.entries.len()],
        b.recipe.registry.entries.as_slice()
    );
    assert_eq!(
        r.recipe.registry.last_issued.get(),
        b.recipe.registry.last_issued.get() + 1
    );
    r.recipe.registry = b.recipe.registry.clone();
    assert_eq!(
        r.recipe.schema.definitions.len(),
        b.recipe.schema.definitions.len() + 1
    );
    let requested = &e.schema[0];
    let i = r
        .recipe
        .schema
        .definitions
        .iter()
        .position(|d| {
            poe_optimizer_core::owned_schema::SchemaSubject::Definition(d.address())
                == requested.subject()
        })
        .unwrap();
    assert_eq!(
        poe_optimizer_import::owned_recipe_extension::SchemaExtensionEntry::Definition(
            r.recipe.schema.definitions.remove(i)
        ),
        *requested
    );
    assert_eq!(r.recipe.rules.owners.len(), b.recipe.rules.owners.len() + 1);
    let i = r
        .recipe
        .rules
        .owners
        .iter()
        .position(|o| o.owner == e.owners[0].owner)
        .unwrap();
    assert_eq!(r.recipe.rules.owners.remove(i), e.owners[0]);
    r.recipe.rules.definitions = b.recipe.rules.definitions.clone();
    r.recipe.routing.definitions = b.recipe.routing.definitions.clone();
    assert_eq!(r.mapping.entries.len(), b.mapping.entries.len() + 1);
    let i = r
        .mapping
        .entries
        .iter()
        .position(|m| m.source == mapping().source)
        .unwrap();
    assert_eq!(r.mapping.entries.remove(i), mapping());
    r.mapping.definitions = b.mapping.definitions.clone();
    r.mapping.registry = b.mapping.registry;
    r.roles.definitions = b.roles.definitions.clone();
    r.roles.mapping = b.roles.mapping;
    r.rewards.definitions = b.rewards.definitions.clone();
    r.rewards.mapping = b.rewards.mapping;
    r.items.definitions = b.items.definitions.clone();
    r.item_source.item_lines = b.item_source.item_lines;
    assert_eq!(r.normalization.encounter, Some(policy()));
    assert_eq!(
        r.normalization.version,
        key("selected-default-encounter-v1")
    );
    r.normalization.encounter = b.normalization.encounter.clone();
    r.normalization.version = b.normalization.version.clone();
    let mut n = serde_json::to_value(&r.normalization).unwrap();
    let old = serde_json::to_value(&b.normalization).unwrap();
    for path in [
        "/gem_quality/value/definitions",
        "/gem_inputs/definitions",
        "/equipment_membership/definitions",
        "/item_modifier_membership/definitions",
        "/item_modifier_membership/item_lines",
        "/item_modifier_membership/item_source",
        "/item_parameter_inputs/definitions",
        "/item_parameter_inputs/item_lines",
        "/item_parameter_inputs/item_source",
        "/gem_inventory/definitions",
        "/gem_inventory/roles",
        "/gem_inventory/scalar_inputs",
        "/configuration_reward_inventory/reward_policy",
    ] {
        *n.pointer_mut(path).unwrap() = old.pointer(path).unwrap().clone();
    }
    r.normalization = serde_json::from_value(n).unwrap();
    assert_eq!(
        r.tree.as_ref().unwrap().content,
        b.tree.as_ref().unwrap().content
    );
    r.tree = b.tree.clone();
    assert_eq!(r.provenance.len(), b.provenance.len() + 1);
    r.provenance.pop();
    assert!(
        r == *b,
        "one encounter definition, mapping and Partial owner plus exact checked dependency commitments only"
    );
}
