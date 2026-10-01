//! Explicit raw item state; no computed channel or rule owner is added.
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::OwnedDefinitionKey};
use poe_optimizer_import::{
    owned_normalize::{ItemParameterInputsPolicy, OrdinaryItemParameterInputs},
    owned_recipe_extension::{OwnedRecipeExtension, SchemaExtensionEntry, extend_owned_recipe},
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, SuccessorBundleInput, TreePolicyTransitionInput,
        transition_owned_catalog_with_membership_refinement_compact,
    },
    owned_tree_policy::OwnedTreeNormalizationPolicy,
    owned_value::OptionToken,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{fs, path::PathBuf};

pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/owned/poe2/3887ae68/ordinary-item-inputs")
}
fn read<T: DeserializeOwned>(file: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(file)).unwrap()).unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
pub fn bindings() -> Vec<OrdinaryItemParameterInputs> {
    read("bindings.json")
}
pub fn extension() -> OwnedRecipeExtension {
    read("extension.json")
}
pub fn rarities() -> Vec<OptionToken> {
    read("rarities.json")
}
pub fn authoring() -> Value {
    read("authoring.json")
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    let a = authoring();
    assert_eq!(prior.receipt().input.to_string(), a["before"]);
    assert_eq!(
        serde_json::to_value(&prior.receipt().definitions).unwrap(),
        a["definitions"]
    );
    assert_eq!(
        serde_json::to_value(prior.receipt().items).unwrap(),
        a["items"]
    );
    assert_eq!(
        serde_json::to_value(prior.receipt().item_source).unwrap(),
        a["item_source"]
    );
    assert_eq!(
        serde_json::to_value(prior.receipt().normalization).unwrap(),
        a["normalization"]
    );
    let b = prior.input();
    assert!(b.normalization.item_parameter_inputs.is_none());
    let e = extension();
    let extended = extend_owned_recipe(prior.assembled(), &e, Default::default()).unwrap();
    assert_eq!(extended.receipt.allocated_entries, 13);
    assert_eq!(extended.receipt.refined_subjects, 2);
    assert_eq!(extended.receipt.appended_programs, 0);
    let carried = transition_owned_catalog_with_membership_refinement_compact(
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
            mappings: vec![],
            source: b.mapping.source.clone(),
            item_policies: CatalogItemPolicyMode::RebindPrior,
        },
        TreePolicyTransitionInput::RebindPrior {
            prior: Box::new(b.tree.clone().unwrap()),
        },
        extended.refinement.unwrap(),
        Default::default(),
    )
    .unwrap();
    let mut full = b.clone();
    full.recipe = carried.recipe().clone();
    full.mapping = carried.mapping().input().clone();
    full.roles = carried.roles().input().clone();
    full.normalization = carried.normalization().clone();
    full.rewards = carried.rewards().input().clone();
    full.items = carried.items().input().clone();
    full.item_source = carried.item_source().input().clone();
    full.tree = carried.tree().map(|v| v.input().clone());
    full.normalization.item_parameter_inputs =
        Some(ItemParameterInputsPolicy::PobFreshOrdinaryInputsV1 {
            definitions: carried.assembled().schema().identity().clone(),
            item_lines: *carried.items().identity(),
            item_source: *carried.item_source().identity(),
            templates: bindings(),
        });
    full.tree = Some(
        OwnedTreeNormalizationPolicy::bind_new(
            full.tree.as_ref().unwrap().content.clone(),
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
        kind: key("explicit-ordinary-item-inputs"),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-ordinary-item-inputs-v1",
            &(a, &e, bindings(), rarities()),
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
        b.recipe.registry.entries.len() + 13
    );
    assert_eq!(
        &r.recipe.registry.entries[..b.recipe.registry.entries.len()],
        b.recipe.registry.entries.as_slice()
    );
    assert_eq!(r.recipe.registry.last_issued.get(), 0x312b);
    r.recipe.registry = b.recipe.registry.clone();
    assert_eq!(
        r.recipe.schema.definitions.len(),
        b.recipe.schema.definitions.len() + 5
    );
    assert_eq!(r.recipe.schema.slots.len(), b.recipe.schema.slots.len() + 8);
    for requested in &e.schema {
        match requested {
            SchemaExtensionEntry::Definition(d) => {
                let i = r
                    .recipe
                    .schema
                    .definitions
                    .iter()
                    .position(|v| v.address() == d.address())
                    .unwrap();
                assert_eq!(r.recipe.schema.definitions[i], *d);
                if let Some(old) = b
                    .recipe
                    .schema
                    .definitions
                    .iter()
                    .find(|v| v.address() == d.address())
                {
                    r.recipe.schema.definitions[i] = old.clone();
                } else {
                    r.recipe.schema.definitions.remove(i);
                }
            }
            SchemaExtensionEntry::Slot(d) => {
                let i = r
                    .recipe
                    .schema
                    .slots
                    .iter()
                    .position(|v| v.address() == d.address())
                    .unwrap();
                assert_eq!(r.recipe.schema.slots.remove(i), *d);
            }
        }
    }
    // Only exact dependency commitments may differ outside the new data and
    // policy. Restoring this finite list permits a whole-endpoint equality check.
    r.recipe.rules.definitions = b.recipe.rules.definitions.clone();
    r.recipe.routing.definitions = b.recipe.routing.definitions.clone();
    r.mapping.definitions = b.mapping.definitions.clone();
    r.mapping.registry = b.mapping.registry;
    r.roles.definitions = b.roles.definitions.clone();
    r.roles.mapping = b.roles.mapping;
    r.rewards.definitions = b.rewards.definitions.clone();
    r.rewards.mapping = b.rewards.mapping;
    r.items.definitions = b.items.definitions.clone();
    r.item_source.item_lines = b.item_source.item_lines;
    let mut n = serde_json::to_value(&r.normalization).unwrap();
    let old = serde_json::to_value(&b.normalization).unwrap();
    let policy = n
        .as_object_mut()
        .unwrap()
        .remove("item_parameter_inputs")
        .unwrap();
    assert_eq!(
        policy["definitions"],
        serde_json::to_value(&next.receipt().definitions).unwrap()
    );
    assert_eq!(
        policy["item_lines"],
        serde_json::to_value(next.receipt().items).unwrap()
    );
    assert_eq!(
        policy["item_source"],
        serde_json::to_value(next.receipt().item_source).unwrap()
    );
    assert_eq!(
        policy["templates"],
        serde_json::to_value(bindings()).unwrap()
    );
    for path in [
        "/gem_quality/value/definitions",
        "/gem_inputs/definitions",
        "/equipment_membership/definitions",
        "/item_modifier_membership/definitions",
        "/item_modifier_membership/item_lines",
        "/item_modifier_membership/item_source",
    ] {
        let target = n.pointer_mut(path).unwrap();
        *target = old.pointer(path).unwrap().clone();
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
        "only 13 allocated definitions/slots, two memberships, scoped policy, dependency commitments and one provenance row may differ"
    );
}
