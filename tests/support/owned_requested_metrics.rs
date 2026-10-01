//! Checked identity publication only; numerical metric bindings remain absent.
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::OwnedDefinitionKey};
use poe_optimizer_import::{
    owned_mapping::MappingEntry,
    owned_recipe_extension::{OwnedRecipeExtension, SchemaExtensionEntry, extend_owned_recipe},
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, SuccessorBundleInput, TreePolicyTransitionInput,
        transition_owned_catalog_with_tree_compact,
    },
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{fs, path::PathBuf};

pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/owned/poe2/3887ae68/requested-metrics")
}
fn read<T: DeserializeOwned>(file: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(file)).unwrap()).unwrap()
}
pub fn extension() -> OwnedRecipeExtension {
    read("extension.json")
}
pub fn mappings() -> Vec<MappingEntry> {
    read("mappings.json")
}
pub fn bindings() -> Value {
    read("bindings.json")
}
pub fn authoring() -> Value {
    read("authoring.json")
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    let a = authoring();
    let receipt = serde_json::to_value(prior.receipt()).unwrap();
    assert_eq!(receipt["input"], a["before"]);
    for field in [
        "registry",
        "definitions",
        "mapping",
        "items",
        "item_source",
        "normalization",
    ] {
        assert_eq!(receipt[field], a[field], "exact prior {field}");
    }
    let b = prior.input();
    assert!(b.evaluation.is_none());
    let e = extension();
    let extended = extend_owned_recipe(prior.assembled(), &e, Default::default()).unwrap();
    assert_eq!(extended.receipt.allocated_entries, 21);
    assert_eq!(extended.receipt.refined_subjects, 0);
    assert_eq!(extended.receipt.appended_programs, 0);
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
            mappings: mappings(),
            source: b.mapping.source.clone(),
            item_policies: CatalogItemPolicyMode::RebindPrior,
        },
        TreePolicyTransitionInput::RebindPrior {
            prior: Box::new(b.tree.clone().unwrap()),
        },
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
    full.provenance.push(OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new("explicit-requested-metric-identities").unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-requested-metric-identities-v1",
            &(a, &e, mappings(), bindings()),
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
    assert_eq!(
        r.recipe.registry.entries.len(),
        b.recipe.registry.entries.len() + 21
    );
    assert_eq!(
        &r.recipe.registry.entries[..b.recipe.registry.entries.len()],
        b.recipe.registry.entries.as_slice()
    );
    assert_eq!(r.recipe.registry.last_issued.get(), 0x3140);
    r.recipe.registry = b.recipe.registry.clone();
    assert_eq!(
        r.recipe.schema.definitions.len(),
        b.recipe.schema.definitions.len() + 21
    );
    assert_eq!(r.recipe.schema.slots, b.recipe.schema.slots);
    for requested in extension().schema {
        let SchemaExtensionEntry::Definition(d) = requested else {
            panic!("no slots in metric identity extension")
        };
        assert!(
            !b.recipe
                .schema
                .definitions
                .iter()
                .any(|v| v.address() == d.address())
        );
        let i = r
            .recipe
            .schema
            .definitions
            .iter()
            .position(|v| v.address() == d.address())
            .unwrap();
        assert_eq!(r.recipe.schema.definitions.remove(i), d);
    }
    assert_eq!(r.mapping.entries.len(), b.mapping.entries.len() + 20);
    for entry in mappings() {
        assert!(!b.mapping.entries.iter().any(|v| v.source == entry.source));
        let i = r
            .mapping
            .entries
            .iter()
            .position(|v| v.source == entry.source)
            .unwrap();
        assert_eq!(r.mapping.entries.remove(i), entry);
    }
    // Every program, source record and old mapping remains exact. Only checked
    // dependent commitments change outside the finite additions above.
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
        "only declared metric identity additions and dependency commitments may change"
    );
    assert!(next.input().evaluation.is_none());
}
