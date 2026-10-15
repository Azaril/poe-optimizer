//! Checked extension and exact inverse shared by data-defined scalar inputs.
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::OwnedDefinitionKey};
use poe_optimizer_import::{
    owned_normalize::{
        ConfigurationDefaultInput, ConfigurationDispositionRule, ConfigurationInputsPolicy,
        configuration_disposition_dependencies,
    },
    owned_recipe_extension::{OwnedRecipeExtension, SchemaExtensionEntry, extend_owned_recipe},
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, SuccessorBundleInput, TreePolicyTransitionInput,
        transition_owned_catalog_with_membership_refinement_compact,
    },
    owned_tree_policy::OwnedTreeNormalizationPolicy,
};
use serde::Serialize;
use serde_json::Value;
use std::fs;
fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn value(v: impl Serialize) -> Value {
    serde_json::to_value(v).unwrap()
}
pub fn stage(
    prior: &StagedOwnedRelease,
    directory: &std::path::Path,
    kind: &'static str,
    allocated: usize,
) -> StagedOwnedRelease {
    let read = |name: &str| -> Value {
        serde_json::from_slice(&fs::read(directory.join(name)).unwrap()).unwrap()
    };
    let input: ConfigurationDefaultInput = serde_json::from_value(read("input.json")).unwrap();
    let accounting: Vec<ConfigurationDispositionRule> =
        serde_json::from_value(read("accounting.json")).unwrap();
    let d: Value = read("dependencies.json");
    let b = prior.input();
    assert_eq!(value(prior.receipt().input), d["before"]);
    assert_eq!(
        b.recipe.registry.last_issued.get(),
        d["registry_last_issued"].as_i64().unwrap()
    );
    assert_eq!(value(&b.normalization.encounter), d["encounter_policy"]);
    if let Some(unit) = d.get("unit_descriptor") {
        assert!(
            b.recipe
                .schema
                .definitions
                .iter()
                .any(|row| value(row) == *unit),
            "existing unit descriptor must match the authenticated prior"
        );
    }
    let extension: OwnedRecipeExtension = serde_json::from_value(read("extension.json")).unwrap();
    let extended = extend_owned_recipe(prior.assembled(), &extension, Default::default()).unwrap();
    assert_eq!(
        (
            extended.receipt.allocated_entries,
            extended.receipt.refined_subjects,
            extended.receipt.appended_programs
        ),
        (allocated, 1, 0)
    );
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
    let Some(ConfigurationInputsPolicy::PobFreshConfigInputsV3 { default_inputs, .. }) =
        &mut full.normalization.configuration_inputs
    else {
        panic!("current default-input policy")
    };
    assert!(
        !default_inputs
            .iter()
            .any(|i| i.source_name == input.source_name)
    );
    default_inputs.push(input.clone());
    let dependencies = configuration_disposition_dependencies(
        &full.normalization,
        carried.rewards(),
        Default::default(),
    )
    .unwrap();
    let dispositions = full
        .normalization
        .configuration_dispositions
        .as_mut()
        .unwrap();
    dispositions.dependencies = dependencies;
    dispositions.rules.extend(accounting.clone());
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
        kind: key(kind),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            kind,
            &(
                read("extension.json"),
                read("input.json"),
                read("accounting.json"),
                read("dependencies.json"),
                read("source-evidence.json"),
            ),
            4 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    // Restore only the declared descriptor additions/refinement, dependency
    // rebindings and new adapter rows; every other complete input stays exact.
    let mut inverse = next.input().clone();
    assert_eq!(&inverse.mapping, carried.mapping().input());
    assert_eq!(&inverse.roles, carried.roles().input());
    assert_eq!(&inverse.rewards, carried.rewards().input());
    assert_eq!(&inverse.items, carried.items().input());
    assert_eq!(&inverse.item_source, carried.item_source().input());
    assert_eq!(
        inverse.tree.as_ref().unwrap().content,
        b.tree.as_ref().unwrap().content
    );
    for entry in &extension.schema {
        let SchemaExtensionEntry::Definition(added) = entry else {
            panic!()
        };
        let at = inverse
            .recipe
            .schema
            .definitions
            .iter()
            .position(|v| v.address() == added.address())
            .unwrap();
        assert_eq!(&inverse.recipe.schema.definitions[at], added);
        if let Some(old) = b
            .recipe
            .schema
            .definitions
            .iter()
            .find(|v| v.address() == added.address())
        {
            inverse.recipe.schema.definitions[at] = old.clone();
        } else {
            inverse.recipe.schema.definitions.remove(at);
        }
    }
    inverse.recipe.schema.release = b.recipe.schema.release.clone();
    inverse.recipe.registry = b.recipe.registry.clone();
    assert_eq!(
        &inverse.recipe.rules.definitions,
        next.assembled().schema().identity()
    );
    assert_eq!(
        &inverse.recipe.routing.definitions,
        next.assembled().schema().identity()
    );
    inverse.recipe.rules.definitions = b.recipe.rules.definitions.clone();
    inverse.recipe.routing.definitions = b.recipe.routing.definitions.clone();
    assert!(
        inverse.recipe.rules == b.recipe.rules,
        "rules differ beyond authenticated schema rebinding"
    );
    assert!(
        inverse.recipe.routing == b.recipe.routing,
        "routing differs beyond authenticated schema rebinding"
    );
    assert!(
        inverse.recipe.schema == b.recipe.schema,
        "schema inverse differs"
    );
    assert!(inverse.recipe == b.recipe, "recipe inverse differs");
    // Successor validation authenticates these rebindings; require all semantic
    // policy contents exact before restoring their dependent identities.
    assert_eq!(
        full_policy_inverse(
            &inverse.normalization,
            carried.normalization(),
            &input,
            &accounting
        ),
        *carried.normalization()
    );
    inverse.normalization = b.normalization.clone();
    inverse.mapping = b.mapping.clone();
    inverse.roles = b.roles.clone();
    inverse.rewards = b.rewards.clone();
    inverse.items = b.items.clone();
    inverse.item_source = b.item_source.clone();
    inverse.tree = b.tree.clone();
    assert_eq!(inverse.provenance.pop().unwrap().kind, key(kind));
    assert!(inverse == *b, "whole release inverse differs");
    next
}

fn full_policy_inverse(
    next: &poe_optimizer_import::owned_normalize::NormalizationPolicy,
    old: &poe_optimizer_import::owned_normalize::NormalizationPolicy,
    input: &ConfigurationDefaultInput,
    rules: &[ConfigurationDispositionRule],
) -> poe_optimizer_import::owned_normalize::NormalizationPolicy {
    let mut inverse = next.clone();
    let Some(ConfigurationInputsPolicy::PobFreshConfigInputsV3 { default_inputs, .. }) =
        &mut inverse.configuration_inputs
    else {
        panic!()
    };
    assert_eq!(&default_inputs.pop().unwrap(), input);
    let dispositions = inverse.configuration_dispositions.as_mut().unwrap();
    assert_eq!(
        dispositions
            .rules
            .split_off(dispositions.rules.len() - rules.len()),
        rules
    );
    dispositions.dependencies = old
        .configuration_dispositions
        .as_ref()
        .unwrap()
        .dependencies
        .clone();
    inverse
}
